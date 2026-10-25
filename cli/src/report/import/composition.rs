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

pub(crate) fn build_curriculum_path(
    parsed: &ParsedMarkdownReport,
    source_lookup: &SourceLookup,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<CurriculumStep> {
    let mut steps = Vec::new();
    for (index, row) in parsed.curriculum_rows.iter().enumerate() {
        let title = curriculum_row_title(row);
        if title.is_empty() || is_placeholder_text(&title) {
            continue;
        }
        let learning_goal = first_non_empty([
            lookup_cell_any(row, &["Essential question"]).as_str(),
            lookup_cell_any(row, &["Learning goal"]).as_str(),
            title.as_str(),
        ]);
        let practice_artifact = first_non_empty([
            lookup_cell_any(row, &["Practice artifact"]).as_str(),
            lookup_cell_any(row, &["Artifact"]).as_str(),
            "Practice artifact not inferred from the structured report surface.",
        ]);
        let progress_criteria = split_listish(&lookup_cell_any(
            row,
            &["Progress criteria", "Criteria", "Assessment"],
        ));
        let source_refs = lookup_cell_any(row, &["Readings", "Sources", "Source IDs"]);
        let id = content_id("step", &[&title]);
        let source_ids = source_lookup.resolve_refs_with_diagnostics(
            &source_refs,
            format!("/report/curriculum_path/{index}/source_ids"),
            &id,
            diagnostics,
        );
        steps.push(CurriculumStep {
            id,
            sequence: (index + 1) as u32,
            title,
            learning_goal,
            prerequisite_ids: Vec::new(),
            practice_artifact,
            progress_criteria,
            source_ids,
        });
    }

    steps
}

pub(crate) fn curriculum_row_title(row: &BTreeMap<String, String>) -> String {
    first_non_empty([
        lookup_cell_any(row, &["Module"]).as_str(),
        lookup_cell_any(row, &["Phase"]).as_str(),
        lookup_cell_any(row, &["Title"]).as_str(),
    ])
}

pub(crate) fn apply_curriculum_prerequisites(
    parsed: &ParsedMarkdownReport,
    steps: &mut [CurriculumStep],
    entity_index: &ExportEntityIndex,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<Relation> {
    let mut refs_by_step_id = BTreeMap::new();
    for row in &parsed.curriculum_rows {
        let title = curriculum_row_title(row);
        if title.is_empty() || is_placeholder_text(&title) {
            continue;
        }
        let raw_refs = lookup_cell_any(
            row,
            &[
                "Prerequisite IDs",
                "Prerequisite ids",
                "Prerequisites",
                "Prereq IDs",
                "Prereqs",
            ],
        );
        if raw_refs.trim().is_empty() {
            continue;
        }
        refs_by_step_id.insert(content_id("step", &[&title]), split_ref_list(&raw_refs));
    }

    let mut prerequisite_relations = Vec::new();
    for (step_index, step) in steps.iter_mut().enumerate() {
        let Some(refs) = refs_by_step_id.get(&step.id) else {
            continue;
        };
        let mut prerequisite_ids = BTreeSet::new();
        for reference in refs {
            match entity_index.resolve_any_legacy_projection(reference) {
                ExportReferenceResolution::Resolved(entity) => {
                    if entity.id == step.id {
                        diagnostics.push(
                            DiagnosticCheck::warning(
                                CHECK_VALIDATE_CURRICULUM_REFERENCE,
                                format!(
                                    "curriculum step {} lists itself as a prerequisite",
                                    step.id
                                ),
                            )
                            .with_target(
                                format!("/report/curriculum_path/{step_index}/prerequisite_ids"),
                                &step.id,
                            ),
                        );
                    } else {
                        let prerequisite_endpoint = RelationEndpoint {
                            entity_type: entity.entity_type,
                            id: entity.id,
                        };
                        prerequisite_ids.insert(prerequisite_endpoint.id.clone());
                        prerequisite_relations.push(Relation {
                            id: content_id(
                                "rel",
                                &[
                                    relation_kind_label(RelationKind::RequiresBefore),
                                    entity_type_label(prerequisite_endpoint.entity_type),
                                    &prerequisite_endpoint.id,
                                    entity_type_label(EntityType::CurriculumStep),
                                    &step.id,
                                ],
                            ),
                            kind: RelationKind::RequiresBefore,
                            from: prerequisite_endpoint,
                            to: RelationEndpoint {
                                entity_type: EntityType::CurriculumStep,
                                id: step.id.clone(),
                            },
                            description: format!(
                                "{} is listed as a prerequisite for {} in the curriculum surface.",
                                reference, step.title
                            ),
                            source_ids: step.source_ids.clone(),
                        });
                    }
                }
                ExportReferenceResolution::Missing => diagnostics.push(
                    DiagnosticCheck::warning(
                        CHECK_EXPORT_UNRESOLVED_REFERENCE,
                        format!(
                            "curriculum step {} could not resolve prerequisite reference {:?}",
                            step.id, reference
                        ),
                    )
                    .with_target(
                        format!("/report/curriculum_path/{step_index}/prerequisite_ids"),
                        &step.id,
                    ),
                ),
                ExportReferenceResolution::Ambiguous(candidates) => diagnostics.push(
                    DiagnosticCheck::error(
                        CHECK_EXPORT_AMBIGUOUS_REFERENCE,
                        format!(
                            "curriculum step {} prerequisite reference {:?} is ambiguous: {}",
                            step.id,
                            reference,
                            candidates.join(", ")
                        ),
                    )
                    .with_target(
                        format!("/report/curriculum_path/{step_index}/prerequisite_ids"),
                        &step.id,
                    ),
                ),
            }
        }
        step.prerequisite_ids = prerequisite_ids.into_iter().collect();
    }
    prerequisite_relations
}

pub(crate) fn build_frontier_debates(
    parsed: &ParsedMarkdownReport,
    source_lookup: &SourceLookup,
    as_of: &str,
    review_after: &str,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> (Vec<FrontierDebateItem>, Vec<ClaimSeed>) {
    let mut items = Vec::new();
    let mut claims = Vec::new();
    let mut seen_titles = BTreeSet::new();
    for row in &parsed.frontier_rows {
        let title = first_non_empty([
            lookup_cell_any(row, &["Problem or debate"]).as_str(),
            lookup_cell_any(row, &["Area"]).as_str(),
            lookup_cell_any(row, &["Title"]).as_str(),
        ]);
        if title.is_empty() || is_placeholder_text(&title) {
            continue;
        }
        seen_titles.insert(normalize_alias_text(&title));
        let summary = first_non_empty([
            lookup_cell_any(row, &["Current state"]).as_str(),
            lookup_cell_any(row, &["Current or frontier issue"]).as_str(),
            lookup_cell_any(row, &["Summary"]).as_str(),
            "Frontier or debate summary not inferred from the structured report surface.",
        ]);
        if is_placeholder_text(&summary) {
            continue;
        }
        let why_it_matters = first_non_empty([
            lookup_cell_any(row, &["Why it matters"]).as_str(),
            lookup_cell_any(row, &["Why it is hard"]).as_str(),
            "This item marks a frontier, debate, or open problem boundary.",
        ]);
        let source_refs = lookup_cell_any(row, &["Key sources", "Sources", "Source IDs"]);
        let id = content_id("frontier", &[&title]);
        let source_ids = source_lookup.resolve_refs_with_diagnostics(
            &source_refs,
            format!("/report/frontier_debates/{}/source_ids", items.len()),
            &id,
            diagnostics,
        );
        let claim_statement = format!("{title}: {summary}");
        let claim_id = content_id("claim", &[&claim_statement]);
        claims.push(ClaimSeed {
            statement: claim_statement,
            claim_type: if normalize_id_text(&title).contains("debate") {
                ClaimType::Debate
            } else {
                ClaimType::Frontier
            },
            evidence_requirement: EvidenceRequirement::ReviewedSource,
            source_refs: split_ref_list(&source_refs),
            confidence: Some(ClaimConfidence::Unknown),
            notes: String::new(),
            temporal_status: Some(TemporalStatus::Current),
        });
        items.push(FrontierDebateItem {
            id,
            kind: if normalize_id_text(&title).contains("debate") {
                FrontierDebateKind::Debate
            } else if normalize_id_text(&title).contains("problem") {
                FrontierDebateKind::OpenProblem
            } else {
                FrontierDebateKind::Frontier
            },
            title,
            summary,
            why_it_matters,
            required_background_ids: Vec::new(),
            claim_ids: vec![claim_id],
            source_ids,
            temporal: TemporalMarker {
                as_of: as_of.to_string(),
                review_after: review_after.to_string(),
                temporal_status: TemporalStatus::Current,
                rationale: String::new(),
            },
        });
    }
    for row in &parsed.deep_structure_rows {
        let element_class = normalize_id_text(&lookup_cell_any(
            row,
            &["Element class", "Class", "Element type", "Element"],
        ));
        if ![
            "frontier",
            "dispute",
            "debate",
            "open problem",
            "controversy",
        ]
        .iter()
        .any(|kind| element_class.contains(kind))
        {
            continue;
        }
        let title = lookup_cell_any(row, &["Observed element", "Element", "Name", "Title"]);
        if title.is_empty()
            || is_placeholder_text(&title)
            || !seen_titles.insert(normalize_alias_text(&title))
        {
            continue;
        }
        let summary = first_non_empty([
            lookup_cell_any(row, &["Actual form in this field", "In this field"]).as_str(),
            lookup_cell_any(row, &["SoK extraction", "Summary"]).as_str(),
            title.as_str(),
        ]);
        let why_it_matters = first_non_empty([
            lookup_cell_any(
                row,
                &[
                    "Load-bearing relations",
                    "Why it matters",
                    "Role",
                    "Role: core / surrounding / context",
                ],
            )
            .as_str(),
            "This field element marks a frontier, debate, or open-problem boundary.",
        ]);
        let source_refs = lookup_cell_any(row, &["Source IDs", "Sources", "Key sources"]);
        let id = content_id("frontier", &[&title]);
        let source_ids = source_lookup.resolve_refs_with_diagnostics(
            &source_refs,
            format!("/report/frontier_debates/{}/source_ids", items.len()),
            &id,
            diagnostics,
        );
        let kind = if element_class.contains("debate")
            || element_class.contains("dispute")
            || element_class.contains("controversy")
        {
            FrontierDebateKind::Debate
        } else if element_class.contains("open problem") {
            FrontierDebateKind::OpenProblem
        } else {
            FrontierDebateKind::Frontier
        };
        let claim_statement = format!("{title}: {summary}");
        let claim_id = content_id("claim", &[&claim_statement]);
        claims.push(ClaimSeed {
            statement: claim_statement,
            claim_type: if kind == FrontierDebateKind::Debate {
                ClaimType::Debate
            } else {
                ClaimType::Frontier
            },
            evidence_requirement: EvidenceRequirement::ReviewedSource,
            source_refs: split_ref_list(&source_refs),
            confidence: Some(ClaimConfidence::Unknown),
            notes: String::new(),
            temporal_status: Some(TemporalStatus::Current),
        });
        items.push(FrontierDebateItem {
            id,
            kind,
            title,
            summary,
            why_it_matters,
            required_background_ids: Vec::new(),
            claim_ids: vec![claim_id],
            source_ids,
            temporal: TemporalMarker {
                as_of: as_of.to_string(),
                review_after: review_after.to_string(),
                temporal_status: TemporalStatus::Current,
                rationale: String::new(),
            },
        });
    }
    (items, claims)
}

pub(crate) fn build_claim_seeds(parsed: &ParsedMarkdownReport) -> Vec<ClaimSeed> {
    let mut seeds = Vec::new();
    for row in &parsed.claim_rows {
        let statement = first_non_empty([
            lookup_cell_any(row, &["Statement"]).as_str(),
            lookup_cell_any(row, &["Claim"]).as_str(),
        ]);
        if statement.is_empty() || is_placeholder_text(&statement) {
            continue;
        }
        seeds.push(ClaimSeed {
            statement,
            claim_type: parse_claim_type(&lookup_cell_any(row, &["Claim type", "Type"])),
            evidence_requirement: parse_evidence_requirement(&lookup_cell_any(
                row,
                &["Evidence requirement", "Requirement"],
            )),
            source_refs: split_ref_list(&lookup_cell_any(
                row,
                &["Source IDs", "Source ids", "Sources", "Key sources"],
            )),
            confidence: parse_claim_confidence(&lookup_cell_any(row, &["Confidence"])),
            notes: lookup_cell_any(row, &["Notes", "Note"]),
            temporal_status: parse_temporal_status(&lookup_cell_any(
                row,
                &["Temporal status", "Temporal"],
            )),
        });
    }
    seeds
}

pub(crate) fn build_claims(
    seeds: Vec<ClaimSeed>,
    source_lookup: &SourceLookup,
    evidence: &[EvidenceEntry],
    stage: ExportStage,
    as_of: &str,
    review_after: &str,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<Claim> {
    let mut claims = Vec::new();
    for seed in seeds {
        let id = content_id("claim", &[&seed.statement]);
        let intended_source_ids = source_lookup.resolve_ref_list_with_diagnostics(
            &seed.source_refs,
            "/report/claims/source_ids",
            &id,
            diagnostics,
        );
        let mut evidence_links = Vec::new();
        let mut saw_cataloged_only = false;
        let mut saw_satisfying_evidence = false;

        for entry in evidence {
            if !intended_source_ids.is_empty() && !intended_source_ids.contains(&entry.source_id) {
                continue;
            }
            if intended_source_ids.is_empty()
                && !entry.claim_ids.iter().any(|claim_id| claim_id == &id)
            {
                continue;
            }
            if !entry.claim_ids.is_empty()
                && !entry.claim_ids.iter().any(|claim_id| claim_id == &id)
            {
                continue;
            }
            if entry.can_satisfy_claim_link() {
                saw_satisfying_evidence = true;
            }
            if entry.verification_status == VerificationStatus::Cataloged {
                saw_cataloged_only = true;
            }
            if let Some(link) = entry.to_visible_evidence_link() {
                if let Some(existing_index) =
                    evidence_links.iter().position(|existing: &EvidenceLink| {
                        duplicate_claim_evidence_link(existing, &link)
                    })
                {
                    if link.verification_status == VerificationStatus::Cataloged
                        && !entry.claim_ids.is_empty()
                    {
                        evidence_links[existing_index] = link;
                    }
                } else {
                    evidence_links.push(link);
                }
            }
        }

        if stage == ExportStage::Final
            && !saw_satisfying_evidence
            && (!intended_source_ids.is_empty() || saw_cataloged_only)
        {
            let check_id = if saw_cataloged_only {
                CHECK_EVIDENCE_CATALOGED_ONLY
            } else {
                CHECK_EXPORT_CLAIM_NEEDS_EVIDENCE
            };
            diagnostics.push(
                DiagnosticCheck::warning(
                    check_id,
                    format!(
                        "claim {} has no reviewed or verified evidence link with usable support metadata",
                        id
                    ),
                )
                .with_target("/report/claims", &id),
            );
        }

        let temporal_status = seed
            .temporal_status
            .unwrap_or(match (stage, seed.claim_type) {
                (ExportStage::Scaffold, _) => TemporalStatus::Unknown,
                (_, ClaimType::Frontier | ClaimType::Currentness | ClaimType::Debate) => {
                    TemporalStatus::Current
                }
                _ => TemporalStatus::Durable,
            });
        claims.push(Claim {
            id,
            statement: seed.statement,
            claim_type: seed.claim_type,
            evidence_requirement: seed.evidence_requirement,
            evidence_links,
            confidence: seed.confidence,
            temporal: TemporalMarker {
                as_of: as_of.to_string(),
                review_after: if matches!(
                    temporal_status,
                    TemporalStatus::Current | TemporalStatus::ReviewDue | TemporalStatus::Stale
                ) {
                    review_after.to_string()
                } else {
                    String::new()
                },
                temporal_status,
                rationale: String::new(),
            },
            notes: seed.notes,
        });
    }
    claims
}

pub(crate) fn duplicate_claim_evidence_link(existing: &EvidenceLink, next: &EvidenceLink) -> bool {
    if existing.verification_status == VerificationStatus::Cataloged
        && next.verification_status == VerificationStatus::Cataloged
    {
        return existing.source_id == next.source_id;
    }
    existing.evidence_id == next.evidence_id && existing.source_id == next.source_id
}
pub(crate) fn build_visual_views(
    parsed: &ParsedMarkdownReport,
    relations: &[Relation],
    entity_index: &ExportEntityIndex,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<VisualView> {
    let relation_lookup = relations
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect::<BTreeMap<_, _>>();
    let mut views = Vec::new();
    let mut seen_ids = BTreeSet::new();

    for (index, row) in parsed.visual_view_rows.iter().enumerate() {
        let title = first_non_empty([
            lookup_cell_any(row, &["Title", "View title", "Name"]).as_str(),
            "Relation-backed visual view",
        ]);
        let kind =
            parse_visual_view_kind_input(&lookup_cell_any(row, &["View kind", "Kind", "Type"]));
        let relation_refs = split_ref_list(&lookup_cell_any(
            row,
            &["Relation IDs", "Relation ids", "Relations", "Relation ID"],
        ));
        if relation_refs.is_empty() {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_MISSING_PUBLIC_FIELD,
                    format!("Visual Views row {} has no relation IDs", index + 1),
                )
                .with_target(format!("/report/visual_views/{index}/edges"), ""),
            );
            continue;
        }

        let mut selected_relations = Vec::new();
        for relation_ref in relation_refs {
            let normalized_relation_ref = if is_stable_id(&relation_ref) {
                relation_ref.clone()
            } else {
                content_id("rel", &[&relation_ref])
            };
            if let Some(relation) = relation_lookup.get(normalized_relation_ref.as_str()) {
                selected_relations.push(*relation);
            } else {
                diagnostics.push(
                    DiagnosticCheck::warning(
                        CHECK_EXPORT_UNRESOLVED_REFERENCE,
                        format!(
                            "Visual Views row {} references missing relation id {}",
                            index + 1,
                            relation_ref
                        ),
                    )
                    .with_target(format!("/report/visual_views/{index}/edges"), ""),
                );
            }
        }
        if selected_relations.is_empty() {
            continue;
        }

        let explicit_id = lookup_cell_any(row, &["View ID", "View id", "ID"]);
        let relation_id_text = selected_relations
            .iter()
            .map(|relation| relation.id.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let id = if explicit_id.trim().is_empty() {
            content_id(
                "view",
                &[
                    &title,
                    visual_view_kind_export_label(kind),
                    &relation_id_text,
                ],
            )
        } else if is_stable_id(&explicit_id) {
            explicit_id
        } else {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_MISSING_PUBLIC_FIELD,
                    format!(
                        "Visual Views row {} has non-stable explicit view id {:?}; generated a stable id",
                        index + 1,
                        explicit_id
                    ),
                )
                .with_target(format!("/report/visual_views/{index}/id"), &explicit_id),
            );
            content_id(
                "view",
                &[
                    &title,
                    visual_view_kind_export_label(kind),
                    &relation_id_text,
                ],
            )
        };
        if !seen_ids.insert(id.clone()) {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_AMBIGUOUS_REFERENCE,
                    format!("Visual Views row {} repeats view id {}", index + 1, id),
                )
                .with_target(format!("/report/visual_views/{index}/id"), &id),
            );
            continue;
        }

        let mut nodes_by_entity_id = BTreeMap::new();
        let mut edges = Vec::new();
        for relation in selected_relations {
            let from_node_id =
                insert_visual_node(&mut nodes_by_entity_id, &relation.from, entity_index);
            let to_node_id =
                insert_visual_node(&mut nodes_by_entity_id, &relation.to, entity_index);
            edges.push(VisualViewEdge {
                from: from_node_id,
                to: to_node_id,
                kind: relation_kind_label(relation.kind).to_string(),
                relation_id: relation.id.clone(),
                label: relation.description.clone(),
            });
        }

        apply_visual_node_emphasis(
            row,
            index,
            &mut nodes_by_entity_id,
            entity_index,
            diagnostics,
        );

        let purpose = first_non_empty([
            lookup_cell_any(row, &["Purpose", "Justification", "Rationale"]).as_str(),
            lookup_cell_any(row, &["Alt text", "Reading guidance"]).as_str(),
        ]);
        let justification = visual_view_justification(&purpose, &parsed.visual_summary);

        views.push(VisualView {
            id,
            kind,
            title,
            justification,
            nodes: nodes_by_entity_id.into_values().collect(),
            edges,
        });
    }
    views
}

