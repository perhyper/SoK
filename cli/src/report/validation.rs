mod evidence;
mod schema;

pub(crate) use evidence::*;
pub(crate) use schema::*;

use super::ids::*;
use super::import::*;
use super::model::*;
use super::provenance::*;
use super::relations::*;
use super::*;
pub(crate) use crate::checks::{is_stable_id, sort_diagnostics};

pub const CHECK_SOURCE_MISSING_ACCESS_STATUS: &str = "source.missing-access-status";
pub const CHECK_SOURCE_MISSING_ACCESS_ROUTE: &str = "source.missing-access-route";
pub const CHECK_SOURCE_MISSING_CURRICULAR_USE: &str = "source.missing-curricular-use";
pub const CHECK_EVIDENCE_CATALOGED_ONLY: &str = "evidence.cataloged-only";
pub const CHECK_EXPORT_UNSUPPORTED_SECTION: &str = "export.unsupported-section";
pub const CHECK_EXPORT_AMBIGUOUS_SECTION: &str = "export.ambiguous-section";
pub const CHECK_EXPORT_UNKNOWN_SURFACE_MARKER: &str = "export.unknown-surface-marker";
pub const CHECK_EXPORT_UNKNOWN_DIRECTIVE: &str = "export.unknown-directive";
pub const CHECK_EXPORT_REPORT_ARCHITECTURE: &str = "export.report-architecture";
pub const CHECK_EXPORT_MISSING_FIELD: &str = "export.missing-field";
pub const CHECK_EXPORT_MISSING_PUBLIC_FIELD: &str = "export.missing-public-field";
pub const CHECK_EXPORT_INTERNAL_SECTION_IN_FINAL: &str = "export.internal-section-in-final";
pub const CHECK_EXPORT_UNKNOWN_EVIDENCE_SOURCE: &str = "export.unknown-evidence-source";
pub const CHECK_EXPORT_CLAIM_NEEDS_EVIDENCE: &str = "export.claim-needs-evidence";
pub const CHECK_EXPORT_UNRESOLVED_REFERENCE: &str = "export.unresolved-reference";
pub const CHECK_EXPORT_AMBIGUOUS_REFERENCE: &str = "export.ambiguous-reference";
pub const CHECK_EXPORT_MACHINE_INPUT: &str = "export.machine-input";
pub const CHECK_VALIDATE_SCHEMA_JSON: &str = "validate.schema.json";
pub const CHECK_VALIDATE_SCHEMA_REQUIRED: &str = "validate.schema.required";
pub const CHECK_VALIDATE_SCHEMA_DESERIALIZE: &str = "validate.schema.deserialize";
pub const CHECK_VALIDATE_PUBLIC_BOUNDARY: &str = "validate.public-boundary";
pub const CHECK_VALIDATE_EVIDENCE_REQUIRED: &str = "validate.evidence.required";
pub const CHECK_VALIDATE_EVIDENCE_SOURCE: &str = "validate.evidence.source";
pub const CHECK_VALIDATE_EVIDENCE_SUPPORT: &str = "validate.evidence.support";
pub const CHECK_VALIDATE_SOURCE_ROLE_REQUIRED: &str = "validate.source-role.required";
pub const CHECK_VALIDATE_SOURCE_ROLE_CONDITIONAL: &str = "validate.source-role.conditional";
pub const CHECK_VALIDATE_SOURCE_ROLE_WAIVER: &str = "validate.source-role.waiver";
pub const CHECK_VALIDATE_CURRENTNESS_METADATA: &str = "validate.currentness.metadata";
pub const CHECK_VALIDATE_CURRENTNESS_REVIEW_DUE: &str = "validate.currentness.review-due";
pub const CHECK_VALIDATE_CURRENTNESS_SOURCE_DATE: &str = "validate.currentness.source-date";
pub const CHECK_VALIDATE_CURRENTNESS_PROSE: &str = "validate.currentness.prose";
pub const CHECK_VALIDATE_RELATION_ENDPOINT: &str = "validate.relation.endpoint";
pub const CHECK_VALIDATE_RELATION_KIND: &str = "validate.relation.kind";
pub const CHECK_VALIDATE_CURRICULUM_REFERENCE: &str = "validate.curriculum.reference";
pub const CHECK_VALIDATE_SOURCE_ACCESS: &str = "validate.source-access";
pub const CHECK_VALIDATE_VISUAL_REFERENCE: &str = "validate.visual.reference";
pub const CHECK_VALIDATE_STRUCTURE_REQUIRED: &str = "validate.structure.required";
pub const CHECK_LINT_STRUCTURAL: &str = "lint.structural";
pub const CHECK_LINT_SCAFFOLD_UNRESOLVED: &str = "lint.scaffold-unresolved";
pub const CHECK_LINT_FINAL_PUBLIC_LEAKAGE: &str = "lint.final-public-leakage";

