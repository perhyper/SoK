use super::ids::*;
use super::import::*;
use super::model::*;
use super::provenance::*;
use super::relations::*;
use super::*;

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

pub(crate) fn validate_schema_level_fields(value: &Value, checks: &mut Vec<DiagnosticCheck>) {
    let Some(root) = value.as_object() else {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_SCHEMA_REQUIRED,
                "root report payload must be a JSON object",
            )
            .with_target("/", ""),
        );
        return;
    };

    for key in root.keys() {
        if !matches!(
            key.as_str(),
            "metadata" | "report" | "internal_context" | "diagnostics"
        ) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("unexpected top-level key {key:?}; expected metadata, report, internal_context, or diagnostics"),
                )
                .with_target(format!("/{key}"), ""),
            );
        }
    }

    let Some(metadata) = require_object_for_validation(root, "metadata", "/", checks) else {
        return;
    };
    let Some(report) = require_object_for_validation(root, "report", "/", checks) else {
        return;
    };

    validate_allowed_keys(
        metadata,
        "/metadata",
        &[
            "schema_version",
            "generated_at",
            "report_type",
            "temporal_review",
            "generator",
        ],
        checks,
    );
    validate_allowed_keys(
        report,
        "/report",
        &[
            "field",
            "scope",
            "domain_profile",
            "presentation",
            "literature_ladder",
            "field_elements",
            "core_ideas",
            "methods",
            "representations",
            "evidence_standards",
            "sources",
            "claims",
            "relations",
            "curriculum_path",
            "frontier_debates",
            "visual_views",
            "structure_waivers",
        ],
        checks,
    );

    for key in [
        "schema_version",
        "generated_at",
        "report_type",
        "temporal_review",
    ] {
        require_key(metadata, key, "/metadata", checks);
    }
    require_non_empty_string(metadata, "schema_version", "/metadata", checks);
    require_non_empty_string(metadata, "generated_at", "/metadata", checks);
    require_non_empty_string(metadata, "report_type", "/metadata", checks);
    if let Some(temporal) =
        require_object_for_validation(metadata, "temporal_review", "/metadata", checks)
    {
        require_temporal_marker_fields(temporal, "/metadata/temporal_review", checks);
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
        require_key(report, key, "/report", checks);
    }
    require_non_empty_string(report, "field", "/report", checks);

    if let Some(scope) = require_object_for_validation(report, "scope", "/report", checks) {
        require_non_empty_string(scope, "summary", "/report/scope", checks);
        require_array(scope, "included", "/report/scope", checks);
        require_array(scope, "excluded", "/report/scope", checks);
    }
    if let Some(domain) = require_object_for_validation(report, "domain_profile", "/report", checks)
    {
        require_non_empty_string(domain, "classification", "/report/domain_profile", checks);
        require_non_empty_string(domain, "rationale", "/report/domain_profile", checks);
    }
    if let Some(value) = report.get("presentation") {
        if let Value::Object(presentation) = value {
            validate_allowed_keys(
                presentation,
                "/report/presentation",
                &[
                    "thesis",
                    "organizing_form",
                    "rationale",
                    "alternatives_considered",
                    "sections",
                ],
                checks,
            );
            require_non_empty_string(presentation, "thesis", "/report/presentation", checks);
            require_non_empty_string(
                presentation,
                "organizing_form",
                "/report/presentation",
                checks,
            );
            require_non_empty_string(presentation, "rationale", "/report/presentation", checks);
            require_array(presentation, "sections", "/report/presentation", checks);
            if let Some(Value::Array(sections)) = presentation.get("sections") {
                if sections.is_empty() {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_SCHEMA_REQUIRED,
                            "/report/presentation/sections must contain at least one public narrative section",
                        )
                        .with_target("/report/presentation/sections", ""),
                    );
                }
                for (section_index, section) in sections.iter().enumerate() {
                    let path = format!("/report/presentation/sections/{section_index}");
                    let Some(section) = section.as_object() else {
                        checks.push(
                            DiagnosticCheck::error(
                                CHECK_VALIDATE_SCHEMA_REQUIRED,
                                format!("{path} must be an object"),
                            )
                            .with_target(path, ""),
                        );
                        continue;
                    };
                    validate_allowed_keys(
                        section,
                        &path,
                        &["id", "title", "purpose", "body_markdown", "visual_view_ids"],
                        checks,
                    );
                    require_non_empty_string(section, "id", &path, checks);
                    require_non_empty_string(section, "title", &path, checks);
                    if section.contains_key("visual_view_ids") {
                        require_array(section, "visual_view_ids", &path, checks);
                    }
                }
            }
        } else {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    "/report/presentation must be an object",
                )
                .with_target("/report/presentation", ""),
            );
        }
        match report.get("field_elements").and_then(Value::as_array) {
            Some(field_elements) if !field_elements.is_empty() => {}
            _ => checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_STRUCTURE_REQUIRED,
                    "a report with presentation must include a non-empty /report/field_elements array",
                )
                .with_target("/report/field_elements", ""),
            ),
        }
    }
    if let Some(standards) =
        require_object_for_validation(report, "evidence_standards", "/report", checks)
    {
        require_non_empty_string(standards, "summary", "/report/evidence_standards", checks);
        require_non_empty_string(
            standards,
            "claim_policy",
            "/report/evidence_standards",
            checks,
        );
        require_array(
            standards,
            "source_role_requirements",
            "/report/evidence_standards",
            checks,
        );
    }

    for key in [
        "core_ideas",
        "methods",
        "representations",
        "sources",
        "claims",
        "relations",
        "curriculum_path",
        "frontier_debates",
    ] {
        require_array(report, key, "/report", checks);
    }
    for key in [
        "literature_ladder",
        "field_elements",
        "visual_views",
        "structure_waivers",
    ] {
        if report.contains_key(key) {
            require_array(report, key, "/report", checks);
        }
    }

    validate_required_array_item_fields(
        report,
        "literature_ladder",
        &[
            "id",
            "layer",
            "start_here",
            "read_for",
            "do_not_infer",
            "source_ids",
        ],
        checks,
    );
    validate_required_array_item_fields(
        report,
        "field_elements",
        &[
            "id",
            "element_class",
            "label",
            "actual_form",
            "role",
            "load_bearing_relations",
        ],
        checks,
    );
    validate_required_array_item_fields(
        report,
        "sources",
        &[
            "id",
            "citation",
            "source_type",
            "roles",
            "access",
            "why_it_matters",
            "verification_status",
        ],
        checks,
    );
    validate_required_array_item_fields(
        report,
        "claims",
        &[
            "id",
            "statement",
            "claim_type",
            "evidence_requirement",
            "evidence_links",
            "temporal",
        ],
        checks,
    );
    validate_required_array_item_fields(report, "relations", &["id", "kind", "from", "to"], checks);
    validate_required_array_item_fields(
        report,
        "curriculum_path",
        &[
            "id",
            "sequence",
            "title",
            "learning_goal",
            "practice_artifact",
            "source_ids",
        ],
        checks,
    );
    validate_required_array_item_fields(
        report,
        "frontier_debates",
        &[
            "id",
            "kind",
            "title",
            "summary",
            "why_it_matters",
            "temporal",
        ],
        checks,
    );
    validate_required_array_item_fields(
        report,
        "structure_waivers",
        &["scope", "rationale"],
        checks,
    );
}

