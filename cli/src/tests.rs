use super::*;
use crate::commands::{ingest_last_manifest, run_download_sources};
use crate::sources::sorted_open_access_statuses;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

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
        assert_contains(
            document,
            "${CODEX_HOME:-$HOME/.codex}/skills/structure-of-knowledge/bin/sok",
        );
        assert_contains(document, "--version");
        assert_contains(document, "(rust)");
        assert_contains(document, "cli/target/release/sok");
        assert_contains(document, "<command> --help");
        assert_contains(document, "header-only");
        assert_not_contains(document, "cd structure-of-knowledge");
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
        "render-html",
        "specificity",
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
fn report_scaffold_uses_domain_hints_for_discovery_not_final_sections() {
    let formal = build_report_scaffold(
        "algebraic topology",
        "doctoral mathematician",
        "enter current research",
        16,
    );
    assert_contains(
        &formal,
        "| Provisional lens | formal or theory-led candidate |",
    );
    assert_contains(&formal, "| Object grammar |");
    assert_contains(&formal, "Do not draft the final table of contents yet");
    assert_contains(&formal, "## Organizing Form Comparison");
    assert_contains(&formal, "## Report Architecture Decision");
    assert_contains(&formal, "| Architecture rationale |  |");
    assert_contains(&formal, "Rejected alternatives and why");
    assert_not_contains(&formal, "| Generative question |  |");
    assert_not_contains(&formal, "| Dependency or prerequisite graph |");
    assert_not_contains(&formal, "## 4. Concept and Prerequisite Map");
    assert_not_contains(&formal, "## Literature Ladder");
    assert_not_contains(&formal, "## Curriculum Roadmap");
    assert_not_contains(&formal, "```mermaid");
    assert_not_contains(&formal, "TODO");

    let ill = build_report_scaffold(
        "comparative constitutional law",
        "policy researcher",
        "read scholarly debates",
        10,
    );
    assert_contains(
        &ill,
        "| Provisional lens | interpretive or contested candidate |",
    );
    assert_contains(&ill, "| Cases and contexts |");
    assert_contains(&ill, "debate network, case constellation, genealogy");
    assert_not_contains(&ill, "## 4. Debate and Case Network");

    let infrastructure = build_report_scaffold(
        "particle physics",
        "physics-adjacent engineer",
        "map the field",
        12,
    );
    assert_contains(
        &infrastructure,
        "| Provisional lens | empirical or infrastructure-bound candidate |",
    );
    assert_contains(&infrastructure, "| Phenomenon-to-data chain |");
    assert_contains(
        &infrastructure,
        "Where does the object of study become an observation",
    );

    let mixed = build_report_scaffold("causal inference", "research lead", "build a path", 6);
    assert_contains(&mixed, "| Provisional lens | open or mixed candidate |");
    assert_contains(&mixed, "| Practices and warrants |");
    assert_contains(
        &mixed,
        "Which candidate organizing form survives comparison against the sources",
    );
}

#[test]
fn declarative_profiles_support_korean_signals_and_open_fallback() {
    assert_eq!(
        profiles::embedded_profile_schema_version(),
        "sok-discovery-profiles/v1"
    );

    let korean_formal = build_report_scaffold("위상수학", "doctoral learner", "map proofs", 8);
    assert_contains(
        &korean_formal,
        "| Provisional lens | formal or theory-led candidate |",
    );
    assert_contains(&korean_formal, "| Profile proposal | formal |");
    assert_contains(&korean_formal, "| Profile locale | ko |");
    assert_contains(
        &korean_formal,
        "Matched field-name signal \"위상수학\" for locale ko; provisional until source review.",
    );

    let korean_interpretive =
        build_report_scaffold("비교 헌법", "legal scholar", "compare cases", 4);
    assert_contains(
        &korean_interpretive,
        "| Provisional lens | interpretive or contested candidate |",
    );
    assert_contains(&korean_interpretive, "| Profile proposal | interpretive |");
    assert_contains(&korean_interpretive, "| Profile locale | ko |");

    let unknown = build_report_scaffold("zzqv untranslated domain", "reader", "orient", 0);
    assert_contains(&unknown, "| Provisional lens | open or mixed candidate |");
    assert_contains(&unknown, "| Profile proposal | open |");
    assert_contains(&unknown, "| Profile locale | und |");
    assert_contains(&unknown, "| Profile confidence | low |");
    assert_contains(
        &unknown,
        "No profile signal matched or the locale is unknown; keep the profile open and untyped until source review.",
    );

    let korean_unmatched = build_report_scaffold("방법론 일반론", "reader", "orient", 0);
    assert_contains(
        &korean_unmatched,
        "| Provisional lens | open or mixed candidate |",
    );
    assert_contains(&korean_unmatched, "| Profile locale | und |");
}

#[test]
fn explicit_profile_proposal_and_korean_projection_terms_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let sources_path = dir.path().join("sources.csv");
    fs::write(
        &sources_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
Open Korean Source,review,fixture:open-korean-source,https://example.test/korean,2026-01-01,open_access,Fixture URL,$0,CC BY,foundation,Supports Korean profile fixture.,Use for profile fixture,Fixture row\n",
    )
    .unwrap();

    let fixture_path = repo_path("cli/tests/fixtures/profiles/explicit-profile-scaffold.md");
    let exported = report::export_markdown_report(
        &fixture_path,
        &sources_path,
        None::<&PathBuf>,
        report::ExportStage::Scaffold,
    )
    .unwrap();

    let internal = exported.internal_context.as_ref().unwrap();
    assert_eq!(internal.profile_proposals.len(), 1);
    let proposal = &internal.profile_proposals[0];
    assert_eq!(proposal.proposer_type, report::ProfileProposerType::User);
    assert_eq!(proposal.profile_id, "interpretive");
    assert_eq!(proposal.profile_version, "1.0.0");
    assert_eq!(proposal.locale, "ko");
    assert_eq!(proposal.confidence, "0.72");
    assert_contains(
        &proposal.rationale,
        "case comparison and schools of interpretation",
    );

    assert!(exported
        .report
        .methods
        .iter()
        .any(|item| item.label == "판례 비교"));
    assert!(exported
        .report
        .representations
        .iter()
        .any(|item| item.label == "권리 분석 틀"));
    let compatibility_labels = exported
        .report
        .core_ideas
        .iter()
        .chain(exported.report.methods.iter())
        .chain(exported.report.representations.iter())
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert!(!compatibility_labels.contains(&"사법 적극주의 논쟁"));

    let validation = report::validate_report_value(&serde_json::to_value(&exported).unwrap());
    assert_eq!(validation.error_count(), 0);
}

#[test]
fn scaffold_inference_and_handoff_work() {
    let title = "# Structure of Knowledge: causal inference\n\nbody";
    assert_eq!(infer_field_from_scaffold(title), "causal inference");

    let table = "| Target learner | doctoral engineer |\n| Goal | read current papers |\n| Time budget | 14 weeks |\n";
    assert_eq!(
        infer_scaffold_table_value(table, &["Learner", "Target learner"]),
        "doctoral engineer"
    );
    assert_eq!(
        infer_scaffold_table_value(table, &["Goal"]),
        "read current papers"
    );
    assert_eq!(infer_weeks_from_scaffold(table), 14);

    let scaffold = "# Structure of Knowledge: particle physics\n\n## Research Frame\n\n| Field | particle physics |\n";
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
fn export_json_final_report_links_only_reviewed_or_verified_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("canonical-human-report.md");
    let sources_path = dir.path().join("sources.csv");
    let cataloged_output = dir.path().join("cataloged-report.json");
    let reviewed_output = dir.path().join("reviewed-report.json");
    let evidence_path = dir.path().join("evidence.jsonl");

    fs::write(
        &sources_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
Open Review,review_article,doi:10.0000/open,https://example.test/open,2025-01-01,open_access,Official URL,$0,CC BY,foundation / frontier,Supports invariant framing.,Use in core module,Reviewed metadata.\n",
    )
    .unwrap();

    let claim_statement = "Topology organizes learning around invariants and maps.";
    fs::write(
        &report_path,
        format!(
            r#"# Structure of Knowledge: Topology

## Research Frame

| Item | Value |
|---|---|
| Field | Topology |
| Learner | SENTINEL_FINAL_LEARNER |
| Goal | SENTINEL_FINAL_GOAL |
| Scope assumption | SENTINEL_FINAL_ASSUMPTION |

## Report Architecture

| Item | Decision |
|---|---|
| Executive thesis | Topology becomes legible through invariants, transformations, and counterexamples. |
| Chosen organizing form | A transformation-and-invariant path. |
| Architecture rationale | This path follows the field's load-bearing relations rather than a generic topic list. |
| Rejected alternatives and why | A subfield survey was rejected because it hides the transformation-and-invariant relation. |

## Domain Decomposition

Topology is organized around invariants, maps, and constructions that preserve structure under continuous deformation.

## Orientation

The report focuses on point-set foundations before algebraic examples.

## Field Element Inventory

| Element class | Observed element | Actual form in this field | Role | Load-bearing relations | Source IDs | Confidence |
|---|---|---|---|---|---|---|
| Object and transformation | Core objects | Spaces, continuous maps, quotient spaces, and invariants. | core | Maps act on spaces; invariants compare what maps preserve. | Open Review | high |
| Method and warrant | Methods and warrants | Proof by construction, counterexample, and functorial comparison. | core | Warrants test whether a proposed invariant or equivalence is valid. | Open Review | high |
| Representation | Representations | Commutative diagrams, chain complexes, and visual maps of spaces. | surrounding | Representations expose dependencies without replacing proof. | Open Review | high |
| Failure mode | Visual intuition as proof | Treating visual metaphors as definitions or assuming invariants are complete classifiers. | context | Counterexamples qualify intuitive claims. | Open Review | high |

## Source Role Probe

| Source role | Status | Candidate source pattern | What it tests | Waiver or revision rule |
|---|---|---|---|---|
| Foundation | Required | Standard text or review | Stabilizes definitions and proof grammar | Cannot be waived |
| Frontier | Conditional | Recent review | Needed only for applied frontier claims | Waive if no frontier section remains |

## Curriculum Roadmap

| Phase | Module | Essential question | Readings | Practice artifact | Progress criteria |
|---|---|---|---|---|---|
| Core | Invariance and maps | What does topology preserve? | Open Review | Compare metric and topological equivalence examples. | Explain maps; use examples |

## Claims

| Statement | Claim type | Evidence requirement | Source IDs | Confidence | Temporal status | Notes |
|---|---|---|---|---|---|---|
| {claim_statement} | structural | reviewed_source | Open Review | high | durable | Public claim. |

## Frontier, Debates, and Open Problems

| Problem or debate | Current state | Key sources | Required background | Why it is hard |
|---|---|---|---|---|
| Applied invariants | Persistent-style invariants test usefulness in noisy settings. | Open Review | Invariance | Connects theory and data. |
"#
        ),
    )
    .unwrap();

    let cataloged = report::export_markdown_report(
        &report_path,
        &sources_path,
        None::<&PathBuf>,
        report::ExportStage::Final,
    )
    .unwrap();
    report::write_json_file(&cataloged_output, &cataloged).unwrap();
    assert_eq!(
        cataloged.metadata.report_type,
        report::ReportType::HumanReport
    );
    assert!(cataloged.internal_context.is_none());
    let cataloged_claim = cataloged
        .report
        .claims
        .iter()
        .find(|claim| claim.statement == claim_statement)
        .unwrap();
    assert_eq!(cataloged_claim.evidence_links.len(), 1);
    assert_eq!(
        cataloged_claim.evidence_links[0].verification_status,
        report::VerificationStatus::Cataloged
    );
    assert!(cataloged
        .diagnostics
        .as_ref()
        .unwrap()
        .checks
        .iter()
        .any(|check| check.check_id == report::CHECK_EVIDENCE_CATALOGED_ONLY));
    assert!(cataloged
        .diagnostics
        .as_ref()
        .unwrap()
        .checks
        .iter()
        .any(|check| check.check_id == report::CHECK_EXPORT_INTERNAL_SECTION_IN_FINAL));

    let source_id = cataloged.report.sources[0].id.clone();
    let claim_id = report::content_id("claim", &[claim_statement]);
    let reviewed = report::EvidenceEntry {
        evidence_id: report::content_id("ev", &[&source_id, &claim_id, "reviewed"]),
        source_id: source_id.clone(),
        verification_status: report::VerificationStatus::Reviewed,
        support_kind: report::SupportKind::Supports,
        locator: "section 1".to_string(),
        support_note: "The source directly frames topology through invariants and maps."
            .to_string(),
        reviewed_at: "2026-07-16".to_string(),
        claim_ids: vec![claim_id.clone()],
        notes: "Reviewed from bounded test fixture.".to_string(),
        ..report::EvidenceEntry::default()
    };
    let cataloged_with_claim = report::EvidenceEntry {
        evidence_id: report::content_id("ev", &[&source_id, &claim_id, "cataloged"]),
        source_id: source_id.clone(),
        verification_status: report::VerificationStatus::Cataloged,
        support_kind: report::SupportKind::Supports,
        locator: "section 2".to_string(),
        support_note: "Cataloged rows must not satisfy claims.".to_string(),
        claim_ids: vec![claim_id],
        notes: "Cataloged candidate only.".to_string(),
        ..report::EvidenceEntry::default()
    };
    report::write_jsonl_file(&evidence_path, &[cataloged_with_claim, reviewed]).unwrap();

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
        reviewed_output.display().to_string(),
    ])
    .unwrap();

    let exported: report::ReportDocument = report::read_json_file(&reviewed_output).unwrap();
    assert!(exported.internal_context.is_none());
    assert_eq!(
        exported.report.sources[0].verification_status,
        report::VerificationStatus::Reviewed
    );
    assert_eq!(exported.report.sources[0].last_reviewed, "2026-07-16");
    let reviewed_claim = exported
        .report
        .claims
        .iter()
        .find(|claim| claim.statement == claim_statement)
        .unwrap();
    assert_eq!(reviewed_claim.evidence_links.len(), 2);
    assert!(
        reviewed_claim.evidence_links.iter().any(|link| {
            link.verification_status == report::VerificationStatus::Cataloged
                && link.source_id == source_id
        }),
        "cataloged claim evidence should remain visible but non-supporting"
    );
    assert!(reviewed_claim.evidence_links.iter().any(|link| {
        link.verification_status == report::VerificationStatus::Reviewed
            && link.source_id == source_id
    }));

    let public_payload = serde_json::to_string(&exported.report).unwrap();
    assert_not_contains(&public_payload, "raw_learner_profile");
    assert_not_contains(&public_payload, "original_goal");
    assert_not_contains(&public_payload, "Scaffold Quality Notes");
    assert_not_contains(&public_payload, "SENTINEL_FINAL_LEARNER");
    assert_not_contains(&public_payload, "SENTINEL_FINAL_GOAL");
    assert_not_contains(&public_payload, "SENTINEL_FINAL_ASSUMPTION");
}