pub fn validate_report_file<P: AsRef<Path>>(path: P) -> Result<ReportValidation> {
    let path = path.as_ref();
    let data = fs::read(path).with_context(|| format!("read JSON {}", path.display()))?;
    let value = match serde_json::from_slice::<Value>(&data) {
        Ok(value) => value,
        Err(err) => {
            return Ok(ReportValidation::new(vec![DiagnosticCheck::error(
                CHECK_VALIDATE_SCHEMA_JSON,
                format!("invalid JSON in {}: {err}", path.display()),
            )
            .with_target("/", "")]));
        }
    };
    Ok(validate_report_value(&value))
}

pub fn validate_report_value(value: &Value) -> ReportValidation {
    let mut checks = Vec::new();
    validate_schema_level_fields(value, &mut checks);
    validate_known_enum_strings(value, &mut checks);

    let document = match serde_json::from_value::<ReportDocument>(value.clone()) {
        Ok(document) => Some(document),
        Err(err) => {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_DESERIALIZE,
                    format!("report JSON does not match the typed report model: {err}"),
                )
                .with_target("/", ""),
            );
            None
        }
    };

    if let Some(document) = document {
        let index = ReportIdIndex::from_report(&document.report, &mut checks);
        validate_embedded_diagnostics(&document, &mut checks);
        validate_public_report_boundary(value, document.metadata.report_type, &mut checks);
        validate_reference_consistency(&document.report, &index, &mut checks);
        validate_claim_evidence_requirements(&document.report, &index, &mut checks);
        validate_source_role_coverage(&document.report, &mut checks);
        validate_currentness(&document, &index, &mut checks);
        validate_relation_consistency(&document.report, &index, &mut checks);
        validate_visual_references(&document.report, &index, &mut checks);
        validate_source_access_metadata(&document.report, &mut checks);
        validate_structure_waivers(&document.report, &mut checks);
        validate_required_structure(&document, &mut checks);
    }

    ReportValidation::new(checks)
}

pub fn lint_markdown_report<R, S, E>(
    report_path: R,
    sources_path: S,
    evidence_path: Option<E>,
    stage: ExportStage,
) -> Result<ReportValidation>
where
    R: AsRef<Path>,
    S: AsRef<Path>,
    E: AsRef<Path>,
{
    let report_path = report_path.as_ref();
    let sources_path = sources_path.as_ref();
    let markdown = fs::read_to_string(report_path)
        .with_context(|| format!("read report Markdown {}", report_path.display()))?;
    let mut parsed = parse_markdown_report(&markdown);
    let normalized_sources = normalize_source_manifest(sources_path)?;
    let sources = normalized_sources.sources;
    let mut evidence = normalized_sources.evidence;
    let mut checks = Vec::new();

    checks.extend(select_lint_parse_diagnostics(&mut parsed.diagnostics));
    checks.extend(normalized_sources.diagnostics);

    if let Some(path) = evidence_path {
        let mut ledger_entries: Vec<EvidenceEntry> = read_jsonl_file(path)?;
        evidence.append(&mut ledger_entries);
    }

    if !parsed_has_any_canonical_content(&parsed) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_LINT_STRUCTURAL,
                "Markdown report has no canonical SoK title or sections",
            )
            .with_target("/report", ""),
        );
    }

    lint_stage_boundary(&markdown, &parsed, stage, &mut checks);
    validate_markdown_report_architecture(&parsed, stage, &mut checks);
    lint_source_role_coverage(&parsed, &sources, &mut checks);
    lint_evidence_sources(&evidence, &sources, &mut checks);
    lint_evidence_semantics(&evidence, &mut checks);
    lint_claim_evidence_support(&parsed, &sources, &evidence, stage, &mut checks);
    lint_markdown_currentness(&markdown, &mut checks);

    Ok(ReportValidation::new(checks))
}

#[derive(Debug, Default)]
pub(crate) struct ReportIdIndex {
    field_elements: BTreeSet<String>,
    concepts: BTreeSet<String>,
    methods: BTreeSet<String>,
    representations: BTreeSet<String>,
    sources: BTreeSet<String>,
    claims: BTreeSet<String>,
    curriculum_steps: BTreeSet<String>,
    frontier_debates: BTreeSet<String>,
    pub(crate) relations: BTreeSet<String>,
    all_entities: BTreeSet<String>,
}

