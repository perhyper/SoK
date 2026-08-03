use super::*;

pub const CHECK_CORE_VERSION: &str = "core.version";
pub const CHECK_CORE_REQUIRED: &str = "core.required";
pub const CHECK_CORE_REFERENCE: &str = "core.reference";
pub const CHECK_CORE_SUPPORT: &str = "core.support";

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

    validate_knowledge_package(&packages.knowledge, &source_ids, &mut checks);
    validate_evidence_package(&packages.evidence, &source_ids, &claim_ids, &mut checks);
    validate_pedagogy_package(
        &packages.pedagogy,
        &source_ids,
        &element_ids,
        &learning_step_ids,
        &compatibility_ids,
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
    checks: &mut Vec<report::DiagnosticCheck>,
) {
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
    }
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
