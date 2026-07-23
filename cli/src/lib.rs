//! Core implementation for the Rust SoK CLI.

mod profiles;
pub mod report;

use anyhow::{anyhow, bail, Context, Result};
use chrono::Local;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_DISPOSITION, CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

const SCHEMA_VERSION: &str = "sok-agent-pack-v0.3";

pub fn version_string() -> String {
    format!("sok {} (rust)", env!("CARGO_PKG_VERSION"))
}

const SOURCE_FIELDS: [&str; 13] = [
    "title",
    "type",
    "identifier",
    "url",
    "date",
    "access_status",
    "access_route",
    "budget_estimate",
    "license",
    "layer",
    "why_it_matters",
    "use_in_curriculum",
    "notes",
];

const OPEN_ACCESS_STATUSES: [&str; 7] = [
    "open_access",
    "free_web",
    "public_domain",
    "cc_by",
    "cc_by_sa",
    "official_open",
    "user_provided",
];

const BLOCKED_ACCESS_STATUSES: [&str; 5] = [
    "paid_book",
    "paywalled",
    "subscription",
    "unknown",
    "restricted",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccessInfo {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub route: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub budget_estimate: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Source {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub citation: String,
    #[serde(default, rename = "type", skip_serializing_if = "String::is_empty")]
    pub source_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub identifier: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub date: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub access_status: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub access_route: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub budget_estimate: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub license: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub layer: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub why_it_matters: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub use_in_curriculum: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<AccessInfo>,
}

#[derive(Debug, Serialize)]
struct AgentPack {
    field: String,
    learner: String,
    goal: String,
    mode: String,
    #[serde(skip_serializing_if = "is_zero")]
    weeks: i32,
    created: String,
    schema: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadRecord {
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub bytes: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content_type: String,
}

fn is_zero(value: &i32) -> bool {
    *value == 0
}

fn is_zero_i64(value: &i64) -> bool {
    *value == 0
}

pub fn run_cli(args: Vec<String>) -> Result<i32> {
    if args.is_empty() {
        print_usage();
        return Ok(2);
    }

    if args
        .get(1)
        .is_some_and(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_command_usage(&args[0])?;
        return Ok(0);
    }

    match args[0].as_str() {
        "-h" | "--help" => {
            print_usage();
            Ok(0)
        }
        "help" => {
            if let Some(command) = args.get(1) {
                print_command_usage(command)?;
            } else {
                print_usage();
            }
            Ok(0)
        }
        "-V" | "--version" | "version" => {
            println!("{}", version_string());
            Ok(0)
        }
        "init" => run_init(&args[1..]).map(|_| 0),
        "brief" => run_brief(&args[1..]).map(|_| 0),
        "scaffold" => run_scaffold(&args[1..]).map(|_| 0),
        "handoff-report" => run_handoff_report(&args[1..]).map(|_| 0),
        "source-template" => run_source_template(&args[1..]).map(|_| 0),
        "audit-sources" => run_audit_sources(&args[1..]).map(|_| 0),
        "download-sources" => run_download_sources(&args[1..]).map(|_| 0),
        "ingest" => run_ingest(&args[1..]).map(|_| 0),
        "export-json" => run_export_json(&args[1..]).map(|_| 0),
        "lint" => run_lint(&args[1..]),
        "validate-report" => run_validate_report(&args[1..]),
        "render-html" => run_render_html(&args[1..]).map(|_| 0),
        "specificity" => run_specificity(&args[1..]).map(|_| 0),
        command => bail!("unknown command {command:?}"),
    }
}

pub fn print_usage() {
    println!(
        r#"SoK agent CLI

Usage:
  sok <command> [options]

Global options:
  -h, --help        Show this help.
  -V, --version     Show the CLI version and implementation.

Commands:
  init              Create an agent workspace.
  brief             Generate an agent handoff brief.
  scaffold          Generate an agent-facing SoK report scaffold.
  handoff-report    Generate a next-agent brief for completing a human-reader report from a scaffold.
  source-template   Create a header-only source manifest CSV template.
  audit-sources     Audit source access metadata.
  download-sources  Download legally accessible open/free sources from a manifest.
  ingest last       Normalize a current source manifest into cataloged evidence JSONL; overwrites --output.
  export-json       Convert bounded Markdown plus source/evidence files to sok-report.json.
  lint              Check Markdown, source, and evidence inputs before JSON export.
  validate-report   Validate sok-report.json reliability gates.
  render-html       Render a validated human_report JSON file to self-contained local HTML.
  specificity       Generate a concreteness checklist for an underspecified domain task.

Run "sok <command> -h" for command options."#
    );
}

fn print_command_usage(command: &str) -> Result<()> {
    match command {
        "init" => print_init_usage(),
        "brief" => print_brief_usage(),
        "scaffold" => print_scaffold_usage(),
        "handoff-report" => print_handoff_report_usage(),
        "source-template" => print_source_template_usage(),
        "audit-sources" => print_audit_sources_usage(),
        "download-sources" => print_download_sources_usage(),
        "ingest" => print_ingest_usage(),
        "export-json" => print_export_json_usage(),
        "lint" => print_lint_usage(),
        "validate-report" => print_validate_report_usage(),
        "render-html" => print_render_html_usage(),
        "specificity" => print_specificity_usage(),
        command => bail!("unknown command {command:?}"),
    }
    Ok(())
}

fn print_init_usage() {
    println!(
        r#"Usage:
  sok init --field <field> --out <directory> [--learner <description>] [--goal <goal>] [--mode research|textbook|model-tuning|curriculum] [--weeks <number>] [--output-format <format>]

Creates an agent workspace with a brief, task list, report scaffold, header-only source manifest, and working directories."#
    );
}

fn print_brief_usage() {
    println!(
        r#"Usage:
  sok brief --field <field> [--learner <description>] [--goal <goal>] [--mode research|textbook|model-tuning|curriculum] [--weeks <number>] [--output-format <format>] [--output <brief.md>]

Writes the brief to --output or prints it to standard output."#
    );
}

fn print_scaffold_usage() {
    println!(
        r#"Usage:
  sok scaffold --field <field> [--learner <description>] [--goal <goal>] [--weeks <number>] [--output <scaffold.md>]

Writes a provisional, domain-aware report scaffold to --output or prints it to standard output."#
    );
}

fn print_handoff_report_usage() {
    println!(
        r#"Usage:
  sok handoff-report --scaffold <scaffold.md> [--field <field>] [--learner <description>] [--goal <goal>] [--weeks <number>] [--output <handoff.md>]
  sok handoff-report --field <field> [--learner <description>] [--goal <goal>] [--weeks <number>] [--output <handoff.md>]

Uses the supplied scaffold when present; otherwise creates one from --field. Writes to --output or standard output."#
    );
}

fn print_source_template_usage() {
    println!(
        r#"Usage:
  sok source-template --output <sources.csv>

Creates a header-only source manifest. Add real source rows before ingesting it; the template never inserts placeholder evidence."#
    );
}

fn print_audit_sources_usage() {
    println!(
        r#"Usage:
  sok audit-sources --manifest <sources.csv|sources.tsv|sources.json> [--strict]

Checks source identity, access, and curricular-role metadata. --strict returns an error when problems are found."#
    );
}

fn print_download_sources_usage() {
    println!(
        r#"Usage:
  sok download-sources --manifest <sources.csv|sources.tsv|sources.json> --out-dir <directory> [--allow-status <comma-separated-statuses>] [--include-unknown] [--dry-run] [--max-mb <number>] [--timeout <seconds>]

Downloads only sources allowed by access status. Inspect a --dry-run before permitting network downloads."#
    );
}

fn print_specificity_usage() {
    println!(
        r#"Usage:
  sok specificity --field <field> [--output <checklist.md>]

Writes a domain-task specificity checklist to --output or prints it to standard output."#
    );
}

pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn run_init(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &[])?;
    let field = flags.string("field", "");
    let learner = flags.string("learner", "scholar new to the field");
    let goal = flags.string("goal", "build field-entry to research-fluent understanding");
    let mode = flags.string("mode", "research");
    let output_format = flags.string("output-format", "markdown");
    let weeks = flags.int("weeks", 0)?;
    let out = flags.string("out", "");

    require_field(&field)?;
    validate_mode(&mode)?;
    if out.trim().is_empty() {
        bail!("--out is required");
    }

    let out_dir = absolute_path(&out)?;
    for dir in [
        out_dir.clone(),
        out_dir.join("downloads"),
        out_dir.join("notes"),
        out_dir.join("logs"),
    ] {
        fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    }

    let pack = AgentPack {
        field: field.clone(),
        learner: learner.clone(),
        goal: goal.clone(),
        mode: mode.clone(),
        weeks,
        created: today(),
        schema: SCHEMA_VERSION.to_string(),
    };
    let pack_json = serde_json::to_string_pretty(&pack)?;
    write_text(out_dir.join("sok-agent-pack.json"), &(pack_json + "\n"))?;
    write_text(
        out_dir.join("agent-brief.md"),
        &build_agent_brief(&field, &learner, &goal, &mode, weeks, &output_format),
    )?;
    write_text(out_dir.join("tasks.md"), &build_tasks(&mode))?;
    write_text(
        out_dir.join("report.md"),
        &build_report_scaffold(&field, &learner, &goal, weeks),
    )?;
    write_sources_csv(out_dir.join("sources.csv"))?;

    println!("Created SoK agent workspace: {}", out_dir.display());
    Ok(())
}

fn run_brief(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &[])?;
    let field = flags.string("field", "");
    let learner = flags.string("learner", "scholar new to the field");
    let goal = flags.string("goal", "build field-entry to research-fluent understanding");
    let mode = flags.string("mode", "research");
    let output_format = flags.string("output-format", "markdown");
    let output = flags.string("output", "");
    let weeks = flags.int("weeks", 0)?;

    require_field(&field)?;
    validate_mode(&mode)?;
    let content = build_agent_brief(&field, &learner, &goal, &mode, weeks, &output_format);
    if output.is_empty() {
        print!("{content}");
    } else {
        write_text(&output, &content)?;
        println!("Wrote {output}");
    }
    Ok(())
}

fn run_scaffold(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &[])?;
    let field = flags.string("field", "");
    let learner = flags.string("learner", "scholar new to the field");
    let goal = flags.string("goal", "build field-entry to research-fluent understanding");
    let output = flags.string("output", "");
    let weeks = flags.int("weeks", 0)?;

    require_field(&field)?;
    let content = build_report_scaffold(&field, &learner, &goal, weeks);
    if output.is_empty() {
        print!("{content}");
    } else {
        write_text(&output, &content)?;
        println!("Wrote {output}");
    }
    Ok(())
}

