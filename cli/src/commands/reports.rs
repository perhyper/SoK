use super::*;

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

pub(super) fn parse_export_stage(stage: &str) -> Result<report::ExportStage> {
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

pub(crate) fn run_export_graph(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &[])?;
    let input = flags.string("input", "");
    let output = flags.string("output", "");
    if input.is_empty() || output.is_empty() {
        bail!("--input and --output are required");
    }
    crate::graph_snapshot::export(Path::new(&input), Path::new(&output))
}