pub(crate) fn validate_allowed_keys(
    object: &serde_json::Map<String, Value>,
    path: &str,
    allowed: &[&str],
    checks: &mut Vec<DiagnosticCheck>,
) {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("{path} contains unsupported key {key:?}"),
                )
                .with_target(format!("{path}/{key}"), ""),
            );
        }
    }
}

pub(crate) fn require_object_for_validation<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) -> Option<&'a serde_json::Map<String, Value>> {
    match object.get(key) {
        Some(Value::Object(child)) => Some(child),
        Some(_) => {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("{path}/{key} must be an object"),
                )
                .with_target(format!("{path}/{key}"), ""),
            );
            None
        }
        None => {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("missing required object {path}/{key}"),
                )
                .with_target(format!("{path}/{key}"), ""),
            );
            None
        }
    }
}

pub(crate) fn require_key(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    if !object.contains_key(key) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_SCHEMA_REQUIRED,
                format!("missing required field {path}/{key}"),
            )
            .with_target(format!("{path}/{key}"), ""),
        );
    }
}

pub(crate) fn require_non_empty_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    match object.get(key) {
        Some(Value::String(value)) if !value.trim().is_empty() => {}
        Some(Value::String(_)) => checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_SCHEMA_REQUIRED,
                format!("{path}/{key} must be a non-empty string"),
            )
            .with_target(format!("{path}/{key}"), ""),
        ),
        Some(_) => checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_SCHEMA_REQUIRED,
                format!("{path}/{key} must be a string"),
            )
            .with_target(format!("{path}/{key}"), ""),
        ),
        None => require_key(object, key, path, checks),
    }
}