#[test]
fn export_json_preserves_structured_markdown_knowledge_surfaces() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("solid-state-battery-shaped-report.md");
    let sources_path = dir.path().join("sources.csv");

    fs::write(
        &sources_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
Open Review,review_article,doi:10.0000/open,https://example.test/open,2025-01-01,open_access,Official URL,$0,CC BY,foundation,Supports coupled-system framing.,Use in module 1,Reviewed metadata.\n\
Interface Study,article,doi:10.0000/interface,https://example.test/interface,2024-06-01,open_access,Official URL,$0,CC BY,method,Explains interface measurements.,Use in module 2,Reviewed metadata.\n",
    )
    .unwrap();

    fs::write(
        &report_path,
r#"# Structure of Knowledge: Solid-State Batteries

## Report Architecture

| Item | Decision |
|---|---|
| Executive thesis | Solid-state battery claims become legible when transport, interfaces, mechanics, and cell conditions are read as a coupled system. |
| Chosen organizing form | A coupled-system dependency path. |
| Architecture rationale | This architecture follows the dependencies behind measurement claims instead of sorting the field by material class. |
| Rejected alternatives and why | A material-class taxonomy was rejected because it separates coupled transport, interface, and cell conditions. |

## Domain Decomposition

Solid-state batteries are an emerging, interdisciplinary field whose practical structure couples transport, interfaces, mechanics, cell architecture, and manufacturing constraints.

## Orientation

Read the field as a coupled system rather than as an electrolyte-conductivity ranking.

## Field Element Inventory

| Element class | Observed element | Actual form in this field | Role | Load-bearing relations | Source IDs | Confidence |
|---|---|---|---|---|---|---|
| Coupled material and cell objects | Core objects | Solid electrolytes, interfaces, electrodes, defects, and cell fixtures. | core | Material properties acquire meaning only within interfaces and cell conditions. | Open Review | high |
| Method and warrant | Methods and warrants | Impedance claims require geometry, density, electrodes, temperature, fitting, and replication details. | core | Measurement warrants qualify comparisons among core objects. | Interface Study | high |
| Representation | Representations | Arrhenius plots, Nyquist plots, cross-sections, pressure-capacity maps, and process-flow diagrams. | surrounding | Representations expose coupled transport and interface behavior. | Interface Study | high |

## Literature Ladder

| Layer | Start here | Read for | Do not infer | Source IDs |
|---|---|---|---|---|
| Foundation | Open Review | Coupled transport-interface-mechanics-cell framing | That conductivity alone yields a practical cell | Open Review |

## Curriculum Roadmap

| Phase | Module | Essential question | Readings | Practice artifact | Progress criteria | Prerequisites |
|---|---|---|---|---|---|---|
| 1 | Electrochemical grammar | What fixes voltage, transport, and polarization in a cell? | Open Review | Annotated cell model | Explain coupled losses | Core objects |
| 2 | Interface measurement | What does each instrument warrant? | Interface Study | EIS reporting sheet | State controls and uncertainty | Electrochemical grammar; Methods and warrants |

## Relations

| Relation ID | Relation kind | From type | From reference | To type | To reference | Rationale | Source IDs |
|---|---|---|---|---|---|---|---|
| rel-interface-measurement-after-grammar | depends_on | curriculum_step | Interface measurement | curriculum_step | Electrochemical grammar | Instrument warrants depend on cell grammar. | Interface Study |
| rel-core-uses-methods | uses_method | concept | Core objects | method | Methods and warrants | The core objects are made comparable through measurement warrants. | Open Review; Interface Study |

## Visual Summary

Read every result as material chemistry to measured transport to interface evolution to mechanical contact to cell conditions.

## Visual Views

| View ID | View kind | Title | Purpose | Relation IDs | Node emphasis |
|---|---|---|---|---|---|
| view-ssb-dependency-path | dependency_path | Solid-state battery dependency path | Show the learning order that keeps measurement claims grounded. | rel-interface-measurement-after-grammar; rel-core-uses-methods | Methods and warrants |
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

    assert_eq!(exported.report.literature_ladder.len(), 1);
    assert_eq!(exported.report.literature_ladder[0].layer, "Foundation");
    assert_eq!(exported.report.literature_ladder[0].source_ids.len(), 1);

    assert_eq!(exported.report.relations.len(), 2);
    let relation = exported
        .report
        .relations
        .iter()
        .find(|relation| relation.id == "rel-interface-measurement-after-grammar")
        .unwrap();
    assert_eq!(relation.kind, report::RelationKind::DependsOn);
    assert_eq!(
        relation.from.entity_type,
        report::EntityType::CurriculumStep
    );
    assert_eq!(relation.to.entity_type, report::EntityType::CurriculumStep);
    assert_eq!(relation.source_ids.len(), 1);

    let interface_step = exported
        .report
        .curriculum_path
        .iter()
        .find(|step| step.title == "Interface measurement")
        .unwrap();
    assert!(interface_step
        .prerequisite_ids
        .iter()
        .any(|id| id.starts_with("step-electrochemical-grammar-")));
    assert!(interface_step
        .prerequisite_ids
        .iter()
        .any(|id| id.starts_with("method-methods-and-warrants-")));

    assert_eq!(exported.report.visual_views.len(), 1);
    let view = &exported.report.visual_views[0];
    assert_eq!(view.id, "view-ssb-dependency-path");
    assert_eq!(view.kind, report::VisualViewKind::DependencyPath);
    assert_eq!(view.edges.len(), 2);
    assert!(view
        .edges
        .iter()
        .any(|edge| edge.relation_id == "rel-core-uses-methods"));
    assert_contains(
        &view.justification,
        "Read every result as material chemistry",
    );
    assert!(view.nodes.iter().any(|node| {
        node.entity_type == report::EntityType::Method && node.description.contains("Emphasized")
    }));

    let diagnostics = exported
        .diagnostics
        .as_ref()
        .map(|diagnostics| &diagnostics.checks)
        .cloned()
        .unwrap_or_default();
    assert!(
        !diagnostics.iter().any(|check| {
            check.check_id == report::CHECK_EXPORT_UNSUPPORTED_SECTION
                && (check.message.contains("Literature Ladder")
                    || check.message.contains("visual")
                    || check.message.contains("Visual"))
        }),
        "supported ladder and visual sections should not produce unsupported-section diagnostics: {diagnostics:?}"
    );

    let validation = report::validate_report_value(&serde_json::to_value(&exported).unwrap());
    assert_eq!(validation.error_count(), 0, "{:?}", validation.diagnostics);
}

