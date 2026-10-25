use super::*;

#[test]
fn core_packages_project_to_sok_report_v2_and_html() {
    let packages = core_fixture_packages();
    let validation = crate::core::validation::validate_core_packages(&packages);
    assert_eq!(validation.error_count(), 0, "{:?}", validation.diagnostics);

    let projected = crate::core::projection::project_report_compatibility(
        &packages,
        &core_projection_context(),
    )
    .unwrap();
    assert_eq!(projected.metadata.schema_version, "sok-report/v2");
    assert_eq!(projected.report.field, "Quantum sensing");
    assert_eq!(projected.report.field_elements.len(), 3);
    assert!(projected
        .report
        .core_ideas
        .iter()
        .any(|item| item.label == "Physical quantity"));
    assert!(projected
        .report
        .methods
        .iter()
        .any(|item| item.label == "Calibration transfer"));
    assert!(projected
        .report
        .representations
        .iter()
        .any(|item| item.label == "Noise model"));
    assert_eq!(projected.report.literature_ladder.len(), 1);
    assert_eq!(projected.report.curriculum_path.len(), 2);

    let report_validation =
        report::validate_report_value(&serde_json::to_value(&projected).unwrap());
    assert_eq!(
        report_validation.error_count(),
        0,
        "{:?}",
        report_validation.diagnostics
    );
    let html = report::render_html_report(&projected).unwrap();
    assert_contains(&html, "Quantum sensing");
    assert_contains(&html, "Calibration transfer");
}

#[test]
fn export_json_accepts_core_sidecars_without_markdown_tables() {
    let dir = tempfile::tempdir().unwrap();
    let output_json_path = dir.path().join("sok-report.json");

    assert_eq!(
        run_cli(vec![
            "export-json".to_string(),
            "--stage".to_string(),
            "scaffold".to_string(),
            "--report".to_string(),
            repo_path("cli/tests/fixtures/input-lanes/machine-only.md")
                .display()
                .to_string(),
            "--sources".to_string(),
            repo_path("cli/tests/fixtures/input-lanes/header-only-sources.csv")
                .display()
                .to_string(),
            "--knowledge".to_string(),
            repo_path("cli/tests/fixtures/core/knowledge-valid.json")
                .display()
                .to_string(),
            "--evidence-package".to_string(),
            repo_path("cli/tests/fixtures/core/evidence-valid.json")
                .display()
                .to_string(),
            "--pedagogy".to_string(),
            repo_path("cli/tests/fixtures/core/pedagogy-valid.json")
                .display()
                .to_string(),
            "--output".to_string(),
            output_json_path.display().to_string(),
        ])
        .unwrap(),
        0
    );

    let exported: report::ReportDocument = report::read_json_file(&output_json_path).unwrap();
    assert_eq!(exported.metadata.report_type, report::ReportType::Scaffold);
    assert_eq!(exported.report.field, "Quantum sensing");
    assert_eq!(exported.report.field_elements.len(), 3);
    assert_eq!(exported.report.sources.len(), 1);
    assert_eq!(exported.report.curriculum_path.len(), 2);
    assert!(
        exported.diagnostics.is_none(),
        "valid sidecars should not emit Markdown table diagnostics: {:?}",
        exported.diagnostics
    );
}

#[test]
fn machine_sidecars_merge_with_markdown_without_overwriting_human_items() {
    let dir = tempfile::tempdir().unwrap();
    let knowledge_path = dir.path().join("extra-knowledge.json");
    report::write_json_file(
        &knowledge_path,
        &json!({
            "schema_version": "sok-knowledge/v1",
            "elements": [
                {
                    "id": "element-field-validation-4444444444",
                    "element_class": "Practice",
                    "label": {
                        "text": "Field validation protocol"
                    },
                    "actual_form": {
                        "text": "A machine-supplied protocol element added without localized Markdown headers."
                    },
                    "semantic_roles": ["surrounding", "practice"],
                    "role_note": "surrounding",
                    "confidence": "medium"
                }
            ]
        }),
    )
    .unwrap();

    let baseline = current_human_report_document();
    let exported = report::export_markdown_report_with_machine_inputs(
        repo_path("cli/tests/fixtures/pipeline/report.md"),
        repo_path("cli/tests/fixtures/pipeline/sources.csv"),
        Some(repo_path(
            "cli/tests/fixtures/pipeline/reviewed-evidence.jsonl",
        )),
        report::ExportStage::Final,
        report::MachineInputPaths {
            knowledge: Some(knowledge_path),
            ..report::MachineInputPaths::default()
        },
    )
    .unwrap();

    assert_eq!(
        exported.report.field_elements.len(),
        baseline.report.field_elements.len() + 1
    );
    assert!(exported
        .report
        .field_elements
        .iter()
        .any(|element| element.label == "Field validation protocol"));
    assert!(exported.diagnostics.is_none(), "{:?}", exported.diagnostics);
}

#[test]
fn sok_json_blocks_feed_core_models_without_localized_table_headers() {
    let exported = report::export_markdown_report(
        repo_path("cli/tests/fixtures/input-lanes/korean-sok-json.md"),
        repo_path("cli/tests/fixtures/input-lanes/header-only-sources.csv"),
        None,
        report::ExportStage::Scaffold,
    )
    .unwrap();

    assert_eq!(exported.report.field, "약물정보학");
    assert_eq!(exported.report.field_elements.len(), 1);
    assert_eq!(exported.report.field_elements[0].label, "의약품 허가사항");
    assert_eq!(exported.report.field_elements[0].role, "core");
    assert!(exported.diagnostics.is_none(), "{:?}", exported.diagnostics);
}

