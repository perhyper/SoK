use super::*;

pub const CHECK_CORE_VERSION: &str = "core.version";
pub const CHECK_CORE_REQUIRED: &str = "core.required";
pub const CHECK_CORE_REFERENCE: &str = "core.reference";
pub const CHECK_CORE_SUPPORT: &str = "core.support";
pub const CHECK_CORE_RELATION: &str = "core.relation";
pub const CHECK_CORE_PREREQUISITE: &str = "core.prerequisite";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreValidation {
    pub diagnostics: report::Diagnostics,
}

impl CoreValidation {
    pub fn new(mut checks: Vec<report::DiagnosticCheck>) -> Self {
        sort_diagnostics(&mut checks);
        let error_count = checks
            .iter()
            .filter(|check| check.severity == report::DiagnosticSeverity::Error)
            .count();
        let warning_count = checks
            .iter()
            .filter(|check| check.severity == report::DiagnosticSeverity::Warning)
            .count();
        Self {
            diagnostics: report::Diagnostics {
                summary: format!(
                    "core validation produced {error_count} error(s) and {warning_count} warning(s)"
                ),
                checks,
            },
        }
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .checks
            .iter()
            .filter(|check| check.severity == report::DiagnosticSeverity::Error)
            .count()
    }

    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }
}

pub fn validate_core_packages(packages: &CorePackages) -> CoreValidation {
    let mut checks = Vec::new();
    validate_versions(packages, &mut checks);

    let source_ids = packages
        .evidence
        .sources
        .iter()
        .map(|source| source.id.as_str())
        .collect::<BTreeSet<_>>();
    let claim_ids = packages
        .evidence
        .claims
        .iter()
        .map(|claim| claim.id.as_str())
        .collect::<BTreeSet<_>>();
    let element_ids = packages
        .knowledge
        .elements
        .iter()
        .map(|element| element.id.as_str())
        .collect::<BTreeSet<_>>();
    let learning_step_ids = packages
        .pedagogy
        .learning_path
        .iter()
        .map(|step| step.id.as_str())
        .collect::<BTreeSet<_>>();
    let compatibility_ids = projection::compatibility_projection_ids(&packages.knowledge);
    let relation_ids = packages
        .knowledge
        .relations
        .iter()
        .map(|relation| relation.id.as_str())
        .collect::<BTreeSet<_>>();

    validate_knowledge_package(
        &packages.knowledge,
        &source_ids,
        &relation_ids,
        &packages.knowledge.relations,
        &mut checks,
    );
    validate_evidence_package(&packages.evidence, &source_ids, &claim_ids, &mut checks);
    validate_relation_graph(
        &packages.knowledge.relations,
        &source_ids,
        &claim_ids,
        &element_ids,
        &learning_step_ids,
        &compatibility_ids,
        &mut checks,
    );
    validate_pedagogy_package(
        &packages.pedagogy,
        &source_ids,
        &element_ids,
        &learning_step_ids,
        &compatibility_ids,
        &packages.knowledge.relations,
        &mut checks,
    );

    CoreValidation::new(checks)
}

fn validate_versions(packages: &CorePackages, checks: &mut Vec<report::DiagnosticCheck>) {
    for (path, actual, expected) in [
        (
            "/knowledge/schema_version",
            packages.knowledge.schema_version.as_str(),
            KNOWLEDGE_SCHEMA_VERSION,
        ),
        (
            "/evidence/schema_version",
            packages.evidence.schema_version.as_str(),
            EVIDENCE_SCHEMA_VERSION,
        ),
        (
            "/pedagogy/schema_version",
            packages.pedagogy.schema_version.as_str(),
            PEDAGOGY_SCHEMA_VERSION,
        ),
    ] {
        if actual != expected {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_VERSION,
                    format!("unsupported core package version {actual:?}; expected {expected}"),
                )
                .with_target(path, ""),
            );
        }
    }
}

