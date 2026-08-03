use super::html::visual_view_kind_label;
use super::ids::*;
use super::import::{lookup_cell_any, ParsedMarkdownReport, SourceLookup};
use super::model::*;
use super::provenance::{first_non_empty, normalize_access_status_label};
use super::validation::*;
use super::*;

pub(crate) fn build_relations(
    parsed: &ParsedMarkdownReport,
    entity_index: &ExportEntityIndex,
    source_lookup: &SourceLookup,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<Relation> {
    let mut relations = Vec::new();
    let mut seen_ids = BTreeSet::new();
    for (index, row) in parsed.relation_rows.iter().enumerate() {
        let kind_text = lookup_cell_any(row, &["Relation kind", "Kind", "Type"]);
        let Some(kind) = parse_relation_kind_input(&kind_text) else {
            diagnostics.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_MISSING_PUBLIC_FIELD,
                    format!(
                        "Relations row {} has unsupported or missing relation kind {:?}",
                        index + 1,
                        kind_text
                    ),
                )
                .with_target(format!("/report/relations/{index}/kind"), ""),
            );
            continue;
        };

        let Some(from) = resolve_relation_endpoint_from_row(
            row,
            index,
            "from",
            &["From type", "Source type", "Subject type"],
            &[
                "From reference",
                "From ref",
                "From",
                "Source reference",
                "Subject",
            ],
            entity_index,
            diagnostics,
        ) else {
            continue;
        };
        let Some(to) = resolve_relation_endpoint_from_row(
            row,
            index,
            "to",
            &["To type", "Target type", "Object type"],
            &["To reference", "To ref", "To", "Target reference", "Object"],
            entity_index,
            diagnostics,
        ) else {
            continue;
        };

        let explicit_id = lookup_cell_any(row, &["Relation ID", "Relation id", "ID"]);
        let id = if explicit_id.trim().is_empty() {
            content_id(
                "rel",
                &[
                    relation_kind_label(kind),
                    entity_type_label(from.entity_type),
                    &from.id,
                    entity_type_label(to.entity_type),
                    &to.id,
                ],
            )
        } else if is_stable_id(&explicit_id) {
            explicit_id
        } else {
            content_id("rel", &[&explicit_id])
        };
        if !seen_ids.insert(id.clone()) {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_AMBIGUOUS_REFERENCE,
                    format!("Relations row {} repeats relation id {}", index + 1, id),
                )
                .with_target(format!("/report/relations/{index}/id"), &id),
            );
            continue;
        }

        let source_refs = lookup_cell_any(
            row,
            &[
                "Source IDs",
                "Source ids",
                "Sources",
                "Evidence sources",
                "Source references",
            ],
        );
        let source_ids = source_lookup.resolve_refs_with_diagnostics(
            &source_refs,
            format!("/report/relations/{index}/source_ids"),
            &id,
            diagnostics,
        );

        relations.push(Relation {
            id,
            kind,
            from,
            to,
            description: lookup_cell_any(row, &["Rationale", "Description", "Why"]),
            source_ids,
        });
    }
    relations
}