pub(crate) fn require_array(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    match object.get(key) {
        Some(Value::Array(_)) => {}
        Some(_) => checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_SCHEMA_REQUIRED,
                format!("{path}/{key} must be an array"),
            )
            .with_target(format!("{path}/{key}"), ""),
        ),
        None => require_key(object, key, path, checks),
    }
}

pub(crate) fn require_temporal_marker_fields(
    temporal: &serde_json::Map<String, Value>,
    path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    require_non_empty_string(temporal, "as_of", path, checks);
    require_non_empty_string(temporal, "temporal_status", path, checks);
}

pub(crate) fn validate_required_array_item_fields(
    report: &serde_json::Map<String, Value>,
    section: &str,
    fields: &[&str],
    checks: &mut Vec<DiagnosticCheck>,
) {
    let Some(Value::Array(items)) = report.get(section) else {
        return;
    };
    for (index, item) in items.iter().enumerate() {
        let path = format!("/report/{section}/{index}");
        let Some(object) = item.as_object() else {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("{path} must be an object"),
                )
                .with_target(path, ""),
            );
            continue;
        };
        for field in fields {
            require_key(object, field, &path, checks);
        }
    }
}

pub(crate) fn validate_known_enum_strings(value: &Value, checks: &mut Vec<DiagnosticCheck>) {
    if let Some(metadata) = value.get("metadata").and_then(Value::as_object) {
        validate_string_enum(
            metadata,
            "report_type",
            &["scaffold", "human_report"],
            CHECK_VALIDATE_SCHEMA_REQUIRED,
            "/metadata",
            checks,
        );
    }

    let Some(report) = value.get("report").and_then(Value::as_object) else {
        return;
    };

    if let Some(field_elements) = report.get("field_elements").and_then(Value::as_array) {
        for (index, element) in field_elements.iter().enumerate() {
            let Some(element) = element.as_object() else {
                continue;
            };
            if element.contains_key("confidence") {
                validate_string_enum(
                    element,
                    "confidence",
                    &["high", "medium", "low", "unknown"],
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    &format!("/report/field_elements/{index}"),
                    checks,
                );
            }
        }
    }

    if let Some(sources) = report.get("sources").and_then(Value::as_array) {
        for (index, source) in sources.iter().enumerate() {
            let Some(source) = source.as_object() else {
                continue;
            };
            if let Some(access) = source.get("access").and_then(Value::as_object) {
                validate_string_enum(
                    access,
                    "status",
                    &[
                        "open_access",
                        "free_web",
                        "public_domain",
                        "official_open",
                        "user_provided",
                        "library",
                        "paid_book",
                        "paywalled",
                        "subscription",
                        "restricted",
                        "unknown",
                    ],
                    CHECK_VALIDATE_SOURCE_ACCESS,
                    &format!("/report/sources/{index}/access"),
                    checks,
                );
            }
        }
    }

    if let Some(claims) = report.get("claims").and_then(Value::as_array) {
        for (claim_index, claim) in claims.iter().enumerate() {
            let Some(claim) = claim.as_object() else {
                continue;
            };
            validate_string_enum(
                claim,
                "claim_type",
                &[
                    "structural",
                    "currentness",
                    "frontier",
                    "debate",
                    "curricular",
                    "interpretive",
                    "methodological",
                ],
                CHECK_VALIDATE_EVIDENCE_REQUIRED,
                &format!("/report/claims/{claim_index}"),
                checks,
            );
            validate_string_enum(
                claim,
                "evidence_requirement",
                &[
                    "none",
                    "cataloged_source",
                    "reviewed_source",
                    "verified_source",
                    "multiple_reviewed_sources",
                ],
                CHECK_VALIDATE_EVIDENCE_REQUIRED,
                &format!("/report/claims/{claim_index}"),
                checks,
            );
            if let Some(links) = claim.get("evidence_links").and_then(Value::as_array) {
                for (link_index, link) in links.iter().enumerate() {
                    let Some(link) = link.as_object() else {
                        continue;
                    };
                    validate_string_enum(
                        link,
                        "verification_status",
                        &["cataloged", "reviewed", "verified"],
                        CHECK_VALIDATE_EVIDENCE_SUPPORT,
                        &format!("/report/claims/{claim_index}/evidence_links/{link_index}"),
                        checks,
                    );
                }
            }
        }
    }

    if let Some(relations) = report.get("relations").and_then(Value::as_array) {
        for (index, relation) in relations.iter().enumerate() {
            let Some(relation) = relation.as_object() else {
                continue;
            };
            validate_string_enum(
                relation,
                "kind",
                &[
                    "depends_on",
                    "supports",
                    "qualifies",
                    "contradicts",
                    "precedes",
                    "introduces",
                    "uses_method",
                    "represented_by",
                    "grounds",
                    "motivates",
                    "part_of",
                    "maps_to",
                ],
                CHECK_VALIDATE_RELATION_KIND,
                &format!("/report/relations/{index}"),
                checks,
            );
            for endpoint_key in ["from", "to"] {
                if let Some(endpoint) = relation.get(endpoint_key).and_then(Value::as_object) {
                    validate_string_enum(
                        endpoint,
                        "entity_type",
                        &[
                            "concept",
                            "field_element",
                            "claim",
                            "source",
                            "curriculum_step",
                            "frontier_debate",
                            "method",
                            "representation",
                        ],
                        CHECK_VALIDATE_RELATION_ENDPOINT,
                        &format!("/report/relations/{index}/{endpoint_key}"),
                        checks,
                    );
                }
            }
        }
    }

    if let Some(waivers) = report.get("structure_waivers").and_then(Value::as_array) {
        for (index, waiver) in waivers.iter().enumerate() {
            let Some(waiver) = waiver.as_object() else {
                continue;
            };
            validate_string_enum(
                waiver,
                "scope",
                &["relations", "curriculum_prerequisites", "visual_views"],
                CHECK_VALIDATE_SCHEMA_REQUIRED,
                &format!("/report/structure_waivers/{index}"),
                checks,
            );
        }
    }
}