#[test]
fn field_element_class_drives_typed_export_without_optional_modules() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("field-elements.md");
    let sources_path = dir.path().join("sources.csv");
    fs::write(
        &sources_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
Instrument Review,review_article,doi:10.0000/instrument,https://example.test/instrument,2025-01-01,open_access,Official URL,$0,CC BY,foundation,Explains measurement and calibration.,Reference,\n",
    )
    .unwrap();
    fs::write(
        &report_path,
        r#"# Structure of Knowledge: Measurement Science

## Report Architecture

| Item | Decision |
|---|---|
| Executive thesis | Measurement science becomes legible by following the transformations that turn a remote signal into a warranted estimate. |
| Chosen organizing form | A phenomenon-to-instrument-to-estimate chain. |
| Architecture rationale | This sequence exposes how instruments and calibration shape the reported observable. |
| Rejected alternatives and why | A taxonomy of instrument types was rejected because it would hide the transformations and warrants shared across devices. |

## When a Signal Becomes Data

The field is organized by the transformations that turn a remote signal into a warranted estimate.

## Field Element Inventory

| Element class | Observed element | Actual form in this field | Role | Load-bearing relations | Source IDs | Confidence |
|---|---|---|---|---|---|---|
| Object or phenomenon | Remote signal | A physical quantity available only through an instrument response. | core | represented by an observable | S1 | high |
| Representation or model | Observable quantities | Calibrated values with units and uncertainty. | core | represents the signal and uses calibration | S1 | high |
| Method or operation | Calibration transfer | A comparison that estimates nuisance transformations. | core | qualifies observable quantities | S1 | high |
| Instrument chain | Receiver chain | An antenna, amplifier, digitizer, and calibration path acting as one measurement system. | surrounding | transforms the remote signal into recorded samples | S1 | medium |

## Relations

| Relation ID | Relation kind | From type | From reference | To type | To reference | Rationale | Source IDs |
|---|---|---|---|---|---|---|---|
| R1 | represented_by | concept | Remote signal | representation | Observable quantities | Instruments expose signals through observables. | S1 |
| R2 | uses_method | representation | Observable quantities | method | Calibration transfer | Reported values depend on calibration. | S1 |
| R3 | maps_to | field_element | Remote signal | field_element | Receiver chain | The phenomenon becomes data only through the instrument chain. | S1 |
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
    assert_eq!(exported.report.field_elements.len(), 4);
    let receiver_chain = exported
        .report
        .field_elements
        .iter()
        .find(|element| element.label == "Receiver chain")
        .unwrap();
    assert_eq!(receiver_chain.element_class, "Instrument chain");
    assert_eq!(receiver_chain.role, "surrounding");
    assert_eq!(
        receiver_chain.confidence,
        Some(report::ClaimConfidence::Medium)
    );
    assert_contains(&receiver_chain.actual_form, "antenna");
    assert_contains(
        &receiver_chain.load_bearing_relations,
        "transforms the remote signal",
    );
    assert_eq!(exported.report.core_ideas.len(), 2);
    assert_eq!(exported.report.representations.len(), 1);
    assert_eq!(exported.report.methods.len(), 1);
    assert_eq!(exported.report.relations.len(), 3);
    let field_relation = exported
        .report
        .relations
        .iter()
        .find(|relation| relation.id == report::content_id("rel", &["R3"]))
        .unwrap();
    assert_eq!(
        field_relation.from.entity_type,
        report::EntityType::FieldElement
    );
    assert_eq!(
        field_relation.to.entity_type,
        report::EntityType::FieldElement
    );
    assert!(exported.report.curriculum_path.is_empty());
    assert!(exported.diagnostics.is_none(), "{:?}", exported.diagnostics);
    let validation = report::validate_report_value(&serde_json::to_value(&exported).unwrap());
    assert_eq!(validation.error_count(), 0, "{:?}", validation.diagnostics);
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
    let korean = report::content_id_identity("element", &["양자 센싱"]);
    let korean_nfd = report::content_id("element", &["양자 센싱"]);
    assert_eq!(korean.stable_id, korean_nfd);
    assert_eq!(korean.display_slug, "item");
    assert_eq!(korean.normalized_identity, "양자 센싱");
    validate_stable_id(&korean.stable_id).unwrap();

    let composed = report::content_id("concept", &["가"]);
    let decomposed = report::content_id("concept", &["가"]);
    assert_eq!(composed, decomposed);

    let greek_upper = report::content_id("concept", &["Μέθοδος"]);
    let greek_lower = report::content_id("concept", &["μέθοδος"]);
    assert_eq!(greek_upper, greek_lower);

    let korean_concept = report::content_id("concept", &["양자 센싱"]);
    let cjk = report::content_id("concept", &["量子センシング"]);
    let greek = report::content_id("concept", &["μέθοδος"]);
    let mixed = report::content_id("concept", &["Graph 그래프"]);
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
    assert!(evidence["$defs"]["claim"]["properties"]
        .as_object()
        .unwrap()
        .contains_key("evidence_links"));
    assert!(pedagogy["$defs"]["learning_step"]["properties"]
        .as_object()
        .unwrap()
        .contains_key("prerequisite_ids"));

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
            "rationale": "A short text-only report may intentionally omit visual views when no relation-backed view improves reading."
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

#[test]
fn validate_report_fixture_passes_without_diagnostics() {
    let final_report = current_human_report_value();
    let validation = report::validate_report_value(&final_report);
    assert_eq!(validation.error_count(), 0);
    assert_eq!(validation.warning_count(), 0);
    assert!(validation.diagnostics.checks.is_empty());

    let dir = tempfile::tempdir().unwrap();
    let input_path = dir.path().join("current-v2-report.json");
    report::write_json_file(&input_path, &final_report).unwrap();
    let exit_code = run_cli(vec![
        "validate-report".to_string(),
        "--input".to_string(),
        input_path.display().to_string(),
        "--strict".to_string(),
    ])
    .unwrap();
    assert_eq!(exit_code, 0);
}

#[test]
fn migrate_ids_writes_ordered_maps_for_v1_and_v2_without_mutating_input() {
    let dir = tempfile::tempdir().unwrap();
    let v2_input = dir.path().join("unicode-v2-report.json");
    let v2_output = dir.path().join("unicode-v2-id-map.json");
    let mut v2 = current_human_report_value();
    v2["report"]["field_elements"][0]["id"] = json!("element-item-4a33eacd5f");
    v2["report"]["field_elements"][0]["label"] = json!("양자 센싱");
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
    assert_eq!(unicode_entry.normalized_identity, "양자 센싱");
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

#[test]
fn validate_report_schema_and_public_boundary_errors_are_stable() {
    let final_report = current_human_report_value();

    let mut missing = final_report.clone();
    missing["report"].as_object_mut().unwrap().remove("field");
    let validation = report::validate_report_value(&missing);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SCHEMA_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut leaked = final_report.clone();
    leaked["report"]["raw_learner_profile"] = json!("SENTINEL_INTERNAL learner");
    leaked["report"]["core_ideas"][0]["description"] =
        json!("validation diagnostic check_id should never be public");
    let validation = report::validate_report_value(&leaked);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_PUBLIC_BOUNDARY,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_evidence_requirements_use_structured_fields() {
    let mut report_value = current_human_report_value();
    let ligo_claim_index =
        report_item_index_containing(&report_value, "claims", "statement", "LIGO");
    let ligo_source_id = report_item_id(
        &report_value,
        "sources",
        "title",
        "Broadband Quantum Enhancement of the LIGO Detectors",
    );
    report_value["report"]["claims"][ligo_claim_index]["evidence_links"] = json!([
        {
            "source_id": ligo_source_id,
            "verification_status": "cataloged",
            "support_kind": "supports",
            "locator": "detector configuration",
            "support_note": "Cataloged links cannot satisfy a reviewed-source requirement."
        }
    ]);
    let validation = report::validate_report_value(&report_value);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut unsupported = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &unsupported,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    let supporting_link_index = array_item_index(
        &unsupported["report"]["claims"][chain_claim_index]["evidence_links"],
        "support_kind",
        "supports",
    );
    unsupported["report"]["claims"][chain_claim_index]["evidence_links"][supporting_link_index]
        ["locator"] = json!("");
    unsupported["report"]["claims"][chain_claim_index]["evidence_links"][supporting_link_index]
        ["support_note"] = json!("");
    let validation = report::validate_report_value(&unsupported);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SUPPORT,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_source_role_coverage_uses_explicit_requirements_and_waivers() {
    let mut required_gap = current_human_report_value();
    required_gap["report"]["evidence_standards"]["source_role_requirements"] = json!([
        {
            "role": "foundation",
            "requirement": "required",
            "minimum_sources": 2,
            "rationale": "Two independent foundation sources are required for this validation probe."
        }
    ]);
    let validation = report::validate_report_value(&required_gap);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SOURCE_ROLE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut missing_waiver = current_human_report_value();
    missing_waiver["report"]["evidence_standards"]["source_role_requirements"] = json!([
        {
            "role": "debate",
            "requirement": "waived",
            "minimum_sources": 0,
            "rationale": "No debate source is needed for the stable measurement-chain fixture."
        }
    ]);
    let validation = report::validate_report_value(&missing_waiver);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SOURCE_ROLE_WAIVER,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_currentness_is_structured_and_prose_is_warning_only() {
    let mut missing_temporal = current_human_report_value();
    let current_claim_index =
        report_item_index_containing(&missing_temporal, "claims", "statement", "Field usefulness");
    missing_temporal["report"]["claims"][current_claim_index]["temporal"]
        .as_object_mut()
        .unwrap()
        .remove("review_after");
    let validation = report::validate_report_value(&missing_temporal);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_CURRENTNESS_METADATA,
        report::DiagnosticSeverity::Error,
    );

    let mut future_source = current_human_report_value();
    let roadmap_source_index = report_item_index(
        &future_source,
        "sources",
        "title",
        "DOE Quantum Information Science Applications Roadmap",
    );
    future_source["report"]["sources"][roadmap_source_index]["date"] = json!("2028");
    let validation = report::validate_report_value(&future_source);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_CURRENTNESS_SOURCE_DATE,
        report::DiagnosticSeverity::Error,
    );

    let mut prose = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &prose,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    prose["report"]["claims"][chain_claim_index]["statement"] =
        json!("The latest quantum-sensing account still begins with the measurement chain.");
    prose["report"]["claims"][chain_claim_index]["temporal"]["as_of"] = json!("2026-07-27");
    prose["report"]["claims"][chain_claim_index]["temporal"]["temporal_status"] = json!("unknown");
    let validation = report::validate_report_value(&prose);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_CURRENTNESS_PROSE,
        report::DiagnosticSeverity::Warning,
    );
    assert_eq!(
        validation.error_count(),
        0,
        "unstructured currentness prose should warn without introducing a validation error"
    );
}

#[test]
fn validate_report_relation_and_curriculum_consistency_are_errors() {
    let mut bad_kind = current_human_report_value();
    bad_kind["report"]["relations"][0]["kind"] = json!("unknown_relation");
    let validation = report::validate_report_value(&bad_kind);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_RELATION_KIND,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_endpoint = current_human_report_value();
    bad_endpoint["report"]["relations"][0]["to"]["id"] = json!("concept-missing");
    let validation = report::validate_report_value(&bad_endpoint);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_RELATION_ENDPOINT,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_step = current_human_report_value();
    bad_step["report"]["curriculum_path"][1]["prerequisite_ids"] = json!(["step-missing"]);
    let validation = report::validate_report_value(&bad_step);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_CURRICULUM_REFERENCE,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_source_access_metadata_is_strict_for_restricted_sources() {
    let mut value = current_human_report_value();
    value["report"]["sources"][1]["access"] = json!({
        "status": "paywalled",
        "route": "unknown",
        "budget_estimate": "",
        "license": "",
        "metadata_only": false,
        "notes": ""
    });
    let validation = report::validate_report_value(&value);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SOURCE_ACCESS,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_strict_mode_fails_on_warnings_but_default_does_not() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("warning-only-report.json");
    let mut value = current_human_report_value();
    value["report"]["evidence_standards"]["source_role_requirements"] = json!([
        {
            "role": "critique",
            "requirement": "conditional",
            "minimum_sources": 1,
            "rationale": "A critique source is conditionally useful for this validation probe."
        }
    ]);
    report::write_json_file(&report_path, &value).unwrap();

    let validation = report::validate_report_file(&report_path).unwrap();
    assert_eq!(validation.error_count(), 0);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SOURCE_ROLE_CONDITIONAL,
        report::DiagnosticSeverity::Warning,
    );

    let default_exit = run_cli(vec![
        "validate-report".to_string(),
        "--input".to_string(),
        report_path.display().to_string(),
    ])
    .unwrap();
    assert_eq!(default_exit, 0);

    let strict_exit = run_cli(vec![
        "validate-report".to_string(),
        "--input".to_string(),
        report_path.display().to_string(),
        "--strict".to_string(),
    ])
    .unwrap();
    assert_eq!(strict_exit, 1);
}

#[test]
fn validate_report_surfaces_embedded_export_diagnostics_in_strict_mode() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("embedded-warning-report.json");
    let mut value = current_human_report_value();
    value["diagnostics"] = json!({
        "summary": "export diagnostics",
        "checks": [
            {
                "check_id": report::CHECK_EXPORT_UNRESOLVED_REFERENCE,
                "severity": "warning",
                "message": "Relation endpoint could not be resolved during export.",
                "target_path": "/report/relations/0",
                "entity_id": "rel-quotient-depends-invariance"
            }
        ]
    });
    report::write_json_file(&report_path, &value).unwrap();

    let validation = report::validate_report_file(&report_path).unwrap();
    assert_eq!(validation.error_count(), 0);
    assert_validation_check(
        &validation,
        report::CHECK_EXPORT_UNRESOLVED_REFERENCE,
        report::DiagnosticSeverity::Warning,
    );

    assert_eq!(
        run_cli(vec![
            "validate-report".to_string(),
            "--input".to_string(),
            report_path.display().to_string(),
        ])
        .unwrap(),
        0
    );
    assert_eq!(
        run_cli(vec![
            "validate-report".to_string(),
            "--input".to_string(),
            report_path.display().to_string(),
            "--strict".to_string(),
        ])
        .unwrap(),
        1
    );

    let waived_path = dir.path().join("accepted-loss-report.json");
    value["diagnostics"]["checks"][0]["status"] = json!("not_applicable");
    value["diagnostics"]["checks"][0]["message"] =
        json!("Accepted loss waiver: unresolved export reference is intentional in this fixture.");
    report::write_json_file(&waived_path, &value).unwrap();
    let waived = report::validate_report_file(&waived_path).unwrap();
    assert_eq!(waived.warning_count(), 0, "{:?}", waived.diagnostics);
    assert_validation_check(
        &waived,
        report::CHECK_EXPORT_UNRESOLVED_REFERENCE,
        report::DiagnosticSeverity::Info,
    );
    assert_eq!(
        run_cli(vec![
            "validate-report".to_string(),
            "--input".to_string(),
            waived_path.display().to_string(),
            "--strict".to_string(),
        ])
        .unwrap(),
        0
    );
}

#[test]
fn validate_report_requires_structure_for_substantial_final_reports() {
    let mut missing_relations = current_human_report_value();
    missing_relations["report"]["relations"] = json!([]);
    let validation = report::validate_report_value(&missing_relations);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_STRUCTURE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut missing_prerequisites = current_human_report_value();
    for step in missing_prerequisites["report"]["curriculum_path"]
        .as_array_mut()
        .unwrap()
    {
        step["prerequisite_ids"] = json!([]);
    }
    let validation = report::validate_report_value(&missing_prerequisites);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_STRUCTURE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut waived = missing_prerequisites.clone();
    waived["report"]["structure_waivers"] = json!([
        {
            "scope": "curriculum_prerequisites",
            "rationale": "This fixture intentionally omits prerequisites to test waiver behavior."
        }
    ]);
    let validation = report::validate_report_value(&waived);
    assert_no_validation_check(
        &validation,
        report::CHECK_VALIDATE_STRUCTURE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut missing_visual = current_human_report_value();
    missing_visual["report"]
        .as_object_mut()
        .unwrap()
        .remove("visual_views");
    missing_visual["diagnostics"] = json!({
        "checks": [
            {
                "check_id": report::CHECK_EXPORT_UNSUPPORTED_SECTION,
                "severity": "warning",
                "message": "Visual Summary was present in source Markdown but no structured Visual Views table preserved it."
            }
        ]
    });
    let validation = report::validate_report_value(&missing_visual);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_STRUCTURE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    missing_visual["report"]["structure_waivers"] = json!([
        {
            "scope": "visual_views",
            "rationale": "A text-only report intentionally omits the visual summary."
        }
    ]);
    let validation = report::validate_report_value(&missing_visual);
    assert_no_validation_check(
        &validation,
        report::CHECK_VALIDATE_STRUCTURE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );
    assert_validation_check(
        &validation,
        report::CHECK_EXPORT_UNSUPPORTED_SECTION,
        report::DiagnosticSeverity::Info,
    );
}

#[test]
fn validate_report_checks_new_structure_endpoints() {
    let mut bad_ladder_source = current_human_report_value();
    let boundary_row_index =
        report_item_index(&bad_ladder_source, "literature_ladder", "layer", "Boundary");
    bad_ladder_source["report"]["literature_ladder"][boundary_row_index]["source_ids"] =
        json!(["src-missing-source"]);
    let validation = report::validate_report_value(&bad_ladder_source);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SOURCE,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_claim_source = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &bad_claim_source,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    let supporting_link_index = array_item_index(
        &bad_claim_source["report"]["claims"][chain_claim_index]["evidence_links"],
        "support_kind",
        "supports",
    );
    bad_claim_source["report"]["claims"][chain_claim_index]["evidence_links"]
        [supporting_link_index]["source_id"] = json!("src-missing-source");
    let validation = report::validate_report_value(&bad_claim_source);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SOURCE,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_frontier_claim = current_human_report_value();
    let foundation_source_id =
        report_item_id(&bad_frontier_claim, "sources", "title", "Quantum Sensing");
    let measurand_id = report_item_id(
        &bad_frontier_claim,
        "field_elements",
        "label",
        "Physical quantity",
    );
    bad_frontier_claim["report"]["frontier_debates"] = json!([
        {
            "id": "frontier-validation-probe",
            "kind": "frontier",
            "title": "Validation probe",
            "summary": "A bounded fixture item for checking public endpoint resolution.",
            "why_it_matters": "A frontier item must resolve every structured claim reference.",
            "required_background_ids": [measurand_id],
            "claim_ids": ["claim-missing"],
            "source_ids": [foundation_source_id],
            "temporal": {
                "as_of": "2026-07-27",
                "review_after": "2027-01-26",
                "temporal_status": "current"
            }
        }
    ]);
    let validation = report::validate_report_value(&bad_frontier_claim);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_RELATION_ENDPOINT,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_visual_ref = current_human_report_value();
    let view_index = report_item_index(
        &bad_visual_ref,
        "visual_views",
        "title",
        "From fragile probe to trustworthy measurement",
    );
    let node_index = array_item_index(
        &bad_visual_ref["report"]["visual_views"][view_index]["nodes"],
        "label",
        "Physical quantity",
    );
    bad_visual_ref["report"]["visual_views"][view_index]["nodes"][node_index]["ref_id"] =
        json!("step-missing");
    let validation = report::validate_report_value(&bad_visual_ref);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_visual_node = current_human_report_value();
    let measurand_id = report_item_id(
        &bad_visual_node,
        "field_elements",
        "label",
        "Physical quantity",
    );
    let encoding_id = report_item_id(
        &bad_visual_node,
        "field_elements",
        "label",
        "Encoding interaction",
    );
    let relation_index =
        report_relation_index_between(&bad_visual_node, &measurand_id, &encoding_id);
    let relation_id = bad_visual_node["report"]["relations"][relation_index]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let view_index = report_item_index(
        &bad_visual_node,
        "visual_views",
        "title",
        "From fragile probe to trustworthy measurement",
    );
    let edge_index = array_item_index(
        &bad_visual_node["report"]["visual_views"][view_index]["edges"],
        "relation_id",
        &relation_id,
    );
    bad_visual_node["report"]["visual_views"][view_index]["edges"][edge_index]["from"] =
        json!("vnode-missing");
    let validation = report::validate_report_value(&bad_visual_node);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_visual_relation = current_human_report_value();
    let measurand_id = report_item_id(
        &bad_visual_relation,
        "field_elements",
        "label",
        "Physical quantity",
    );
    let encoding_id = report_item_id(
        &bad_visual_relation,
        "field_elements",
        "label",
        "Encoding interaction",
    );
    let relation_index =
        report_relation_index_between(&bad_visual_relation, &measurand_id, &encoding_id);
    let relation_id = bad_visual_relation["report"]["relations"][relation_index]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let view_index = report_item_index(
        &bad_visual_relation,
        "visual_views",
        "title",
        "From fragile probe to trustworthy measurement",
    );
    let edge_index = array_item_index(
        &bad_visual_relation["report"]["visual_views"][view_index]["edges"],
        "relation_id",
        &relation_id,
    );
    bad_visual_relation["report"]["visual_views"][view_index]["edges"][edge_index]["relation_id"] =
        json!("rel-missing");
    let validation = report::validate_report_value(&bad_visual_relation);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );

    let mut missing_elements = current_human_report_value();
    missing_elements["report"]["core_ideas"] = json!([]);
    missing_elements["report"]["methods"] = json!([]);
    missing_elements["report"]["representations"] = json!([]);
    missing_elements["report"]
        .as_object_mut()
        .unwrap()
        .remove("field_elements");
    let validation = report::validate_report_value(&missing_elements);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_STRUCTURE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_visual_views_require_stable_unambiguous_renderable_declarations() {
    let fixture = current_human_report_value();
    let validation = report::validate_report_value(&fixture);
    assert_no_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );

    let mut unstable_view_id = fixture.clone();
    unstable_view_id["report"]["visual_views"][0]["id"] = json!("BAD VIEW ID");
    assert_validation_message(
        &report::validate_report_value(&unstable_view_id),
        "visual view id is not stable",
    );

    let mut duplicate_view_id = fixture.clone();
    let duplicate_view = duplicate_view_id["report"]["visual_views"][0].clone();
    duplicate_view_id["report"]["visual_views"]
        .as_array_mut()
        .unwrap()
        .push(duplicate_view);
    assert_validation_message(
        &report::validate_report_value(&duplicate_view_id),
        "duplicate visual view id",
    );

    let mut unsupported_kind = fixture.clone();
    unsupported_kind["report"]["visual_views"][0]["kind"] = json!("custom");
    assert_validation_message(
        &report::validate_report_value(&unsupported_kind),
        "unsupported kind custom",
    );

    let mut blank_justification = fixture.clone();
    blank_justification["report"]["visual_views"][0]["justification"] = json!(" \n ");
    assert_validation_message(
        &report::validate_report_value(&blank_justification),
        "requires a non-empty justification",
    );

    let mut too_few_nodes = fixture.clone();
    too_few_nodes["report"]["visual_views"][0]["nodes"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    too_few_nodes["report"]["visual_views"][0]["edges"] = json!([]);
    let validation = report::validate_report_value(&too_few_nodes);
    assert_validation_message(&validation, "requires at least two nodes");
    assert_validation_message(
        &validation,
        "requires at least one valid relation-backed edge",
    );

    let mut unstable_node_id = fixture.clone();
    unstable_node_id["report"]["visual_views"][0]["nodes"][0]["id"] = json!("BAD NODE ID");
    assert_validation_message(
        &report::validate_report_value(&unstable_node_id),
        "node id is not stable",
    );

    let mut duplicate_node_id = fixture.clone();
    let first_node_id = duplicate_node_id["report"]["visual_views"][0]["nodes"][0]["id"].clone();
    duplicate_node_id["report"]["visual_views"][0]["nodes"][1]["id"] = first_node_id;
    assert_validation_message(
        &report::validate_report_value(&duplicate_node_id),
        "duplicate node id",
    );

    let mut missing_node_ref = fixture.clone();
    missing_node_ref["report"]["visual_views"][0]["nodes"][0]
        .as_object_mut()
        .unwrap()
        .remove("ref_id");
    assert_validation_message(
        &report::validate_report_value(&missing_node_ref),
        "requires an entity ref_id",
    );

    let mut self_edge = fixture;
    let from = self_edge["report"]["visual_views"][0]["edges"][0]["from"].clone();
    self_edge["report"]["visual_views"][0]["edges"][0]["to"] = from;
    assert_validation_message(&report::validate_report_value(&self_edge), "is a self-edge");
}

#[test]
fn validate_report_evidence_support_semantics_are_strict() {
    let mut qualifies_only = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &qualifies_only,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    let foundation_source_id =
        report_item_id(&qualifies_only, "sources", "title", "Quantum Sensing");
    qualifies_only["report"]["claims"][chain_claim_index]["evidence_links"] = json!([
        {
            "source_id": foundation_source_id,
            "verification_status": "reviewed",
            "support_kind": "qualifies",
            "locator": "measurement protocols and noise",
            "support_note": "This narrows but does not affirm the claim.",
            "reviewed_at": "2026-07-27"
        }
    ]);
    let validation = report::validate_report_value(&qualifies_only);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SUPPORT,
        report::DiagnosticSeverity::Warning,
    );

    let mut contradicts = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &contradicts,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    let metrology_source_id = report_item_id(
        &contradicts,
        "sources",
        "title",
        "Advances in Quantum Metrology",
    );
    contradicts["report"]["claims"][chain_claim_index]["evidence_links"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "source_id": metrology_source_id,
            "verification_status": "reviewed",
            "support_kind": "contradicts",
            "locator": "precision bounds and resource accounting",
            "support_note": "This conflicts with an overbroad form of the claim.",
            "reviewed_at": "2026-07-27"
        }));
    let validation = report::validate_report_value(&contradicts);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SUPPORT,
        report::DiagnosticSeverity::Warning,
    );
    assert_eq!(validation.error_count(), 0, "{:?}", validation.diagnostics);

    let mut missing_reviewed_at = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &missing_reviewed_at,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    let supporting_link_index = array_item_index(
        &missing_reviewed_at["report"]["claims"][chain_claim_index]["evidence_links"],
        "support_kind",
        "supports",
    );
    missing_reviewed_at["report"]["claims"][chain_claim_index]["evidence_links"]
        [supporting_link_index]
        .as_object_mut()
        .unwrap()
        .remove("reviewed_at");
    let validation = report::validate_report_value(&missing_reviewed_at);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SUPPORT,
        report::DiagnosticSeverity::Error,
    );

    let mut cataloged_visible = current_human_report_value();
    let chain_claim_index = report_item_index_containing(
        &cataloged_visible,
        "claims",
        "statement",
        "end-to-end measurement chain",
    );
    let foundation_source_id =
        report_item_id(&cataloged_visible, "sources", "title", "Quantum Sensing");
    cataloged_visible["report"]["claims"][chain_claim_index]["evidence_links"] = json!([
        {
            "source_id": foundation_source_id,
            "verification_status": "cataloged",
            "support_kind": "supports"
        }
    ]);
    let validation = report::validate_report_value(&cataloged_visible);
    assert_validation_check(
        &validation,
        report::CHECK_EVIDENCE_CATALOGED_ONLY,
        report::DiagnosticSeverity::Warning,
    );
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_output_order_is_deterministic() {
    let mut value = current_human_report_value();
    value["report"]["sources"][1]["access"]["route"] = json!("unknown");
    value["report"]["claims"][2]["evidence_links"] = json!([]);
    value["report"]["relations"][0]["to"]["id"] = json!("concept-missing");

    let validation = report::validate_report_value(&value);
    let actual = validation_diagnostic_order(&validation);
    let mut sorted = actual.clone();
    sorted.sort();
    assert_eq!(actual, sorted);

    let repeated = validation_diagnostic_order(&report::validate_report_value(&value));
    assert_eq!(actual, repeated);
}

