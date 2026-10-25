use super::*;

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

    let interface_step = exported
        .report
        .curriculum_path
        .iter()
        .find(|step| step.title == "Interface measurement")
        .unwrap();
    assert_eq!(exported.report.relations.len(), 4);
    let relation = exported
        .report
        .relations
        .iter()
        .find(|relation| relation.id == "rel-interface-measurement-after-grammar")
        .unwrap();
    assert_eq!(relation.kind, report::RelationKind::RequiresBefore);
    assert_eq!(
        relation.from.entity_type,
        report::EntityType::CurriculumStep
    );
    assert!(relation
        .from
        .id
        .starts_with("step-electrochemical-grammar-"));
    assert_eq!(relation.to.id, interface_step.id);
    assert_eq!(relation.to.entity_type, report::EntityType::CurriculumStep);
    assert_eq!(relation.source_ids.len(), 1);

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
        "phenomenon becomes data",
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