pub(crate) fn validate_string_enum(
    object: &serde_json::Map<String, Value>,
    key: &str,
    allowed: &[&str],
    check_id: &str,
    path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    let Some(Value::String(value)) = object.get(key) else {
        return;
    };
    if !allowed.contains(&value.as_str()) {
        checks.push(
            DiagnosticCheck::error(
                check_id,
                format!("{path}/{key} has unknown value {value:?}"),
            )
            .with_target(format!("{path}/{key}"), ""),
        );
    }
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
                            format!("human_report public payload contains non-public placeholder or internal text {phrase:?}"),
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

pub(crate) fn validate_claim_evidence_requirements(
    report: &PublicReport,
    index: &ReportIdIndex,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for (claim_index, claim) in report.claims.iter().enumerate() {
        let path = format!("/report/claims/{claim_index}");
        let mut usable_reviewed_sources = BTreeSet::new();
        let mut usable_verified_sources = BTreeSet::new();
        let mut saw_cataloged = false;
        let mut saw_qualifying_without_support = false;
        for (link_index, link) in claim.evidence_links.iter().enumerate() {
            let link_path = format!("{path}/evidence_links/{link_index}");
            if !index.sources.contains(&link.source_id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_EVIDENCE_SOURCE,
                        format!(
                            "claim {} evidence link references missing source {}",
                            claim.id, link.source_id
                        ),
                    )
                    .with_target(&link_path, &claim.id),
                );
            }
            if link.verification_status == VerificationStatus::Cataloged {
                saw_cataloged = true;
                continue;
            }
            validate_reviewed_evidence_link_metadata(claim, link, &link_path, checks);

            match link.support_kind {
                SupportKind::Supports => {}
                SupportKind::Qualifies => {
                    saw_qualifying_without_support = true;
                    continue;
                }
                SupportKind::Contradicts => {
                    checks.push(
                        DiagnosticCheck::warning(
                            CHECK_VALIDATE_EVIDENCE_SUPPORT,
                            format!(
                                "claim {} has contradictory evidence from {}; record a conflict or lower-confidence interpretation",
                                claim.id, link.source_id
                            ),
                        )
                        .with_target(&link_path, &claim.id),
                    );
                    continue;
                }
                SupportKind::Background | SupportKind::Example => {
                    continue;
                }
            }

            if !reviewed_link_can_support_claim(link) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_EVIDENCE_SUPPORT,
                        format!(
                            "claim {} evidence link {} cannot satisfy claim support without reviewed support metadata",
                            claim.id, link.source_id
                        ),
                    )
                    .with_target(&link_path, &claim.id),
                );
                continue;
            }
            match link.verification_status {
                VerificationStatus::Reviewed => {
                    usable_reviewed_sources.insert(link.source_id.clone());
                }
                VerificationStatus::Verified => {
                    usable_reviewed_sources.insert(link.source_id.clone());
                    usable_verified_sources.insert(link.source_id.clone());
                }
                VerificationStatus::Cataloged => {}
            }
        }

        if !claim_requires_reviewed_or_verified_evidence(claim) {
            continue;
        }

        let satisfied = match claim.evidence_requirement {
            EvidenceRequirement::VerifiedSource => !usable_verified_sources.is_empty(),
            EvidenceRequirement::MultipleReviewedSources => usable_reviewed_sources.len() >= 2,
            EvidenceRequirement::None
            | EvidenceRequirement::CatalogedSource
            | EvidenceRequirement::ReviewedSource => !usable_reviewed_sources.is_empty(),
        };
        if !satisfied {
            if saw_cataloged {
                checks.push(
                    DiagnosticCheck::warning(
                        CHECK_EVIDENCE_CATALOGED_ONLY,
                        format!(
                            "claim {} has cataloged evidence links, but cataloged evidence cannot satisfy reviewed support",
                            claim.id
                        ),
                    )
                    .with_target(&path, &claim.id),
                );
            }
            if saw_qualifying_without_support {
                checks.push(
                    DiagnosticCheck::warning(
                        CHECK_VALIDATE_EVIDENCE_SUPPORT,
                        format!(
                            "claim {} has qualifying evidence but no affirmative supporting evidence",
                            claim.id
                        ),
                    )
                    .with_target(&path, &claim.id),
                );
            }
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_EVIDENCE_REQUIRED,
                    format!(
                        "claim {} requires reviewed or verified evidence with a locator or support_note based on claim_type {:?} and evidence_requirement {:?}",
                        claim.id, claim.claim_type, claim.evidence_requirement
                    ),
                )
                .with_target(&path, &claim.id),
            );
        }
    }
}

