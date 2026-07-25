use super::*;
use serde_json::{json, Value};
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
fn report_scaffold_keeps_domain_specific_contracts() {
    let formal = build_report_scaffold(
        "algebraic topology",
        "doctoral mathematician",
        "enter current research",
        16,
    );
    assert_contains(
        &formal,
        "| Domain classification | formal / well-structured |",
    );
    assert_contains(&formal, "Use prerequisite and proof graphs");
    assert_contains(&formal, "## 4. Concept and Prerequisite Map");
    assert_contains(&formal, "Counterexamples");
    assert_contains(&formal, "| Recent survey / frontier | Conditional |");
    assert_contains(&formal, "Formal-field audit");
    assert_not_contains(&formal, "TODO");

    let ill = build_report_scaffold(
        "comparative constitutional law",
        "policy researcher",
        "read scholarly debates",
        10,
    );
    assert_contains(
        &ill,
        "| Domain classification | ill-structured / interpretive |",
    );
    assert_contains(&ill, "cases, schools, interpretive lenses");
    assert_contains(&ill, "## 4. Debate and Case Network");
    assert_contains(&ill, "Do not let one school become the field");
    assert_contains(&ill, "Ill-structured-field audit");

    let infrastructure = build_report_scaffold(
        "particle physics",
        "physics-adjacent engineer",
        "map the field",
        12,
    );
    assert_contains(
        &infrastructure,
        "| Domain classification | infrastructure-bound / instrument-bound |",
    );
    assert_contains(&infrastructure, "Instrument, Data, and Standards Map");
    assert_contains(
        &infrastructure,
        "Cannot be waived for infrastructure-bound claims",
    );
    assert_contains(&infrastructure, "Infrastructure-bound-field audit");

    let mixed = build_report_scaffold("causal inference", "research lead", "build a path", 6);
    assert_contains(
        &mixed,
        "| Domain classification | mixed / classification to verify |",
    );
    assert_contains(&mixed, "hybrid profile unresolved");
    assert_contains(&mixed, "Waive for stable core-skill paths with rationale");
    assert_contains(
        &mixed,
        "Revise the profile after source review; do not let the initial keyword guess settle the structure.",
    );
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
        "Select only visual views justified by the report's structured relations",
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
    let final_report: report::ReportDocument =
        report::read_json_file(repo_path("reports/examples/sok-report.json")).unwrap();
    assert_eq!(
        final_report.metadata.report_type,
        report::ReportType::HumanReport
    );
    assert_eq!(
        final_report.report.sources[0].access.status,
        report::AccessStatus::PaidBook
    );
    assert_eq!(
        final_report.report.claims[0].evidence_links[0].verification_status,
        report::VerificationStatus::Reviewed
    );

    let encoded_link =
        serde_json::to_string(&final_report.report.claims[0].evidence_links[0]).unwrap();
    assert_contains(&encoded_link, "\"source_id\"");
    assert_contains(&encoded_link, "\"support_kind\"");
    assert_not_contains(&encoded_link, "sourceId");

    let scaffold: report::ReportDocument =
        report::read_json_file(repo_path("reports/examples/sok-scaffold-report.json")).unwrap();
    assert_eq!(scaffold.metadata.report_type, report::ReportType::Scaffold);
    assert!(scaffold.internal_context.is_some());
    assert_eq!(
        scaffold.diagnostics.unwrap().checks[0].severity,
        report::DiagnosticSeverity::Warning
    );
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
        "| Profile hypothesis | ",
        "| Profile hypothesis | SENTINEL_INTERNAL_ASSUMPTION: ",
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
    assert_eq!(exported.metadata.schema_version, "sok-report/v1");
    assert_eq!(exported.report.field, "topology");
    assert!(!exported.report.sources.is_empty());
    assert!(exported.diagnostics.is_some());

    let internal = exported.internal_context.as_ref().unwrap();
    assert_eq!(internal.raw_learner_profile, "SENTINEL_INTERNAL_LEARNER");
    assert_eq!(internal.original_goal, "SENTINEL_INTERNAL_GOAL");
    assert!(internal.placeholder_state.contains_key("placeholder_lines"));
    assert!(internal
        .prompt_derived_assumptions
        .iter()
        .any(|note| note.contains("SENTINEL_INTERNAL_ASSUMPTION")));
    assert!(internal
        .handoff_notes
        .iter()
        .any(|note| note.contains("starter source rows")));

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

## Domain Decomposition

Topology is organized around invariants, maps, and constructions that preserve structure under continuous deformation.

## Orientation

The report focuses on point-set foundations before algebraic examples.

## Deep Structure

| Element | SoK extraction |
|---|---|
| Core objects | Spaces, continuous maps, quotient spaces, and invariants. |
| Syntactic structure | Proof by construction, counterexample, and functorial comparison. |
| Representations | Commutative diagrams, chain complexes, and visual maps of spaces. |
| Failure modes | Treating visual metaphors as definitions; assuming invariants are complete classifiers. |

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

## Domain Decomposition

Solid-state batteries are an emerging, interdisciplinary field whose practical structure couples transport, interfaces, mechanics, cell architecture, and manufacturing constraints.

## Orientation

Read the field as a coupled system rather than as an electrolyte-conductivity ranking.

## Deep Structure

| Element | In this field | Sources |
|---|---|---|
| Core objects | Solid electrolytes, interfaces, electrodes, defects, and cell fixtures. | Open Review |
| Methods and warrants | Impedance claims require geometry, density, electrodes, temperature, fitting, and replication details. | Interface Study |
| Representations | Arrhenius plots, Nyquist plots, cross-sections, pressure-capacity maps, and process-flow diagrams. | Interface Study |

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

## Domain Decomposition

Solid-state batteries couple transport and cell design.

## Deep Structure

| Element | In this field |
|---|---|
| Core objects | Solid electrolytes and interfaces. |
| Open Review | A concept label that intentionally collides with a source title. |
| Methods and warrants | Measurement warrants. |
| Representations | Nyquist plots. |

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
    assert!(first.starts_with("claim-persistent-homology-"));

    let left = report::content_id_map("src", ["Alpha Source", "Beta Source"]);
    let right = report::content_id_map("src", ["Beta Source", "Alpha Source"]);
    assert_eq!(left, right);

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
}

