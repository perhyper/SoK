use super::*;

#[test]
fn run_manifest_validator_rejects_unknown_versions_and_short_hashes() {
    let fixture = run_manifest::RunFile::new(
        "bounded-input",
        repo_path("cli/tests/fixtures/run-manifest/bounded-input.txt"),
    );
    let mut spec = run_manifest::RunManifestSpec::new("fixture-command");
    spec.stage = "fixture".to_string();
    spec.profile_version = "1.0.0".to_string();
    spec.started_at = "2026-07-16T00:00:00Z".to_string();
    spec.finished_at = "2026-07-16T00:00:01Z".to_string();
    spec.declared_permissions =
        run_manifest::declared_permissions_for_files(std::slice::from_ref(&fixture), &[], false);
    spec.bounded_input_files = vec![fixture.clone()];
    spec.semantic_payloads = vec![run_manifest::SemanticPayload::from_text(
        "fixture",
        "semantic",
        "stable semantic payload",
    )];
    let manifest = run_manifest::build_run_manifest(spec).unwrap();
    assert_eq!(manifest.schema_version, "sok-run/v1");
    assert_eq!(
        manifest.bounded_input_files[0].sha256,
        "a5ac294458d1e41db8cdd8420503bdf4feaddef9b167e565b3a9b84c3328664c"
    );
    assert_eq!(manifest.bounded_input_files[0].sha256.len(), 64);

    let mut unknown_version = manifest.clone();
    unknown_version.schema_version = "sok-run/v99".to_string();
    assert_contains(
        &run_manifest::validate_run_manifest(&unknown_version)
            .unwrap_err()
            .to_string(),
        "unsupported run manifest schema_version",
    );

    let mut short_hash = manifest.clone();
    short_hash.bounded_input_files[0].sha256 = "a5ac294458d1e41d".to_string();
    assert_contains(
        &run_manifest::validate_run_manifest(&short_hash)
            .unwrap_err()
            .to_string(),
        "full SHA-256",
    );
}

#[test]
fn run_manifest_semantic_digest_ignores_timestamps() {
    let fixture = run_manifest::RunFile::new(
        "bounded-input",
        repo_path("cli/tests/fixtures/run-manifest/bounded-input.txt"),
    );
    let mut first = run_manifest::RunManifestSpec::new("fixture-command");
    first.stage = "fixture".to_string();
    first.profile_version = "1.0.0".to_string();
    first.started_at = "2026-07-16T00:00:00Z".to_string();
    first.finished_at = "2026-07-16T00:00:01Z".to_string();
    first.declared_permissions =
        run_manifest::declared_permissions_for_files(std::slice::from_ref(&fixture), &[], false);
    first.bounded_input_files = vec![fixture.clone()];
    first.semantic_payloads = vec![run_manifest::SemanticPayload::from_text(
        "fixture",
        "semantic",
        "stable semantic payload",
    )];

    let mut second = first.clone();
    second.started_at = "2026-07-17T01:02:03Z".to_string();
    second.finished_at = "2026-07-17T01:02:04Z".to_string();

    let first = run_manifest::build_run_manifest(first).unwrap();
    let second = run_manifest::build_run_manifest(second).unwrap();
    assert_ne!(first.timestamps, second.timestamps);
    assert_eq!(first.semantic_digest, second.semantic_digest);
}