fn validate_knowledge_package(
    package: &knowledge::KnowledgePackage,
    source_ids: &BTreeSet<&str>,
    relation_ids: &BTreeSet<&str>,
    relations: &[relations::Relation],
    checks: &mut Vec<report::DiagnosticCheck>,
) {
    let relation_lookup = relations
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect::<BTreeMap<_, _>>();
    let mut element_ids = BTreeSet::new();
    for (index, element) in package.elements.iter().enumerate() {
        let path = format!("/knowledge/elements/{index}");
        validate_stable_id(&element.id, &format!("{path}/id"), checks);
        if !element_ids.insert(element.id.as_str()) {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_REQUIRED,
                    format!("duplicate knowledge element id {}", element.id),
                )
                .with_target(format!("{path}/id"), &element.id),
            );
        }
        for (field, value) in [
            ("element_class", element.element_class.as_str()),
            ("label/text", element.label.text.as_str()),
            ("actual_form/text", element.actual_form.text.as_str()),
        ] {
            require_non_empty(value, &format!("{path}/{field}"), &element.id, checks);
        }
        for source_id in &element.source_ids {
            if !source_ids.contains(source_id.as_str()) {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_CORE_REFERENCE,
                        format!(
                            "knowledge element {} references missing source {}",
                            element.id, source_id
                        ),
                    )
                    .with_target(format!("{path}/source_ids"), &element.id),
                );
            }
        }
        for relation_id in &element.relation_ids {
            if !relation_ids.contains(relation_id.as_str()) {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_CORE_REFERENCE,
                        format!(
                            "knowledge element {} references missing relation {}",
                            element.id, relation_id
                        ),
                    )
                    .with_target(format!("{path}/relation_ids"), &element.id),
                );
            } else if relation_lookup
                .get(relation_id.as_str())
                .map(|relation| relation.from.id != element.id && relation.to.id != element.id)
                .unwrap_or(false)
            {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_CORE_RELATION,
                        format!(
                            "knowledge element {} relation_id {} does not touch the element",
                            element.id, relation_id
                        ),
                    )
                    .with_target(format!("{path}/relation_ids"), &element.id),
                );
            }
        }
    }
}

fn validate_relation_graph(
    relations: &[relations::Relation],
    source_ids: &BTreeSet<&str>,
    claim_ids: &BTreeSet<&str>,
    element_ids: &BTreeSet<&str>,
    learning_step_ids: &BTreeSet<&str>,
    compatibility_ids: &BTreeSet<String>,
    checks: &mut Vec<report::DiagnosticCheck>,
) {
    let mut seen_ids = BTreeSet::new();
    let mut seen_semantics = BTreeSet::new();
    for (index, relation) in relations.iter().enumerate() {
        let path = format!("/knowledge/relations/{index}");
        validate_stable_id(&relation.id, &format!("{path}/id"), checks);
        if !seen_ids.insert(relation.id.as_str()) {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_RELATION,
                    format!("duplicate relation id {}", relation.id),
                )
                .with_target(format!("{path}/id"), &relation.id),
            );
        }
        let semantic_key = relation_semantic_key(relation);
        if !seen_semantics.insert(semantic_key) {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_RELATION,
                    format!(
                        "duplicate semantic relation {} {}:{} -> {}:{}",
                        relations::relation_kind_label(relation.kind),
                        relations::endpoint_type_label(relation.from.entity_type),
                        relation.from.id,
                        relations::endpoint_type_label(relation.to.entity_type),
                        relation.to.id
                    ),
                )
                .with_target(&path, &relation.id),
            );
        }
        for source_id in &relation.source_ids {
            if !source_ids.contains(source_id.as_str()) {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_CORE_REFERENCE,
                        format!(
                            "relation {} references missing source {}",
                            relation.id, source_id
                        ),
                    )
                    .with_target(format!("{path}/source_ids"), &relation.id),
                );
            }
        }
        for (endpoint_name, endpoint) in [("from", &relation.from), ("to", &relation.to)] {
            if !core_endpoint_exists(
                endpoint,
                source_ids,
                claim_ids,
                element_ids,
                learning_step_ids,
                compatibility_ids,
            ) {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_CORE_REFERENCE,
                        format!(
                            "relation {} {} endpoint references missing {}:{}",
                            relation.id,
                            endpoint_name,
                            relations::endpoint_type_label(endpoint.entity_type),
                            endpoint.id
                        ),
                    )
                    .with_target(format!("{path}/{endpoint_name}"), &relation.id),
                );
            }
        }
        if relation.from == relation.to {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_RELATION,
                    format!(
                        "relation {} is a self-loop: {}",
                        relation.id,
                        core_endpoint_key(&relation.from)
                    ),
                )
                .with_target(&path, &relation.id),
            );
        }
    }

    if let Some(cycle) = hard_prerequisite_cycle(relations) {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_CORE_PREREQUISITE,
                format!("hard prerequisite cycle: {}", cycle.join(" -> ")),
            )
            .with_target("/knowledge/relations", ""),
        );
    }
}

