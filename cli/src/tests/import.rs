use super::*;

#[test]
fn source_manifest_normalization_creates_cataloged_report_sources_and_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("sources.csv");
    fs::write(
        &csv_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
Open Paper,review_article,doi:10.0000/open,https://example.test/open,2024-01-02,cc_by,Official URL,$0,CC BY,foundation / method,Explains the method.,Read before module 1,Reviewed metadata only\n\
Mystery Source,dataset,,,date after lookup,unknown,,budget unknown,,dataset,,,\n",
    )
    .unwrap();

    let normalized = report::normalize_source_manifest(&csv_path).unwrap();
    assert_eq!(normalized.sources.len(), 2);
    assert_eq!(normalized.evidence.len(), 2);

    let open = &normalized.sources[0];
    assert!(open.id.starts_with("src-open-paper-"));
    assert_eq!(
        open.verification_status,
        report::VerificationStatus::Cataloged
    );
    assert_eq!(open.access.status, report::AccessStatus::OpenAccess);
    assert_eq!(open.access.metadata_only, Some(false));
    assert_eq!(open.access.license, "CC BY");
    assert_eq!(open.date, "2024-01-02");
    assert!(open.roles.contains(&report::SourceRole::Foundation));
    assert!(open.roles.contains(&report::SourceRole::Method));

    let cataloged = &normalized.evidence[0];
    assert_eq!(
        cataloged.verification_status,
        report::VerificationStatus::Cataloged
    );
    assert_eq!(cataloged.support_kind, report::SupportKind::Background);
    assert!(cataloged.claim_ids.is_empty());
    assert!(cataloged.locator.is_empty());
    assert!(cataloged.support_note.is_empty());
    assert!(!cataloged.can_satisfy_claim_link());
    assert!(cataloged.to_evidence_link().is_none());
    assert_contains(&cataloged.notes, "not been read");

    assert!(normalized
        .diagnostics
        .iter()
        .any(|check| check.check_id == report::CHECK_SOURCE_MISSING_ACCESS_STATUS));
    assert!(normalized
        .diagnostics
        .iter()
        .any(|check| check.check_id == report::CHECK_SOURCE_MISSING_ACCESS_ROUTE));
}

#[test]
fn ingest_last_writes_cataloged_evidence_jsonl_from_csv_and_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("sources.csv");
    let output = dir.path().join("evidence.jsonl");
    fs::write(
        &csv_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
Open Paper,review_article,doi:10.0000/open,https://example.test/open,2024-01-02,cc_by,Official URL,$0,CC BY,foundation / method,Explains the method.,Read before module 1,Reviewed metadata only\n\
Mystery Source,dataset,,,date after lookup,unknown,,budget unknown,,dataset,,,\n",
    )
    .unwrap();
    fs::write(&output, "stale evidence that must be overwritten\n").unwrap();

    let normalized = ingest_last_manifest(
        &csv_path.display().to_string(),
        &output.display().to_string(),
    )
    .unwrap();
    assert_eq!(normalized.evidence.len(), 2);
    assert!(normalized
        .diagnostics
        .iter()
        .any(|check| check.check_id == report::CHECK_SOURCE_MISSING_ACCESS_STATUS));
    assert!(normalized
        .diagnostics
        .iter()
        .any(|check| check.check_id == report::CHECK_SOURCE_MISSING_ACCESS_ROUTE));

    let contents = fs::read_to_string(&output).unwrap();
    assert_not_contains(&contents, "stale evidence");
    let lines = contents.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    for line in &lines {
        let value: Value = serde_json::from_str(line).unwrap();
        assert!(value.is_object());
        assert!(value
            .get("evidence_id")
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with("ev-"));
        assert_eq!(value["verification_status"], json!("cataloged"));
        assert_eq!(value["claim_ids"], json!([]));
        assert!(value["source_access"].is_object());
    }

    let entries: Vec<report::EvidenceEntry> = report::read_jsonl_file(&output).unwrap();
    let open = &entries[0];
    assert_eq!(
        open.verification_status,
        report::VerificationStatus::Cataloged
    );
    assert_eq!(open.support_kind, report::SupportKind::Background);
    assert!(open.locator.is_empty());
    assert!(open.support_note.is_empty());
    assert!(open.claim_ids.is_empty());
    assert!(!open.can_satisfy_claim_link());
    assert!(open.to_evidence_link().is_none());
    assert_contains(
        &open.notes,
        "not been read, crawled, or externally verified",
    );
    assert_eq!(
        open.input_provenance.input_path,
        csv_path.display().to_string()
    );
    assert_eq!(open.input_provenance.input_kind, "source_manifest_csv");
    assert_eq!(open.input_provenance.row_number, Some(1));
    assert_eq!(open.input_provenance.row_hash.len(), 16);
    assert!(open.evidence_id.starts_with("ev-src-open-paper-"));
    assert!(open.source_id.starts_with("src-open-paper-"));
    assert!(open.source_roles.contains(&report::SourceRole::Foundation));
    assert!(open.source_roles.contains(&report::SourceRole::Method));
    assert_eq!(open.source_identifier, "doi:10.0000/open");
    assert_eq!(open.source_url, "https://example.test/open");
    assert_eq!(open.curricular_use, "Read before module 1");
    let access = open.source_access.as_ref().unwrap();
    assert_eq!(access.status, report::AccessStatus::OpenAccess);
    assert_eq!(access.route, "Official URL");
    assert_eq!(access.budget_estimate, "$0");
    assert_eq!(access.license, "CC BY");

    let repeat_output = dir.path().join("evidence-repeat.jsonl");
    let repeat = ingest_last_manifest(
        &csv_path.display().to_string(),
        &repeat_output.display().to_string(),
    )
    .unwrap();
    assert_eq!(
        normalized
            .evidence
            .iter()
            .map(|entry| (
                &entry.evidence_id,
                &entry.source_id,
                &entry.input_provenance.row_hash
            ))
            .collect::<Vec<_>>(),
        repeat
            .evidence
            .iter()
            .map(|entry| (
                &entry.evidence_id,
                &entry.source_id,
                &entry.input_provenance.row_hash
            ))
            .collect::<Vec<_>>()
    );
}

