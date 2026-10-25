use super::*;

#[test]
fn work_order_validators_cover_supported_task_kinds_and_reject_unknowns() {
    for task_kind in supported_work_task_kinds() {
        let order = test_work_order(task_kind);
        let validation = work::validate_work_order(&order);
        assert!(
            validation.valid,
            "{task_kind:?}: {:?}",
            validation.diagnostics
        );
    }

    let mut unknown_version = test_work_order(work::WorkTaskKind::Framing);
    unknown_version.schema_version = "sok-work-order/v99".to_string();
    let validation = work::validate_work_order(&unknown_version);
    assert_work_check(&validation, work::validation::CHECK_WORK_VERSION);

    let unsupported_task_kind = json!({
        "schema_version": "sok-work-order/v1",
        "work_order_id": "work-order-1111111111",
        "task_kind": "book_generation",
        "objective": "Out of scope task.",
        "allowed_patch_paths": ["/knowledge/elements"]
    });
    assert!(serde_json::from_value::<work::WorkOrder>(unsupported_task_kind).is_err());
}

#[test]
fn work_result_rejects_unknown_versions_and_unsupported_task_kinds() {
    let order = test_work_order(work::WorkTaskKind::FieldElementExtraction);
    let mut result = test_work_result(
        &order,
        vec![valid_element_patch("patch-element-1111111111")],
    );
    let validation = work::validate_work_result(&order, &result);
    assert!(validation.valid, "{:?}", validation.diagnostics);

    result.schema_version = "sok-work-result/v99".to_string();
    let validation = work::validate_work_result(&order, &result);
    assert_work_check(&validation, work::validation::CHECK_WORK_VERSION);

    let unsupported_task_kind = json!({
        "schema_version": "sok-work-result/v1",
        "work_order_id": order.work_order_id,
        "result_id": "work-result-1111111111",
        "task_kind": "book_generation",
        "status": "refusal",
        "capability_response": {
            "status": "refusal",
            "missing_capabilities": ["basic_completion"],
            "rationale": "unsupported"
        }
    });
    assert!(serde_json::from_value::<work::WorkResult>(unsupported_task_kind).is_err());
}

#[test]
fn work_protocol_fixtures_validate_and_apply_against_core_packages() {
    let order: work::WorkOrder = report::read_json_file(repo_path(
        "cli/tests/fixtures/work/work-order-field-element.json",
    ))
    .unwrap();
    let result: work::WorkResult = report::read_json_file(repo_path(
        "cli/tests/fixtures/work/work-result-field-element.json",
    ))
    .unwrap();
    assert!(work::validate_work_order(&order).valid);
    assert!(work::validate_work_result(&order, &result).valid);

    let patched =
        work::accept_work_result_patches(&order, &result, &core_fixture_packages()).unwrap();
    assert!(patched
        .knowledge
        .elements
        .iter()
        .any(|element| element.id == "element-work-fixture-4444444444"));
}

#[test]
fn work_capability_negotiation_validates_match_refusal_and_degradation() {
    let mut order = test_work_order(work::WorkTaskKind::ReportProjection);
    order.required_capabilities = vec![
        work::WorkCapability::BasicCompletion,
        work::WorkCapability::StructuredOutput,
        work::WorkCapability::LongContextAgent,
    ];
    let offered = vec![
        work::WorkCapability::BasicCompletion,
        work::WorkCapability::StructuredOutput,
    ];

    let missing = work::missing_capabilities(&order.required_capabilities, &offered);
    assert_eq!(missing, vec![work::WorkCapability::LongContextAgent]);

    let refusal = work::negotiate_capabilities(
        &order.required_capabilities,
        &offered,
        work::NegotiationMode::Refusal,
    );
    assert_eq!(refusal.status, work::CapabilityResolutionStatus::Refusal);
    let refusal_result = work::WorkResult {
        schema_version: "sok-work-result/v1".to_string(),
        work_order_id: order.work_order_id.clone(),
        result_id: "work-result-refusal-1111111111".to_string(),
        task_kind: order.task_kind,
        status: work::WorkResultStatus::Refusal,
        offered_capabilities: offered.clone(),
        capability_response: refusal,
        patches: Vec::new(),
        notes: String::new(),
    };
    assert!(work::validate_work_result(&order, &refusal_result).valid);

    let degradation = work::negotiate_capabilities(
        &order.required_capabilities,
        &offered,
        work::NegotiationMode::DegradationPlan,
    );
    assert_eq!(
        degradation.status,
        work::CapabilityResolutionStatus::DegradationPlan
    );
    assert!(!degradation.degradation_plan.is_empty());
    let degradation_result = work::WorkResult {
        schema_version: "sok-work-result/v1".to_string(),
        work_order_id: order.work_order_id.clone(),
        result_id: "work-result-degradation-1111111111".to_string(),
        task_kind: order.task_kind,
        status: work::WorkResultStatus::DegradationPlan,
        offered_capabilities: offered.clone(),
        capability_response: degradation,
        patches: Vec::new(),
        notes: String::new(),
    };
    assert!(work::validate_work_result(&order, &degradation_result).valid);

    let mut bad = refusal_result.clone();
    bad.status = work::WorkResultStatus::PatchProposal;
    bad.capability_response = work::CapabilityResponse::accepted();
    bad.patches = vec![valid_element_patch("patch-element-2222222222")];
    let validation = work::validate_work_result(&order, &bad);
    assert_work_check(&validation, work::validation::CHECK_WORK_CAPABILITY);
}

