use super::*;

fn disputed_report() -> Value {
    let mut value = current_human_report_value();
    let source_id = value["report"]["sources"][0]["id"].clone();
    let claim = &mut value["report"]["claims"][0];
    let mut opposing = json!({"source_id":source_id, "verification_status":"reviewed",
        "support_kind":"contradicts", "reviewed_at":"2026-10-25", "locator":"Section 3"});
    opposing["support_kind"] = json!("contradicts");
    opposing["support_note"] = json!("Counterexample under a different boundary condition.");
    opposing["evidence_id"] = json!("evidence-counterexample-1111111111");
    claim["conflict_review"] = json!({
        "explanation": "The conclusion is conditional; the counterexample remains unresolved.",
        "reviewed_at": opposing["reviewed_at"], "source_ids": [opposing["source_id"]]
    });
    claim["evidence_links"]
        .as_array_mut()
        .unwrap()
        .push(opposing);
    value
}

#[test]
fn reviewed_disputes_preserve_opposition_without_counting_it_as_support() {
    let value = disputed_report();
    let result = report::validate_report_value(&value);
    assert_eq!(
        result.error_count() + result.warning_count(),
        0,
        "{:?}",
        result.diagnostics
    );
    let document: report::ReportDocument = serde_json::from_value(value.clone()).unwrap();
    let projected = crate::core::projection::project_report_document(&document).unwrap();
    assert_eq!(projected.report.claims[0], document.report.claims[0]);
    let mut missing_review = value.clone();
    missing_review["report"]["claims"][0]
        .as_object_mut()
        .unwrap()
        .remove("conflict_review");
    assert!(report::validate_report_value(&missing_review).has_warnings());
    for review in [
        json!({"explanation":"", "reviewed_at":"2026-01-01", "source_ids":[]}),
        json!({"explanation":"Reviewed", "reviewed_at":"not-a-date", "source_ids":[]}),
    ] {
        let mut invalid = value.clone();
        invalid["report"]["claims"][0]["conflict_review"] = review;
        assert!(report::validate_report_value(&invalid).has_errors());
    }
    let mut only_opposing = value.clone();
    let links = only_opposing["report"]["claims"][0]["evidence_links"]
        .as_array_mut()
        .unwrap();
    links.retain(|link| link["support_kind"] == "contradicts");
    only_opposing["report"]["claims"][0]["evidence_requirement"] = json!("reviewed_source");
    assert!(report::validate_report_value(&only_opposing).has_errors());
    let mut stale = value;
    stale["report"]["claims"][0]["conflict_review"]["reviewed_at"] = json!("2000-01-01");
    assert!(report::validate_report_value(&stale).has_errors());
}

#[test]
fn relation_meaning_and_direction_survive_core_round_trips() {
    let base = json!({"id":"rel-context-1111111111",
        "from":{"entity_type":"concept", "id":"a"},
        "to":{"entity_type":"concept", "id":"b"}});
    for (kind, canonical, reverse) in [
        ("supports", "supports", false),
        ("part_of", "part_of", false),
        ("represented_by", "represented_by", false),
        ("qualifies", "qualifies", false),
        ("contradicts", "contradicts", false),
        ("supported_by", "supports", true),
        ("contains", "part_of", true),
        ("depends_on", "requires_before", true),
        ("prerequisite_for", "requires_before", false),
    ] {
        let mut value = base.clone();
        value["kind"] = json!(kind);
        let core: crate::core::relations::Relation = serde_json::from_value(value.clone()).unwrap();
        let public: report::Relation = serde_json::from_value(value).unwrap();
        let output = serde_json::to_value(core).unwrap();
        assert_eq!(output, serde_json::to_value(public).unwrap());
        assert_eq!(output["kind"], canonical);
        assert_eq!(output["from"]["id"], if reverse { "b" } else { "a" });
    }
    let mut document = current_human_report_document();
    let mut distinct = document.report.relations[0].clone();
    distinct.id = "rel-another-context-1111111111".into();
    distinct.description = "Only under additional assumptions.".into();
    document.report.relations.push(distinct.clone());
    let projected = crate::core::projection::project_report_document(&document).unwrap();
    assert_eq!(
        projected.report.relations.len(),
        document.report.relations.len()
    );
    assert!(projected.report.relations.contains(&distinct));
}

