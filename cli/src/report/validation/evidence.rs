use super::*;

pub(crate) fn validate_claim_evidence_requirements(
    report: &PublicReport,
    index: &ReportIdIndex,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for (claim_index, claim) in report.claims.iter().enumerate() {
        let path = format!("/report/claims/{claim_index}");
        validate_conflict_review(claim, &path, checks);
        let mut usable_reviewed_sources = BTreeSet::new();
        let mut usable_verified_sources = BTreeSet::new();
        let mut saw_cataloged = false;
        let mut saw_qualifying_without_support = false;
        for (link_index, link) in claim.evidence_links.iter().enumerate() {
            let link_path = format!("{path}/evidence_links/{link_index}");
            if !index.sources.contains(&link.source_id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_EVIDENCE_SOURCE,
                        format!(
                            "claim {} evidence link references missing source {}",
                            claim.id, link.source_id
                        ),
                    )
                    .with_target(&link_path, &claim.id),
                );
            }
            if link.verification_status == VerificationStatus::Cataloged {
                saw_cataloged = true;
                continue;
            }
            validate_reviewed_evidence_link_metadata(claim, link, &link_path, checks);

            match link.support_kind {
                SupportKind::Supports => {}
                SupportKind::Qualifies => {
                    saw_qualifying_without_support = true;
                    continue;
                }
                SupportKind::Contradicts => {
                    if claim.conflict_review.is_none() {
                        checks.push(
                            DiagnosticCheck::warning(
                                CHECK_VALIDATE_EVIDENCE_SUPPORT,
                                format!(
                                "claim {} has contradictory evidence from {}; record a conflict \
                                    or lower-confidence interpretation",
                                claim.id, link.source_id
                            ),
                            )
                            .with_target(&link_path, &claim.id),
                        );
                    }
                    continue;
                }
                SupportKind::Background | SupportKind::Example => {
                    continue;
                }
            }

            if !reviewed_link_can_support_claim(link) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_EVIDENCE_SUPPORT,
                        format!(
                            "claim {} evidence link {} cannot satisfy claim support without \
                                reviewed support metadata",
                            claim.id, link.source_id
                        ),
                    )
                    .with_target(&link_path, &claim.id),
                );
                continue;
            }
            match link.verification_status {
                VerificationStatus::Reviewed => {
                    usable_reviewed_sources.insert(link.source_id.clone());
                }
                VerificationStatus::Verified => {
                    usable_reviewed_sources.insert(link.source_id.clone());
                    usable_verified_sources.insert(link.source_id.clone());
                }
                VerificationStatus::Cataloged => {}
            }
        }

        if !claim_requires_reviewed_or_verified_evidence(claim) {
            continue;
        }

        let satisfied = match claim.evidence_requirement {
            EvidenceRequirement::VerifiedSource => !usable_verified_sources.is_empty(),
            EvidenceRequirement::MultipleReviewedSources => usable_reviewed_sources.len() >= 2,
            EvidenceRequirement::None
            | EvidenceRequirement::CatalogedSource
            | EvidenceRequirement::ReviewedSource => !usable_reviewed_sources.is_empty(),
        };
        if !satisfied {
            if saw_cataloged {
                checks.push(
                    DiagnosticCheck::warning(
                        CHECK_EVIDENCE_CATALOGED_ONLY,
                        format!(
                            "claim {} has cataloged evidence links, but cataloged evidence \
                                cannot satisfy reviewed support",
                            claim.id
                        ),
                    )
                    .with_target(&path, &claim.id),
                );
            }
            if saw_qualifying_without_support {
                checks.push(
                    DiagnosticCheck::warning(
                        CHECK_VALIDATE_EVIDENCE_SUPPORT,
                        format!(
                            "claim {} has qualifying evidence but no affirmative supporting evidence",
                            claim.id
                        ),
                    )
                    .with_target(&path, &claim.id),
                );
            }
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_EVIDENCE_REQUIRED,
                    format!(
                        "claim {} requires reviewed or verified evidence with a locator or \
                            support_note based on claim_type {:?} and evidence_requirement {:?}",
                        claim.id, claim.claim_type, claim.evidence_requirement
                    ),
                )
                .with_target(&path, &claim.id),
            );
        }
    }
}