fn run_handoff_report(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &[])?;
    let mut field = flags.string("field", "");
    let mut learner = flags.string("learner", "");
    let mut goal = flags.string("goal", "");
    let scaffold_path = flags.string("scaffold", "");
    let output = flags.string("output", "");
    let mut weeks = flags.int("weeks", 0)?;

    let mut scaffold = String::new();
    if !scaffold_path.is_empty() {
        scaffold = fs::read_to_string(&scaffold_path)
            .with_context(|| format!("read scaffold {scaffold_path}"))?;
    }
    if field.trim().is_empty() {
        field = infer_field_from_scaffold(&scaffold);
    }
    if learner.trim().is_empty() {
        learner = infer_scaffold_table_value(&scaffold, &["Learner", "Target learner"]);
    }
    if goal.trim().is_empty() {
        goal = infer_scaffold_table_value(&scaffold, &["Goal"]);
    }
    if weeks == 0 {
        weeks = infer_weeks_from_scaffold(&scaffold);
    }
    require_field(&field)?;
    if learner.trim().is_empty() {
        learner = "scholar new to the field".to_string();
    }
    if goal.trim().is_empty() {
        goal = "build field-entry to research-fluent understanding".to_string();
    }
    if scaffold.trim().is_empty() {
        scaffold = build_report_scaffold(&field, &learner, &goal, weeks);
    }

    let content = build_human_report_handoff(&field, &learner, &goal, weeks, &scaffold);
    if output.is_empty() {
        print!("{content}");
    } else {
        write_text(&output, &content)?;
        println!("Wrote {output}");
    }
    Ok(())
}

fn run_source_template(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &[])?;
    let output = flags.string("output", "");
    if output.is_empty() {
        bail!("--output is required");
    }
    write_sources_csv(&output)?;
    println!("Wrote {output}");
    Ok(())
}

fn run_ingest(args: &[String]) -> Result<()> {
    let Some(command) = args.first() else {
        bail!("ingest subcommand is required; usage: sok ingest last --sources <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl>");
    };
    match command.as_str() {
        "-h" | "--help" | "help" => {
            print_ingest_usage();
            Ok(())
        }
        "last" => run_ingest_last(&args[1..]),
        command => bail!("unknown ingest command {command:?}"),
    }
}

fn print_ingest_usage() {
    println!(
        r#"SoK ingest commands

Usage:
  sok ingest last --sources <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl>

Commands:
  last   Normalize the current run's source manifest into cataloged evidence JSONL.

The last command is bounded to the named input manifest and overwrites --output."#
    );
}

