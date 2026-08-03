//! Command implementations for the Rust SoK CLI.

use anyhow::{anyhow, bail, Context, Result};
use reqwest::blocking::Client;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::downloader::*;
use crate::eval;
use crate::report;
use crate::run_manifest::{self, RunFile, RunManifestSpec, SemanticPayload};
use crate::scaffold::*;
use crate::sources::*;
use crate::work;

pub(crate) fn run_init(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &[])?;
    let field = flags.string("field", "");
    let learner = flags.string("learner", "scholar new to the field");
    let goal = flags.string("goal", "build field-entry to research-fluent understanding");
    let mode = flags.string("mode", "research");
    let output_format = flags.string("output-format", "markdown");
    let weeks = flags.int("weeks", 0)?;
    let out = flags.string("out", "");
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();

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
    let output_files = vec![
        run_file("agent-pack", out_dir.join("sok-agent-pack.json")),
        run_file("agent-brief", out_dir.join("agent-brief.md")),
        run_file("tasks", out_dir.join("tasks.md")),
        run_file("report-scaffold", out_dir.join("report.md")),
        run_file("source-manifest", out_dir.join("sources.csv")),
    ];
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "init",
        args,
        stage: "init",
        profile_version: run_manifest::default_profile_version_for_field(&field),
        started_at,
        inputs: Vec::new(),
        outputs: output_files,
        semantic_payloads: Vec::new(),
        network: false,
    })?;
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
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();

    require_field(&field)?;
    validate_mode(&mode)?;
    let content = build_agent_brief(&field, &learner, &goal, &mode, weeks, &output_format);
    if output.is_empty() {
        print!("{content}");
    } else {
        write_text(&output, &content)?;
        println!("Wrote {output}");
    }
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "brief",
        args,
        stage: "brief",
        profile_version: run_manifest::default_profile_version_for_field(&field),
        started_at,
        inputs: Vec::new(),
        outputs: output_file("brief", &output),
        semantic_payloads: vec![SemanticPayload::from_text(
            "brief",
            "stdout-or-file",
            &content,
        )],
        network: false,
    })?;
    Ok(())
}

pub(crate) fn run_scaffold(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &[])?;
    let field = flags.string("field", "");
    let learner = flags.string("learner", "scholar new to the field");
    let goal = flags.string("goal", "build field-entry to research-fluent understanding");
    let output = flags.string("output", "");
    let weeks = flags.int("weeks", 0)?;
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();

    require_field(&field)?;
    let content = build_report_scaffold(&field, &learner, &goal, weeks);
    if output.is_empty() {
        print!("{content}");
    } else {
        write_text(&output, &content)?;
        println!("Wrote {output}");
    }
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "scaffold",
        args,
        stage: "scaffold",
        profile_version: run_manifest::default_profile_version_for_field(&field),
        started_at,
        inputs: Vec::new(),
        outputs: output_file("report-scaffold", &output),
        semantic_payloads: vec![SemanticPayload::from_text(
            "report-scaffold",
            "stdout-or-file",
            &content,
        )],
        network: false,
    })?;
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
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();

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
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "handoff-report",
        args,
        stage: "handoff-report",
        profile_version: run_manifest::default_profile_version_for_field(&field),
        started_at,
        inputs: input_file("scaffold", &scaffold_path),
        outputs: output_file("handoff-report", &output),
        semantic_payloads: vec![SemanticPayload::from_text(
            "handoff-report",
            "stdout-or-file",
            &content,
        )],
        network: false,
    })?;
    Ok(())
}

