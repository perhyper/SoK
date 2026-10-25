use super::*;

#[test]
fn sok_report_schema_declares_json_first_contract() {
    let schema = load_repo_json("specs/sok-report.schema.json");
    assert_eq!(
        schema["$schema"],
        json!("https://json-schema.org/draft/2020-12/schema")
    );
    assert!(json_array_contains(&schema["required"], "metadata"));
    assert!(json_array_contains(&schema["required"], "report"));

    let defs = schema["$defs"].as_object().unwrap();
    for name in [
        "source_access_metadata",
        "source_role_requirement",
        "source_role_waiver",
        "claim",
        "evidence_link",
        "verification_status",
        "temporal_marker",
        "relation",
        "literature_ladder_row",
        "curriculum_step",
        "diagnostic",
        "visual_view_node",
        "visual_view_edge",
        "structure_waiver",
        "report_presentation",
        "report_section",
        "field_element",
    ] {
        assert!(defs.contains_key(name), "schema should define {name}");
    }

    let report_required = &defs["public_report"]["required"];
    for name in [
        "field",
        "scope",
        "domain_profile",
        "core_ideas",
        "methods",
        "representations",
        "evidence_standards",
        "sources",
        "claims",
        "relations",
        "curriculum_path",
        "frontier_debates",
    ] {
        assert!(
            json_array_contains(report_required, name),
            "public report should require {name}"
        );
    }
    assert!(
        defs["public_report"]["properties"]
            .as_object()
            .unwrap()
            .contains_key("presentation"),
        "schema should describe optional ordered presentation sections"
    );
    assert!(
        defs["public_report"]["properties"]
            .as_object()
            .unwrap()
            .contains_key("field_elements"),
        "schema should preserve optional field-native elements for v1 compatibility"
    );
    assert!(
        defs["public_report"]["properties"]
            .as_object()
            .unwrap()
            .contains_key("literature_ladder"),
        "schema should describe optional literature ladder rows"
    );
    assert!(
        defs["public_report"]["properties"]
            .as_object()
            .unwrap()
            .contains_key("visual_views"),
        "schema should describe optional visual views"
    );
    assert!(
        defs["public_report"]["properties"]
            .as_object()
            .unwrap()
            .contains_key("structure_waivers"),
        "schema should describe optional structure waivers"
    );
    assert!(
        !json_array_contains(report_required, "presentation"),
        "ordered presentation should remain optional for legacy JSON"
    );
    assert!(
        !json_array_contains(report_required, "field_elements"),
        "field_elements should remain schema-optional for legacy JSON"
    );
    let presentation_required = &defs["report_presentation"]["required"];
    for name in ["thesis", "organizing_form", "rationale", "sections"] {
        assert!(
            json_array_contains(presentation_required, name),
            "a declared presentation should require {name}"
        );
    }
    assert_eq!(
        defs["report_presentation"]["properties"]["sections"]["minItems"],
        json!(1)
    );
    assert!(defs["report_presentation"]["properties"]
        .as_object()
        .unwrap()
        .contains_key("alternatives_considered"));
    assert!(
        !json_array_contains(presentation_required, "alternatives_considered"),
        "the alternatives trace is v2-required but remains optional for legacy v1 presentations"
    );
    assert!(
        !json_array_contains(report_required, "literature_ladder"),
        "literature ladder rows should be declared but not globally required"
    );
    assert!(
        !json_array_contains(report_required, "visual_views"),
        "visual views should be declared but not globally required"
    );
    assert!(
        !json_array_contains(report_required, "structure_waivers"),
        "structure waivers should be declared but not globally required"
    );
    assert!(json_array_contains(
        &defs["structure_waiver"]["properties"]["scope"]["enum"],
        "curriculum_prerequisites"
    ));
    assert!(json_array_contains(
        &defs["relation_endpoint"]["properties"]["entity_type"]["enum"],
        "field_element"
    ));
    assert!(json_array_contains(
        &defs["visual_view_node"]["required"],
        "ref_id"
    ));
    assert!(json_array_contains(
        &defs["visual_view_edge"]["required"],
        "relation_id"
    ));
    assert_eq!(
        defs["visual_view"]["properties"]["nodes"]["minItems"],
        json!(2)
    );
    assert_eq!(
        defs["visual_view"]["properties"]["edges"]["minItems"],
        json!(1)
    );
    assert!(!json_array_contains(
        &defs["visual_view"]["properties"]["kind"]["enum"],
        "custom"
    ));
}