fn run_ingest_last(args: &[String]) -> Result<()> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_ingest_last_usage();
        return Ok(());
    }

    let flags = parse_flags(args, &[])?;
    let sources = flags.string("sources", "");
    let manifest = flags.string("manifest", "");
    let output = flags.string("output", "");

    if !sources.is_empty() && !manifest.is_empty() && sources != manifest {
        bail!("use only one of --sources or --manifest");
    }
    let input = first_non_empty([&sources, &manifest]);
    if input.is_empty() {
        bail!("--sources is required (or --manifest)");
    }
    if output.is_empty() {
        bail!("--output is required");
    }

    let normalized = ingest_last_manifest(&input, &output)?;
    println!(
        "Wrote {} cataloged evidence entries to {output}",
        normalized.evidence.len()
    );
    if !normalized.diagnostics.is_empty() {
        println!("Diagnostics: {}", normalized.diagnostics.len());
        for check in normalized.diagnostics {
            println!(
                "- {} {}: {}",
                diagnostic_severity_label(check.severity),
                check.check_id,
                check.message
            );
        }
    }
    Ok(())
}

fn ingest_last_manifest(input: &str, output: &str) -> Result<report::NormalizedSourceManifest> {
    let normalized = report::normalize_source_manifest(input)?;
    report::write_jsonl_file(output, &normalized.evidence)?;
    Ok(normalized)
}

fn print_ingest_last_usage() {
    println!(
        r#"Usage:
  sok ingest last --sources <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl>
  sok ingest last --manifest <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl>

Writes one cataloged EvidenceEntry JSON object per source row.
The command does not download, crawl, browse, or append to any global ledger.
The requested output file is overwritten."#
    );
}

fn run_export_json(args: &[String]) -> Result<()> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_export_json_usage();
        return Ok(());
    }

    let flags = parse_flags(args, &[])?;
    let stage = flags.string("stage", "");
    let report_path = flags.string("report", "");
    let scaffold_path = flags.string("scaffold", "");
    let sources = flags.string("sources", "");
    let evidence = flags.string("evidence", "");
    let output = flags.string("output", "");

    let stage = parse_export_stage(&stage)?;
    if !report_path.is_empty() && !scaffold_path.is_empty() && report_path != scaffold_path {
        bail!("use only one of --report or --scaffold");
    }
    let report_input = first_non_empty([&report_path, &scaffold_path]);
    if report_input.is_empty() {
        bail!("--report is required (or --scaffold)");
    }
    if sources.is_empty() {
        bail!("--sources is required");
    }
    if output.is_empty() {
        bail!("--output is required");
    }

    let evidence_input = if evidence.is_empty() {
        None
    } else {
        Some(evidence.as_str())
    };
    let document = report::export_markdown_report(
        report_input.as_str(),
        sources.as_str(),
        evidence_input,
        stage,
    )?;
    report::write_json_file(&output, &document)?;
    println!("Wrote {output}");
    if let Some(diagnostics) = document.diagnostics {
        println!("Diagnostics: {}", diagnostics.checks.len());
        for check in diagnostics.checks {
            println!(
                "- {} {}: {}",
                diagnostic_severity_label(check.severity),
                check.check_id,
                check.message
            );
        }
    }
    Ok(())
}

fn parse_export_stage(stage: &str) -> Result<report::ExportStage> {
    match stage.trim() {
        "scaffold" => Ok(report::ExportStage::Scaffold),
        "final" => Ok(report::ExportStage::Final),
        "" => bail!("--stage is required; choose scaffold or final"),
        other => bail!("invalid --stage {other:?}; choose scaffold or final"),
    }
}

fn print_export_json_usage() {
    println!(
        r#"Usage:
  sok export-json --stage scaffold|final --report <report.md> --sources <sources.csv|sources.tsv|sources.json> --output <sok-report.json> [--evidence <evidence.jsonl>]
  sok export-json --stage scaffold|final --scaffold <report.md> --sources <sources.csv|sources.tsv|sources.json> --output <sok-report.json> [--evidence <evidence.jsonl>]

The --scaffold flag is a path alias for --report.
The stage is always explicit; filenames and headings do not select the public/internal contract.
Only canonical SoK headings and Markdown table shapes are interpreted."#
    );
}

fn run_lint(args: &[String]) -> Result<i32> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_lint_usage();
        return Ok(0);
    }

    let flags = parse_flags(args, &["strict"])?;
    let stage = flags.string("stage", "");
    let report_path = flags.string("report", "");
    let scaffold_path = flags.string("scaffold", "");
    let sources = flags.string("sources", "");
    let evidence = flags.string("evidence", "");
    let strict = flags.bool("strict");

    let stage = parse_export_stage(&stage)?;
    if !report_path.is_empty() && !scaffold_path.is_empty() && report_path != scaffold_path {
        bail!("use only one of --report or --scaffold");
    }
    let report_input = first_non_empty([&report_path, &scaffold_path]);
    if report_input.is_empty() {
        bail!("--report is required (or --scaffold)");
    }
    if sources.is_empty() {
        bail!("--sources is required");
    }

    let evidence_input = if evidence.is_empty() {
        None
    } else {
        Some(evidence.as_str())
    };
    let lint = report::lint_markdown_report(
        report_input.as_str(),
        sources.as_str(),
        evidence_input,
        stage,
    )?;
    println!(
        "Lint: {} error(s), {} warning(s), {} info(s)",
        lint.error_count(),
        lint.warning_count(),
        lint.info_count()
    );
    if lint.diagnostics.checks.is_empty() {
        println!("No pre-export lint diagnostics.");
    } else {
        for check in &lint.diagnostics.checks {
            println!(
                "- {} {} {}: {}",
                diagnostic_severity_label(check.severity),
                check.check_id,
                diagnostic_target_label(check),
                check.message
            );
        }
    }

    if lint.has_errors() || (strict && lint.has_warnings()) {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn print_lint_usage() {
    println!(
        r#"Usage:
  sok lint --stage scaffold|final --report <report.md> --sources <sources.csv|sources.tsv|sources.json> [--evidence <evidence.jsonl>] [--strict]
  sok lint --stage scaffold|final --scaffold <report.md> --sources <sources.csv|sources.tsv|sources.json> [--evidence <evidence.jsonl>] [--strict]

Checks bounded Markdown, source manifests, and optional evidence ledgers before JSON export.
The --scaffold flag is a path alias for --report.
Default mode returns nonzero for errors. --strict also returns nonzero for warnings."#
    );
}

fn run_validate_report(args: &[String]) -> Result<i32> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_validate_report_usage();
        return Ok(0);
    }

    let flags = parse_flags(args, &["strict"])?;
    let input = flags.string("input", "");
    let strict = flags.bool("strict");
    if input.is_empty() {
        bail!("--input is required");
    }

    let validation = report::validate_report_file(&input)?;
    println!(
        "Validation: {} error(s), {} warning(s)",
        validation.error_count(),
        validation.warning_count()
    );
    if validation.diagnostics.checks.is_empty() {
        println!("No report validation diagnostics.");
    } else {
        for check in &validation.diagnostics.checks {
            println!(
                "- {} {} {}: {}",
                diagnostic_severity_label(check.severity),
                check.check_id,
                diagnostic_target_label(check),
                check.message
            );
        }
    }

    if validation.has_errors() || (strict && validation.has_warnings()) {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn print_validate_report_usage() {
    println!(
        r#"Usage:
  sok validate-report --input <sok-report.json> [--strict]

Validates the JSON-first SoK report contract and reliability gates.
Default mode returns nonzero for errors. --strict also returns nonzero for warnings."#
    );
}

fn run_render_html(args: &[String]) -> Result<()> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_render_html_usage();
        return Ok(());
    }

    let flags = parse_flags(args, &[])?;
    let input = flags.string("input", "");
    let output = flags.string("output", "");
    if input.is_empty() {
        bail!("--input is required");
    }
    if output.is_empty() {
        bail!("--output is required");
    }

    report::render_html_report_file(&input, &output)?;
    println!("Wrote {output}");
    Ok(())
}

