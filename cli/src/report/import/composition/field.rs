use super::*;

pub(crate) fn build_scope(
    parsed: &ParsedMarkdownReport,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Scope {
    let mut included = split_listish(&first_non_empty([
        parsed
            .research_frame
            .get("Scope included")
            .map_or("", String::as_str),
        parsed
            .research_frame
            .get("Included")
            .map_or("", String::as_str),
    ]));
    let excluded = split_listish(&first_non_empty([
        parsed
            .research_frame
            .get("Scope excluded")
            .map_or("", String::as_str),
        parsed
            .research_frame
            .get("Excluded")
            .map_or("", String::as_str),
    ]));
    if included.is_empty() {
        included.push(infer_report_field(parsed));
    }

    let architecture_thesis =
        architecture_value(&parsed.report_architecture, &["Executive thesis", "Thesis"]);
    let first_narrative = parsed
        .narrative_sections
        .first()
        .map(|section| public_section_text(&section.body_markdown))
        .unwrap_or_default();
    let summary = first_non_empty([
        parsed.domain_decomposition.as_str(),
        parsed.orientation.as_str(),
        architecture_thesis.as_str(),
        first_narrative.as_str(),
        "Bounded SoK export; public scope could not be inferred from the report narrative.",
    ]);
    if parsed.domain_decomposition.is_empty()
        && parsed.orientation.is_empty()
        && architecture_thesis.is_empty()
        && first_narrative.is_empty()
    {
        diagnostics.push(DiagnosticCheck::warning(
            CHECK_EXPORT_MISSING_PUBLIC_FIELD,
            "could not infer a public scope summary from report architecture or narrative",
        ));
    }

    let mut interpretive_notes = Vec::new();
    if !parsed.orientation.is_empty() && parsed.orientation != summary {
        interpretive_notes.push(parsed.orientation.clone());
    }

    Scope {
        summary,
        included,
        excluded,
        assumptions: Vec::new(),
        interpretive_notes,
    }
}

pub(crate) fn build_domain_profile(
    parsed: &ParsedMarkdownReport,
    stage: ExportStage,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> DomainProfile {
    let declared_profile = architecture_value(
        &parsed.report_architecture,
        &["Domain profile", "Domain classification"],
    );
    let raw = match stage {
        ExportStage::Scaffold => first_non_empty([
            parsed
                .research_frame
                .get("Domain classification")
                .map_or("", String::as_str),
            parsed
                .research_frame
                .get("Domain type")
                .map_or("", String::as_str),
            parsed
                .research_frame
                .get("Provisional lens")
                .map_or("", String::as_str),
            parsed.domain_decomposition.as_str(),
        ]),
        ExportStage::Final => first_non_empty([
            declared_profile.as_str(),
            parsed.domain_decomposition.as_str(),
        ]),
    };
    if raw.is_empty() && parsed.narrative_sections.is_empty() {
        diagnostics.push(DiagnosticCheck::warning(
            CHECK_EXPORT_MISSING_PUBLIC_FIELD,
            "could not infer domain classification; using mixed",
        ));
    }
    let classifications = domain_classifications_from_text(&raw);
    let classification = classifications
        .first()
        .copied()
        .unwrap_or(DomainClassification::Mixed);
    let secondary_characteristics = classifications
        .into_iter()
        .filter(|candidate| *candidate != classification)
        .collect();
    let architecture_rationale = architecture_value(
        &parsed.report_architecture,
        &["Architecture rationale", "Rationale", "Why this form"],
    );
    let rationale = first_non_empty([
        parsed.domain_decomposition.as_str(),
        raw.as_str(),
        architecture_rationale.as_str(),
        "No single domain label was allowed to determine the report architecture.",
    ]);
    let failure_modes = parsed
        .deep_structure_rows
        .iter()
        .find(|row| {
            normalize_id_text(&lookup_cell_any(
                row,
                &["Observed element", "Element", "Element class"],
            ))
            .contains("failure mode")
        })
        .map(|row| {
            split_listish(&first_non_empty([
                lookup_cell_any(row, &["SoK extraction"]).as_str(),
                lookup_cell_any(row, &["In this field"]).as_str(),
                lookup_cell_any(row, &["Actual form in this field"]).as_str(),
                lookup_cell_any(row, &["Why it matters"]).as_str(),
            ]))
        })
        .unwrap_or_default();

    DomainProfile {
        classification,
        secondary_characteristics,
        rationale,
        failure_modes,
    }
}

pub(crate) fn domain_classifications_from_text(raw: &str) -> Vec<DomainClassification> {
    let mut out = Vec::new();
    for classification_id in profiles::domain_classification_ids_from_text(raw) {
        if let Some(classification) = domain_classification_from_id(&classification_id) {
            out.push(classification);
        }
    }
    out
}

pub(crate) fn domain_classification_from_id(raw: &str) -> Option<DomainClassification> {
    match raw {
        "formal" => Some(DomainClassification::Formal),
        "well_structured" => Some(DomainClassification::WellStructured),
        "ill_structured" => Some(DomainClassification::IllStructured),
        "professional_practice" => Some(DomainClassification::ProfessionalPractice),
        "instrument_bound" => Some(DomainClassification::InstrumentBound),
        "infrastructure_bound" => Some(DomainClassification::InfrastructureBound),
        "emerging" => Some(DomainClassification::Emerging),
        "interdisciplinary" => Some(DomainClassification::Interdisciplinary),
        "mixed" => Some(DomainClassification::Mixed),
        _ => None,
    }
}

pub(crate) fn build_field_elements(
    parsed: &ParsedMarkdownReport,
    source_lookup: &SourceLookup,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<FieldElement> {
    let mut elements = Vec::new();
    for (index, row) in parsed.deep_structure_rows.iter().enumerate() {
        if !is_usable_deep_structure_row(row) {
            continue;
        }
        let label = lookup_cell_any(row, &["Observed element", "Element", "Name", "Title"]);
        let explicit_id =
            clean_inline_markdown(&lookup_cell_any(row, &["Element ID", "Element Id", "ID"]));
        let id = if explicit_id.trim().is_empty() {
            content_id("element", &[&label])
        } else {
            explicit_id
        };
        let fallback_actual_form = deep_structure_description(row);
        let actual_form = first_non_empty([
            lookup_cell_any(
                row,
                &[
                    "Actual form in this field",
                    "In this field",
                    "SoK extraction",
                    "First-pass scholarly representation",
                    "Research-grade representation",
                    "Why it matters",
                ],
            )
            .as_str(),
            fallback_actual_form.as_str(),
        ]);
        let source_refs =
            lookup_cell_any(row, &["Source IDs", "Sources", "Key sources", "Readings"]);
        elements.push(FieldElement {
            id: id.clone(),
            element_class: lookup_cell_any(row, &["Element class", "Class", "Element type"]),
            label,
            actual_form,
            role: lookup_cell_any(
                row,
                &[
                    "Role",
                    "Role: core / surrounding / context",
                    "Structural role",
                ],
            ),
            relation_ids: Vec::new(),
            load_bearing_relations: lookup_cell_any(
                row,
                &[
                    "Load-bearing relations",
                    "Load bearing relations",
                    "Key relations",
                    "Relations",
                ],
            ),
            source_ids: source_lookup.resolve_refs_with_diagnostics(
                &source_refs,
                format!("/report/field_elements/{index}/source_ids"),
                &id,
                diagnostics,
            ),
            confidence: parse_claim_confidence(&lookup_cell_any(row, &["Confidence"])),
        });
    }
    elements
}

pub(crate) fn build_knowledge_items(
    field_elements: &[FieldElement],
    parsed: &ParsedMarkdownReport,
) -> (Vec<KnowledgeItem>, Vec<KnowledgeItem>, Vec<KnowledgeItem>) {
    let mut core_ideas = Vec::new();
    let mut methods = Vec::new();
    let mut representations = Vec::new();
    for element in field_elements {
        let element_class =
            first_non_empty([element.element_class.as_str(), element.label.as_str()]);
        let projection = profiles::projection_for_element_class(&element_class);
        let description = collapse_whitespace(
            &[
                element.actual_form.as_str(),
                element.role.as_str(),
                element.load_bearing_relations.as_str(),
            ]
            .into_iter()
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>()
            .join("; "),
        );
        let item = KnowledgeItem {
            id: content_id(&projection.item_prefix, &[&element.label]),
            label: element.label.clone(),
            description,
            source_ids: element.source_ids.clone(),
            ..KnowledgeItem::default()
        };
        match projection.role {
            ProjectionRole::Representation => representations.push(item),
            ProjectionRole::Method => methods.push(item),
            ProjectionRole::Omit => continue,
            ProjectionRole::CoreIdea => core_ideas.push(item),
        }
    }

    if core_ideas.is_empty() && !parsed.orientation.is_empty() {
        core_ideas.push(KnowledgeItem {
            id: content_id("concept", &[&parsed.orientation]),
            label: "Orientation thesis".to_string(),
            description: parsed.orientation.clone(),
            ..KnowledgeItem::default()
        });
    }
    if methods.is_empty() && !parsed.practice_text.is_empty() {
        methods.push(KnowledgeItem {
            id: content_id("method", &[&parsed.practice_text]),
            label: "Practice and assessment method".to_string(),
            description: parsed.practice_text.clone(),
            ..KnowledgeItem::default()
        });
    }
    (core_ideas, methods, representations)
}

pub(crate) fn deep_structure_description(row: &BTreeMap<String, String>) -> String {
    let mut parts = Vec::new();
    for key in [
        "sok extraction",
        "so k extraction",
        "in this field",
        "actual form in this field",
        "role",
        "role core surrounding context",
        "load bearing relations",
        "first pass scholarly representation",
        "research grade representation",
        "why it matters",
    ] {
        if let Some(value) = row.get(key) {
            if !value.trim().is_empty() {
                parts.push(value.trim().to_string());
            }
        }
    }
    collapse_whitespace(&parts.join("; "))
}

pub(crate) fn is_usable_deep_structure_row(row: &BTreeMap<String, String>) -> bool {
    let observed_element = lookup_cell_any(row, &["Observed element", "Element", "Name", "Title"]);
    let description = deep_structure_description(row);
    !observed_element.is_empty() && !description.is_empty() && !is_placeholder_text(&description)
}

pub(crate) fn build_evidence_standards(parsed: &ParsedMarkdownReport) -> EvidenceStandards {
    let mut requirements = Vec::new();
    for row in &parsed.source_role_rows {
        let role_text = lookup_cell_any(row, &["Source role", "Role"]);
        if role_text.trim().is_empty() {
            continue;
        }
        let requirement_text = lookup_cell_any(row, &["Status", "Requirement"]);
        let requirement = parse_source_requirement(&requirement_text);
        let rationale = first_non_empty([
            lookup_cell_any(row, &["What it tests"]).as_str(),
            lookup_cell_any(row, &["Candidate source pattern"]).as_str(),
            lookup_cell_any(row, &["Waiver or revision rule"]).as_str(),
            "Requirement imported from Source Role Probe.",
        ]);
        let waiver = if requirement == SourceRoleRequirementKind::Waived {
            Some(SourceRoleWaiver {
                rationale: first_non_empty([
                    lookup_cell_any(row, &["Waiver or revision rule"]).as_str(),
                    rationale.as_str(),
                ]),
                as_of: Utc::now()
                    .to_rfc3339_opts(SecondsFormat::Secs, true)
                    .split('T')
                    .next()
                    .unwrap_or("1970-01-01")
                    .to_string(),
                review_after: String::new(),
            })
        } else {
            None
        };
        requirements.push(SourceRoleRequirement {
            role: parse_source_role(&role_text),
            requirement,
            minimum_sources: match requirement {
                SourceRoleRequirementKind::Required | SourceRoleRequirementKind::Conditional => {
                    Some(1)
                }
                SourceRoleRequirementKind::Waived | SourceRoleRequirementKind::NotApplicable => {
                    Some(0)
                }
            },
            rationale,
            waiver,
        });
    }

    EvidenceStandards {
        summary:
            "Sources are cataloged from the manifest; claim support requires reviewed or verified \
            evidence links."
                .to_string(),
        claim_policy:
            "Cataloged source records remain visible as source records but do not satisfy claim \
            evidence requirements."
                .to_string(),
        source_role_requirements: requirements,
    }
}

pub(crate) fn build_literature_ladder(
    parsed: &ParsedMarkdownReport,
    source_lookup: &SourceLookup,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<LiteratureLadderRow> {
    let mut rows = Vec::new();
    for (index, row) in parsed.literature_ladder_rows.iter().enumerate() {
        let layer = lookup_cell_any(row, &["Layer", "Level"]);
        let start_here = lookup_cell_any(row, &["Start here", "Start", "Entry point"]);
        let read_for = lookup_cell_any(row, &["Read for", "Read for / use for", "Use for"]);
        let do_not_infer = lookup_cell_any(row, &["Do not infer", "Do-not-infer", "Caveat"]);
        let missing = [
            ("Layer", layer.as_str()),
            ("Start here", start_here.as_str()),
            ("Read for", read_for.as_str()),
            ("Do not infer", do_not_infer.as_str()),
        ]
        .into_iter()
        .filter_map(|(field, value)| value.trim().is_empty().then_some(field))
        .collect::<Vec<_>>();
        if !missing.is_empty() {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_MISSING_PUBLIC_FIELD,
                    format!(
                        "Literature Ladder row {} is missing required field(s): {}",
                        index + 1,
                        missing.join(", ")
                    ),
                )
                .with_target(format!("/report/literature_ladder/{index}"), ""),
            );
            continue;
        }

        let explicit_id = lookup_cell_any(row, &["ID", "Row ID", "Ladder ID"]);
        let id = if explicit_id.trim().is_empty() {
            content_id("ladder", &[&layer, &start_here])
        } else if is_stable_id(&explicit_id) {
            explicit_id
        } else {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_MISSING_PUBLIC_FIELD,
                    format!(
                        "Literature Ladder row {} has non-stable id {:?}; generated a stable id",
                        index + 1,
                        explicit_id
                    ),
                )
                .with_target(
                    format!("/report/literature_ladder/{index}/id"),
                    &explicit_id,
                ),
            );
            content_id("ladder", &[&layer, &start_here])
        };

        let source_refs = first_non_empty([
            lookup_cell_any(
                row,
                &[
                    "Source IDs",
                    "Source ids",
                    "Sources",
                    "Readings",
                    "Source references",
                ],
            )
            .as_str(),
            start_here.as_str(),
        ]);
        let source_ids = source_lookup.resolve_refs_with_diagnostics(
            &source_refs,
            format!("/report/literature_ladder/{index}/source_ids"),
            &id,
            diagnostics,
        );

        rows.push(LiteratureLadderRow {
            id,
            layer,
            start_here,
            read_for,
            do_not_infer,
            source_ids,
            notes: lookup_cell_any(row, &["Notes", "Note"]),
        });
    }
    rows
}

pub(crate) fn parse_source_requirement(raw: &str) -> SourceRoleRequirementKind {
    let text = normalize_id_text(raw);
    if text.contains("waiv") {
        SourceRoleRequirementKind::Waived
    } else if text.contains("conditional") || text.contains("if ") {
        SourceRoleRequirementKind::Conditional
    } else if text.contains("not applicable") || text.contains("n a") {
        SourceRoleRequirementKind::NotApplicable
    } else if text.contains("required") || text.contains("cannot be waived") {
        SourceRoleRequirementKind::Required
    } else {
        SourceRoleRequirementKind::Conditional
    }
}