pub(crate) fn resolve_relation_endpoint_from_row(
    row: &BTreeMap<String, String>,
    row_index: usize,
    endpoint_name: &str,
    type_keys: &[&str],
    reference_keys: &[&str],
    entity_index: &ExportEntityIndex,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Option<RelationEndpoint> {
    let type_text = lookup_cell_any(row, type_keys);
    let reference = lookup_cell_any(row, reference_keys);
    let Some(entity_type) = parse_entity_type_input(&type_text) else {
        diagnostics.push(
            DiagnosticCheck::warning(
                CHECK_EXPORT_MISSING_PUBLIC_FIELD,
                format!(
                    "Relations row {} has unsupported or missing {endpoint_name} entity type {:?}",
                    row_index + 1,
                    type_text
                ),
            )
            .with_target(
                format!("/report/relations/{row_index}/{endpoint_name}/entity_type"),
                "",
            ),
        );
        return None;
    };
    if reference.trim().is_empty() {
        diagnostics.push(
            DiagnosticCheck::warning(
                CHECK_EXPORT_MISSING_PUBLIC_FIELD,
                format!(
                    "Relations row {} is missing {endpoint_name} reference",
                    row_index + 1
                ),
            )
            .with_target(
                format!("/report/relations/{row_index}/{endpoint_name}/id"),
                "",
            ),
        );
        return None;
    }
    match entity_index.resolve_typed(entity_type, &reference) {
        ExportReferenceResolution::Resolved(entity) => Some(RelationEndpoint {
            entity_type,
            id: entity.id,
        }),
        ExportReferenceResolution::Missing => {
            let type_hint = match entity_index.resolve_any(&reference) {
                ExportReferenceResolution::Resolved(candidate)
                    if candidate.entity_type != entity_type =>
                {
                    format!(
                        "; the field-element inventory exports this reference as {}, so revise the declared endpoint type if that classification is intended",
                        entity_type_label(candidate.entity_type)
                    )
                }
                _ => String::new(),
            };
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_UNRESOLVED_REFERENCE,
                    format!(
                        "Relations row {} could not resolve {endpoint_name} endpoint {}:{:?}{type_hint}",
                        row_index + 1,
                        entity_type_label(entity_type),
                        reference
                    ),
                )
                .with_target(
                    format!("/report/relations/{row_index}/{endpoint_name}/id"),
                    "",
                ),
            );
            None
        }
        ExportReferenceResolution::Ambiguous(candidates) => {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_AMBIGUOUS_REFERENCE,
                    format!(
                        "Relations row {} {endpoint_name} endpoint {:?} is ambiguous for {}: {}",
                        row_index + 1,
                        reference,
                        entity_type_label(entity_type),
                        candidates.join(", ")
                    ),
                )
                .with_target(
                    format!("/report/relations/{row_index}/{endpoint_name}/id"),
                    "",
                ),
            );
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExportEntityRef {
    pub(crate) entity_type: EntityType,
    pub(crate) id: String,
    pub(crate) label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExportReferenceResolution {
    Resolved(ExportEntityRef),
    Missing,
    Ambiguous(Vec<String>),
}

#[derive(Debug, Default)]
pub(crate) struct ExportEntityIndex {
    by_exact_id: BTreeMap<String, ExportEntityRef>,
    aliases_by_type: BTreeMap<EntityType, BTreeMap<String, BTreeSet<String>>>,
    aliases_all: BTreeMap<String, BTreeSet<String>>,
}

impl ExportEntityIndex {
    pub(crate) fn new(
        core_ideas: &[KnowledgeItem],
        methods: &[KnowledgeItem],
        representations: &[KnowledgeItem],
        sources: &[ReportSource],
        claims: &[Claim],
        curriculum_path: &[CurriculumStep],
        frontier_debates: &[FrontierDebateItem],
    ) -> Self {
        let mut index = Self::default();
        for item in core_ideas {
            index.insert(
                EntityType::Concept,
                &item.id,
                &item.label,
                [&item.id, &item.label],
            );
            for alias in &item.aliases {
                index.insert_alias(EntityType::Concept, &item.id, alias);
            }
        }
        for item in methods {
            index.insert(
                EntityType::Method,
                &item.id,
                &item.label,
                [&item.id, &item.label],
            );
            for alias in &item.aliases {
                index.insert_alias(EntityType::Method, &item.id, alias);
            }
        }
        for item in representations {
            index.insert(
                EntityType::Representation,
                &item.id,
                &item.label,
                [&item.id, &item.label],
            );
            for alias in &item.aliases {
                index.insert_alias(EntityType::Representation, &item.id, alias);
            }
        }
        for (source_index, source) in sources.iter().enumerate() {
            index.insert(
                EntityType::Source,
                &source.id,
                &first_non_empty([source.title.as_str(), source.citation.as_str(), &source.id]),
                [
                    source.id.as_str(),
                    source.title.as_str(),
                    source.citation.as_str(),
                    source.identifier.as_str(),
                    source.url.as_str(),
                ],
            );
            index.insert_alias(
                EntityType::Source,
                &source.id,
                &format!("S{}", source_index + 1),
            );
        }
        for claim in claims {
            index.insert(
                EntityType::Claim,
                &claim.id,
                &claim.statement,
                [&claim.id, &claim.statement],
            );
        }
        for step in curriculum_path {
            index.insert(
                EntityType::CurriculumStep,
                &step.id,
                &step.title,
                [&step.id, &step.title],
            );
        }
        for item in frontier_debates {
            index.insert(
                EntityType::FrontierDebate,
                &item.id,
                &item.title,
                [&item.id, &item.title],
            );
        }
        index
    }

    pub(crate) fn add_field_elements(&mut self, field_elements: &[FieldElement]) {
        for item in field_elements {
            self.insert(
                EntityType::FieldElement,
                &item.id,
                &item.label,
                [&item.id, &item.label],
            );
        }
    }

    fn insert<const N: usize>(
        &mut self,
        entity_type: EntityType,
        id: &str,
        label: &str,
        aliases: [&str; N],
    ) {
        if id.trim().is_empty() {
            return;
        }
        let entity = ExportEntityRef {
            entity_type,
            id: id.to_string(),
            label: first_non_empty([label, id]),
        };
        self.by_exact_id.insert(id.to_string(), entity);
        for alias in aliases {
            self.insert_alias(entity_type, id, alias);
        }
    }

    fn insert_alias(&mut self, entity_type: EntityType, id: &str, alias: &str) {
        let normalized = normalize_alias_text(alias);
        if normalized.is_empty() {
            return;
        }
        self.aliases_by_type
            .entry(entity_type)
            .or_default()
            .entry(normalized.clone())
            .or_default()
            .insert(id.to_string());
        self.aliases_all
            .entry(normalized)
            .or_default()
            .insert(id.to_string());
    }

    pub(crate) fn resolve_typed(
        &self,
        entity_type: EntityType,
        reference: &str,
    ) -> ExportReferenceResolution {
        let reference = reference.trim();
        if reference.is_empty() {
            return ExportReferenceResolution::Missing;
        }
        if let Some(entity) = self.by_exact_id.get(reference) {
            if entity.entity_type == entity_type {
                return ExportReferenceResolution::Resolved(entity.clone());
            }
            return ExportReferenceResolution::Missing;
        }
        let normalized = normalize_alias_text(reference);
        let Some(candidates) = self
            .aliases_by_type
            .get(&entity_type)
            .and_then(|aliases| aliases.get(&normalized))
        else {
            return ExportReferenceResolution::Missing;
        };
        self.resolve_candidate_set(candidates)
    }

    pub(crate) fn resolve_any(&self, reference: &str) -> ExportReferenceResolution {
        let reference = reference.trim();
        if reference.is_empty() {
            return ExportReferenceResolution::Missing;
        }
        if let Some(entity) = self.by_exact_id.get(reference) {
            return ExportReferenceResolution::Resolved(entity.clone());
        }
        let normalized = normalize_alias_text(reference);
        let Some(candidates) = self.aliases_all.get(&normalized) else {
            return ExportReferenceResolution::Missing;
        };
        self.resolve_candidate_set(candidates)
    }

    pub(crate) fn resolve_any_legacy_projection(
        &self,
        reference: &str,
    ) -> ExportReferenceResolution {
        let reference = reference.trim();
        if reference.is_empty() {
            return ExportReferenceResolution::Missing;
        }
        if let Some(entity) = self.by_exact_id.get(reference) {
            return ExportReferenceResolution::Resolved(entity.clone());
        }
        let normalized = normalize_alias_text(reference);
        let Some(candidates) = self.aliases_all.get(&normalized) else {
            return ExportReferenceResolution::Missing;
        };
        let legacy_candidates = candidates
            .iter()
            .filter(|id| {
                self.by_exact_id
                    .get(*id)
                    .map(|entity| entity.entity_type != EntityType::FieldElement)
                    .unwrap_or(false)
            })
            .cloned()
            .collect::<BTreeSet<_>>();
        if legacy_candidates.is_empty() {
            self.resolve_candidate_set(candidates)
        } else {
            self.resolve_candidate_set_without_field_preference(&legacy_candidates)
        }
    }

    fn resolve_candidate_set(&self, candidates: &BTreeSet<String>) -> ExportReferenceResolution {
        if candidates.len() == 1 {
            let id = candidates
                .iter()
                .next()
                .expect("candidate set is not empty");
            self.by_exact_id
                .get(id)
                .cloned()
                .map(ExportReferenceResolution::Resolved)
                .unwrap_or(ExportReferenceResolution::Missing)
        } else {
            let field_elements = candidates
                .iter()
                .filter_map(|id| self.by_exact_id.get(id))
                .filter(|entity| entity.entity_type == EntityType::FieldElement)
                .collect::<Vec<_>>();
            if field_elements.len() == 1 {
                let field_element = field_elements[0];
                let field_label = normalize_alias_text(&field_element.label);
                let only_derived_projections = candidates
                    .iter()
                    .filter_map(|id| self.by_exact_id.get(id))
                    .filter(|entity| entity.id != field_element.id)
                    .all(|entity| {
                        matches!(
                            entity.entity_type,
                            EntityType::Concept | EntityType::Method | EntityType::Representation
                        ) && normalize_alias_text(&entity.label) == field_label
                    });
                if only_derived_projections {
                    return ExportReferenceResolution::Resolved(field_element.clone());
                }
            }
            self.resolve_candidate_set_without_field_preference(candidates)
        }
    }

    fn resolve_candidate_set_without_field_preference(
        &self,
        candidates: &BTreeSet<String>,
    ) -> ExportReferenceResolution {
        if candidates.len() == 1 {
            let id = candidates
                .iter()
                .next()
                .expect("candidate set is not empty");
            self.by_exact_id
                .get(id)
                .cloned()
                .map(ExportReferenceResolution::Resolved)
                .unwrap_or(ExportReferenceResolution::Missing)
        } else {
            ExportReferenceResolution::Ambiguous(candidates.iter().cloned().collect())
        }
    }

    pub(crate) fn record(&self, id: &str) -> Option<&ExportEntityRef> {
        self.by_exact_id.get(id)
    }
}

pub(crate) fn parse_entity_type_input(raw: &str) -> Option<EntityType> {
    match normalize_access_status_label(raw).as_str() {
        "field_element" | "field_elements" | "element" | "elements" => {
            Some(EntityType::FieldElement)
        }
        "concept" | "core_idea" | "core_ideas" | "idea" | "threshold_concept" => {
            Some(EntityType::Concept)
        }
        "claim" | "claims" => Some(EntityType::Claim),
        "source" | "sources" | "src" | "reading" => Some(EntityType::Source),
        "curriculum_step" | "curriculum" | "step" | "module" => Some(EntityType::CurriculumStep),
        "frontier_debate" | "frontier" | "debate" | "open_problem" => {
            Some(EntityType::FrontierDebate)
        }
        "method" | "methods" | "warrant" | "warrants" => Some(EntityType::Method),
        "representation" | "representations" | "rep" => Some(EntityType::Representation),
        _ => None,
    }
}

pub(crate) fn parse_relation_kind_input(raw: &str) -> Option<RelationKind> {
    match normalize_access_status_label(raw).as_str() {
        "depends_on" | "depends" | "requires" | "prerequisite" | "prerequisite_for" => {
            Some(RelationKind::DependsOn)
        }
        "supports" | "support" | "supported_by" => Some(RelationKind::Supports),
        "qualifies" | "qualify" | "limits" | "conditions" => Some(RelationKind::Qualifies),
        "contradicts" | "contradict" | "conflicts" => Some(RelationKind::Contradicts),
        "precedes" | "precedes_in_curriculum" | "before" | "prior_to" => {
            Some(RelationKind::Precedes)
        }
        "introduces" | "introduce" => Some(RelationKind::Introduces),
        "uses_method" | "uses" | "uses_method_or_warrant" => Some(RelationKind::UsesMethod),
        "represented_by" | "represented" | "has_representation" => {
            Some(RelationKind::RepresentedBy)
        }
        "grounds" | "grounded_by" => Some(RelationKind::Grounds),
        "motivates" | "motivation" => Some(RelationKind::Motivates),
        "part_of" | "contains" | "component_of" => Some(RelationKind::PartOf),
        "maps_to" | "maps" | "corresponds_to" => Some(RelationKind::MapsTo),
        _ => None,
    }
}

pub(crate) fn relation_kind_label(kind: RelationKind) -> &'static str {
    match kind {
        RelationKind::DependsOn => "depends_on",
        RelationKind::Supports => "supports",
        RelationKind::Qualifies => "qualifies",
        RelationKind::Contradicts => "contradicts",
        RelationKind::Precedes => "precedes",
        RelationKind::Introduces => "introduces",
        RelationKind::UsesMethod => "uses_method",
        RelationKind::RepresentedBy => "represented_by",
        RelationKind::Grounds => "grounds",
        RelationKind::Motivates => "motivates",
        RelationKind::PartOf => "part_of",
        RelationKind::MapsTo => "maps_to",
    }
}

