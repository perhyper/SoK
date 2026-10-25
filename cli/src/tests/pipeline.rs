use super::*;

#[test]
fn documented_scaffold_lane_generates_lints_and_exports_internal_stage_json() {
    let dir = tempfile::tempdir().unwrap();
    let sources_path = dir.path().join("sources.csv");
    let evidence_path = dir.path().join("evidence.jsonl");
    let scaffold_path = dir.path().join("sok-scaffold.md");
    let scaffold_json_path = dir.path().join("sok-scaffold.json");
    let html_path = dir.path().join("sok-scaffold.html");

    assert_eq!(
        run_cli(vec![
            "source-template".to_string(),
            "--output".to_string(),
            sources_path.display().to_string(),
        ])
        .unwrap(),
        0
    );
    assert_eq!(
        run_cli(vec![
            "ingest".to_string(),
            "last".to_string(),
            "--sources".to_string(),
            sources_path.display().to_string(),
            "--output".to_string(),
            evidence_path.display().to_string(),
        ])
        .unwrap(),
        0
    );
    assert_eq!(
        run_cli(vec![
            "scaffold".to_string(),
            "--field".to_string(),
            "topology".to_string(),
            "--output".to_string(),
            scaffold_path.display().to_string(),
        ])
        .unwrap(),
        0
    );
    assert_eq!(
        run_cli(vec![
            "lint".to_string(),
            "--stage".to_string(),
            "scaffold".to_string(),
            "--scaffold".to_string(),
            scaffold_path.display().to_string(),
            "--sources".to_string(),
            sources_path.display().to_string(),
            "--evidence".to_string(),
            evidence_path.display().to_string(),
        ])
        .unwrap(),
        0
    );
    assert_eq!(
        run_cli(vec![
            "export-json".to_string(),
            "--stage".to_string(),
            "scaffold".to_string(),
            "--scaffold".to_string(),
            scaffold_path.display().to_string(),
            "--sources".to_string(),
            sources_path.display().to_string(),
            "--evidence".to_string(),
            evidence_path.display().to_string(),
            "--output".to_string(),
            scaffold_json_path.display().to_string(),
        ])
        .unwrap(),
        0
    );

    let exported: report::ReportDocument = report::read_json_file(&scaffold_json_path).unwrap();
    assert_eq!(exported.metadata.report_type, report::ReportType::Scaffold);
    assert!(exported.internal_context.is_some());
    assert!(exported.diagnostics.is_some());
    assert!(!html_path.exists(), "scaffold lane must not render HTML");
}