#[test]
fn render_html_renders_legacy_v1_report_as_self_contained_one_view() {
    let dir = tempfile::tempdir().unwrap();
    let input_path = dir.path().join("legacy-v1-report.json");
    let output_path = dir.path().join("sok-report.html");
    report::write_json_file(&input_path, &legacy_v1_report_value()).unwrap();
    let exit = run_cli(vec![
        "render-html".to_string(),
        "--input".to_string(),
        input_path.display().to_string(),
        "--output".to_string(),
        output_path.display().to_string(),
    ])
    .unwrap();
    assert_eq!(exit, 0);

    let html = fs::read_to_string(&output_path).unwrap();
    assert_contains(&html, "<!doctype html>");
    assert_contains(&html, "<style>");
    assert_contains(
        &html,
        "<script type=\"application/json\" id=\"sok-visual-data\">",
    );
    assert_contains(&html, "Structure of Knowledge Report");
    assert_contains(&html, "Reading Guide");
    assert_contains(&html, "Knowledge Map");
    assert_contains(&html, "Reading Ladder");
    assert_contains(&html, "Domain Profile");
    assert_contains(&html, "Field Elements");
    assert_not_contains(&html, "id=\"core-ideas\"");
    assert_not_contains(&html, "id=\"methods\"");
    assert_not_contains(&html, "id=\"representations\"");
    assert_contains(&html, "Evidence Standards");
    assert_contains(&html, "Source Catalog");
    assert_contains(&html, "Claim Evidence Guide");
    assert_contains(&html, "Curriculum Path");
    assert_contains(&html, "Frontier Guidance");
    assert_contains(&html, "Relation Audit");
    assert_contains(&html, "id=\"visualizations\"");
    assert_contains(&html, "From fragile probe to trustworthy measurement");
    assert_contains(&html, "\"kind\":\"knowledge_spine\"");
    assert_contains(&html, "\"relation_id\":\"rel-measurand-to-encoding\"");
    assert_contains(
        &html,
        "\"relation_id\":\"rel-estimation-grounds-performance\"",
    );
    assert_contains(&html, "class=\"claim-card\"");
    assert_contains(&html, "<summary>Raw claim identifiers</summary>");
    assert_contains(&html, "class=\"visual-fallback\"");
    assert_contains(&html, "Physical quantity to Encoding interaction");
    assert_contains(&html, "class=\"text-fallback\"");
    assert_contains(&html, "Text alternative for curriculum path");
    assert_contains(&html, "role=\"region\"");
    assert_contains(&html, "<noscript>");
    assert_contains(&html, "@media (prefers-color-scheme: dark)");
    assert_contains(&html, "@media (max-width: 720px)");
    assert_contains(&html, "@media print");
    assert_contains(&html, ".table-scroll");
    assert_contains(
        &html,
        ".narrative-body {\n  max-width: 880px;\n  min-width: 0;\n  overflow-x: auto;",
    );
    assert_contains(
        &html,
        "details.structured-appendix > summary {\n    display: none;\n  }\n  details.structured-appendix:not([open]) > :not(summary) {\n    display: block !important;",
    );
    assert_before(&html, "id=\"reading-guide\"", "id=\"visualizations\"");
    assert_before(&html, "id=\"visualizations\"", "id=\"curriculum-path\"");
    assert_before(&html, "id=\"curriculum-path\"", "id=\"reading-ladder\"");
    assert_before(&html, "id=\"reading-ladder\"", "id=\"claims\"");
    assert_before(&html, "id=\"claims\"", "id=\"frontier-debates\"");
    assert_before(&html, "id=\"frontier-debates\"", "id=\"scope\"");
    assert_not_contains(&html, "<script src");
    assert_not_contains(&html, "<link");
    assert_not_contains(&html.to_ascii_lowercase(), "mermaid");
    assert_not_contains(&html.to_ascii_lowercase(), "cdn");
}

#[test]
fn render_html_keeps_report_readable_without_visual_shell_when_views_are_absent() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    value["report"]
        .as_object_mut()
        .unwrap()
        .remove("visual_views");
    for section in value["report"]["presentation"]["sections"]
        .as_array_mut()
        .unwrap()
    {
        section.as_object_mut().unwrap().remove("visual_view_ids");
    }
    let html = render_report_value_to_html(dir.path(), "zero-view-report.json", &value);

    assert_contains(&html, "Fragility Becomes Signal");
    assert_contains(&html, "One Measurement Grammar, Many Platforms");
    assert_contains(&html, "Source Catalog");
    assert_contains(&html, "Claim Evidence Guide");
    assert_contains(&html, "Curriculum Path");
    assert_contains(&html, "Frontier Guidance");
    assert_not_contains(&html, "id=\"visualizations\"");
    assert_not_contains(&html, "class=\"visual-card\"");
    assert_not_contains(&html, "data-visual-mount=");
    assert_not_contains(&html, "id=\"sok-visual-data\"");
}