impl ReportIdIndex {
    fn from_report(report: &PublicReport, checks: &mut Vec<DiagnosticCheck>) -> Self {
        let mut index = Self::default();
        let mut field_element_labels = BTreeMap::<String, String>::new();
        for (item_index, item) in report.field_elements.iter().enumerate() {
            index.insert_entity(
                "field_element",
                &item.id,
                format!("/report/field_elements/{item_index}/id"),
                checks,
            );
            for (field, value) in [
                ("element_class", item.element_class.as_str()),
                ("label", item.label.as_str()),
                ("actual_form", item.actual_form.as_str()),
                ("role", item.role.as_str()),
                (
                    "load_bearing_relations",
                    item.load_bearing_relations.as_str(),
                ),
            ] {
                if value.trim().is_empty() || is_placeholder_text(value) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_SCHEMA_REQUIRED,
                            format!(
                                "/report/field_elements/{item_index}/{field} must be non-empty and researched"
                            ),
                        )
                        .with_target(
                            format!("/report/field_elements/{item_index}/{field}"),
                            &item.id,
                        ),
                    );
                }
            }
            let normalized_label = normalize_alias_text(&item.label);
            if !normalized_label.is_empty() {
                if let Some(existing_id) =
                    field_element_labels.insert(normalized_label, item.id.clone())
                {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_SCHEMA_REQUIRED,
                            format!(
                                "duplicate field element label {:?} on {} and {}",
                                item.label, existing_id, item.id
                            ),
                        )
                        .with_target(
                            format!("/report/field_elements/{item_index}/label"),
                            &item.id,
                        ),
                    );
                }
            }
        }
        for (item_index, item) in report.core_ideas.iter().enumerate() {
            index.insert_entity(
                "concept",
                &item.id,
                format!("/report/core_ideas/{item_index}/id"),
                checks,
            );
        }
        for (item_index, item) in report.methods.iter().enumerate() {
            index.insert_entity(
                "method",
                &item.id,
                format!("/report/methods/{item_index}/id"),
                checks,
            );
        }
        for (item_index, item) in report.representations.iter().enumerate() {
            index.insert_entity(
                "representation",
                &item.id,
                format!("/report/representations/{item_index}/id"),
                checks,
            );
        }
        for (item_index, source) in report.sources.iter().enumerate() {
            index.insert_entity(
                "source",
                &source.id,
                format!("/report/sources/{item_index}/id"),
                checks,
            );
        }
        for (item_index, claim) in report.claims.iter().enumerate() {
            index.insert_entity(
                "claim",
                &claim.id,
                format!("/report/claims/{item_index}/id"),
                checks,
            );
        }
        for (item_index, step) in report.curriculum_path.iter().enumerate() {
            index.insert_entity(
                "curriculum_step",
                &step.id,
                format!("/report/curriculum_path/{item_index}/id"),
                checks,
            );
        }
        for (item_index, frontier) in report.frontier_debates.iter().enumerate() {
            index.insert_entity(
                "frontier_debate",
                &frontier.id,
                format!("/report/frontier_debates/{item_index}/id"),
                checks,
            );
        }
        for (item_index, relation) in report.relations.iter().enumerate() {
            if !index.relations.insert(relation.id.clone()) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_RELATION_ENDPOINT,
                        format!("duplicate relation id {}", relation.id),
                    )
                    .with_target(format!("/report/relations/{item_index}/id"), &relation.id),
                );
            }
        }
        index
    }

    fn insert_entity(
        &mut self,
        entity_type: &str,
        id: &str,
        path: String,
        checks: &mut Vec<DiagnosticCheck>,
    ) {
        if !is_stable_id(id) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("{path} is not a stable id: {id:?}"),
                )
                .with_target(&path, id),
            );
        }
        if !self.all_entities.insert(id.to_string()) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("duplicate public entity id {id}"),
                )
                .with_target(path.clone(), id),
            );
        }
        match entity_type {
            "field_element" => {
                self.field_elements.insert(id.to_string());
            }
            "concept" => {
                self.concepts.insert(id.to_string());
            }
            "method" => {
                self.methods.insert(id.to_string());
            }
            "representation" => {
                self.representations.insert(id.to_string());
            }
            "source" => {
                self.sources.insert(id.to_string());
            }
            "claim" => {
                self.claims.insert(id.to_string());
            }
            "curriculum_step" => {
                self.curriculum_steps.insert(id.to_string());
            }
            "frontier_debate" => {
                self.frontier_debates.insert(id.to_string());
            }
            _ => {}
        }
    }

    pub(crate) fn has_entity(&self, entity_type: EntityType, id: &str) -> bool {
        match entity_type {
            EntityType::FieldElement => self.field_elements.contains(id),
            EntityType::Concept => self.concepts.contains(id),
            EntityType::Claim => self.claims.contains(id),
            EntityType::Source => self.sources.contains(id),
            EntityType::CurriculumStep => self.curriculum_steps.contains(id),
            EntityType::FrontierDebate => self.frontier_debates.contains(id),
            EntityType::Method => self.methods.contains(id),
            EntityType::Representation => self.representations.contains(id),
        }
    }

    fn has_any_entity(&self, id: &str) -> bool {
        self.all_entities.contains(id)
    }
}