#[test]
fn ingest_last_accepts_manifest_alias_and_requires_input_and_output_flags() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = dir.path().join("sources.csv");
    let output = dir.path().join("evidence.jsonl");
    fs::write(
        &manifest,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
Open Paper,review_article,doi:10.0000/open,https://example.test/open,2024-01-02,open_access,Official URL,$0,CC BY,foundation,Explains the method.,Read before module 1,Reviewed metadata only\n",
    )
    .unwrap();

    run_cli(vec![
        "ingest".to_string(),
        "last".to_string(),
        "--manifest".to_string(),
        manifest.display().to_string(),
        "--output".to_string(),
        output.display().to_string(),
    ])
    .unwrap();
    let entries: Vec<report::EvidenceEntry> = report::read_jsonl_file(&output).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].verification_status,
        report::VerificationStatus::Cataloged
    );

    let missing_sources = run_cli(vec![
        "ingest".to_string(),
        "last".to_string(),
        "--output".to_string(),
        output.display().to_string(),
    ])
    .unwrap_err()
    .to_string();
    assert_contains(&missing_sources, "--sources is required");

    let missing_output = run_cli(vec![
        "ingest".to_string(),
        "last".to_string(),
        "--sources".to_string(),
        manifest.display().to_string(),
    ])
    .unwrap_err()
    .to_string();
    assert_contains(&missing_output, "--output is required");

    let conflicting_input = run_cli(vec![
        "ingest".to_string(),
        "last".to_string(),
        "--sources".to_string(),
        manifest.display().to_string(),
        "--manifest".to_string(),
        dir.path().join("other.csv").display().to_string(),
        "--output".to_string(),
        output.display().to_string(),
    ])
    .unwrap_err()
    .to_string();
    assert_contains(
        &conflicting_input,
        "use only one of --sources or --manifest",
    );
}