pub(crate) fn validate_reviewed_evidence_link_metadata(
    claim: &Claim,
    link: &EvidenceLink,
    link_path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    if !matches!(
        link.verification_status,
        VerificationStatus::Reviewed | VerificationStatus::Verified
    ) {
        return;
    }
    if link.reviewed_at.trim().is_empty() {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_EVIDENCE_SUPPORT,
                format!(
                    "claim {} evidence link {} needs reviewed_at",
                    claim.id, link.source_id
                ),
            )
            .with_target(link_path, &claim.id),
        );
    } else if !looks_like_iso_date(&link.reviewed_at) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_EVIDENCE_SUPPORT,
                format!(
                    "claim {} evidence link {} reviewed_at must be YYYY-MM-DD",
                    claim.id, link.source_id
                ),
            )
            .with_target(link_path, &claim.id),
        );
    }
    if link.locator.trim().is_empty() && link.support_note.trim().is_empty() {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_EVIDENCE_SUPPORT,
                format!(
                    "claim {} evidence link {} needs a locator or support_note",
                    claim.id, link.source_id
                ),
            )
            .with_target(link_path, &claim.id),
        );
    }
}

pub(crate) fn reviewed_link_can_support_claim(link: &EvidenceLink) -> bool {
    matches!(
        link.verification_status,
        VerificationStatus::Reviewed | VerificationStatus::Verified
    ) && link.support_kind == SupportKind::Supports
        && looks_like_iso_date(&link.reviewed_at)
        && (!link.locator.trim().is_empty() || !link.support_note.trim().is_empty())
}

pub(crate) fn claim_requires_reviewed_or_verified_evidence(claim: &Claim) -> bool {
    claim.evidence_requirement != EvidenceRequirement::None
        || matches!(
            claim.claim_type,
            ClaimType::Currentness | ClaimType::Frontier | ClaimType::Debate
        )
}

pub(crate) fn validate_source_role_coverage(
    report: &PublicReport,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for (requirement_index, requirement) in report
        .evidence_standards
        .source_role_requirements
        .iter()
        .enumerate()
    {
        let path =
            format!("/report/evidence_standards/source_role_requirements/{requirement_index}");
        let minimum = requirement
            .minimum_sources
            .unwrap_or(match requirement.requirement {
                SourceRoleRequirementKind::Required | SourceRoleRequirementKind::Conditional => 1,
                SourceRoleRequirementKind::Waived | SourceRoleRequirementKind::NotApplicable => 0,
            }) as usize;
        let count = report
            .sources
            .iter()
            .filter(|source| source.roles.contains(&requirement.role))
            .count();
        if requirement.rationale.trim().is_empty() {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!(
                        "source role {} needs a recorded rationale",
                        source_role_label(requirement.role)
                    ),
                )
                .with_target(&path, ""),
            );
        }

        match requirement.requirement {
            SourceRoleRequirementKind::Required if count < minimum => {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_SOURCE_ROLE_REQUIRED,
                        format!(
                            "required source role {} has {count} source(s), minimum is {minimum}",
                            source_role_label(requirement.role)
                        ),
                    )
                    .with_target(&path, ""),
                );
            }
            SourceRoleRequirementKind::Conditional if count < minimum => {
                if !has_usable_waiver(&requirement.waiver) {
                    checks.push(
                        DiagnosticCheck::warning(
                            CHECK_VALIDATE_SOURCE_ROLE_CONDITIONAL,
                            format!(
                                "conditional source role {} has {count} source(s), minimum is \
                                    {minimum}; record a waiver rationale if the condition is not \
                                    active",
                                source_role_label(requirement.role)
                            ),
                        )
                        .with_target(&path, ""),
                    );
                }
            }
            SourceRoleRequirementKind::Waived => {
                if !has_usable_waiver(&requirement.waiver) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_SOURCE_ROLE_WAIVER,
                            format!(
                                "waived source role {} needs waiver rationale and as_of",
                                source_role_label(requirement.role)
                            ),
                        )
                        .with_target(&path, ""),
                    );
                }
            }
            SourceRoleRequirementKind::NotApplicable => {
                if let Some(waiver) = &requirement.waiver {
                    if waiver.rationale.trim().is_empty() {
                        checks.push(
                            DiagnosticCheck::warning(
                                CHECK_VALIDATE_SOURCE_ROLE_WAIVER,
                                format!(
                                    "not_applicable source role {} has an empty waiver rationale",
                                    source_role_label(requirement.role)
                                ),
                            )
                            .with_target(&path, ""),
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn has_usable_waiver(waiver: &Option<SourceRoleWaiver>) -> bool {
    waiver
        .as_ref()
        .map(|waiver| !waiver.rationale.trim().is_empty() && !waiver.as_of.trim().is_empty())
        .unwrap_or(false)
}

pub(crate) fn validate_currentness(
    document: &ReportDocument,
    index: &ReportIdIndex,
    checks: &mut Vec<DiagnosticCheck>,
) {
    validate_temporal_marker(
        &document.metadata.temporal_review,
        "/metadata/temporal_review",
        "",
        document.metadata.report_type == ReportType::HumanReport,
        checks,
    );

    let sources_by_id = document
        .report
        .sources
        .iter()
        .map(|source| (source.id.as_str(), source))
        .collect::<BTreeMap<_, _>>();

    for (claim_index, claim) in document.report.claims.iter().enumerate() {
        let path = format!("/report/claims/{claim_index}/temporal");
        let needs_current_metadata = claim_needs_currentness_metadata(claim);
        validate_temporal_marker(
            &claim.temporal,
            &path,
            &claim.id,
            needs_current_metadata,
            checks,
        );
        if has_currentness_words(&format!("{} {}", claim.statement, claim.notes))
            && structured_temporal_metadata_absent(&claim.temporal)
        {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_VALIDATE_CURRENTNESS_PROSE,
                    format!(
                        "claim {} uses currentness prose but lacks structured temporal metadata",
                        claim.id
                    ),
                )
                .with_target(format!("/report/claims/{claim_index}/statement"), &claim.id),
            );
        }
        if needs_current_metadata {
            validate_claim_source_dates(claim, &sources_by_id, index, claim_index, checks);
        }
    }

    for (item_index, item) in document.report.frontier_debates.iter().enumerate() {
        validate_temporal_marker(
            &item.temporal,
            &format!("/report/frontier_debates/{item_index}/temporal"),
            &item.id,
            true,
            checks,
        );
        if has_currentness_words(&format!("{} {}", item.title, item.summary))
            && structured_temporal_metadata_absent(&item.temporal)
        {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_VALIDATE_CURRENTNESS_PROSE,
                    format!(
                        "frontier/debate item {} uses currentness prose but lacks structured temporal metadata",
                        item.id
                    ),
                )
                .with_target(
                    format!("/report/frontier_debates/{item_index}/summary"),
                    &item.id,
                ),
            );
        }
    }
}

