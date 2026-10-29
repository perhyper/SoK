use super::*;

#[test]
fn canonical_relation_semantics_and_legacy_labels_map_explicitly() {
    for (raw, expected, reverse) in [
        (
            "requires_before",
            report::RelationKind::RequiresBefore,
            false,
        ),
        ("introduced_by", report::RelationKind::IntroducedBy, false),
        ("revisits", report::RelationKind::Revisits, false),
        ("deepens", report::RelationKind::Deepens, false),
        ("applies", report::RelationKind::Applies, false),
        ("assessed_by", report::RelationKind::AssessedBy, false),
        ("remediates", report::RelationKind::Remediates, false),
        ("precedes", report::RelationKind::RequiresBefore, false),
        ("depends_on", report::RelationKind::RequiresBefore, true),
        ("prerequisite", report::RelationKind::RequiresBefore, true),
        (
            "prerequisite_for",
            report::RelationKind::RequiresBefore,
            false,
        ),
        ("introduces", report::RelationKind::IntroducedBy, true),
        ("uses_method", report::RelationKind::UsesMethod, false),
        ("qualifies", report::RelationKind::Qualifies, false),
    ] {
        let mapping = crate::core::relations::relation_kind_mapping(raw).unwrap();
        assert_eq!(mapping.kind, expected, "{raw}");
        assert_eq!(mapping.reverse_endpoints, reverse, "{raw}");
    }
}

#[test]
fn distinct_assertions_keep_their_sources_on_import() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("duplicate-relations.md");
    let sources_path = dir.path().join("sources.csv");
    fs::write(
        &sources_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
Open Review,review_article,doi:10.0000/open,https://example.test/open,2025-01-01,open_access,Official URL,$0,CC BY,foundation,Supports relation deduplication.,Use in module 1,Reviewed metadata.\n\
Interface Study,article,doi:10.0000/interface,https://example.test/interface,2024-06-01,open_access,Official URL,$0,CC BY,method,Adds a second relation source.,Use in module 2,Reviewed metadata.\n",
    )
    .unwrap();
    fs::write(
        &report_path,
        r#"# Structure of Knowledge: Relation Deduplication

## Field Element Inventory

| Element class | Observed element | Actual form in this field | Role | Load-bearing relations | Source IDs | Confidence |
|---|---|---|---|---|---|---|
| Concept | Alpha | A source-backed element. | core | Alpha applies to beta. | Open Review | high |
| Concept | Beta | A second source-backed element. | surrounding | Beta receives alpha. | Interface Study | high |

## Relations

| Relation ID | Relation kind | From type | From reference | To type | To reference | Rationale | Source IDs |
|---|---|---|---|---|---|---|---|
| rel-z-duplicate-1111111111 | applies | field_element | Alpha | field_element | Beta | Duplicate semantic relation. | Open Review |
| rel-a-duplicate-2222222222 | maps_to | field_element | Alpha | field_element | Beta | Duplicate semantic relation. | Interface Study |
"#,
    )
    .unwrap();

    let exported = report::export_markdown_report(
        &report_path,
        &sources_path,
        None::<&PathBuf>,
        report::ExportStage::Scaffold,
    )
    .unwrap();

    assert_eq!(exported.report.relations.len(), 2);
    assert!(exported
        .report
        .relations
        .iter()
        .all(|r| r.source_ids.len() == 1));
    let kinds: BTreeSet<_> = exported.report.relations.iter().map(|r| r.kind).collect();
    assert_eq!(
        kinds,
        BTreeSet::from([report::RelationKind::Applies, report::RelationKind::MapsTo])
    );
}

#[test]
fn legacy_v1_and_v2_relation_kinds_read_through_canonical_mapping() {
    for mut value in [current_human_report_value(), legacy_v1_report_value()] {
        let index = first_hard_curriculum_relation_index(&value);
        let from = value["report"]["relations"][index]["from"].clone();
        let to = value["report"]["relations"][index]["to"].clone();
        value["report"]["relations"][index]["kind"] = json!("depends_on");
        value["report"]["relations"][index]["from"] = to;
        value["report"]["relations"][index]["to"] = from;

        let document: report::ReportDocument = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            document.report.relations[index].kind,
            report::RelationKind::RequiresBefore
        );
        let validation = report::validate_report_value(&value);
        assert_eq!(validation.error_count(), 0, "{:?}", validation.diagnostics);
    }
}

