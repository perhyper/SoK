use super::*;

pub(crate) fn select_lint_parse_diagnostics(
    checks: &mut Vec<DiagnosticCheck>,
) -> Vec<DiagnosticCheck> {
    checks
        .drain(..)
        .filter(|check| {
            check.check_id == CHECK_EXPORT_AMBIGUOUS_SECTION
                || check.check_id == CHECK_EXPORT_UNKNOWN_SURFACE_MARKER
                || check.check_id == CHECK_EXPORT_UNKNOWN_DIRECTIVE
                || check.message.contains("unclosed fenced code block")
        })
        .collect()
}

pub(crate) fn parsed_has_any_canonical_content(parsed: &ParsedMarkdownReport) -> bool {
    !parsed.title_field.trim().is_empty()
        || !parsed.sections.is_empty()
        || !parsed.narrative_sections.is_empty()
}

pub(crate) fn lint_stage_boundary(
    markdown: &str,
    parsed: &ParsedMarkdownReport,
    stage: ExportStage,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for section in [
        CanonicalSection::ResearchFrame,
        CanonicalSection::ScaffoldQualityNotes,
        CanonicalSection::SoKUsefulnessNotes,
    ] {
        if parsed_has_section(parsed, section) {
            let label = canonical_section_label(section);
            match stage {
                ExportStage::Scaffold => checks.push(
                    DiagnosticCheck::new(
                        CHECK_LINT_SCAFFOLD_UNRESOLVED,
                        DiagnosticSeverity::Info,
                        format!(
                            "scaffold-stage Markdown contains expected internal section {label}"
                        ),
                    )
                    .with_target("/report/sections", ""),
                ),
                ExportStage::Final => checks.push(
                    DiagnosticCheck::error(
                        CHECK_LINT_FINAL_PUBLIC_LEAKAGE,
                        format!("final-stage Markdown contains internal scaffold section {label}"),
                    )
                    .with_target("/report/sections", ""),
                ),
            }
        }
    }

    let mut seen = BTreeSet::new();
    for (line_index, line) in markdown_lines_outside_fences(markdown) {
        let trimmed = line.trim();
        if trimmed.is_empty() || !contains_non_public_scaffold_text(trimmed) {
            continue;
        }
        let normalized = normalize_id_text(trimmed);
        if !seen.insert(normalized) {
            continue;
        }
        let target = format!("/report/markdown/line-{}", line_index + 1);
        match stage {
            ExportStage::Scaffold => checks.push(
                DiagnosticCheck::new(
                    CHECK_LINT_SCAFFOLD_UNRESOLVED,
                    DiagnosticSeverity::Info,
                    format!("scaffold-stage unresolved or internal note is expected before finalization: {trimmed}"),
                )
                .with_target(target, ""),
            ),
            ExportStage::Final => checks.push(
                DiagnosticCheck::error(
                    CHECK_LINT_FINAL_PUBLIC_LEAKAGE,
                    format!("final-stage Markdown would leak scaffold/internal text: {trimmed}"),
                )
                .with_target(target, ""),
            ),
        }
    }
}