fn print_render_html_usage() {
    println!(
        r#"Usage:
  sok render-html --input <sok-report.json> --output <report.html>

Renders a validated human_report JSON file into a self-contained local HTML report.
Only the explicit public report payload is rendered; internal_context and diagnostics are ignored."#
    );
}

fn diagnostic_target_label(check: &report::DiagnosticCheck) -> String {
    match (
        check.target_path.trim().is_empty(),
        check.entity_id.trim().is_empty(),
    ) {
        (true, true) => "-".to_string(),
        (false, true) => check.target_path.clone(),
        (true, false) => check.entity_id.clone(),
        (false, false) => format!("{}#{}", check.target_path, check.entity_id),
    }
}

fn diagnostic_severity_label(severity: report::DiagnosticSeverity) -> &'static str {
    match severity {
        report::DiagnosticSeverity::Info => "info",
        report::DiagnosticSeverity::Warning => "warning",
        report::DiagnosticSeverity::Error => "error",
    }
}

fn run_audit_sources(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &["strict"])?;
    let manifest = flags.string("manifest", "");
    let strict = flags.bool("strict");
    if manifest.is_empty() {
        bail!("--manifest is required");
    }
    let sources = load_sources(&manifest)?;
    let mut problems = Vec::new();
    for (index, item) in sources.iter().enumerate() {
        problems.extend(audit_source(item, index));
    }
    println!("Sources: {}", sources.len());
    if problems.is_empty() {
        println!("No source metadata problems found.");
        return Ok(());
    }
    println!("Problems: {}", problems.len());
    for problem in &problems {
        println!("- {problem}");
    }
    if strict {
        bail!("source audit failed");
    }
    Ok(())
}

fn run_download_sources(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &["include-unknown", "dry-run"])?;
    let manifest = flags.string("manifest", "");
    let out_dir = flags.string("out-dir", "");
    let allow_status = flags.string("allow-status", &sorted_open_access_statuses().join(","));
    let include_unknown = flags.bool("include-unknown");
    let dry_run = flags.bool("dry-run");
    let max_mb = flags.int("max-mb", 100)?;
    let timeout_seconds = flags.int("timeout", 30)?;

    if manifest.is_empty() {
        bail!("--manifest is required");
    }
    if out_dir.is_empty() {
        bail!("--out-dir is required");
    }
    if max_mb <= 0 {
        bail!("--max-mb must be positive");
    }
    if timeout_seconds <= 0 {
        bail!("--timeout must be positive");
    }

    let sources = load_sources(&manifest)?;
    fs::create_dir_all(&out_dir).with_context(|| format!("create {out_dir}"))?;
    let allowed = parse_status_set(&allow_status);
    let max_bytes = i64::from(max_mb) * 1024 * 1024;
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_seconds as u64))
        .build()?;
    let log_path = Path::new(&out_dir).join("download-log.jsonl");
    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("open {}", log_path.display()))?;

    for (index, item) in sources.iter().enumerate() {
        let mut record = DownloadRecord {
            title: source_identity(item, index),
            url: source_url(item),
            status: "skipped".to_string(),
            ..DownloadRecord::default()
        };
        let (ok, reason) = can_download(item, &allowed, include_unknown);
        if !ok {
            record.reason = reason.clone();
            write_download_record(&mut log_file, &record)?;
            println!("SKIP {}: {reason}", record.title);
            continue;
        }
        if !is_http_url(&record.url) {
            record.reason = "missing http(s) url".to_string();
            write_download_record(&mut log_file, &record)?;
            println!("SKIP {}: missing http(s) url", record.title);
            continue;
        }
        if dry_run {
            record.status = "dry-run".to_string();
            write_download_record(&mut log_file, &record)?;
            println!("DRY {}: {}", record.title, record.url);
            continue;
        }
        match download_one(
            &client,
            &record.url.clone(),
            &record.title.clone(),
            &out_dir,
            max_bytes,
            &mut record,
        ) {
            Ok(()) => {
                write_download_record(&mut log_file, &record)?;
                println!("DOWNLOADED {}: {}", record.title, record.path);
            }
            Err(err) => {
                record.status = "error".to_string();
                record.reason = err.to_string();
                write_download_record(&mut log_file, &record)?;
                println!("ERROR {}: {err}", record.title);
            }
        }
    }
    println!("Wrote log: {}", log_path.display());
    Ok(())
}

fn run_specificity(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &[])?;
    let field = flags.string("field", "");
    let output = flags.string("output", "");
    require_field(&field)?;
    let content = build_specificity_checklist(&field);
    if output.is_empty() {
        print!("{content}");
    } else {
        write_text(&output, &content)?;
        println!("Wrote {output}");
    }
    Ok(())
}

#[derive(Debug, Default)]
struct ParsedFlags {
    values: HashMap<String, String>,
    bools: HashSet<String>,
}