pub(crate) fn run_source_template(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &[])?;
    let output = flags.string("output", "");
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();
    if output.is_empty() {
        bail!("--output is required");
    }
    write_sources_csv(&output)?;
    println!("Wrote {output}");
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "source-template",
        args,
        stage: "source-template",
        profile_version: "unknown".to_string(),
        started_at,
        inputs: Vec::new(),
        outputs: output_file("source-manifest-template", &output),
        semantic_payloads: Vec::new(),
        network: false,
    })?;
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
  sok ingest last --sources <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl> [--run-manifest <sok-run.json>]

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
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();

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
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "ingest last",
        args,
        stage: "ingest",
        profile_version: "unknown".to_string(),
        started_at,
        inputs: input_file("source-manifest", &input),
        outputs: output_file("evidence-jsonl", &output),
        semantic_payloads: Vec::new(),
        network: false,
    })?;
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
  sok ingest last --sources <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl> [--run-manifest <sok-run.json>]
  sok ingest last --manifest <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl> [--run-manifest <sok-run.json>]

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
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();

    let stage = parse_export_stage(&stage)?;
    let stage_label = export_stage_label(stage);
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
    let mut inputs = vec![
        run_file("markdown-report", report_input),
        run_file("source-manifest", sources.as_str()),
    ];
    inputs.extend(input_file("evidence-jsonl", &evidence));
    inputs.extend(input_file("sok-knowledge", &knowledge));
    inputs.extend(input_file("sok-evidence", &evidence_package));
    inputs.extend(input_file("sok-pedagogy", &pedagogy));
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "export-json",
        args,
        stage: stage_label,
        profile_version: run_manifest::profile_version_from_report(&document),
        started_at,
        inputs,
        outputs: output_file("sok-report", &output),
        semantic_payloads: vec![SemanticPayload::from_report_document(
            "sok-report",
            &document,
        )?],
        network: false,
    })?;
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
  sok export-json --stage scaffold|final --report <report.md> --sources <sources.csv|sources.tsv|sources.json> --output <sok-report.json> [--evidence <evidence.jsonl>] [--run-manifest <sok-run.json>]
  sok export-json --stage scaffold|final --scaffold <report.md> --sources <sources.csv|sources.tsv|sources.json> --output <sok-report.json> [--evidence <evidence.jsonl>] [--run-manifest <sok-run.json>]

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
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();
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
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "migrate-ids",
        args,
        stage: "migrate-ids",
        profile_version: "unknown".to_string(),
        started_at,
        inputs: input_file("sok-report", &input),
        outputs: output_file("id-migration-map", &output),
        semantic_payloads: vec![SemanticPayload::from_json_value(
            "id-migration-map",
            "id-migration-map",
            &serde_json::to_value(&migration)?,
        )?],
        network: false,
    })?;
    Ok(())
}

pub(crate) fn print_migrate_ids_usage() {
    println!(
        r#"Usage:
  sok migrate-ids --input <sok-report.json> --output <id-map.json> [--run-manifest <sok-run.json>]

Reads a sok-report/v1 or sok-report/v2 artifact and writes a deterministic old-to-new ID migration map.
The input report is never rewritten in place."#
    );
}

pub(crate) fn run_eval(args: &[String]) -> Result<i32> {
    let Some(command) = args.first() else {
        print_eval_usage();
        return Ok(2);
    };
    if matches!(command.as_str(), "-h" | "--help" | "help") {
        print_eval_usage();
        return Ok(0);
    }
    match command.as_str() {
        "conformance" => run_eval_conformance(&args[1..]),
        command => bail!("unknown eval command {command:?}"),
    }
}

pub(crate) fn print_eval_usage() {
    println!(
        r#"SoK eval commands

Usage:
  sok eval conformance --fixtures <directory> [--output <sok-conformance-report.json>]

The conformance scorer reads saved provider-neutral fixtures and runs deterministic validators only.
It does not call model providers, browse the network, or repair artifacts in place."#
    );
}

fn run_eval_conformance(args: &[String]) -> Result<i32> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_eval_usage();
        return Ok(0);
    }
    let flags = parse_flags(args, &[])?;
    let fixtures = flags.string("fixtures", "");
    let output = flags.string("output", "");
    if fixtures.trim().is_empty() {
        bail!("--fixtures is required");
    }

    let report = eval::score_conformance_dir(&fixtures)?;
    if output.trim().is_empty() {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        report::write_json_file(&output, &report)?;
        println!("Wrote {output}");
        println!(
            "Conformance: {} expectation(s) passed, {} failed",
            report.expectation_passed, report.expectation_failed
        );
        println!("Validation pass rate: {}", report.validation_pass_rate);
        println!("Validation errors: {}", report.validation_error_count);
        println!("Repairs recorded: {}", report.repair_count);
        println!(
            "Human corrections recorded: {}",
            report.human_correction_count
        );
    }
    Ok(if report.all_expectations_passed() {
        0
    } else {
        1
    })
}

