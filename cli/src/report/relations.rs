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
        let Some(kind_mapping) = parse_relation_kind_mapping_input(&kind_text) else {
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
        let kind = kind_mapping.kind;
        let (from, to) = if kind_mapping.reverse_endpoints {
            (to, from)
        } else {
            (from, to)
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

pub(crate) fn deduplicate_relations(relations: Vec<Relation>) -> Vec<Relation> {
    let mut by_semantic_key = BTreeMap::<RelationSemanticKey, Relation>::new();
    for mut relation in relations {
        relation.source_ids = sorted_unique(relation.source_ids);
        let key = RelationSemanticKey::from(&relation);
        by_semantic_key
            .entry(key)
            .and_modify(|existing| merge_relation(existing, &relation))
            .or_insert(relation);
    }
    by_semantic_key.into_values().collect()
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RelationSemanticKey {
    kind: RelationKind,
    from_type: EntityType,
    from_id: String,
    to_type: EntityType,
    to_id: String,
}

impl From<&Relation> for RelationSemanticKey {
    fn from(relation: &Relation) -> Self {
        Self {
            kind: relation.kind,
            from_type: relation.from.entity_type,
            from_id: relation.from.id.clone(),
            to_type: relation.to.entity_type,
            to_id: relation.to.id.clone(),
        }
    }
}

fn merge_relation(existing: &mut Relation, duplicate: &Relation) {
    if duplicate.id < existing.id {
        existing.id = duplicate.id.clone();
    }
    if !duplicate.description.trim().is_empty()
        && (existing.description.trim().is_empty() || duplicate.description < existing.description)
    {
        existing.description = duplicate.description.clone();
    }
    existing
        .source_ids
        .extend(duplicate.source_ids.iter().cloned());
    existing.source_ids = sorted_unique(std::mem::take(&mut existing.source_ids));
}

fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
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
    parse_relation_kind_mapping_input(raw).map(|mapping| mapping.kind)
}

pub(crate) fn parse_relation_kind_mapping_input(
    raw: &str,
) -> Option<crate::core::relations::RelationKindMapping> {
    crate::core::relations::relation_kind_mapping(raw)
}

pub(crate) fn relation_kind_label(kind: RelationKind) -> &'static str {
    crate::core::relations::relation_kind_label(kind)
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
        if relation.from == relation.to {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_RELATION_ENDPOINT,
                    format!(
                        "relation {} is a self-loop: {}",
                        relation.id,
                        report_endpoint_key(&relation.from)
                    ),
                )
                .with_target(format!("/report/relations/{relation_index}"), &relation.id),
            );
        }
        if crate::core::relations::is_hard_prerequisite(relation.kind)
            && (relation.from.entity_type == EntityType::CurriculumStep
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
    validate_field_element_relation_projection(report, checks);
    validate_curriculum_relation_projection(report, checks);
    if let Some(cycle) = hard_prerequisite_cycle(&report.relations) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_CURRICULUM_REFERENCE,
                format!("hard prerequisite cycle: {}", cycle.join(" -> ")),
            )
            .with_target("/report/relations", ""),
        );
    }
}

fn validate_field_element_relation_projection(
    report: &PublicReport,
    checks: &mut Vec<DiagnosticCheck>,
) {
    let relation_lookup = report
        .relations
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect::<BTreeMap<_, _>>();
    for (element_index, element) in report.field_elements.iter().enumerate() {
        for relation_id in &element.relation_ids {
            let Some(relation) = relation_lookup.get(relation_id.as_str()) else {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_RELATION_ENDPOINT,
                        format!(
                            "field element {} references missing relation {}",
                            element.id, relation_id
                        ),
                    )
                    .with_target(
                        format!("/report/field_elements/{element_index}/relation_ids"),
                        &element.id,
                    ),
                );
                continue;
            };
            let touches_element = [(&relation.from), (&relation.to)]
                .into_iter()
                .any(|endpoint| {
                    endpoint.entity_type == EntityType::FieldElement && endpoint.id == element.id
                });
            if !touches_element {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_RELATION_ENDPOINT,
                        format!(
                            "field element {} relation_id {} does not touch the field element",
                            element.id, relation_id
                        ),
                    )
                    .with_target(
                        format!("/report/field_elements/{element_index}/relation_ids"),
                        &element.id,
                    ),
                );
            }
        }
    }
}