pub(crate) fn validate_embedded_diagnostics(
    document: &ReportDocument,
    checks: &mut Vec<DiagnosticCheck>,
) {
    let Some(diagnostics) = &document.diagnostics else {
        return;
    };
    for (index, embedded) in diagnostics.checks.iter().enumerate() {
        let mut check = embedded.clone();
        if check.target_path.trim().is_empty() {
            check.target_path = format!("/diagnostics/checks/{index}");
        }
        if embedded_diagnostic_has_accepted_loss_waiver(document, &check) {
            check.severity = DiagnosticSeverity::Info;
        }
        checks.push(check);
    }
}

pub(crate) fn embedded_diagnostic_has_accepted_loss_waiver(
    document: &ReportDocument,
    check: &DiagnosticCheck,
) -> bool {
    if check.status == Some(DiagnosticStatus::NotApplicable)
        && normalize_id_text(&check.message).contains("accepted loss")
    {
        return true;
    }
    embedded_visual_loss_diagnostic(check)
        && has_structure_waiver(&document.report, StructureWaiverScope::VisualViews)
}

pub(crate) fn embedded_visual_loss_diagnostic(check: &DiagnosticCheck) -> bool {
    check.check_id == CHECK_EXPORT_UNSUPPORTED_SECTION && {
        let message = normalize_id_text(&check.message);
        message.contains("visual summary") || message.contains("visual map")
    }
}

pub(crate) fn validate_public_report_boundary(
    value: &Value,
    report_type: ReportType,
    checks: &mut Vec<DiagnosticCheck>,
) {
    if report_type != ReportType::HumanReport {
        return;
    }
    let Some(report) = value.get("report") else {
        return;
    };
    scan_public_boundary(report, "/report", checks);
}

pub(crate) fn scan_public_boundary(value: &Value, path: &str, checks: &mut Vec<DiagnosticCheck>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let child_path = format!("{path}/{key}");
                if is_public_boundary_key(key) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_PUBLIC_BOUNDARY,
                            format!("human_report public payload contains non-public key {key:?}"),
                        )
                        .with_target(&child_path, ""),
                    );
                }
                scan_public_boundary(child, &child_path, checks);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                scan_public_boundary(child, &format!("{path}/{index}"), checks);
            }
        }
        Value::String(text) => {
            let normalized = normalize_id_text(text);
            for phrase in [
                "sentinel internal",
                "raw prompt intent",
                "prompt intent",
                "raw learner profile",
                "learner profile",
                "original goal",
                "scaffold quality notes",
                "internal context",
                "source to verify",
                "date after lookup",
                "placeholder",
                "todo",
            ] {
                if normalized.contains(phrase) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_PUBLIC_BOUNDARY,
                            format!(
                                "human_report public payload contains non-public placeholder or \
                                internal text {phrase:?}"
                            ),
                        )
                        .with_target(path.to_string(), ""),
                    );
                }
            }
        }
        _ => {}
    }
}

pub(crate) fn is_public_boundary_key(key: &str) -> bool {
    matches!(
        key,
        "internal_context"
            | "diagnostics"
            | "check_id"
            | "severity"
            | "target_path"
            | "raw_learner_profile"
            | "original_goal"
            | "prompt_derived_assumptions"
            | "placeholder_state"
            | "profile_proposals"
            | "handoff_notes"
            | "raw_prompt_intent"
            | "prompt_intent"
            | "research_frame"
            | "scaffold_quality_notes"
            | "quality_gate_diagnostics"
    )
}

