use super::*;

#[test]
fn version_identifies_the_rust_cli() {
    let version = version_string();
    assert!(version.starts_with("sok "));
    assert!(version.ends_with(" (rust)"));
}

#[test]
fn agent_docs_resolve_and_verify_the_rust_cli() {
    let skill = fs::read_to_string(repo_path("structure-of-knowledge/SKILL.md")).unwrap();
    let collaboration = fs::read_to_string(repo_path(
        "structure-of-knowledge/references/agent-collaboration.md",
    ))
    .unwrap();

    for document in [&skill, &collaboration] {
        assert_contains(document, "cli-workflow.md");
        assert_not_contains(document, "cd structure-of-knowledge");
    }
    let workflow = fs::read_to_string(repo_path(
        "structure-of-knowledge/references/cli-workflow.md",
    ))
    .unwrap();
    for required in [
        "${CODEX_HOME:-$HOME/.codex}/skills/structure-of-knowledge/bin/sok",
        "--version",
        "(rust)",
        "cli/target/release/sok",
        "<command> --help",
        "header-only",
        "validate-report",
        "--strict",
    ] {
        assert_contains(&workflow, required);
    }
}

#[test]
fn every_documented_command_supports_immediate_help() {
    for command in [
        "init",
        "brief",
        "scaffold",
        "handoff-report",
        "source-template",
        "audit-sources",
        "download-sources",
        "ingest",
        "export-json",
        "lint",
        "validate-report",
        "migrate-ids",
        "eval",
        "render-html",
        "specificity",
        "work",
    ] {
        assert_eq!(
            run_cli(vec![command.to_string(), "--help".to_string()]).unwrap(),
            0,
            "{command} --help should succeed"
        );
        assert_eq!(
            run_cli(vec!["help".to_string(), command.to_string()]).unwrap(),
            0,
            "help {command} should succeed"
        );
    }
    assert_eq!(
        run_cli(vec![
            "ingest".to_string(),
            "last".to_string(),
            "--help".to_string(),
        ])
        .unwrap(),
        0
    );
}

#[test]
fn require_field_rejects_blank_values() {
    for field in ["", "   ", "\n\t"] {
        assert!(require_field(field).is_err());
    }
    require_field("history of mathematics").unwrap();
}

#[test]
fn validate_mode_lists_sorted_modes() {
    for mode in ["research", "textbook", "model-tuning", "curriculum"] {
        validate_mode(mode).unwrap();
    }
    let err = validate_mode("survey").unwrap_err().to_string();
    assert_contains(&err, "invalid --mode \"survey\"");
    assert_contains(&err, "curriculum, model-tuning, research, textbook");
}

#[test]
fn scaffold_inference_and_handoff_work() {
    let title = "# Structure of Knowledge: causal inference\n\nbody";
    assert_eq!(infer_field_from_scaffold(title), "causal inference");

    let table = "| Target learner | doctoral engineer |\n| Goal | read current papers |\n| Time budget | 14 \
        weeks |\n";
    assert_eq!(
        infer_scaffold_table_value(table, &["Learner", "Target learner"]),
        "doctoral engineer"
    );
    assert_eq!(
        infer_scaffold_table_value(table, &["Goal"]),
        "read current papers"
    );
    assert_eq!(infer_weeks_from_scaffold(table), 14);

    let scaffold =
        "# Structure of Knowledge: particle physics\n\n## Research Frame\n\n| Field | particle \
        physics |\n";
    let handoff = build_human_report_handoff(
        "particle physics",
        "physics-adjacent engineer",
        "complete a report",
        12,
        scaffold,
    );
    assert_contains(&handoff, "# SoK Human-Report Handoff: particle physics");
    assert_contains(&handoff, "Generated: ");
    assert_contains(&handoff, "- Learner profile: physics-adjacent engineer");
    assert_contains(
        &handoff,
        "Do not render this internal context as a front-matter table",
    );
    assert_contains(&handoff, "Do not include sections named `Research Frame`");
    assert_contains(
        &handoff,
        "select only relation-backed visuals that clarify the chosen architecture",
    );
    assert_not_contains(&handoff, "Mermaid");
    assert_contains(&handoff, "````markdown");
    assert_contains(&handoff, scaffold);
}

