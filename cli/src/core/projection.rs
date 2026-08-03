use super::evidence::{EvidenceClaim, EvidencePackage, EvidenceSource};
use super::knowledge::{
    Confidence, KnowledgeElement, KnowledgePackage, LocalizedText, SemanticRole,
};
use super::pedagogy::{LearningStep, PedagogyPackage, ReadingLadderRow};
use super::relations::{
    is_hard_prerequisite, relation_kind_human_label, Relation, RelationEndpoint,
    RelationEndpointType,
};
use super::validation::validate_core_packages;
use super::*;
use crate::profiles::{self, ProjectionRole};
use anyhow::{bail, Result};

pub fn core_packages_from_report(document: &report::ReportDocument) -> CorePackages {
    CorePackages {
        knowledge: knowledge_from_report(&document.report),
        evidence: evidence_from_report(&document.report),
        pedagogy: pedagogy_from_report(&document.report),
    }
}

pub fn project_report_document(
    document: &report::ReportDocument,
) -> Result<report::ReportDocument> {
    let packages = core_packages_from_report(document);
    project_report_compatibility(&packages, document)
}

pub fn project_report_compatibility(
    packages: &CorePackages,
    context: &report::ReportDocument,
) -> Result<report::ReportDocument> {
    let validation = validate_core_packages(packages);
    if validation.has_errors() {
        bail!(
            "cannot project core packages with {} validation error(s)",
            validation.error_count()
        );
    }

    let mut document = context.clone();
    if !packages.knowledge.field.trim().is_empty() {
        document.report.field = packages.knowledge.field.clone();
    }

    if !packages.knowledge.elements.is_empty() {
        document.report.field_elements = packages
            .knowledge
            .elements
            .iter()
            .map(|element| project_field_element(element, &packages.knowledge.relations))
            .collect();
        let (core_ideas, methods, representations) =
            project_compatibility_knowledge_items(&packages.knowledge);
        document.report.core_ideas = core_ideas;
        document.report.methods = methods;
        document.report.representations = representations;
    }
    document.report.relations = packages
        .knowledge
        .relations
        .iter()
        .map(report_relation_from_core)
        .collect();

    document.report.sources = packages
        .evidence
        .sources
        .iter()
        .map(report::ReportSource::from)
        .collect();
    document.report.claims = packages
        .evidence
        .claims
        .iter()
        .map(report::Claim::from)
        .collect();
    document.report.literature_ladder = packages
        .pedagogy
        .reading_ladder
        .iter()
        .map(report::LiteratureLadderRow::from)
        .collect();
    document.report.curriculum_path = packages
        .pedagogy
        .learning_path
        .iter()
        .map(|step| project_learning_step(step, &packages.knowledge.relations))
        .collect();
    Ok(document)
}

pub fn knowledge_from_report(report: &report::PublicReport) -> KnowledgePackage {
    let relations = super::relations::deduplicate_relations(
        report
            .relations
            .iter()
            .map(core_relation_from_report)
            .collect::<Vec<_>>(),
    );
    KnowledgePackage {
        schema_version: super::KNOWLEDGE_SCHEMA_VERSION.to_string(),
        field: report.field.clone(),
        elements: report
            .field_elements
            .iter()
            .map(|element| knowledge_element_from_field_element(element, &relations))
            .collect(),
        relations,
    }
}

pub fn evidence_from_report(report: &report::PublicReport) -> EvidencePackage {
    EvidencePackage {
        schema_version: super::EVIDENCE_SCHEMA_VERSION.to_string(),
        sources: report.sources.iter().map(EvidenceSource::from).collect(),
        claims: report.claims.iter().map(EvidenceClaim::from).collect(),
    }
}

pub fn pedagogy_from_report(report: &report::PublicReport) -> PedagogyPackage {
    PedagogyPackage {
        schema_version: super::PEDAGOGY_SCHEMA_VERSION.to_string(),
        reading_ladder: report
            .literature_ladder
            .iter()
            .map(ReadingLadderRow::from)
            .collect(),
        learning_path: report
            .curriculum_path
            .iter()
            .map(|step| learning_step_from_report(step, &report.relations))
            .collect(),
    }
}

pub fn project_compatibility_knowledge_items(
    knowledge: &KnowledgePackage,
) -> (
    Vec<report::KnowledgeItem>,
    Vec<report::KnowledgeItem>,
    Vec<report::KnowledgeItem>,
) {
    let mut core_ideas = Vec::new();
    let mut methods = Vec::new();
    let mut representations = Vec::new();

    for element in &knowledge.elements {
        let projection = compatibility_projection_for_element(element);
        if projection.role == ProjectionRole::Omit {
            continue;
        }
        let item = report::KnowledgeItem {
            id: report::content_id(&projection.item_prefix, &[&element.label.text]),
            label: element.label.text.clone(),
            description: compatibility_description(element, &knowledge.relations),
            source_ids: element.source_ids.clone(),
            ..report::KnowledgeItem::default()
        };
        match projection.role {
            ProjectionRole::CoreIdea => core_ideas.push(item),
            ProjectionRole::Method => methods.push(item),
            ProjectionRole::Representation => representations.push(item),
            ProjectionRole::Omit => {}
        }
    }

    (core_ideas, methods, representations)
}