pub(crate) fn validate_reference_consistency(
    report: &PublicReport,
    index: &ReportIdIndex,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for (item_index, item) in report.literature_ladder.iter().enumerate() {
        if !is_stable_id(&item.id) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!(
                        "/report/literature_ladder/{item_index}/id is not a stable id: {:?}",
                        item.id
                    ),
                )
                .with_target(
                    format!("/report/literature_ladder/{item_index}/id"),
                    &item.id,
                ),
            );
        }
        for (field, value) in [
            ("layer", &item.layer),
            ("start_here", &item.start_here),
            ("read_for", &item.read_for),
            ("do_not_infer", &item.do_not_infer),
        ] {
            if value.trim().is_empty() {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_SCHEMA_REQUIRED,
                        format!("/report/literature_ladder/{item_index}/{field} must be a non-empty string"),
                    )
                    .with_target(
                        format!("/report/literature_ladder/{item_index}/{field}"),
                        &item.id,
                    ),
                );
            }
        }
        if item.source_ids.is_empty() {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_EVIDENCE_SOURCE,
                    format!("literature ladder row {} has no source_ids", item.id),
                )
                .with_target(
                    format!("/report/literature_ladder/{item_index}/source_ids"),
                    &item.id,
                ),
            );
        }
        validate_source_refs(
            &item.source_ids,
            index,
            &format!("/report/literature_ladder/{item_index}/source_ids"),
            &item.id,
            checks,
        );
    }
    for (item_index, item) in report.field_elements.iter().enumerate() {
        validate_source_refs(
            &item.source_ids,
            index,
            &format!("/report/field_elements/{item_index}/source_ids"),
            &item.id,
            checks,
        );
    }
    for (item_index, item) in report.core_ideas.iter().enumerate() {
        validate_source_refs(
            &item.source_ids,
            index,
            &format!("/report/core_ideas/{item_index}/source_ids"),
            &item.id,
            checks,
        );
    }
    for (item_index, item) in report.methods.iter().enumerate() {
        validate_source_refs(
            &item.source_ids,
            index,
            &format!("/report/methods/{item_index}/source_ids"),
            &item.id,
            checks,
        );
    }
    for (item_index, item) in report.representations.iter().enumerate() {
        validate_source_refs(
            &item.source_ids,
            index,
            &format!("/report/representations/{item_index}/source_ids"),
            &item.id,
            checks,
        );
    }
    for (step_index, step) in report.curriculum_path.iter().enumerate() {
        validate_source_refs(
            &step.source_ids,
            index,
            &format!("/report/curriculum_path/{step_index}/source_ids"),
            &step.id,
            checks,
        );
        for prerequisite_id in &step.prerequisite_ids {
            if !index.has_any_entity(prerequisite_id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_CURRICULUM_REFERENCE,
                        format!(
                            "curriculum step {} references missing prerequisite {}",
                            step.id, prerequisite_id
                        ),
                    )
                    .with_target(
                        format!("/report/curriculum_path/{step_index}/prerequisite_ids"),
                        &step.id,
                    ),
                );
            }
            if prerequisite_id.starts_with("step-")
                && !index.curriculum_steps.contains(prerequisite_id)
            {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_CURRICULUM_REFERENCE,
                        format!(
                            "curriculum step {} references missing curriculum step {}",
                            step.id, prerequisite_id
                        ),
                    )
                    .with_target(
                        format!("/report/curriculum_path/{step_index}/prerequisite_ids"),
                        &step.id,
                    ),
                );
            }
        }
    }
    for (item_index, item) in report.frontier_debates.iter().enumerate() {
        validate_source_refs(
            &item.source_ids,
            index,
            &format!("/report/frontier_debates/{item_index}/source_ids"),
            &item.id,
            checks,
        );
        for claim_id in &item.claim_ids {
            if !index.claims.contains(claim_id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_RELATION_ENDPOINT,
                        format!(
                            "frontier/debate item {} references missing claim {}",
                            item.id, claim_id
                        ),
                    )
                    .with_target(
                        format!("/report/frontier_debates/{item_index}/claim_ids"),
                        &item.id,
                    ),
                );
            }
        }
        for background_id in &item.required_background_ids {
            if !index.has_any_entity(background_id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_RELATION_ENDPOINT,
                        format!(
                            "frontier/debate item {} references missing background {}",
                            item.id, background_id
                        ),
                    )
                    .with_target(
                        format!("/report/frontier_debates/{item_index}/required_background_ids"),
                        &item.id,
                    ),
                );
            }
        }
    }
    for (relation_index, relation) in report.relations.iter().enumerate() {
        validate_source_refs(
            &relation.source_ids,
            index,
            &format!("/report/relations/{relation_index}/source_ids"),
            &relation.id,
            checks,
        );
    }
    if let Some(presentation) = &report.presentation {
        let visual_view_ids = report
            .visual_views
            .iter()
            .map(|view| view.id.as_str())
            .collect::<BTreeSet<_>>();
        let mut section_ids = BTreeSet::new();
        let mut placed_visual_ids = BTreeSet::new();
        for (section_index, section) in presentation.sections.iter().enumerate() {
            if !is_stable_id(&section.id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_SCHEMA_REQUIRED,
                        format!(
                            "/report/presentation/sections/{section_index}/id is not a stable id: {:?}",
                            section.id
                        ),
                    )
                    .with_target(
                        format!("/report/presentation/sections/{section_index}/id"),
                        &section.id,
                    ),
                );
            }
            if is_reserved_renderer_id(&section.id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_SCHEMA_REQUIRED,
                        format!(
                            "report presentation section id {} is reserved by the HTML renderer",
                            section.id
                        ),
                    )
                    .with_target(
                        format!("/report/presentation/sections/{section_index}/id"),
                        &section.id,
                    ),
                );
            }
            if !section_ids.insert(section.id.as_str()) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_SCHEMA_REQUIRED,
                        format!("duplicate report presentation section id {}", section.id),
                    )
                    .with_target(
                        format!("/report/presentation/sections/{section_index}/id"),
                        &section.id,
                    ),
                );
            }
            for visual_view_id in &section.visual_view_ids {
                if !is_stable_id(visual_view_id) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_SCHEMA_REQUIRED,
                            format!(
                                "presentation section {} has non-stable visual view id {:?}",
                                section.id, visual_view_id
                            ),
                        )
                        .with_target(
                            format!(
                                "/report/presentation/sections/{section_index}/visual_view_ids"
                            ),
                            visual_view_id,
                        ),
                    );
                }
                if !visual_view_ids.contains(visual_view_id.as_str()) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_VISUAL_REFERENCE,
                            format!(
                                "report section {} references missing visual view {}",
                                section.id, visual_view_id
                            ),
                        )
                        .with_target(
                            format!(
                                "/report/presentation/sections/{section_index}/visual_view_ids"
                            ),
                            &section.id,
                        ),
                    );
                }
                if !placed_visual_ids.insert(visual_view_id.as_str()) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_VISUAL_REFERENCE,
                            format!(
                                "visual view {visual_view_id} is placed more than once in report.presentation"
                            ),
                        )
                        .with_target(
                            format!(
                                "/report/presentation/sections/{section_index}/visual_view_ids"
                            ),
                            visual_view_id,
                        ),
                    );
                }
            }
        }
    }
}

