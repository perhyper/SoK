use super::*;

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
