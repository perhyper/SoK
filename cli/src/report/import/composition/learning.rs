use super::*;

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
