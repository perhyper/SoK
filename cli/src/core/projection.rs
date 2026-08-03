use super::evidence::{EvidenceClaim, EvidencePackage, EvidenceSource};
use super::knowledge::{
    Confidence, KnowledgeElement, KnowledgePackage, LocalizedText, SemanticRole,
};
use super::pedagogy::{LearningStep, PedagogyPackage, ReadingLadderRow};
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
            .map(project_field_element)
            .collect();
        let (core_ideas, methods, representations) =
            project_compatibility_knowledge_items(&packages.knowledge);
        document.report.core_ideas = core_ideas;
        document.report.methods = methods;
        document.report.representations = representations;
    }

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
        .map(report::CurriculumStep::from)
        .collect();
    Ok(document)
}

pub fn knowledge_from_report(report: &report::PublicReport) -> KnowledgePackage {
    KnowledgePackage {
        schema_version: super::KNOWLEDGE_SCHEMA_VERSION.to_string(),
        field: report.field.clone(),
        elements: report
            .field_elements
            .iter()
            .map(knowledge_element_from_field_element)
            .collect(),
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
            .map(LearningStep::from)
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
            description: compatibility_description(element),
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

fn knowledge_element_from_field_element(element: &report::FieldElement) -> KnowledgeElement {
    KnowledgeElement {
        id: element.id.clone(),
        element_class: element.element_class.clone(),
        label: LocalizedText::plain(element.label.clone()),
        actual_form: LocalizedText::plain(element.actual_form.clone()),
        semantic_roles: semantic_roles_for_field_element(element),
        role_note: element.role.clone(),
        load_bearing_relations: element.load_bearing_relations.clone(),
        source_ids: element.source_ids.clone(),
        confidence: element.confidence.map(Confidence::from),
    }
}

fn project_field_element(element: &KnowledgeElement) -> report::FieldElement {
    report::FieldElement {
        id: element.id.clone(),
        element_class: element.element_class.clone(),
        label: element.label.text.clone(),
        actual_form: element.actual_form.text.clone(),
        role: element.role_note.clone(),
        load_bearing_relations: element.load_bearing_relations.clone(),
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

fn compatibility_description(element: &KnowledgeElement) -> String {
    collapse_whitespace(
        &[
            element.actual_form.text.as_str(),
            element.role_note.as_str(),
            element.load_bearing_relations.as_str(),
        ]
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join("; "),
    )
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