pub fn compatibility_projection_ids(knowledge: &KnowledgePackage) -> BTreeSet<String> {
    let (core_ideas, methods, representations) = project_compatibility_knowledge_items(knowledge);
    core_ideas
        .into_iter()
        .chain(methods)
        .chain(representations)
        .map(|item| item.id)
        .collect()
}

fn knowledge_element_from_field_element(
    element: &report::FieldElement,
    relations: &[Relation],
) -> KnowledgeElement {
    let relation_ids = if element.relation_ids.is_empty() {
        relations
            .iter()
            .filter(|relation| relation_touches_entity(relation, &element.id))
            .map(|relation| relation.id.clone())
            .collect()
    } else {
        element.relation_ids.clone()
    };
    KnowledgeElement {
        id: element.id.clone(),
        element_class: element.element_class.clone(),
        label: LocalizedText::plain(element.label.clone()),
        actual_form: LocalizedText::plain(element.actual_form.clone()),
        semantic_roles: semantic_roles_for_field_element(element),
        role_note: element.role.clone(),
        relation_ids,
        load_bearing_relations: element.load_bearing_relations.clone(),
        source_ids: element.source_ids.clone(),
        confidence: element.confidence.map(Confidence::from),
    }
}

fn project_field_element(
    element: &KnowledgeElement,
    relations: &[Relation],
) -> report::FieldElement {
    let summary = relation_summary_for_element(element, relations);
    report::FieldElement {
        id: element.id.clone(),
        element_class: element.element_class.clone(),
        label: element.label.text.clone(),
        actual_form: element.actual_form.text.clone(),
        role: element.role_note.clone(),
        relation_ids: element.relation_ids.clone(),
        load_bearing_relations: first_non_empty([
            summary.as_str(),
            element.load_bearing_relations.as_str(),
        ]),
        source_ids: element.source_ids.clone(),
        confidence: element.confidence.map(report::ClaimConfidence::from),
    }
}

fn semantic_roles_for_field_element(element: &report::FieldElement) -> Vec<SemanticRole> {
    let mut roles = BTreeSet::new();
    let role_note = report::normalize_id_text(&element.role);
    if role_note.contains("core") {
        roles.insert(SemanticRole::Core);
    }
    if role_note.contains("surrounding") {
        roles.insert(SemanticRole::Surrounding);
    }
    if role_note.contains("context") {
        roles.insert(SemanticRole::Context);
    }
    let projection = profiles::projection_for_element_class(&first_non_empty([
        element.element_class.as_str(),
        element.label.as_str(),
    ]));
    match projection.role {
        ProjectionRole::CoreIdea => {
            roles.insert(SemanticRole::Concept);
        }
        ProjectionRole::Method => {
            roles.insert(SemanticRole::Method);
        }
        ProjectionRole::Representation => {
            roles.insert(SemanticRole::Representation);
        }
        ProjectionRole::Omit => {
            roles.insert(SemanticRole::Other);
        }
    }
    if roles.is_empty() {
        roles.insert(SemanticRole::Other);
    }
    roles.into_iter().collect()
}

fn compatibility_projection_for_element(element: &KnowledgeElement) -> profiles::ProjectionMatch {
    let element_class =
        first_non_empty([element.element_class.as_str(), element.label.text.as_str()]);
    profiles::projection_for_element_class(&element_class)
}

fn compatibility_description(element: &KnowledgeElement, relations: &[Relation]) -> String {
    let relation_summary = relation_summary_for_element(element, relations);
    let relation_text = first_non_empty([
        relation_summary.as_str(),
        element.load_bearing_relations.as_str(),
    ]);
    collapse_whitespace(
        &[
            element.actual_form.text.as_str(),
            element.role_note.as_str(),
            relation_text.as_str(),
        ]
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join("; "),
    )
}

fn project_learning_step(step: &LearningStep, relations: &[Relation]) -> report::CurriculumStep {
    let mut projected = report::CurriculumStep::from(step);
    let prerequisite_ids = prerequisite_ids_for_step(step, relations);
    if !prerequisite_ids.is_empty() {
        projected.prerequisite_ids = prerequisite_ids;
    }
    projected
}

fn learning_step_from_report(
    step: &report::CurriculumStep,
    relations: &[report::Relation],
) -> LearningStep {
    let prerequisite_relation_ids = relations
        .iter()
        .filter(|relation| {
            is_hard_prerequisite(relation.kind)
                && relation.to.entity_type == report::EntityType::CurriculumStep
                && relation.to.id == step.id
                && step.prerequisite_ids.contains(&relation.from.id)
        })
        .map(|relation| relation.id.clone())
        .collect();
    LearningStep {
        prerequisite_relation_ids,
        ..LearningStep::from(step)
    }
}