pub(crate) fn print_validate_report_usage() {
    println!(
        r#"Usage:
  sok validate-report --input <sok-report.json> [--strict]

Validates the JSON-first SoK report contract and reliability gates.
Default mode returns nonzero for errors. --strict also returns nonzero for warnings."#
    );
}

pub(crate) fn run_work(args: &[String]) -> Result<i32> {
    let Some(command) = args.first() else {
        print_work_usage();
        return Ok(2);
    };
    if matches!(command.as_str(), "-h" | "--help" | "help") {
        print_work_usage();
        return Ok(0);
    }
    match command.as_str() {
        "create-order" => run_work_create_order(&args[1..]).map(|_| 0),
        "validate-order" => run_work_validate_order(&args[1..]),
        "validate-result" => run_work_validate_result(&args[1..]),
        "negotiate" => run_work_negotiate(&args[1..]).map(|_| 0),
        "accept-patch" => run_work_accept_patch(&args[1..]).map(|_| 0),
        command => bail!("unknown work command {command:?}"),
    }
}

pub(crate) fn print_work_usage() {
    println!(
        r#"SoK work commands

Usage:
  sok work create-order --work-order-id <id> --task-kind <kind> --objective <text> --output <sok-work-order.json> [--required-capabilities <comma-list>] [--allowed-patch-paths <comma-list>] [--read-roots <comma-list>] [--write-roots <comma-list>] [--input-files <comma-list>] [--output-files <comma-list>]
  sok work validate-order --input <sok-work-order.json>
  sok work validate-order --jsonl
  sok work validate-result --order <sok-work-order.json> --input <sok-work-result.json>
  sok work validate-result --order <sok-work-order.json> --jsonl
  sok work negotiate --order <sok-work-order.json> --capabilities <comma-list> [--degrade] [--output <sok-capability-response.json>]
  sok work accept-patch --order <sok-work-order.json> --result <sok-work-result.json> --input <core-packages.json> --output <core-packages.json> [--run-manifest <sok-run.json>]

Supported task kinds: framing, source_role_classification, field_element_extraction, claim_proposals, relation_proposals, curriculum_prerequisite_proposals, architecture_comparison, consistency_critique, report_projection.
Supported capabilities: basic_completion, structured_output, tool_capable, long_context_agent.
Work results are accepted only as proposed patches against core packages; source inputs are never mutated in place."#
    );
}

fn run_work_create_order(args: &[String]) -> Result<()> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_work_usage();
        return Ok(());
    }
    let flags = parse_flags(args, &["network", "subprocess"])?;
    let work_order_id = flags.string("work-order-id", "");
    let task_kind = parse_work_task_kind(&flags.string("task-kind", ""))?;
    let objective = flags.string("objective", "");
    let output = flags.string("output", "");
    if work_order_id.trim().is_empty() {
        bail!("--work-order-id is required");
    }
    if objective.trim().is_empty() {
        bail!("--objective is required");
    }
    if output.trim().is_empty() {
        bail!("--output is required");
    }

    let mut order = work::WorkOrder::new(work_order_id, task_kind, objective);
    order.instructions = flags.string("instructions", "");
    order.source_text = flags.string("source-text", "");
    if !flags.string("required-capabilities", "").trim().is_empty() {
        order.required_capabilities =
            parse_work_capabilities(&flags.string("required-capabilities", ""))?;
    }
    if !flags.string("allowed-patch-paths", "").trim().is_empty() {
        order.allowed_patch_paths = comma_values(&flags.string("allowed-patch-paths", ""));
    }
    order.permissions = work::PermissionEnvelope {
        network: flags.bool("network"),
        subprocess: flags.bool("subprocess"),
        read_roots: comma_values(&flags.string("read-roots", "")),
        write_roots: comma_values(&flags.string("write-roots", "")),
    };
    order.input_files = comma_values(&flags.string("input-files", ""))
        .into_iter()
        .map(|path| work::WorkFileRef::new("bounded-input", path))
        .collect();
    order.output_files = comma_values(&flags.string("output-files", ""))
        .into_iter()
        .map(|path| work::WorkFileRef::new("bounded-output", path))
        .collect();

    let validation = work::validate_work_order(&order);
    if validation.has_errors() {
        bail!(
            "work order failed validation: {}",
            validation.diagnostics.summary
        );
    }
    report::write_json_file(&output, &order)?;
    println!("Wrote {output}");
    Ok(())
}

