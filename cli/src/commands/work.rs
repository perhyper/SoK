use super::*;

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