#[test]
fn brief_tasks_and_specificity_match_existing_contract() {
    let brief = build_agent_brief(
        "computational drug discovery",
        "pharmacist",
        "design a corpus",
        "model-tuning",
        8,
        "jsonl",
    );
    assert_contains(&brief, "# SoK Agent Brief: computational drug discovery");
    assert_contains(&brief, "Mode: model-tuning");
    assert_contains(
        &brief,
        "Mode description: Plan a legally usable domain corpus and evaluation set for model adaptation.",
    );
    assert_contains(&brief, "Time budget: 8 weeks");
    assert_contains(&brief, "Requested output format: jsonl");
    assert_contains(
        &brief,
        "- Legally usable corpus plan with license/access audit",
    );

    let without_weeks = build_agent_brief(
        "logic",
        "student",
        "learn proofs",
        "research",
        0,
        "markdown",
    );
    assert_contains(&without_weeks, "Time budget: not fixed");

    let tasks = build_tasks("textbook");
    assert_contains(&tasks, "- [ ] Textbook thesis and reader model");
    assert_contains(&tasks, "- [ ] Exercise ladder and worked examples plan");

    let checklist = build_specificity_checklist("comparative constitutional law");
    assert_contains(
        &checklist,
        "# Specificity Checklist: comparative constitutional law",
    );
    assert_contains(
        &checklist,
        "Which paid or paywalled sources are metadata-only?",
    );
    assert_contains(&checklist, "Does every source have access metadata?");
}

#[test]
fn source_template_is_header_only_and_csv_json_manifests_load() {
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("sources.csv");
    write_sources_csv(&csv_path).unwrap();
    let sources = load_sources(&csv_path).unwrap();
    assert!(sources.is_empty());
    let template = fs::read_to_string(&csv_path).unwrap();
    assert_eq!(template.lines().count(), 1);
    assert_contains(&template, "title,type,identifier,url,date,access_status");
    assert_not_contains(&template, "example.org");
    assert_not_contains(&template, "Replace this row");

    let tsv_path = dir.path().join("sources.tsv");
    fs::write(
        &tsv_path,
        "title\ttype\tidentifier\turl\taccess_status\taccess_route\tbudget_estimate\tlicense\tlayer\twhy_it_matters\tuse_in_curriculum\tnotes\nOpen paper\tpaper\tdoi:10.0000/example\thttps://example.test/paper.pdf\tfree_web\tOfficial URL\t$0\tcc_by\tfoundation\tExplains the method.\tUse in Module 2.\tReviewed.\n",
    )
    .unwrap();
    let tsv_sources = load_sources(&tsv_path).unwrap();
    assert_eq!(tsv_sources[0].title, "Open paper");
    assert_eq!(tsv_sources[0].source_type, "paper");
    assert_eq!(tsv_sources[0].why_it_matters, "Explains the method.");

    let json_path = dir.path().join("sources.json");
    fs::write(
        &json_path,
        r#"[
  {
    "title": "Nested access paper",
    "type": "paper",
    "identifier": "arXiv:2501.00001",
    "url": "https://example.test/nested.pdf",
    "access_status": "paywalled",
    "access_route": "Library",
    "access": {
      "status": "official_open",
      "route": "https://official.example.test/nested.pdf",
      "budget_estimate": "$0",
      "notes": "Official repository"
    },
    "why_it_matters": "Canonical introduction."
  }
]"#,
    )
    .unwrap();
    let json_sources = load_sources(&json_path).unwrap();
    let access = source_access(&json_sources[0]);
    assert_eq!(access.status, "official_open");
    assert_eq!(access.route, "https://official.example.test/nested.pdf");

    let wrapped_path = dir.path().join("wrapped.json");
    fs::write(
        &wrapped_path,
        r#"{"sources":[{"title":"Wrapped source","type":"book","identifier":"isbn:0000000000","access_status":"paid_book","access_route":"Library purchase","budget_estimate":"$45","why_it_matters":"Core reference."}]}"#,
    )
    .unwrap();
    assert_eq!(
        load_sources(&wrapped_path).unwrap()[0].title,
        "Wrapped source"
    );

    let txt_path = dir.path().join("sources.txt");
    fs::write(&txt_path, "title\n").unwrap();
    assert_contains(
        &load_sources(&txt_path).unwrap_err().to_string(),
        "manifest must be .json, .csv, or .tsv",
    );
}

#[test]
fn report_models_deserialize_schema_fixtures() {
    let final_report = current_human_report_document();
    assert_eq!(
        final_report.metadata.report_type,
        report::ReportType::HumanReport
    );
    assert_eq!(final_report.metadata.schema_version, "sok-report/v2");
    let reviewed_link = final_report
        .report
        .claims
        .iter()
        .flat_map(|claim| claim.evidence_links.iter())
        .find(|link| {
            link.verification_status == report::VerificationStatus::Reviewed
                && link.support_kind == report::SupportKind::Supports
        })
        .expect("current report should preserve reviewed supporting evidence");

    let encoded_link = serde_json::to_string(reviewed_link).unwrap();
    assert_contains(&encoded_link, "\"source_id\"");
    assert_contains(&encoded_link, "\"support_kind\"");
    assert_not_contains(&encoded_link, "sourceId");

    let scaffold = current_scaffold_report_document();
    assert_eq!(scaffold.metadata.schema_version, "sok-report/v2");
    assert_eq!(scaffold.metadata.report_type, report::ReportType::Scaffold);
    assert!(scaffold.internal_context.is_some());
    assert!(scaffold.diagnostics.is_some());
}