fn validate_curriculum_relation_projection(
    report: &PublicReport,
    checks: &mut Vec<DiagnosticCheck>,
) {
    let hard_relations = report
        .relations
        .iter()
        .filter(|relation| crate::core::relations::is_hard_prerequisite(relation.kind))
        .collect::<Vec<_>>();
    let curriculum_steps = report
        .curriculum_path
        .iter()
        .map(|step| (step.id.as_str(), step))
        .collect::<BTreeMap<_, _>>();

    for (step_index, step) in report.curriculum_path.iter().enumerate() {
        for prerequisite_id in &step.prerequisite_ids {
            let has_relation = hard_relations.iter().any(|relation| {
                relation.to.entity_type == EntityType::CurriculumStep
                    && relation.to.id == step.id
                    && relation.from.id == *prerequisite_id
            });
            if !has_relation {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_CURRICULUM_REFERENCE,
                        format!(
                            "curriculum step {} prerequisite {} has no canonical requires_before relation",
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

    for relation in hard_relations {
        if relation.to.entity_type != EntityType::CurriculumStep {
            continue;
        }
        let Some(step) = curriculum_steps.get(relation.to.id.as_str()) else {
            continue;
        };
        if !step.prerequisite_ids.contains(&relation.from.id) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_CURRICULUM_REFERENCE,
                    format!(
                        "canonical requires_before relation {} is not projected in curriculum step {} prerequisite_ids",
                        relation.id, step.id
                    ),
                )
                .with_target("/report/curriculum_path", &step.id),
            );
        }
    }
}

fn hard_prerequisite_cycle(relations: &[Relation]) -> Option<Vec<String>> {
    let mut graph = BTreeMap::<String, BTreeSet<String>>::new();
    for relation in relations {
        if !crate::core::relations::is_hard_prerequisite(relation.kind)
            || relation.from == relation.to
        {
            continue;
        }
        let from = report_endpoint_key(&relation.from);
        let to = report_endpoint_key(&relation.to);
        graph.entry(from.clone()).or_default().insert(to.clone());
        graph.entry(to).or_default();
    }

    let mut visited = BTreeSet::new();
    let mut stack = Vec::<String>::new();
    let mut in_stack = BTreeSet::new();
    for node in graph.keys() {
        if visited.contains(node) {
            continue;
        }
        if let Some(cycle) =
            hard_prerequisite_cycle_from(node, &graph, &mut visited, &mut stack, &mut in_stack)
        {
            return Some(cycle);
        }
    }
    None
}

fn hard_prerequisite_cycle_from(
    node: &str,
    graph: &BTreeMap<String, BTreeSet<String>>,
    visited: &mut BTreeSet<String>,
    stack: &mut Vec<String>,
    in_stack: &mut BTreeSet<String>,
) -> Option<Vec<String>> {
    visited.insert(node.to_string());
    stack.push(node.to_string());
    in_stack.insert(node.to_string());

    if let Some(next_nodes) = graph.get(node) {
        for next in next_nodes {
            if !visited.contains(next.as_str()) {
                if let Some(cycle) =
                    hard_prerequisite_cycle_from(next, graph, visited, stack, in_stack)
                {
                    return Some(cycle);
                }
            } else if in_stack.contains(next.as_str()) {
                if let Some(position) = stack.iter().position(|item| item == next) {
                    let mut cycle = stack[position..].to_vec();
                    cycle.push(next.clone());
                    return Some(cycle);
                }
            }
        }
    }

    in_stack.remove(node);
    stack.pop();
    None
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
                if let Some(edge_kind) = parse_relation_kind_input(&edge.kind) {
                    if edge_kind != relation.kind {
                        edge_is_valid = false;
                        checks.push(
                            DiagnosticCheck::error(
                                CHECK_VALIDATE_VISUAL_REFERENCE,
                                format!(
                                    "visual view {} edge {} -> {} kind {} disagrees with relation {} kind {}",
                                    view.id,
                                    edge.from,
                                    edge.to,
                                    edge.kind,
                                    edge.relation_id,
                                    relation_kind_label(relation.kind)
                                ),
                            )
                            .with_target(format!("{edge_path}/kind"), &view.id),
                        );
                    }
                } else {
                    edge_is_valid = false;
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_VISUAL_REFERENCE,
                            format!(
                                "visual view {} edge {} -> {} has unsupported relation kind {}",
                                view.id, edge.from, edge.to, edge.kind
                            ),
                        )
                        .with_target(format!("{edge_path}/kind"), &view.id),
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
    visual_endpoint_matches_node(&relation.from, from_node)
        && visual_endpoint_matches_node(&relation.to, to_node)
}

pub(crate) fn visual_endpoint_matches_node(
    endpoint: &RelationEndpoint,
    node: &VisualViewNode,
) -> bool {
    endpoint.entity_type == node.entity_type && endpoint.id == node.ref_id
}

fn report_endpoint_key(endpoint: &RelationEndpoint) -> String {
    format!(
        "{}:{}",
        entity_type_label(endpoint.entity_type),
        endpoint.id
    )
}