#[test]
fn render_html_instantiates_multiple_declared_supported_views() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    let existing_view = value["report"]["visual_views"][0].clone();
    value["report"]["frontier_debates"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "frontier-field-validation-gap",
            "kind": "uncertainty",
            "title": "Field validation gap",
            "summary": "Laboratory sensitivity does not by itself establish calibrated field utility.",
            "why_it_matters": "Deployment evidence determines whether the measurement chain remains useful outside controlled conditions.",
            "required_background_ids": [
                "element-deployment-environment-eb7bb13b46"
            ],
            "claim_ids": [
                "claim-field-usefulness-requires-performance-to-be-re-evaluated-b493f3b20d"
            ],
            "source_ids": [
                "src-bringing-quantum-sensors-to-fruition-ostp-nstc-2022-6b3219ece3"
            ],
            "temporal": {
                "as_of": "2026-07-27",
                "review_after": "2027-01-26",
                "temporal_status": "current",
                "rationale": "Public translation guidance should be reviewed as deployment evidence changes."
            }
        }));
    value["report"]["relations"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "rel-quantum-sensing-supports-encoding",
            "kind": "supports",
            "from": {
                "entity_type": "source",
                "id": "src-quantum-sensing-doi-10-1103-revmodphys-89-035002-568d4f0855"
            },
            "to": {
                "entity_type": "field_element",
                "id": "element-encoding-interaction-f15ae8c981"
            },
            "description": "The experimental review supports the field's encoding-interaction model."
        }));
    value["report"]["relations"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "rel-deployment-claim-maps-to-gap",
            "kind": "maps_to",
            "from": {
                "entity_type": "claim",
                "id": "claim-field-usefulness-requires-performance-to-be-re-evaluated-b493f3b20d"
            },
            "to": {
                "entity_type": "frontier_debate",
                "id": "frontier-field-validation-gap"
            },
            "description": "The deployment claim identifies the unresolved field-validation gap."
        }));
    value["report"]["relations"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "rel-nstc-supports-field-gap",
            "kind": "supports",
            "from": {
                "entity_type": "source",
                "id": "src-bringing-quantum-sensors-to-fruition-ostp-nstc-2022-6b3219ece3"
            },
            "to": {
                "entity_type": "frontier_debate",
                "id": "frontier-field-validation-gap"
            },
            "description": "The public roadmap documents engineering, validation, and adoption barriers."
        }));
    value["report"]["visual_views"] = json!([
        existing_view,
        {
            "id": "view-concept-source-map",
            "kind": "concept_source",
            "title": "Encoding source map",
            "justification": "A concept-source view is justified because the encoding model is tied to a reviewed experimental source.",
            "nodes": [
                {
                    "id": "vnode-encoding-interaction",
                    "label": "Encoding interaction",
                    "entity_type": "field_element",
                    "ref_id": "element-encoding-interaction-f15ae8c981"
                },
                {
                    "id": "vnode-quantum-sensing-source",
                    "label": "Quantum Sensing",
                    "entity_type": "source",
                    "ref_id": "src-quantum-sensing-doi-10-1103-revmodphys-89-035002-568d4f0855"
                }
            ],
            "edges": [
                {
                    "from": "vnode-quantum-sensing-source",
                    "to": "vnode-encoding-interaction",
                    "kind": "supports",
                    "relation_id": "rel-quantum-sensing-supports-encoding"
                }
            ]
        },
        {
            "id": "view-frontier-debate-map",
            "kind": "frontier_debate",
            "title": "Field validation gap",
            "justification": "A frontier/debate view is justified because field utility depends on a dated public source and deployment claim.",
            "nodes": [
                {
                    "id": "vnode-frontier-field-gap",
                    "label": "Field validation gap",
                    "entity_type": "frontier_debate",
                    "ref_id": "frontier-field-validation-gap"
                },
                {
                    "id": "vnode-claim-field-utility",
                    "label": "Field utility claim",
                    "entity_type": "claim",
                    "ref_id": "claim-field-usefulness-requires-performance-to-be-re-evaluated-b493f3b20d"
                },
                {
                    "id": "vnode-source-nstc",
                    "label": "Bringing Quantum Sensors to Fruition",
                    "entity_type": "source",
                    "ref_id": "src-bringing-quantum-sensors-to-fruition-ostp-nstc-2022-6b3219ece3"
                }
            ],
            "edges": [
                {
                    "from": "vnode-source-nstc",
                    "to": "vnode-frontier-field-gap",
                    "kind": "supports",
                    "relation_id": "rel-nstc-supports-field-gap"
                },
                {
                    "from": "vnode-claim-field-utility",
                    "to": "vnode-frontier-field-gap",
                    "kind": "maps_to",
                    "relation_id": "rel-deployment-claim-maps-to-gap"
                }
            ]
        }
    ]);

    let html = render_report_value_to_html(dir.path(), "multiple-views-report.json", &value);
    assert_eq!(html.matches("class=\"visual-card\"").count(), 3);
    assert_contains(&html, "\"kind\":\"knowledge_spine\"");
    assert_contains(&html, "\"kind\":\"concept_source\"");
    assert_contains(&html, "\"kind\":\"frontier_debate\"");
    assert_contains(&html, "Encoding source map");
    assert_contains(&html, "Field validation gap");
    assert_contains(&html, "Text alternative for Encoding source map");
    assert_contains(&html, "Nodes");
    assert_contains(&html, "Edges");
}

#[test]
fn render_html_orders_directed_spines_from_relations_not_node_declaration_order() {
    fn visual_layouts(html: &str) -> std::collections::BTreeMap<String, (String, u64, u64, u64)> {
        let payload = html
            .split_once("<script type=\"application/json\" id=\"sok-visual-data\">")
            .unwrap()
            .1
            .split_once("</script>")
            .unwrap()
            .0;
        let views: Value = serde_json::from_str(payload).unwrap();
        views[0]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| {
                (
                    node["label"].as_str().unwrap().to_string(),
                    (
                        node["layout"]["lane"].as_str().unwrap().to_string(),
                        node["layout"]["order"].as_u64().unwrap(),
                        node["layout"]["x"].as_u64().unwrap(),
                        node["layout"]["y"].as_u64().unwrap(),
                    ),
                )
            })
            .collect()
    }

    let dir = tempfile::tempdir().unwrap();
    let ordered = current_human_report_value();
    let ordered_html =
        render_report_value_to_html(dir.path(), "directed-layout-ordered.json", &ordered);
    let ordered_layouts = visual_layouts(&ordered_html);
    assert_contains(
        &ordered_html,
        "\"label_lines\":[\"Reported\",\"performance vector\"]",
    );
    assert_contains(
        &ordered_html,
        "\"label_lines\":[\"Estimation and\",\"uncertainty\"]",
    );
    assert_contains(
        &ordered_html,
        "\"label_lines\":[\"Calibration and\",\"traceability\"]",
    );
    assert_contains(&ordered_html, "var tspan = el(\"tspan\"");

    let mut reversed = ordered.clone();
    reversed["report"]["visual_views"][0]["nodes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let reversed_html =
        render_report_value_to_html(dir.path(), "directed-layout-reversed.json", &reversed);
    let reversed_layouts = visual_layouts(&reversed_html);
    assert_eq!(ordered_layouts, reversed_layouts);

    let chain = [
        "Physical quantity",
        "Encoding interaction",
        "Readout and transduction",
        "Estimation and uncertainty",
        "Reported performance vector",
    ];
    for (expected_order, label) in chain.iter().enumerate() {
        let (lane, order, _, _) = &ordered_layouts[*label];
        assert_eq!(lane, "primary");
        assert_eq!(*order, expected_order as u64);
    }
    for pair in chain.windows(2) {
        assert!(
            ordered_layouts[pair[0]].2 < ordered_layouts[pair[1]].2,
            "{} should render before {}",
            pair[0],
            pair[1]
        );
    }

    let primary_y = ordered_layouts["Reported performance vector"].3;
    for label in ["Calibration and traceability", "Deployment environment"] {
        let (lane, _, _, y) = &ordered_layouts[label];
        assert_eq!(lane, "secondary");
        assert!(*y > primary_y);
    }
}

#[test]
fn render_html_renders_literature_ladder_as_reader_path() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    value["report"]["literature_ladder"] = json!([
        {
            "id": "ladder-measurement-chain-quantum-sensing",
            "layer": "Experimental grammar",
            "start_here": "Start with the full measurement chain before comparing probe platforms.",
            "read_for": "Read for encoding, control, readout, estimation, and dominant noise.",
            "do_not_infer": "Do not infer that laboratory sensitivity alone establishes field utility.",
            "source_ids": ["src-quantum-sensing-doi-10-1103-revmodphys-89-035002-568d4f0855"],
            "notes": "Use this before comparing nonclassical resources."
        }
    ]);

    let html = render_report_value_to_html(dir.path(), "ladder-report.json", &value);
    assert_contains(&html, "Start point");
    assert_contains(&html, "Read for");
    assert_contains(&html, "Do not infer");
    assert_contains(
        &html,
        "Start with the full measurement chain before comparing probe platforms.",
    );
    assert_contains(&html, "Quantum Sensing");
    assert_contains(&html, "<summary>Raw ladder identifiers</summary>");
    assert_before(
        &html,
        "Start with the full measurement chain",
        "<summary>Raw ladder identifiers</summary>",
    );
}

#[test]
fn render_html_claim_cards_group_support_qualifiers_and_contradictions() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    value["report"]["claims"][0]["evidence_links"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "source_id": "src-jcgm-100-2008-guide-to-the-expression-of-8b71053703",
            "verification_status": "reviewed",
            "support_kind": "qualifies",
            "locator": "Clauses 4-8",
            "support_note": "Qualifies the chain interpretation with explicit uncertainty and reporting requirements.",
            "reviewed_at": "2026-07-27"
        }));
    value["report"]["claims"][0]["evidence_links"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "source_id": "src-doe-quantum-information-science-applications-roadmap-doe-qis-5749cee471",
            "verification_status": "reviewed",
            "support_kind": "contradicts",
            "locator": "Quantum sensing applications and cross-cutting needs",
            "support_note": "Challenges any reading that treats one laboratory measurement chain as sufficient for every deployment.",
            "reviewed_at": "2026-07-27"
        }));

    let html = render_report_value_to_html(dir.path(), "claim-card-report.json", &value);
    assert_contains(&html, "class=\"claim-card\"");
    assert_contains(&html, "Supporting Evidence");
    assert_contains(&html, "Qualifying Evidence");
    assert_contains(&html, "Contradictory Evidence");
    assert_contains(
        &html,
        "Qualifies the chain interpretation with explicit uncertainty and reporting requirements.",
    );
    assert_contains(
        &html,
        "Challenges any reading that treats one laboratory measurement chain as sufficient for every deployment.",
    );
    assert_contains(
        &html,
        "JCGM 100:2008 Guide to the Expression of Uncertainty in Measurement",
    );
    assert_contains(
        &html,
        "DOE Quantum Information Science Applications Roadmap",
    );
    assert_contains(&html, "<summary>Raw claim identifiers</summary>");
}

#[test]
fn render_html_rejects_visuals_when_edges_are_not_relation_backed() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    for edge in value["report"]["visual_views"][0]["edges"]
        .as_array_mut()
        .unwrap()
    {
        edge.as_object_mut().unwrap().remove("relation_id");
    }

    let validation = report::validate_report_value(&value);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );
    assert!(validation
        .diagnostics
        .checks
        .iter()
        .any(|check| check.message.contains("requires a relation_id")));
    assert_validation_message(
        &validation,
        "requires at least one valid relation-backed edge",
    );

    let input_path = dir.path().join("relationless-view-report.json");
    let output_path = dir.path().join("relationless-view-report.html");
    report::write_json_file(&input_path, &value).unwrap();
    assert!(report::render_html_report_file(&input_path, &output_path).is_err());
    assert!(!output_path.exists());
}

#[test]
fn render_html_rejects_visuals_when_relation_does_not_match_node_refs() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    value["report"]["visual_views"][0]["edges"][0]["relation_id"] =
        json!("rel-encoding-before-readout");

    let validation = report::validate_report_value(&value);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );
    assert!(validation.diagnostics.checks.iter().any(|check| check
        .message
        .contains("does not match the endpoint entity references")));

    let input_path = dir.path().join("mismatched-view-report.json");
    let output_path = dir.path().join("mismatched-view-report.html");
    report::write_json_file(&input_path, &value).unwrap();
    assert!(report::render_html_report_file(&input_path, &output_path).is_err());
    assert!(!output_path.exists());
}

#[test]
fn render_html_escapes_report_content_and_omits_internal_payloads() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    value["report"]["field"] = json!("Topology <img src=x onerror=alert(1)> \"quoted\"");
    value["report"]["scope"]["summary"] =
        json!("Summary </script><script>alert(1)</script> & <b>bold</b>");
    value["report"]["visual_views"][0]["nodes"][0]["label"] =
        json!("Node </script><script>alert(2)</script>");
    value["report"]["presentation"] = json!({
        "thesis": "Narrative Markdown must remain safe to render.",
        "organizing_form": "Security test",
        "rationale": "Exercise the safe Markdown renderer.",
        "alternatives_considered": "A raw HTML rendering path was rejected because it would bypass sanitization.",
        "sections": [
            {
                "id": "section-security-test",
                "title": "Narrative safety",
                "body_markdown": "Safe prose. <script>alert(3)</script>\n\n[unsafe](javascript:alert(4)) [safe](https://example.org/reference)\n\n![remote](https://assets.example.org/remote.png) ![relative](assets/local.png) ![svg](data:image/svg+xml;base64,PHN2Zz4=) ![embedded](data:image/png;base64,iVBORw0KGgo=)"
            }
        ]
    });
    value["internal_context"] = json!({
        "raw_learner_profile": "SENTINEL_INTERNAL_RENDER",
        "original_goal": "SENTINEL_INTERNAL_GOAL"
    });
    value["diagnostics"] = json!({
        "summary": "SENTINEL_DIAGNOSTIC_RENDER",
        "checks": [
            {
                "check_id": "sentinel.diagnostic",
                "severity": "warning",
                "message": "SENTINEL_DIAGNOSTIC_MESSAGE"
            }
        ]
    });

    let html = render_report_value_to_html(dir.path(), "escaped-report.json", &value);
    assert_contains(&html, "&lt;img src=x onerror=alert(1)&gt;");
    assert_contains(
        &html,
        "Summary &lt;/script&gt;&lt;script&gt;alert(1)&lt;/script&gt; &amp; &lt;b&gt;bold&lt;/b&gt;",
    );
    assert_contains(&html, "\\u003C/script\\u003E\\u003Cscript\\u003Ealert(2)");
    assert_not_contains(&html, "<img src=x");
    assert_not_contains(&html, "</script><script>alert(1)</script>");
    assert_not_contains(&html, "</script><script>alert(2)</script>");
    assert_contains(&html, "&lt;script&gt;alert(3)&lt;/script&gt;");
    assert_not_contains(&html, "<script>alert(3)</script>");
    assert_contains(&html, "href=\"#blocked-unsafe-link\"");
    assert_not_contains(&html, "href=\"javascript:");
    assert_contains(&html, "href=\"https://example.org/reference\"");
    assert_not_contains(&html, "https://assets.example.org/remote.png");
    assert_not_contains(&html, "assets/local.png");
    assert_not_contains(&html, "data:image/svg+xml");
    assert_contains(&html, "src=\"#blocked-external-image\"");
    assert_contains(&html, "src=\"data:image/png;base64,iVBORw0KGgo=\"");
    assert_not_contains(&html, "SENTINEL_INTERNAL_RENDER");
    assert_not_contains(&html, "SENTINEL_INTERNAL_GOAL");
    assert_not_contains(&html, "SENTINEL_DIAGNOSTIC_RENDER");
    assert_not_contains(&html, "SENTINEL_DIAGNOSTIC_MESSAGE");
    assert_not_contains(&html, "sentinel.diagnostic");
}

