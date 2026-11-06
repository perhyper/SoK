use super::*;

#[test]
fn content_ids_are_content_derived_and_order_independent() {
    let first = report::content_id("claim", &["  Persistent   Homology! "]);
    let second = report::content_id("claim", &["persistent homology"]);
    assert_eq!(first, second);
    assert_eq!(first, "claim-persistent-homology-c48a3a31c5");
    assert!(first.starts_with("claim-persistent-homology-"));

    let left = report::content_id_map("src", ["Alpha Source", "Beta Source"]);
    let right = report::content_id_map("src", ["Beta Source", "Alpha Source"]);
    assert_eq!(left, right);
    assert_eq!(
        left.get("alpha source").map(String::as_str),
        Some("src-alpha-source-5a23ba1b22")
    );
    assert_eq!(
        left.get("beta source").map(String::as_str),
        Some("src-beta-source-f7c3e018ea")
    );

    let source_a = Source {
        title: "Alpha Source".to_string(),
        identifier: "doi:10.0000/alpha".to_string(),
        ..Source::default()
    };
    let source_b = Source {
        title: "Beta Source".to_string(),
        identifier: "doi:10.0000/beta".to_string(),
        ..Source::default()
    };
    let ids_forward = report_source_ids_by_title(&[source_a.clone(), source_b.clone()]);
    let ids_reverse = report_source_ids_by_title(&[source_b, source_a]);
    assert_eq!(ids_forward, ids_reverse);
}

#[test]
fn content_ids_are_unicode_safe_and_identity_hashes_use_normalized_utf8() {
    let korean = report::content_id_identity("element", &["\u{c591}\u{c790} \u{c13c}\u{c2f1}"]);
    let korean_nfd = report::content_id("element", &["\u{110b}\u{1163}\u{11bc}\u{110c}\u{1161} \u{1109}\u{1166}\u{11ab}\u{1109}\u{1175}\u{11bc}"]);
    assert_eq!(korean.stable_id, korean_nfd);
    assert_eq!(korean.display_slug, "item");
    assert_eq!(
        korean.normalized_identity,
        "\u{c591}\u{c790} \u{c13c}\u{c2f1}"
    );
    validate_stable_id(&korean.stable_id).unwrap();

    let composed = report::content_id("concept", &["\u{ac00}"]);
    let decomposed = report::content_id("concept", &["\u{1100}\u{1161}"]);
    assert_eq!(composed, decomposed);

    let greek_upper = report::content_id("concept", &["Μέθοδος"]);
    let greek_lower = report::content_id("concept", &["μέθοδος"]);
    assert_eq!(greek_upper, greek_lower);

    let korean_concept = report::content_id("concept", &["\u{c591}\u{c790} \u{c13c}\u{c2f1}"]);
    let cjk = report::content_id("concept", &["量子センシング"]);
    let greek = report::content_id("concept", &["μέθοδος"]);
    let mixed = report::content_id("concept", &["Graph \u{adf8}\u{b798}\u{d504}"]);
    let ids = BTreeSet::from([korean_concept, cjk, greek, mixed]);
    assert_eq!(ids.len(), 4);
    assert!(ids.iter().all(|id| validate_stable_id(id).is_ok()));
    assert!(ids.iter().any(|id| id.starts_with("concept-graph-")));
}