#[test]
fn export_json_from_generated_scaffold_keeps_internal_context_private() {
    let dir = tempfile::tempdir().unwrap();
    let scaffold_path = dir.path().join("scaffold.md");
    let sources_path = dir.path().join("sources.csv");
    let evidence_path = dir.path().join("evidence.jsonl");
    let output_path = dir.path().join("sok-report.json");

    let scaffold = build_report_scaffold(
        "topology",
        "SENTINEL_INTERNAL_LEARNER",
        "SENTINEL_INTERNAL_GOAL",
        12,
    )
    .replace(
        "| Why only provisional | ",
        "| Why only provisional | SENTINEL_INTERNAL_ASSUMPTION: ",
    );
    fs::write(&scaffold_path, scaffold).unwrap();
    fs::write(
        &sources_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
Open Topology Notes,notes,https://example.test/topology,https://example.test/topology,2024-01-02,open_access,Official URL,$0,CC BY,foundation,Introduces the field vocabulary.,Read before module 1,Metadata fixture\n",
    )
    .unwrap();
    ingest_last_manifest(
        &sources_path.display().to_string(),
        &evidence_path.display().to_string(),
    )
    .unwrap();

    let missing_stage = run_cli(vec![
        "export-json".to_string(),
        "--scaffold".to_string(),
        scaffold_path.display().to_string(),
        "--sources".to_string(),
        sources_path.display().to_string(),
        "--output".to_string(),
        output_path.display().to_string(),
    ])
    .unwrap_err()
    .to_string();
    assert_contains(&missing_stage, "--stage is required");

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
        output_path.display().to_string(),
    ])
    .unwrap();

    let exported: report::ReportDocument = report::read_json_file(&output_path).unwrap();
    assert_eq!(exported.metadata.report_type, report::ReportType::Scaffold);
    assert_eq!(exported.metadata.schema_version, "sok-report/v2");
    assert_eq!(exported.report.field, "topology");
    assert!(!exported.report.sources.is_empty());
    assert!(exported.diagnostics.is_some());

    let internal = exported.internal_context.as_ref().unwrap();
    assert_eq!(internal.raw_learner_profile, "SENTINEL_INTERNAL_LEARNER");
    assert_eq!(internal.original_goal, "SENTINEL_INTERNAL_GOAL");
    assert!(internal.placeholder_state.contains_key("placeholder_lines"));
    assert_eq!(internal.profile_proposals.len(), 1);
    let proposal = &internal.profile_proposals[0];
    assert_eq!(
        proposal.proposer_type,
        report::ProfileProposerType::Inference
    );
    assert_eq!(proposal.profile_id, "formal");
    assert_eq!(proposal.profile_version, "1.0.0");
    assert_eq!(proposal.locale, "en");
    assert_eq!(proposal.confidence, "medium");
    assert_contains(&proposal.rationale, "provisional until source review");
    assert!(internal
        .prompt_derived_assumptions
        .iter()
        .any(|note| note.contains("SENTINEL_INTERNAL_ASSUMPTION")));
    assert!(internal
        .handoff_notes
        .iter()
        .any(|note| note.contains("field name selected a provisional")));

    let public_payload = serde_json::to_string(&exported.report).unwrap();
    assert_not_contains(&public_payload, "SENTINEL_INTERNAL_LEARNER");
    assert_not_contains(&public_payload, "SENTINEL_INTERNAL_GOAL");
    assert_not_contains(&public_payload, "SENTINEL_INTERNAL_ASSUMPTION");
    assert_not_contains(&public_payload, "source to verify");
    assert_not_contains(&public_payload, "date after lookup");
    assert_not_contains(&public_payload, "raw_learner_profile");
    assert_not_contains(&public_payload, "original_goal");
}

#[test]
fn export_json_reports_unresolved_structured_markdown_references() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("broken-relations.md");
    let sources_path = dir.path().join("sources.csv");
    fs::write(
        &sources_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
Open Review,review_article,doi:10.0000/open,https://example.test/open,2025-01-01,open_access,Official URL,$0,CC BY,foundation,Supports coupled-system framing.,Use in module 1,Reviewed metadata.\n",
    )
    .unwrap();
    fs::write(
        &report_path,
r#"# Structure of Knowledge: Solid-State Batteries

## Report Architecture

| Item | Decision |
|---|---|
| Executive thesis | Solid-state battery evidence depends on coupled transport and cell-design relations. |
| Chosen organizing form | A coupled-system path. |
| Architecture rationale | The relation under test is a dependency, so the report follows that dependency explicitly. |
| Rejected alternatives and why | A component inventory was rejected because it would not expose the dependency under test. |

## Domain Decomposition

Solid-state batteries couple transport and cell design.

## Field Element Inventory

| Element class | Observed element | Actual form in this field | Role | Load-bearing relations | Source IDs | Confidence |
|---|---|---|---|---|---|---|
| Coupled objects | Core objects | Solid electrolytes and interfaces. | core | Interfaces couple transport and cell design. | Open Review | high |
| Boundary concept | Open Review | A concept label that intentionally collides with a source title. | context | Tests typed-reference ambiguity. | Open Review | high |
| Method and warrant | Methods and warrants | Measurement warrants. | core | Warrants qualify measurements. | Open Review | high |
| Representation | Representations | Nyquist plots. | surrounding | Plots represent impedance behavior. | Open Review | high |

## Curriculum Roadmap

| Phase | Module | Essential question | Readings | Practice artifact | Progress criteria | Prerequisites |
|---|---|---|---|---|---|---|
| 1 | Electrochemical grammar | What fixes cell behavior? | Open Review | Cell model | Explain losses | Open Review |

## Relations

| Relation ID | Relation kind | From type | From reference | To type | To reference | Rationale |
|---|---|---|---|---|---|---|
| rel-missing-endpoint | depends_on | concept | Missing concept | curriculum_step | Electrochemical grammar | This endpoint should not resolve. |

## Visual Summary

This summary has no structured visual view to attach to.
"#,
    )
    .unwrap();

    let exported = report::export_markdown_report(
        &report_path,
        &sources_path,
        None::<&PathBuf>,
        report::ExportStage::Final,
    )
    .unwrap();

    assert!(exported.report.relations.is_empty());
    assert!(exported.report.visual_views.is_empty());
    let diagnostics = exported.diagnostics.as_ref().unwrap();
    assert!(diagnostics
        .checks
        .iter()
        .any(|check| check.check_id == report::CHECK_EXPORT_UNRESOLVED_REFERENCE));
    assert!(diagnostics
        .checks
        .iter()
        .any(|check| check.check_id == report::CHECK_EXPORT_AMBIGUOUS_REFERENCE));
    assert!(diagnostics.checks.iter().any(|check| {
        check.check_id == report::CHECK_EXPORT_UNSUPPORTED_SECTION
            && check.message.contains("Visual Summary")
    }));
}