pub(crate) fn insert_visual_node(
    nodes_by_entity_id: &mut BTreeMap<String, VisualViewNode>,
    endpoint: &RelationEndpoint,
    entity_index: &ExportEntityIndex,
) -> String {
    let node_id = visual_node_id(endpoint.entity_type, &endpoint.id);
    nodes_by_entity_id
        .entry(endpoint.id.clone())
        .or_insert_with(|| {
            let label = entity_index
                .record(&endpoint.id)
                .map(|entity| entity.label.clone())
                .unwrap_or_else(|| endpoint.id.clone());
            VisualViewNode {
                id: node_id.clone(),
                label,
                entity_type: endpoint.entity_type,
                ref_id: endpoint.id.clone(),
                description: String::new(),
            }
        });
    node_id
}

pub(crate) fn apply_visual_node_emphasis(
    row: &BTreeMap<String, String>,
    row_index: usize,
    nodes_by_entity_id: &mut BTreeMap<String, VisualViewNode>,
    entity_index: &ExportEntityIndex,
    diagnostics: &mut Vec<DiagnosticCheck>,
) {
    let emphasis_refs = split_ref_list(&lookup_cell_any(
        row,
        &[
            "Node emphasis",
            "Emphasis",
            "Emphasized nodes",
            "Node emphasis IDs",
            "Node emphasis ids",
        ],
    ));
    for reference in emphasis_refs {
        let normalized_reference = normalize_alias_text(&reference);
        let matching_existing_nodes = nodes_by_entity_id
            .iter()
            .filter(|(_, node)| {
                node.ref_id == reference
                    || normalize_alias_text(&node.label) == normalized_reference
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if matching_existing_nodes.len() == 1 {
            if let Some(node) = nodes_by_entity_id.get_mut(&matching_existing_nodes[0]) {
                node.description =
                    "Emphasized by the Markdown visual view declaration.".to_string();
            }
            continue;
        }
        match entity_index.resolve_any(&reference) {
            ExportReferenceResolution::Resolved(entity) => {
                let endpoint = RelationEndpoint {
                    entity_type: entity.entity_type,
                    id: entity.id,
                };
                let node_id = insert_visual_node(nodes_by_entity_id, &endpoint, entity_index);
                if let Some(node) = nodes_by_entity_id.get_mut(&endpoint.id) {
                    node.description =
                        "Emphasized by the Markdown visual view declaration.".to_string();
                    node.id = node_id;
                }
            }
            ExportReferenceResolution::Missing => diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_UNRESOLVED_REFERENCE,
                    format!(
                        "Visual Views row {} could not resolve node emphasis reference {:?}",
                        row_index + 1,
                        reference
                    ),
                )
                .with_target(format!("/report/visual_views/{row_index}/nodes"), ""),
            ),
            ExportReferenceResolution::Ambiguous(candidates) => diagnostics.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_AMBIGUOUS_REFERENCE,
                    format!(
                        "Visual Views row {} node emphasis reference {:?} is ambiguous: {}",
                        row_index + 1,
                        reference,
                        candidates.join(", ")
                    ),
                )
                .with_target(format!("/report/visual_views/{row_index}/nodes"), ""),
            ),
        }
    }
}

