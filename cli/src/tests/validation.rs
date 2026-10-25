use super::*;

#[test]
fn validate_report_fixture_passes_without_diagnostics() {
    let final_report = current_human_report_value();
    let validation = report::validate_report_value(&final_report);
    assert_eq!(validation.error_count(), 0);
    assert_eq!(validation.warning_count(), 0);
    assert!(validation.diagnostics.checks.is_empty());

    let dir = tempfile::tempdir().unwrap();
    let input_path = dir.path().join("current-v2-report.json");
    report::write_json_file(&input_path, &final_report).unwrap();
    let exit_code = run_cli(vec![
        "validate-report".to_string(),
        "--input".to_string(),
        input_path.display().to_string(),
        "--strict".to_string(),
    ])
    .unwrap();
    assert_eq!(exit_code, 0);
}

#[test]
fn validate_report_schema_and_public_boundary_errors_are_stable() {
    let final_report = current_human_report_value();

    let mut missing = final_report.clone();
    missing["report"].as_object_mut().unwrap().remove("field");
    let validation = report::validate_report_value(&missing);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SCHEMA_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut leaked = final_report.clone();
    leaked["report"]["raw_learner_profile"] = json!("SENTINEL_INTERNAL learner");
    leaked["report"]["core_ideas"][0]["description"] =
        json!("validation diagnostic check_id should never be public");
    let validation = report::validate_report_value(&leaked);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_PUBLIC_BOUNDARY,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_evidence_requirements_use_structured_fields() {
    let mut report_value = current_human_report_value();
    let ligo_claim_index =
        report_item_index_containing(&report_value, "claims", "statement", "LIGO");
    let ligo_source_id = report_item_id(
        &report_value,
        "sources",
        "title",
        "Broadband Quantum Enhancement of the LIGO Detectors",
    );
    report_value["report"]["claims"][ligo_claim_index]["evidence_links"] = json!([
        {
            "source_id": ligo_source_id,
            "verification_status": "cataloged",
            "support_kind": "supports",
            "locator": "detector configuration",
            "support_note": "Cataloged links cannot satisfy a reviewed-source requirement."
        }
    ]);
    let validation = report::validate_report_value(&report_value);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut unsupported = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &unsupported,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    let supporting_link_index = array_item_index(
        &unsupported["report"]["claims"][chain_claim_index]["evidence_links"],
        "support_kind",
        "supports",
    );
    unsupported["report"]["claims"][chain_claim_index]["evidence_links"][supporting_link_index]
        ["locator"] = json!("");
    unsupported["report"]["claims"][chain_claim_index]["evidence_links"][supporting_link_index]
        ["support_note"] = json!("");
    let validation = report::validate_report_value(&unsupported);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SUPPORT,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_source_role_coverage_uses_explicit_requirements_and_waivers() {
    let mut required_gap = current_human_report_value();
    required_gap["report"]["evidence_standards"]["source_role_requirements"] = json!([
        {
            "role": "foundation",
            "requirement": "required",
            "minimum_sources": 2,
            "rationale": "Two independent foundation sources are required for this validation probe."
        }
    ]);
    let validation = report::validate_report_value(&required_gap);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SOURCE_ROLE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut missing_waiver = current_human_report_value();
    missing_waiver["report"]["evidence_standards"]["source_role_requirements"] = json!([
        {
            "role": "debate",
            "requirement": "waived",
            "minimum_sources": 0,
            "rationale": "No debate source is needed for the stable measurement-chain fixture."
        }
    ]);
    let validation = report::validate_report_value(&missing_waiver);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SOURCE_ROLE_WAIVER,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_currentness_is_structured_and_prose_is_warning_only() {
    let mut missing_temporal = current_human_report_value();
    let current_claim_index =
        report_item_index_containing(&missing_temporal, "claims", "statement", "Field usefulness");
    missing_temporal["report"]["claims"][current_claim_index]["temporal"]
        .as_object_mut()
        .unwrap()
        .remove("review_after");
    let validation = report::validate_report_value(&missing_temporal);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_CURRENTNESS_METADATA,
        report::DiagnosticSeverity::Error,
    );

    let mut future_source = current_human_report_value();
    let roadmap_source_index = report_item_index(
        &future_source,
        "sources",
        "title",
        "DOE Quantum Information Science Applications Roadmap",
    );
    future_source["report"]["sources"][roadmap_source_index]["date"] = json!("2028");
    let validation = report::validate_report_value(&future_source);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_CURRENTNESS_SOURCE_DATE,
        report::DiagnosticSeverity::Error,
    );

    let mut prose = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &prose,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    prose["report"]["claims"][chain_claim_index]["statement"] =
        json!("The latest quantum-sensing account still begins with the measurement chain.");
    prose["report"]["claims"][chain_claim_index]["temporal"]["as_of"] = json!("2026-07-27");
    prose["report"]["claims"][chain_claim_index]["temporal"]["temporal_status"] = json!("unknown");
    let validation = report::validate_report_value(&prose);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_CURRENTNESS_PROSE,
        report::DiagnosticSeverity::Warning,
    );
    assert_eq!(
        validation.error_count(),
        0,
        "unstructured currentness prose should warn without introducing a validation error"
    );
}