pub(crate) fn parse_visual_view_kind_input(raw: &str) -> VisualViewKind {
    match normalize_access_status_label(raw).as_str() {
        "knowledge_spine" | "spine" => VisualViewKind::KnowledgeSpine,
        "concept_source" | "source_map" | "concept_source_map" => VisualViewKind::ConceptSource,
        "dependency_path" | "curriculum_path" | "prerequisite_path" => {
            VisualViewKind::DependencyPath
        }
        "frontier_debate" | "frontier_debate_map" | "debate_map" => VisualViewKind::FrontierDebate,
        _ => VisualViewKind::Custom,
    }
}

pub(crate) fn visual_view_kind_export_label(kind: VisualViewKind) -> &'static str {
    match kind {
        VisualViewKind::KnowledgeSpine => "knowledge_spine",
        VisualViewKind::ConceptSource => "concept_source",
        VisualViewKind::DependencyPath => "dependency_path",
        VisualViewKind::FrontierDebate => "frontier_debate",
        VisualViewKind::Custom => "custom",
    }
}

pub(crate) fn validate_relation_consistency(
    report: &PublicReport,
    index: &ReportIdIndex,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for (relation_index, relation) in report.relations.iter().enumerate() {
        validate_relation_endpoint_consistency(
            relation,
            "from",
            &relation.from,
            relation_index,
            index,
            checks,
        );
        validate_relation_endpoint_consistency(
            relation,
            "to",
            &relation.to,
            relation_index,
            index,
            checks,
        );
        if matches!(
            relation.kind,
            RelationKind::DependsOn | RelationKind::Precedes
        ) && (relation.from.entity_type == EntityType::CurriculumStep
            || relation.to.entity_type == EntityType::CurriculumStep)
            && (!index.has_entity(relation.from.entity_type, &relation.from.id)
                || !index.has_entity(relation.to.entity_type, &relation.to.id))
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_CURRICULUM_REFERENCE,
                    format!(
                        "dependency/curriculum relation {} references a missing endpoint",
                        relation.id
                    ),
                )
                .with_target(format!("/report/relations/{relation_index}"), &relation.id),
            );
        }
    }
}

