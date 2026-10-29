use super::*;

pub(crate) fn push_nav(html: &mut String, report: &PublicReport, has_visuals: bool) {
    let mut items = vec![
        ("reading-guide", "Reading Guide"),
        ("curriculum-path", "Curriculum Path"),
        ("reading-ladder", "Reading Ladder"),
        ("claims", "Claim Evidence"),
        ("frontier-debates", "Frontier Guidance"),
        ("scope", "Scope"),
        ("domain-profile", "Domain Profile"),
        ("evidence-standards", "Evidence Standards"),
        ("sources-evidence", "Source Catalog"),
        ("relations", "Relation Audit"),
    ];
    let structure_index = items
        .iter()
        .position(|(id, _)| *id == "evidence-standards")
        .unwrap_or(items.len());
    if report.field_elements.is_empty() {
        for item in [
            ("representations", "Representations"),
            ("methods", "Methods"),
            ("core-ideas", "Core Ideas"),
        ] {
            items.insert(structure_index, item);
        }
    } else {
        items.insert(structure_index, ("field-elements", "Field Elements"));
    }
    if has_visuals {
        items.insert(1, ("visualizations", "Knowledge Map"));
    }
    html.push_str("<nav class=\"section-nav\" aria-label=\"Report sections\">\n");
    for (id, label) in items {
        html.push_str("<a href=\"#");
        push_escaped_attr(html, id);
        html.push_str("\">");
        push_escaped(html, label);
        html.push_str("</a>\n");
    }
    html.push_str("</nav>\n");
}

pub(crate) fn push_reading_guide_section(
    html: &mut String,
    document: &ReportDocument,
    has_visuals: bool,
) {
    let report = &document.report;
    push_section_open(html, "reading-guide", "Reading Guide");
    push_paragraph(html, &report.scope.summary);
    html.push_str("<ol class=\"guide-list\">\n");
    if has_visuals {
        push_guide_item(
            html,
            "Orient with the knowledge map",
            "Use the relation-backed visual view first; every drawn edge comes from a validated public relation.",
        );
    }
    push_guide_item(
        html,
        "Follow the curriculum path",
        "Read the steps in sequence and check prerequisites before moving to each new layer.",
    );
    push_guide_item(
        html,
        "Use the reading ladder",
        "Start from the layer guidance and read sources for the stated purpose, including the \
            explicit do-not-infer caveats.",
    );
    push_guide_item(
        html,
        "Check claims against evidence",
        "Read each claim with its supporting, qualifying, and contradictory evidence before \
            treating it as settled.",
    );
    if !report.frontier_debates.is_empty() {
        push_guide_item(
            html,
            "Handle frontier guidance last",
            "Treat current or debate-facing material as time-bound and review the stated \
                background and source support.",
        );
    }
    html.push_str("</ol>\n");
    html.push_str("<p class=\"meta\"><strong>Report review:</strong> ");
    push_escaped(
        html,
        &format_temporal_inline(&document.metadata.temporal_review),
    );
    html.push_str("</p>\n</section>\n");
}

pub(crate) fn push_guide_item(html: &mut String, title: &str, body: &str) {
    html.push_str("<li><strong>");
    push_escaped(html, title);
    html.push_str("</strong><span>");
    push_escaped(html, body);
    html.push_str("</span></li>\n");
}

pub(crate) fn push_scope_section(html: &mut String, scope: &Scope) {
    push_section_open(html, "scope", "Scope");
    push_paragraph(html, &scope.summary);
    html.push_str("<div class=\"split-list\">\n");
    push_string_list(html, "Included", &scope.included);
    push_string_list(html, "Excluded", &scope.excluded);
    push_string_list(html, "Assumptions", &scope.assumptions);
    push_string_list(html, "Interpretive Notes", &scope.interpretive_notes);
    html.push_str("</div>\n</section>\n");
}

pub(crate) fn push_domain_profile_section(html: &mut String, profile: &DomainProfile) {
    push_section_open(html, "domain-profile", "Domain Profile");
    let mut classifications = vec![domain_classification_label(profile.classification).to_string()];
    classifications.extend(
        profile
            .secondary_characteristics
            .iter()
            .map(|classification| domain_classification_label(*classification).to_string()),
    );
    html.push_str("<p class=\"meta\"><strong>Classification:</strong> ");
    push_escaped(html, &classifications.join(", "));
    html.push_str("</p>\n");
    push_paragraph(html, &profile.rationale);
    push_string_list(html, "Failure Modes", &profile.failure_modes);
    html.push_str("</section>\n");
}