pub(crate) fn validate_markdown_report_architecture(
    parsed: &ParsedMarkdownReport,
    stage: ExportStage,
    checks: &mut Vec<DiagnosticCheck>,
) {
    if stage != ExportStage::Final {
        return;
    }
    if !parsed_has_section(parsed, CanonicalSection::ReportArchitecture) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_EXPORT_REPORT_ARCHITECTURE,
                "final Markdown needs a Report Architecture surface chosen after field discovery",
            )
            .with_target("/report/presentation", ""),
        );
    }
    for (label, keys) in [
        ("Executive thesis", &["Executive thesis", "Thesis"][..]),
        (
            "Chosen organizing form",
            &["Chosen organizing form", "Organizing form"][..],
        ),
        (
            "Architecture rationale",
            &["Architecture rationale", "Rationale", "Why this form"][..],
        ),
        (
            "Rejected alternatives and why",
            &[
                "Rejected alternatives and why",
                "Alternatives considered",
                "Rejected alternatives",
            ][..],
        ),
    ] {
        if architecture_value(&parsed.report_architecture, keys)
            .trim()
            .is_empty()
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_REPORT_ARCHITECTURE,
                    format!("Report Architecture needs a non-empty {label} decision"),
                )
                .with_target("/report/presentation", label),
            );
        }
    }
    if parsed.narrative_sections.is_empty() {
        checks.push(
            DiagnosticCheck::error(
                CHECK_EXPORT_REPORT_ARCHITECTURE,
                "final Markdown needs at least one field-specific public H2 section",
            )
            .with_target("/report/presentation/sections", ""),
        );
    }
    let mut complete_field_rows = 0usize;
    let mut saw_core = false;
    let mut saw_surrounding = false;
    for (row_index, row) in parsed.deep_structure_rows.iter().enumerate() {
        let required = [
            (
                "Element class",
                lookup_cell_any(row, &["Element class", "Class", "Element type"]),
            ),
            (
                "Observed element",
                lookup_cell_any(row, &["Observed element", "Element", "Name", "Title"]),
            ),
            (
                "Actual form in this field",
                lookup_cell_any(row, &["Actual form in this field"]),
            ),
            (
                "Role",
                lookup_cell_any(
                    row,
                    &[
                        "Role",
                        "Role: core / surrounding / context",
                        "Structural role",
                    ],
                ),
            ),
            (
                "Load-bearing relations",
                lookup_cell_any(
                    row,
                    &[
                        "Load-bearing relations",
                        "Load bearing relations",
                        "Key relations",
                    ],
                ),
            ),
            (
                "Source IDs",
                lookup_cell_any(row, &["Source IDs", "Sources", "Key sources"]),
            ),
        ];
        let missing = required
            .iter()
            .filter_map(|(label, value)| {
                (value.trim().is_empty() || is_placeholder_text(value)).then_some(*label)
            })
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_REPORT_ARCHITECTURE,
                    format!(
                        "field-element inventory row {} needs researched values for {}",
                        row_index + 1,
                        missing.join(", ")
                    ),
                )
                .with_target(format!("/report/field_elements/{row_index}"), ""),
            );
            continue;
        }
        complete_field_rows += 1;
        let role = normalize_id_text(&required[3].1);
        saw_core |= role.contains("core");
        saw_surrounding |= role.contains("surrounding") || role.contains("context");
    }
    if complete_field_rows == 0 {
        checks.push(
            DiagnosticCheck::error(
                CHECK_EXPORT_REPORT_ARCHITECTURE,
                "final Markdown needs at least one complete field-element inventory row so the \
                    architecture is grounded in observed field forms",
            )
            .with_target("/report/field_elements", ""),
        );
    } else {
        if !saw_core {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_REPORT_ARCHITECTURE,
                    "field-element inventory needs at least one element identified as core",
                )
                .with_target("/report/field_elements", ""),
            );
        }
        if !saw_surrounding {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_REPORT_ARCHITECTURE,
                    "field-element inventory needs at least one surrounding or context element \
                        needed to make the core intelligible",
                )
                .with_target("/report/field_elements", ""),
            );
        }
    }
}

pub(crate) fn contains_non_public_scaffold_text(text: &str) -> bool {
    let normalized = normalize_id_text(text);
    is_placeholder_text(text)
        || [
            "raw intent",
            "raw prompt",
            "prompt intent",
            "original goal",
            "learner profile",
            "profile hypothesis",
            "scaffold stance",
            "scoped assumption",
            "evidence posture",
            "scaffold quality notes",
            "sok usefulness notes",
        ]
        .iter()
        .any(|phrase| normalized.contains(phrase))
}