#[test]
fn work_permission_envelopes_default_deny_and_constrain_roots() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.json");
    let output = dir.path().join("output.json");
    let mut order = test_work_order(work::WorkTaskKind::FieldElementExtraction);
    order.input_files = vec![work::WorkFileRef::new(
        "core-packages",
        input.display().to_string(),
    )];
    order.output_files = vec![work::WorkFileRef::new(
        "core-packages",
        output.display().to_string(),
    )];

    let validation = work::validate_work_order(&order);
    assert_work_check(&validation, work::validation::CHECK_WORK_PERMISSION);

    order.permissions.read_roots = vec![dir.path().display().to_string()];
    order.permissions.write_roots = vec![dir.path().display().to_string()];
    let validation = work::validate_work_order(&order);
    assert!(validation.valid, "{:?}", validation.diagnostics);

    let outside = dir.path().join("..").join("outside.json");
    order.output_files = vec![work::WorkFileRef::new(
        "core-packages",
        outside.display().to_string(),
    )];
    let validation = work::validate_work_order(&order);
    assert_work_check(&validation, work::validation::CHECK_WORK_PERMISSION);
}

#[cfg(unix)]
#[test]
fn work_permission_envelopes_reject_symlink_escapes() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_input = outside.path().join("outside-input.json");
    fs::write(&outside_input, "{}\n").unwrap();
    let escape = root.path().join("escape");
    symlink(outside.path(), &escape).unwrap();

    let escaped_input = escape.join("outside-input.json");
    let escaped_output = escape.join("outside-output.json");
    let mut order = test_work_order(work::WorkTaskKind::FieldElementExtraction);
    order.permissions.read_roots = vec![root.path().display().to_string()];
    order.permissions.write_roots = vec![root.path().display().to_string()];
    order.input_files = vec![work::WorkFileRef::new(
        "core-packages",
        escaped_input.display().to_string(),
    )];
    order.output_files = vec![work::WorkFileRef::new(
        "core-packages",
        escaped_output.display().to_string(),
    )];

    let validation = work::validate_work_order(&order);
    assert_work_check(&validation, work::validation::CHECK_WORK_PERMISSION);
    assert!(work::permissions::ensure_read_path(&order.permissions, &escaped_input).is_err());
    assert!(work::permissions::ensure_write_path(&order.permissions, &escaped_output).is_err());

    let outside_child = outside.path().join("child");
    fs::create_dir(&outside_child).unwrap();
    let outside_parent_input = outside.path().join("parent-input.json");
    fs::write(&outside_parent_input, "{}\n").unwrap();
    let jump = root.path().join("jump");
    symlink(&outside_child, &jump).unwrap();
    let parent_escape_input = jump.join("..").join("parent-input.json");
    let parent_escape_output = jump.join("..").join("parent-output.json");

    assert!(work::permissions::ensure_read_path(&order.permissions, &parent_escape_input).is_err());
    assert!(
        work::permissions::ensure_write_path(&order.permissions, &parent_escape_output).is_err()
    );
}