pub(crate) fn push_field_elements_or_legacy(
    html: &mut String,
    report: &PublicReport,
    source_labels: &BTreeMap<String, String>,
) {
    if report.field_elements.is_empty() {
        push_knowledge_section(
            html,
            "core-ideas",
            "Core Ideas",
            &report.core_ideas,
            source_labels,
        );
        push_knowledge_section(html, "methods", "Methods", &report.methods, source_labels);
        push_knowledge_section(
            html,
            "representations",
            "Representations",
            &report.representations,
            source_labels,
        );
    } else {
        push_field_elements_section(html, &report.field_elements, source_labels);
    }
}

pub(crate) fn push_field_elements_section(
    html: &mut String,
    elements: &[FieldElement],
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "field-elements", "Field Elements");
    html.push_str("<div class=\"item-grid\">\n");
    for element in elements {
        html.push_str("<article class=\"item-card\">\n<h3>");
        push_escaped(html, &element.label);
        html.push_str("</h3>\n<div class=\"badge-row\">");
        push_badge(html, &element.element_class);
        push_badge(html, &element.role);
        if let Some(confidence) = element.confidence {
            push_badge(html, claim_confidence_label(confidence));
        }
        html.push_str("</div>\n<dl class=\"compact-dl\">\n");
        push_dl_item(html, "Actual form", &element.actual_form);
        push_dl_item(
            html,
            "Load-bearing relations",
            &element.load_bearing_relations,
        );
        html.push_str("</dl>\n");
        push_source_reference_list(html, "Sources", &element.source_ids, source_labels);
        push_raw_details(
            html,
            "Raw field element identifier",
            &[
                ("Element ID", element.id.clone()),
                ("Source IDs", element.source_ids.join(", ")),
            ],
        );
        html.push_str("</article>\n");
    }
    html.push_str("</div>\n</section>\n");
}

