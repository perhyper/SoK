use super::ids::*;
use super::model::*;
use super::provenance::*;
use super::relations::*;
use super::validation::*;
use super::*;
use crate::profiles::{self, ProjectionRole};

#[derive(Debug, Clone)]
pub(crate) struct MarkdownSection {
    title: String,
    canonical: Option<CanonicalSection>,
    body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CanonicalSection {
    ResearchFrame,
    ReportArchitecture,
    DomainDecomposition,
    Orientation,
    DeepStructure,
    SourceRoleProbe,
    LiteratureLadder,
    Relations,
    CurriculumRoadmap,
    PracticeAssessment,
    FrontierDebates,
    VisualMap,
    VisualSummary,
    VisualViews,
    SourcesFurtherReading,
    SourcePointers,
    SoKUsefulnessNotes,
    ScaffoldQualityNotes,
    Claims,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedNarrativeSection {
    title: String,
    purpose: String,
    body_markdown: String,
    visual_view_ids: Vec<String>,
}

#[derive(Debug, Default)]
pub(crate) struct ParsedMarkdownReport {
    pub(crate) title_field: String,
    pub(crate) sections: BTreeSet<CanonicalSection>,
    pub(crate) research_frame: BTreeMap<String, String>,
    pub(crate) report_architecture: BTreeMap<String, String>,
    pub(crate) narrative_sections: Vec<ParsedNarrativeSection>,
    pub(crate) domain_decomposition: String,
    pub(crate) orientation: String,
    pub(crate) deep_structure_rows: Vec<BTreeMap<String, String>>,
    pub(crate) source_role_rows: Vec<BTreeMap<String, String>>,
    pub(crate) literature_ladder_rows: Vec<BTreeMap<String, String>>,
    pub(crate) relation_rows: Vec<BTreeMap<String, String>>,
    pub(crate) curriculum_rows: Vec<BTreeMap<String, String>>,
    pub(crate) frontier_rows: Vec<BTreeMap<String, String>>,
    pub(crate) visual_view_rows: Vec<BTreeMap<String, String>>,
    pub(crate) visual_summary: String,
    pub(crate) claim_rows: Vec<BTreeMap<String, String>>,
    pub(crate) practice_text: String,
    pub(crate) usefulness_notes: Vec<String>,
    pub(crate) quality_notes: Vec<String>,
    pub(crate) placeholder_lines: Vec<String>,
    pub(crate) diagnostics: Vec<DiagnosticCheck>,
}

#[derive(Debug, Clone)]
pub(crate) struct ClaimSeed {
    statement: String,
    claim_type: ClaimType,
    evidence_requirement: EvidenceRequirement,
    source_refs: Vec<String>,
    confidence: Option<ClaimConfidence>,
    notes: String,
    temporal_status: Option<TemporalStatus>,
}

pub fn export_markdown_report<P>(
    report_path: P,
    sources_path: P,
    evidence_path: Option<P>,
    stage: ExportStage,
) -> Result<ReportDocument>
where
    P: AsRef<Path>,
{
    let report_path = report_path.as_ref();
    let sources_path = sources_path.as_ref();
    let markdown = fs::read_to_string(report_path)
        .with_context(|| format!("read report Markdown {}", report_path.display()))?;
    let mut parsed = parse_markdown_report(&markdown);

    let normalized_sources = normalize_source_manifest(sources_path)?;
    let mut sources = normalized_sources.sources;
    let mut evidence = normalized_sources.evidence;
    let mut diagnostics = Vec::new();
    diagnostics.append(&mut parsed.diagnostics);
    diagnostics.extend(normalized_sources.diagnostics);
    validate_markdown_report_architecture(&parsed, stage, &mut diagnostics);

    if let Some(path) = evidence_path.as_ref() {
        let mut reviewed_evidence: Vec<EvidenceEntry> = read_jsonl_file(path.as_ref())?;
        evidence.append(&mut reviewed_evidence);
    }

    let source_ids = sources
        .iter()
        .map(|source| source.id.clone())
        .collect::<BTreeSet<_>>();
    diagnostics.extend(evidence_source_diagnostics(&evidence, &source_ids));
    apply_evidence_review_status(&mut sources, &evidence);

    let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let as_of = generated_at
        .split('T')
        .next()
        .unwrap_or("1970-01-01")
        .to_string();
    let review_after = review_after_date(&as_of);
    let field = infer_report_field(&parsed);
    if field == "Unknown field" {
        diagnostics.push(DiagnosticCheck::warning(
            CHECK_EXPORT_MISSING_FIELD,
            "could not infer report field from canonical title or Research Frame table",
        ));
    }

    let source_lookup = SourceLookup::new(&sources);
    let scope = build_scope(&parsed, &mut diagnostics);
    let domain_profile = build_domain_profile(&parsed, stage, &mut diagnostics);
    let field_elements = build_field_elements(&parsed, &source_lookup, &mut diagnostics);
    let (core_ideas, methods, representations) = build_knowledge_items(&field_elements, &parsed);
    let evidence_standards = build_evidence_standards(&parsed);
    let literature_ladder = build_literature_ladder(&parsed, &source_lookup, &mut diagnostics);
    let mut curriculum_path = build_curriculum_path(&parsed, &source_lookup, &mut diagnostics);
    let (frontier_debates, frontier_claims) = build_frontier_debates(
        &parsed,
        &source_lookup,
        &as_of,
        &review_after,
        &mut diagnostics,
    );
    let mut claim_seeds = build_claim_seeds(&parsed);
    claim_seeds.extend(frontier_claims);
    let claims = build_claims(
        claim_seeds,
        &source_lookup,
        &evidence,
        stage,
        &as_of,
        &review_after,
        &mut diagnostics,
    );
    let mut entity_index = ExportEntityIndex::new(
        &core_ideas,
        &methods,
        &representations,
        &sources,
        &claims,
        &curriculum_path,
        &frontier_debates,
    );
    entity_index.add_field_elements(&field_elements);
    apply_curriculum_prerequisites(
        &parsed,
        &mut curriculum_path,
        &entity_index,
        &mut diagnostics,
    );
    let relations = build_relations(&parsed, &entity_index, &source_lookup, &mut diagnostics);
    let visual_views = build_visual_views(&parsed, &relations, &entity_index, &mut diagnostics);
    warn_if_unpreserved_visual_sections(&parsed, &visual_views, &mut diagnostics);

    warn_if_empty_public_sections(
        &field_elements,
        &core_ideas,
        &methods,
        &representations,
        &mut diagnostics,
    );

    if stage == ExportStage::Final {
        for section in [
            CanonicalSection::ResearchFrame,
            CanonicalSection::ScaffoldQualityNotes,
            CanonicalSection::SoKUsefulnessNotes,
        ] {
            if parsed_has_section(&parsed, section) {
                diagnostics.push(DiagnosticCheck::warning(
                    CHECK_EXPORT_INTERNAL_SECTION_IN_FINAL,
                    format!(
                        "final-stage export ignored internal scaffold section {}",
                        canonical_section_label(section)
                    ),
                ));
            }
        }
    }

    let report_type = match stage {
        ExportStage::Scaffold => ReportType::Scaffold,
        ExportStage::Final => ReportType::HumanReport,
    };
    let presentation = match stage {
        ExportStage::Scaffold => None,
        ExportStage::Final => build_report_presentation(&parsed, &visual_views, &mut diagnostics),
    };
    let mut document = ReportDocument {
        metadata: ReportMetadata {
            schema_version: "sok-report/v2".to_string(),
            generated_at,
            report_type,
            temporal_review: TemporalMarker {
                as_of: as_of.clone(),
                review_after: match stage {
                    ExportStage::Scaffold => String::new(),
                    ExportStage::Final => review_after.clone(),
                },
                temporal_status: match stage {
                    ExportStage::Scaffold => TemporalStatus::Unknown,
                    ExportStage::Final => TemporalStatus::Current,
                },
                rationale: match stage {
                    ExportStage::Scaffold => {
                        "Exported from a bounded scaffold before final evidence validation."
                            .to_string()
                    }
                    ExportStage::Final => {
                        "Exported from a bounded human report and source/evidence files."
                            .to_string()
                    }
                },
            },
            generator: Some(GeneratorInfo {
                name: "sok export-json".to_string(),
                version: "1".to_string(),
            }),
        },
        report: PublicReport {
            field,
            scope,
            domain_profile,
            presentation,
            literature_ladder,
            field_elements,
            core_ideas,
            methods,
            representations,
            evidence_standards,
            sources,
            claims,
            relations,
            curriculum_path,
            frontier_debates,
            visual_views,
            structure_waivers: Vec::new(),
        },
        internal_context: match stage {
            ExportStage::Scaffold => Some(build_internal_context(&parsed)),
            ExportStage::Final => None,
        },
        diagnostics: None,
    };

    if !diagnostics.is_empty() {
        document.diagnostics = Some(Diagnostics {
            summary: format!(
                "export-json emitted {} diagnostic(s) while converting bounded Markdown.",
                diagnostics.len()
            ),
            checks: diagnostics,
        });
    }

    Ok(document)
}

pub(crate) fn parse_markdown_report(markdown: &str) -> ParsedMarkdownReport {
    let mut parsed = ParsedMarkdownReport {
        title_field: infer_title_field(markdown),
        placeholder_lines: collect_placeholder_lines(markdown),
        ..ParsedMarkdownReport::default()
    };
    let sections = parse_markdown_sections(markdown, &mut parsed.diagnostics);
    let mut seen = BTreeSet::new();

    for section in sections {
        let public_body = strip_sok_directives(&section.body);
        let visual_view_ids = sok_directive_values(&section.body, "visual-view");
        if is_public_narrative_section(section.canonical) {
            parsed.narrative_sections.push(ParsedNarrativeSection {
                title: section.title.clone(),
                purpose: first_sok_directive_value(&section.body, "purpose"),
                body_markdown: public_body,
                visual_view_ids,
            });
        }

        let Some(canonical) = section.canonical else {
            // Arbitrary H2 sections form the ordered public narrative. The
            // canonical surfaces below remain available for machine extraction
            // without dictating the report's table of contents.
            continue;
        };
        parsed.sections.insert(canonical);
        if !seen.insert(canonical) {
            parsed.diagnostics.push(DiagnosticCheck::warning(
                CHECK_EXPORT_AMBIGUOUS_SECTION,
                format!(
                    "duplicate canonical Markdown section {}; only deterministic table extraction is supported",
                    canonical_section_label(canonical)
                ),
            ));
        }

        match canonical {
            CanonicalSection::ResearchFrame => {
                parsed
                    .research_frame
                    .extend(parse_key_value_table(&section.body));
            }
            CanonicalSection::ReportArchitecture => {
                parsed
                    .report_architecture
                    .extend(parse_key_value_table(&section.body));
            }
            CanonicalSection::DomainDecomposition => {
                parsed.domain_decomposition = public_section_text(&section.body);
            }
            CanonicalSection::Orientation => {
                parsed.orientation = public_section_text(&section.body);
            }
            CanonicalSection::DeepStructure => {
                parsed
                    .deep_structure_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::SourceRoleProbe => {
                parsed
                    .source_role_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::LiteratureLadder => {
                parsed
                    .literature_ladder_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::Relations => {
                parsed
                    .relation_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::CurriculumRoadmap => {
                parsed
                    .curriculum_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::FrontierDebates => {
                parsed
                    .frontier_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::Claims => {
                parsed
                    .claim_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::PracticeAssessment => {
                parsed.practice_text = public_section_text(&section.body);
            }
            CanonicalSection::SoKUsefulnessNotes => {
                parsed.usefulness_notes = bullet_or_paragraph_lines(&section.body);
            }
            CanonicalSection::ScaffoldQualityNotes => {
                parsed.quality_notes = bullet_or_paragraph_lines(&section.body);
            }
            CanonicalSection::VisualSummary => {
                parsed.visual_summary = public_section_text(&section.body);
            }
            CanonicalSection::VisualViews => {
                parsed
                    .visual_view_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::VisualMap => {
                // Mermaid or prose maps are recognized here but intentionally not
                // interpreted as semantic relations. Export diagnostics are emitted
                // later if no structured Visual Views table preserves the intent.
            }
            CanonicalSection::SourcesFurtherReading | CanonicalSection::SourcePointers => {
                parsed.diagnostics.push(DiagnosticCheck::warning(
                    CHECK_EXPORT_UNSUPPORTED_SECTION,
                    format!(
                        "Markdown source section {} was ignored; --sources is the source-of-truth manifest",
                        canonical_section_label(canonical)
                    ),
                ));
            }
        }
    }

    parsed
}

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
                "final Markdown needs at least one complete field-element inventory row so the architecture is grounded in observed field forms",
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
                    "field-element inventory needs at least one surrounding or context element needed to make the core intelligible",
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
                                "conditional source role {} has {count} source(s), minimum is {minimum}; record a waiver rationale if the condition is not active",
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

pub(crate) fn parse_markdown_sections(
    markdown: &str,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<MarkdownSection> {
    let mut sections = Vec::new();
    let mut current_title = String::new();
    let mut current_body = Vec::new();
    let mut fence = None;

    for line in markdown.lines() {
        let trimmed = line.trim();
        let was_fenced = fence.is_some();
        let is_fence_delimiter = update_markdown_fence(line, &mut fence);
        let is_fenced = was_fenced || is_fence_delimiter || fence.is_some();
        if !is_fenced && trimmed.starts_with("## ") && !trimmed.starts_with("### ") {
            if !current_title.is_empty() {
                let body = current_body.join("\n");
                push_markdown_section(&mut sections, current_title, body, diagnostics);
                current_body.clear();
            }
            current_title = trimmed.trim_start_matches('#').trim().to_string();
        } else if !current_title.is_empty() {
            current_body.push(line.to_string());
        }
    }

    if fence.is_some() {
        diagnostics.push(DiagnosticCheck::warning(
            CHECK_EXPORT_UNSUPPORTED_SECTION,
            "Markdown has an unclosed fenced code block; fenced content was not interpreted",
        ));
    }
    if !current_title.is_empty() {
        let body = current_body.join("\n");
        push_markdown_section(&mut sections, current_title, body, diagnostics);
    }
    sections
}

pub(crate) fn push_markdown_section(
    sections: &mut Vec<MarkdownSection>,
    title: String,
    body: String,
    diagnostics: &mut Vec<DiagnosticCheck>,
) {
    for (key, value) in standalone_sok_directives(&body) {
        if !matches!(key.as_str(), "surface" | "purpose" | "visual-view") {
            diagnostics.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_UNKNOWN_DIRECTIVE,
                    format!("Markdown section {title:?} has unknown sok directive {key:?}"),
                )
                .with_target("/report/sections", &key),
            );
        } else if value.is_empty() {
            diagnostics.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_UNKNOWN_DIRECTIVE,
                    format!("Markdown section {title:?} has empty sok:{key} directive"),
                )
                .with_target("/report/sections", &key),
            );
        }
    }
    let markers = sok_directive_values(&body, "surface");
    let canonical_from_marker = canonical_section_from_marker(&body);
    if markers.len() > 1 {
        diagnostics.push(
            DiagnosticCheck::error(
                CHECK_EXPORT_AMBIGUOUS_SECTION,
                format!(
                    "Markdown section {title:?} has multiple sok:surface markers: {}",
                    markers.join(", ")
                ),
            )
            .with_target("/report/sections", &title),
        );
    }
    if let Some(marker) = markers.first() {
        if canonical_from_marker.is_none() {
            diagnostics.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_UNKNOWN_SURFACE_MARKER,
                    format!("Markdown section {title:?} has unknown sok:surface marker {marker:?}"),
                )
                .with_target("/report/sections", marker),
            );
        }
    }
    sections.push(MarkdownSection {
        canonical: canonical_from_marker.or_else(|| canonical_section(&title)),
        title,
        body,
    });
}

pub(crate) fn canonical_section_from_marker(body: &str) -> Option<CanonicalSection> {
    let surface = first_sok_directive_value(body, "surface");
    match normalize_id_text(&surface).as_str() {
        "report architecture" | "architecture" => Some(CanonicalSection::ReportArchitecture),
        "domain decomposition" => Some(CanonicalSection::DomainDecomposition),
        "orientation" => Some(CanonicalSection::Orientation),
        "deep structure" | "field elements" | "element inventory" => {
            Some(CanonicalSection::DeepStructure)
        }
        "source role probe" => Some(CanonicalSection::SourceRoleProbe),
        "literature ladder" => Some(CanonicalSection::LiteratureLadder),
        "relations" => Some(CanonicalSection::Relations),
        "curriculum roadmap" | "curriculum" => Some(CanonicalSection::CurriculumRoadmap),
        "practice assessment" | "practice" => Some(CanonicalSection::PracticeAssessment),
        "frontier debates" | "frontier" => Some(CanonicalSection::FrontierDebates),
        "visual map" => Some(CanonicalSection::VisualMap),
        "visual summary" => Some(CanonicalSection::VisualSummary),
        "visual views" => Some(CanonicalSection::VisualViews),
        "claims" => Some(CanonicalSection::Claims),
        _ => None,
    }
}

pub(crate) fn is_public_narrative_section(section: Option<CanonicalSection>) -> bool {
    matches!(
        section,
        None | Some(CanonicalSection::DomainDecomposition)
            | Some(CanonicalSection::Orientation)
            | Some(CanonicalSection::DeepStructure)
            | Some(CanonicalSection::CurriculumRoadmap)
            | Some(CanonicalSection::PracticeAssessment)
            | Some(CanonicalSection::FrontierDebates)
    )
}

pub(crate) fn sok_directive_values(body: &str, key: &str) -> Vec<String> {
    standalone_sok_directives(body)
        .into_iter()
        .filter_map(|(candidate, value)| (candidate == key).then_some(value))
        .filter(|value| !value.is_empty())
        .collect()
}

pub(crate) fn standalone_sok_directives(body: &str) -> Vec<(String, String)> {
    let mut fence = None;
    let mut values = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if update_markdown_fence(line, &mut fence) {
            continue;
        }
        if fence.is_some() {
            continue;
        }
        if let Some(directive) = trimmed
            .strip_prefix("<!-- sok:")
            .and_then(|value| value.strip_suffix("-->"))
            .map(str::trim)
        {
            let mut parts = directive.splitn(2, char::is_whitespace);
            let key = parts.next().unwrap_or_default().trim().to_string();
            let value = parts.next().unwrap_or_default().trim().to_string();
            values.push((key, value));
        }
    }
    values
}

pub(crate) fn first_sok_directive_value(body: &str, key: &str) -> String {
    sok_directive_values(body, key)
        .into_iter()
        .next()
        .unwrap_or_default()
}

pub(crate) fn strip_sok_directives(body: &str) -> String {
    let mut fence = None;
    let mut lines = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if update_markdown_fence(line, &mut fence) {
            lines.push(line);
            continue;
        }
        if fence.is_none() && trimmed.starts_with("<!-- sok:") && trimmed.ends_with("-->") {
            continue;
        }
        lines.push(line);
    }
    lines.join("\n").trim().to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MarkdownFence {
    marker: char,
    length: usize,
}

pub(crate) fn update_markdown_fence(line: &str, fence: &mut Option<MarkdownFence>) -> bool {
    let trimmed = line.trim();
    let Some(marker) = trimmed.chars().next() else {
        return false;
    };
    if !matches!(marker, '`' | '~') {
        return false;
    }
    let length = trimmed.chars().take_while(|ch| *ch == marker).count();
    if length < 3 {
        return false;
    }
    let remainder = &trimmed[length..];

    if let Some(active) = *fence {
        if marker == active.marker && length >= active.length && remainder.trim().is_empty() {
            *fence = None;
            return true;
        }
        return false;
    }

    // CommonMark does not allow a backtick in the info string of a
    // backtick-delimited code fence.
    if marker == '`' && remainder.contains('`') {
        return false;
    }
    *fence = Some(MarkdownFence { marker, length });
    true
}

pub(crate) fn markdown_lines_outside_fences(markdown: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut fence = None;
    markdown.lines().enumerate().filter(move |(_, line)| {
        if update_markdown_fence(line, &mut fence) {
            return false;
        }
        fence.is_none()
    })
}

pub(crate) fn canonical_section(title: &str) -> Option<CanonicalSection> {
    let title = normalize_heading_title(title);
    match title.as_str() {
        "research frame" | "internal context" => Some(CanonicalSection::ResearchFrame),
        "report architecture" | "report architecture decision" | "narrative architecture" => {
            Some(CanonicalSection::ReportArchitecture)
        }
        "domain decomposition" => Some(CanonicalSection::DomainDecomposition),
        "orientation" | "executive orientation" => Some(CanonicalSection::Orientation),
        "the field s deep structure"
        | "field s deep structure"
        | "deep structure"
        | "field element inventory" => Some(CanonicalSection::DeepStructure),
        "source role probe" => Some(CanonicalSection::SourceRoleProbe),
        "literature ladder" | "literature ladder with access metadata" => {
            Some(CanonicalSection::LiteratureLadder)
        }
        "relations" | "relation table" | "semantic relations" | "knowledge relations" => {
            Some(CanonicalSection::Relations)
        }
        "curriculum roadmap" | "research roadmap" => Some(CanonicalSection::CurriculumRoadmap),
        "practice and assessment" => Some(CanonicalSection::PracticeAssessment),
        "frontier debates and open problems"
        | "frontier debate map"
        | "frontier and debate map"
        | "frontier and debates"
        | "frontier debates"
        | "frontier and debate"
        | "frontier open problems and debates" => Some(CanonicalSection::FrontierDebates),
        "visual summary" => Some(CanonicalSection::VisualSummary),
        "visual views" | "visual view declarations" | "visual declarations" => {
            Some(CanonicalSection::VisualViews)
        }
        "concept and prerequisite map"
        | "debate and case network"
        | "instrument data and standards map"
        | "concept map" => Some(CanonicalSection::VisualMap),
        "sources and further reading" => Some(CanonicalSection::SourcesFurtherReading),
        "source pointers" => Some(CanonicalSection::SourcePointers),
        "sok usefulness notes" => Some(CanonicalSection::SoKUsefulnessNotes),
        "scaffold quality notes" => Some(CanonicalSection::ScaffoldQualityNotes),
        "claims" | "evidence backed claims" | "public claims" => Some(CanonicalSection::Claims),
        _ => None,
    }
}

pub(crate) fn canonical_section_label(section: CanonicalSection) -> &'static str {
    match section {
        CanonicalSection::ResearchFrame => "Research Frame",
        CanonicalSection::ReportArchitecture => "Report Architecture",
        CanonicalSection::DomainDecomposition => "Domain Decomposition",
        CanonicalSection::Orientation => "Orientation",
        CanonicalSection::DeepStructure => "Deep Structure",
        CanonicalSection::SourceRoleProbe => "Source Role Probe",
        CanonicalSection::LiteratureLadder => "Literature Ladder",
        CanonicalSection::Relations => "Relations",
        CanonicalSection::CurriculumRoadmap => "Curriculum Roadmap",
        CanonicalSection::PracticeAssessment => "Practice and Assessment",
        CanonicalSection::FrontierDebates => "Frontier, Debates, and Open Problems",
        CanonicalSection::VisualMap => "Visual Map",
        CanonicalSection::VisualSummary => "Visual Summary",
        CanonicalSection::VisualViews => "Visual Views",
        CanonicalSection::SourcesFurtherReading => "Sources and Further Reading",
        CanonicalSection::SourcePointers => "Source Pointers",
        CanonicalSection::SoKUsefulnessNotes => "SoK Usefulness Notes",
        CanonicalSection::ScaffoldQualityNotes => "Scaffold Quality Notes",
        CanonicalSection::Claims => "Claims",
    }
}

pub(crate) fn normalize_heading_title(title: &str) -> String {
    let normalized = normalize_id_text(title);
    normalized
        .trim_start_matches(|ch: char| ch.is_ascii_digit() || ch.is_whitespace())
        .trim()
        .to_string()
}

pub(crate) fn parse_key_value_table(markdown: &str) -> BTreeMap<String, String> {
    parse_first_markdown_table(markdown)
        .into_iter()
        .filter_map(|row| {
            let key = lookup_cell(&row, &["item", "key", "field", "label"]);
            let value = lookup_cell(
                &row,
                &["value", "decision", "so k extraction", "in this field"],
            );
            if key.is_empty() || value.is_empty() {
                None
            } else {
                Some((key, value))
            }
        })
        .collect()
}

pub(crate) fn parse_first_markdown_table(markdown: &str) -> Vec<BTreeMap<String, String>> {
    let mut header: Vec<String> = Vec::new();
    let mut rows = Vec::new();
    let mut in_table = false;

    for (_, line) in markdown_lines_outside_fences(markdown) {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            if in_table && !trimmed.is_empty() {
                break;
            }
            continue;
        }
        let cells = parse_table_cells(trimmed);
        if cells.is_empty() || is_table_separator(&cells) {
            continue;
        }
        if header.is_empty() {
            header = cells
                .into_iter()
                .map(|cell| normalize_id_text(&cell))
                .collect();
            in_table = true;
            continue;
        }
        let mut row = BTreeMap::new();
        for (index, value) in cells.into_iter().enumerate() {
            let key = header
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("column_{index}"));
            row.insert(key, clean_inline_markdown(&value));
        }
        if !row.values().all(|value| value.trim().is_empty()) {
            rows.push(row);
        }
    }

    rows
}

pub(crate) fn parse_table_cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

pub(crate) fn is_table_separator(cells: &[String]) -> bool {
    cells.iter().all(|cell| {
        let value = cell.trim();
        !value.is_empty() && value.chars().all(|ch| matches!(ch, '-' | ':' | ' ' | '\t'))
    })
}

pub(crate) fn public_section_text(markdown: &str) -> String {
    collapse_whitespace(
        &markdown_lines_outside_fences(markdown)
            .filter_map(|(_, line)| {
                let trimmed = line.trim();
                (!trimmed.is_empty() && !trimmed.starts_with('|') && !is_placeholder_text(trimmed))
                    .then_some(line)
            })
            .map(clean_inline_markdown)
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub(crate) fn bullet_or_paragraph_lines(markdown: &str) -> Vec<String> {
    markdown_lines_outside_fences(markdown)
        .map(|(_, line)| {
            line.trim()
                .trim_start_matches("- ")
                .trim_start_matches("* ")
                .trim()
                .to_string()
        })
        .filter(|line| !line.is_empty() && !line.starts_with('|'))
        .map(|line| clean_inline_markdown(&line))
        .collect()
}

pub(crate) fn clean_inline_markdown(raw: &str) -> String {
    collapse_whitespace(
        &raw.replace('`', "")
            .replace("**", "")
            .replace("__", "")
            .replace("<br>", "; ")
            .replace("<br/>", "; ")
            .replace("<br />", "; "),
    )
}

pub(crate) fn collapse_whitespace(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn collect_placeholder_lines(markdown: &str) -> Vec<String> {
    markdown_lines_outside_fences(markdown)
        .map(|(_, line)| line.trim())
        .filter(|line| is_placeholder_text(line))
        .map(clean_inline_markdown)
        .collect()
}

pub(crate) fn is_placeholder_text(text: &str) -> bool {
    let normalized = normalize_id_text(text);
    normalized.contains("source to verify")
        || normalized.contains("date after lookup")
        || normalized.contains("budget unknown")
        || normalized.contains("until verified")
        || normalized.contains("replace starter source")
        || normalized.contains("scaffold quality")
        || normalized.contains("sentinel internal")
}

pub(crate) fn lookup_cell(row: &BTreeMap<String, String>, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| row.get(*key))
        .cloned()
        .unwrap_or_default()
        .trim()
        .to_string()
}

pub(crate) fn lookup_cell_any(row: &BTreeMap<String, String>, keys: &[&str]) -> String {
    for key in keys {
        let normalized = normalize_id_text(key);
        if let Some(value) = row.get(&normalized) {
            return value.trim().to_string();
        }
    }
    String::new()
}

pub(crate) fn infer_title_field(markdown: &str) -> String {
    for (_, line) in markdown_lines_outside_fences(markdown) {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("# Structure of Knowledge:") {
            return clean_inline_markdown(value);
        }
        if let Some(value) = line.strip_prefix("# SoK v0 Sample:") {
            return clean_inline_markdown(value);
        }
        if line.starts_with("# SoK") && line.contains(':') {
            if let Some((_, value)) = line.split_once(':') {
                return clean_inline_markdown(value);
            }
        }
    }
    String::new()
}

pub(crate) fn infer_report_field(parsed: &ParsedMarkdownReport) -> String {
    first_non_empty([
        parsed
            .research_frame
            .get("Field")
            .map_or("", String::as_str),
        parsed.title_field.as_str(),
        "Unknown field",
    ])
}

pub(crate) fn build_scope(
    parsed: &ParsedMarkdownReport,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Scope {
    let mut included = split_listish(&first_non_empty([
        parsed
            .research_frame
            .get("Scope included")
            .map_or("", String::as_str),
        parsed
            .research_frame
            .get("Included")
            .map_or("", String::as_str),
    ]));
    let excluded = split_listish(&first_non_empty([
        parsed
            .research_frame
            .get("Scope excluded")
            .map_or("", String::as_str),
        parsed
            .research_frame
            .get("Excluded")
            .map_or("", String::as_str),
    ]));
    if included.is_empty() {
        included.push(infer_report_field(parsed));
    }

    let architecture_thesis =
        architecture_value(&parsed.report_architecture, &["Executive thesis", "Thesis"]);
    let first_narrative = parsed
        .narrative_sections
        .first()
        .map(|section| public_section_text(&section.body_markdown))
        .unwrap_or_default();
    let summary = first_non_empty([
        parsed.domain_decomposition.as_str(),
        parsed.orientation.as_str(),
        architecture_thesis.as_str(),
        first_narrative.as_str(),
        "Bounded SoK export; public scope could not be inferred from the report narrative.",
    ]);
    if parsed.domain_decomposition.is_empty()
        && parsed.orientation.is_empty()
        && architecture_thesis.is_empty()
        && first_narrative.is_empty()
    {
        diagnostics.push(DiagnosticCheck::warning(
            CHECK_EXPORT_MISSING_PUBLIC_FIELD,
            "could not infer a public scope summary from report architecture or narrative",
        ));
    }

    let mut interpretive_notes = Vec::new();
    if !parsed.orientation.is_empty() && parsed.orientation != summary {
        interpretive_notes.push(parsed.orientation.clone());
    }

    Scope {
        summary,
        included,
        excluded,
        assumptions: Vec::new(),
        interpretive_notes,
    }
}

pub(crate) fn build_domain_profile(
    parsed: &ParsedMarkdownReport,
    stage: ExportStage,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> DomainProfile {
    let declared_profile = architecture_value(
        &parsed.report_architecture,
        &["Domain profile", "Domain classification"],
    );
    let raw = match stage {
        ExportStage::Scaffold => first_non_empty([
            parsed
                .research_frame
                .get("Domain classification")
                .map_or("", String::as_str),
            parsed
                .research_frame
                .get("Domain type")
                .map_or("", String::as_str),
            parsed
                .research_frame
                .get("Provisional lens")
                .map_or("", String::as_str),
            parsed.domain_decomposition.as_str(),
        ]),
        ExportStage::Final => first_non_empty([
            declared_profile.as_str(),
            parsed.domain_decomposition.as_str(),
        ]),
    };
    if raw.is_empty() && parsed.narrative_sections.is_empty() {
        diagnostics.push(DiagnosticCheck::warning(
            CHECK_EXPORT_MISSING_PUBLIC_FIELD,
            "could not infer domain classification; using mixed",
        ));
    }
    let classifications = domain_classifications_from_text(&raw);
    let classification = classifications
        .first()
        .copied()
        .unwrap_or(DomainClassification::Mixed);
    let secondary_characteristics = classifications
        .into_iter()
        .filter(|candidate| *candidate != classification)
        .collect();
    let architecture_rationale = architecture_value(
        &parsed.report_architecture,
        &["Architecture rationale", "Rationale", "Why this form"],
    );
    let rationale = first_non_empty([
        parsed.domain_decomposition.as_str(),
        raw.as_str(),
        architecture_rationale.as_str(),
        "No single domain label was allowed to determine the report architecture.",
    ]);
    let failure_modes = parsed
        .deep_structure_rows
        .iter()
        .find(|row| {
            normalize_id_text(&lookup_cell_any(
                row,
                &["Observed element", "Element", "Element class"],
            ))
            .contains("failure mode")
        })
        .map(|row| {
            split_listish(&first_non_empty([
                lookup_cell_any(row, &["SoK extraction"]).as_str(),
                lookup_cell_any(row, &["In this field"]).as_str(),
                lookup_cell_any(row, &["Actual form in this field"]).as_str(),
                lookup_cell_any(row, &["Why it matters"]).as_str(),
            ]))
        })
        .unwrap_or_default();

    DomainProfile {
        classification,
        secondary_characteristics,
        rationale,
        failure_modes,
    }
}

pub(crate) fn domain_classifications_from_text(raw: &str) -> Vec<DomainClassification> {
    let mut out = Vec::new();
    for classification_id in profiles::domain_classification_ids_from_text(raw) {
        if let Some(classification) = domain_classification_from_id(&classification_id) {
            out.push(classification);
        }
    }
    out
}

pub(crate) fn domain_classification_from_id(raw: &str) -> Option<DomainClassification> {
    match raw {
        "formal" => Some(DomainClassification::Formal),
        "well_structured" => Some(DomainClassification::WellStructured),
        "ill_structured" => Some(DomainClassification::IllStructured),
        "professional_practice" => Some(DomainClassification::ProfessionalPractice),
        "instrument_bound" => Some(DomainClassification::InstrumentBound),
        "infrastructure_bound" => Some(DomainClassification::InfrastructureBound),
        "emerging" => Some(DomainClassification::Emerging),
        "interdisciplinary" => Some(DomainClassification::Interdisciplinary),
        "mixed" => Some(DomainClassification::Mixed),
        _ => None,
    }
}

pub(crate) fn build_field_elements(
    parsed: &ParsedMarkdownReport,
    source_lookup: &SourceLookup,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<FieldElement> {
    let mut elements = Vec::new();
    for (index, row) in parsed.deep_structure_rows.iter().enumerate() {
        if !is_usable_deep_structure_row(row) {
            continue;
        }
        let label = lookup_cell_any(row, &["Observed element", "Element", "Name", "Title"]);
        let explicit_id =
            clean_inline_markdown(&lookup_cell_any(row, &["Element ID", "Element Id", "ID"]));
        let id = if explicit_id.trim().is_empty() {
            content_id("element", &[&label])
        } else {
            explicit_id
        };
        let fallback_actual_form = deep_structure_description(row);
        let actual_form = first_non_empty([
            lookup_cell_any(
                row,
                &[
                    "Actual form in this field",
                    "In this field",
                    "SoK extraction",
                    "First-pass scholarly representation",
                    "Research-grade representation",
                    "Why it matters",
                ],
            )
            .as_str(),
            fallback_actual_form.as_str(),
        ]);
        let source_refs =
            lookup_cell_any(row, &["Source IDs", "Sources", "Key sources", "Readings"]);
        elements.push(FieldElement {
            id: id.clone(),
            element_class: lookup_cell_any(row, &["Element class", "Class", "Element type"]),
            label,
            actual_form,
            role: lookup_cell_any(
                row,
                &[
                    "Role",
                    "Role: core / surrounding / context",
                    "Structural role",
                ],
            ),
            load_bearing_relations: lookup_cell_any(
                row,
                &[
                    "Load-bearing relations",
                    "Load bearing relations",
                    "Key relations",
                    "Relations",
                ],
            ),
            source_ids: source_lookup.resolve_refs_with_diagnostics(
                &source_refs,
                format!("/report/field_elements/{index}/source_ids"),
                &id,
                diagnostics,
            ),
            confidence: parse_claim_confidence(&lookup_cell_any(row, &["Confidence"])),
        });
    }
    elements
}

pub(crate) fn build_knowledge_items(
    field_elements: &[FieldElement],
    parsed: &ParsedMarkdownReport,
) -> (Vec<KnowledgeItem>, Vec<KnowledgeItem>, Vec<KnowledgeItem>) {
    let mut core_ideas = Vec::new();
    let mut methods = Vec::new();
    let mut representations = Vec::new();
    for element in field_elements {
        let element_class =
            first_non_empty([element.element_class.as_str(), element.label.as_str()]);
        let projection = profiles::projection_for_element_class(&element_class);
        let description = collapse_whitespace(
            &[
                element.actual_form.as_str(),
                element.role.as_str(),
                element.load_bearing_relations.as_str(),
            ]
            .into_iter()
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>()
            .join("; "),
        );
        let item = KnowledgeItem {
            id: content_id(&projection.item_prefix, &[&element.label]),
            label: element.label.clone(),
            description,
            source_ids: element.source_ids.clone(),
            ..KnowledgeItem::default()
        };
        match projection.role {
            ProjectionRole::Representation => representations.push(item),
            ProjectionRole::Method => methods.push(item),
            ProjectionRole::Omit => continue,
            ProjectionRole::CoreIdea => core_ideas.push(item),
        }
    }

    if core_ideas.is_empty() && !parsed.orientation.is_empty() {
        core_ideas.push(KnowledgeItem {
            id: content_id("concept", &[&parsed.orientation]),
            label: "Orientation thesis".to_string(),
            description: parsed.orientation.clone(),
            ..KnowledgeItem::default()
        });
    }
    if methods.is_empty() && !parsed.practice_text.is_empty() {
        methods.push(KnowledgeItem {
            id: content_id("method", &[&parsed.practice_text]),
            label: "Practice and assessment method".to_string(),
            description: parsed.practice_text.clone(),
            ..KnowledgeItem::default()
        });
    }
    (core_ideas, methods, representations)
}

pub(crate) fn deep_structure_description(row: &BTreeMap<String, String>) -> String {
    let mut parts = Vec::new();
    for key in [
        "sok extraction",
        "so k extraction",
        "in this field",
        "actual form in this field",
        "role",
        "role core surrounding context",
        "load bearing relations",
        "first pass scholarly representation",
        "research grade representation",
        "why it matters",
    ] {
        if let Some(value) = row.get(key) {
            if !value.trim().is_empty() {
                parts.push(value.trim().to_string());
            }
        }
    }
    collapse_whitespace(&parts.join("; "))
}

pub(crate) fn is_usable_deep_structure_row(row: &BTreeMap<String, String>) -> bool {
    let observed_element = lookup_cell_any(row, &["Observed element", "Element", "Name", "Title"]);
    let description = deep_structure_description(row);
    !observed_element.is_empty() && !description.is_empty() && !is_placeholder_text(&description)
}

pub(crate) fn build_evidence_standards(parsed: &ParsedMarkdownReport) -> EvidenceStandards {
    let mut requirements = Vec::new();
    for row in &parsed.source_role_rows {
        let role_text = lookup_cell_any(row, &["Source role", "Role"]);
        if role_text.trim().is_empty() {
            continue;
        }
        let requirement_text = lookup_cell_any(row, &["Status", "Requirement"]);
        let requirement = parse_source_requirement(&requirement_text);
        let rationale = first_non_empty([
            lookup_cell_any(row, &["What it tests"]).as_str(),
            lookup_cell_any(row, &["Candidate source pattern"]).as_str(),
            lookup_cell_any(row, &["Waiver or revision rule"]).as_str(),
            "Requirement imported from Source Role Probe.",
        ]);
        let waiver = if requirement == SourceRoleRequirementKind::Waived {
            Some(SourceRoleWaiver {
                rationale: first_non_empty([
                    lookup_cell_any(row, &["Waiver or revision rule"]).as_str(),
                    rationale.as_str(),
                ]),
                as_of: Utc::now()
                    .to_rfc3339_opts(SecondsFormat::Secs, true)
                    .split('T')
                    .next()
                    .unwrap_or("1970-01-01")
                    .to_string(),
                review_after: String::new(),
            })
        } else {
            None
        };
        requirements.push(SourceRoleRequirement {
            role: parse_source_role(&role_text),
            requirement,
            minimum_sources: match requirement {
                SourceRoleRequirementKind::Required | SourceRoleRequirementKind::Conditional => {
                    Some(1)
                }
                SourceRoleRequirementKind::Waived | SourceRoleRequirementKind::NotApplicable => {
                    Some(0)
                }
            },
            rationale,
            waiver,
        });
    }

    EvidenceStandards {
        summary: "Sources are cataloged from the manifest; claim support requires reviewed or verified evidence links.".to_string(),
        claim_policy: "Cataloged source records remain visible as source records but do not satisfy claim evidence requirements.".to_string(),
        source_role_requirements: requirements,
    }
}

pub(crate) fn build_literature_ladder(
    parsed: &ParsedMarkdownReport,
    source_lookup: &SourceLookup,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<LiteratureLadderRow> {
    let mut rows = Vec::new();
    for (index, row) in parsed.literature_ladder_rows.iter().enumerate() {
        let layer = lookup_cell_any(row, &["Layer", "Level"]);
        let start_here = lookup_cell_any(row, &["Start here", "Start", "Entry point"]);
        let read_for = lookup_cell_any(row, &["Read for", "Read for / use for", "Use for"]);
        let do_not_infer = lookup_cell_any(row, &["Do not infer", "Do-not-infer", "Caveat"]);
        let missing = [
            ("Layer", layer.as_str()),
            ("Start here", start_here.as_str()),
            ("Read for", read_for.as_str()),
            ("Do not infer", do_not_infer.as_str()),
        ]
        .into_iter()
        .filter_map(|(field, value)| value.trim().is_empty().then_some(field))
        .collect::<Vec<_>>();
        if !missing.is_empty() {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_MISSING_PUBLIC_FIELD,
                    format!(
                        "Literature Ladder row {} is missing required field(s): {}",
                        index + 1,
                        missing.join(", ")
                    ),
                )
                .with_target(format!("/report/literature_ladder/{index}"), ""),
            );
            continue;
        }

        let explicit_id = lookup_cell_any(row, &["ID", "Row ID", "Ladder ID"]);
        let id = if explicit_id.trim().is_empty() {
            content_id("ladder", &[&layer, &start_here])
        } else if is_stable_id(&explicit_id) {
            explicit_id
        } else {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_MISSING_PUBLIC_FIELD,
                    format!(
                        "Literature Ladder row {} has non-stable id {:?}; generated a stable id",
                        index + 1,
                        explicit_id
                    ),
                )
                .with_target(
                    format!("/report/literature_ladder/{index}/id"),
                    &explicit_id,
                ),
            );
            content_id("ladder", &[&layer, &start_here])
        };

        let source_refs = first_non_empty([
            lookup_cell_any(
                row,
                &[
                    "Source IDs",
                    "Source ids",
                    "Sources",
                    "Readings",
                    "Source references",
                ],
            )
            .as_str(),
            start_here.as_str(),
        ]);
        let source_ids = source_lookup.resolve_refs_with_diagnostics(
            &source_refs,
            format!("/report/literature_ladder/{index}/source_ids"),
            &id,
            diagnostics,
        );

        rows.push(LiteratureLadderRow {
            id,
            layer,
            start_here,
            read_for,
            do_not_infer,
            source_ids,
            notes: lookup_cell_any(row, &["Notes", "Note"]),
        });
    }
    rows
}

pub(crate) fn parse_source_requirement(raw: &str) -> SourceRoleRequirementKind {
    let text = normalize_id_text(raw);
    if text.contains("waiv") {
        SourceRoleRequirementKind::Waived
    } else if text.contains("conditional") || text.contains("if ") {
        SourceRoleRequirementKind::Conditional
    } else if text.contains("not applicable") || text.contains("n a") {
        SourceRoleRequirementKind::NotApplicable
    } else if text.contains("required") || text.contains("cannot be waived") {
        SourceRoleRequirementKind::Required
    } else {
        SourceRoleRequirementKind::Conditional
    }
}

pub(crate) fn build_curriculum_path(
    parsed: &ParsedMarkdownReport,
    source_lookup: &SourceLookup,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<CurriculumStep> {
    let mut steps = Vec::new();
    for (index, row) in parsed.curriculum_rows.iter().enumerate() {
        let title = curriculum_row_title(row);
        if title.is_empty() || is_placeholder_text(&title) {
            continue;
        }
        let learning_goal = first_non_empty([
            lookup_cell_any(row, &["Essential question"]).as_str(),
            lookup_cell_any(row, &["Learning goal"]).as_str(),
            title.as_str(),
        ]);
        let practice_artifact = first_non_empty([
            lookup_cell_any(row, &["Practice artifact"]).as_str(),
            lookup_cell_any(row, &["Artifact"]).as_str(),
            "Practice artifact not inferred from the structured report surface.",
        ]);
        let progress_criteria = split_listish(&lookup_cell_any(
            row,
            &["Progress criteria", "Criteria", "Assessment"],
        ));
        let source_refs = lookup_cell_any(row, &["Readings", "Sources", "Source IDs"]);
        let id = content_id("step", &[&title]);
        let source_ids = source_lookup.resolve_refs_with_diagnostics(
            &source_refs,
            format!("/report/curriculum_path/{index}/source_ids"),
            &id,
            diagnostics,
        );
        steps.push(CurriculumStep {
            id,
            sequence: (index + 1) as u32,
            title,
            learning_goal,
            prerequisite_ids: Vec::new(),
            practice_artifact,
            progress_criteria,
            source_ids,
        });
    }

    steps
}

pub(crate) fn curriculum_row_title(row: &BTreeMap<String, String>) -> String {
    first_non_empty([
        lookup_cell_any(row, &["Module"]).as_str(),
        lookup_cell_any(row, &["Phase"]).as_str(),
        lookup_cell_any(row, &["Title"]).as_str(),
    ])
}

pub(crate) fn apply_curriculum_prerequisites(
    parsed: &ParsedMarkdownReport,
    steps: &mut [CurriculumStep],
    entity_index: &ExportEntityIndex,
    diagnostics: &mut Vec<DiagnosticCheck>,
) {
    let mut refs_by_step_id = BTreeMap::new();
    for row in &parsed.curriculum_rows {
        let title = curriculum_row_title(row);
        if title.is_empty() || is_placeholder_text(&title) {
            continue;
        }
        let raw_refs = lookup_cell_any(
            row,
            &[
                "Prerequisite IDs",
                "Prerequisite ids",
                "Prerequisites",
                "Prereq IDs",
                "Prereqs",
            ],
        );
        if raw_refs.trim().is_empty() {
            continue;
        }
        refs_by_step_id.insert(content_id("step", &[&title]), split_ref_list(&raw_refs));
    }

    for (step_index, step) in steps.iter_mut().enumerate() {
        let Some(refs) = refs_by_step_id.get(&step.id) else {
            continue;
        };
        let mut prerequisite_ids = BTreeSet::new();
        for reference in refs {
            match entity_index.resolve_any_legacy_projection(reference) {
                ExportReferenceResolution::Resolved(entity) => {
                    if entity.id == step.id {
                        diagnostics.push(
                            DiagnosticCheck::warning(
                                CHECK_VALIDATE_CURRICULUM_REFERENCE,
                                format!(
                                    "curriculum step {} lists itself as a prerequisite",
                                    step.id
                                ),
                            )
                            .with_target(
                                format!("/report/curriculum_path/{step_index}/prerequisite_ids"),
                                &step.id,
                            ),
                        );
                    } else {
                        prerequisite_ids.insert(entity.id);
                    }
                }
                ExportReferenceResolution::Missing => diagnostics.push(
                    DiagnosticCheck::warning(
                        CHECK_EXPORT_UNRESOLVED_REFERENCE,
                        format!(
                            "curriculum step {} could not resolve prerequisite reference {:?}",
                            step.id, reference
                        ),
                    )
                    .with_target(
                        format!("/report/curriculum_path/{step_index}/prerequisite_ids"),
                        &step.id,
                    ),
                ),
                ExportReferenceResolution::Ambiguous(candidates) => diagnostics.push(
                    DiagnosticCheck::error(
                        CHECK_EXPORT_AMBIGUOUS_REFERENCE,
                        format!(
                            "curriculum step {} prerequisite reference {:?} is ambiguous: {}",
                            step.id,
                            reference,
                            candidates.join(", ")
                        ),
                    )
                    .with_target(
                        format!("/report/curriculum_path/{step_index}/prerequisite_ids"),
                        &step.id,
                    ),
                ),
            }
        }
        step.prerequisite_ids = prerequisite_ids.into_iter().collect();
    }
}

pub(crate) fn build_frontier_debates(
    parsed: &ParsedMarkdownReport,
    source_lookup: &SourceLookup,
    as_of: &str,
    review_after: &str,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> (Vec<FrontierDebateItem>, Vec<ClaimSeed>) {
    let mut items = Vec::new();
    let mut claims = Vec::new();
    let mut seen_titles = BTreeSet::new();
    for row in &parsed.frontier_rows {
        let title = first_non_empty([
            lookup_cell_any(row, &["Problem or debate"]).as_str(),
            lookup_cell_any(row, &["Area"]).as_str(),
            lookup_cell_any(row, &["Title"]).as_str(),
        ]);
        if title.is_empty() || is_placeholder_text(&title) {
            continue;
        }
        seen_titles.insert(normalize_alias_text(&title));
        let summary = first_non_empty([
            lookup_cell_any(row, &["Current state"]).as_str(),
            lookup_cell_any(row, &["Current or frontier issue"]).as_str(),
            lookup_cell_any(row, &["Summary"]).as_str(),
            "Frontier or debate summary not inferred from the structured report surface.",
        ]);
        if is_placeholder_text(&summary) {
            continue;
        }
        let why_it_matters = first_non_empty([
            lookup_cell_any(row, &["Why it matters"]).as_str(),
            lookup_cell_any(row, &["Why it is hard"]).as_str(),
            "This item marks a frontier, debate, or open problem boundary.",
        ]);
        let source_refs = lookup_cell_any(row, &["Key sources", "Sources", "Source IDs"]);
        let id = content_id("frontier", &[&title]);
        let source_ids = source_lookup.resolve_refs_with_diagnostics(
            &source_refs,
            format!("/report/frontier_debates/{}/source_ids", items.len()),
            &id,
            diagnostics,
        );
        let claim_statement = format!("{title}: {summary}");
        let claim_id = content_id("claim", &[&claim_statement]);
        claims.push(ClaimSeed {
            statement: claim_statement,
            claim_type: if normalize_id_text(&title).contains("debate") {
                ClaimType::Debate
            } else {
                ClaimType::Frontier
            },
            evidence_requirement: EvidenceRequirement::ReviewedSource,
            source_refs: split_ref_list(&source_refs),
            confidence: Some(ClaimConfidence::Unknown),
            notes: String::new(),
            temporal_status: Some(TemporalStatus::Current),
        });
        items.push(FrontierDebateItem {
            id,
            kind: if normalize_id_text(&title).contains("debate") {
                FrontierDebateKind::Debate
            } else if normalize_id_text(&title).contains("problem") {
                FrontierDebateKind::OpenProblem
            } else {
                FrontierDebateKind::Frontier
            },
            title,
            summary,
            why_it_matters,
            required_background_ids: Vec::new(),
            claim_ids: vec![claim_id],
            source_ids,
            temporal: TemporalMarker {
                as_of: as_of.to_string(),
                review_after: review_after.to_string(),
                temporal_status: TemporalStatus::Current,
                rationale: String::new(),
            },
        });
    }
    for row in &parsed.deep_structure_rows {
        let element_class = normalize_id_text(&lookup_cell_any(
            row,
            &["Element class", "Class", "Element type", "Element"],
        ));
        if ![
            "frontier",
            "dispute",
            "debate",
            "open problem",
            "controversy",
        ]
        .iter()
        .any(|kind| element_class.contains(kind))
        {
            continue;
        }
        let title = lookup_cell_any(row, &["Observed element", "Element", "Name", "Title"]);
        if title.is_empty()
            || is_placeholder_text(&title)
            || !seen_titles.insert(normalize_alias_text(&title))
        {
            continue;
        }
        let summary = first_non_empty([
            lookup_cell_any(row, &["Actual form in this field", "In this field"]).as_str(),
            lookup_cell_any(row, &["SoK extraction", "Summary"]).as_str(),
            title.as_str(),
        ]);
        let why_it_matters = first_non_empty([
            lookup_cell_any(
                row,
                &[
                    "Load-bearing relations",
                    "Why it matters",
                    "Role",
                    "Role: core / surrounding / context",
                ],
            )
            .as_str(),
            "This field element marks a frontier, debate, or open-problem boundary.",
        ]);
        let source_refs = lookup_cell_any(row, &["Source IDs", "Sources", "Key sources"]);
        let id = content_id("frontier", &[&title]);
        let source_ids = source_lookup.resolve_refs_with_diagnostics(
            &source_refs,
            format!("/report/frontier_debates/{}/source_ids", items.len()),
            &id,
            diagnostics,
        );
        let kind = if element_class.contains("debate")
            || element_class.contains("dispute")
            || element_class.contains("controversy")
        {
            FrontierDebateKind::Debate
        } else if element_class.contains("open problem") {
            FrontierDebateKind::OpenProblem
        } else {
            FrontierDebateKind::Frontier
        };
        let claim_statement = format!("{title}: {summary}");
        let claim_id = content_id("claim", &[&claim_statement]);
        claims.push(ClaimSeed {
            statement: claim_statement,
            claim_type: if kind == FrontierDebateKind::Debate {
                ClaimType::Debate
            } else {
                ClaimType::Frontier
            },
            evidence_requirement: EvidenceRequirement::ReviewedSource,
            source_refs: split_ref_list(&source_refs),
            confidence: Some(ClaimConfidence::Unknown),
            notes: String::new(),
            temporal_status: Some(TemporalStatus::Current),
        });
        items.push(FrontierDebateItem {
            id,
            kind,
            title,
            summary,
            why_it_matters,
            required_background_ids: Vec::new(),
            claim_ids: vec![claim_id],
            source_ids,
            temporal: TemporalMarker {
                as_of: as_of.to_string(),
                review_after: review_after.to_string(),
                temporal_status: TemporalStatus::Current,
                rationale: String::new(),
            },
        });
    }
    (items, claims)
}

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
            "Markdown visual map was not converted; Mermaid arrows require matching rows in a structured Relations table plus a Visual Views table",
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

pub(crate) fn build_report_presentation(
    parsed: &ParsedMarkdownReport,
    visual_views: &[VisualView],
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Option<ReportPresentation> {
    if parsed.narrative_sections.is_empty() && parsed.report_architecture.is_empty() {
        return None;
    }

    let known_view_ids = visual_views
        .iter()
        .map(|view| view.id.as_str())
        .collect::<BTreeSet<_>>();
    let sections = parsed
        .narrative_sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            for visual_view_id in &section.visual_view_ids {
                if !known_view_ids.contains(visual_view_id.as_str()) {
                    diagnostics.push(
                        DiagnosticCheck::warning(
                            CHECK_EXPORT_UNRESOLVED_REFERENCE,
                            format!(
                                "narrative section {:?} references unknown visual view {visual_view_id:?}",
                                section.title
                            ),
                        )
                        .with_target(
                            format!("/report/presentation/sections/{index}/visual_view_ids"),
                            visual_view_id,
                        ),
                    );
                }
            }
            ReportSection {
                id: content_id("section", &[&index.to_string(), &section.title]),
                title: section.title.clone(),
                purpose: section.purpose.clone(),
                body_markdown: section.body_markdown.clone(),
                visual_view_ids: section.visual_view_ids.clone(),
            }
        })
        .collect();

    let declared_thesis =
        architecture_value(&parsed.report_architecture, &["Executive thesis", "Thesis"]);
    Some(ReportPresentation {
        thesis: first_non_empty([declared_thesis.as_str(), parsed.orientation.as_str()]),
        organizing_form: architecture_value(
            &parsed.report_architecture,
            &["Chosen organizing form", "Organizing form"],
        ),
        rationale: architecture_value(
            &parsed.report_architecture,
            &["Architecture rationale", "Rationale", "Why this form"],
        ),
        alternatives_considered: architecture_value(
            &parsed.report_architecture,
            &[
                "Rejected alternatives and why",
                "Alternatives considered",
                "Rejected alternatives",
            ],
        ),
        sections,
    })
}

pub(crate) fn architecture_value(values: &BTreeMap<String, String>, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| {
            let normalized_key = normalize_id_text(key);
            values
                .iter()
                .find(|(candidate, _)| normalize_id_text(candidate) == normalized_key)
                .map(|(_, value)| value.clone())
        })
        .unwrap_or_default()
}

pub(crate) fn build_internal_context(parsed: &ParsedMarkdownReport) -> InternalContext {
    let mut placeholder_state = BTreeMap::new();
    if !parsed.placeholder_lines.is_empty() {
        placeholder_state.insert(
            "placeholder_lines".to_string(),
            serde_json::Value::Array(
                parsed
                    .placeholder_lines
                    .iter()
                    .map(|line| serde_json::Value::String(line.clone()))
                    .collect(),
            ),
        );
    }
    if !parsed.claim_rows.is_empty() {
        placeholder_state.insert(
            "candidate_claim_rows".to_string(),
            serde_json::Value::Number(parsed.claim_rows.len().into()),
        );
    }

    let prompt_derived_assumptions = [
        "Scoped assumption",
        "Profile hypothesis",
        "Scaffold stance",
        "Evidence posture",
        "Domain classification",
        "Provisional lens",
        "Why only provisional",
    ]
    .into_iter()
    .filter_map(|key| parsed.research_frame.get(key).cloned())
    .filter(|value| !value.trim().is_empty())
    .collect();

    let mut handoff_notes = parsed.quality_notes.clone();
    handoff_notes.extend(parsed.usefulness_notes.clone());

    InternalContext {
        raw_learner_profile: first_non_empty([
            parsed
                .research_frame
                .get("Learner")
                .map_or("", String::as_str),
            parsed
                .research_frame
                .get("Target learner")
                .map_or("", String::as_str),
        ]),
        original_goal: parsed
            .research_frame
            .get("Goal")
            .cloned()
            .unwrap_or_default(),
        prompt_derived_assumptions,
        placeholder_state,
        profile_proposals: profile_proposals_from_research_frame(&parsed.research_frame),
        handoff_notes,
    }
}

pub(crate) fn profile_proposals_from_research_frame(
    research_frame: &BTreeMap<String, String>,
) -> Vec<ProfileProposal> {
    let profile_id = architecture_value(
        research_frame,
        &["Profile proposal", "Proposed profile", "Profile ID"],
    );
    let profile_version = architecture_value(
        research_frame,
        &[
            "Profile version",
            "Proposed profile version",
            "Profile proposal version",
        ],
    );
    let locale = architecture_value(
        research_frame,
        &["Profile locale", "Proposed profile locale", "Locale"],
    );
    let proposer = architecture_value(
        research_frame,
        &[
            "Profile proposer type",
            "Profile proposer",
            "Proposer type",
            "Proposed by",
        ],
    );
    let confidence = architecture_value(
        research_frame,
        &[
            "Profile confidence",
            "Profile proposal confidence",
            "Confidence",
        ],
    );
    let rationale = architecture_value(
        research_frame,
        &[
            "Profile rationale",
            "Profile proposal rationale",
            "Rationale",
        ],
    );

    if [
        profile_id.as_str(),
        profile_version.as_str(),
        locale.as_str(),
        proposer.as_str(),
        confidence.as_str(),
        rationale.as_str(),
    ]
    .iter()
    .all(|value| value.trim().is_empty())
    {
        return Vec::new();
    }

    vec![ProfileProposal {
        proposer_type: parse_profile_proposer_type(&proposer),
        profile_id: first_non_empty([profile_id.as_str(), "open"]),
        profile_version: first_non_empty([profile_version.as_str(), "unknown"]),
        locale: first_non_empty([locale.as_str(), "und"]),
        confidence: first_non_empty([confidence.as_str(), "unknown"]),
        rationale: first_non_empty([
            rationale.as_str(),
            "Profile proposal imported from Research Frame; provisional until source review.",
        ]),
    }]
}

pub(crate) fn parse_profile_proposer_type(raw: &str) -> ProfileProposerType {
    match normalize_id_text(raw).as_str() {
        "user" => ProfileProposerType::User,
        "agent" => ProfileProposerType::Agent,
        "inference" => ProfileProposerType::Inference,
        _ => ProfileProposerType::Unknown,
    }
}

pub(crate) fn warn_if_empty_public_sections(
    field_elements: &[FieldElement],
    core_ideas: &[KnowledgeItem],
    methods: &[KnowledgeItem],
    representations: &[KnowledgeItem],
    diagnostics: &mut Vec<DiagnosticCheck>,
) {
    if field_elements.is_empty()
        && core_ideas.is_empty()
        && methods.is_empty()
        && representations.is_empty()
    {
        diagnostics.push(DiagnosticCheck::warning(
            CHECK_EXPORT_MISSING_PUBLIC_FIELD,
            "could not infer any field elements from structured Markdown surfaces",
        ));
    }
}

pub(crate) fn parsed_has_section(parsed: &ParsedMarkdownReport, section: CanonicalSection) -> bool {
    parsed.sections.contains(&section)
}

#[derive(Debug)]
pub(crate) struct SourceLookup {
    aliases: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceReferenceResolution {
    Resolved(String),
    Missing,
    Ambiguous(Vec<String>),
}

impl SourceLookup {
    fn new(sources: &[ReportSource]) -> Self {
        let aliases = BTreeMap::new();
        let mut lookup = Self { aliases };
        for (index, source) in sources.iter().enumerate() {
            for alias in [
                source.id.as_str(),
                source.title.as_str(),
                source.citation.as_str(),
                source.identifier.as_str(),
                source.url.as_str(),
            ] {
                lookup.insert_alias(alias, &source.id);
            }
            lookup.insert_alias(&format!("S{}", index + 1), &source.id);
        }
        lookup
    }

    fn insert_alias(&mut self, alias: &str, source_id: &str) {
        let normalized = normalize_alias_text(alias);
        if normalized.is_empty() {
            return;
        }
        self.aliases
            .entry(normalized)
            .or_default()
            .insert(source_id.to_string());
    }

    pub(crate) fn resolve_refs_with_diagnostics(
        &self,
        refs: &str,
        path: impl Into<String>,
        entity_id: &str,
        diagnostics: &mut Vec<DiagnosticCheck>,
    ) -> Vec<String> {
        self.resolve_ref_list_with_diagnostics(&split_ref_list(refs), path, entity_id, diagnostics)
    }

    pub(crate) fn resolve_ref_list_with_diagnostics(
        &self,
        refs: &[String],
        path: impl Into<String>,
        entity_id: &str,
        diagnostics: &mut Vec<DiagnosticCheck>,
    ) -> Vec<String> {
        let path = path.into();
        let mut ids = BTreeSet::new();
        for reference in refs {
            match self.resolve_one(reference) {
                SourceReferenceResolution::Resolved(id) => {
                    ids.insert(id);
                }
                SourceReferenceResolution::Missing => diagnostics.push(
                    DiagnosticCheck::warning(
                        CHECK_EXPORT_UNRESOLVED_REFERENCE,
                        format!("could not resolve source reference {:?}", reference),
                    )
                    .with_target(path.clone(), entity_id),
                ),
                SourceReferenceResolution::Ambiguous(candidates) => diagnostics.push(
                    DiagnosticCheck::error(
                        CHECK_EXPORT_AMBIGUOUS_REFERENCE,
                        format!(
                            "source reference {:?} is ambiguous: {}",
                            reference,
                            candidates.join(", ")
                        ),
                    )
                    .with_target(path.clone(), entity_id),
                ),
            }
        }
        ids.into_iter().collect()
    }

    pub(crate) fn resolve_one(&self, reference: &str) -> SourceReferenceResolution {
        let normalized = normalize_alias_text(reference);
        if normalized.is_empty() {
            return SourceReferenceResolution::Missing;
        }
        if let Some(candidates) = self.aliases.get(&normalized) {
            return source_candidate_resolution(candidates);
        }

        let mut candidates = BTreeSet::new();
        for (alias, ids) in &self.aliases {
            if alias.contains(&normalized) || normalized.contains(alias) {
                candidates.extend(ids.iter().cloned());
            }
        }
        source_candidate_resolution(&candidates)
    }
}

fn source_candidate_resolution(candidates: &BTreeSet<String>) -> SourceReferenceResolution {
    match candidates.len() {
        0 => SourceReferenceResolution::Missing,
        1 => SourceReferenceResolution::Resolved(
            candidates
                .iter()
                .next()
                .expect("candidate set has exactly one item")
                .clone(),
        ),
        _ => SourceReferenceResolution::Ambiguous(candidates.iter().cloned().collect()),
    }
}
pub(crate) fn split_ref_list(raw: &str) -> Vec<String> {
    raw.split([',', ';', '\n'])
        .map(clean_inline_markdown)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty() && !is_placeholder_text(item))
        .collect()
}

pub(crate) fn split_listish(raw: &str) -> Vec<String> {
    raw.split([';', '\n'])
        .flat_map(|part| part.split(" / "))
        .map(clean_inline_markdown)
        .map(|item| item.trim().trim_start_matches("- ").to_string())
        .filter(|item| !item.is_empty() && !is_placeholder_text(item))
        .collect()
}
pub(crate) fn parse_source_role(raw: &str) -> SourceRole {
    let text = normalize_id_text(raw);
    for (needle, role) in [
        ("orientation", SourceRole::Orientation),
        ("foundation", SourceRole::Foundation),
        ("method", SourceRole::Method),
        ("representation", SourceRole::Representation),
        ("evidence", SourceRole::Evidence),
        ("frontier", SourceRole::Frontier),
        ("debate", SourceRole::Debate),
        ("infrastructure", SourceRole::Infrastructure),
        ("standard", SourceRole::Standard),
        ("dataset", SourceRole::Dataset),
        ("data", SourceRole::Dataset),
        ("synthesis", SourceRole::Synthesis),
        ("critique", SourceRole::Critique),
        ("pedagogical", SourceRole::Curriculum),
        ("curriculum", SourceRole::Curriculum),
    ] {
        if text.contains(needle) {
            return role;
        }
    }
    SourceRole::Other
}

pub(crate) fn parse_claim_type(raw: &str) -> ClaimType {
    let text = normalize_id_text(raw);
    if text.contains("structural") {
        ClaimType::Structural
    } else if text.contains("current") {
        ClaimType::Currentness
    } else if text.contains("frontier") {
        ClaimType::Frontier
    } else if text.contains("debate") {
        ClaimType::Debate
    } else if text.contains("curricular") || text.contains("curriculum") {
        ClaimType::Curricular
    } else if text.contains("method") {
        ClaimType::Methodological
    } else {
        ClaimType::Interpretive
    }
}

pub(crate) fn parse_evidence_requirement(raw: &str) -> EvidenceRequirement {
    let text = normalize_id_text(raw);
    if text.contains("multiple") {
        EvidenceRequirement::MultipleReviewedSources
    } else if text.contains("verified") {
        EvidenceRequirement::VerifiedSource
    } else if text.contains("catalog") {
        EvidenceRequirement::CatalogedSource
    } else if text.contains("none") || text.contains("not required") {
        EvidenceRequirement::None
    } else {
        EvidenceRequirement::ReviewedSource
    }
}

pub(crate) fn parse_claim_confidence(raw: &str) -> Option<ClaimConfidence> {
    let text = normalize_id_text(raw);
    if text.is_empty() {
        None
    } else if text.contains("high") {
        Some(ClaimConfidence::High)
    } else if text.contains("medium") {
        Some(ClaimConfidence::Medium)
    } else if text.contains("low") {
        Some(ClaimConfidence::Low)
    } else {
        Some(ClaimConfidence::Unknown)
    }
}

pub(crate) fn parse_temporal_status(raw: &str) -> Option<TemporalStatus> {
    let text = normalize_id_text(raw);
    if text.is_empty() {
        None
    } else if text.contains("durable") {
        Some(TemporalStatus::Durable)
    } else if text.contains("current") {
        Some(TemporalStatus::Current)
    } else if text.contains("review due") {
        Some(TemporalStatus::ReviewDue)
    } else if text.contains("stale") {
        Some(TemporalStatus::Stale)
    } else {
        Some(TemporalStatus::Unknown)
    }
}

pub fn read_json_file<T, P>(path: P) -> Result<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let data = fs::read(path).with_context(|| format!("read JSON {}", path.display()))?;
    serde_json::from_slice(&data).with_context(|| format!("parse JSON {}", path.display()))
}

pub fn write_json_file<T, P>(path: P, value: &T) -> Result<()>
where
    T: Serialize,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    ensure_parent_dir(path)?;
    let mut data = serde_json::to_vec_pretty(value)
        .with_context(|| format!("encode JSON {}", path.display()))?;
    data.push(b'\n');
    fs::write(path, data).with_context(|| format!("write JSON {}", path.display()))?;
    Ok(())
}

pub fn read_jsonl_file<T, P>(path: P) -> Result<Vec<T>>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let file = File::open(path).with_context(|| format!("open JSONL {}", path.display()))?;
    let mut items = Vec::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line_number = index + 1;
        let line =
            line.with_context(|| format!("read JSONL {} line {line_number}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let item = serde_json::from_str(line.trim())
            .with_context(|| format!("parse JSONL {} line {line_number}", path.display()))?;
        items.push(item);
    }
    Ok(items)
}

pub fn write_jsonl_file<T, P>(path: P, values: &[T]) -> Result<()>
where
    T: Serialize,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    ensure_parent_dir(path)?;
    let file = File::create(path).with_context(|| format!("create JSONL {}", path.display()))?;
    let mut writer = BufWriter::new(file);
    for value in values {
        serde_json::to_writer(&mut writer, value)
            .with_context(|| format!("encode JSONL {}", path.display()))?;
        writer
            .write_all(b"\n")
            .with_context(|| format!("write JSONL {}", path.display()))?;
    }
    writer
        .flush()
        .with_context(|| format!("flush JSONL {}", path.display()))?;
    Ok(())
}

pub(crate) fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
    }
    Ok(())
}