#[test]
fn documented_final_report_lane_lints_exports_validates_and_renders_html() {
    let dir = tempfile::tempdir().unwrap();
    let output_json_path = dir.path().join("sok-report.json");
    let output_html_path = dir.path().join("sok-report.html");
    let report_path = repo_path("cli/tests/fixtures/pipeline/report.md");
    let sources_path = repo_path("cli/tests/fixtures/pipeline/sources.csv");
    let evidence_path = repo_path("cli/tests/fixtures/pipeline/reviewed-evidence.jsonl");

    assert_eq!(
        run_cli(vec![
            "lint".to_string(),
            "--stage".to_string(),
            "final".to_string(),
            "--report".to_string(),
            report_path.display().to_string(),
            "--sources".to_string(),
            sources_path.display().to_string(),
            "--evidence".to_string(),
            evidence_path.display().to_string(),
            "--strict".to_string(),
        ])
        .unwrap(),
        0
    );
    assert_eq!(
        run_cli(vec![
            "export-json".to_string(),
            "--stage".to_string(),
            "final".to_string(),
            "--report".to_string(),
            report_path.display().to_string(),
            "--sources".to_string(),
            sources_path.display().to_string(),
            "--evidence".to_string(),
            evidence_path.display().to_string(),
            "--output".to_string(),
            output_json_path.display().to_string(),
        ])
        .unwrap(),
        0
    );

    let exported: report::ReportDocument = report::read_json_file(&output_json_path).unwrap();
    assert_eq!(
        exported.metadata.report_type,
        report::ReportType::HumanReport
    );
    assert!(exported.internal_context.is_none());
    assert!(exported.diagnostics.is_none());
    let presentation = exported.report.presentation.as_ref().unwrap();
    assert_contains(&presentation.organizing_form, "measurement chain");
    assert_contains(&presentation.alternatives_considered, "taxonomy");
    assert!(
        presentation.sections.len() >= 3,
        "the final report should expose a field-specific narrative, not only structured surfaces"
    );
    assert!(presentation
        .sections
        .iter()
        .any(|section| !section.visual_view_ids.is_empty()));
    let section_titles = presentation
        .sections
        .iter()
        .map(|section| section.title.clone())
        .collect::<Vec<_>>();
    assert!(
        section_titles
            .iter()
            .all(|title| !title.trim().is_empty() && !title.starts_with("Section ")),
        "public sections should keep field-authored titles"
    );
    assert!(!exported.report.visual_views.is_empty());
    assert!(!exported.report.relations.is_empty());
    assert!(!exported.report.literature_ladder.is_empty());
    assert!(exported
        .report
        .curriculum_path
        .iter()
        .any(|step| !step.prerequisite_ids.is_empty()));
    assert!(exported
        .report
        .field_elements
        .iter()
        .any(|element| element.role == "core"));
    assert!(exported
        .report
        .field_elements
        .iter()
        .any(|element| element.role == "surrounding" || element.role == "context"));
    let relation_ids = exported
        .report
        .relations
        .iter()
        .map(|relation| relation.id.as_str())
        .collect::<BTreeSet<_>>();
    assert!(exported.report.visual_views.iter().all(|view| {
        !view.nodes.is_empty()
            && !view.edges.is_empty()
            && view
                .edges
                .iter()
                .all(|edge| relation_ids.contains(edge.relation_id.as_str()))
    }));
    assert!(exported
        .report
        .claims
        .iter()
        .all(
            |claim| claim.evidence_links.iter().any(|link| link.support_kind
                == report::SupportKind::Supports
                && matches!(
                    link.verification_status,
                    report::VerificationStatus::Reviewed | report::VerificationStatus::Verified
                ))
        ));
    let mounted_view_id = presentation
        .sections
        .iter()
        .flat_map(|section| section.visual_view_ids.iter())
        .next()
        .unwrap()
        .clone();

    assert_eq!(
        run_cli(vec![
            "validate-report".to_string(),
            "--input".to_string(),
            output_json_path.display().to_string(),
            "--strict".to_string(),
        ])
        .unwrap(),
        0
    );
    assert_eq!(
        run_cli(vec![
            "render-html".to_string(),
            "--input".to_string(),
            output_json_path.display().to_string(),
            "--output".to_string(),
            output_html_path.display().to_string(),
        ])
        .unwrap(),
        0
    );

    let html = fs::read_to_string(&output_html_path).unwrap();
    assert_contains(&html, "Structure of Knowledge Report");
    assert_contains(&html, &exported.report.field);
    assert_contains(&html, "Evidence and structured data");
    assert_contains(&html, &format!("data-visual-mount=\"{mounted_view_id}\""));
    let narrative_positions = presentation
        .sections
        .iter()
        .map(|section| {
            let anchor = format!("id=\"{}\"", section.id);
            html.find(&anchor).unwrap_or_else(|| {
                panic!(
                    "rendered HTML should contain section {:?} at {}",
                    section.title, section.id
                )
            })
        })
        .collect::<Vec<_>>();
    assert!(
        narrative_positions.windows(2).all(|pair| pair[0] < pair[1]),
        "rendered HTML should preserve the authored public section order"
    );
    assert_not_contains(&html, "internal_context");
    assert_not_contains(&html, "\"diagnostics\":");
    assert_not_contains(&html, "&quot;diagnostics&quot;");
    assert_not_contains(&html, "Scaffold Quality Notes");
    assert_not_contains(&html.to_ascii_lowercase(), "mermaid");
}