#[test]
fn export_json_resolves_unicode_aliases_without_losing_ascii_safe_ids() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("unicode-report.md");
    let sources_path = dir.path().join("sources.csv");
    fs::write(
        &sources_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
양자 센싱 입문,review_article,doi:10.0000/ko,https://example.test/ko,2025-01-01,open_access,Official URL,$0,CC BY,foundation,한국어 source alias resolution fixture.,Use in module 1,Reviewed metadata.\n",
    )
    .unwrap();
    fs::write(
        &report_path,
        r#"# Structure of Knowledge: 양자 센싱

## Report Architecture

| Item | Decision |
|---|---|
| Executive thesis | 양자 센싱은 신호 모델과 추정기의 관계로 설명된다. |
| Chosen organizing form | A relation-centered path. |
| Architecture rationale | The report follows the relation under test. |
| Rejected alternatives and why | A glossary-only form would hide the relation. |

## Domain Decomposition

양자 센싱은 신호 모델과 추정기를 함께 다룬다.

## Field Element Inventory

| Element class | Observed element | Actual form in this field | Role | Load-bearing relations | Source IDs | Confidence |
|---|---|---|---|---|---|---|
| Concept | 신호 모델 | 관측 신호를 수학적으로 표현한다. | core | 추정기가 이 모델을 사용한다. | 양자 센싱 입문 | high |
| Method | 추정기 | 신호 모델에서 파라미터를 추정한다. | surrounding | 신호 모델을 적용한다. | 양자 센싱 입문 | high |

## Relations

| Relation ID | Relation kind | From type | From reference | To type | To reference | Rationale |
|---|---|---|---|---|---|---|
| rel-korean-reference-1111111111 | uses_method | field_element | 추정기 | field_element | 신호 모델 | Korean labels should resolve through Unicode-aware aliases. |
"#,
    )
    .unwrap();

    let exported = report::export_markdown_report(
        &report_path,
        &sources_path,
        None::<&PathBuf>,
        report::ExportStage::Final,
    )
    .unwrap();

    let source_id = &exported.report.sources[0].id;
    assert!(validate_stable_id(source_id).is_ok());
    assert!(exported
        .report
        .field_elements
        .iter()
        .all(|element| element.source_ids == vec![source_id.clone()]));
    let signal_model = exported
        .report
        .field_elements
        .iter()
        .find(|element| element.label == "신호 모델")
        .unwrap();
    assert!(signal_model.id.starts_with("element-item-"));
    validate_stable_id(&signal_model.id).unwrap();
    let relation = exported.report.relations.first().unwrap();
    assert_eq!(relation.to.id, signal_model.id);
    assert!(exported
        .diagnostics
        .as_ref()
        .map(|diagnostics| diagnostics.checks.iter().all(|check| {
            check.check_id != report::CHECK_EXPORT_UNRESOLVED_REFERENCE
                && check.check_id != report::CHECK_EXPORT_AMBIGUOUS_REFERENCE
        }))
        .unwrap_or(true));
}