#[test]
fn work_patch_accepts_valid_patches_and_rejects_invalid_paths_versions_and_dangling_refs() {
    let packages = core_fixture_packages();
    let order = test_work_order(work::WorkTaskKind::FieldElementExtraction);
    let result = test_work_result(
        &order,
        vec![valid_element_patch("patch-element-1111111111")],
    );
    let patched = work::accept_work_result_patches(&order, &result, &packages).unwrap();
    assert!(patched
        .knowledge
        .elements
        .iter()
        .any(|element| element.id == "element-command-string-4444444444"));
    assert!(!packages
        .knowledge
        .elements
        .iter()
        .any(|element| element.id == "element-command-string-4444444444"));

    let mut unknown_patch_version = test_work_result(
        &order,
        vec![valid_element_patch("patch-element-version-1111111111")],
    );
    unknown_patch_version.patches[0].schema_version = "sok-work-patch/v99".to_string();
    let validation = work::validate_work_result(&order, &unknown_patch_version);
    assert_work_check(&validation, work::validation::CHECK_WORK_VERSION);

    let undeclared = work::WorkPatch::add(
        "patch-claim-1111111111",
        work::CorePackageArea::Evidence,
        "/evidence/claims/-",
        json!({
            "id": "claim-undeclared-1111111111",
            "statement": "This should not be accepted through a field-element order.",
            "claim_type": "structural",
            "evidence_requirement": "reviewed_source",
            "temporal": {
                "as_of": "2026-07-16",
                "review_after": "2027-01-16",
                "temporal_status": "durable",
                "rationale": "Fixture."
            }
        }),
    );
    let undeclared_result = test_work_result(&order, vec![undeclared]);
    let validation = work::validate_work_result(&order, &undeclared_result);
    assert_work_check(&validation, work::validation::CHECK_WORK_PATCH);

    let mut unsupported_operation = valid_element_patch("patch-operation-1111111111");
    unsupported_operation.op = work::PatchOperation::Replace;
    unsupported_operation.path = "/knowledge/elements".to_string();
    let validation = work::validate_work_result(
        &order,
        &test_work_result(&order, vec![unsupported_operation]),
    );
    assert_work_check(&validation, work::validation::CHECK_WORK_PATCH);

    let mut bad_shape = valid_element_patch("patch-shape-1111111111");
    bad_shape.value = json!({
        "id": "element-command-string-4444444444"
    });
    let validation = work::validate_work_result(&order, &test_work_result(&order, vec![bad_shape]));
    assert_work_check(&validation, work::validation::CHECK_WORK_PATCH);

    let relation_order = test_work_order(work::WorkTaskKind::RelationProposals);
    let dangling_relation = work::WorkPatch::add(
        "patch-relation-dangling-1111111111",
        work::CorePackageArea::Knowledge,
        "/knowledge/relations/-",
        json!({
            "id": "rel-dangling-1111111111",
            "kind": "requires_before",
            "from": {
                "entity_type": "field_element",
                "id": "element-missing-9999999999"
            },
            "to": {
                "entity_type": "field_element",
                "id": "element-calibration-transfer-2222222222"
            }
        }),
    );
    let dangling_result = test_work_result(&relation_order, vec![dangling_relation]);
    assert_contains(
        &work::accept_work_result_patches(&relation_order, &dangling_result, &packages)
            .unwrap_err()
            .to_string(),
        "patched core packages failed validation",
    );

    let legacy_relation_kind = work::WorkPatch::add(
        "patch-relation-legacy-1111111111",
        work::CorePackageArea::Knowledge,
        "/knowledge/relations/-",
        json!({
            "id": "rel-legacy-kind-1111111111",
            "kind": "depends_on",
            "from": {
                "entity_type": "field_element",
                "id": "element-calibration-transfer-2222222222"
            },
            "to": {
                "entity_type": "field_element",
                "id": "element-noise-model-3333333333"
            }
        }),
    );
    let validation = work::validate_work_result(
        &relation_order,
        &test_work_result(&relation_order, vec![legacy_relation_kind]),
    );
    assert_work_check(&validation, work::validation::CHECK_WORK_PATCH);
}