pub(crate) fn validate_reviewed_evidence_link_metadata(
    claim: &Claim,
    link: &EvidenceLink,
    link_path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    if !matches!(
        link.verification_status,
        VerificationStatus::Reviewed | VerificationStatus::Verified
    ) {
        return;
    }
    if link.reviewed_at.trim().is_empty() {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_EVIDENCE_SUPPORT,
                format!(
                    "claim {} evidence link {} needs reviewed_at",
                    claim.id, link.source_id
                ),
            )
            .with_target(link_path, &claim.id),
        );
    } else if !looks_like_iso_date(&link.reviewed_at) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_EVIDENCE_SUPPORT,
                format!(
                    "claim {} evidence link {} reviewed_at must be YYYY-MM-DD",
                    claim.id, link.source_id
                ),
            )
            .with_target(link_path, &claim.id),
        );
    }
    if link.locator.trim().is_empty() && link.support_note.trim().is_empty() {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_EVIDENCE_SUPPORT,
                format!(
                    "claim {} evidence link {} needs a locator or support_note",
                    claim.id, link.source_id
                ),
            )
            .with_target(link_path, &claim.id),
        );
    }
}

pub(crate) fn reviewed_link_can_support_claim(link: &EvidenceLink) -> bool {
    matches!(
        link.verification_status,
        VerificationStatus::Reviewed | VerificationStatus::Verified
    ) && link.support_kind == SupportKind::Supports
        && looks_like_iso_date(&link.reviewed_at)
        && (!link.locator.trim().is_empty() || !link.support_note.trim().is_empty())
}