#[test]
fn render_html_namespaces_markdown_footnotes_by_presentation_section() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    value["report"]["presentation"] = json!({
        "thesis": "Repeated footnote labels remain local to their narrative sections.",
        "organizing_form": "Two-section test",
        "rationale": "Exercise generated DOM identifiers.",
        "alternatives_considered": "A single combined narrative was rejected because it would not exercise section-local identifiers.",
        "sections": [
            {
                "id": "section-first",
                "title": "First section",
                "body_markdown": "First claim.[^note]\n\n[^note]: First section note."
            },
            {
                "id": "section-second",
                "title": "Second section",
                "body_markdown": "Second claim.[^note]\n\n[^note]: Second section note."
            }
        ]
    });

    let html = render_report_value_to_html(dir.path(), "footnote-report.json", &value);
    assert_contains(&html, "href=\"#section-first-note\"");
    assert_contains(&html, "id=\"section-first-note\"");
    assert_contains(&html, "href=\"#section-second-note\"");
    assert_contains(&html, "id=\"section-second-note\"");
}

#[test]
fn render_html_rejects_non_human_or_invalid_reports() {
    let dir = tempfile::tempdir().unwrap();
    let scaffold_path = dir.path().join("scaffold-report.json");
    let output_path = dir.path().join("scaffold.html");
    let mut scaffold = current_human_report_value();
    scaffold["metadata"]["report_type"] = json!("scaffold");
    report::write_json_file(&scaffold_path, &scaffold).unwrap();
    assert_contains(
        &report::render_html_report_file(&scaffold_path, &output_path)
            .unwrap_err()
            .to_string(),
        "human_report",
    );

    let invalid_path = dir.path().join("invalid-report.json");
    let mut invalid = current_human_report_value();
    invalid["report"]["claims"][2]["evidence_links"] = json!([]);
    report::write_json_file(&invalid_path, &invalid).unwrap();
    assert_contains(
        &report::render_html_report_file(&invalid_path, &output_path)
            .unwrap_err()
            .to_string(),
        "validation has",
    );
}

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
            "{}\n\nCurrent density, current collector design, current focusing, stripping current, and critical current are technical quantities in this paragraph.\n",
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

#[test]
fn access_audit_and_download_policy_match_contract() {
    let flat = Source {
        url: "https://example.test/source.pdf".to_string(),
        access_status: " Free_Web ".to_string(),
        budget_estimate: "$0".to_string(),
        ..Source::default()
    };
    let flat_access = source_access(&flat);
    assert_eq!(flat_access.status, "free_web");
    assert_eq!(flat_access.route, "https://example.test/source.pdf");

    let complete = Source {
        title: "Open review".to_string(),
        source_type: "paper".to_string(),
        url: "https://example.test/review.pdf".to_string(),
        access_status: "open_access".to_string(),
        access_route: "Official URL".to_string(),
        why_it_matters: "Sets the research frame.".to_string(),
        use_in_curriculum: "Read before the first module.".to_string(),
        ..Source::default()
    };
    assert!(audit_source(&complete, 0).is_empty());

    let missing = audit_source(&Source::default(), 0);
    for want in [
        "source-1: missing title/citation",
        "source-1: missing type",
        "source-1: missing identifier or url",
        "source-1: missing access status",
        "source-1: missing access route",
        "source-1: missing source role or relevance note",
    ] {
        assert!(
            missing.contains(&want.to_string()),
            "{missing:?} should contain {want}"
        );
    }

    let allowed = parse_status_set(&sorted_open_access_statuses().join(","));
    for status in [
        "open_access",
        " FREE_WEB ",
        "public_domain",
        "cc_by",
        "cc_by_sa",
        "official_open",
        "user_provided",
    ] {
        assert_eq!(
            can_download(
                &Source {
                    access_status: status.to_string(),
                    ..Source::default()
                },
                &allowed,
                false
            ),
            (true, "allowed".to_string())
        );
    }
    assert_eq!(
        can_download(
            &Source {
                access_status: "unknown".to_string(),
                ..Source::default()
            },
            &allowed,
            false,
        ),
        (false, "blocked access status: unknown".to_string())
    );
    assert_eq!(
        can_download(&Source::default(), &allowed, true),
        (true, "unknown allowed by flag".to_string())
    );
}

#[test]
fn filename_and_download_record_helpers_are_stable() {
    assert_eq!(
        sanitize_filename("Field Notes: 2026.pdf"),
        "Field-Notes-2026.pdf"
    );
    assert_eq!(
        sanitize_filename(" folder/report?.pdf "),
        "folder-report-.pdf"
    );
    assert_eq!(sanitize_filename("../../secret.pdf"), "secret.pdf");
    assert_eq!(sanitize_filename("***"), "source.bin");
    assert_eq!(common_extension("Application/PDF"), Some(".pdf"));

    assert_eq!(
        filename_from_headers(
            "https://example.test/ignored",
            Some(r#"attachment; filename="../../My Paper Final.pdf""#),
            Some("application/pdf"),
            "fallback",
        ),
        "My-Paper-Final.pdf"
    );
    assert_eq!(
        filename_from_headers(
            "https://example.test/files/report.pdf?download=1",
            None,
            Some("application/pdf"),
            "fallback",
        ),
        "report.pdf"
    );
    assert_eq!(
        filename_from_headers(
            "https://example.test/",
            None,
            Some("text/plain; charset=utf-8"),
            "Lecture Notes"
        ),
        "Lecture-Notes.txt"
    );
    assert_eq!(
        filename_from_headers("https://example.test/", None, None, "Untitled Source"),
        "Untitled-Source.bin"
    );

    let mut buf = Vec::new();
    write_download_record(
        &mut buf,
        &DownloadRecord {
            title: "Open paper".to_string(),
            url: "https://example.test/open.pdf".to_string(),
            status: "downloaded".to_string(),
            path: "/tmp/open.pdf".to_string(),
            bytes: 123,
            content_type: "application/pdf".to_string(),
            ..DownloadRecord::default()
        },
    )
    .unwrap();
    assert!(String::from_utf8(buf.clone()).unwrap().ends_with('\n'));
    let decoded: DownloadRecord = serde_json::from_slice(buf.split_last().unwrap().1).unwrap();
    assert_eq!(decoded.title, "Open paper");
    assert_eq!(decoded.bytes, 123);
}

#[test]
fn download_sources_writes_success_blocked_and_failed_records() {
    let Some(server) = spawn_server(2, |path| match path {
        "/open" => http_response(
            "200 OK",
            &[
                ("Content-Type", "text/plain"),
                (
                    "Content-Disposition",
                    r#"attachment; filename="../../Unsafe Notes.txt""#,
                ),
            ],
            b"open body",
        ),
        "/fail" => http_response(
            "500 Internal Server Error",
            &[("Content-Type", "text/plain")],
            b"failed",
        ),
        _ => http_response(
            "404 Not Found",
            &[("Content-Type", "text/plain")],
            b"missing",
        ),
    }) else {
        return;
    };

    let dir = tempfile::tempdir().unwrap();
    let manifest = dir.path().join("sources.json");
    let sources = vec![
        Source {
            title: "Open normalized".to_string(),
            url: format!("{server}/open"),
            access_status: " Open_Access ".to_string(),
            ..Source::default()
        },
        Source {
            title: "Unknown blocked".to_string(),
            url: format!("{server}/open"),
            access_status: " Unknown ".to_string(),
            ..Source::default()
        },
        Source {
            title: "Server failure".to_string(),
            url: format!("{server}/fail"),
            access_status: "free_web".to_string(),
            ..Source::default()
        },
    ];
    fs::write(&manifest, serde_json::to_string(&sources).unwrap()).unwrap();
    let out_dir = dir.path().join("downloads");
    run_download_sources(&[
        "--manifest".to_string(),
        manifest.display().to_string(),
        "--out-dir".to_string(),
        out_dir.display().to_string(),
        "--allow-status".to_string(),
        " OPEN_ACCESS , FREE_WEB ".to_string(),
        "--timeout".to_string(),
        "5".to_string(),
    ])
    .unwrap();

    let records = read_download_records(&out_dir.join("download-log.jsonl"));
    assert_eq!(records.len(), 3);
    let by_title = records
        .into_iter()
        .map(|record| (record.title.clone(), record))
        .collect::<HashMap<_, _>>();
    let open = &by_title["Open normalized"];
    assert_eq!(open.status, "downloaded");
    assert_eq!(open.bytes, "open body".len() as i64);
    assert_eq!(open.content_type, "text/plain");
    let base = Path::new(&open.path).file_name().unwrap().to_string_lossy();
    assert!(!base.contains(".."));
    assert!(base.ends_with("-Unsafe-Notes.txt"));

    let blocked = &by_title["Unknown blocked"];
    assert_eq!(blocked.status, "skipped");
    assert_eq!(blocked.reason, "blocked access status: unknown");

    let failed = &by_title["Server failure"];
    assert_eq!(failed.status, "error");
    assert_eq!(failed.reason, "HTTP 500");
}

#[test]
fn download_sources_removes_oversized_partial_file() {
    let body = vec![b'x'; 1024 * 1024 + 1];
    let Some(server) = spawn_server(1, move |_| {
        http_response(
            "200 OK",
            &[
                ("Content-Type", "application/pdf"),
                (
                    "Content-Disposition",
                    r#"attachment; filename="../too-big.pdf""#,
                ),
            ],
            &body,
        )
    }) else {
        return;
    };

    let dir = tempfile::tempdir().unwrap();
    let manifest = dir.path().join("sources.json");
    let sources = vec![Source {
        title: "Too big".to_string(),
        url: format!("{server}/too-big.pdf"),
        access_status: "free_web".to_string(),
        ..Source::default()
    }];
    fs::write(&manifest, serde_json::to_string(&sources).unwrap()).unwrap();
    let out_dir = dir.path().join("downloads");
    run_download_sources(&[
        "--manifest".to_string(),
        manifest.display().to_string(),
        "--out-dir".to_string(),
        out_dir.display().to_string(),
        "--max-mb".to_string(),
        "1".to_string(),
    ])
    .unwrap();

    let records = read_download_records(&out_dir.join("download-log.jsonl"));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status, "error");
    assert_contains(&records[0].reason, "download exceeded 1048576 bytes limit");
    let entries = fs::read_dir(&out_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert_eq!(entries, vec!["download-log.jsonl"]);
}

fn spawn_server<F>(expected_requests: usize, handler: F) -> Option<String>
where
    F: Fn(&str) -> Vec<u8> + Send + Sync + 'static,
{
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => listener,
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("skipping local HTTP test; sandbox denied localhost bind: {err}");
            return None;
        }
        Err(err) => panic!("bind test HTTP server: {err}"),
    };
    let address = listener.local_addr().unwrap();
    let handler = Arc::new(handler);
    thread::spawn(move || {
        for stream in listener.incoming().take(expected_requests) {
            let mut stream = stream.unwrap();
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");
            let response = handler(path);
            stream.write_all(&response).unwrap();
        }
    });
    Some(format!("http://{address}"))
}

