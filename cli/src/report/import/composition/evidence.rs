use super::*;

pub(crate) fn build_claim_seeds(parsed: &ParsedMarkdownReport) -> Vec<ClaimSeed> {
    let mut seeds = Vec::new();
    for row in &parsed.claim_rows {
        let statement = first_non_empty([
            lookup_cell_any(row, &["Statement"]).as_str(),
            lookup_cell_any(row, &["Claim"]).as_str(),
        ]);
        if statement.is_empty() || is_placeholder_text(&statement) {
            continue;
        }
        seeds.push(ClaimSeed {
            statement,
            claim_type: parse_claim_type(&lookup_cell_any(row, &["Claim type", "Type"])),
            evidence_requirement: parse_evidence_requirement(&lookup_cell_any(
                row,
                &["Evidence requirement", "Requirement"],
            )),
            source_refs: split_ref_list(&lookup_cell_any(
                row,
                &["Source IDs", "Source ids", "Sources", "Key sources"],
            )),
            confidence: parse_claim_confidence(&lookup_cell_any(row, &["Confidence"])),
            notes: lookup_cell_any(row, &["Notes", "Note"]),
            temporal_status: parse_temporal_status(&lookup_cell_any(
                row,
                &["Temporal status", "Temporal"],
            )),
        });
    }
    seeds
}

pub(crate) fn build_claims(
    seeds: Vec<ClaimSeed>,
    source_lookup: &SourceLookup,
    evidence: &[EvidenceEntry],
    stage: ExportStage,
    as_of: &str,
    review_after: &str,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<Claim> {
    let mut claims = Vec::new();
    for seed in seeds {
        let id = content_id("claim", &[&seed.statement]);
        let intended_source_ids = source_lookup.resolve_ref_list_with_diagnostics(
            &seed.source_refs,
            "/report/claims/source_ids",
            &id,
            diagnostics,
        );
        let mut evidence_links = Vec::new();
        let mut saw_cataloged_only = false;
        let mut saw_satisfying_evidence = false;

        for entry in evidence {
            if !intended_source_ids.is_empty() && !intended_source_ids.contains(&entry.source_id) {
                continue;
            }
            if intended_source_ids.is_empty()
                && !entry.claim_ids.iter().any(|claim_id| claim_id == &id)
            {
                continue;
            }
            if !entry.claim_ids.is_empty()
                && !entry.claim_ids.iter().any(|claim_id| claim_id == &id)
            {
                continue;
            }
            if entry.can_satisfy_claim_link() {
                saw_satisfying_evidence = true;
            }
            if entry.verification_status == VerificationStatus::Cataloged {
                saw_cataloged_only = true;
            }
            if let Some(link) = entry.to_visible_evidence_link() {
                if let Some(existing_index) =
                    evidence_links.iter().position(|existing: &EvidenceLink| {
                        duplicate_claim_evidence_link(existing, &link)
                    })
                {
                    if link.verification_status == VerificationStatus::Cataloged
                        && !entry.claim_ids.is_empty()
                    {
                        evidence_links[existing_index] = link;
                    }
                } else {
                    evidence_links.push(link);
                }
            }
        }

        if stage == ExportStage::Final
            && !saw_satisfying_evidence
            && (!intended_source_ids.is_empty() || saw_cataloged_only)
        {
            let check_id = if saw_cataloged_only {
                CHECK_EVIDENCE_CATALOGED_ONLY
            } else {
                CHECK_EXPORT_CLAIM_NEEDS_EVIDENCE
            };
            diagnostics.push(
                DiagnosticCheck::warning(
                    check_id,
                    format!(
                        "claim {} has no reviewed or verified evidence link with usable support metadata",
                        id
                    ),
                )
                .with_target("/report/claims", &id),
            );
        }

        let temporal_status = seed
            .temporal_status
            .unwrap_or(match (stage, seed.claim_type) {
                (ExportStage::Scaffold, _) => TemporalStatus::Unknown,
                (_, ClaimType::Frontier | ClaimType::Currentness | ClaimType::Debate) => {
                    TemporalStatus::Current
                }
                _ => TemporalStatus::Durable,
            });
        claims.push(Claim {
            id,
            statement: seed.statement,
            claim_type: seed.claim_type,
            evidence_requirement: seed.evidence_requirement,
            evidence_links,
            confidence: seed.confidence,
            conflict_review: None,
            temporal: TemporalMarker {
                as_of: as_of.to_string(),
                review_after: if matches!(
                    temporal_status,
                    TemporalStatus::Current | TemporalStatus::ReviewDue | TemporalStatus::Stale
                ) {
                    review_after.to_string()
                } else {
                    String::new()
                },
                temporal_status,
                rationale: String::new(),
            },
            notes: seed.notes,
        });
    }
    claims
}

pub(crate) fn duplicate_claim_evidence_link(existing: &EvidenceLink, next: &EvidenceLink) -> bool {
    if existing.verification_status == VerificationStatus::Cataloged
        && next.verification_status == VerificationStatus::Cataloged
    {
        return existing.source_id == next.source_id;
    }
    existing.evidence_id == next.evidence_id && existing.source_id == next.source_id
}
pub(crate) fn evidence_source_diagnostics(
    evidence: &[EvidenceEntry],
    source_ids: &BTreeSet<String>,
) -> Vec<DiagnosticCheck> {
    evidence
        .iter()
        .filter(|entry| !source_ids.contains(&entry.source_id))
        .map(|entry| {
            DiagnosticCheck::warning(
                CHECK_EXPORT_UNKNOWN_EVIDENCE_SOURCE,
                format!(
                    "evidence {} references unknown source_id {}",
                    entry.evidence_id, entry.source_id
                ),
            )
            .with_target("/evidence", &entry.evidence_id)
        })
        .collect()
}

pub(crate) fn apply_evidence_review_status(
    sources: &mut [ReportSource],
    evidence: &[EvidenceEntry],
) {
    for source in sources {
        let mut status = source.verification_status;
        let mut last_reviewed = source.last_reviewed.clone();
        for entry in evidence.iter().filter(|entry| entry.source_id == source.id) {
            status = max_verification_status(status, entry.verification_status);
            let reviewed_at =
                first_non_empty([entry.reviewed_at.as_str(), entry.observed_at.as_str()]);
            if !reviewed_at.is_empty() && reviewed_at > last_reviewed {
                last_reviewed = reviewed_at;
            }
        }
        source.verification_status = status;
        source.last_reviewed = last_reviewed;
    }
}

pub(crate) fn max_verification_status(
    left: VerificationStatus,
    right: VerificationStatus,
) -> VerificationStatus {
    if verification_rank(right) > verification_rank(left) {
        right
    } else {
        left
    }
}

pub(crate) fn verification_rank(status: VerificationStatus) -> u8 {
    match status {
        VerificationStatus::Cataloged => 0,
        VerificationStatus::Reviewed => 1,
        VerificationStatus::Verified => 2,
    }
}