pub(crate) fn push_knowledge_section(
    html: &mut String,
    id: &str,
    title: &str,
    items: &[KnowledgeItem],
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, id, title);
    if items.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"item-grid\">\n");
        for item in items {
            html.push_str("<article class=\"item-card\">\n<h3>");
            push_escaped(html, &item.label);
            html.push_str("</h3>\n");
            push_paragraph(html, &item.description);
            push_nested_string_list(html, "Aliases", &item.aliases);
            push_source_reference_list(html, "Sources", &item.source_ids, source_labels);
            push_raw_details(
                html,
                "Raw item identifiers",
                &[
                    ("Item ID", item.id.clone()),
                    ("Source IDs", item.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_evidence_standards_section(html: &mut String, standards: &EvidenceStandards) {
    push_section_open(html, "evidence-standards", "Evidence Standards");
    push_paragraph(html, &standards.summary);
    push_paragraph(html, &standards.claim_policy);
    let rows = standards
        .source_role_requirements
        .iter()
        .map(|requirement| {
            vec![
                source_role_label(requirement.role).to_string(),
                source_role_requirement_label(requirement.requirement).to_string(),
                requirement
                    .minimum_sources
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                requirement.rationale.clone(),
                requirement
                    .waiver
                    .as_ref()
                    .map(format_source_role_waiver)
                    .unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    push_table(
        html,
        &["Role", "Requirement", "Minimum", "Rationale", "Waiver"],
        &rows,
    );
    html.push_str("</section>\n");
}

pub(crate) fn push_sources_and_evidence_section(html: &mut String, report: &PublicReport) {
    push_section_open(html, "sources-evidence", "Source Catalog");
    if report.sources.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"source-grid\">\n");
        for source in &report.sources {
            html.push_str("<article class=\"source-card\">\n<h3>");
            push_escaped(html, &source_display_label(source));
            html.push_str("</h3>\n<div class=\"badge-row\">");
            push_badge(html, &source.source_type);
            push_badge(html, verification_status_label(source.verification_status));
            for role in &source.roles {
                push_badge(html, source_role_label(*role));
            }
            html.push_str("</div>\n");
            push_paragraph(html, &source.why_it_matters);
            html.push_str("<dl class=\"compact-dl\">\n");
            push_dl_item(html, "Roles", &format_source_roles(&source.roles));
            push_dl_item(html, "Access", &format_source_access(&source.access));
            push_dl_item(html, "Identifier", &source.identifier);
            push_dl_item(html, "Date", &source.date);
            push_dl_item(html, "Last reviewed", &source.last_reviewed);
            push_dl_item(html, "Notes", &source.notes);
            html.push_str("</dl>\n");
            push_raw_details(
                html,
                "Raw source fields",
                &[
                    ("Source ID", source.id.clone()),
                    ("Citation", source.citation.clone()),
                    ("URL", source.url.clone()),
                ],
            );
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_claims_section(
    html: &mut String,
    claims: &[Claim],
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "claims", "Claim Evidence Guide");
    if claims.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"claim-grid\">\n");
        for claim in claims {
            html.push_str("<article class=\"claim-card\">\n<h3>");
            push_escaped(html, &claim.statement);
            html.push_str("</h3>\n<div class=\"badge-row\">");
            push_badge(html, claim_type_label(claim.claim_type));
            push_badge(html, evidence_requirement_label(claim.evidence_requirement));
            push_badge(
                html,
                claim
                    .confidence
                    .map(claim_confidence_label)
                    .unwrap_or("unknown"),
            );
            push_badge(html, temporal_status_label(claim.temporal.temporal_status));
            html.push_str("</div>\n");
            html.push_str("<p class=\"meta\"><strong>Temporal status:</strong> ");
            push_escaped(html, &format_temporal_inline(&claim.temporal));
            html.push_str("</p>\n");
            push_evidence_group(
                html,
                "Supporting Evidence",
                &claim.evidence_links,
                source_labels,
                |kind| kind == SupportKind::Supports,
            );
            push_evidence_group(
                html,
                "Qualifying Evidence",
                &claim.evidence_links,
                source_labels,
                |kind| kind == SupportKind::Qualifies,
            );
            push_evidence_group(
                html,
                "Contradictory Evidence",
                &claim.evidence_links,
                source_labels,
                |kind| kind == SupportKind::Contradicts,
            );
            push_evidence_group(
                html,
                "Context Evidence",
                &claim.evidence_links,
                source_labels,
                |kind| matches!(kind, SupportKind::Background | SupportKind::Example),
            );
            if let Some(review) = &claim.conflict_review {
                push_paragraph(
                    html,
                    &format!(
                        "Conflict review ({}): {}",
                        review.reviewed_at, review.explanation
                    ),
                );
            }
            push_paragraph(html, &claim.notes);
            push_claim_raw_details(html, claim);
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_curriculum_section(
    html: &mut String,
    steps: &[CurriculumStep],
    entity_labels: &BTreeMap<String, String>,
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "curriculum-path", "Curriculum Path");
    if steps.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<ol class=\"curriculum-path\" aria-label=\"Visual curriculum path\">\n");
        for step in steps {
            html.push_str(
                "<li class=\"path-step\">\n<div class=\"path-marker\" aria-hidden=\"true\">",
            );
            push_escaped(html, &step.sequence.to_string());
            html.push_str("</div>\n<article class=\"path-card\">\n<h3>");
            push_escaped(html, &step.title);
            html.push_str("</h3>\n");
            push_paragraph(html, &step.learning_goal);
            if step.prerequisite_ids.is_empty() {
                html.push_str(
                    "<p class=\"meta\"><strong>Prerequisites:</strong> Entry point</p>\n",
                );
            } else {
                html.push_str("<p class=\"meta\"><strong>Prerequisites:</strong> ");
                push_escaped(html, &labels_for_ids(&step.prerequisite_ids, entity_labels));
                html.push_str("</p>\n");
            }
            push_paragraph(html, &step.practice_artifact);
            push_nested_string_list(html, "Progress Criteria", &step.progress_criteria);
            push_source_reference_list(html, "Sources", &step.source_ids, source_labels);
            push_raw_details(
                html,
                "Raw curriculum identifiers",
                &[
                    ("Step ID", step.id.clone()),
                    ("Prerequisite IDs", step.prerequisite_ids.join(", ")),
                    ("Source IDs", step.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n</li>\n");
        }
        html.push_str("</ol>\n");
        html.push_str(
            "<div class=\"text-fallback\" role=\"group\" aria-label=\"Text alternative for \
            curriculum path\">\n<h3>Text Alternative</h3>\n<ol>\n",
        );
        for step in steps {
            html.push_str("<li>");
            push_escaped(html, &step.title);
            if step.prerequisite_ids.is_empty() {
                html.push_str(": entry point.");
            } else {
                html.push_str(": follows ");
                push_escaped(html, &labels_for_ids(&step.prerequisite_ids, entity_labels));
                html.push('.');
            }
            html.push_str("</li>\n");
        }
        html.push_str("</ol>\n</div>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_frontier_section(
    html: &mut String,
    report: &PublicReport,
    entity_labels: &BTreeMap<String, String>,
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "frontier-debates", "Frontier Guidance");
    if report.frontier_debates.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"frontier-list\">\n");
        for item in &report.frontier_debates {
            html.push_str("<article class=\"frontier-card\">\n<h3>");
            push_escaped(html, &item.title);
            html.push_str("</h3>\n<div class=\"badge-row\">");
            push_badge(html, frontier_debate_kind_label(item.kind));
            push_badge(html, temporal_status_label(item.temporal.temporal_status));
            html.push_str("</div>\n");
            push_paragraph(html, &item.summary);
            push_paragraph(html, &item.why_it_matters);
            push_entity_reference_list(
                html,
                "Required Background",
                &item.required_background_ids,
                entity_labels,
            );
            push_entity_reference_list(html, "Related Claims", &item.claim_ids, entity_labels);
            push_source_reference_list(html, "Sources", &item.source_ids, source_labels);
            html.push_str("<p class=\"meta\"><strong>Temporal status:</strong> ");
            push_escaped(html, &format_temporal_inline(&item.temporal));
            html.push_str("</p>\n");
            push_raw_details(
                html,
                "Raw frontier identifiers",
                &[
                    ("Frontier ID", item.id.clone()),
                    ("Background IDs", item.required_background_ids.join(", ")),
                    ("Claim IDs", item.claim_ids.join(", ")),
                    ("Source IDs", item.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_reading_ladder_section(
    html: &mut String,
    report: &PublicReport,
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "reading-ladder", "Reading Ladder");
    if report.literature_ladder.is_empty() {
        push_source_ladder(html, &report.sources);
    } else {
        html.push_str("<ol class=\"ladder-list\">\n");
        for row in &report.literature_ladder {
            html.push_str("<li class=\"ladder-step\">\n<article class=\"ladder-card\">\n<h3>");
            push_escaped(html, &row.layer);
            html.push_str("</h3>\n<dl class=\"reader-dl\">\n");
            push_dl_item(html, "Start point", &row.start_here);
            push_dl_item(html, "Read for", &row.read_for);
            push_dl_item(html, "Do not infer", &row.do_not_infer);
            push_dl_item(html, "Notes", &row.notes);
            html.push_str("</dl>\n");
            push_source_reference_list(html, "Sources", &row.source_ids, source_labels);
            push_raw_details(
                html,
                "Raw ladder identifiers",
                &[
                    ("Ladder row ID", row.id.clone()),
                    ("Source IDs", row.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n</li>\n");
        }
        html.push_str("</ol>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_source_ladder(html: &mut String, sources: &[ReportSource]) {
    if sources.is_empty() {
        push_empty_note(html);
        return;
    }
    html.push_str("<p class=\"prose\">No explicit literature ladder is present; use the source metadata below \
        as the public source ladder.</p>\n");
    html.push_str("<ol class=\"ladder-list\">\n");
    for source in sources {
        let start_point = source_display_label(source);
        html.push_str("<li class=\"ladder-step\">\n<article class=\"ladder-card\">\n<h3>");
        push_escaped(html, &start_point);
        html.push_str("</h3>\n<div class=\"badge-row\">");
        for role in &source.roles {
            push_badge(html, source_role_label(*role));
        }
        push_badge(html, verification_status_label(source.verification_status));
        html.push_str("</div>\n<dl class=\"reader-dl\">\n");
        push_dl_item(html, "Start point", &start_point);
        push_dl_item(html, "Read for", &source.why_it_matters);
        push_dl_item(html, "Do not infer", &source.notes);
        html.push_str("</dl>\n");
        push_raw_details(
            html,
            "Raw source identifiers",
            &[
                ("Source ID", source.id.clone()),
                ("Identifier", source.identifier.clone()),
                ("URL", source.url.clone()),
            ],
        );
        html.push_str("</article>\n</li>\n");
    }
    html.push_str("</ol>\n");
}

pub(crate) fn push_relations_section(
    html: &mut String,
    relations: &[Relation],
    entity_labels: &BTreeMap<String, String>,
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "relations", "Relation Audit");
    if relations.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"relation-list\">\n");
        for relation in relations {
            let from = label_for_id(&relation.from.id, entity_labels);
            let to = label_for_id(&relation.to.id, entity_labels);
            html.push_str("<article class=\"relation-card\">\n<h3>");
            push_escaped(html, &from);
            html.push_str(" -> ");
            push_escaped(html, &to);
            html.push_str("</h3>\n<div class=\"badge-row\">");
            push_badge(html, relation_kind_label(relation.kind));
            push_badge(html, entity_type_label(relation.from.entity_type));
            push_badge(html, entity_type_label(relation.to.entity_type));
            html.push_str("</div>\n");
            push_paragraph(html, &relation.description);
            push_source_reference_list(html, "Sources", &relation.source_ids, source_labels);
            push_raw_details(
                html,
                "Raw relation identifiers",
                &[
                    ("Relation ID", relation.id.clone()),
                    ("From", relation.from.id.clone()),
                    ("To", relation.to.id.clone()),
                    ("Source IDs", relation.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_visual_section(html: &mut String, views: &[RenderVisualView]) {
    push_section_open(html, "visualizations", "Knowledge Map");
    for view in views {
        push_visual_card(html, view);
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_visual_card(html: &mut String, view: &RenderVisualView) {
    html.push_str("<article class=\"visual-card\" aria-labelledby=\"visual-title-");
    push_escaped_attr(html, &view.id);
    html.push_str("\">\n<h3 id=\"visual-title-");
    push_escaped_attr(html, &view.id);
    html.push_str("\">");
    push_escaped(html, &view.title);
    html.push_str("</h3>\n<p class=\"meta\">");
    push_escaped(html, &view.justification);
    html.push_str("</p>\n<div class=\"visual-canvas\" data-visual-mount=\"");
    push_escaped_attr(html, &view.id);
    html.push_str("\" role=\"region\" aria-label=\"Interactive visual view: ");
    push_escaped_attr(html, &view.title);
    html.push_str("\"></div>\n");
    html.push_str("<noscript><p class=\"muted\">The text alternative below contains the same graph nodes and \
        edges.</p></noscript>\n");
    html.push_str(
        "<div class=\"visual-fallback\" role=\"group\" aria-label=\"Text alternative for ",
    );
    push_escaped_attr(html, &view.title);
    html.push_str("\">\n<p>");
    push_escaped(html, &view.alt);
    html.push_str("</p>\n<h4>Nodes</h4>\n<ul>\n");
    for node in &view.nodes {
        html.push_str("<li><strong>");
        push_escaped(html, &node.label);
        html.push_str("</strong> ");
        push_escaped(html, &format!("({})", node.entity_type));
        if !node.description.trim().is_empty() {
            html.push_str(": ");
            push_escaped(html, &node.description);
        }
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n<h4>Edges</h4>\n<ul>\n");
    for edge in &view.edges {
        html.push_str("<li>");
        push_escaped(
            html,
            &format!(
                "{} -> {} ({})",
                edge.from_label,
                edge.to_label,
                first_non_empty([edge.label.as_str(), edge.kind.as_str(), "related"])
            ),
        );
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n");
    push_raw_details(
        html,
        "Raw visual identifiers",
        &[
            ("Visual view ID", view.id.clone()),
            (
                "Relation IDs",
                view.edges
                    .iter()
                    .map(|edge| edge.relation_id.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        ],
    );
    html.push_str("</div>\n</article>\n");
}