#[test]
fn sok_report_fixtures_match_smoke_contract() {
    let final_report = load_repo_json("reports/examples/sok-report.json");
    validate_sok_report_smoke(&final_report).unwrap();
    assert_eq!(
        final_report["metadata"]["report_type"],
        json!("human_report")
    );
    assert!(final_report.get("internal_context").is_none());

    let scaffold_report = load_repo_json("reports/examples/sok-scaffold-report.json");
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
    let mut structured = load_repo_json("reports/examples/sok-report.json");
    structured["report"]["literature_ladder"] = json!([
        {
            "id": "ladder-foundation-munkres",
            "layer": "foundation",
            "start_here": "Begin with point-set definitions before algebraic topology.",
            "read_for": "Read for spaces, continuous maps, compactness, and quotient examples.",
            "do_not_infer": "Do not infer that visual deformation metaphors replace formal definitions.",
            "source_ids": ["src-munkres-topology"],
            "notes": "Compact fixture row for public contract coverage."
        }
    ]);

    validate_sok_report_smoke(&structured).unwrap();
    let validation = report::validate_report_value(&structured);
    assert_eq!(validation.error_count(), 0);
    assert_eq!(validation.warning_count(), 0);

    let document: report::ReportDocument = serde_json::from_value(structured.clone()).unwrap();
    assert_eq!(document.report.literature_ladder.len(), 1);
    assert_eq!(
        document.report.literature_ladder[0].source_ids,
        vec!["src-munkres-topology".to_string()]
    );
    assert_eq!(
        structured["report"]["relations"][0]["from"]["id"],
        json!("concept-quotient")
    );
    assert_eq!(
        structured["report"]["curriculum_path"][1]["prerequisite_ids"][0],
        json!("step-point-set")
    );
    assert_eq!(
        structured["report"]["visual_views"][0]["edges"][0]["relation_id"],
        json!("rel-step-algebraic-after-point-set")
    );

    let mut waived = load_repo_json("reports/examples/sok-report.json");
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

    let final_report = load_repo_json("reports/examples/sok-report.json");

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
    bad_relation_id["report"]["relations"][0]["to"]["id"] = json!("claim with spaces");
    assert_contains(
        &validate_sok_report_smoke(&bad_relation_id)
            .unwrap_err()
            .to_string(),
        "invalid stable id",
    );

    let mut bad_source_ref = final_report.clone();
    bad_source_ref["report"]["claims"][0]["evidence_links"][0]["source_id"] =
        json!("src-missing-source");
    assert_contains(
        &validate_sok_report_smoke(&bad_source_ref)
            .unwrap_err()
            .to_string(),
        "unknown evidence source_id",
    );
}