fn run_work_validate_order(args: &[String]) -> Result<i32> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_work_usage();
        return Ok(0);
    }
    let flags = parse_flags(args, &["jsonl"])?;
    if flags.bool("jsonl") {
        let input = read_stdin()?;
        let (output, all_valid) = work::validate_work_order_jsonl(&input)?;
        print!("{output}");
        return Ok(if all_valid { 0 } else { 1 });
    }
    let input = flags.string("input", "");
    if input.trim().is_empty() {
        bail!("--input is required");
    }
    let order: work::WorkOrder = report::read_json_file(&input)?;
    let validation = work::validate_work_order(&order);
    print_work_validation(&validation);
    Ok(if validation.has_errors() { 1 } else { 0 })
}

fn run_work_validate_result(args: &[String]) -> Result<i32> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_work_usage();
        return Ok(0);
    }
    let flags = parse_flags(args, &["jsonl"])?;
    let order_path = flags.string("order", "");
    if order_path.trim().is_empty() {
        bail!("--order is required");
    }
    let order: work::WorkOrder = report::read_json_file(&order_path)?;
    if flags.bool("jsonl") {
        let input = read_stdin()?;
        let (output, all_valid) = work::validate_work_result_jsonl(&order, &input)?;
        print!("{output}");
        return Ok(if all_valid { 0 } else { 1 });
    }
    let input = flags.string("input", "");
    if input.trim().is_empty() {
        bail!("--input is required");
    }
    let result: work::WorkResult = report::read_json_file(&input)?;
    let validation = work::validate_work_result(&order, &result);
    print_work_validation(&validation);
    Ok(if validation.has_errors() { 1 } else { 0 })
}

fn run_work_negotiate(args: &[String]) -> Result<()> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_work_usage();
        return Ok(());
    }
    let flags = parse_flags(args, &["degrade"])?;
    let order_path = flags.string("order", "");
    let capabilities = flags.string("capabilities", "");
    let output = flags.string("output", "");
    if order_path.trim().is_empty() {
        bail!("--order is required");
    }
    let order: work::WorkOrder = report::read_json_file(&order_path)?;
    let validation = work::validate_work_order(&order);
    if validation.has_errors() {
        bail!(
            "work order failed validation: {}",
            validation.diagnostics.summary
        );
    }
    let offered = parse_work_capabilities(&capabilities)?;
    let mode = if flags.bool("degrade") {
        work::NegotiationMode::DegradationPlan
    } else {
        work::NegotiationMode::Refusal
    };
    let response = work::negotiate_capabilities(&order.required_capabilities, &offered, mode);
    if output.trim().is_empty() {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        report::write_json_file(&output, &response)?;
        println!("Wrote {output}");
    }
    Ok(())
}