#[test]
fn hard_prerequisite_cycles_are_stable_errors_but_revisits_and_deepens_may_cycle() {
    let valid = report::validate_report_value(&current_human_report_value());
    assert_eq!(valid.error_count(), 0, "{:?}", valid.diagnostics);

    let mut self_loop = current_human_report_value();
    self_loop["report"]["relations"][0]["to"] = self_loop["report"]["relations"][0]["from"].clone();
    let validation = report::validate_report_value(&self_loop);
    assert!(
        validation
            .diagnostics
            .checks
            .iter()
            .any(|check| check.message.contains("self-loop")),
        "{:?}",
        validation.diagnostics.checks
    );

    let mut hard_cycle = current_human_report_value();
    let encoding_id = report_item_id(
        &hard_cycle,
        "field_elements",
        "label",
        "Encoding interaction",
    );
    let readout_id = report_item_id(
        &hard_cycle,
        "field_elements",
        "label",
        "Readout and transduction",
    );
    hard_cycle["report"]["relations"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "rel-readout-before-encoding-cycle-1111111111",
            "kind": "requires_before",
            "from": {"entity_type": "field_element", "id": readout_id.clone()},
            "to": {"entity_type": "field_element", "id": encoding_id.clone()},
            "description": "Cycle fixture."
        }));
    let validation = report::validate_report_value(&hard_cycle);
    let cycle_message = validation
        .diagnostics
        .checks
        .iter()
        .find(|check| check.message.contains("hard prerequisite cycle"))
        .map(|check| check.message.clone())
        .unwrap_or_else(|| {
            panic!(
                "expected hard cycle diagnostic: {:?}",
                validation.diagnostics
            )
        });
    assert_contains(
        &cycle_message,
        "field_element:element-encoding-interaction-",
    );
    assert_contains(
        &cycle_message,
        "field_element:element-readout-and-transduction-",
    );

    let mut allowed_cycle = current_human_report_value();
    allowed_cycle["report"]["relations"]
        .as_array_mut()
        .unwrap()
        .extend([
            json!({
                "id": "rel-encoding-revisits-readout-1111111111",
                "kind": "revisits",
                "from": {"entity_type": "field_element", "id": encoding_id.clone()},
                "to": {"entity_type": "field_element", "id": readout_id.clone()},
                "description": "Allowed revisit cycle fixture."
            }),
            json!({
                "id": "rel-readout-deepens-encoding-2222222222",
                "kind": "deepens",
                "from": {"entity_type": "field_element", "id": readout_id.clone()},
                "to": {"entity_type": "field_element", "id": encoding_id.clone()},
                "description": "Allowed deepening cycle fixture."
            }),
        ]);
    let validation = report::validate_report_value(&allowed_cycle);
    assert_eq!(validation.error_count(), 0, "{:?}", validation.diagnostics);
}

#[test]
fn curriculum_and_visual_projections_must_reconcile_to_canonical_relations() {
    let mut missing_curriculum_relation = current_human_report_value();
    let relation_index = first_hard_curriculum_relation_index(&missing_curriculum_relation);
    missing_curriculum_relation["report"]["relations"]
        .as_array_mut()
        .unwrap()
        .remove(relation_index);
    let validation = report::validate_report_value(&missing_curriculum_relation);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_CURRICULUM_REFERENCE,
        report::DiagnosticSeverity::Error,
    );

    let mut visual_kind_mismatch = current_human_report_value();
    visual_kind_mismatch["report"]["visual_views"][0]["edges"][0]["kind"] = json!("remediates");
    let validation = report::validate_report_value(&visual_kind_mismatch);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );

    let mut visual_direction_mismatch = current_human_report_value();
    let visual_edge = &mut visual_direction_mismatch["report"]["visual_views"][0]["edges"][0];
    let from = visual_edge["from"].clone();
    visual_edge["from"] = visual_edge["to"].clone();
    visual_edge["to"] = from;
    let validation = report::validate_report_value(&visual_direction_mismatch);
    assert_validation_message(&validation, "does not match the endpoint entity references");
}