impl ParsedFlags {
    fn string(&self, key: &str, default: &str) -> String {
        self.values
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    fn int(&self, key: &str, default: i32) -> Result<i32> {
        match self.values.get(key) {
            Some(value) => value
                .parse::<i32>()
                .with_context(|| format!("invalid --{key} value {value:?}")),
            None => Ok(default),
        }
    }

    fn bool(&self, key: &str) -> bool {
        self.bools.contains(key)
    }
}

fn parse_flags(args: &[String], bool_flags: &[&str]) -> Result<ParsedFlags> {
    let bool_flags: HashSet<&str> = bool_flags.iter().copied().collect();
    let mut parsed = ParsedFlags::default();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "-h" || arg == "--help" {
            bail!("subcommand help is not implemented");
        }
        if !arg.starts_with('-') {
            bail!("unexpected argument {arg:?}");
        }

        let raw = arg.trim_start_matches('-');
        let (key, inline_value) = match raw.split_once('=') {
            Some((key, value)) => (key, Some(value.to_string())),
            None => (raw, None),
        };
        if key.is_empty() {
            bail!("invalid flag {arg:?}");
        }

        if bool_flags.contains(key) {
            let value = inline_value.unwrap_or_else(|| "true".to_string());
            if value == "true" {
                parsed.bools.insert(key.to_string());
            } else if value != "false" {
                bail!("invalid boolean value for --{key}: {value:?}");
            }
            index += 1;
            continue;
        }

        let value = match inline_value {
            Some(value) => value,
            None => {
                index += 1;
                args.get(index)
                    .cloned()
                    .ok_or_else(|| anyhow!("flag needs an argument: -{key}"))?
            }
        };
        parsed.values.insert(key.to_string(), value);
        index += 1;
    }
    Ok(parsed)
}

pub fn require_field(field: &str) -> Result<()> {
    if field.trim().is_empty() {
        bail!("--field is required");
    }
    Ok(())
}

pub fn validate_mode(mode: &str) -> Result<()> {
    if mode_descriptions().contains_key(mode) {
        return Ok(());
    }
    let modes = mode_descriptions().keys().copied().collect::<Vec<_>>();
    bail!(
        "invalid --mode {mode:?}; choose one of {}",
        modes.join(", ")
    );
}

fn mode_descriptions() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (
            "curriculum",
            "Design an evidence-grounded learning sequence and assessment path.",
        ),
        (
            "model-tuning",
            "Plan a legally usable domain corpus and evaluation set for model adaptation.",
        ),
        (
            "research",
            "Produce a SoK research report and curriculum map.",
        ),
        (
            "textbook",
            "Plan a textbook or long-form course artifact from the field structure.",
        ),
    ])
}

fn write_text<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
    }
    fs::write(path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn absolute_path(path: &str) -> Result<PathBuf> {
    let raw = PathBuf::from(path);
    let joined = if raw.is_absolute() {
        raw
    } else {
        std::env::current_dir()?.join(raw)
    };
    Ok(clean_path(&joined))
}

fn clean_path(path: &Path) -> PathBuf {
    let mut cleaned = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                cleaned.pop();
            }
            _ => cleaned.push(component.as_os_str()),
        }
    }
    cleaned
}

pub fn infer_field_from_scaffold(scaffold: &str) -> String {
    for raw_line in scaffold.lines() {
        let line = raw_line.trim();
        if let Some(value) = line.strip_prefix("# Structure of Knowledge:") {
            return value.trim().to_string();
        }
        if line.starts_with("# SoK") && line.contains(':') {
            if let Some((_, value)) = line.split_once(':') {
                if !value.trim().is_empty() {
                    return value.trim().to_string();
                }
            }
        }
    }
    infer_scaffold_table_value(scaffold, &["Field"])
}

pub fn infer_scaffold_table_value(scaffold: &str, labels: &[&str]) -> String {
    for raw_line in scaffold.lines() {
        let line = raw_line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<_> = line.trim_matches('|').split('|').collect();
        if cells.len() < 2 {
            continue;
        }
        let key = cells[0].trim();
        let value = cells[1].trim();
        if !value.is_empty() && labels.iter().any(|label| key.eq_ignore_ascii_case(label)) {
            return value.to_string();
        }
    }
    String::new()
}

pub fn infer_weeks_from_scaffold(scaffold: &str) -> i32 {
    let value = infer_scaffold_table_value(scaffold, &["Time budget"]);
    value
        .trim()
        .strip_suffix(" weeks")
        .and_then(|weeks| weeks.parse::<i32>().ok())
        .unwrap_or(0)
}

pub fn build_human_report_handoff(
    field: &str,
    learner: &str,
    goal: &str,
    weeks: i32,
    scaffold: &str,
) -> String {
    let duration = if weeks > 0 {
        format!("{weeks} weeks")
    } else {
        "not fixed".to_string()
    };
    format!(
        r#"# SoK Human-Report Handoff: {field}

Generated: {generated}

## Purpose

Use the scaffold below as internal planning material for a next agent. Complete it into a polished, source-grounded deep research report for a human reader.

## Internal Context

- Field: {field}
- Learner profile: {learner}
- Original goal: {goal}
- Time budget: {duration}

Do not render this internal context as a front-matter table in the final report. Use it only to calibrate depth, scope, source selection, and sequencing.

## Visibility Rules

- Do not include sections named `Research Frame`, `Scaffold Quality Notes`, `SoK Usefulness Notes`, or raw prompt-intent notes in the human-facing report.
- Do not expose the learner profile, original goal, prompt wording, or time budget unless the user explicitly asks for a curriculum plan where that context belongs.
- Convert `Scoped assumption` into a concise public scope note only when it affects interpretation.
- Replace scaffold placeholders such as `source to verify`, `date after lookup`, and generic source rows with researched citations or remove them.
- Keep access metadata and verification limits, but move them into source notes or a short final limitations section.

## Human-Reader Report Contract

1. Start with an executive thesis: what the field is, why it matters, and the few structural claims that organize the report.
2. Explain the knowledge architecture: generative questions, core objects, representations, methods, evidence standards, threshold concepts, and failure modes.
3. Select only visual views justified by the report's structured relations. Use the smallest set that clarifies hierarchy, dependency, evidence flow, or debate structure for the intended reader.
4. Turn the literature ladder into an evidence base: explain why each source matters, how it should be used, and what access route exists.
5. Include a research roadmap that moves from field entry to research fluency through concrete scholarly performances.
6. Date all current frontier claims and distinguish durable foundations from active debates, standards, tools, datasets, or strategic reports.
7. End with verification limits and next checks, not with scaffold-maintenance notes.

## Quality Gate

The final report should read like a deep research briefing, not a filled worksheet. It should preserve the scaffold's domain-fit intelligence while hiding the scaffolding machinery from the human reader.

## Input Scaffold

````markdown
{scaffold}
````
"#,
        generated = today()
    )
}