pub(crate) fn validate_relation_endpoint_consistency(
    relation: &Relation,
    endpoint_name: &str,
    endpoint: &RelationEndpoint,
    relation_index: usize,
    index: &ReportIdIndex,
    checks: &mut Vec<DiagnosticCheck>,
) {
    if !index.has_entity(endpoint.entity_type, &endpoint.id) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_RELATION_ENDPOINT,
                format!(
                    "relation {} {} endpoint references missing {}:{}",
                    relation.id,
                    endpoint_name,
                    entity_type_label(endpoint.entity_type),
                    endpoint.id
                ),
            )
            .with_target(
                format!("/report/relations/{relation_index}/{endpoint_name}"),
                &relation.id,
            ),
        );
    }
}

pub(crate) fn validate_visual_references(
    report: &PublicReport,
    index: &ReportIdIndex,
    checks: &mut Vec<DiagnosticCheck>,
) {
    let relation_lookup = report
        .relations
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect::<BTreeMap<_, _>>();
    let mut view_ids = BTreeSet::new();

    for (view_index, view) in report.visual_views.iter().enumerate() {
        let view_path = format!("/report/visual_views/{view_index}");
        if !is_stable_id(&view.id) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_VISUAL_REFERENCE,
                    format!("visual view id is not stable: {:?}", view.id),
                )
                .with_target(format!("{view_path}/id"), &view.id),
            );
        }
        if !view_ids.insert(view.id.as_str()) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_VISUAL_REFERENCE,
                    format!("duplicate visual view id {}", view.id),
                )
                .with_target(format!("{view_path}/id"), &view.id),
            );
        }
        if visual_view_kind_label(view.kind).is_none() {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_VISUAL_REFERENCE,
                    format!("visual view {} has unsupported kind custom", view.id),
                )
                .with_target(format!("{view_path}/kind"), &view.id),
            );
        }
        if view.justification.trim().is_empty() {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_VISUAL_REFERENCE,
                    format!("visual view {} requires a non-empty justification", view.id),
                )
                .with_target(format!("{view_path}/justification"), &view.id),
            );
        }
        if view.nodes.len() < 2 {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_VISUAL_REFERENCE,
                    format!("visual view {} requires at least two nodes", view.id),
                )
                .with_target(format!("{view_path}/nodes"), &view.id),
            );
        }

        let mut node_lookup = BTreeMap::new();
        let mut duplicate_node_ids = BTreeSet::new();
        for (node_index, node) in view.nodes.iter().enumerate() {
            let node_path = format!("{view_path}/nodes/{node_index}");
            if !is_stable_id(&node.id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!(
                            "visual view {} node id is not stable: {:?}",
                            view.id, node.id
                        ),
                    )
                    .with_target(format!("{node_path}/id"), &view.id),
                );
            }
            if node_lookup.insert(node.id.as_str(), node).is_some() {
                duplicate_node_ids.insert(node.id.as_str());
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!("visual view {} has duplicate node id {}", view.id, node.id),
                    )
                    .with_target(format!("{node_path}/id"), &view.id),
                );
            }
            if node.ref_id.trim().is_empty() {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!(
                            "visual view {} node {} requires an entity ref_id",
                            view.id, node.id
                        ),
                    )
                    .with_target(format!("{node_path}/ref_id"), &view.id),
                );
            } else if !index.has_entity(node.entity_type, &node.ref_id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!(
                            "visual view {} node {} references missing {}:{}",
                            view.id,
                            node.id,
                            entity_type_label(node.entity_type),
                            node.ref_id
                        ),
                    )
                    .with_target(format!("{node_path}/ref_id"), &view.id),
                );
            }
        }

        let mut valid_edge_count = 0;
        for (edge_index, edge) in view.edges.iter().enumerate() {
            let edge_path = format!("{view_path}/edges/{edge_index}");
            let mut edge_is_valid = true;
            for (endpoint_name, endpoint) in [("from", &edge.from), ("to", &edge.to)] {
                if !node_lookup.contains_key(endpoint.as_str()) {
                    edge_is_valid = false;
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_VISUAL_REFERENCE,
                            format!(
                                "visual view {} edge references missing node {}",
                                view.id, endpoint
                            ),
                        )
                        .with_target(format!("{edge_path}/{endpoint_name}"), &view.id),
                    );
                } else if duplicate_node_ids.contains(endpoint.as_str()) {
                    edge_is_valid = false;
                }
            }
            if edge.from == edge.to {
                edge_is_valid = false;
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!(
                            "visual view {} edge {} -> {} is a self-edge",
                            view.id, edge.from, edge.to
                        ),
                    )
                    .with_target(&edge_path, &view.id),
                );
            }

            let relation = if edge.relation_id.trim().is_empty() {
                edge_is_valid = false;
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!(
                            "visual view {} edge {} -> {} requires a relation_id",
                            view.id, edge.from, edge.to
                        ),
                    )
                    .with_target(format!("{edge_path}/relation_id"), &view.id),
                );
                None
            } else if !index.relations.contains(&edge.relation_id) {
                edge_is_valid = false;
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!(
                            "visual view {} edge references missing relation {}",
                            view.id, edge.relation_id
                        ),
                    )
                    .with_target(format!("{edge_path}/relation_id"), &view.id),
                );
                None
            } else {
                relation_lookup.get(edge.relation_id.as_str()).copied()
            };

            if let (Some(relation), Some(from_node), Some(to_node)) = (
                relation,
                node_lookup.get(edge.from.as_str()).copied(),
                node_lookup.get(edge.to.as_str()).copied(),
            ) {
                let endpoints_are_valid = [from_node, to_node].into_iter().all(|node| {
                    is_stable_id(&node.id)
                        && !duplicate_node_ids.contains(node.id.as_str())
                        && !node.ref_id.trim().is_empty()
                        && index.has_entity(node.entity_type, &node.ref_id)
                });
                if !endpoints_are_valid {
                    edge_is_valid = false;
                } else if !visual_relation_matches_nodes(relation, from_node, to_node) {
                    edge_is_valid = false;
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_VISUAL_REFERENCE,
                            format!(
                                "visual view {} edge {} -> {} relation {} does not match the endpoint entity references",
                                view.id, edge.from, edge.to, edge.relation_id
                            ),
                        )
                        .with_target(format!("{edge_path}/relation_id"), &view.id),
                    );
                }
            } else {
                edge_is_valid = false;
            }

            if edge_is_valid {
                valid_edge_count += 1;
            }
        }
        if valid_edge_count == 0 {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_VISUAL_REFERENCE,
                    format!(
                        "visual view {} requires at least one valid relation-backed edge",
                        view.id
                    ),
                )
                .with_target(format!("{view_path}/edges"), &view.id),
            );
        }
    }
}

pub(crate) fn visual_relation_matches_nodes(
    relation: &Relation,
    from_node: &VisualViewNode,
    to_node: &VisualViewNode,
) -> bool {
    (visual_endpoint_matches_node(&relation.from, from_node)
        && visual_endpoint_matches_node(&relation.to, to_node))
        || (visual_endpoint_matches_node(&relation.from, to_node)
            && visual_endpoint_matches_node(&relation.to, from_node))
}

pub(crate) fn visual_endpoint_matches_node(
    endpoint: &RelationEndpoint,
    node: &VisualViewNode,
) -> bool {
    endpoint.entity_type == node.entity_type && endpoint.id == node.ref_id
}