#[test]
fn validate_report_fixture_passes_without_diagnostics() {
    let final_report = load_repo_json("reports/examples/sok-report.json");
    let validation = report::validate_report_value(&final_report);
    assert_eq!(validation.error_count(), 0);
    assert_eq!(validation.warning_count(), 0);
    assert!(validation.diagnostics.checks.is_empty());

    let exit_code = run_cli(vec![
        "validate-report".to_string(),
        "--input".to_string(),
        repo_path("reports/examples/sok-report.json")
            .display()
            .to_string(),
        "--strict".to_string(),
    ])
    .unwrap();
    assert_eq!(exit_code, 0);
}

#[test]
fn validate_report_schema_and_public_boundary_errors_are_stable() {
    let final_report = load_repo_json("reports/examples/sok-report.json");

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
    let mut report_value = load_repo_json("reports/examples/sok-report.json");
    report_value["report"]["claims"][2]["evidence_links"] = json!([
        {
            "source_id": "src-otter-persistent-homology",
            "verification_status": "cataloged",
            "support_kind": "supports",
            "locator": "roadmap overview",
            "support_note": "Cataloged links cannot support frontier claims."
        }
    ]);
    let validation = report::validate_report_value(&report_value);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut unsupported = load_repo_json("reports/examples/sok-report.json");
    unsupported["report"]["claims"][0]["evidence_links"][0]["locator"] = json!("");
    unsupported["report"]["claims"][0]["evidence_links"][0]["support_note"] = json!("");
    let validation = report::validate_report_value(&unsupported);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SUPPORT,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_source_role_coverage_uses_explicit_requirements_and_waivers() {
    let mut required_gap = load_repo_json("reports/examples/sok-report.json");
    required_gap["report"]["evidence_standards"]["source_role_requirements"][0]
        ["minimum_sources"] = json!(3);
    let validation = report::validate_report_value(&required_gap);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SOURCE_ROLE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut missing_waiver = load_repo_json("reports/examples/sok-report.json");
    missing_waiver["report"]["evidence_standards"]["source_role_requirements"][2]
        .as_object_mut()
        .unwrap()
        .remove("waiver");
    let validation = report::validate_report_value(&missing_waiver);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_SOURCE_ROLE_WAIVER,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_currentness_is_structured_and_prose_is_warning_only() {
    let mut missing_temporal = load_repo_json("reports/examples/sok-report.json");
    missing_temporal["report"]["claims"][2]["temporal"]
        .as_object_mut()
        .unwrap()
        .remove("review_after");
    let validation = report::validate_report_value(&missing_temporal);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_CURRENTNESS_METADATA,
        report::DiagnosticSeverity::Error,
    );

    let mut future_source = load_repo_json("reports/examples/sok-report.json");
    future_source["report"]["sources"][2]["date"] = json!("2028");
    let validation = report::validate_report_value(&future_source);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_CURRENTNESS_SOURCE_DATE,
        report::DiagnosticSeverity::Error,
    );

    let mut prose = load_repo_json("reports/examples/sok-report.json");
    prose["report"]["claims"][0]["statement"] =
        json!("The latest topology curriculum still begins with invariance.");
    prose["report"]["claims"][0]["temporal"]["as_of"] = json!("");
    prose["report"]["claims"][0]["temporal"]["temporal_status"] = json!("unknown");
    let validation = report::validate_report_value(&prose);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_CURRENTNESS_PROSE,
        report::DiagnosticSeverity::Warning,
    );
}

