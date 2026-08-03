//! Command implementations for the Rust SoK CLI.

use anyhow::{anyhow, bail, Context, Result};
use reqwest::blocking::Client;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::downloader::*;
use crate::report;
use crate::scaffold::*;
use crate::sources::*;

pub(crate) fn run_init(args: &[String]) -> Result<()> {
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

pub(crate) fn run_brief(args: &[String]) -> Result<()> {
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

pub(crate) fn run_scaffold(args: &[String]) -> Result<()> {
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

pub(crate) fn run_handoff_report(args: &[String]) -> Result<()> {
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

pub(crate) fn run_source_template(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &[])?;
    let output = flags.string("output", "");
    if output.is_empty() {
        bail!("--output is required");
    }
    write_sources_csv(&output)?;
    println!("Wrote {output}");
    Ok(())
}

pub(crate) fn run_ingest(args: &[String]) -> Result<()> {
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

pub(crate) fn print_ingest_usage() {
    println!(
        r#"SoK ingest commands

Usage:
  sok ingest last --sources <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl>

Commands:
  last   Normalize the current run's source manifest into cataloged evidence JSONL.

The last command is bounded to the named input manifest and overwrites --output."#
    );
}

pub(crate) fn run_ingest_last(args: &[String]) -> Result<()> {
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

pub(crate) fn ingest_last_manifest(
    input: &str,
    output: &str,
) -> Result<report::NormalizedSourceManifest> {
    let normalized = report::normalize_source_manifest(input)?;
    report::write_jsonl_file(output, &normalized.evidence)?;
    Ok(normalized)
}

pub(crate) fn print_ingest_last_usage() {
    println!(
        r#"Usage:
  sok ingest last --sources <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl>
  sok ingest last --manifest <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl>

Writes one cataloged EvidenceEntry JSON object per source row.
The command does not download, crawl, browse, or append to any global ledger.
The requested output file is overwritten."#
    );
}

pub(crate) fn run_export_json(args: &[String]) -> Result<()> {
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
    let knowledge = flags.string("knowledge", "");
    let evidence_package = flags.string("evidence-package", "");
    let pedagogy = flags.string("pedagogy", "");
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
    let machine_inputs = report::MachineInputPaths {
        knowledge: (!knowledge.is_empty()).then(|| PathBuf::from(&knowledge)),
        evidence_package: (!evidence_package.is_empty()).then(|| PathBuf::from(&evidence_package)),
        pedagogy: (!pedagogy.is_empty()).then(|| PathBuf::from(&pedagogy)),
    };
    let document = report::export_markdown_report_with_machine_inputs(
        report_input.as_str(),
        sources.as_str(),
        evidence_input,
        stage,
        machine_inputs,
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

pub(crate) fn print_export_json_usage() {
    println!(
        r#"Usage:
  sok export-json --stage scaffold|final --report <report.md> --sources <sources.csv|sources.tsv|sources.json> --output <sok-report.json> [--evidence <evidence.jsonl>]
  sok export-json --stage scaffold|final --scaffold <report.md> --sources <sources.csv|sources.tsv|sources.json> --output <sok-report.json> [--evidence <evidence.jsonl>]

The --scaffold flag is a path alias for --report.
Machine lane sidecars are optional: --knowledge <sok-knowledge.json>, --evidence-package <sok-evidence.json>, and --pedagogy <sok-pedagogy.json>.
The stage is always explicit; filenames and headings do not select the public/internal contract.
Public H2 sections are preserved in order. Canonical headings or sok:surface markers identify machine-verifiable tables."#
    );
}

pub(crate) fn run_lint(args: &[String]) -> Result<i32> {
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

pub(crate) fn print_lint_usage() {
    println!(
        r#"Usage:
  sok lint --stage scaffold|final --report <report.md> --sources <sources.csv|sources.tsv|sources.json> [--evidence <evidence.jsonl>] [--strict]
  sok lint --stage scaffold|final --scaffold <report.md> --sources <sources.csv|sources.tsv|sources.json> [--evidence <evidence.jsonl>] [--strict]

Checks bounded Markdown, source manifests, and optional evidence ledgers before JSON export.
The --scaffold flag is a path alias for --report.
Default mode returns nonzero for errors. --strict also returns nonzero for warnings."#
    );
}

pub(crate) fn run_validate_report(args: &[String]) -> Result<i32> {
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

pub(crate) fn run_migrate_ids(args: &[String]) -> Result<()> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_migrate_ids_usage();
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

    let document: report::ReportDocument = report::read_json_file(&input)?;
    if !matches!(
        document.metadata.schema_version.as_str(),
        "sok-report/v1" | "sok-report/v2"
    ) {
        bail!(
            "migrate-ids supports sok-report/v1 and sok-report/v2, got {:?}",
            document.metadata.schema_version
        );
    }
    let migration = report::build_id_migration(&document);
    report::write_json_file(&output, &migration)?;
    println!("Wrote {output}");
    Ok(())
}

pub(crate) fn print_migrate_ids_usage() {
    println!(
        r#"Usage:
  sok migrate-ids --input <sok-report.json> --output <id-map.json>

Reads a sok-report/v1 or sok-report/v2 artifact and writes a deterministic old-to-new ID migration map.
The input report is never rewritten in place."#
    );
}

pub(crate) fn print_validate_report_usage() {
    println!(
        r#"Usage:
  sok validate-report --input <sok-report.json> [--strict]

Validates the JSON-first SoK report contract and reliability gates.
Default mode returns nonzero for errors. --strict also returns nonzero for warnings."#
    );
}

pub(crate) fn run_render_html(args: &[String]) -> Result<()> {
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

pub(crate) fn print_render_html_usage() {
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

pub(crate) fn run_audit_sources(args: &[String]) -> Result<()> {
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

pub(crate) fn run_download_sources(args: &[String]) -> Result<()> {
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

pub(crate) fn run_specificity(args: &[String]) -> Result<()> {
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