#[test]
fn json_and_jsonl_helpers_round_trip_with_clear_errors() {
    let dir = tempfile::tempdir().unwrap();
    let diagnostic = report::DiagnosticCheck::warning(
        "test.warning",
        "A warning that can be serialized as shared diagnostics.",
    );
    let json_path = dir.path().join("diagnostic.json");
    report::write_json_file(&json_path, &diagnostic).unwrap();
    let decoded: report::DiagnosticCheck = report::read_json_file(&json_path).unwrap();
    assert_eq!(decoded, diagnostic);

    let entry = report::EvidenceEntry {
        evidence_id: "ev-jsonl-example-1111111111".to_string(),
        source_id: "src-jsonl-example-1111111111".to_string(),
        verification_status: report::VerificationStatus::Reviewed,
        support_kind: report::SupportKind::Supports,
        locator: "p. 1".to_string(),
        notes: "Reviewed from bounded test input.".to_string(),
        claim_ids: vec!["claim-jsonl-example-1111111111".to_string()],
        ..report::EvidenceEntry::default()
    };
    let jsonl_path = dir.path().join("evidence.jsonl");
    report::write_jsonl_file(&jsonl_path, std::slice::from_ref(&entry)).unwrap();
    let round_trip: Vec<report::EvidenceEntry> = report::read_jsonl_file(&jsonl_path).unwrap();
    assert_eq!(round_trip, vec![entry]);

    let bad_jsonl_path = dir.path().join("bad.jsonl");
    fs::write(&bad_jsonl_path, "{}\nnot-json\n").unwrap();
    let err = report::read_jsonl_file::<serde_json::Value, _>(&bad_jsonl_path)
        .unwrap_err()
        .to_string();
    assert_contains(&err, "line 2");
}

#[test]
fn migrate_ids_writes_ordered_maps_for_v1_and_v2_without_mutating_input() {
    let dir = tempfile::tempdir().unwrap();
    let v2_input = dir.path().join("unicode-v2-report.json");
    let v2_output = dir.path().join("unicode-v2-id-map.json");
    let mut v2 = current_human_report_value();
    v2["report"]["field_elements"][0]["id"] = json!("element-item-4a33eacd5f");
    v2["report"]["field_elements"][0]["label"] = json!("\u{c591}\u{c790} \u{c13c}\u{c2f1}");
    report::write_json_file(&v2_input, &v2).unwrap();
    let before = fs::read_to_string(&v2_input).unwrap();

    let exit = run_cli(vec![
        "migrate-ids".to_string(),
        "--input".to_string(),
        v2_input.display().to_string(),
        "--output".to_string(),
        v2_output.display().to_string(),
    ])
    .unwrap();
    assert_eq!(exit, 0);
    assert_eq!(fs::read_to_string(&v2_input).unwrap(), before);

    let migration: report::IdMigrationDocument = report::read_json_file(&v2_output).unwrap();
    assert_eq!(migration.schema_version, "sok-id-migration/v1");
    assert_eq!(migration.source_schema_version, "sok-report/v2");
    let keys = migration
        .mappings
        .iter()
        .map(|entry| {
            (
                entry.old_id.clone(),
                entry.entity_type.clone(),
                entry.path.clone(),
            )
        })
        .collect::<Vec<_>>();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
    let unicode_entry = migration
        .mappings
        .iter()
        .find(|entry| entry.old_id == "element-item-4a33eacd5f")
        .unwrap();
    assert!(unicode_entry.changed);
    assert_ne!(unicode_entry.old_id, unicode_entry.new_id);
    assert!(unicode_entry.new_id.starts_with("element-item-"));
    assert_eq!(
        unicode_entry.normalized_identity,
        "\u{c591}\u{c790} \u{c13c}\u{c2f1}"
    );
    validate_stable_id(&unicode_entry.new_id).unwrap();

    let v1_input = dir.path().join("legacy-v1-report.json");
    let v1_output = dir.path().join("legacy-v1-id-map.json");
    report::write_json_file(&v1_input, &legacy_v1_report_value()).unwrap();
    let exit = run_cli(vec![
        "migrate-ids".to_string(),
        "--input".to_string(),
        v1_input.display().to_string(),
        "--output".to_string(),
        v1_output.display().to_string(),
    ])
    .unwrap();
    assert_eq!(exit, 0);
    let legacy_migration: report::IdMigrationDocument = report::read_json_file(&v1_output).unwrap();
    assert_eq!(legacy_migration.source_schema_version, "sok-report/v1");
}