#[test]
fn validate_report_relation_and_curriculum_consistency_are_errors() {
    let mut bad_kind = current_human_report_value();
    bad_kind["report"]["relations"][0]["kind"] = json!("unknown_relation");
    let validation = report::validate_report_value(&bad_kind);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_RELATION_KIND,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_endpoint = current_human_report_value();
    bad_endpoint["report"]["relations"][0]["to"]["id"] = json!("concept-missing");
    let validation = report::validate_report_value(&bad_endpoint);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_RELATION_ENDPOINT,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_step = current_human_report_value();
    bad_step["report"]["curriculum_path"][1]["prerequisite_ids"] = json!(["step-missing"]);
    let validation = report::validate_report_value(&bad_step);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_CURRICULUM_REFERENCE,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_source_access_metadata_is_strict_for_restricted_sources() {
    let mut value = current_human_report_value();
    value["report"]["sources"][1]["access"] = json!({
        "status": "paywalled",
        "route": "unknown",
        "budget_estimate": "",
        "license": "",
        "metadata_only": false,
        "notes": ""
    });
    let validation = report::validate_report_value(&value);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SOURCE_ACCESS,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_strict_mode_fails_on_warnings_but_default_does_not() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("warning-only-report.json");
    let mut value = current_human_report_value();
    value["report"]["evidence_standards"]["source_role_requirements"] = json!([
        {
            "role": "critique",
            "requirement": "conditional",
            "minimum_sources": 1,
            "rationale": "A critique source is conditionally useful for this validation probe."
        }
    ]);
    report::write_json_file(&report_path, &value).unwrap();

    let validation = report::validate_report_file(&report_path).unwrap();
    assert_eq!(validation.error_count(), 0);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SOURCE_ROLE_CONDITIONAL,
        report::DiagnosticSeverity::Warning,
    );

    let default_exit = run_cli(vec![
        "validate-report".to_string(),
        "--input".to_string(),
        report_path.display().to_string(),
    ])
    .unwrap();
    assert_eq!(default_exit, 0);

    let strict_exit = run_cli(vec![
        "validate-report".to_string(),
        "--input".to_string(),
        report_path.display().to_string(),
        "--strict".to_string(),
    ])
    .unwrap();
    assert_eq!(strict_exit, 1);
}

#[test]
fn validate_report_surfaces_embedded_export_diagnostics_in_strict_mode() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("embedded-warning-report.json");
    let mut value = current_human_report_value();
    value["diagnostics"] = json!({
        "summary": "export diagnostics",
        "checks": [
            {
                "check_id": report::CHECK_EXPORT_UNRESOLVED_REFERENCE,
                "severity": "warning",
                "message": "Relation endpoint could not be resolved during export.",
                "target_path": "/report/relations/0",
                "entity_id": "rel-quotient-depends-invariance"
            }
        ]
    });
    report::write_json_file(&report_path, &value).unwrap();

    let validation = report::validate_report_file(&report_path).unwrap();
    assert_eq!(validation.error_count(), 0);
    assert_validation_check(
        &validation,
        report::CHECK_EXPORT_UNRESOLVED_REFERENCE,
        report::DiagnosticSeverity::Warning,
    );

    assert_eq!(
        run_cli(vec![
            "validate-report".to_string(),
            "--input".to_string(),
            report_path.display().to_string(),
        ])
        .unwrap(),
        0
    );
    assert_eq!(
        run_cli(vec![
            "validate-report".to_string(),
            "--input".to_string(),
            report_path.display().to_string(),
            "--strict".to_string(),
        ])
        .unwrap(),
        1
    );

    let waived_path = dir.path().join("accepted-loss-report.json");
    value["diagnostics"]["checks"][0]["status"] = json!("not_applicable");
    value["diagnostics"]["checks"][0]["message"] =
        json!("Accepted loss waiver: unresolved export reference is intentional in this fixture.");
    report::write_json_file(&waived_path, &value).unwrap();
    let waived = report::validate_report_file(&waived_path).unwrap();
    assert_eq!(waived.warning_count(), 0, "{:?}", waived.diagnostics);
    assert_validation_check(
        &waived,
        report::CHECK_EXPORT_UNRESOLVED_REFERENCE,
        report::DiagnosticSeverity::Info,
    );
    assert_eq!(
        run_cli(vec![
            "validate-report".to_string(),
            "--input".to_string(),
            waived_path.display().to_string(),
            "--strict".to_string(),
        ])
        .unwrap(),
        0
    );
}