pub(crate) fn is_reserved_renderer_id(id: &str) -> bool {
    matches!(
        id,
        "main-report"
            | "evidence-appendix"
            | "unplaced-visualizations"
            | "reading-guide"
            | "visualizations"
            | "curriculum-path"
            | "reading-ladder"
            | "claims"
            | "frontier-debates"
            | "scope"
            | "domain-profile"
            | "field-elements"
            | "core-ideas"
            | "methods"
            | "representations"
            | "evidence-standards"
            | "sources-evidence"
            | "relations"
            | "sok-visual-data"
    ) || id.starts_with("visual-title-")
}

pub(crate) fn validate_source_refs(
    source_ids: &[String],
    index: &ReportIdIndex,
    path: &str,
    entity_id: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for source_id in source_ids {
        if !index.sources.contains(source_id) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_EVIDENCE_SOURCE,
                    format!("{entity_id} references missing source {source_id}"),
                )
                .with_target(path.to_string(), entity_id),
            );
        }
    }
}

pub(crate) fn validate_structure_waivers(report: &PublicReport, checks: &mut Vec<DiagnosticCheck>) {
    for (waiver_index, waiver) in report.structure_waivers.iter().enumerate() {
        if waiver.rationale.trim().is_empty() {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    "structure waiver rationale must be a non-empty string",
                )
                .with_target(
                    format!("/report/structure_waivers/{waiver_index}/rationale"),
                    "",
                ),
            );
        }
    }
}