fn validate_evidence_package(
    package: &evidence::EvidencePackage,
    source_ids: &BTreeSet<&str>,
    claim_ids: &BTreeSet<&str>,
    checks: &mut Vec<report::DiagnosticCheck>,
) {
    let mut seen_sources = BTreeSet::new();
    for (index, source) in package.sources.iter().enumerate() {
        let path = format!("/evidence/sources/{index}");
        validate_stable_id(&source.id, &format!("{path}/id"), checks);
        if !seen_sources.insert(source.id.as_str()) {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_REQUIRED,
                    format!("duplicate evidence source id {}", source.id),
                )
                .with_target(format!("{path}/id"), &source.id),
            );
        }
        for (field, value) in [
            ("citation", source.citation.as_str()),
            ("source_type", source.source_type.as_str()),
            ("why_it_matters", source.why_it_matters.as_str()),
        ] {
            require_non_empty(value, &format!("{path}/{field}"), &source.id, checks);
        }
    }

    let mut seen_claims = BTreeSet::new();
    for (claim_index, claim) in package.claims.iter().enumerate() {
        let path = format!("/evidence/claims/{claim_index}");
        validate_stable_id(&claim.id, &format!("{path}/id"), checks);
        if !seen_claims.insert(claim.id.as_str()) {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_REQUIRED,
                    format!("duplicate evidence claim id {}", claim.id),
                )
                .with_target(format!("{path}/id"), &claim.id),
            );
        }
        require_non_empty(
            &claim.statement,
            &format!("{path}/statement"),
            &claim.id,
            checks,
        );
        for (link_index, link) in claim.evidence_links.iter().enumerate() {
            let link_path = format!("{path}/evidence_links/{link_index}");
            if !link.evidence_id.trim().is_empty() {
                validate_stable_id(
                    &link.evidence_id,
                    &format!("{link_path}/evidence_id"),
                    checks,
                );
            }
            if !source_ids.contains(link.source_id.as_str()) {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_CORE_REFERENCE,
                        format!(
                            "claim {} evidence link references missing source {}",
                            claim.id, link.source_id
                        ),
                    )
                    .with_target(format!("{link_path}/source_id"), &claim.id),
                );
            }
            if matches!(
                link.verification_status,
                report::VerificationStatus::Reviewed | report::VerificationStatus::Verified
            ) && link.support_kind == report::SupportKind::Supports
                && (link.reviewed_at.trim().is_empty()
                    || (link.locator.trim().is_empty() && link.support_note.trim().is_empty()))
            {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_CORE_SUPPORT,
                        format!(
                            "reviewed support link for claim {} needs reviewed_at plus locator or support_note",
                            claim.id
                        ),
                    )
                    .with_target(&link_path, &claim.id),
                );
            }
        }
    }

    for claim_id in claim_ids {
        if !seen_claims.contains(claim_id) {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_REFERENCE,
                    format!("claim id index contains missing claim {claim_id}"),
                )
                .with_target("/evidence/claims", *claim_id),
            );
        }
    }
}