fn http_response(status: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (key, value) in headers {
        response.push_str(key);
        response.push_str(": ");
        response.push_str(value);
        response.push_str("\r\n");
    }
    response.push_str("\r\n");
    let mut bytes = response.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

fn read_download_records(path: &Path) -> Vec<DownloadRecord> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn report_source_ids_by_title(sources: &[Source]) -> BTreeMap<String, String> {
    report::normalize_report_sources(sources)
        .into_iter()
        .map(|source| (source.title, source.id))
        .collect()
}

fn write_lint_final_bundle(
    dir: &Path,
    include_reviewed_evidence: bool,
) -> (PathBuf, PathBuf, PathBuf, String) {
    let report_path = dir.join("human-report.md");
    let sources_path = dir.join("sources.csv");
    let evidence_path = dir.join("evidence.jsonl");
    let claim_statement = "Topology organizes learning around invariants and maps.".to_string();

    fs::write(
        &report_path,
        canonical_lint_final_markdown(&claim_statement),
    )
    .unwrap();
    fs::write(
        &sources_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
Open Review,review_article,doi:10.0000/open,https://example.test/open,2025-01-01,open_access,Official URL,$0,CC BY,foundation,Supports invariant framing.,Use in core module,Reviewed metadata.\n",
    )
    .unwrap();

    if include_reviewed_evidence {
        let normalized = report::normalize_source_manifest(&sources_path).unwrap();
        let source_id = normalized.sources[0].id.clone();
        let claim_id = report::content_id("claim", &[&claim_statement]);
        let reviewed = report::EvidenceEntry {
            evidence_id: report::content_id("ev", &[&source_id, &claim_id, "reviewed"]),
            source_id,
            verification_status: report::VerificationStatus::Reviewed,
            support_kind: report::SupportKind::Supports,
            locator: "section 1".to_string(),
            support_note: "The source directly frames topology through invariants and maps."
                .to_string(),
            reviewed_at: "2026-07-16".to_string(),
            claim_ids: vec![claim_id],
            notes: "Reviewed from bounded test fixture.".to_string(),
            ..report::EvidenceEntry::default()
        };
        report::write_jsonl_file(&evidence_path, &[reviewed]).unwrap();
    }

    (report_path, sources_path, evidence_path, claim_statement)
}

fn canonical_lint_final_markdown(claim_statement: &str) -> String {
    format!(
        r#"# Structure of Knowledge: Topology

## Report Architecture

| Item | Decision |
|---|---|
| Executive thesis | Topology becomes legible through the relation between transformations, invariants, and counterexamples. |
| Chosen organizing form | A transformation-and-invariant path. |
| Architecture rationale | This form follows the field's load-bearing relation instead of copying a generic topic hierarchy. |
| Rejected alternatives and why | A subfield survey was rejected because it hides the transformation-and-invariant relation. |

## Domain Decomposition

Topology studies properties of spaces preserved by continuous maps, using invariants to compare shape without relying on metric detail.

## Orientation

The report focuses on point-set foundations before algebraic examples.

## Field Element Inventory

| Element class | Observed element | Actual form in this field | Role | Load-bearing relations | Source IDs | Confidence |
|---|---|---|---|---|---|---|
| Object and transformation | Core objects | Spaces, continuous maps, quotient spaces, and invariants. | core | Maps act on spaces; invariants compare what maps preserve. | Open Review | high |
| Method and warrant | Methods and warrants | Proof by construction, counterexample, and functorial comparison. | core | Warrants test whether a proposed invariant or equivalence is valid. | Open Review | high |
| Representation | Representations | Commutative diagrams, chain complexes, and visual maps of spaces. | surrounding | Representations expose dependencies without replacing proof. | Open Review | high |
| Failure mode | Visual intuition as proof | Treating visual metaphors as definitions or assuming invariants are complete classifiers. | context | Counterexamples qualify intuitive claims. | Open Review | high |

## Source Role Probe

| Source role | Status | Candidate source pattern | What it tests | Waiver or revision rule |
|---|---|---|---|---|
| Foundation | Required | Standard text or review | Stabilizes definitions and proof grammar | Cannot be waived |

## Curriculum Roadmap

| Phase | Module | Essential question | Readings | Practice artifact | Progress criteria |
|---|---|---|---|---|---|
| Core | Invariance and maps | What does topology preserve? | Open Review | Compare metric and topological equivalence examples. | Explain maps; use examples |

## Claims

| Statement | Claim type | Evidence requirement | Source IDs | Confidence | Temporal status | Notes |
|---|---|---|---|---|---|---|
| {claim_statement} | structural | reviewed_source | Open Review | high | durable | Public claim. |
"#
    )
}

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate should live below repo root")
        .join(relative)
}

fn current_human_report_document() -> report::ReportDocument {
    report::export_markdown_report(
        &repo_path("cli/tests/fixtures/pipeline/report.md"),
        &repo_path("cli/tests/fixtures/pipeline/sources.csv"),
        Some(&repo_path(
            "cli/tests/fixtures/pipeline/reviewed-evidence.jsonl",
        )),
        report::ExportStage::Final,
    )
    .unwrap()
}

fn current_human_report_value() -> Value {
    serde_json::to_value(current_human_report_document()).unwrap()
}

fn current_scaffold_report_document() -> report::ReportDocument {
    let dir = tempfile::tempdir().unwrap();
    let scaffold_path = dir.path().join("scaffold.md");
    fs::write(
        &scaffold_path,
        build_report_scaffold(
            "Quantum sensing and metrology",
            "technical reader testing the current scaffold contract",
            "exercise the current scaffold export lane",
            8,
        ),
    )
    .unwrap();
    report::export_markdown_report(
        &scaffold_path,
        &repo_path("cli/tests/fixtures/pipeline/sources.csv"),
        Some(&repo_path(
            "cli/tests/fixtures/pipeline/reviewed-evidence.jsonl",
        )),
        report::ExportStage::Scaffold,
    )
    .unwrap()
}

fn current_scaffold_report_value() -> Value {
    serde_json::to_value(current_scaffold_report_document()).unwrap()
}

fn core_fixture_packages() -> crate::core::CorePackages {
    crate::core::CorePackages {
        knowledge: report::read_json_file(repo_path(
            "cli/tests/fixtures/core/knowledge-valid.json",
        ))
        .unwrap(),
        evidence: report::read_json_file(repo_path("cli/tests/fixtures/core/evidence-valid.json"))
            .unwrap(),
        pedagogy: report::read_json_file(repo_path("cli/tests/fixtures/core/pedagogy-valid.json"))
            .unwrap(),
    }
}

fn core_projection_context() -> report::ReportDocument {
    report::ReportDocument {
        metadata: report::ReportMetadata {
            schema_version: "sok-report/v2".to_string(),
            generated_at: "2026-07-16T00:00:00Z".to_string(),
            report_type: report::ReportType::HumanReport,
            temporal_review: report::TemporalMarker {
                as_of: "2026-07-16".to_string(),
                review_after: "2027-01-16".to_string(),
                temporal_status: report::TemporalStatus::Current,
                rationale: "Projection fixture context.".to_string(),
            },
            generator: Some(report::GeneratorInfo {
                name: "core projection fixture".to_string(),
                version: "1".to_string(),
            }),
        },
        report: report::PublicReport {
            field: "Unset field".to_string(),
            scope: report::Scope {
                summary: "Quantum sensing is read as a measurement chain.".to_string(),
                included: vec!["Core measurement-chain concepts".to_string()],
                excluded: Vec::new(),
                assumptions: Vec::new(),
                interpretive_notes: Vec::new(),
            },
            domain_profile: report::DomainProfile {
                classification: report::DomainClassification::InstrumentBound,
                rationale: "The fixture follows instrument-mediated measurement.".to_string(),
                ..report::DomainProfile::default()
            },
            presentation: Some(report::ReportPresentation {
                thesis: "Quantum sensing claims become legible as a measurement chain.".to_string(),
                organizing_form: "Measurement-chain path".to_string(),
                rationale: "This follows how a quantity becomes an estimate.".to_string(),
                alternatives_considered:
                    "A glossary-only report was rejected because it hides support relations."
                        .to_string(),
                sections: vec![report::ReportSection {
                    id: "section-measurement-chain-1111111111".to_string(),
                    title: "Measurement Chain".to_string(),
                    purpose: "Introduce the projected public narrative.".to_string(),
                    body_markdown:
                        "A physical quantity is encoded, disturbed, calibrated, and estimated."
                            .to_string(),
                    visual_view_ids: Vec::new(),
                }],
            }),
            evidence_standards: report::EvidenceStandards {
                summary: "Claims require reviewed support links in the evidence package."
                    .to_string(),
                claim_policy:
                    "Reviewed supports links need reviewed_at plus a locator or support note."
                        .to_string(),
                source_role_requirements: Vec::new(),
            },
            structure_waivers: vec![report::StructureWaiver {
                scope: report::StructureWaiverScope::Relations,
                rationale:
                    "Core projection fixture omits relation graph until canonical relation tests."
                        .to_string(),
            }],
            ..report::PublicReport::default()
        },
        internal_context: None,
        diagnostics: None,
    }
}

fn legacy_v1_report_value() -> Value {
    let mut value = current_human_report_value();
    value["metadata"]["schema_version"] = json!("sok-report/v1");
    value["report"]
        .as_object_mut()
        .unwrap()
        .remove("presentation");
    value
}

fn report_item_index(report_value: &Value, collection: &str, key: &str, expected: &str) -> usize {
    report_value["report"][collection]
        .as_array()
        .unwrap_or_else(|| panic!("report.{collection} should be an array"))
        .iter()
        .position(|item| item[key].as_str() == Some(expected))
        .unwrap_or_else(|| {
            panic!("report.{collection} should contain an item with {key} equal to {expected:?}")
        })
}

fn report_item_index_containing(
    report_value: &Value,
    collection: &str,
    key: &str,
    fragment: &str,
) -> usize {
    report_value["report"][collection]
        .as_array()
        .unwrap_or_else(|| panic!("report.{collection} should be an array"))
        .iter()
        .position(|item| {
            item[key]
                .as_str()
                .is_some_and(|value| value.contains(fragment))
        })
        .unwrap_or_else(|| {
            panic!("report.{collection} should contain an item whose {key} contains {fragment:?}")
        })
}

fn report_item_id(report_value: &Value, collection: &str, key: &str, expected: &str) -> String {
    let index = report_item_index(report_value, collection, key, expected);
    report_value["report"][collection][index]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("report.{collection}[{index}].id should be a string"))
        .to_string()
}

fn array_item_index(array: &Value, key: &str, expected: &str) -> usize {
    array
        .as_array()
        .expect("test fixture value should be an array")
        .iter()
        .position(|item| item[key].as_str() == Some(expected))
        .unwrap_or_else(|| panic!("array should contain an item with {key} equal to {expected:?}"))
}

fn report_relation_index_between(report_value: &Value, from_id: &str, to_id: &str) -> usize {
    report_value["report"]["relations"]
        .as_array()
        .expect("report.relations should be an array")
        .iter()
        .position(|relation| {
            relation["from"]["id"].as_str() == Some(from_id)
                && relation["to"]["id"].as_str() == Some(to_id)
        })
        .unwrap_or_else(|| {
            panic!("report.relations should contain an edge from {from_id:?} to {to_id:?}")
        })
}

fn load_repo_json(relative: &str) -> Value {
    let path = repo_path(relative);
    let data =
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    serde_json::from_str(&data).unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
}

fn render_report_value_to_html(dir: &Path, name: &str, value: &Value) -> String {
    let input_path = dir.join(name);
    let output_path = dir.join(format!("{name}.html"));
    report::write_json_file(&input_path, value).unwrap();
    report::render_html_report_file(&input_path, &output_path).unwrap();
    fs::read_to_string(&output_path).unwrap()
}

fn json_array_contains(value: &Value, needle: &str) -> bool {
    value
        .as_array()
        .map(|items| items.iter().any(|item| item.as_str() == Some(needle)))
        .unwrap_or(false)
}

fn validate_sok_report_smoke(value: &Value) -> std::result::Result<(), String> {
    let metadata = object_field(value, "metadata")?;
    let report = object_field(value, "report")?;
    require_string(metadata, "schema_version", "metadata")?;
    require_string(metadata, "generated_at", "metadata")?;
    let report_type = require_string(metadata, "report_type", "metadata")?;
    if !matches!(report_type, "scaffold" | "human_report") {
        return Err(format!("invalid report_type {report_type:?}"));
    }

    let temporal = object_child(metadata, "temporal_review", "metadata")?;
    require_string(temporal, "as_of", "metadata.temporal_review")?;
    let temporal_status = require_string(temporal, "temporal_status", "metadata.temporal_review")?;
    if !matches!(
        temporal_status,
        "durable" | "current" | "review_due" | "stale" | "unknown"
    ) {
        return Err(format!("invalid temporal_status {temporal_status:?}"));
    }

    for key in [
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
        if !report.contains_key(key) {
            return Err(format!("missing report.{key}"));
        }
    }

    for key in [
        "internal_context",
        "diagnostics",
        "raw_learner_profile",
        "original_goal",
        "prompt_derived_assumptions",
        "placeholder_state",
        "profile_proposals",
        "handoff_notes",
        "research_frame",
        "scaffold_quality_notes",
    ] {
        if report.contains_key(key) {
            return Err(format!("public report contains internal key {key}"));
        }
    }

    let mut ids: HashMap<String, BTreeSet<String>> = HashMap::new();
    collect_item_ids(report, "core_ideas", "concept", &mut ids)?;
    collect_item_ids(report, "methods", "method", &mut ids)?;
    collect_item_ids(report, "representations", "representation", &mut ids)?;
    if report.contains_key("field_elements") {
        collect_item_ids(report, "field_elements", "field_element", &mut ids)?;
    }
    collect_item_ids(report, "sources", "source", &mut ids)?;
    collect_item_ids(report, "claims", "claim", &mut ids)?;
    collect_item_ids(report, "curriculum_path", "curriculum_step", &mut ids)?;
    collect_item_ids(report, "frontier_debates", "frontier_debate", &mut ids)?;

    validate_sources(report)?;
    validate_literature_ladder_refs(report, &ids)?;
    validate_knowledge_source_refs(report, &ids)?;
    validate_claim_evidence(report, &ids)?;
    validate_curriculum_refs(report, &ids)?;
    validate_frontier_refs(report, &ids)?;
    let relation_ids = validate_relations(report, &ids)?;
    validate_visual_views(report, &ids, &relation_ids)?;
    validate_structure_waivers(report)?;
    Ok(())
}

fn object_field<'a>(
    value: &'a Value,
    key: &str,
) -> std::result::Result<&'a serde_json::Map<String, Value>, String> {
    value
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("missing {key} object"))
}

fn object_child<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> std::result::Result<&'a serde_json::Map<String, Value>, String> {
    object
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("missing {path}.{key} object"))
}

fn array_child<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> std::result::Result<&'a Vec<Value>, String> {
    object
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing {path}.{key} array"))
}