#[test]
fn sok_id_migration_schema_declares_reviewable_map_contract() {
    let schema = load_repo_json("specs/sok-id-migration.schema.json");
    assert_eq!(
        schema["$schema"],
        json!("https://json-schema.org/draft/2020-12/schema")
    );
    assert!(json_array_contains(&schema["required"], "schema_version"));
    assert!(json_array_contains(
        &schema["required"],
        "source_schema_version"
    ));
    assert!(json_array_contains(&schema["required"], "mappings"));
    assert_eq!(
        schema["properties"]["schema_version"]["const"],
        json!("sok-id-migration/v1")
    );
    assert!(json_array_contains(
        &schema["$defs"]["mapping"]["required"],
        "normalized_identity"
    ));
}

#[test]
fn sok_core_schemas_declare_versioned_package_contracts() {
    let knowledge = load_repo_json("specs/sok-knowledge.schema.json");
    let evidence = load_repo_json("specs/sok-evidence.schema.json");
    let pedagogy = load_repo_json("specs/sok-pedagogy.schema.json");
    assert_eq!(
        knowledge["$schema"],
        json!("https://json-schema.org/draft/2020-12/schema")
    );
    assert_eq!(
        knowledge["properties"]["schema_version"]["const"],
        json!("sok-knowledge/v1")
    );
    assert_eq!(
        evidence["properties"]["schema_version"]["const"],
        json!("sok-evidence/v1")
    );
    assert_eq!(
        pedagogy["properties"]["schema_version"]["const"],
        json!("sok-pedagogy/v1")
    );
    assert!(knowledge["$defs"]["knowledge_element"]["properties"]
        .as_object()
        .unwrap()
        .contains_key("semantic_roles"));
    assert!(knowledge["$defs"]["knowledge_element"]["properties"]
        .as_object()
        .unwrap()
        .contains_key("relation_ids"));
    assert!(knowledge["properties"]
        .as_object()
        .unwrap()
        .contains_key("relations"));
    assert!(json_array_contains(
        &knowledge["$defs"]["relation_kind"]["enum"],
        "requires_before"
    ));
    assert!(evidence["$defs"]["claim"]["properties"]
        .as_object()
        .unwrap()
        .contains_key("evidence_links"));
    assert!(pedagogy["$defs"]["learning_step"]["properties"]
        .as_object()
        .unwrap()
        .contains_key("prerequisite_ids"));
    assert!(pedagogy["$defs"]["learning_step"]["properties"]
        .as_object()
        .unwrap()
        .contains_key("prerequisite_relation_ids"));

    let report_schema = load_repo_json("specs/sok-report.schema.json");
    let public_report_properties = report_schema["$defs"]["public_report"]["properties"]
        .as_object()
        .unwrap();
    for disallowed in ["knowledge", "evidence_package", "pedagogy", "core_packages"] {
        assert!(
            !public_report_properties.contains_key(disallowed),
            "sok-report schema must not embed secondary core package field {disallowed}"
        );
    }
}