pub(crate) fn claim_requires_reviewed_or_verified_evidence(claim: &Claim) -> bool {
    claim.evidence_requirement != EvidenceRequirement::None
        || matches!(
            claim.claim_type,
            ClaimType::Currentness | ClaimType::Frontier | ClaimType::Debate
        )
}

pub(crate) fn validate_source_role_coverage(
    report: &PublicReport,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for (requirement_index, requirement) in report
        .evidence_standards
        .source_role_requirements
        .iter()
        .enumerate()
    {
        let path =
            format!("/report/evidence_standards/source_role_requirements/{requirement_index}");
        let minimum = requirement
            .minimum_sources
            .unwrap_or(match requirement.requirement {
                SourceRoleRequirementKind::Required | SourceRoleRequirementKind::Conditional => 1,
                SourceRoleRequirementKind::Waived | SourceRoleRequirementKind::NotApplicable => 0,
            }) as usize;
        let count = report
            .sources
            .iter()
            .filter(|source| source.roles.contains(&requirement.role))
            .count();
        if requirement.rationale.trim().is_empty() {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!(
                        "source role {} needs a recorded rationale",
                        source_role_label(requirement.role)
                    ),
                )
                .with_target(&path, ""),
            );
        }

        match requirement.requirement {
            SourceRoleRequirementKind::Required if count < minimum => {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_SOURCE_ROLE_REQUIRED,
                        format!(
                            "required source role {} has {count} source(s), minimum is {minimum}",
                            source_role_label(requirement.role)
                        ),
                    )
                    .with_target(&path, ""),
                );
            }
            SourceRoleRequirementKind::Conditional if count < minimum => {
                if !has_usable_waiver(&requirement.waiver) {
                    checks.push(
                        DiagnosticCheck::warning(
                            CHECK_VALIDATE_SOURCE_ROLE_CONDITIONAL,
                            format!(
                                "conditional source role {} has {count} source(s), minimum is {minimum}; record a waiver rationale if the condition is not active",
                                source_role_label(requirement.role)
                            ),
                        )
                        .with_target(&path, ""),
                    );
                }
            }
            SourceRoleRequirementKind::Waived => {
                if !has_usable_waiver(&requirement.waiver) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_SOURCE_ROLE_WAIVER,
                            format!(
                                "waived source role {} needs waiver rationale and as_of",
                                source_role_label(requirement.role)
                            ),
                        )
                        .with_target(&path, ""),
                    );
                }
            }
            SourceRoleRequirementKind::NotApplicable => {
                if let Some(waiver) = &requirement.waiver {
                    if waiver.rationale.trim().is_empty() {
                        checks.push(
                            DiagnosticCheck::warning(
                                CHECK_VALIDATE_SOURCE_ROLE_WAIVER,
                                format!(
                                    "not_applicable source role {} has an empty waiver rationale",
                                    source_role_label(requirement.role)
                                ),
                            )
                            .with_target(&path, ""),
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn has_usable_waiver(waiver: &Option<SourceRoleWaiver>) -> bool {
    waiver
        .as_ref()
        .map(|waiver| !waiver.rationale.trim().is_empty() && !waiver.as_of.trim().is_empty())
        .unwrap_or(false)
}

pub(crate) fn validate_currentness(
    document: &ReportDocument,
    index: &ReportIdIndex,
    checks: &mut Vec<DiagnosticCheck>,
) {
    validate_temporal_marker(
        &document.metadata.temporal_review,
        "/metadata/temporal_review",
        "",
        document.metadata.report_type == ReportType::HumanReport,
        checks,
    );

    let sources_by_id = document
        .report
        .sources
        .iter()
        .map(|source| (source.id.as_str(), source))
        .collect::<BTreeMap<_, _>>();

    for (claim_index, claim) in document.report.claims.iter().enumerate() {
        let path = format!("/report/claims/{claim_index}/temporal");
        let needs_current_metadata = claim_needs_currentness_metadata(claim);
        validate_temporal_marker(
            &claim.temporal,
            &path,
            &claim.id,
            needs_current_metadata,
            checks,
        );
        if has_currentness_words(&format!("{} {}", claim.statement, claim.notes))
            && structured_temporal_metadata_absent(&claim.temporal)
        {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_VALIDATE_CURRENTNESS_PROSE,
                    format!(
                        "claim {} uses currentness prose but lacks structured temporal metadata",
                        claim.id
                    ),
                )
                .with_target(format!("/report/claims/{claim_index}/statement"), &claim.id),
            );
        }
        if needs_current_metadata {
            validate_claim_source_dates(claim, &sources_by_id, index, claim_index, checks);
        }
    }

    for (item_index, item) in document.report.frontier_debates.iter().enumerate() {
        validate_temporal_marker(
            &item.temporal,
            &format!("/report/frontier_debates/{item_index}/temporal"),
            &item.id,
            true,
            checks,
        );
        if has_currentness_words(&format!("{} {}", item.title, item.summary))
            && structured_temporal_metadata_absent(&item.temporal)
        {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_VALIDATE_CURRENTNESS_PROSE,
                    format!(
                        "frontier/debate item {} uses currentness prose but lacks structured temporal metadata",
                        item.id
                    ),
                )
                .with_target(
                    format!("/report/frontier_debates/{item_index}/summary"),
                    &item.id,
                ),
            );
        }
    }
}