pub fn build_report_scaffold(field: &str, learner: &str, goal: &str, weeks: i32) -> String {
    let duration = if weeks > 0 {
        format!("{weeks} weeks")
    } else {
        "unspecified duration".to_string()
    };
    let profile = profiles::infer_scaffold_profile(field);
    format!(
        r#"# Structure of Knowledge: {field}

## Research Frame

| Item | Value |
|---|---|
| Field | {field} |
| Learner | {learner} |
| Goal | {goal} |
| Time budget | {duration} |
| Domain classification | {classification} |
| Profile hypothesis | {profile_hypothesis} |
| Scaffold stance | {scaffold_stance} |
| Scoped assumption | {scoped_assumption} |
| Evidence posture | {evidence_posture} |

## 1. Domain Decomposition

{decomposition}

## 2. Orientation

{orientation}

## 3. The Field's Deep Structure

| Element | In this field | First-pass representation | Deeper synthesis | Why it matters |
|---|---|---|---|---|
{structure_rows}

## 4. {map_title}

{map_caption}

```mermaid
{concept_map}
```

## 5. Source Role Probe

Source roles are diagnostic, not quotas. Use this table to decide which evidence roles are necessary for this run, and mark irrelevant roles as waived with a rationale after initial source review.

| Source role | Status | Candidate source pattern | What it tests | Waiver or revision rule |
|---|---|---|---|---|
{source_probe_rows}

## 6. Literature Ladder

| Layer | Source | Type | Identifier | Access route | Budget | Difficulty | Read for | Notes |
|---|---|---|---|---|---:|---:|---|---|
{literature_rows}

## 7. Curriculum Roadmap

| Phase | Module | Essential question | Readings | Practice artifact | Progress criteria |
|---|---|---|---|---|---|
{curriculum_rows}

## 8. Practice and Assessment

{practice}

## 9. Frontier, Debates, and Open Problems

| Problem or debate | Current state | Key sources | Required background | Why it is hard |
|---|---|---|---|---|
{frontier_rows}

## 10. Visual Summary

{visual_caption}

```mermaid
{visual_map}
```

## 11. Sources and Further Reading

Source priorities: {source_priorities}

| Source | Date | Type | Identifier | Access route | Budget | Layer | Why it matters | Confidence |
|---|---:|---|---|---|---:|---|---|---|
| {field} orientation source to verify | date after lookup | handbook/syllabus/notes | official URL/ISBN/DOI | open, publisher, or library route | $0 or library preferred | orientation | Establishes the field boundary and learner-facing advance organizer | medium until verified |
| {field} foundation or corpus source to verify | date after lookup | book/paper/standard/corpus/case | DOI/arXiv/ISBN/URL/catalog ID | official route with access status | budget unknown; library preferred | foundation | Anchors the durable concepts, methods, objects, cases, or corpora of the field | medium until verified |
| {field} method or warrant source to verify | date after lookup | methods text/problem source/protocol/standard | DOI/arXiv/ISBN/URL | official route with access status | $0 or library preferred | method | Shows how claims, interpretations, proofs, measurements, or performances are judged | medium until verified |
| {field} conditional source-role check | date after lookup or waived | recent survey/report/preprint/dataset/standard/primary corpus | official route or waiver rationale | $0, license terms, or library preferred | conditional | Verifies only the frontier, dataset, standard, infrastructure, corpus, or case roles needed for this goal | low until role is confirmed |

## 12. Scaffold Quality Notes

- This starter follows the harness gates: domain fit, source access metadata, pedagogical ladder, and not-a-topic-list.
- Treat existing curricula as evidence of stabilized pedagogical consensus, not as the boundary of the field.
- Replace starter source rows with researched citations before treating the report as authoritative.
- {quality_focus}
"#,
        classification = profile.classification,
        profile_hypothesis = profile.profile_hypothesis,
        scaffold_stance = profile.scaffold_stance,
        scoped_assumption = profile.scoped_assumption.as_str(),
        evidence_posture = profile.evidence_posture,
        decomposition = profile.decomposition.as_str(),
        orientation = profile.orientation.as_str(),
        structure_rows = profile.render_structure_rows(),
        map_title = profile.map_title,
        map_caption = profile.map_caption,
        concept_map = profile.concept_map.as_str(),
        source_probe_rows = profile.render_source_role_rows(),
        literature_rows = profile.literature_rows.join("\n"),
        curriculum_rows = profile.curriculum_rows.join("\n"),
        practice = profile.practice,
        frontier_rows = profile.frontier_rows.join("\n"),
        visual_caption = profile.visual_caption,
        visual_map = profile.visual_map.as_str(),
        source_priorities = profile.source_priorities,
        quality_focus = profile.quality_focus
    )
}

fn mode_deliverables(mode: &str) -> Vec<&'static str> {
    let mut common = vec![
        "Domain decomposition and scoped assumptions",
        "Substantive and syntactic structure map",
        "Source manifest with access metadata",
        "Evidence matrix with curricular roles",
        "Quality-gate self-audit",
    ];
    match mode {
        "textbook" => common.extend([
            "Textbook thesis and reader model",
            "Chapter architecture with dependencies",
            "Exercise ladder and worked examples plan",
            "Citation and permissions plan",
        ]),
        "model-tuning" => common.extend([
            "Legally usable corpus plan with license/access audit",
            "Data mixture by source type and subdomain",
            "Exclusion list for paywalled or license-unclear materials",
            "Evaluation set blueprint and contamination controls",
        ]),
        "curriculum" => common.extend([
            "Module-by-module curriculum roadmap",
            "Practice artifacts and progress criteria",
            "Assessment rubrics for scholarly performance",
        ]),
        _ => common.extend([
            "Narrative SoK report",
            "Literature ladder",
            "Frontier/debate map",
            "Relation-backed visual views when useful",
        ]),
    }
    common
}

pub fn build_agent_brief(
    field: &str,
    learner: &str,
    goal: &str,
    mode: &str,
    weeks: i32,
    output_format: &str,
) -> String {
    let duration = if weeks > 0 {
        format!("{weeks} weeks")
    } else {
        "not fixed".to_string()
    };
    let deliverables = mode_deliverables(mode)
        .into_iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mode_description = mode_descriptions()
        .get(mode)
        .copied()
        .unwrap_or("Produce a SoK research report and curriculum map.");
    format!(
        r#"# SoK Agent Brief: {field}

Generated: {generated}
Mode: {mode}
Mode description: {mode_description}

## Objective

Map {field} as a structured knowledge domain for {learner}.

Goal: {goal}
Time budget: {duration}
Requested output format: {output_format}

## Operating Rules

- Preserve domain precision while calibrating explanations, sequence, representations, and tasks to the intended audience.
- Classify the domain before committing to a curriculum structure.
- If the field is broad, decompose it into subfields and state the scoped path.
- Separate substantive structure from syntactic structure.
- Assign every source a curricular role and actionable access route.
- Treat paywalled, paid, subscription, and license-unclear sources as metadata-only unless the user supplies access or permission.
- Use current sources for frontier claims, active standards, tools, datasets, and strategic reports.

## Required Deliverables

{deliverables}

## Suggested Execution Stages

1. Frame: define field, learner, scope, assumptions, and domain type.
2. Decompose: identify subfields, shared foundations, divergent methods, and path options.
3. Source: build a source stack across orientation, foundation, methods, synthesis, frontier, and critique.
4. Extract: identify generative questions, core objects, representations, methods, evidence standards, thresholds, and failure modes.
5. Design: produce the curriculum or artifact architecture for the selected mode.
6. Verify: check currentness, access metadata, domain fit, and whether the result is more than a topic list.

## Handoff Files

- sources.csv: add candidate sources with access metadata.
- report.md: fill with the SoK report or artifact blueprint.
- tasks.md: use as an agent work queue.
- downloads/: place only legally downloadable open/free/user-provided materials.
"#,
        generated = today()
    )
}