pub(crate) fn lint_source_role_coverage(
    parsed: &ParsedMarkdownReport,
    sources: &[ReportSource],
    checks: &mut Vec<DiagnosticCheck>,
) {
    let standards = build_evidence_standards(parsed);
    for (requirement_index, requirement) in standards.source_role_requirements.iter().enumerate() {
        let minimum = requirement
            .minimum_sources
            .unwrap_or(match requirement.requirement {
                SourceRoleRequirementKind::Required | SourceRoleRequirementKind::Conditional => 1,
                SourceRoleRequirementKind::Waived | SourceRoleRequirementKind::NotApplicable => 0,
            }) as usize;
        let count = sources
            .iter()
            .filter(|source| source.roles.contains(&requirement.role))
            .count();
        let target = format!("/report/source_role_probe/{requirement_index}");
        match requirement.requirement {
            SourceRoleRequirementKind::Required if count < minimum => checks.push(
                DiagnosticCheck::warning(
                    CHECK_VALIDATE_SOURCE_ROLE_REQUIRED,
                    format!(
                        "required source role {} has {count} source(s), minimum is {minimum}",
                        source_role_label(requirement.role)
                    ),
                )
                .with_target(target, ""),
            ),
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
                        .with_target(target, ""),
                    );
                }
            }
            SourceRoleRequirementKind::Waived if !has_usable_waiver(&requirement.waiver) => {
                checks.push(
                    DiagnosticCheck::warning(
                        CHECK_VALIDATE_SOURCE_ROLE_WAIVER,
                        format!(
                            "waived source role {} needs waiver rationale and as_of",
                            source_role_label(requirement.role)
                        ),
                    )
                    .with_target(target, ""),
                );
            }
            _ => {}
        }
    }
}

pub(crate) fn lint_evidence_sources(
    evidence: &[EvidenceEntry],
    sources: &[ReportSource],
    checks: &mut Vec<DiagnosticCheck>,
) {
    let source_ids = sources
        .iter()
        .map(|source| source.id.clone())
        .collect::<BTreeSet<_>>();
    for mut check in evidence_source_diagnostics(evidence, &source_ids) {
        check.severity = DiagnosticSeverity::Error;
        checks.push(check);
    }
}

pub(crate) fn lint_evidence_semantics(
    evidence: &[EvidenceEntry],
    checks: &mut Vec<DiagnosticCheck>,
) {
    for entry in evidence {
        if !matches!(
            entry.verification_status,
            VerificationStatus::Reviewed | VerificationStatus::Verified
        ) {
            continue;
        }
        if entry.reviewed_at.trim().is_empty() {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_VALIDATE_EVIDENCE_SUPPORT,
                    format!("evidence {} needs reviewed_at", entry.evidence_id),
                )
                .with_target("/evidence", &entry.evidence_id),
            );
        } else if !looks_like_iso_date(&entry.reviewed_at) {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_VALIDATE_EVIDENCE_SUPPORT,
                    format!(
                        "evidence {} reviewed_at must be YYYY-MM-DD",
                        entry.evidence_id
                    ),
                )
                .with_target("/evidence", &entry.evidence_id),
            );
        }
        if entry.locator.trim().is_empty() && entry.support_note.trim().is_empty() {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_VALIDATE_EVIDENCE_SUPPORT,
                    format!(
                        "evidence {} needs a locator or support_note",
                        entry.evidence_id
                    ),
                )
                .with_target("/evidence", &entry.evidence_id),
            );
        }
        if entry.support_kind == SupportKind::Contradicts {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_VALIDATE_EVIDENCE_SUPPORT,
                    format!(
                        "evidence {} is contradictory and must be handled as conflict or low-confidence context",
                        entry.evidence_id
                    ),
                )
                .with_target("/evidence", &entry.evidence_id),
            );
        }
    }
}