pub(crate) fn validate_temporal_marker(
    marker: &TemporalMarker,
    path: &str,
    entity_id: &str,
    require_review_after: bool,
    checks: &mut Vec<DiagnosticCheck>,
) {
    if marker.as_of.trim().is_empty() {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_CURRENTNESS_METADATA,
                format!("{path}/as_of is required for deterministic currentness"),
            )
            .with_target(format!("{path}/as_of"), entity_id),
        );
    } else if !looks_like_iso_date(&marker.as_of) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_CURRENTNESS_METADATA,
                format!("{path}/as_of must be YYYY-MM-DD"),
            )
            .with_target(format!("{path}/as_of"), entity_id),
        );
    }

    if require_review_after && marker.review_after.trim().is_empty() {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_CURRENTNESS_METADATA,
                format!("{path}/review_after is required for currentness/frontier/debate claims"),
            )
            .with_target(format!("{path}/review_after"), entity_id),
        );
    }
    if !marker.review_after.trim().is_empty() && !looks_like_iso_date(&marker.review_after) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_CURRENTNESS_METADATA,
                format!("{path}/review_after must be YYYY-MM-DD"),
            )
            .with_target(format!("{path}/review_after"), entity_id),
        );
    }

    if !marker.as_of.trim().is_empty()
        && !marker.review_after.trim().is_empty()
        && looks_like_iso_date(&marker.as_of)
        && looks_like_iso_date(&marker.review_after)
        && marker.review_after.as_str() <= marker.as_of.as_str()
    {
        let severity = if marker.temporal_status == TemporalStatus::Current {
            DiagnosticSeverity::Error
        } else {
            DiagnosticSeverity::Warning
        };
        checks.push(
            DiagnosticCheck::new(
                CHECK_VALIDATE_CURRENTNESS_REVIEW_DUE,
                severity,
                format!(
                    "{path}/review_after {} is not after as_of {}",
                    marker.review_after, marker.as_of
                ),
            )
            .with_target(format!("{path}/review_after"), entity_id),
        );
    }

    match marker.temporal_status {
        TemporalStatus::Stale => checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_CURRENTNESS_REVIEW_DUE,
                format!("{path}/temporal_status is stale"),
            )
            .with_target(format!("{path}/temporal_status"), entity_id),
        ),
        TemporalStatus::ReviewDue => checks.push(
            DiagnosticCheck::warning(
                CHECK_VALIDATE_CURRENTNESS_REVIEW_DUE,
                format!("{path}/temporal_status is review_due"),
            )
            .with_target(format!("{path}/temporal_status"), entity_id),
        ),
        TemporalStatus::Unknown if require_review_after => checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_CURRENTNESS_METADATA,
                format!("{path}/temporal_status cannot be unknown for currentness/frontier/debate claims"),
            )
            .with_target(format!("{path}/temporal_status"), entity_id),
        ),
        _ => {}
    }
}