pub(crate) fn validate_temporal_marker(
    marker: &TemporalMarker,
    path: &str,
    entity_id: &str,
    require_review_after: bool,
    checks: &mut Vec<DiagnosticCheck>,
) {
    if marker.as_of.trim().is_empty() {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_CURRENTNESS_METADATA,
                format!("{path}/as_of is required for deterministic currentness"),
            )
            .with_target(format!("{path}/as_of"), entity_id),
        );
    } else if !looks_like_iso_date(&marker.as_of) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_CURRENTNESS_METADATA,
                format!("{path}/as_of must be YYYY-MM-DD"),
            )
            .with_target(format!("{path}/as_of"), entity_id),
        );
    }

    if require_review_after && marker.review_after.trim().is_empty() {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_CURRENTNESS_METADATA,
                format!("{path}/review_after is required for currentness/frontier/debate claims"),
            )
            .with_target(format!("{path}/review_after"), entity_id),
        );
    }
    if !marker.review_after.trim().is_empty() && !looks_like_iso_date(&marker.review_after) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_CURRENTNESS_METADATA,
                format!("{path}/review_after must be YYYY-MM-DD"),
            )
            .with_target(format!("{path}/review_after"), entity_id),
        );
    }

    if !marker.as_of.trim().is_empty()
        && !marker.review_after.trim().is_empty()
        && looks_like_iso_date(&marker.as_of)
        && looks_like_iso_date(&marker.review_after)
        && marker.review_after.as_str() <= marker.as_of.as_str()
    {
        let severity = if marker.temporal_status == TemporalStatus::Current {
            DiagnosticSeverity::Error
        } else {
            DiagnosticSeverity::Warning
        };
        checks.push(
            DiagnosticCheck::new(
                CHECK_VALIDATE_CURRENTNESS_REVIEW_DUE,
                severity,
                format!(
                    "{path}/review_after {} is not after as_of {}",
                    marker.review_after, marker.as_of
                ),
            )
            .with_target(format!("{path}/review_after"), entity_id),
        );
    }

    match marker.temporal_status {
        TemporalStatus::Stale => checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_CURRENTNESS_REVIEW_DUE,
                format!("{path}/temporal_status is stale"),
            )
            .with_target(format!("{path}/temporal_status"), entity_id),
        ),
        TemporalStatus::ReviewDue => checks.push(
            DiagnosticCheck::warning(
                CHECK_VALIDATE_CURRENTNESS_REVIEW_DUE,
                format!("{path}/temporal_status is review_due"),
            )
            .with_target(format!("{path}/temporal_status"), entity_id),
        ),
        TemporalStatus::Unknown if require_review_after => checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_CURRENTNESS_METADATA,
                format!("{path}/temporal_status cannot be unknown for currentness/frontier/debate claims"),
            )
            .with_target(format!("{path}/temporal_status"), entity_id),
        ),
        _ => {}
    }
}