#[test]
fn conformance_fixtures_score_without_provider_calls() {
    let fixture_dir = repo_path("cli/tests/fixtures/conformance");
    let report = crate::eval::score_conformance_dir(&fixture_dir).unwrap();

    assert_eq!(report.schema_version, "sok-conformance-report/v1");
    assert_eq!(report.suite_id, "sok-harness-hardening-v1");
    assert!(report.all_expectations_passed(), "{report:#?}");
    assert!(report.validation_passed > 0);
    assert!(report.validation_failed > 0);
    assert_eq!(report.validation_pass_rate, "5/18");
    assert_eq!(report.repair_count, 3);
    assert_eq!(report.human_correction_count, 1);

    let contracts = report
        .cases
        .iter()
        .map(|case| case.contract.as_str())
        .collect::<BTreeSet<_>>();
    for contract in [
        "core conformance",
        "stable-ID uniqueness",
        "reference closure",
        "relation validity",
        "evidence support",
        "currentness metadata",
        "public/internal boundary",
        "multilingual retention",
        "prerequisite violations",
        "source-role coverage",
        "validation pass rate",
        "unknown report schema versions",
        "unknown core schema versions",
        "unknown run schema versions",
        "unknown work schema versions",
        "run manifests",
        "WorkOrder sidecars",
        "WorkResult capability response",
    ] {
        assert!(
            contracts.contains(contract),
            "conformance suite should cover {contract}"
        );
    }

    let public_boundary = report
        .cases
        .iter()
        .find(|case| case.id == "public-internal-boundary")
        .unwrap();
    assert!(!public_boundary.actual_valid);
    assert_eq!(public_boundary.human_correction.count, 1);
    assert_eq!(public_boundary.repair_count, 0);

    let output_dir = tempfile::tempdir().unwrap();
    let output_path = output_dir.path().join("conformance-report.json");
    let exit = run_cli(vec![
        "eval".to_string(),
        "conformance".to_string(),
        "--fixtures".to_string(),
        fixture_dir.display().to_string(),
        "--output".to_string(),
        output_path.display().to_string(),
    ])
    .unwrap();
    assert_eq!(exit, 0);
    let saved: crate::eval::ConformanceReport = report::read_json_file(&output_path).unwrap();
    assert_eq!(saved.expectation_failed, 0);
    assert_eq!(saved.repair_count, report.repair_count);
}

#[test]
fn optional_run_manifests_are_private_sidecars_for_report_and_html_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let output_json_path = dir.path().join("sok-report.json");
    let output_html_path = dir.path().join("sok-report.html");
    let export_manifest_path = dir.path().join("private-export-run.json");
    let render_manifest_path = dir.path().join("private-render-run.json");
    let report_path = repo_path("cli/tests/fixtures/pipeline/report.md");
    let sources_path = repo_path("cli/tests/fixtures/pipeline/sources.csv");
    let evidence_path = repo_path("cli/tests/fixtures/pipeline/reviewed-evidence.jsonl");

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
            "--run-manifest".to_string(),
            export_manifest_path.display().to_string(),
        ])
        .unwrap(),
        0
    );

    let export_manifest = run_manifest::read_run_manifest(&export_manifest_path).unwrap();
    assert_eq!(export_manifest.schema_version, "sok-run/v1");
    assert_eq!(export_manifest.command.name, "export-json");
    assert_eq!(export_manifest.stage, "final");
    assert!(export_manifest
        .bounded_input_files
        .iter()
        .any(|file| file.kind == "markdown-report" && file.sha256.len() == 64));
    assert!(export_manifest
        .bounded_input_files
        .iter()
        .any(|file| file.kind == "source-manifest" && file.sha256.len() == 64));
    assert!(export_manifest
        .bounded_input_files
        .iter()
        .any(|file| file.kind == "evidence-jsonl" && file.sha256.len() == 64));
    assert!(export_manifest
        .output_files
        .iter()
        .any(|file| file.kind == "sok-report" && file.sha256.len() == 64));
    assert!(export_manifest.work_order_hashes.is_empty());
    assert!(export_manifest.work_result_hashes.is_empty());

    let report_json = fs::read_to_string(&output_json_path).unwrap();
    assert_not_contains(&report_json, "sok-run/v1");
    assert_not_contains(&report_json, "private-export-run.json");
    assert_not_contains(&report_json, "run_id");
    assert_not_contains(&report_json, "executor");
    assert_not_contains(&report_json, "declared_permissions");

    assert_eq!(
        run_cli(vec![
            "render-html".to_string(),
            "--input".to_string(),
            output_json_path.display().to_string(),
            "--output".to_string(),
            output_html_path.display().to_string(),
            "--run-manifest".to_string(),
            render_manifest_path.display().to_string(),
        ])
        .unwrap(),
        0
    );
    let render_manifest = run_manifest::read_run_manifest(&render_manifest_path).unwrap();
    assert_eq!(render_manifest.command.name, "render-html");
    assert!(render_manifest
        .bounded_input_files
        .iter()
        .any(|file| file.kind == "sok-report" && file.sha256.len() == 64));
    assert!(render_manifest
        .output_files
        .iter()
        .any(|file| file.kind == "html-report" && file.sha256.len() == 64));

    let html = fs::read_to_string(&output_html_path).unwrap();
    assert_not_contains(&html, "sok-run/v1");
    assert_not_contains(&html, "private-render-run.json");
    assert_not_contains(&html, "run_id");
    assert_not_contains(&html, "executor");
    assert_not_contains(&html, "declared_permissions");
}
