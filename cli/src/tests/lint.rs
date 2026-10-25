use super::*;

#[test]
fn lint_scaffold_accepts_generated_scaffold_with_expected_internal_notes() {
    let dir = tempfile::tempdir().unwrap();
    let scaffold_path = dir.path().join("scaffold.md");
    let sources_path = dir.path().join("sources.csv");
    let evidence_path = dir.path().join("evidence.jsonl");

    fs::write(
        &scaffold_path,
        build_report_scaffold("topology", "doctoral learner", "map the field", 8),
    )
    .unwrap();
    write_sources_csv(&sources_path).unwrap();
    ingest_last_manifest(
        &sources_path.display().to_string(),
        &evidence_path.display().to_string(),
    )
    .unwrap();

    let lint = report::lint_markdown_report(
        &scaffold_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Scaffold,
    )
    .unwrap();
    assert_eq!(lint.error_count(), 0);
    assert_validation_check(
        &lint,
        report::CHECK_LINT_SCAFFOLD_UNRESOLVED,
        report::DiagnosticSeverity::Info,
    );

    let exit = run_cli(vec![
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
    .unwrap();
    assert_eq!(exit, 0);
}

#[test]
fn lint_final_accepts_canonical_human_markdown_with_reviewed_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let (report_path, sources_path, evidence_path, _) = write_lint_final_bundle(dir.path(), true);

    let lint = report::lint_markdown_report(
        &report_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    assert_eq!(lint.error_count(), 0);
    assert_eq!(lint.warning_count(), 0);

    let exit = run_cli(vec![
        "lint".to_string(),
        "--stage".to_string(),
        "final".to_string(),
        "--report".to_string(),
        report_path.display().to_string(),
        "--sources".to_string(),
        sources_path.display().to_string(),
        "--evidence".to_string(),
        evidence_path.display().to_string(),
    ])
    .unwrap();
    assert_eq!(exit, 0);
}

#[test]
fn lint_final_rejects_leaked_scaffold_notes_and_placeholders() {
    let dir = tempfile::tempdir().unwrap();
    let (report_path, sources_path, evidence_path, claim_statement) =
        write_lint_final_bundle(dir.path(), true);
    fs::write(
        &report_path,
        format!(
            "{}\n\n## Scaffold Quality Notes\n\n- source to verify\n- raw prompt intent should stay private\n",
            canonical_lint_final_markdown(&claim_statement)
        ),
    )
    .unwrap();

    let lint = report::lint_markdown_report(
        &report_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    assert_validation_check(
        &lint,
        report::CHECK_LINT_FINAL_PUBLIC_LEAKAGE,
        report::DiagnosticSeverity::Error,
    );

    let exit = run_cli(vec![
        "lint".to_string(),
        "--stage".to_string(),
        "final".to_string(),
        "--report".to_string(),
        report_path.display().to_string(),
        "--sources".to_string(),
        sources_path.display().to_string(),
        "--evidence".to_string(),
        evidence_path.display().to_string(),
    ])
    .unwrap();
    assert_eq!(exit, 1);
}

#[test]
fn lint_reports_missing_source_access_metadata_without_failing_default_scaffold() {
    let dir = tempfile::tempdir().unwrap();
    let scaffold_path = dir.path().join("scaffold.md");
    let sources_path = dir.path().join("sources.csv");
    fs::write(
        &scaffold_path,
        build_report_scaffold("topology", "doctoral learner", "map the field", 8),
    )
    .unwrap();
    fs::write(
        &sources_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
Mystery Source,paper,,,,,,,unknown,foundation,Needs review,,\n",
    )
    .unwrap();

    let lint = report::lint_markdown_report(
        &scaffold_path,
        &sources_path,
        None::<&PathBuf>,
        report::ExportStage::Scaffold,
    )
    .unwrap();
    assert_eq!(lint.error_count(), 0);
    assert_validation_check(
        &lint,
        report::CHECK_SOURCE_MISSING_ACCESS_STATUS,
        report::DiagnosticSeverity::Warning,
    );
    assert_validation_check(
        &lint,
        report::CHECK_SOURCE_MISSING_ACCESS_ROUTE,
        report::DiagnosticSeverity::Warning,
    );

    let exit = run_cli(vec![
        "lint".to_string(),
        "--stage".to_string(),
        "scaffold".to_string(),
        "--report".to_string(),
        scaffold_path.display().to_string(),
        "--sources".to_string(),
        sources_path.display().to_string(),
    ])
    .unwrap();
    assert_eq!(exit, 0);
}

#[test]
fn lint_warns_when_cataloged_evidence_cannot_support_claims() {
    let dir = tempfile::tempdir().unwrap();
    let (report_path, sources_path, _, _) = write_lint_final_bundle(dir.path(), false);

    let lint = report::lint_markdown_report(
        &report_path,
        &sources_path,
        None::<&PathBuf>,
        report::ExportStage::Final,
    )
    .unwrap();
    assert_eq!(lint.error_count(), 0);
    assert_validation_check(
        &lint,
        report::CHECK_EVIDENCE_CATALOGED_ONLY,
        report::DiagnosticSeverity::Warning,
    );

    let strict_exit = run_cli(vec![
        "lint".to_string(),
        "--stage".to_string(),
        "final".to_string(),
        "--report".to_string(),
        report_path.display().to_string(),
        "--sources".to_string(),
        sources_path.display().to_string(),
        "--strict".to_string(),
    ])
    .unwrap();
    assert_eq!(strict_exit, 1);
}

#[test]
fn lint_rejects_evidence_rows_that_reference_unknown_sources() {
    let dir = tempfile::tempdir().unwrap();
    let (report_path, sources_path, evidence_path, claim_statement) =
        write_lint_final_bundle(dir.path(), false);
    let claim_id = report::content_id("claim", &[&claim_statement]);
    let bad = report::EvidenceEntry {
        evidence_id: "ev-bad-source-1111111111".to_string(),
        source_id: "src-missing-source-1111111111".to_string(),
        verification_status: report::VerificationStatus::Reviewed,
        support_kind: report::SupportKind::Supports,
        locator: "section 1".to_string(),
        support_note: "References a source outside the manifest.".to_string(),
        reviewed_at: "2026-07-16".to_string(),
        claim_ids: vec![claim_id],
        ..report::EvidenceEntry::default()
    };
    report::write_jsonl_file(&evidence_path, &[bad]).unwrap();

    let lint = report::lint_markdown_report(
        &report_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    assert_validation_check(
        &lint,
        report::CHECK_EXPORT_UNKNOWN_EVIDENCE_SOURCE,
        report::DiagnosticSeverity::Error,
    );

    let exit = run_cli(vec![
        "lint".to_string(),
        "--stage".to_string(),
        "final".to_string(),
        "--report".to_string(),
        report_path.display().to_string(),
        "--sources".to_string(),
        sources_path.display().to_string(),
        "--evidence".to_string(),
        evidence_path.display().to_string(),
    ])
    .unwrap();
    assert_eq!(exit, 1);
}

#[test]
fn lint_flags_currentness_prose_without_dates() {
    let dir = tempfile::tempdir().unwrap();
    let (report_path, sources_path, evidence_path, claim_statement) =
        write_lint_final_bundle(dir.path(), true);
    fs::write(
        &report_path,
        format!(
            "{}\n\nThe latest classification remains unsettled without a dated review marker.\n",
            canonical_lint_final_markdown(&claim_statement)
        ),
    )
    .unwrap();

    let lint = report::lint_markdown_report(
        &report_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    assert_validation_check(
        &lint,
        report::CHECK_VALIDATE_CURRENTNESS_PROSE,
        report::DiagnosticSeverity::Warning,
    );
}

#[test]
fn lint_currentness_ignores_common_technical_current_compounds() {
    let dir = tempfile::tempdir().unwrap();
    let (report_path, sources_path, evidence_path, claim_statement) =
        write_lint_final_bundle(dir.path(), true);
    fs::write(
        &report_path,
        format!(
            "{}\n\nCurrent density, current collector design, current focusing, stripping \
                current, and critical current are technical quantities in this paragraph.\n",
            canonical_lint_final_markdown(&claim_statement)
        ),
    )
    .unwrap();

    let lint = report::lint_markdown_report(
        &report_path,
        &sources_path,
        Some(&evidence_path),
        report::ExportStage::Final,
    )
    .unwrap();
    assert_eq!(lint.error_count(), 0, "{:?}", lint.diagnostics);
    assert_no_validation_check(
        &lint,
        report::CHECK_VALIDATE_CURRENTNESS_PROSE,
        report::DiagnosticSeverity::Warning,
    );
}