fn run_work_accept_patch(args: &[String]) -> Result<()> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_work_usage();
        return Ok(());
    }
    let flags = parse_flags(args, &[])?;
    let order_path = flags.string("order", "");
    let result_path = flags.string("result", "");
    let input = flags.string("input", "");
    let output = flags.string("output", "");
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();
    if order_path.trim().is_empty() {
        bail!("--order is required");
    }
    if result_path.trim().is_empty() {
        bail!("--result is required");
    }
    if input.trim().is_empty() {
        bail!("--input is required");
    }
    if output.trim().is_empty() {
        bail!("--output is required");
    }
    ensure_distinct_paths(&input, &output)?;

    let order: work::WorkOrder = report::read_json_file(&order_path)?;
    work::permissions::ensure_read_path(&order.permissions, &input)?;
    work::permissions::ensure_read_path(&order.permissions, &result_path)?;
    work::permissions::ensure_write_path(&order.permissions, &output)?;
    if !run_manifest_path.trim().is_empty() {
        work::permissions::ensure_write_path(&order.permissions, &run_manifest_path)?;
    }
    let result: work::WorkResult = report::read_json_file(&result_path)?;
    let packages: crate::core::CorePackages = report::read_json_file(&input)?;
    let patched = work::accept_work_result_patches(&order, &result, &packages)?;
    report::write_json_file(&output, &patched)?;
    println!("Wrote {output}");
    write_work_accept_run_manifest(WorkAcceptRunManifest {
        path: &run_manifest_path,
        args,
        started_at,
        order_path: &order_path,
        result_path: &result_path,
        input: &input,
        output: &output,
        permissions: &order.permissions,
        patched: &patched,
    })?;
    Ok(())
}

struct WorkAcceptRunManifest<'a> {
    path: &'a str,
    args: &'a [String],
    started_at: String,
    order_path: &'a str,
    result_path: &'a str,
    input: &'a str,
    output: &'a str,
    permissions: &'a work::PermissionEnvelope,
    patched: &'a crate::core::CorePackages,
}

fn write_work_accept_run_manifest(request: WorkAcceptRunManifest<'_>) -> Result<()> {
    if request.path.trim().is_empty() {
        return Ok(());
    }
    let mut spec = RunManifestSpec::new("work accept-patch");
    spec.command_args = request.args.to_vec();
    spec.stage = "work-accept-patch".to_string();
    spec.profile_version = "unknown".to_string();
    spec.started_at = request.started_at;
    spec.finished_at = run_manifest::timestamp_now();
    spec.declared_permissions = run_manifest::DeclaredPermissions {
        network: request.permissions.network,
        subprocess: request.permissions.subprocess,
        read_roots: request.permissions.read_roots.clone(),
        write_roots: request.permissions.write_roots.clone(),
    };
    spec.bounded_input_files = input_file("core-packages", request.input);
    spec.work_order_files = input_file("sok-work-order", request.order_path);
    spec.work_result_files = input_file("sok-work-result", request.result_path);
    spec.output_files = output_file("core-packages", request.output);
    spec.semantic_payloads = vec![SemanticPayload::from_json_value(
        "core-packages",
        "patched-core-packages",
        &serde_json::to_value(request.patched)?,
    )?];
    let manifest = run_manifest::build_run_manifest(spec)?;
    run_manifest::write_run_manifest(request.path, &manifest)?;
    println!("Wrote {}", request.path);
    Ok(())
}

fn print_work_validation(validation: &work::WorkValidationReport) {
    println!("{}", validation.diagnostics.summary);
    if validation.diagnostics.checks.is_empty() {
        println!("No work validation diagnostics.");
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
}

fn parse_work_task_kind(raw: &str) -> Result<work::WorkTaskKind> {
    if raw.trim().is_empty() {
        bail!("--task-kind is required");
    }
    serde_json::from_value(serde_json::Value::String(raw.trim().to_string()))
        .with_context(|| format!("invalid --task-kind {raw:?}"))
}

fn parse_work_capabilities(raw: &str) -> Result<Vec<work::WorkCapability>> {
    let mut capabilities = Vec::new();
    for value in comma_values(raw) {
        let capability = serde_json::from_value(serde_json::Value::String(value.clone()))
            .with_context(|| format!("invalid capability {value:?}"))?;
        capabilities.push(capability);
    }
    Ok(capabilities)
}

fn comma_values(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn read_stdin() -> Result<String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .context("read standard input")?;
    Ok(input)
}

fn ensure_distinct_paths(input: &str, output: &str) -> Result<()> {
    let input = work::permissions::absolute_normalized(Path::new(input))?;
    let output = work::permissions::absolute_normalized(Path::new(output))?;
    if input == output {
        bail!("--output must be a new file; refusing to mutate --input in place");
    }
    Ok(())
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
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();
    if input.is_empty() {
        bail!("--input is required");
    }
    if output.is_empty() {
        bail!("--output is required");
    }

    report::render_html_report_file(&input, &output)?;
    println!("Wrote {output}");
    let document: report::ReportDocument = report::read_json_file(&input)?;
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "render-html",
        args,
        stage: "render-html",
        profile_version: run_manifest::profile_version_from_report(&document),
        started_at,
        inputs: input_file("sok-report", &input),
        outputs: output_file("html-report", &output),
        semantic_payloads: vec![SemanticPayload::from_report_document(
            "rendered-sok-report",
            &document,
        )?],
        network: false,
    })?;
    Ok(())
}