pub(crate) fn visual_node_id(entity_type: EntityType, entity_id: &str) -> String {
    content_id("vnode", &[entity_type_label(entity_type), entity_id])
}

pub(crate) fn visual_view_justification(purpose: &str, visual_summary: &str) -> String {
    match (purpose.trim().is_empty(), visual_summary.trim().is_empty()) {
        (false, false) => format!("{purpose} Visual guidance: {visual_summary}"),
        (false, true) => purpose.trim().to_string(),
        (true, false) => visual_summary.trim().to_string(),
        (true, true) => "Relation-backed visual view declared in Markdown.".to_string(),
    }
}

pub(crate) fn warn_if_unpreserved_visual_sections(
    parsed: &ParsedMarkdownReport,
    visual_views: &[VisualView],
    diagnostics: &mut Vec<DiagnosticCheck>,
) {
    if !visual_views.is_empty() {
        return;
    }
    if parsed_has_section(parsed, CanonicalSection::VisualMap) {
        diagnostics.push(DiagnosticCheck::warning(
            CHECK_EXPORT_UNSUPPORTED_SECTION,
            "Markdown visual map was not converted; Mermaid arrows require matching rows in a \
                structured Relations table plus a Visual Views table",
        ));
    }
    if parsed_has_section(parsed, CanonicalSection::VisualSummary)
        && !parsed.visual_summary.trim().is_empty()
    {
        diagnostics.push(DiagnosticCheck::warning(
            CHECK_EXPORT_UNSUPPORTED_SECTION,
            "Markdown Visual Summary was not attached to an explicit Visual Views table",
        ));
    }
}