fn validate_pedagogy_package(
    package: &pedagogy::PedagogyPackage,
    source_ids: &BTreeSet<&str>,
    element_ids: &BTreeSet<&str>,
    learning_step_ids: &BTreeSet<&str>,
    compatibility_ids: &BTreeSet<String>,
    relations: &[relations::Relation],
    checks: &mut Vec<report::DiagnosticCheck>,
) {
    let mut ladder_ids = BTreeSet::new();
    for (index, row) in package.reading_ladder.iter().enumerate() {
        let path = format!("/pedagogy/reading_ladder/{index}");
        validate_stable_id(&row.id, &format!("{path}/id"), checks);
        if !ladder_ids.insert(row.id.as_str()) {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_REQUIRED,
                    format!("duplicate reading ladder id {}", row.id),
                )
                .with_target(format!("{path}/id"), &row.id),
            );
        }
        for (field, value) in [
            ("layer", row.layer.as_str()),
            ("start_here", row.start_here.as_str()),
            ("read_for", row.read_for.as_str()),
            ("do_not_infer", row.do_not_infer.as_str()),
        ] {
            require_non_empty(value, &format!("{path}/{field}"), &row.id, checks);
        }
        validate_source_refs(
            &row.source_ids,
            source_ids,
            &format!("{path}/source_ids"),
            &row.id,
            checks,
        );
    }

    let mut step_ids = BTreeSet::new();
    for (index, step) in package.learning_path.iter().enumerate() {
        let path = format!("/pedagogy/learning_path/{index}");
        validate_stable_id(&step.id, &format!("{path}/id"), checks);
        if !step_ids.insert(step.id.as_str()) {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_REQUIRED,
                    format!("duplicate learning step id {}", step.id),
                )
                .with_target(format!("{path}/id"), &step.id),
            );
        }
        for (field, value) in [
            ("title", step.title.as_str()),
            ("learning_goal", step.learning_goal.as_str()),
            ("practice_artifact", step.practice_artifact.as_str()),
        ] {
            require_non_empty(value, &format!("{path}/{field}"), &step.id, checks);
        }
        validate_source_refs(
            &step.source_ids,
            source_ids,
            &format!("{path}/source_ids"),
            &step.id,
            checks,
        );
        for prerequisite_id in &step.prerequisite_ids {
            if !learning_step_ids.contains(prerequisite_id.as_str())
                && !element_ids.contains(prerequisite_id.as_str())
                && !compatibility_ids.contains(prerequisite_id)
            {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_CORE_REFERENCE,
                        format!(
                            "learning step {} references missing prerequisite {}",
                            step.id, prerequisite_id
                        ),
                    )
                    .with_target(format!("{path}/prerequisite_ids"), &step.id),
                );
            }
        }
        validate_learning_step_prerequisite_relations(step, index, relations, checks);
    }
}

fn validate_learning_step_prerequisite_relations(
    step: &pedagogy::LearningStep,
    step_index: usize,
    relations: &[relations::Relation],
    checks: &mut Vec<report::DiagnosticCheck>,
) {
    let relation_lookup = relations
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect::<BTreeMap<_, _>>();
    let expected_relation_ids = relations
        .iter()
        .filter(|relation| {
            relations::is_hard_prerequisite(relation.kind)
                && relation.to.entity_type == relations::RelationEndpointType::CurriculumStep
                && relation.to.id == step.id
                && step.prerequisite_ids.contains(&relation.from.id)
        })
        .map(|relation| relation.id.as_str())
        .collect::<BTreeSet<_>>();
    let declared_relation_ids = step
        .prerequisite_relation_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    for relation_id in &step.prerequisite_relation_ids {
        let Some(relation) = relation_lookup.get(relation_id.as_str()) else {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_REFERENCE,
                    format!(
                        "learning step {} references missing prerequisite relation {}",
                        step.id, relation_id
                    ),
                )
                .with_target(
                    format!("/pedagogy/learning_path/{step_index}/prerequisite_relation_ids"),
                    &step.id,
                ),
            );
            continue;
        };
        if !relations::is_hard_prerequisite(relation.kind)
            || relation.to.entity_type != relations::RelationEndpointType::CurriculumStep
            || relation.to.id != step.id
            || !step.prerequisite_ids.contains(&relation.from.id)
        {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_PREREQUISITE,
                    format!(
                        "learning step {} prerequisite relation {} does not match prerequisite_ids",
                        step.id, relation_id
                    ),
                )
                .with_target(
                    format!("/pedagogy/learning_path/{step_index}/prerequisite_relation_ids"),
                    &step.id,
                ),
            );
        }
    }

    for prerequisite_id in &step.prerequisite_ids {
        let has_relation = expected_relation_ids.iter().any(|relation_id| {
            relation_lookup
                .get(*relation_id)
                .map(|relation| relation.from.id == *prerequisite_id)
                .unwrap_or(false)
        });
        if !has_relation {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_PREREQUISITE,
                    format!(
                        "learning step {} prerequisite {} has no canonical requires_before relation",
                        step.id, prerequisite_id
                    ),
                )
                .with_target(
                    format!("/pedagogy/learning_path/{step_index}/prerequisite_ids"),
                    &step.id,
                ),
            );
        }
    }

    if !declared_relation_ids.is_empty() && declared_relation_ids != expected_relation_ids {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_CORE_PREREQUISITE,
                format!(
                    "learning step {} prerequisite_relation_ids disagree with canonical graph",
                    step.id
                ),
            )
            .with_target(
                format!("/pedagogy/learning_path/{step_index}/prerequisite_relation_ids"),
                &step.id,
            ),
        );
    }
}