#[test]
fn final_markdown_requires_an_explicit_architecture_decision() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("missing-architecture.md");
    let sources_path = repo_path("cli/tests/fixtures/pipeline/sources.csv");
    let evidence_path = repo_path("cli/tests/fixtures/pipeline/reviewed-evidence.jsonl");
    let markdown = fs::read_to_string(repo_path("cli/tests/fixtures/pipeline/report.md"))
        .unwrap()
        .replace("## Report Architecture", "## How This Report Was Composed");
    fs::write(&report_path, markdown).unwrap();

    let lint = report::lint_markdown_report(
        &report_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    assert_validation_check(
        &lint,
        report::CHECK_EXPORT_REPORT_ARCHITECTURE,
        report::DiagnosticSeverity::Error,
    );

    let exported = report::export_markdown_report(
        &report_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    let validation = report::validate_report_value(&serde_json::to_value(exported).unwrap());
    assert_validation_check(
        &validation,
        report::CHECK_EXPORT_REPORT_ARCHITECTURE,
        report::DiagnosticSeverity::Error,
    );
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SCHEMA_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let ungrounded_path = dir.path().join("missing-field-elements.md");
    let markdown = fs::read_to_string(repo_path("cli/tests/fixtures/pipeline/report.md"))
        .unwrap()
        .replace(
            "<!-- sok:surface field-elements -->",
            "<!-- field-element surface intentionally removed -->",
        );
    fs::write(&ungrounded_path, markdown).unwrap();
    let lint = report::lint_markdown_report(
        &ungrounded_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    assert!(lint.diagnostics.checks.iter().any(|check| {
        check.check_id == report::CHECK_EXPORT_REPORT_ARCHITECTURE
            && check.message.contains("field-element inventory")
    }));

    let unusable_path = dir.path().join("unusable-field-elements.md");
    let mut markdown =
        fs::read_to_string(repo_path("cli/tests/fixtures/pipeline/report.md")).unwrap();
    let field_element_marker = "<!-- sok:surface field-elements -->";
    let inventory_start = markdown.find(field_element_marker).unwrap() + field_element_marker.len();
    let inventory_end = inventory_start
        + markdown[inventory_start..]
            .find("\n## ")
            .expect("the fixture should have a section after its field-element inventory");
    markdown.replace_range(
        inventory_start..inventory_end,
        r#"

| Element class | Observed element | Actual form in this field | Role | Load-bearing relations | Source IDs | Confidence |
|---|---|---|---|---|---|---|
| Candidate class |  |  | core |  |  |  |
| Candidate class | Candidate element | Source to verify | core |  |  |  |
"#,
    );
    fs::write(&unusable_path, markdown).unwrap();
    let lint = report::lint_markdown_report(
        &unusable_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    assert!(lint.diagnostics.checks.iter().any(|check| {
        check.check_id == report::CHECK_EXPORT_REPORT_ARCHITECTURE
            && check
                .message
                .contains("complete field-element inventory row")
    }));
}

#[test]
fn unknown_surface_markers_are_blocking_instead_of_silently_losing_data() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("unknown-surface.md");
    let sources_path = repo_path("cli/tests/fixtures/pipeline/sources.csv");
    let evidence_path = repo_path("cli/tests/fixtures/pipeline/reviewed-evidence.jsonl");
    let markdown = fs::read_to_string(repo_path("cli/tests/fixtures/pipeline/report.md"))
        .unwrap()
        .replacen(
            "## Claims",
            "## Evidence-Bearing Statements\n<!-- sok:surface claim -->",
            1,
        );
    fs::write(&report_path, markdown).unwrap();

    let lint = report::lint_markdown_report(
        &report_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    assert_validation_check(
        &lint,
        report::CHECK_EXPORT_UNKNOWN_SURFACE_MARKER,
        report::DiagnosticSeverity::Error,
    );

    let exported = report::export_markdown_report(
        &report_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    assert!(exported.report.claims.is_empty());
    let validation = report::validate_report_value(&serde_json::to_value(exported).unwrap());
    assert_validation_check(
        &validation,
        report::CHECK_EXPORT_UNKNOWN_SURFACE_MARKER,
        report::DiagnosticSeverity::Error,
    );

    let typo_path = dir.path().join("unknown-directive.md");
    let markdown = fs::read_to_string(repo_path("cli/tests/fixtures/pipeline/report.md"))
        .unwrap()
        .replace("sok:visual-view", "sok:visual-veiw");
    fs::write(&typo_path, markdown).unwrap();
    let lint = report::lint_markdown_report(
        &typo_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    assert_validation_check(
        &lint,
        report::CHECK_EXPORT_UNKNOWN_DIRECTIVE,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn visual_only_sections_survive_and_fenced_directives_remain_examples() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("directive-boundaries.md");
    let sources_path = repo_path("cli/tests/fixtures/pipeline/sources.csv");
    let evidence_path = repo_path("cli/tests/fixtures/pipeline/reviewed-evidence.jsonl");
    let baseline = current_human_report_document();
    let view_id = baseline
        .report
        .visual_views
        .first()
        .expect("the current final-report fixture should declare a visual")
        .id
        .clone();
    let baseline_claims = baseline
        .report
        .claims
        .iter()
        .map(|claim| claim.statement.as_str())
        .collect::<BTreeSet<_>>();
    let mut markdown =
        fs::read_to_string(repo_path("cli/tests/fixtures/pipeline/report.md")).unwrap();
    let original_visual_directive = format!("<!-- sok:visual-view {view_id} -->");
    assert_contains(&markdown, &original_visual_directive);
    markdown = markdown.replacen(&original_visual_directive, "", 1);
    markdown = markdown.replacen(
        "## Claims",
        r#"## Claims

~~~markdown
| Statement | Claim type | Evidence requirement | Source IDs |
|---|---|---|---|
| This currently fake row is only syntax. | currentness | reviewed_source | Example Source |
~~~"#,
        1,
    );

    let field_element_marker = "<!-- sok:surface field-elements -->";
    let marker_index = markdown
        .find(field_element_marker)
        .expect("the current fixture should declare its field-element surface");
    let insertion_index = markdown[..marker_index]
        .rfind("\n## ")
        .map(|index| index + 1)
        .expect("the field-element surface should have a public heading");
    let directive_examples = format!(
        r#"## Measurement Chain at a Glance
<!-- sok:visual-view {view_id} -->

## Directive Syntax Is Data

````markdown
## Backtick Example Is Not A Report Section
<!-- sok:surface claims -->
<!-- sok:visual-view missing-view -->
```
````

~~~markdown
## Tilde Example Is Not A Report Section
<!-- sok:surface claims -->
<!-- sok:visual-view another-missing-view -->
~~~

"#
    );
    markdown.insert_str(insertion_index, &directive_examples);
    fs::write(&report_path, markdown).unwrap();

    let lint = report::lint_markdown_report(
        &report_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    assert!(!lint
        .diagnostics
        .checks
        .iter()
        .any(|check| check.check_id == report::CHECK_VALIDATE_CURRENTNESS_PROSE));

    let exported = report::export_markdown_report(
        &report_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    let presentation = exported.report.presentation.as_ref().unwrap();
    let visual_section = presentation
        .sections
        .iter()
        .find(|section| section.title == "Measurement Chain at a Glance")
        .unwrap();
    assert_eq!(visual_section.visual_view_ids, vec![view_id.clone()]);
    let example_section = presentation
        .sections
        .iter()
        .find(|section| section.title == "Directive Syntax Is Data")
        .unwrap();
    assert_contains(&example_section.body_markdown, "sok:surface claims");
    assert_contains(
        &example_section.body_markdown,
        "sok:visual-view missing-view",
    );
    assert_contains(
        &example_section.body_markdown,
        "## Backtick Example Is Not A Report Section",
    );
    assert_contains(
        &example_section.body_markdown,
        "## Tilde Example Is Not A Report Section",
    );
    assert!(example_section.visual_view_ids.is_empty());
    assert!(presentation.sections.iter().all(|section| section.title
        != "Backtick Example Is Not A Report Section"
        && section.title != "Tilde Example Is Not A Report Section"));
    let exported_claims = exported
        .report
        .claims
        .iter()
        .map(|claim| claim.statement.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        exported_claims, baseline_claims,
        "fenced table examples must not add or replace report claims"
    );

    let validation = report::validate_report_value(&serde_json::to_value(&exported).unwrap());
    assert_eq!(validation.error_count(), 0, "{:?}", validation.diagnostics);
    let html = report::render_html_report(&exported).unwrap();
    assert_contains(&html, "Measurement Chain at a Glance");
    assert_contains(&html, &format!("data-visual-mount=\"{view_id}\""));
    assert_contains(&html, "sok:surface claims");
}

#[test]
fn presentation_ids_and_visual_placement_are_strictly_validated() {
    let mut invalid_id = current_human_report_value();
    invalid_id["report"]["presentation"] = json!({
        "thesis": "Topology is organized by transformations and invariants.",
        "organizing_form": "Transformation path",
        "rationale": "The path follows the field's load-bearing relations.",
        "sections": [
            {
                "id": "BAD ID",
                "title": "First section",
                "visual_view_ids": ["view-topology-dependency-path"]
            }
        ]
    });
    let validation = report::validate_report_value(&invalid_id);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SCHEMA_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut reserved_id = current_human_report_value();
    reserved_id["report"]["presentation"] = json!({
        "thesis": "Topology is organized by transformations and invariants.",
        "organizing_form": "Transformation path",
        "rationale": "The path follows the field's load-bearing relations.",
        "sections": [
            {
                "id": "evidence-appendix",
                "title": "Conflicting section"
            }
        ]
    });
    let validation = report::validate_report_value(&reserved_id);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SCHEMA_REQUIRED,
        report::DiagnosticSeverity::Error,
    );
    assert!(
        validation
            .diagnostics
            .checks
            .iter()
            .any(|check| check.message.contains("reserved by the HTML renderer")),
        "{:?}",
        validation.diagnostics
    );

    let mut duplicate_visual = invalid_id;
    duplicate_visual["report"]["presentation"]["sections"] = json!([
        {
            "id": "section-first",
            "title": "First section",
            "visual_view_ids": ["view-topology-dependency-path"]
        },
        {
            "id": "section-second",
            "title": "Second section",
            "visual_view_ids": ["view-topology-dependency-path"]
        }
    ]);
    let validation = report::validate_report_value(&duplicate_visual);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );
}