pub(crate) fn validate_required_structure(
    document: &ReportDocument,
    checks: &mut Vec<DiagnosticCheck>,
) {
    let report = &document.report;
    if report.presentation.is_some() && report.field_elements.is_empty() {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_STRUCTURE_REQUIRED,
                "a declared report presentation needs non-empty report.field_elements",
            )
            .with_target("/report/field_elements", ""),
        );
    }
    if document.metadata.report_type != ReportType::HumanReport {
        return;
    }
    if document.metadata.schema_version == "sok-report/v2" {
        if report
            .presentation
            .as_ref()
            .map(|presentation| presentation.sections.is_empty())
            .unwrap_or(true)
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_STRUCTURE_REQUIRED,
                    "sok-report/v2 human_report needs a non-empty report.presentation",
                )
                .with_target("/report/presentation", ""),
            );
        }
        if report.field_elements.is_empty() {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_STRUCTURE_REQUIRED,
                    "sok-report/v2 human_report needs non-empty report.field_elements",
                )
                .with_target("/report/field_elements", ""),
            );
        }
        if report
            .presentation
            .as_ref()
            .is_some_and(|presentation| presentation.alternatives_considered.trim().is_empty())
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_STRUCTURE_REQUIRED,
                    "sok-report/v2 human_report needs a non-empty \
                        presentation.alternatives_considered decision trace",
                )
                .with_target("/report/presentation/alternatives_considered", ""),
            );
        }
        for (index, element) in report.field_elements.iter().enumerate() {
            let role = normalize_id_text(&element.role);
            if (role.contains("core") || role.contains("surrounding"))
                && element.source_ids.is_empty()
            {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_EVIDENCE_SOURCE,
                        format!(
                            "field element {} has role {:?} but no source_ids",
                            element.id, element.role
                        ),
                    )
                    .with_target(
                        format!("/report/field_elements/{index}/source_ids"),
                        &element.id,
                    ),
                );
            }
        }
    }
    if substantial_final_report(report) {
        if report.relations.is_empty()
            && !has_structure_waiver(report, StructureWaiverScope::Relations)
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_STRUCTURE_REQUIRED,
                    "substantial human_report needs non-empty report.relations or a structure waiver",
                )
                .with_target("/report/relations", ""),
            );
        }
        let has_curriculum_prerequisite = report
            .curriculum_path
            .iter()
            .any(|step| !step.prerequisite_ids.is_empty());
        if report.curriculum_path.len() > 1
            && !has_curriculum_prerequisite
            && !has_structure_waiver(report, StructureWaiverScope::CurriculumPrerequisites)
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_STRUCTURE_REQUIRED,
                    "a multi-step curriculum needs at least one prerequisite or a structure waiver",
                )
                .with_target("/report/curriculum_path", ""),
            );
        }
    }

    if report.visual_views.is_empty()
        && embedded_visual_intent_was_declared(document)
        && !has_structure_waiver(report, StructureWaiverScope::VisualViews)
    {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_STRUCTURE_REQUIRED,
                "source Markdown declared a visual map or visual summary, but report.visual_views is empty",
            )
            .with_target("/report/visual_views", ""),
        );
    }
}

pub(crate) fn substantial_final_report(report: &PublicReport) -> bool {
    if report.curriculum_path.len() > 1 {
        return true;
    }
    if report.claims.len() > 2 || report.frontier_debates.len() > 1 {
        return true;
    }
    let field_structure_count = if report.field_elements.is_empty() {
        report.core_ideas.len() + report.methods.len() + report.representations.len()
    } else {
        report.field_elements.len()
    };
    let structural_count = field_structure_count
        + report.literature_ladder.len()
        + report.curriculum_path.len()
        + report.frontier_debates.len();
    structural_count >= 7
}

pub(crate) fn embedded_visual_intent_was_declared(document: &ReportDocument) -> bool {
    document
        .diagnostics
        .as_ref()
        .map(|diagnostics| {
            diagnostics
                .checks
                .iter()
                .any(embedded_visual_loss_diagnostic)
        })
        .unwrap_or(false)
}

pub(crate) fn has_structure_waiver(report: &PublicReport, scope: StructureWaiverScope) -> bool {
    report
        .structure_waivers
        .iter()
        .any(|waiver| waiver.scope == scope && !waiver.rationale.trim().is_empty())
}

pub(crate) fn source_access_requires_notes(status: AccessStatus) -> bool {
    matches!(
        status,
        AccessStatus::PaidBook
            | AccessStatus::Paywalled
            | AccessStatus::Subscription
            | AccessStatus::Restricted
            | AccessStatus::Unknown
    )
}

pub(crate) fn source_access_requires_metadata_only(status: AccessStatus) -> bool {
    matches!(
        status,
        AccessStatus::PaidBook
            | AccessStatus::Paywalled
            | AccessStatus::Subscription
            | AccessStatus::Restricted
            | AccessStatus::Unknown
    )
}

pub(crate) fn structured_temporal_metadata_absent(marker: &TemporalMarker) -> bool {
    marker.as_of.trim().is_empty() || marker.temporal_status == TemporalStatus::Unknown
}