#[test]
fn validate_report_relation_and_curriculum_consistency_are_errors() {
    let mut bad_kind = load_repo_json("reports/examples/sok-report.json");
    bad_kind["report"]["relations"][0]["kind"] = json!("unknown_relation");
    let validation = report::validate_report_value(&bad_kind);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_RELATION_KIND,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_endpoint = load_repo_json("reports/examples/sok-report.json");
    bad_endpoint["report"]["relations"][0]["to"]["id"] = json!("concept-missing");
    let validation = report::validate_report_value(&bad_endpoint);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_RELATION_ENDPOINT,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_step = load_repo_json("reports/examples/sok-report.json");
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
    let mut value = load_repo_json("reports/examples/sok-report.json");
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
    let mut value = load_repo_json("reports/examples/sok-report.json");
    value["report"]["evidence_standards"]["source_role_requirements"][1]["minimum_sources"] =
        json!(2);
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
    let mut value = load_repo_json("reports/examples/sok-report.json");
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
    let mut missing_relations = load_repo_json("reports/examples/sok-report.json");
    missing_relations["report"]["relations"] = json!([]);
    let validation = report::validate_report_value(&missing_relations);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_STRUCTURE_REQUIRED,
        report::DiagnosticSeverity::Error,
    );

    let mut missing_prerequisites = load_repo_json("reports/examples/sok-report.json");
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

    let mut missing_visual = load_repo_json("reports/examples/sok-report.json");
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
    let mut bad_ladder_source = load_repo_json("reports/examples/sok-report.json");
    bad_ladder_source["report"]["literature_ladder"] = json!([
        {
            "id": "ladder-bad-source",
            "layer": "foundation",
            "start_here": "Start with a missing source.",
            "read_for": "Endpoint validation.",
            "do_not_infer": "Do not infer missing sources.",
            "source_ids": ["src-missing-source"]
        }
    ]);
    let validation = report::validate_report_value(&bad_ladder_source);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SOURCE,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_claim_source = load_repo_json("reports/examples/sok-report.json");
    bad_claim_source["report"]["claims"][0]["evidence_links"][0]["source_id"] =
        json!("src-missing-source");
    let validation = report::validate_report_value(&bad_claim_source);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SOURCE,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_frontier_claim = load_repo_json("reports/examples/sok-report.json");
    bad_frontier_claim["report"]["frontier_debates"][0]["claim_ids"] = json!(["claim-missing"]);
    let validation = report::validate_report_value(&bad_frontier_claim);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_RELATION_ENDPOINT,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_visual_ref = load_repo_json("reports/examples/sok-report.json");
    bad_visual_ref["report"]["visual_views"][0]["nodes"][0]["ref_id"] = json!("step-missing");
    let validation = report::validate_report_value(&bad_visual_ref);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_visual_node = load_repo_json("reports/examples/sok-report.json");
    bad_visual_node["report"]["visual_views"][0]["edges"][0]["from"] = json!("vnode-missing");
    let validation = report::validate_report_value(&bad_visual_node);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );

    let mut bad_visual_relation = load_repo_json("reports/examples/sok-report.json");
    bad_visual_relation["report"]["visual_views"][0]["edges"][0]["relation_id"] =
        json!("rel-missing");
    let validation = report::validate_report_value(&bad_visual_relation);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );
}