pub(crate) fn print_render_html_usage() {
    println!(
        r#"Usage:
  sok render-html --input <sok-report.json> --output <report.html> [--run-manifest <sok-run.json>]

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

fn run_file(kind: &str, path: impl Into<PathBuf>) -> RunFile {
    RunFile::new(kind, path)
}

fn input_file(kind: &str, path: &str) -> Vec<RunFile> {
    if path.trim().is_empty() {
        Vec::new()
    } else {
        vec![run_file(kind, path)]
    }
}

fn output_file(kind: &str, path: &str) -> Vec<RunFile> {
    if path.trim().is_empty() {
        Vec::new()
    } else {
        vec![run_file(kind, path)]
    }
}

struct OptionalRunManifest<'a> {
    path: &'a str,
    command: &'a str,
    args: &'a [String],
    stage: &'a str,
    profile_version: String,
    started_at: String,
    inputs: Vec<RunFile>,
    outputs: Vec<RunFile>,
    semantic_payloads: Vec<SemanticPayload>,
    network: bool,
}

fn write_optional_run_manifest(request: OptionalRunManifest<'_>) -> Result<()> {
    if request.path.trim().is_empty() {
        return Ok(());
    }

    let mut spec = RunManifestSpec::new(request.command);
    spec.command_args = request.args.to_vec();
    spec.stage = request.stage.to_string();
    spec.profile_version = request.profile_version;
    spec.started_at = request.started_at;
    spec.finished_at = run_manifest::timestamp_now();
    spec.declared_permissions = run_manifest::declared_permissions_for_files(
        &request.inputs,
        &request.outputs,
        request.network,
    );
    spec.bounded_input_files = request.inputs;
    spec.output_files = request.outputs;
    spec.semantic_payloads = request.semantic_payloads;
    let manifest = run_manifest::build_run_manifest(spec)?;
    run_manifest::write_run_manifest(request.path, &manifest)?;
    println!("Wrote {}", request.path);
    Ok(())
}

fn export_stage_label(stage: report::ExportStage) -> &'static str {
    match stage {
        report::ExportStage::Scaffold => "scaffold",
        report::ExportStage::Final => "final",
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
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();

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
    let mut downloaded_outputs = Vec::new();

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
                if !record.path.trim().is_empty() {
                    downloaded_outputs.push(run_file("downloaded-source", record.path.as_str()));
                }
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
    let mut output_files = vec![run_file("download-log", log_path)];
    output_files.extend(downloaded_outputs);
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "download-sources",
        args,
        stage: if dry_run {
            "download-dry-run"
        } else {
            "download"
        },
        profile_version: "unknown".to_string(),
        started_at,
        inputs: input_file("source-manifest", &manifest),
        outputs: output_files,
        semantic_payloads: Vec::new(),
        network: !dry_run,
    })?;
    Ok(())
}

pub(crate) fn run_specificity(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &[])?;
    let field = flags.string("field", "");
    let output = flags.string("output", "");
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();
    require_field(&field)?;
    let content = build_specificity_checklist(&field);
    if output.is_empty() {
        print!("{content}");
    } else {
        write_text(&output, &content)?;
        println!("Wrote {output}");
    }
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "specificity",
        args,
        stage: "specificity",
        profile_version: run_manifest::default_profile_version_for_field(&field),
        started_at,
        inputs: Vec::new(),
        outputs: output_file("specificity-checklist", &output),
        semantic_payloads: vec![SemanticPayload::from_text(
            "specificity-checklist",
            "stdout-or-file",
            &content,
        )],
        network: false,
    })?;
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
