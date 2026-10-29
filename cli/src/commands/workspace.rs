use super::*;

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