#[test]
fn validate_report_evidence_support_semantics_are_strict() {
    let mut qualifies_only = load_repo_json("reports/examples/sok-report.json");
    qualifies_only["report"]["claims"][0]["evidence_links"] = json!([
        {
            "source_id": "src-munkres-topology",
            "verification_status": "reviewed",
            "support_kind": "qualifies",
            "locator": "introductory chapters",
            "support_note": "This narrows but does not affirm the claim.",
            "reviewed_at": "2026-07-16"
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

    let mut contradicts = load_repo_json("reports/examples/sok-report.json");
    contradicts["report"]["claims"][0]["evidence_links"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "source_id": "src-hatcher-algebraic-topology",
            "verification_status": "reviewed",
            "support_kind": "contradicts",
            "locator": "opening chapters",
            "support_note": "This conflicts with an overbroad form of the claim.",
            "reviewed_at": "2026-07-16"
        }));
    let validation = report::validate_report_value(&contradicts);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SUPPORT,
        report::DiagnosticSeverity::Warning,
    );
    assert_eq!(validation.error_count(), 0, "{:?}", validation.diagnostics);

    let mut missing_reviewed_at = load_repo_json("reports/examples/sok-report.json");
    missing_reviewed_at["report"]["claims"][0]["evidence_links"][0]
        .as_object_mut()
        .unwrap()
        .remove("reviewed_at");
    let validation = report::validate_report_value(&missing_reviewed_at);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_EVIDENCE_SUPPORT,
        report::DiagnosticSeverity::Error,
    );

    let mut cataloged_visible = load_repo_json("reports/examples/sok-report.json");
    cataloged_visible["report"]["claims"][0]["evidence_links"] = json!([
        {
            "source_id": "src-munkres-topology",
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
    let mut value = load_repo_json("reports/examples/sok-report.json");
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
fn render_html_renders_valid_fixture_as_self_contained_one_view() {
    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("sok-report.html");
    let exit = run_cli(vec![
        "render-html".to_string(),
        "--input".to_string(),
        repo_path("reports/examples/sok-report.json")
            .display()
            .to_string(),
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
    assert_contains(&html, "Core Ideas");
    assert_contains(&html, "Methods");
    assert_contains(&html, "Representations");
    assert_contains(&html, "Evidence Standards");
    assert_contains(&html, "Source Catalog");
    assert_contains(&html, "Claim Evidence Guide");
    assert_contains(&html, "Curriculum Path");
    assert_contains(&html, "Frontier Guidance");
    assert_contains(&html, "Relation Audit");
    assert_contains(&html, "id=\"visualizations\"");
    assert_contains(&html, "Topology dependency path");
    assert_contains(&html, "\"kind\":\"dependency_path\"");
    assert_contains(
        &html,
        "\"relation_id\":\"rel-step-algebraic-after-point-set\"",
    );
    assert_not_contains(&html, "\"kind\":\"extension\"");
    assert_contains(&html, "class=\"claim-card\"");
    assert_contains(&html, "<summary>Raw claim identifiers</summary>");
    assert_contains(&html, "class=\"visual-fallback\"");
    assert_contains(&html, "Point-set grammar -&gt; Homotopy and homology");
    assert_contains(&html, "class=\"text-fallback\"");
    assert_contains(&html, "Text alternative for curriculum path");
    assert_contains(&html, "role=\"region\"");
    assert_contains(&html, "<noscript>");
    assert_contains(&html, "@media (prefers-color-scheme: dark)");
    assert_contains(&html, "@media (max-width: 720px)");
    assert_contains(&html, "@media print");
    assert_contains(&html, ".table-scroll");
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
    let mut value = load_repo_json("reports/examples/sok-report.json");
    value["report"]
        .as_object_mut()
        .unwrap()
        .remove("visual_views");
    let html = render_report_value_to_html(dir.path(), "zero-view-report.json", &value);

    assert_contains(&html, "Reading Guide");
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
    let mut value = load_repo_json("reports/examples/sok-report.json");
    let existing_view = value["report"]["visual_views"][0].clone();
    value["report"]["relations"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "rel-munkres-supports-invariance",
            "kind": "supports",
            "from": {
                "entity_type": "source",
                "id": "src-munkres-topology"
            },
            "to": {
                "entity_type": "concept",
                "id": "concept-invariance"
            },
            "description": "The source supports the introductory invariant concept."
        }));
    value["report"]["visual_views"] = json!([
        existing_view,
        {
            "id": "view-concept-source-map",
            "kind": "concept_source",
            "title": "Concept source map",
            "justification": "A concept-source view is justified because foundational concepts are tied to reviewed source roles.",
            "nodes": [
                {
                    "id": "vnode-invariance",
                    "label": "Invariance",
                    "entity_type": "concept",
                    "ref_id": "concept-invariance"
                },
                {
                    "id": "vnode-munkres-source",
                    "label": "Munkres topology",
                    "entity_type": "source",
                    "ref_id": "src-munkres-topology"
                }
            ],
            "edges": [
                {
                    "from": "vnode-munkres-source",
                    "to": "vnode-invariance",
                    "kind": "supports",
                    "relation_id": "rel-munkres-supports-invariance"
                }
            ]
        },
        {
            "id": "view-frontier-debate-map",
            "kind": "frontier_debate",
            "title": "Applied frontier support",
            "justification": "A frontier/debate view is justified because the applied branch depends on a dated reviewed source and claim.",
            "nodes": [
                {
                    "id": "vnode-frontier-tda",
                    "label": "Persistent homology computation",
                    "entity_type": "frontier_debate",
                    "ref_id": "frontier-tda-computation"
                },
                {
                    "id": "vnode-claim-frontier",
                    "label": "Applied frontier claim",
                    "entity_type": "claim",
                    "ref_id": "claim-persistent-homology-frontier"
                },
                {
                    "id": "vnode-source-otter",
                    "label": "Otter roadmap",
                    "entity_type": "source",
                    "ref_id": "src-otter-persistent-homology"
                }
            ],
            "edges": [
                {
                    "from": "vnode-source-otter",
                    "to": "vnode-claim-frontier",
                    "kind": "supports",
                    "relation_id": "rel-frontier-supported-by-otter"
                },
                {
                    "from": "vnode-claim-frontier",
                    "to": "vnode-frontier-tda",
                    "kind": "maps_to"
                }
            ]
        }
    ]);

    let html = render_report_value_to_html(dir.path(), "multiple-views-report.json", &value);
    assert_eq!(html.matches("class=\"visual-card\"").count(), 3);
    assert_contains(&html, "\"kind\":\"dependency_path\"");
    assert_contains(&html, "\"kind\":\"concept_source\"");
    assert_contains(&html, "\"kind\":\"frontier_debate\"");
    assert_contains(&html, "Concept source map");
    assert_contains(&html, "Applied frontier support");
    assert_contains(&html, "Text alternative for Concept source map");
    assert_contains(&html, "Nodes");
    assert_contains(&html, "Edges");
}

#[test]
fn render_html_renders_literature_ladder_as_reader_path() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = load_repo_json("reports/examples/sok-report.json");
    value["report"]["literature_ladder"] = json!([
        {
            "id": "ladder-foundation-munkres",
            "layer": "Foundation",
            "start_here": "Start with point-set definitions before quotient examples.",
            "read_for": "Read for spaces, continuous maps, compactness, and quotient topology.",
            "do_not_infer": "Do not infer that visual deformation metaphors replace formal definitions.",
            "source_ids": ["src-munkres-topology"],
            "notes": "Use this before algebraic topology."
        }
    ]);

    let html = render_report_value_to_html(dir.path(), "ladder-report.json", &value);
    assert_contains(&html, "Start point");
    assert_contains(&html, "Read for");
    assert_contains(&html, "Do not infer");
    assert_contains(
        &html,
        "Start with point-set definitions before quotient examples.",
    );
    assert_contains(&html, "Topology");
    assert_contains(&html, "<summary>Raw ladder identifiers</summary>");
    assert_before(
        &html,
        "Start with point-set definitions",
        "<summary>Raw ladder identifiers</summary>",
    );
}

#[test]
fn render_html_claim_cards_group_support_qualifiers_and_contradictions() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = load_repo_json("reports/examples/sok-report.json");
    value["report"]["claims"][0]["evidence_links"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "source_id": "src-hatcher-algebraic-topology",
            "verification_status": "reviewed",
            "support_kind": "qualifies",
            "locator": "opening chapters",
            "support_note": "Narrows the statement to invariants built by specific constructions.",
            "reviewed_at": "2026-07-16"
        }));
    value["report"]["claims"][0]["evidence_links"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "source_id": "src-otter-persistent-homology",
            "verification_status": "reviewed",
            "support_kind": "contradicts",
            "locator": "roadmap caveats",
            "support_note": "Warns against treating one invariant as a complete classifier.",
            "reviewed_at": "2026-07-16"
        }));

    let html = render_report_value_to_html(dir.path(), "claim-card-report.json", &value);
    assert_contains(&html, "class=\"claim-card\"");
    assert_contains(&html, "Supporting Evidence");
    assert_contains(&html, "Qualifying Evidence");
    assert_contains(&html, "Contradictory Evidence");
    assert_contains(
        &html,
        "Narrows the statement to invariants built by specific constructions.",
    );
    assert_contains(
        &html,
        "Warns against treating one invariant as a complete classifier.",
    );
    assert_contains(&html, "Algebraic Topology");
    assert_contains(
        &html,
        "A roadmap for the computation of persistent homology",
    );
    assert_contains(&html, "<summary>Raw claim identifiers</summary>");
}