#[test]
fn sok_run_schema_declares_private_manifest_contract() {
    let schema = load_repo_json("specs/sok-run.schema.json");
    assert_eq!(
        schema["$schema"],
        json!("https://json-schema.org/draft/2020-12/schema")
    );
    assert_eq!(
        schema["properties"]["schema_version"]["const"],
        json!("sok-run/v1")
    );
    for required in [
        "run_id",
        "sok_revision",
        "skill_revision",
        "profile_version",
        "command",
        "declared_permissions",
        "bounded_input_files",
        "work_order_hashes",
        "work_result_hashes",
        "output_files",
        "semantic_digest",
        "timestamps",
    ] {
        assert!(json_array_contains(&schema["required"], required));
    }
    assert_eq!(
        schema["$defs"]["sha256"]["pattern"],
        json!("^[0-9a-f]{64}$")
    );

    let report_schema = load_repo_json("specs/sok-report.schema.json");
    let public_report_properties = report_schema["$defs"]["public_report"]["properties"]
        .as_object()
        .unwrap();
    let metadata_properties = report_schema["$defs"]["metadata"]["properties"]
        .as_object()
        .unwrap();
    for disallowed in [
        "run_manifest",
        "run_id",
        "executor",
        "declared_permissions",
        "bounded_input_files",
        "output_files",
        "semantic_digest",
    ] {
        assert!(
            !public_report_properties.contains_key(disallowed),
            "public report schema must not expose private run field {disallowed}"
        );
        assert!(
            !metadata_properties.contains_key(disallowed),
            "public report metadata must not expose private run field {disallowed}"
        );
    }
}

#[test]
fn sok_work_schemas_declare_provider_neutral_protocol_contracts() {
    let order = load_repo_json("specs/sok-work-order.schema.json");
    let result = load_repo_json("specs/sok-work-result.schema.json");
    let patch = load_repo_json("specs/sok-work-patch.schema.json");
    assert_eq!(
        order["properties"]["schema_version"]["const"],
        json!("sok-work-order/v1")
    );
    assert_eq!(
        result["properties"]["schema_version"]["const"],
        json!("sok-work-result/v1")
    );
    assert_eq!(
        patch["properties"]["schema_version"]["const"],
        json!("sok-work-patch/v1")
    );

    for task_kind in supported_work_task_kind_labels() {
        assert!(json_array_contains(
            &order["$defs"]["task_kind"]["enum"],
            task_kind
        ));
        assert!(json_array_contains(
            &result["$defs"]["task_kind"]["enum"],
            task_kind
        ));
    }
    for capability in [
        "basic_completion",
        "structured_output",
        "tool_capable",
        "long_context_agent",
    ] {
        assert!(json_array_contains(
            &order["$defs"]["capability"]["enum"],
            capability
        ));
        assert!(json_array_contains(
            &result["$defs"]["capability"]["enum"],
            capability
        ));
    }
}

#[test]
fn sok_report_fixtures_match_smoke_contract() {
    let final_report = current_human_report_value();
    validate_sok_report_smoke(&final_report).unwrap();
    assert_eq!(
        final_report["metadata"]["report_type"],
        json!("human_report")
    );
    assert!(final_report.get("internal_context").is_none());

    let scaffold_report = current_scaffold_report_value();
    validate_sok_report_smoke(&scaffold_report).unwrap();
    assert_eq!(
        scaffold_report["metadata"]["report_type"],
        json!("scaffold")
    );
    assert!(scaffold_report.get("internal_context").is_some());

    let scaffold_public = serde_json::to_string(&scaffold_report["report"]).unwrap();
    assert_not_contains(&scaffold_public, "SENTINEL_INTERNAL");
    assert_not_contains(&scaffold_public, "raw_learner_profile");
    assert_not_contains(&scaffold_public, "original_goal");
    assert_not_contains(&scaffold_public, "handoff_notes");
}