pub(crate) fn has_currentness_words(text: &str) -> bool {
    let normalized = normalize_id_text(text);
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    if words
        .iter()
        .any(|word| matches!(*word, "currently" | "recent" | "recently" | "latest"))
    {
        return true;
    }
    if contains_word_pair(&words, "as", "of") {
        return true;
    }
    if contains_review_date_construction(&words) {
        return true;
    }
    for index in 0..words.len().saturating_sub(1) {
        if words[index] != "current" {
            continue;
        }
        let next = words[index + 1];
        if is_exempt_current_compound(next) {
            continue;
        }
        if matches!(
            next,
            "state"
                | "status"
                | "evidence"
                | "literature"
                | "consensus"
                | "practice"
                | "recommendation"
                | "recommendations"
                | "guidance"
                | "frontier"
                | "standard"
                | "standards"
                | "version"
                | "review"
                | "snapshot"
        ) {
            return true;
        }
    }
    false
}

pub(crate) fn contains_word_pair(words: &[&str], left: &str, right: &str) -> bool {
    words
        .windows(2)
        .any(|pair| pair.first() == Some(&left) && pair.get(1) == Some(&right))
}

pub(crate) fn contains_review_date_construction(words: &[&str]) -> bool {
    contains_word_pair(words, "reviewed", "on")
        || contains_word_pair(words, "reviewed", "at")
        || contains_word_pair(words, "review", "date")
        || contains_word_pair(words, "review", "dated")
        || contains_word_pair(words, "last", "reviewed")
}

pub(crate) fn is_exempt_current_compound(next_word: &str) -> bool {
    matches!(
        next_word,
        "density" | "collector" | "focusing" | "stripping" | "critical"
    )
}

pub(crate) fn source_date_after_as_of(source_date: &str, as_of: &str) -> bool {
    if !looks_like_iso_date(as_of) {
        return false;
    }
    if looks_like_iso_date(source_date) {
        return source_date > as_of;
    }
    let Some(source_year) = leading_year(source_date) else {
        return false;
    };
    let Some(as_of_year) = leading_year(as_of) else {
        return false;
    };
    source_year > as_of_year
}

pub(crate) fn leading_year(value: &str) -> Option<i32> {
    let year = value.get(0..4)?;
    if year.chars().all(|ch| ch.is_ascii_digit()) {
        year.parse().ok()
    } else {
        None
    }
}

pub(crate) fn looks_like_iso_date(value: &str) -> bool {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

pub(crate) fn source_role_label(role: SourceRole) -> &'static str {
    match role {
        SourceRole::Orientation => "orientation",
        SourceRole::Foundation => "foundation",
        SourceRole::Method => "method",
        SourceRole::Representation => "representation",
        SourceRole::Evidence => "evidence",
        SourceRole::Synthesis => "synthesis",
        SourceRole::Frontier => "frontier",
        SourceRole::Debate => "debate",
        SourceRole::Standard => "standard",
        SourceRole::Dataset => "dataset",
        SourceRole::Infrastructure => "infrastructure",
        SourceRole::Critique => "critique",
        SourceRole::Curriculum => "curriculum",
        SourceRole::Other => "other",
    }
}

pub(crate) fn access_status_label(status: AccessStatus) -> &'static str {
    match status {
        AccessStatus::OpenAccess => "open_access",
        AccessStatus::FreeWeb => "free_web",
        AccessStatus::PublicDomain => "public_domain",
        AccessStatus::OfficialOpen => "official_open",
        AccessStatus::UserProvided => "user_provided",
        AccessStatus::Library => "library",
        AccessStatus::PaidBook => "paid_book",
        AccessStatus::Paywalled => "paywalled",
        AccessStatus::Subscription => "subscription",
        AccessStatus::Restricted => "restricted",
        AccessStatus::Unknown => "unknown",
    }
}

pub(crate) fn entity_type_label(entity_type: EntityType) -> &'static str {
    match entity_type {
        EntityType::Concept => "concept",
        EntityType::FieldElement => "field_element",
        EntityType::Claim => "claim",
        EntityType::Source => "source",
        EntityType::CurriculumStep => "curriculum_step",
        EntityType::FrontierDebate => "frontier_debate",
        EntityType::Method => "method",
        EntityType::Representation => "representation",
    }
}

impl DiagnosticCheck {
    pub fn new(
        check_id: impl Into<String>,
        severity: DiagnosticSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            check_id: check_id.into(),
            severity,
            message: message.into(),
            ..Self::default()
        }
    }

    pub fn warning(check_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(check_id, DiagnosticSeverity::Warning, message)
    }

    pub fn error(check_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(check_id, DiagnosticSeverity::Error, message)
    }

    pub fn with_target(
        mut self,
        target_path: impl Into<String>,
        entity_id: impl Into<String>,
    ) -> Self {
        self.target_path = target_path.into();
        self.entity_id = entity_id.into();
        self
    }
}