pub(crate) fn lint_claim_evidence_support(
    parsed: &ParsedMarkdownReport,
    sources: &[ReportSource],
    evidence: &[EvidenceEntry],
    stage: ExportStage,
    checks: &mut Vec<DiagnosticCheck>,
) {
    let source_lookup = SourceLookup::new(sources);
    let mut seeds = build_claim_seeds(parsed);
    let lint_as_of = "1970-01-01";
    let lint_review_after = review_after_date(lint_as_of);
    let (_, frontier_claims) = build_frontier_debates(
        parsed,
        &source_lookup,
        lint_as_of,
        &lint_review_after,
        checks,
    );
    seeds.extend(frontier_claims);

    for seed in seeds {
        let claim_id = content_id("claim", &[&seed.statement]);
        let intended_source_ids = source_lookup.resolve_ref_list_with_diagnostics(
            &seed.source_refs,
            "/report/claims/source_ids",
            &claim_id,
            checks,
        );
        if intended_source_ids.is_empty() && !seed.source_refs.is_empty() {
            continue;
        }

        let mut saw_cataloged_only = false;
        let mut saw_matching_evidence = false;
        let mut saw_satisfying_evidence = false;
        for entry in evidence {
            if !intended_source_ids.is_empty() && !intended_source_ids.contains(&entry.source_id) {
                continue;
            }
            if intended_source_ids.is_empty()
                && !entry
                    .claim_ids
                    .iter()
                    .any(|entry_claim_id| entry_claim_id == &claim_id)
            {
                continue;
            }
            if !entry.claim_ids.is_empty()
                && !entry
                    .claim_ids
                    .iter()
                    .any(|entry_claim_id| entry_claim_id == &claim_id)
            {
                continue;
            }
            saw_matching_evidence = true;
            if entry.can_satisfy_claim_link() {
                saw_satisfying_evidence = true;
            } else if entry.verification_status == VerificationStatus::Cataloged {
                saw_cataloged_only = true;
            }
        }

        if saw_satisfying_evidence || !claim_seed_requires_evidence(&seed, stage) {
            continue;
        }
        if saw_cataloged_only {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_EVIDENCE_CATALOGED_ONLY,
                    format!(
                        "claim {} only has cataloged evidence; cataloged rows cannot support claims before review",
                        claim_id
                    ),
                )
                .with_target("/report/claims", &claim_id),
            );
        } else if saw_matching_evidence || !intended_source_ids.is_empty() {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_CLAIM_NEEDS_EVIDENCE,
                    format!(
                        "claim {} has no reviewed or verified evidence with usable support metadata",
                        claim_id
                    ),
                )
                .with_target("/report/claims", &claim_id),
            );
        }
    }
}

pub(crate) fn claim_seed_requires_evidence(seed: &ClaimSeed, stage: ExportStage) -> bool {
    stage == ExportStage::Final
        || seed.evidence_requirement != EvidenceRequirement::None
        || matches!(
            seed.claim_type,
            ClaimType::Currentness | ClaimType::Frontier | ClaimType::Debate
        )
}

pub(crate) fn lint_markdown_currentness(markdown: &str, checks: &mut Vec<DiagnosticCheck>) {
    for (line_index, line) in markdown_lines_outside_fences(markdown) {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || is_likely_currentness_table_header(trimmed)
        {
            continue;
        }
        let normalized = normalize_id_text(trimmed);
        let has_temporal_word = has_currentness_words(trimmed)
            || normalized
                .split_whitespace()
                .any(|word| matches!(word, "stale" | "outdated"));
        if has_temporal_word && !line_contains_iso_date(trimmed) {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_VALIDATE_CURRENTNESS_PROSE,
                    "Markdown uses currentness or stale-date prose without an explicit YYYY-MM-DD marker",
                )
                .with_target(format!("/report/markdown/line-{}", line_index + 1), ""),
            );
        }
    }
}

pub(crate) fn is_likely_currentness_table_header(line: &str) -> bool {
    let normalized = normalize_id_text(line);
    line.starts_with('|')
        && !line.contains('.')
        && !line.contains(':')
        && (normalized.contains("current state") || normalized.contains("temporal status"))
}

pub(crate) fn line_contains_iso_date(line: &str) -> bool {
    line.as_bytes().windows(10).any(|window| {
        window[0..4].iter().all(u8::is_ascii_digit)
            && window[4] == b'-'
            && window[5..7].iter().all(u8::is_ascii_digit)
            && window[7] == b'-'
            && window[8..10].iter().all(u8::is_ascii_digit)
            && std::str::from_utf8(window)
                .map(looks_like_iso_date)
                .unwrap_or(false)
    })
}