pub(crate) fn evidence_source_diagnostics(
    evidence: &[EvidenceEntry],
    source_ids: &BTreeSet<String>,
) -> Vec<DiagnosticCheck> {
    evidence
        .iter()
        .filter(|entry| !source_ids.contains(&entry.source_id))
        .map(|entry| {
            DiagnosticCheck::warning(
                CHECK_EXPORT_UNKNOWN_EVIDENCE_SOURCE,
                format!(
                    "evidence {} references unknown source_id {}",
                    entry.evidence_id, entry.source_id
                ),
            )
            .with_target("/evidence", &entry.evidence_id)
        })
        .collect()
}

pub(crate) fn apply_evidence_review_status(
    sources: &mut [ReportSource],
    evidence: &[EvidenceEntry],
) {
    for source in sources {
        let mut status = source.verification_status;
        let mut last_reviewed = source.last_reviewed.clone();
        for entry in evidence.iter().filter(|entry| entry.source_id == source.id) {
            status = max_verification_status(status, entry.verification_status);
            let reviewed_at =
                first_non_empty([entry.reviewed_at.as_str(), entry.observed_at.as_str()]);
            if !reviewed_at.is_empty() && reviewed_at > last_reviewed {
                last_reviewed = reviewed_at;
            }
        }
        source.verification_status = status;
        source.last_reviewed = last_reviewed;
    }
}

pub(crate) fn max_verification_status(
    left: VerificationStatus,
    right: VerificationStatus,
) -> VerificationStatus {
    if verification_rank(right) > verification_rank(left) {
        right
    } else {
        left
    }
}

pub(crate) fn verification_rank(status: VerificationStatus) -> u8 {
    match status {
        VerificationStatus::Cataloged => 0,
        VerificationStatus::Reviewed => 1,
        VerificationStatus::Verified => 2,
    }
}