#[test]
fn sok_json_blocks_reject_schema_errors_with_diagnostics() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("bad-sok-json.md");
    fs::write(
        &report_path,
        r#"# Structure of Knowledge: Broken machine lane

```sok-json
{
  "field": "Missing schema",
  "elements": []
}
```
"#,
    )
    .unwrap();

    let exported = report::export_markdown_report(
        &report_path,
        &repo_path("cli/tests/fixtures/input-lanes/header-only-sources.csv"),
        None,
        report::ExportStage::Scaffold,
    )
    .unwrap();
    let diagnostics = exported.diagnostics.clone().unwrap();
    assert_validation_check(
        &report::ReportValidation { diagnostics },
        report::CHECK_EXPORT_MACHINE_INPUT,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn sok_json_examples_inside_markdown_fences_are_not_machine_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("example-sok-json.md");
    fs::write(
        &report_path,
        r#"# Structure of Knowledge: Example-only machine syntax

## Orientation

````markdown
```sok-json
{
  "field": "This is example text, not input"
}
```
````
"#,
    )
    .unwrap();

    let exported = report::export_markdown_report(
        &report_path,
        &repo_path("cli/tests/fixtures/input-lanes/header-only-sources.csv"),
        None,
        report::ExportStage::Scaffold,
    )
    .unwrap();
    if let Some(diagnostics) = exported.diagnostics {
        assert!(
            diagnostics
                .checks
                .iter()
                .all(|check| check.check_id != report::CHECK_EXPORT_MACHINE_INPUT),
            "fenced examples must not be parsed as machine input: {:?}",
            diagnostics.checks
        );
    }
}

#[test]
fn conflicting_human_and_machine_lane_inputs_emit_diagnostics() {
    let dir = tempfile::tempdir().unwrap();
    let baseline = current_human_report_document();
    let first_element = baseline.report.field_elements.first().unwrap();
    let knowledge_path = dir.path().join("conflict-knowledge.json");
    report::write_json_file(
        &knowledge_path,
        &json!({
            "schema_version": "sok-knowledge/v1",
            "field": baseline.report.field.clone(),
            "elements": [
                {
                    "id": first_element.id.clone(),
                    "element_class": first_element.element_class.clone(),
                    "label": {
                        "text": "Conflicting machine label"
                    },
                    "actual_form": {
                        "text": "Conflicting machine content"
                    },
                    "semantic_roles": ["core"],
                    "role_note": "core"
                }
            ]
        }),
    )
    .unwrap();

    let exported = report::export_markdown_report_with_machine_inputs(
        repo_path("cli/tests/fixtures/pipeline/report.md"),
        repo_path("cli/tests/fixtures/pipeline/sources.csv"),
        Some(repo_path(
            "cli/tests/fixtures/pipeline/reviewed-evidence.jsonl",
        )),
        report::ExportStage::Final,
        report::MachineInputPaths {
            knowledge: Some(knowledge_path),
            ..report::MachineInputPaths::default()
        },
    )
    .unwrap();

    assert!(exported
        .report
        .field_elements
        .iter()
        .any(|element| element.id == first_element.id && element.label == first_element.label));
    let diagnostics = exported.diagnostics.clone().unwrap();
    assert_validation_check(
        &report::ReportValidation {
            diagnostics: diagnostics.clone(),
        },
        report::CHECK_EXPORT_MACHINE_INPUT,
        report::DiagnosticSeverity::Error,
    );
    assert!(diagnostics
        .checks
        .iter()
        .any(|check| check.message.contains("kept first value")));
}

#[test]
fn core_validators_reject_unknown_versions_and_dangling_references() {
    assert!(
        serde_json::from_value::<crate::core::knowledge::KnowledgePackage>(json!({
            "elements": []
        }))
        .is_err()
    );

    let mut packages = core_fixture_packages();
    let valid = crate::core::validation::validate_core_packages(&packages);
    assert_eq!(valid.error_count(), 0, "{:?}", valid.diagnostics);

    packages.knowledge.schema_version = "sok-knowledge/v99".to_string();
    packages.knowledge.elements[0]
        .source_ids
        .push("src-missing-source-9999999999".to_string());
    packages.pedagogy.learning_path[1]
        .prerequisite_ids
        .push("element-missing-prerequisite-9999999999".to_string());
    let validation = crate::core::validation::validate_core_packages(&packages);
    assert_validation_check(
        &report::ReportValidation {
            diagnostics: validation.diagnostics.clone(),
        },
        crate::core::validation::CHECK_CORE_VERSION,
        report::DiagnosticSeverity::Error,
    );
    assert_validation_check(
        &report::ReportValidation {
            diagnostics: validation.diagnostics.clone(),
        },
        crate::core::validation::CHECK_CORE_REFERENCE,
        report::DiagnosticSeverity::Error,
    );

    let mut broken_support = core_fixture_packages();
    broken_support.evidence.claims[0].evidence_links[0]
        .reviewed_at
        .clear();
    let support_validation = crate::core::validation::validate_core_packages(&broken_support);
    assert_validation_check(
        &report::ReportValidation {
            diagnostics: support_validation.diagnostics,
        },
        crate::core::validation::CHECK_CORE_SUPPORT,
        report::DiagnosticSeverity::Error,
    );
}