#[test]
fn validate_report_requires_structure_for_substantial_final_reports() {
    let mut missing_relations = current_human_report_value();
    missing_relations["report"]["relations"] = json!([]);
    let validation = report::validate_report_value(&missing_relations);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_STRUCTURE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut missing_prerequisites = current_human_report_value();
    for step in missing_prerequisites["report"]["curriculum_path"]
        .as_array_mut()
        .unwrap()
    {
        step["prerequisite_ids"] = json!([]);
    }
    let validation = report::validate_report_value(&missing_prerequisites);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_STRUCTURE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut waived = missing_prerequisites.clone();
    waived["report"]["structure_waivers"] = json!([
        {
            "scope": "curriculum_prerequisites",
            "rationale": "This fixture intentionally omits prerequisites to test waiver behavior."
        }
    ]);
    let validation = report::validate_report_value(&waived);
    assert_no_validation_check(
        &validation,
        report::CHECK_VALIDATE_STRUCTURE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut missing_visual = current_human_report_value();
    missing_visual["report"]
        .as_object_mut()
        .unwrap()
        .remove("visual_views");
    missing_visual["diagnostics"] = json!({
        "checks": [
            {
                "check_id": report::CHECK_EXPORT_UNSUPPORTED_SECTION,
                "severity": "warning",
                "message": "Visual Summary was present in source Markdown but no structured Visual Views \
                    table preserved it."
            }
        ]
    });
    let validation = report::validate_report_value(&missing_visual);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_STRUCTURE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    missing_visual["report"]["structure_waivers"] = json!([
        {
            "scope": "visual_views",
            "rationale": "A text-only report intentionally omits the visual summary."
        }
    ]);
    let validation = report::validate_report_value(&missing_visual);
    assert_no_validation_check(
        &validation,
        report::CHECK_VALIDATE_STRUCTURE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );
    assert_validation_check(
        &validation,
        report::CHECK_EXPORT_UNSUPPORTED_SECTION,
        report::DiagnosticSeverity::Info,
    );
}

#[test]
fn validate_report_checks_new_structure_endpoints() {
    let mut bad_ladder_source = current_human_report_value();
    let boundary_row_index =
        report_item_index(&bad_ladder_source, "literature_ladder", "layer", "Boundary");
    bad_ladder_source["report"]["literature_ladder"][boundary_row_index]["source_ids"] =
        json!(["src-missing-source"]);
    let validation = report::validate_report_value(&bad_ladder_source);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SOURCE,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_claim_source = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &bad_claim_source,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    let supporting_link_index = array_item_index(
        &bad_claim_source["report"]["claims"][chain_claim_index]["evidence_links"],
        "support_kind",
        "supports",
    );
    bad_claim_source["report"]["claims"][chain_claim_index]["evidence_links"]
        [supporting_link_index]["source_id"] = json!("src-missing-source");
    let validation = report::validate_report_value(&bad_claim_source);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SOURCE,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_frontier_claim = current_human_report_value();
    let foundation_source_id =
        report_item_id(&bad_frontier_claim, "sources", "title", "Quantum Sensing");
    let measurand_id = report_item_id(
        &bad_frontier_claim,
        "field_elements",
        "label",
        "Physical quantity",
    );
    bad_frontier_claim["report"]["frontier_debates"] = json!([
        {
            "id": "frontier-validation-probe",
            "kind": "frontier",
            "title": "Validation probe",
            "summary": "A bounded fixture item for checking public endpoint resolution.",
            "why_it_matters": "A frontier item must resolve every structured claim reference.",
            "required_background_ids": [measurand_id],
            "claim_ids": ["claim-missing"],
            "source_ids": [foundation_source_id],
            "temporal": {
                "as_of": "2026-07-27",
                "review_after": "2027-01-26",
                "temporal_status": "current"
            }
        }
    ]);
    let validation = report::validate_report_value(&bad_frontier_claim);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_RELATION_ENDPOINT,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_visual_ref = current_human_report_value();
    let view_index = report_item_index(
        &bad_visual_ref,
        "visual_views",
        "title",
        "From fragile probe to trustworthy measurement",
    );
    let node_index = array_item_index(
        &bad_visual_ref["report"]["visual_views"][view_index]["nodes"],
        "label",
        "Physical quantity",
    );
    bad_visual_ref["report"]["visual_views"][view_index]["nodes"][node_index]["ref_id"] =
        json!("step-missing");
    let validation = report::validate_report_value(&bad_visual_ref);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_visual_node = current_human_report_value();
    let measurand_id = report_item_id(
        &bad_visual_node,
        "field_elements",
        "label",
        "Physical quantity",
    );
    let encoding_id = report_item_id(
        &bad_visual_node,
        "field_elements",
        "label",
        "Encoding interaction",
    );
    let relation_index =
        report_relation_index_between(&bad_visual_node, &measurand_id, &encoding_id);
    let relation_id = bad_visual_node["report"]["relations"][relation_index]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let view_index = report_item_index(
        &bad_visual_node,
        "visual_views",
        "title",
        "From fragile probe to trustworthy measurement",
    );
    let edge_index = array_item_index(
        &bad_visual_node["report"]["visual_views"][view_index]["edges"],
        "relation_id",
        &relation_id,
    );
    bad_visual_node["report"]["visual_views"][view_index]["edges"][edge_index]["from"] =
        json!("vnode-missing");
    let validation = report::validate_report_value(&bad_visual_node);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_visual_relation = current_human_report_value();
    let measurand_id = report_item_id(
        &bad_visual_relation,
        "field_elements",
        "label",
        "Physical quantity",
    );
    let encoding_id = report_item_id(
        &bad_visual_relation,
        "field_elements",
        "label",
        "Encoding interaction",
    );
    let relation_index =
        report_relation_index_between(&bad_visual_relation, &measurand_id, &encoding_id);
    let relation_id = bad_visual_relation["report"]["relations"][relation_index]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let view_index = report_item_index(
        &bad_visual_relation,
        "visual_views",
        "title",
        "From fragile probe to trustworthy measurement",
    );
    let edge_index = array_item_index(
        &bad_visual_relation["report"]["visual_views"][view_index]["edges"],
        "relation_id",
        &relation_id,
    );
    bad_visual_relation["report"]["visual_views"][view_index]["edges"][edge_index]["relation_id"] =
        json!("rel-missing");
    let validation = report::validate_report_value(&bad_visual_relation);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );

    let mut missing_elements = current_human_report_value();
    missing_elements["report"]["core_ideas"] = json!([]);
    missing_elements["report"]["methods"] = json!([]);
    missing_elements["report"]["representations"] = json!([]);
    missing_elements["report"]
        .as_object_mut()
        .unwrap()
        .remove("field_elements");
    let validation = report::validate_report_value(&missing_elements);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_STRUCTURE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_visual_views_require_stable_unambiguous_renderable_declarations() {
    let fixture = current_human_report_value();
    let validation = report::validate_report_value(&fixture);
    assert_no_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );

    let mut unstable_view_id = fixture.clone();
    unstable_view_id["report"]["visual_views"][0]["id"] = json!("BAD VIEW ID");
    assert_validation_message(
        &report::validate_report_value(&unstable_view_id),
        "visual view id is not stable",
    );

    let mut duplicate_view_id = fixture.clone();
    let duplicate_view = duplicate_view_id["report"]["visual_views"][0].clone();
    duplicate_view_id["report"]["visual_views"]
        .as_array_mut()
        .unwrap()
        .push(duplicate_view);
    assert_validation_message(
        &report::validate_report_value(&duplicate_view_id),
        "duplicate visual view id",
    );

    let mut unsupported_kind = fixture.clone();
    unsupported_kind["report"]["visual_views"][0]["kind"] = json!("custom");
    assert_validation_message(
        &report::validate_report_value(&unsupported_kind),
        "unsupported kind custom",
    );

    let mut blank_justification = fixture.clone();
    blank_justification["report"]["visual_views"][0]["justification"] = json!(" \n ");
    assert_validation_message(
        &report::validate_report_value(&blank_justification),
        "requires a non-empty justification",
    );

    let mut too_few_nodes = fixture.clone();
    too_few_nodes["report"]["visual_views"][0]["nodes"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    too_few_nodes["report"]["visual_views"][0]["edges"] = json!([]);
    let validation = report::validate_report_value(&too_few_nodes);
    assert_validation_message(&validation, "requires at least two nodes");
    assert_validation_message(
        &validation,
        "requires at least one valid relation-backed edge",
    );

    let mut unstable_node_id = fixture.clone();
    unstable_node_id["report"]["visual_views"][0]["nodes"][0]["id"] = json!("BAD NODE ID");
    assert_validation_message(
        &report::validate_report_value(&unstable_node_id),
        "node id is not stable",
    );

    let mut duplicate_node_id = fixture.clone();
    let first_node_id = duplicate_node_id["report"]["visual_views"][0]["nodes"][0]["id"].clone();
    duplicate_node_id["report"]["visual_views"][0]["nodes"][1]["id"] = first_node_id;
    assert_validation_message(
        &report::validate_report_value(&duplicate_node_id),
        "duplicate node id",
    );

    let mut missing_node_ref = fixture.clone();
    missing_node_ref["report"]["visual_views"][0]["nodes"][0]
        .as_object_mut()
        .unwrap()
        .remove("ref_id");
    assert_validation_message(
        &report::validate_report_value(&missing_node_ref),
        "requires an entity ref_id",
    );

    let mut self_edge = fixture;
    let from = self_edge["report"]["visual_views"][0]["edges"][0]["from"].clone();
    self_edge["report"]["visual_views"][0]["edges"][0]["to"] = from;
    assert_validation_message(&report::validate_report_value(&self_edge), "is a self-edge");
}

#[test]
fn validate_report_evidence_support_semantics_are_strict() {
    let mut qualifies_only = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &qualifies_only,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    let foundation_source_id =
        report_item_id(&qualifies_only, "sources", "title", "Quantum Sensing");
    qualifies_only["report"]["claims"][chain_claim_index]["evidence_links"] = json!([
        {
            "source_id": foundation_source_id,
            "verification_status": "reviewed",
            "support_kind": "qualifies",
            "locator": "measurement protocols and noise",
            "support_note": "This narrows but does not affirm the claim.",
            "reviewed_at": "2026-07-27"
        }
    ]);
    let validation = report::validate_report_value(&qualifies_only);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SUPPORT,
        report::DiagnosticSeverity::Warning,
    );

    let mut contradicts = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &contradicts,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    let metrology_source_id = report_item_id(
        &contradicts,
        "sources",
        "title",
        "Advances in Quantum Metrology",
    );
    contradicts["report"]["claims"][chain_claim_index]["evidence_links"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "source_id": metrology_source_id,
            "verification_status": "reviewed",
            "support_kind": "contradicts",
            "locator": "precision bounds and resource accounting",
            "support_note": "This conflicts with an overbroad form of the claim.",
            "reviewed_at": "2026-07-27"
        }));
    let validation = report::validate_report_value(&contradicts);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SUPPORT,
        report::DiagnosticSeverity::Warning,
    );
    assert_eq!(validation.error_count(), 0, "{:?}", validation.diagnostics);

    let mut missing_reviewed_at = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &missing_reviewed_at,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    let supporting_link_index = array_item_index(
        &missing_reviewed_at["report"]["claims"][chain_claim_index]["evidence_links"],
        "support_kind",
        "supports",
    );
    missing_reviewed_at["report"]["claims"][chain_claim_index]["evidence_links"]
        [supporting_link_index]
        .as_object_mut()
        .unwrap()
        .remove("reviewed_at");
    let validation = report::validate_report_value(&missing_reviewed_at);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SUPPORT,
        report::DiagnosticSeverity::Error,
    );

    let mut cataloged_visible = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &cataloged_visible,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    let foundation_source_id =
        report_item_id(&cataloged_visible, "sources", "title", "Quantum Sensing");
    cataloged_visible["report"]["claims"][chain_claim_index]["evidence_links"] = json!([
        {
            "source_id": foundation_source_id,
            "verification_status": "cataloged",
            "support_kind": "supports"
        }
    ]);
    let validation = report::validate_report_value(&cataloged_visible);
    assert_validation_check(
        &validation,
        report::CHECK_EVIDENCE_CATALOGED_ONLY,
        report::DiagnosticSeverity::Warning,
    );
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_output_order_is_deterministic() {
    let mut value = current_human_report_value();
    value["report"]["sources"][1]["access"]["route"] = json!("unknown");
    value["report"]["claims"][2]["evidence_links"] = json!([]);
    value["report"]["relations"][0]["to"]["id"] = json!("concept-missing");

    let validation = report::validate_report_value(&value);
    let actual = validation_diagnostic_order(&validation);
    let mut sorted = actual.clone();
    sorted.sort();
    assert_eq!(actual, sorted);

    let repeated = validation_diagnostic_order(&report::validate_report_value(&value));
    assert_eq!(actual, repeated);
}