pub fn build_tasks(mode: &str) -> String {
    let deliverables = mode_deliverables(mode)
        .into_iter()
        .map(|item| format!("- [ ] {item}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"# SoK Agent Tasks

## Setup

- [ ] Confirm field, learner, goal, and scope.
- [ ] Classify domain type and choose visual model.
- [ ] Decide whether broad-field decomposition is required.

## Sources

- [ ] Fill sources.csv.
- [ ] Run sok audit-sources --manifest sources.csv.
- [ ] Download only legally available open/free/user-provided sources.
- [ ] Mark paid/paywalled/subscription/unknown sources as metadata-only.

## Deliverables

{deliverables}

## Final Audit

- [ ] Source access metadata is complete.
- [ ] Current claims use current sources and dates.
- [ ] Output explains relations and warrants, not just topics.
- [ ] Curriculum or artifact path is useful for scholar-level work.
"#
    )
}

pub fn build_specificity_checklist(field: &str) -> String {
    format!(
        r#"# Specificity Checklist: {field}

Use this to turn an underspecified domain task into an executable agent brief.

## Scope
- What is the target artifact: report, textbook, curriculum, corpus, evaluation set, or interactive map?
- What is excluded from the field for this run?
- Which subfield path should be primary if the field is broad?

## Learner or Model Target
- What prior knowledge is assumed?
- What performance should be possible after completion?
- What artifacts should demonstrate mastery?

## Sources
- Which source types are mandatory: books, surveys, papers, standards, syllabi, datasets, software, archives?
- Which sources are legally downloadable?
- Which paid or paywalled sources are metadata-only?

## Domain Structure
- What is the substantive structure?
- What is the syntactic structure?
- What are the threshold concepts and bottlenecks?
- What visual form fits: prerequisite graph, network, debate map, timeline, instrument map, or mixed?

## Quality Gates
- Does every source have access metadata?
- Are current claims dated and sourced?
- Is the result more than a topic list?
"#
    )
}

pub fn write_sources_csv<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
    }
    let file = File::create(path).with_context(|| format!("create {}", path.display()))?;
    let mut writer = csv::Writer::from_writer(file);
    writer.write_record(SOURCE_FIELDS)?;
    writer.flush()?;
    Ok(())
}

pub fn load_sources<P: AsRef<Path>>(path: P) -> Result<Vec<Source>> {
    let path = path.as_ref();
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "csv" => load_sources_table(path, b','),
        "tsv" => load_sources_table(path, b'\t'),
        "json" => load_sources_json(path),
        _ => bail!("manifest must be .json, .csv, or .tsv"),
    }
}

fn load_sources_table(path: &Path, delimiter: u8) -> Result<Vec<Source>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(delimiter)
        .from_path(path)
        .with_context(|| format!("open {}", path.display()))?;
    let mut rows = reader.records();
    let Some(headers) = rows.next() else {
        return Ok(Vec::new());
    };
    let headers = headers?;
    let headers = headers.iter().map(str::to_string).collect::<Vec<_>>();
    let mut sources = Vec::new();
    for row in rows {
        let row = row?;
        let values = headers
            .iter()
            .enumerate()
            .map(|(index, header)| (header.as_str(), row.get(index).unwrap_or("")))
            .collect::<HashMap<_, _>>();
        sources.push(Source {
            title: value_for(&values, "title"),
            source_type: value_for(&values, "type"),
            identifier: value_for(&values, "identifier"),
            url: value_for(&values, "url"),
            date: value_for(&values, "date"),
            access_status: value_for(&values, "access_status"),
            access_route: value_for(&values, "access_route"),
            budget_estimate: value_for(&values, "budget_estimate"),
            license: value_for(&values, "license"),
            layer: value_for(&values, "layer"),
            why_it_matters: value_for(&values, "why_it_matters"),
            use_in_curriculum: value_for(&values, "use_in_curriculum"),
            notes: value_for(&values, "notes"),
            ..Source::default()
        });
    }
    Ok(sources)
}

fn value_for(values: &HashMap<&str, &str>, key: &str) -> String {
    values.get(key).copied().unwrap_or("").to_string()
}

fn load_sources_json(path: &Path) -> Result<Vec<Source>> {
    let data = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    if let Ok(list) = serde_json::from_slice::<Vec<Source>>(&data) {
        return Ok(list);
    }
    #[derive(Deserialize)]
    struct WrappedSources {
        sources: Vec<Source>,
    }
    let wrapped: WrappedSources = serde_json::from_slice(&data)?;
    Ok(wrapped.sources)
}

pub fn source_access(item: &Source) -> AccessInfo {
    if let Some(access) = &item.access {
        let mut access = access.clone();
        access.status = normalize_access_status(&access.status);
        return access;
    }
    AccessInfo {
        status: normalize_access_status(&first_non_empty([&item.access_status])),
        route: first_non_empty([&item.access_route, &item.url]),
        budget_estimate: item.budget_estimate.clone(),
        notes: String::new(),
    }
}

fn first_non_empty<const N: usize>(values: [&str; N]) -> String {
    for value in values {
        if !value.trim().is_empty() {
            return value.to_string();
        }
    }
    String::new()
}

pub fn source_identity(item: &Source, index: usize) -> String {
    first_non_empty([
        &item.title,
        &item.citation,
        &item.url,
        &format!("source-{}", index + 1),
    ])
}

pub fn source_url(item: &Source) -> String {
    let access = source_access(item);
    first_non_empty([&item.url, &access.route])
}