fn require_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> std::result::Result<&'a str, String> {
    let value = object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing {path}.{key} string"))?;
    if value.trim().is_empty() {
        return Err(format!("empty {path}.{key}"));
    }
    Ok(value)
}

fn collect_item_ids(
    report: &serde_json::Map<String, Value>,
    key: &str,
    entity_type: &str,
    ids: &mut HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    for item in array_child(report, key, "report")? {
        let object = item
            .as_object()
            .ok_or_else(|| format!("report.{key} item should be object"))?;
        let id = require_string(object, "id", &format!("report.{key}[]"))?;
        validate_stable_id(id)?;
        let inserted = ids
            .entry(entity_type.to_string())
            .or_default()
            .insert(id.to_string());
        if !inserted {
            return Err(format!("duplicate {entity_type} id {id}"));
        }
    }
    Ok(())
}

fn validate_sources(report: &serde_json::Map<String, Value>) -> std::result::Result<(), String> {
    for source in array_child(report, "sources", "report")? {
        let source = source
            .as_object()
            .ok_or_else(|| "report.sources item should be object".to_string())?;
        let id = require_string(source, "id", "report.sources[]")?;
        validate_stable_id(id)?;
        let access = object_child(source, "access", "report.sources[]")?;
        require_string(access, "status", "report.sources[].access")?;
        require_string(access, "route", "report.sources[].access")?;
        let verification = require_string(source, "verification_status", "report.sources[]")?;
        validate_verification_status(verification)?;
    }
    Ok(())
}

fn validate_knowledge_source_refs(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    for section in ["core_ideas", "methods", "representations", "field_elements"] {
        if !report.contains_key(section) {
            continue;
        }
        for item in array_child(report, section, "report")? {
            let item = item
                .as_object()
                .ok_or_else(|| format!("report.{section} item should be object"))?;
            validate_source_id_array(item, "source_ids", ids, &format!("report.{section}[]"))?;
        }
    }
    Ok(())
}

fn validate_literature_ladder_refs(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    let Some(rows) = report.get("literature_ladder") else {
        return Ok(());
    };
    let rows = rows
        .as_array()
        .ok_or_else(|| "report.literature_ladder should be array".to_string())?;
    for row in rows {
        let row = row
            .as_object()
            .ok_or_else(|| "report.literature_ladder item should be object".to_string())?;
        validate_stable_id(require_string(row, "id", "report.literature_ladder[]")?)?;
        require_string(row, "layer", "report.literature_ladder[]")?;
        require_string(row, "start_here", "report.literature_ladder[]")?;
        require_string(row, "read_for", "report.literature_ladder[]")?;
        require_string(row, "do_not_infer", "report.literature_ladder[]")?;
        validate_source_id_array(row, "source_ids", ids, "report.literature_ladder[]")?;
    }
    Ok(())
}

fn validate_claim_evidence(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    for claim in array_child(report, "claims", "report")? {
        let claim = claim
            .as_object()
            .ok_or_else(|| "report.claims item should be object".to_string())?;
        let links = array_child(claim, "evidence_links", "report.claims[]")?;
        for link in links {
            let link = link
                .as_object()
                .ok_or_else(|| "claim evidence link should be object".to_string())?;
            let source_id = require_string(link, "source_id", "claim.evidence_links[]")?;
            validate_stable_id(source_id)?;
            if !entity_id_exists(ids, "source", source_id) {
                return Err(format!("unknown evidence source_id {source_id}"));
            }
            let verification =
                require_string(link, "verification_status", "claim.evidence_links[]")?;
            validate_verification_status(verification)?;
        }
    }
    Ok(())
}

fn validate_curriculum_refs(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    for step in array_child(report, "curriculum_path", "report")? {
        let step = step
            .as_object()
            .ok_or_else(|| "report.curriculum_path item should be object".to_string())?;
        validate_source_id_array(step, "source_ids", ids, "report.curriculum_path[]")?;
        validate_known_id_array(step, "prerequisite_ids", ids, "report.curriculum_path[]")?;
    }
    Ok(())
}

fn validate_frontier_refs(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    for item in array_child(report, "frontier_debates", "report")? {
        let item = item
            .as_object()
            .ok_or_else(|| "report.frontier_debates item should be object".to_string())?;
        validate_source_id_array(item, "source_ids", ids, "report.frontier_debates[]")?;
        validate_typed_id_array(item, "claim_ids", "claim", ids, "report.frontier_debates[]")?;
        validate_known_id_array(
            item,
            "required_background_ids",
            ids,
            "report.frontier_debates[]",
        )?;
    }
    Ok(())
}

fn validate_source_id_array(
    object: &serde_json::Map<String, Value>,
    key: &str,
    ids: &HashMap<String, BTreeSet<String>>,
    path: &str,
) -> std::result::Result<(), String> {
    validate_typed_id_array(object, key, "source", ids, path)
}

fn validate_typed_id_array(
    object: &serde_json::Map<String, Value>,
    key: &str,
    entity_type: &str,
    ids: &HashMap<String, BTreeSet<String>>,
    path: &str,
) -> std::result::Result<(), String> {
    let Some(values) = object.get(key) else {
        return Ok(());
    };
    let values = values
        .as_array()
        .ok_or_else(|| format!("{path}.{key} should be array"))?;
    for value in values {
        let id = value
            .as_str()
            .ok_or_else(|| format!("{path}.{key} item should be string"))?;
        validate_stable_id(id)?;
        if !entity_id_exists(ids, entity_type, id) {
            return Err(format!("unknown {entity_type} id {id}"));
        }
    }
    Ok(())
}

fn validate_known_id_array(
    object: &serde_json::Map<String, Value>,
    key: &str,
    ids: &HashMap<String, BTreeSet<String>>,
    path: &str,
) -> std::result::Result<(), String> {
    let Some(values) = object.get(key) else {
        return Ok(());
    };
    let values = values
        .as_array()
        .ok_or_else(|| format!("{path}.{key} should be array"))?;
    for value in values {
        let id = value
            .as_str()
            .ok_or_else(|| format!("{path}.{key} item should be string"))?;
        validate_stable_id(id)?;
        if !known_id_exists(ids, id) {
            return Err(format!("unknown referenced id {id}"));
        }
    }
    Ok(())
}

fn validate_relations(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<BTreeSet<String>, String> {
    let mut relation_ids = BTreeSet::new();
    for relation in array_child(report, "relations", "report")? {
        let relation = relation
            .as_object()
            .ok_or_else(|| "report.relations item should be object".to_string())?;
        let id = require_string(relation, "id", "report.relations[]")?;
        validate_stable_id(id)?;
        if !relation_ids.insert(id.to_string()) {
            return Err(format!("duplicate relation id {id}"));
        }
        validate_relation_endpoint(relation, "from", ids)?;
        validate_relation_endpoint(relation, "to", ids)?;
    }
    Ok(relation_ids)
}

fn validate_relation_endpoint(
    relation: &serde_json::Map<String, Value>,
    key: &str,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    let endpoint = object_child(relation, key, "report.relations[]")?;
    let entity_type = require_string(endpoint, "entity_type", "relation endpoint")?;
    if !valid_entity_type(entity_type) {
        return Err(format!("invalid relation entity_type {entity_type}"));
    }
    let id = require_string(endpoint, "id", "relation endpoint")?;
    validate_stable_id(id)?;
    if !entity_id_exists(ids, entity_type, id) {
        return Err(format!("unknown relation endpoint {entity_type}:{id}"));
    }
    Ok(())
}

fn validate_visual_views(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
    relation_ids: &BTreeSet<String>,
) -> std::result::Result<(), String> {
    let Some(views) = report.get("visual_views") else {
        return Ok(());
    };
    let views = views
        .as_array()
        .ok_or_else(|| "report.visual_views should be array".to_string())?;
    for view in views {
        let view = view
            .as_object()
            .ok_or_else(|| "visual view should be object".to_string())?;
        validate_stable_id(require_string(view, "id", "visual_views[]")?)?;
        let mut visual_node_ids = BTreeSet::new();
        for node in array_child(view, "nodes", "visual_views[]")? {
            let node = node
                .as_object()
                .ok_or_else(|| "visual view node should be object".to_string())?;
            let node_id = require_string(node, "id", "visual_view.nodes[]")?;
            validate_stable_id(node_id)?;
            visual_node_ids.insert(node_id.to_string());
            if let Some(ref_id) = node.get("ref_id").and_then(Value::as_str) {
                validate_stable_id(ref_id)?;
                let entity_type = require_string(node, "entity_type", "visual_view.nodes[]")?;
                if !entity_id_exists(ids, entity_type, ref_id) {
                    return Err(format!("unknown visual ref_id {entity_type}:{ref_id}"));
                }
            }
        }
        for edge in array_child(view, "edges", "visual_views[]")? {
            let edge = edge
                .as_object()
                .ok_or_else(|| "visual view edge should be object".to_string())?;
            for key in ["from", "to"] {
                let endpoint = require_string(edge, key, "visual_view.edges[]")?;
                validate_stable_id(endpoint)?;
                if !visual_node_ids.contains(endpoint) {
                    return Err(format!("unknown visual edge endpoint {endpoint}"));
                }
            }
            if let Some(relation_id) = edge.get("relation_id").and_then(Value::as_str) {
                validate_stable_id(relation_id)?;
                if !relation_ids.contains(relation_id) {
                    return Err(format!("unknown visual relation_id {relation_id}"));
                }
            }
        }
    }
    Ok(())
}

fn validate_structure_waivers(
    report: &serde_json::Map<String, Value>,
) -> std::result::Result<(), String> {
    let Some(waivers) = report.get("structure_waivers") else {
        return Ok(());
    };
    let waivers = waivers
        .as_array()
        .ok_or_else(|| "report.structure_waivers should be array".to_string())?;
    for waiver in waivers {
        let waiver = waiver
            .as_object()
            .ok_or_else(|| "structure waiver should be object".to_string())?;
        let scope = require_string(waiver, "scope", "report.structure_waivers[]")?;
        if !matches!(
            scope,
            "relations" | "curriculum_prerequisites" | "visual_views"
        ) {
            return Err(format!("invalid structure waiver scope {scope:?}"));
        }
        require_string(waiver, "rationale", "report.structure_waivers[]")?;
    }
    Ok(())
}

fn validate_stable_id(id: &str) -> std::result::Result<(), String> {
    let valid = id.contains('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .chars()
            .next()
            .map(|first| first.is_ascii_lowercase())
            .unwrap_or(false)
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
    if valid {
        Ok(())
    } else {
        Err(format!("invalid stable id {id:?}"))
    }
}

fn validate_verification_status(status: &str) -> std::result::Result<(), String> {
    if matches!(status, "cataloged" | "reviewed" | "verified") {
        Ok(())
    } else {
        Err(format!("invalid verification status {status:?}"))
    }
}

fn entity_id_exists(ids: &HashMap<String, BTreeSet<String>>, entity_type: &str, id: &str) -> bool {
    ids.get(entity_type)
        .map(|known| known.contains(id))
        .unwrap_or(false)
}

fn known_id_exists(ids: &HashMap<String, BTreeSet<String>>, id: &str) -> bool {
    ids.values().any(|known| known.contains(id))
}

fn valid_entity_type(entity_type: &str) -> bool {
    matches!(
        entity_type,
        "concept"
            | "claim"
            | "source"
            | "curriculum_step"
            | "frontier_debate"
            | "method"
            | "representation"
            | "field_element"
    )
}

fn assert_validation_check(
    validation: &report::ReportValidation,
    check_id: &str,
    severity: report::DiagnosticSeverity,
) {
    assert!(
        validation
            .diagnostics
            .checks
            .iter()
            .any(|check| check.check_id == check_id && check.severity == severity),
        "expected validation diagnostics to include {severity:?} {check_id}; got {:?}",
        validation.diagnostics.checks
    );
}

fn assert_validation_message(validation: &report::ReportValidation, message_fragment: &str) {
    assert!(
        validation
            .diagnostics
            .checks
            .iter()
            .any(
                |check| check.check_id == report::CHECK_VALIDATE_VISUAL_REFERENCE
                    && check.severity == report::DiagnosticSeverity::Error
                    && check.message.contains(message_fragment)
            ),
        "expected visual validation error containing {message_fragment:?}; got {:?}",
        validation.diagnostics.checks
    );
}

fn assert_no_validation_check(
    validation: &report::ReportValidation,
    check_id: &str,
    severity: report::DiagnosticSeverity,
) {
    assert!(
        !validation
            .diagnostics
            .checks
            .iter()
            .any(|check| check.check_id == check_id && check.severity == severity),
        "expected validation diagnostics not to include {severity:?} {check_id}; got {:?}",
        validation.diagnostics.checks
    );
}

fn validation_diagnostic_order(
    validation: &report::ReportValidation,
) -> Vec<(u8, String, String, String, String)> {
    validation
        .diagnostics
        .checks
        .iter()
        .map(|check| {
            (
                match check.severity {
                    report::DiagnosticSeverity::Error => 0,
                    report::DiagnosticSeverity::Warning => 1,
                    report::DiagnosticSeverity::Info => 2,
                },
                check.check_id.clone(),
                check.target_path.clone(),
                check.entity_id.clone(),
                check.message.clone(),
            )
        })
        .collect()
}

fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "expected {haystack:?} to contain {needle:?}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str) {
    assert!(
        !haystack.contains(needle),
        "expected {haystack:?} not to contain {needle:?}"
    );
}

fn assert_before(haystack: &str, first: &str, second: &str) {
    let first_index = haystack
        .find(first)
        .unwrap_or_else(|| panic!("expected {haystack:?} to contain {first:?}"));
    let second_index = haystack
        .find(second)
        .unwrap_or_else(|| panic!("expected {haystack:?} to contain {second:?}"));
    assert!(
        first_index < second_index,
        "expected {first:?} to appear before {second:?}"
    );
}