fn prerequisite_ids_for_step(step: &LearningStep, relations: &[Relation]) -> Vec<String> {
    let relation_lookup = relations
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect::<BTreeMap<_, _>>();
    let mut prerequisite_ids = BTreeSet::new();
    if step.prerequisite_relation_ids.is_empty() {
        for relation in relations {
            if is_hard_prerequisite(relation.kind)
                && relation.to.entity_type == RelationEndpointType::CurriculumStep
                && relation.to.id == step.id
            {
                prerequisite_ids.insert(relation.from.id.clone());
            }
        }
    } else {
        for relation_id in &step.prerequisite_relation_ids {
            if let Some(relation) = relation_lookup.get(relation_id.as_str()) {
                if is_hard_prerequisite(relation.kind)
                    && relation.to.entity_type == RelationEndpointType::CurriculumStep
                    && relation.to.id == step.id
                {
                    prerequisite_ids.insert(relation.from.id.clone());
                }
            }
        }
    }
    if prerequisite_ids.is_empty() {
        step.prerequisite_ids.clone()
    } else {
        prerequisite_ids.into_iter().collect()
    }
}

fn relation_summary_for_element(element: &KnowledgeElement, relations: &[Relation]) -> String {
    let relation_lookup = relations
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect::<BTreeMap<_, _>>();
    let mut summaries = Vec::new();
    for relation_id in &element.relation_ids {
        let Some(relation) = relation_lookup.get(relation_id.as_str()) else {
            continue;
        };
        let from_label = if relation.from.id == element.id {
            element.label.text.as_str()
        } else {
            relation.from.id.as_str()
        };
        let to_label = if relation.to.id == element.id {
            element.label.text.as_str()
        } else {
            relation.to.id.as_str()
        };
        let mut summary = format!(
            "{from_label} {} {to_label}",
            relation_kind_human_label(relation.kind)
        );
        if !relation.description.trim().is_empty() {
            summary.push_str(": ");
            summary.push_str(relation.description.trim());
        }
        summaries.push(summary);
    }
    summaries.join("; ")
}

fn relation_touches_entity(relation: &Relation, entity_id: &str) -> bool {
    relation.from.id == entity_id || relation.to.id == entity_id
}

fn core_relation_from_report(relation: &report::Relation) -> Relation {
    Relation {
        id: relation.id.clone(),
        kind: relation.kind,
        from: core_endpoint_from_report(&relation.from),
        to: core_endpoint_from_report(&relation.to),
        description: relation.description.clone(),
        source_ids: relation.source_ids.clone(),
    }
}

fn report_relation_from_core(relation: &Relation) -> report::Relation {
    report::Relation {
        id: relation.id.clone(),
        kind: relation.kind,
        from: report_endpoint_from_core(&relation.from),
        to: report_endpoint_from_core(&relation.to),
        description: relation.description.clone(),
        source_ids: relation.source_ids.clone(),
    }
}

fn core_endpoint_from_report(endpoint: &report::RelationEndpoint) -> RelationEndpoint {
    RelationEndpoint {
        entity_type: core_endpoint_type_from_report(endpoint.entity_type),
        id: endpoint.id.clone(),
    }
}

fn report_endpoint_from_core(endpoint: &RelationEndpoint) -> report::RelationEndpoint {
    report::RelationEndpoint {
        entity_type: report_endpoint_type_from_core(endpoint.entity_type),
        id: endpoint.id.clone(),
    }
}

fn core_endpoint_type_from_report(entity_type: report::EntityType) -> RelationEndpointType {
    match entity_type {
        report::EntityType::FieldElement => RelationEndpointType::FieldElement,
        report::EntityType::Concept => RelationEndpointType::Concept,
        report::EntityType::Claim => RelationEndpointType::Claim,
        report::EntityType::Source => RelationEndpointType::Source,
        report::EntityType::CurriculumStep => RelationEndpointType::CurriculumStep,
        report::EntityType::FrontierDebate => RelationEndpointType::FrontierDebate,
        report::EntityType::Method => RelationEndpointType::Method,
        report::EntityType::Representation => RelationEndpointType::Representation,
    }
}

fn report_endpoint_type_from_core(entity_type: RelationEndpointType) -> report::EntityType {
    match entity_type {
        RelationEndpointType::FieldElement => report::EntityType::FieldElement,
        RelationEndpointType::Concept => report::EntityType::Concept,
        RelationEndpointType::Claim => report::EntityType::Claim,
        RelationEndpointType::Source => report::EntityType::Source,
        RelationEndpointType::CurriculumStep => report::EntityType::CurriculumStep,
        RelationEndpointType::FrontierDebate => report::EntityType::FrontierDebate,
        RelationEndpointType::Method => report::EntityType::Method,
        RelationEndpointType::Representation => report::EntityType::Representation,
    }
}

fn first_non_empty<const N: usize>(values: [&str; N]) -> String {
    values
        .into_iter()
        .find(|value| !value.trim().is_empty())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn collapse_whitespace(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}