pub fn audit_source(item: &Source, index: usize) -> Vec<String> {
    let label = source_identity(item, index);
    let access = source_access(item);
    let mut problems = Vec::new();
    if first_non_empty([&item.title, &item.citation]).is_empty() {
        problems.push(format!("{label}: missing title/citation"));
    }
    if item.source_type.trim().is_empty() {
        problems.push(format!("{label}: missing type"));
    }
    if first_non_empty([&item.identifier, &item.url]).is_empty() {
        problems.push(format!("{label}: missing identifier or url"));
    }
    if access.status.trim().is_empty() {
        problems.push(format!("{label}: missing access status"));
    }
    if access.route.trim().is_empty() {
        problems.push(format!("{label}: missing access route"));
    }
    if blocked_access_statuses().contains(access.status.as_str())
        && access.budget_estimate.trim().is_empty()
    {
        problems.push(format!(
            "{label}: paid/paywalled/unknown source needs budget estimate or library guidance"
        ));
    }
    if first_non_empty([&item.why_it_matters, &item.use_in_curriculum]).is_empty() {
        problems.push(format!("{label}: missing curricular role"));
    }
    problems
}

pub fn parse_status_set(raw: &str) -> BTreeSet<String> {
    raw.split(',')
        .map(normalize_access_status)
        .filter(|status| !status.is_empty())
        .collect()
}

pub fn normalize_access_status(status: &str) -> String {
    status.trim().to_ascii_lowercase()
}

pub fn can_download(
    item: &Source,
    allowed: &BTreeSet<String>,
    include_unknown: bool,
) -> (bool, String) {
    let status = source_access(item).status;
    if status.is_empty() {
        if include_unknown {
            return (true, "unknown allowed by flag".to_string());
        }
        return (false, "blocked access status: missing".to_string());
    }
    if status == "unknown" {
        if include_unknown {
            return (true, "unknown allowed by flag".to_string());
        }
        return (false, "blocked access status: unknown".to_string());
    }
    if allowed.contains(&status) {
        return (true, "allowed".to_string());
    }
    if blocked_access_statuses().contains(status.as_str()) {
        return (false, format!("blocked access status: {status}"));
    }
    (false, format!("not in allowed statuses: {status}"))
}

fn blocked_access_statuses() -> BTreeSet<&'static str> {
    BLOCKED_ACCESS_STATUSES.into_iter().collect()
}

fn sorted_open_access_statuses() -> Vec<&'static str> {
    let mut statuses = OPEN_ACCESS_STATUSES.to_vec();
    statuses.sort_unstable();
    statuses
}

pub fn is_http_url(raw: &str) -> bool {
    url::Url::parse(raw)
        .map(|parsed| matches!(parsed.scheme(), "http" | "https"))
        .unwrap_or(false)
}

fn download_one(
    client: &Client,
    raw_url: &str,
    label: &str,
    out_dir: &str,
    max_bytes: i64,
    record: &mut DownloadRecord,
) -> Result<()> {
    let mut response = client
        .get(raw_url)
        .header(
            USER_AGENT,
            format!("SoK-Agent-CLI/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()?;
    if !response.status().is_success() {
        bail!("HTTP {}", response.status().as_u16());
    }

    let content_disposition = response
        .headers()
        .get(CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .unwrap_or_default();
    let filename = filename_from_headers(
        raw_url,
        content_disposition.as_deref(),
        if content_type.is_empty() {
            None
        } else {
            Some(content_type.as_str())
        },
        label,
    );
    let mut hasher = Sha256::new();
    hasher.update(raw_url.as_bytes());
    let prefix = &hex::encode(hasher.finalize())[..10];
    let output_path = Path::new(out_dir).join(format!("{prefix}-{filename}"));
    let temp_path = output_path.with_extension(format!(
        "{}part",
        output_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!("{ext}."))
            .unwrap_or_default()
    ));
    let mut file = File::create(&temp_path)?;
    let mut seen = 0_i64;
    let mut buffer = [0_u8; 8192];

    loop {
        let read = response.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let read = read as i64;
        if seen + read > max_bytes {
            drop(file);
            let _ = fs::remove_file(&temp_path);
            bail!("download exceeded {max_bytes} bytes limit");
        }
        file.write_all(&buffer[..read as usize])?;
        seen += read;
    }
    file.flush()?;
    drop(file);
    fs::rename(&temp_path, &output_path).inspect_err(|_err| {
        let _ = fs::remove_file(&temp_path);
    })?;

    record.status = "downloaded".to_string();
    record.path = output_path.display().to_string();
    record.bytes = seen;
    record.content_type = content_type;
    Ok(())
}

pub fn filename_from_headers(
    raw_url: &str,
    content_disposition: Option<&str>,
    content_type: Option<&str>,
    fallback: &str,
) -> String {
    if let Some(disposition) = content_disposition {
        if let Some(filename) = content_disposition_filename(disposition) {
            return sanitize_filename(&filename);
        }
    }
    if let Ok(parsed) = url::Url::parse(raw_url) {
        if let Some(base) = parsed
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .filter(|base| !base.is_empty())
        {
            return sanitize_filename(base);
        }
    }
    if let Some(content_type) = content_type {
        let media_type = content_type
            .split(';')
            .next()
            .unwrap_or(content_type)
            .trim()
            .to_ascii_lowercase();
        if let Some(ext) = common_extension(&media_type) {
            return sanitize_filename(&format!("{fallback}{ext}"));
        }
    }
    sanitize_filename(&format!("{fallback}.bin"))
}

fn content_disposition_filename(disposition: &str) -> Option<String> {
    disposition.split(';').skip(1).find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        if !key.trim().eq_ignore_ascii_case("filename") {
            return None;
        }
        let value = value.trim().trim_matches('"').to_string();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    })
}

pub fn common_extension(media_type: &str) -> Option<&'static str> {
    match media_type.to_ascii_lowercase().as_str() {
        "application/pdf" => Some(".pdf"),
        "text/html" => Some(".html"),
        "text/plain" => Some(".txt"),
        "text/markdown" => Some(".md"),
        "application/json" => Some(".json"),
        "text/csv" => Some(".csv"),
        _ => None,
    }
}

pub fn sanitize_filename(name: &str) -> String {
    let mut output = String::new();
    let mut previous_replacement = false;
    for ch in name.trim().chars() {
        let safe = ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-');
        if safe {
            output.push(ch);
            previous_replacement = false;
        } else if !previous_replacement {
            output.push('-');
            previous_replacement = true;
        }
    }
    let trimmed = output.trim_matches(['-', '.']).to_string();
    if trimmed.is_empty() {
        "source.bin".to_string()
    } else {
        trimmed
    }
}

pub fn write_download_record<W: Write>(writer: &mut W, record: &DownloadRecord) -> Result<()> {
    if serde_json::to_writer(&mut *writer, record).is_err() {
        writer.write_all(br#"{"status":"error","reason":"failed to encode download record"}"#)?;
    }
    writer.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests;
