use super::*;

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
                    format!(
                        "unexpected top-level key {key:?}; expected metadata, report, \
                        internal_context, or diagnostics"
                    ),
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
    if let Some(value) = root.get("internal_context") {
        if let Some(internal_context) = value.as_object() {
            validate_internal_context_fields(internal_context, checks);
        } else {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    "/internal_context must be an object",
                )
                .with_target("/internal_context", ""),
            );
        }
    }

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
    if let Some(schema_version) = metadata.get("schema_version").and_then(Value::as_str) {
        if !matches!(schema_version, "sok-report/v1" | "sok-report/v2") {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!(
                        "/metadata/schema_version {:?} is unsupported; expected sok-report/v1 or sok-report/v2",
                        schema_version
                    ),
                )
                .with_target("/metadata/schema_version", ""),
            );
        }
    }
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

pub(crate) fn validate_internal_context_fields(
    internal_context: &serde_json::Map<String, Value>,
    checks: &mut Vec<DiagnosticCheck>,
) {
    validate_allowed_keys(
        internal_context,
        "/internal_context",
        &[
            "raw_learner_profile",
            "original_goal",
            "prompt_derived_assumptions",
            "placeholder_state",
            "profile_proposals",
            "handoff_notes",
        ],
        checks,
    );

    if internal_context.contains_key("profile_proposals") {
        require_array(
            internal_context,
            "profile_proposals",
            "/internal_context",
            checks,
        );
    }
    let Some(profile_proposals) = internal_context
        .get("profile_proposals")
        .and_then(Value::as_array)
    else {
        return;
    };

    for (index, proposal) in profile_proposals.iter().enumerate() {
        let path = format!("/internal_context/profile_proposals/{index}");
        let Some(proposal) = proposal.as_object() else {
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
            proposal,
            &path,
            &[
                "proposer_type",
                "profile_id",
                "profile_version",
                "locale",
                "confidence",
                "rationale",
            ],
            checks,
        );
        for key in [
            "proposer_type",
            "profile_id",
            "profile_version",
            "locale",
            "confidence",
            "rationale",
        ] {
            require_non_empty_string(proposal, key, &path, checks);
        }
        validate_string_enum(
            proposal,
            "proposer_type",
            &["user", "agent", "inference", "unknown"],
            CHECK_VALIDATE_SCHEMA_REQUIRED,
            &path,
            checks,
        );
    }
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
                    "requires_before",
                    "introduced_by",
                    "revisits",
                    "deepens",
                    "applies",
                    "assessed_by",
                    "remediates",
                    "depends_on",
                    "requires",
                    "prerequisite",
                    "prerequisite_for",
                    "precedes",
                    "introduces",
                    "uses_method",
                    "represented_by",
                    "grounds",
                    "motivates",
                    "part_of",
                    "maps_to",
                    "supports",
                    "qualifies",
                    "contradicts",
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