#[test]
fn export_json_reports_ambiguous_and_dangling_unicode_source_aliases() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("ambiguous-source-report.md");
    let sources_path = dir.path().join("sources.csv");
    fs::write(
        &sources_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
공통 제목,review_article,doi:10.0000/a,https://example.test/a,2025-01-01,open_access,Official URL,$0,CC BY,foundation,First duplicate alias.,Use in module 1,Reviewed metadata.\n\
공통 제목,review_article,doi:10.0000/b,https://example.test/b,2025-01-02,open_access,Official URL,$0,CC BY,foundation,Second duplicate alias.,Use in module 1,Reviewed metadata.\n",
    )
    .unwrap();
    fs::write(
        &report_path,
        r#"# Structure of Knowledge: 중복 alias

## Field Element Inventory

| Element class | Observed element | Actual form in this field | Role | Load-bearing relations | Source IDs | Confidence |
|---|---|---|---|---|---|---|
| Concept | 공통 개념 | 중복 source alias와 dangling alias를 검사한다. | core | source resolution must not guess. | 공통 제목; 없는 자료 | high |
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
    let diagnostics = exported.diagnostics.as_ref().unwrap();
    let ambiguous = diagnostics
        .checks
        .iter()
        .find(|check| check.check_id == report::CHECK_EXPORT_AMBIGUOUS_REFERENCE)
        .unwrap();
    assert_eq!(ambiguous.severity, report::DiagnosticSeverity::Error);
    assert_contains(&ambiguous.message, "공통 제목");
    assert_contains(&ambiguous.message, "src-");
    let dangling = diagnostics
        .checks
        .iter()
        .find(|check| check.check_id == report::CHECK_EXPORT_UNRESOLVED_REFERENCE)
        .unwrap();
    assert_contains(&dangling.message, "없는 자료");
    assert!(exported.report.field_elements[0].source_ids.is_empty());
    let validation = report::validate_report_value(&serde_json::to_value(&exported).unwrap());
    assert_validation_check(
        &validation,
        report::CHECK_EXPORT_AMBIGUOUS_REFERENCE,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn reviewed_or_verified_evidence_needs_usable_support_metadata() {
    let reviewed = report::EvidenceEntry {
        evidence_id: "ev-example-reviewed-1111111111".to_string(),
        source_id: "src-example-source-1111111111".to_string(),
        verification_status: report::VerificationStatus::Reviewed,
        support_kind: report::SupportKind::Supports,
        locator: "chapter 2".to_string(),
        support_note: String::new(),
        reviewed_at: "2026-07-16".to_string(),
        claim_ids: vec!["claim-example-1111111111".to_string()],
        ..report::EvidenceEntry::default()
    };
    assert!(reviewed.can_satisfy_claim_link());
    let link = reviewed.to_evidence_link().unwrap();
    assert_eq!(link.source_id, reviewed.source_id);
    assert_eq!(link.reviewed_at, "2026-07-16");

    let cataloged = report::EvidenceEntry {
        verification_status: report::VerificationStatus::Cataloged,
        ..reviewed.clone()
    };
    assert!(!cataloged.can_satisfy_claim_link());
    assert!(cataloged.to_evidence_link().is_none());
    assert!(cataloged.to_visible_evidence_link().is_some());

    let no_locator_or_note = report::EvidenceEntry {
        locator: String::new(),
        support_note: String::new(),
        ..reviewed.clone()
    };
    assert!(!no_locator_or_note.can_satisfy_claim_link());

    let background = report::EvidenceEntry {
        support_kind: report::SupportKind::Background,
        ..reviewed.clone()
    };
    assert!(!background.can_satisfy_claim_link());

    let qualifies = report::EvidenceEntry {
        support_kind: report::SupportKind::Qualifies,
        ..reviewed.clone()
    };
    assert!(!qualifies.can_satisfy_claim_link());
    assert!(qualifies.to_visible_evidence_link().is_some());

    let contradicts = report::EvidenceEntry {
        support_kind: report::SupportKind::Contradicts,
        ..reviewed.clone()
    };
    assert!(!contradicts.can_satisfy_claim_link());
    assert!(contradicts.to_visible_evidence_link().is_some());

    let missing_reviewed_at = report::EvidenceEntry {
        reviewed_at: String::new(),
        ..reviewed.clone()
    };
    assert!(!missing_reviewed_at.can_satisfy_claim_link());

    let verified_with_note = report::EvidenceEntry {
        verification_status: report::VerificationStatus::Verified,
        locator: String::new(),
        support_note: "Directly checks the claim.".to_string(),
        ..reviewed
    };
    assert!(verified_with_note.can_satisfy_claim_link());
}