pub(crate) fn claim_needs_currentness_metadata(claim: &Claim) -> bool {
    matches!(
        claim.claim_type,
        ClaimType::Currentness | ClaimType::Frontier | ClaimType::Debate
    ) || matches!(
        claim.temporal.temporal_status,
        TemporalStatus::Current | TemporalStatus::ReviewDue | TemporalStatus::Stale
    )
}

pub(crate) fn validate_claim_source_dates(
    claim: &Claim,
    sources_by_id: &BTreeMap<&str, &ReportSource>,
    index: &ReportIdIndex,
    claim_index: usize,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for link in &claim.evidence_links {
        if !index.sources.contains(&link.source_id) {
            continue;
        }
        let Some(source) = sources_by_id.get(link.source_id.as_str()) else {
            continue;
        };
        if source.date.trim().is_empty() {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_VALIDATE_CURRENTNESS_SOURCE_DATE,
                    format!(
                        "currentness claim {} uses source {} with no source date",
                        claim.id, source.id
                    ),
                )
                .with_target(
                    format!("/report/claims/{claim_index}/evidence_links"),
                    &claim.id,
                ),
            );
        } else if source_date_after_as_of(&source.date, &claim.temporal.as_of) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_CURRENTNESS_SOURCE_DATE,
                    format!(
                        "claim {} has as_of {} before source {} date {}",
                        claim.id, claim.temporal.as_of, source.id, source.date
                    ),
                )
                .with_target(
                    format!("/report/claims/{claim_index}/evidence_links"),
                    &claim.id,
                ),
            );
        }
    }
}
pub(crate) fn validate_source_access_metadata(
    report: &PublicReport,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for (source_index, source) in report.sources.iter().enumerate() {
        let path = format!("/report/sources/{source_index}/access");
        if source.access.status == AccessStatus::Unknown {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SOURCE_ACCESS,
                    format!("source {} has unknown access status", source.id),
                )
                .with_target(format!("{path}/status"), &source.id),
            );
        }
        if source.access.route.trim().is_empty()
            || source.access.route.trim().eq_ignore_ascii_case("unknown")
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SOURCE_ACCESS,
                    format!("source {} has no actionable access route", source.id),
                )
                .with_target(format!("{path}/route"), &source.id),
            );
        }
        if source_access_requires_notes(source.access.status)
            && source.access.budget_estimate.trim().is_empty()
            && source.access.license.trim().is_empty()
            && source.access.notes.trim().is_empty()
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SOURCE_ACCESS,
                    format!(
                        "source {} needs budget, license, or access notes for {} access",
                        source.id,
                        access_status_label(source.access.status)
                    ),
                )
                .with_target(path.clone(), &source.id),
            );
        }
        if source_access_requires_metadata_only(source.access.status)
            && source.access.metadata_only != Some(true)
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SOURCE_ACCESS,
                    format!(
                        "source {} with {} access must be marked metadata_only",
                        source.id,
                        access_status_label(source.access.status)
                    ),
                )
                .with_target(format!("{path}/metadata_only"), &source.id),
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
                    "sok-report/v2 human_report needs a non-empty presentation.alternatives_considered decision trace",
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

pub(crate) fn is_stable_id(id: &str) -> bool {
    id.contains('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .chars()
            .next()
            .map(|first| first.is_ascii_lowercase())
            .unwrap_or(false)
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
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

pub(crate) fn sort_diagnostics(checks: &mut [DiagnosticCheck]) {
    checks.sort_by(|left, right| {
        severity_sort_key(left.severity)
            .cmp(&severity_sort_key(right.severity))
            .then_with(|| left.check_id.cmp(&right.check_id))
            .then_with(|| left.target_path.cmp(&right.target_path))
            .then_with(|| left.entity_id.cmp(&right.entity_id))
            .then_with(|| left.message.cmp(&right.message))
    });
}

pub(crate) fn severity_sort_key(severity: DiagnosticSeverity) -> u8 {
    match severity {
        DiagnosticSeverity::Error => 0,
        DiagnosticSeverity::Warning => 1,
        DiagnosticSeverity::Info => 2,
    }
}