#[test]
fn graph_scopes_entities_keeps_evidence_and_requires_reviewed_identity_links() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("collection.json");
    let output = dir.path().join("graph.json");
    fs::write(
        dir.path().join("report.json"),
        serde_json::to_vec(&disputed_report()).unwrap(),
    )
    .unwrap();
    let mut manifest = json!({"schema_version":"sok-graph-input/v1", "reports":[
        {"report_id":"first", "path":"report.json"},
        {"report_id":"second", "path":"report.json"}
    ]});
    let save = |value: &Value| fs::write(&input, serde_json::to_vec(value).unwrap()).unwrap();
    save(&manifest);
    assert_eq!(
        run_cli(vec![
            "export-graph".into(),
            "--input".into(),
            input.display().to_string(),
            "--output".into(),
            output.display().to_string()
        ])
        .unwrap(),
        0
    );
    let first_bytes = fs::read(&output).unwrap();
    crate::graph_snapshot::export(&input, &output).unwrap();
    assert_eq!(fs::read(&output).unwrap(), first_bytes);
    let graph: Value = serde_json::from_slice(&first_bytes).unwrap();
    let nodes = graph["nodes"].as_array().unwrap();
    let left = &nodes[0]["key"];
    let right = nodes
        .iter()
        .find(|n| {
            n["key"]["report_id"] == "second"
                && n["key"]["id"] == left["id"]
                && n["key"]["entity_type"] == left["entity_type"]
        })
        .unwrap();
    assert_ne!(*left, right["key"]);
    assert!(graph["assertions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["kind"] == "contradicts"));
    let source = nodes
        .iter()
        .find(|n| n["key"]["entity_type"] == "source")
        .unwrap();
    manifest["identity_links"] = json!([{"from":left, "to":right["key"],
        "kind":"equivalent_to", "explanation":"Reviewed definitions in both scopes.",
        "reviewer":"test reviewer", "reviewed_at":"2026-10-25", "source_refs":[source["key"]]}]);
    save(&manifest);
    crate::graph_snapshot::export(&input, &output).unwrap();
    let linked: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(linked["nodes"], graph["nodes"]); // Equivalence never merges nodes.
    for field in ["reviewer", "explanation", "reviewed_at"] {
        let mut invalid = manifest.clone();
        invalid["identity_links"][0][field] = json!("");
        save(&invalid);
        assert!(crate::graph_snapshot::export(&input, &output).is_err());
    }
    let mut missing_source = manifest.clone();
    missing_source["identity_links"][0]["source_refs"] = json!([]);
    save(&missing_source);
    assert!(crate::graph_snapshot::export(&input, &output).is_err());
    manifest["identity_links"][0]["from"]["revision"] = json!("stale");
    save(&manifest);
    let bytes = fs::read(&output).unwrap();
    assert!(crate::graph_snapshot::export(&input, &output).is_err());
    assert_eq!(bytes, fs::read(&output).unwrap());
    manifest["identity_links"] = json!([]);
    save(&manifest);
    assert!(crate::graph_snapshot::export(&input, &input).is_err());
    assert!(crate::graph_snapshot::export(&input, &dir.path().join("report.json")).is_err());
    manifest["reports"][1]["report_id"] = json!("first");
    save(&manifest);
    assert!(crate::graph_snapshot::export(&input, &output).is_err());
}

#[test]
fn graph_rejects_unvalidated_reports_and_unknown_manifest_contracts() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.json");
    let output = dir.path().join("graph.json");
    let mut value = disputed_report();
    value["report"]["claims"][0]
        .as_object_mut()
        .unwrap()
        .remove("conflict_review");
    fs::write(
        dir.path().join("report.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    let manifest = json!({"schema_version":"sok-graph-input/v1",
        "reports":[{"report_id":"a", "path":"report.json"}]});
    fs::write(&input, serde_json::to_vec(&manifest).unwrap()).unwrap();
    fs::write(&output, "keep existing snapshot").unwrap();
    assert!(crate::graph_snapshot::export(&input, &output).is_err());
    assert_eq!(
        fs::read_to_string(&output).unwrap(),
        "keep existing snapshot"
    );
    for field in ["unknown", "schema_version"] {
        let mut invalid = manifest.clone();
        invalid[field] = json!("unsupported");
        fs::write(&input, serde_json::to_vec(&invalid).unwrap()).unwrap();
        assert!(crate::graph_snapshot::export(&input, &output).is_err());
    }
}

#[test]
fn relation_schemas_cover_every_canonical_mapping() {
    let report_schema = load_repo_json("specs/sok-report.schema.json");
    let core_schema = load_repo_json("specs/sok-knowledge.schema.json");
    let labels = core_schema["$defs"]["relation_kind"]["enum"]
        .as_array()
        .unwrap();
    assert_eq!(labels.len(), 20);
    for label in labels {
        let mapping =
            crate::core::relations::relation_kind_mapping(label.as_str().unwrap()).unwrap();
        assert!(!mapping.reverse_endpoints);
        assert_eq!(serde_json::to_value(mapping.kind).unwrap(), *label);
        assert!(
            report_schema["$defs"]["relation"]["properties"]["kind"]["enum"]
                .as_array()
                .unwrap()
                .contains(label)
        );
    }
    for path in [
        "specs/sok-graph-input.schema.json",
        "specs/sok-graph.schema.json",
    ] {
        let schema = load_repo_json(path);
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("schema_version")));
    }
}