pub(crate) fn claim_needs_currentness_metadata(claim: &Claim) -> bool {
    matches!(
        claim.claim_type,
        ClaimType::Currentness | ClaimType::Frontier | ClaimType::Debate
    ) || matches!(
        claim.temporal.temporal_status,
        TemporalStatus::Current | TemporalStatus::ReviewDue | TemporalStatus::Stale
    )
}

pub(crate) fn validate_claim_source_dates(
    claim: &Claim,
    sources_by_id: &BTreeMap<&str, &ReportSource>,
    index: &ReportIdIndex,
    claim_index: usize,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for link in &claim.evidence_links {
        if !index.sources.contains(&link.source_id) {
            continue;
        }
        let Some(source) = sources_by_id.get(link.source_id.as_str()) else {
            continue;
        };
        if source.date.trim().is_empty() {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_VALIDATE_CURRENTNESS_SOURCE_DATE,
                    format!(
                        "currentness claim {} uses source {} with no source date",
                        claim.id, source.id
                    ),
                )
                .with_target(
                    format!("/report/claims/{claim_index}/evidence_links"),
                    &claim.id,
                ),
            );
        } else if source_date_after_as_of(&source.date, &claim.temporal.as_of) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_CURRENTNESS_SOURCE_DATE,
                    format!(
                        "claim {} has as_of {} before source {} date {}",
                        claim.id, claim.temporal.as_of, source.id, source.date
                    ),
                )
                .with_target(
                    format!("/report/claims/{claim_index}/evidence_links"),
                    &claim.id,
                ),
            );
        }
    }
}
pub(crate) fn validate_source_access_metadata(
    report: &PublicReport,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for (source_index, source) in report.sources.iter().enumerate() {
        let path = format!("/report/sources/{source_index}/access");
        if source.access.status == AccessStatus::Unknown {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SOURCE_ACCESS,
                    format!("source {} has unknown access status", source.id),
                )
                .with_target(format!("{path}/status"), &source.id),
            );
        }
        if source.access.route.trim().is_empty()
            || source.access.route.trim().eq_ignore_ascii_case("unknown")
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SOURCE_ACCESS,
                    format!("source {} has no actionable access route", source.id),
                )
                .with_target(format!("{path}/route"), &source.id),
            );
        }
        if source_access_requires_notes(source.access.status)
            && source.access.budget_estimate.trim().is_empty()
            && source.access.license.trim().is_empty()
            && source.access.notes.trim().is_empty()
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SOURCE_ACCESS,
                    format!(
                        "source {} needs budget, license, or access notes for {} access",
                        source.id,
                        access_status_label(source.access.status)
                    ),
                )
                .with_target(path.clone(), &source.id),
            );
        }
        if source_access_requires_metadata_only(source.access.status)
            && source.access.metadata_only != Some(true)
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SOURCE_ACCESS,
                    format!(
                        "source {} with {} access must be marked metadata_only",
                        source.id,
                        access_status_label(source.access.status)
                    ),
                )
                .with_target(format!("{path}/metadata_only"), &source.id),
            );
        }
    }
}

/// Reviews cover all opposing links and must be renewed after new evidence reviews.
pub(crate) fn validate_conflict_review(
    claim: &Claim,
    path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    let Some(review) = &claim.conflict_review else {
        return;
    };
    let opposing: Vec<_> = claim
        .evidence_links
        .iter()
        .filter(|link| link.support_kind == SupportKind::Contradicts)
        .collect();
    let declared: BTreeSet<_> = review.source_ids.iter().collect();
    let actual: BTreeSet<_> = opposing.iter().map(|link| &link.source_id).collect();
    if review.explanation.trim().is_empty()
        || !looks_like_iso_date(&review.reviewed_at)
        || opposing.is_empty()
        || declared != actual
        || declared.len() != review.source_ids.len()
        || opposing
            .iter()
            .any(|link| link.reviewed_at > review.reviewed_at)
    {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_EVIDENCE_SUPPORT,
                format!(
                    "claim {} conflict_review needs an explanation, valid review date, \
                and exactly the opposing source IDs; renew it after evidence changes",
                    claim.id
                ),
            )
            .with_target(format!("{path}/conflict_review"), &claim.id),
        );
    }
}