fn relation_semantic_key(relation: &relations::Relation) -> String {
    format!(
        "{}|{}:{}|{}:{}",
        relations::relation_kind_label(relation.kind),
        relations::endpoint_type_label(relation.from.entity_type),
        relation.from.id,
        relations::endpoint_type_label(relation.to.entity_type),
        relation.to.id
    )
}

fn core_endpoint_exists(
    endpoint: &relations::RelationEndpoint,
    source_ids: &BTreeSet<&str>,
    claim_ids: &BTreeSet<&str>,
    element_ids: &BTreeSet<&str>,
    learning_step_ids: &BTreeSet<&str>,
    compatibility_ids: &BTreeSet<String>,
) -> bool {
    match endpoint.entity_type {
        relations::RelationEndpointType::FieldElement => element_ids.contains(endpoint.id.as_str()),
        relations::RelationEndpointType::Concept
        | relations::RelationEndpointType::Method
        | relations::RelationEndpointType::Representation => {
            compatibility_ids.contains(&endpoint.id)
        }
        relations::RelationEndpointType::Claim => claim_ids.contains(endpoint.id.as_str()),
        relations::RelationEndpointType::Source => source_ids.contains(endpoint.id.as_str()),
        relations::RelationEndpointType::CurriculumStep => {
            learning_step_ids.contains(endpoint.id.as_str())
        }
        relations::RelationEndpointType::FrontierDebate => true,
    }
}

fn core_endpoint_key(endpoint: &relations::RelationEndpoint) -> String {
    format!(
        "{}:{}",
        relations::endpoint_type_label(endpoint.entity_type),
        endpoint.id
    )
}

fn hard_prerequisite_cycle(relations: &[relations::Relation]) -> Option<Vec<String>> {
    let mut graph = BTreeMap::<String, BTreeSet<String>>::new();
    for relation in relations {
        if !relations::is_hard_prerequisite(relation.kind) || relation.from == relation.to {
            continue;
        }
        let from = core_endpoint_key(&relation.from);
        let to = core_endpoint_key(&relation.to);
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

fn validate_source_refs(
    ids: &[String],
    source_ids: &BTreeSet<&str>,
    path: &str,
    entity_id: &str,
    checks: &mut Vec<report::DiagnosticCheck>,
) {
    for source_id in ids {
        if !source_ids.contains(source_id.as_str()) {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_CORE_REFERENCE,
                    format!("{entity_id} references missing source {source_id}"),
                )
                .with_target(path, entity_id),
            );
        }
    }
}

fn require_non_empty(
    value: &str,
    path: &str,
    entity_id: &str,
    checks: &mut Vec<report::DiagnosticCheck>,
) {
    if value.trim().is_empty() {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_CORE_REQUIRED,
                format!("{path} must be a non-empty string"),
            )
            .with_target(path, entity_id),
        );
    }
}

fn validate_stable_id(id: &str, path: &str, checks: &mut Vec<report::DiagnosticCheck>) {
    if !is_stable_id(id) {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_CORE_REQUIRED,
                format!("{path} is not a stable id: {id:?}"),
            )
            .with_target(path, id),
        );
    }
}

fn is_stable_id(id: &str) -> bool {
    id.contains('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .chars()
            .next()
            .map(|first| first.is_ascii_lowercase())
            .unwrap_or(false)
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn sort_diagnostics(checks: &mut [report::DiagnosticCheck]) {
    checks.sort_by(|left, right| {
        severity_sort_key(left.severity)
            .cmp(&severity_sort_key(right.severity))
            .then_with(|| left.check_id.cmp(&right.check_id))
            .then_with(|| left.target_path.cmp(&right.target_path))
            .then_with(|| left.entity_id.cmp(&right.entity_id))
            .then_with(|| left.message.cmp(&right.message))
    });
}

fn severity_sort_key(severity: report::DiagnosticSeverity) -> u8 {
    match severity {
        report::DiagnosticSeverity::Error => 0,
        report::DiagnosticSeverity::Warning => 1,
        report::DiagnosticSeverity::Info => 2,
    }
}