#[test]
fn render_html_omits_visual_shell_when_edges_are_not_relation_backed() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = load_repo_json("reports/examples/sok-report.json");
    for edge in value["report"]["visual_views"][0]["edges"]
        .as_array_mut()
        .unwrap()
    {
        edge.as_object_mut().unwrap().remove("relation_id");
    }

    let html = render_report_value_to_html(dir.path(), "relationless-view-report.json", &value);
    assert_not_contains(&html, "id=\"visualizations\"");
    assert_not_contains(&html, "class=\"visual-card\"");
    assert_not_contains(&html, "data-visual-mount=");
    assert_not_contains(&html, "id=\"sok-visual-data\"");
}

#[test]
fn render_html_omits_visual_edge_when_relation_does_not_match_node_refs() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = load_repo_json("reports/examples/sok-report.json");
    value["report"]["visual_views"][0]["edges"][0]["relation_id"] =
        json!("rel-frontier-supported-by-otter");

    let html = render_report_value_to_html(dir.path(), "mismatched-view-report.json", &value);
    assert_not_contains(&html, "id=\"visualizations\"");
    assert_not_contains(&html, "data-visual-mount=");
    assert_not_contains(&html, "id=\"sok-visual-data\"");
}

#[test]
fn render_html_escapes_report_content_and_omits_internal_payloads() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = load_repo_json("reports/examples/sok-report.json");
    value["report"]["field"] = json!("Topology <img src=x onerror=alert(1)> \"quoted\"");
    value["report"]["scope"]["summary"] =
        json!("Summary </script><script>alert(1)</script> & <b>bold</b>");
    value["report"]["visual_views"][0]["nodes"][0]["label"] =
        json!("Node </script><script>alert(2)</script>");
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
    assert_not_contains(&html, "SENTINEL_INTERNAL_RENDER");
    assert_not_contains(&html, "SENTINEL_INTERNAL_GOAL");
    assert_not_contains(&html, "SENTINEL_DIAGNOSTIC_RENDER");
    assert_not_contains(&html, "SENTINEL_DIAGNOSTIC_MESSAGE");
    assert_not_contains(&html, "sentinel.diagnostic");
}