#[test]
fn sok_report_optional_structure_fields_round_trip_and_validate() {
    let structured = current_human_report_value();
    let boundary_source_id =
        report_item_id(&structured, "sources", "title", "Quantum Sensing Explained");
    let measurement_model_id =
        report_item_id(&structured, "curriculum_path", "title", "Measurement model");
    let encoding_step_index = report_item_index(
        &structured,
        "curriculum_path",
        "title",
        "Quantum encoding and control",
    );
    let measurand_id = report_item_id(&structured, "field_elements", "label", "Physical quantity");
    let encoding_id = report_item_id(
        &structured,
        "field_elements",
        "label",
        "Encoding interaction",
    );
    let measurement_relation_index =
        report_relation_index_between(&structured, &measurand_id, &encoding_id);
    let measurement_relation_id = structured["report"]["relations"][measurement_relation_index]
        ["id"]
        .as_str()
        .unwrap()
        .to_string();
    let measurement_view_index = report_item_index(
        &structured,
        "visual_views",
        "title",
        "From fragile probe to trustworthy measurement",
    );

    validate_sok_report_smoke(&structured).unwrap();
    let validation = report::validate_report_value(&structured);
    assert_eq!(validation.error_count(), 0);
    assert_eq!(validation.warning_count(), 0);

    let document: report::ReportDocument = serde_json::from_value(structured.clone()).unwrap();
    let boundary_row = document
        .report
        .literature_ladder
        .iter()
        .find(|row| row.layer == "Boundary")
        .expect("fresh report should preserve its boundary literature row");
    assert!(
        boundary_row.source_ids.contains(&boundary_source_id),
        "boundary row should resolve the named NIST source"
    );
    assert_eq!(
        structured["report"]["curriculum_path"][encoding_step_index]["prerequisite_ids"],
        json!([measurement_model_id])
    );
    assert!(
        structured["report"]["visual_views"][measurement_view_index]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["relation_id"].as_str() == Some(&measurement_relation_id)),
        "measurement view should preserve the relation that begins the chain"
    );

    let mut waived = current_human_report_value();
    waived["report"]
        .as_object_mut()
        .unwrap()
        .remove("visual_views");
    waived["report"]["structure_waivers"] = json!([
        {
            "scope": "visual_views",
            "rationale": "A short text-only report may intentionally omit visual views when no \
                relation-backed view improves reading."
        }
    ]);
    validate_sok_report_smoke(&waived).unwrap();
    let waiver_document: report::ReportDocument = serde_json::from_value(waived).unwrap();
    assert_eq!(waiver_document.report.visual_views.len(), 0);
    assert_eq!(waiver_document.report.structure_waivers.len(), 1);
    assert_eq!(
        waiver_document.report.structure_waivers[0].scope,
        report::StructureWaiverScope::VisualViews
    );
}

#[test]
fn sok_report_smoke_contract_rejects_bad_shapes() {
    assert!(serde_json::from_str::<Value>(r#"{"metadata":"#).is_err());

    let final_report = current_human_report_value();

    let mut missing_metadata = final_report.clone();
    missing_metadata.as_object_mut().unwrap().remove("metadata");
    assert_contains(
        &validate_sok_report_smoke(&missing_metadata)
            .unwrap_err()
            .to_string(),
        "missing metadata",
    );

    let mut invalid_type = final_report.clone();
    invalid_type["metadata"]["report_type"] = json!("wiki");
    assert_contains(
        &validate_sok_report_smoke(&invalid_type)
            .unwrap_err()
            .to_string(),
        "invalid report_type",
    );

    let mut leaked_context = final_report.clone();
    leaked_context["report"]["internal_context"] =
        json!({"raw_learner_profile": "should never be public"});
    assert_contains(
        &validate_sok_report_smoke(&leaked_context)
            .unwrap_err()
            .to_string(),
        "public report contains internal key internal_context",
    );

    let mut bad_relation_id = final_report.clone();
    let measurand_id = report_item_id(
        &bad_relation_id,
        "field_elements",
        "label",
        "Physical quantity",
    );
    let encoding_id = report_item_id(
        &bad_relation_id,
        "field_elements",
        "label",
        "Encoding interaction",
    );
    let relation_index =
        report_relation_index_between(&bad_relation_id, &measurand_id, &encoding_id);
    bad_relation_id["report"]["relations"][relation_index]["to"]["id"] = json!("claim with spaces");
    assert_contains(
        &validate_sok_report_smoke(&bad_relation_id)
            .unwrap_err()
            .to_string(),
        "invalid stable id",
    );

    let mut bad_source_ref = final_report.clone();
    let claim_index = report_item_index_containing(
        &bad_source_ref,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    let evidence_index = array_item_index(
        &bad_source_ref["report"]["claims"][claim_index]["evidence_links"],
        "support_kind",
        "supports",
    );
    bad_source_ref["report"]["claims"][claim_index]["evidence_links"][evidence_index]
        ["source_id"] = json!("src-missing-source");
    assert_contains(
        &validate_sok_report_smoke(&bad_source_ref)
            .unwrap_err()
            .to_string(),
        "unknown evidence source_id",
    );
}