#[test]
fn work_accept_patch_cli_writes_new_validated_output_without_mutating_input() {
    let dir = tempfile::tempdir().unwrap();
    let input_path = dir.path().join("core-input.json");
    let output_path = dir.path().join("core-output.json");
    let order_path = dir.path().join("order.json");
    let result_path = dir.path().join("result.json");
    let packages = core_fixture_packages();
    report::write_json_file(&input_path, &packages).unwrap();
    let input_before = fs::read_to_string(&input_path).unwrap();

    let mut order = test_work_order(work::WorkTaskKind::FieldElementExtraction);
    order.permissions.read_roots = vec![dir.path().display().to_string()];
    order.permissions.write_roots = vec![dir.path().display().to_string()];
    order.input_files = vec![work::WorkFileRef::new(
        "core-packages",
        input_path.display().to_string(),
    )];
    order.output_files = vec![work::WorkFileRef::new(
        "core-packages",
        output_path.display().to_string(),
    )];
    let result = test_work_result(
        &order,
        vec![valid_element_patch("patch-element-3333333333")],
    );
    report::write_json_file(&order_path, &order).unwrap();
    report::write_json_file(&result_path, &result).unwrap();

    assert_eq!(
        run_cli(vec![
            "work".to_string(),
            "accept-patch".to_string(),
            "--order".to_string(),
            order_path.display().to_string(),
            "--result".to_string(),
            result_path.display().to_string(),
            "--input".to_string(),
            input_path.display().to_string(),
            "--output".to_string(),
            output_path.display().to_string(),
        ])
        .unwrap(),
        0
    );
    assert_eq!(fs::read_to_string(&input_path).unwrap(), input_before);
    let patched: crate::core::CorePackages = report::read_json_file(&output_path).unwrap();
    assert!(patched
        .knowledge
        .elements
        .iter()
        .any(|element| element.id == "element-command-string-4444444444"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let alias_path = dir.path().join("core-input-alias.json");
        symlink(&input_path, &alias_path).unwrap();
        let err = run_cli(vec![
            "work".to_string(),
            "accept-patch".to_string(),
            "--order".to_string(),
            order_path.display().to_string(),
            "--result".to_string(),
            result_path.display().to_string(),
            "--input".to_string(),
            input_path.display().to_string(),
            "--output".to_string(),
            alias_path.display().to_string(),
        ])
        .unwrap_err();
        assert_contains(
            &err.to_string(),
            "--output must be a new file; refusing to mutate --input in place",
        );
        assert_eq!(fs::read_to_string(&input_path).unwrap(), input_before);
    }
}

#[test]
fn work_jsonl_batch_validation_reports_each_line_without_provider_calls() {
    let order = test_work_order(work::WorkTaskKind::FieldElementExtraction);
    let mut bad_order = order.clone();
    bad_order.schema_version = "sok-work-order/v99".to_string();
    let input = format!(
        "{}\n{}\n",
        serde_json::to_string(&order).unwrap(),
        serde_json::to_string(&bad_order).unwrap()
    );
    let (output, all_valid) = work::validate_work_order_jsonl(&input).unwrap();
    assert!(!all_valid);
    let rows = output
        .lines()
        .map(|line| serde_json::from_str::<work::WorkJsonlValidationLine>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].valid);
    assert!(!rows[1].valid);
    assert_eq!(rows[1].line, 2);

    let good_result = test_work_result(
        &order,
        vec![valid_element_patch("patch-element-5555555555")],
    );
    let mut bad_result = good_result.clone();
    bad_result.capability_response = work::CapabilityResponse::accepted();
    bad_result.offered_capabilities.clear();
    let input = format!(
        "{}\n{}\n",
        serde_json::to_string(&good_result).unwrap(),
        serde_json::to_string(&bad_result).unwrap()
    );
    let (output, all_valid) = work::validate_work_result_jsonl(&order, &input).unwrap();
    assert!(!all_valid);
    let rows = output
        .lines()
        .map(|line| serde_json::from_str::<work::WorkJsonlValidationLine>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].valid);
    assert!(!rows[1].valid);
}

#[test]
fn work_patch_shell_like_strings_remain_data() {
    let dir = tempfile::tempdir().unwrap();
    let sentinel = dir.path().join("shell-sentinel");
    let packages = core_fixture_packages();
    let order = test_work_order(work::WorkTaskKind::FieldElementExtraction);
    let mut patch = valid_element_patch("patch-element-shell-1111111111");
    patch.value["actual_form"]["text"] = json!(format!("sh -c 'touch {}'", sentinel.display()));
    let result = test_work_result(&order, vec![patch]);
    let patched = work::accept_work_result_patches(&order, &result, &packages).unwrap();
    assert!(patched.knowledge.elements.iter().any(|element| element
        .actual_form
        .text
        .contains(&sentinel.display().to_string())));
    assert!(!sentinel.exists());
}