#[test]
fn render_html_rejects_non_human_or_invalid_reports() {
    let dir = tempfile::tempdir().unwrap();
    let scaffold_path = dir.path().join("scaffold-report.json");
    let output_path = dir.path().join("scaffold.html");
    let mut scaffold = load_repo_json("reports/examples/sok-report.json");
    scaffold["metadata"]["report_type"] = json!("scaffold");
    report::write_json_file(&scaffold_path, &scaffold).unwrap();
    assert_contains(
        &report::render_html_report_file(&scaffold_path, &output_path)
            .unwrap_err()
            .to_string(),
        "human_report",
    );

    let invalid_path = dir.path().join("invalid-report.json");
    let mut invalid = load_repo_json("reports/examples/sok-report.json");
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
    let report_path = repo_path("reports/examples/json-first-human-report.md");
    let sources_path = repo_path("reports/examples/json-first-sources.csv");
    let evidence_path = repo_path("reports/examples/json-first-reviewed-evidence.jsonl");

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
    assert!(exported.report.visual_views.is_empty());

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
    assert_contains(&html, "Topology");
    assert_not_contains(&html, "internal_context");
    assert_not_contains(&html, "diagnostics");
    assert_not_contains(&html, "Scaffold Quality Notes");
    assert_not_contains(&html.to_ascii_lowercase(), "mermaid");
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
        "source-1: missing curricular role",
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

## Domain Decomposition

Topology studies properties of spaces preserved by continuous maps, using invariants to compare shape without relying on metric detail.

## Orientation

The report focuses on point-set foundations before algebraic examples.

## Deep Structure

| Element | SoK extraction |
|---|---|
| Core objects | Spaces, continuous maps, quotient spaces, and invariants. |
| Syntactic structure | Proof by construction, counterexample, and functorial comparison. |
| Representations | Commutative diagrams, chain complexes, and visual maps of spaces. |
| Failure modes | Treating visual metaphors as definitions; assuming invariants are complete classifiers. |

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
    for section in ["core_ideas", "methods", "representations"] {
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
