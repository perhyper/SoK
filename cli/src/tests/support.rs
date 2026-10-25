use super::*;

pub(super) fn spawn_server<F>(expected_requests: usize, handler: F) -> Option<String>
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

pub(super) fn http_response(status: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
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

pub(super) fn read_download_records(path: &Path) -> Vec<DownloadRecord> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

pub(super) fn report_source_ids_by_title(sources: &[Source]) -> BTreeMap<String, String> {
    report::normalize_report_sources(sources)
        .into_iter()
        .map(|source| (source.title, source.id))
        .collect()
}

pub(super) fn write_lint_final_bundle(
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

pub(super) fn canonical_lint_final_markdown(claim_statement: &str) -> String {
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

pub(super) fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate should live below repo root")
        .join(relative)
}

pub(super) fn current_human_report_document() -> report::ReportDocument {
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

pub(super) fn current_human_report_value() -> Value {
    serde_json::to_value(current_human_report_document()).unwrap()
}

pub(super) fn current_scaffold_report_document() -> report::ReportDocument {
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

pub(super) fn current_scaffold_report_value() -> Value {
    serde_json::to_value(current_scaffold_report_document()).unwrap()
}

pub(super) fn supported_work_task_kinds() -> Vec<work::WorkTaskKind> {
    vec![
        work::WorkTaskKind::Framing,
        work::WorkTaskKind::SourceRoleClassification,
        work::WorkTaskKind::FieldElementExtraction,
        work::WorkTaskKind::ClaimProposals,
        work::WorkTaskKind::RelationProposals,
        work::WorkTaskKind::CurriculumPrerequisiteProposals,
        work::WorkTaskKind::ArchitectureComparison,
        work::WorkTaskKind::ConsistencyCritique,
        work::WorkTaskKind::ReportProjection,
    ]
}

pub(super) fn supported_work_task_kind_labels() -> Vec<&'static str> {
    supported_work_task_kinds()
        .into_iter()
        .map(work::work_task_kind_label)
        .collect()
}

pub(super) fn test_work_order(task_kind: work::WorkTaskKind) -> work::WorkOrder {
    let mut order = work::WorkOrder::new(
        "work-order-1111111111",
        task_kind,
        "Propose bounded SoK core patches.",
    );
    order.required_capabilities = vec![
        work::WorkCapability::BasicCompletion,
        work::WorkCapability::StructuredOutput,
    ];
    order
}

pub(super) fn test_work_result(
    order: &work::WorkOrder,
    patches: Vec<work::WorkPatch>,
) -> work::WorkResult {
    work::WorkResult::patch_proposal(
        order.work_order_id.clone(),
        "work-result-1111111111",
        order.task_kind,
        order.required_capabilities.clone(),
        patches,
    )
}

pub(super) fn valid_element_patch(patch_id: &str) -> work::WorkPatch {
    let element = crate::core::knowledge::KnowledgeElement {
        id: "element-command-string-4444444444".to_string(),
        element_class: "Context cue".to_string(),
        label: crate::core::knowledge::LocalizedText::plain("Command-like source phrase"),
        actual_form: crate::core::knowledge::LocalizedText::plain(
            "A shell-like phrase remains evidence text only.",
        ),
        semantic_roles: vec![crate::core::knowledge::SemanticRole::Context],
        role_note: "untrusted source text".to_string(),
        relation_ids: Vec::new(),
        load_bearing_relations: String::new(),
        source_ids: vec!["src-quantum-sensing-explained-1111111111".to_string()],
        confidence: Some(crate::core::knowledge::Confidence::Low),
    };
    work::WorkPatch::add(
        patch_id,
        work::CorePackageArea::Knowledge,
        "/knowledge/elements/-",
        serde_json::to_value(element).unwrap(),
    )
}

pub(super) fn core_fixture_packages() -> crate::core::CorePackages {
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

pub(super) fn core_projection_context() -> report::ReportDocument {
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

pub(super) fn legacy_v1_report_value() -> Value {
    let mut value = current_human_report_value();
    value["metadata"]["schema_version"] = json!("sok-report/v1");
    value["report"]
        .as_object_mut()
        .unwrap()
        .remove("presentation");
    value
}

pub(super) fn report_item_index(
    report_value: &Value,
    collection: &str,
    key: &str,
    expected: &str,
) -> usize {
    report_value["report"][collection]
        .as_array()
        .unwrap_or_else(|| panic!("report.{collection} should be an array"))
        .iter()
        .position(|item| item[key].as_str() == Some(expected))
        .unwrap_or_else(|| {
            panic!("report.{collection} should contain an item with {key} equal to {expected:?}")
        })
}

pub(super) fn report_item_index_containing(
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

pub(super) fn report_item_id(
    report_value: &Value,
    collection: &str,
    key: &str,
    expected: &str,
) -> String {
    let index = report_item_index(report_value, collection, key, expected);
    report_value["report"][collection][index]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("report.{collection}[{index}].id should be a string"))
        .to_string()
}

pub(super) fn array_item_index(array: &Value, key: &str, expected: &str) -> usize {
    array
        .as_array()
        .expect("test fixture value should be an array")
        .iter()
        .position(|item| item[key].as_str() == Some(expected))
        .unwrap_or_else(|| panic!("array should contain an item with {key} equal to {expected:?}"))
}

pub(super) fn report_relation_index_between(
    report_value: &Value,
    from_id: &str,
    to_id: &str,
) -> usize {
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

pub(super) fn first_hard_curriculum_relation_index(report_value: &Value) -> usize {
    report_value["report"]["relations"]
        .as_array()
        .expect("report.relations should be an array")
        .iter()
        .position(|relation| {
            relation["kind"].as_str() == Some("requires_before")
                && relation["to"]["entity_type"].as_str() == Some("curriculum_step")
        })
        .expect("report should contain a hard relation projected to curriculum")
}

pub(super) fn load_repo_json(relative: &str) -> Value {
    let path = repo_path(relative);
    let data =
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    serde_json::from_str(&data).unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
}

pub(super) fn render_report_value_to_html(dir: &Path, name: &str, value: &Value) -> String {
    let input_path = dir.join(name);
    let output_path = dir.join(format!("{name}.html"));
    report::write_json_file(&input_path, value).unwrap();
    report::render_html_report_file(&input_path, &output_path).unwrap();
    fs::read_to_string(&output_path).unwrap()
}

pub(super) fn json_array_contains(value: &Value, needle: &str) -> bool {
    value
        .as_array()
        .map(|items| items.iter().any(|item| item.as_str() == Some(needle)))
        .unwrap_or(false)
}

pub(super) fn assert_validation_check(
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

pub(super) fn assert_work_check(validation: &work::WorkValidationReport, check_id: &str) {
    assert!(
        validation.diagnostics.checks.iter().any(|check| {
            check.check_id == check_id && check.severity == report::DiagnosticSeverity::Error
        }),
        "expected work validation diagnostics to include error {check_id}; got {:?}",
        validation.diagnostics.checks
    );
}

pub(super) fn assert_validation_message(
    validation: &report::ReportValidation,
    message_fragment: &str,
) {
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

pub(super) fn assert_no_validation_check(
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

pub(super) fn validation_diagnostic_order(
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

pub(super) fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "expected {haystack:?} to contain {needle:?}"
    );
}

pub(super) fn assert_not_contains(haystack: &str, needle: &str) {
    assert!(
        !haystack.contains(needle),
        "expected {haystack:?} not to contain {needle:?}"
    );
}

pub(super) fn assert_before(haystack: &str, first: &str, second: &str) {
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
