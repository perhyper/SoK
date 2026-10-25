mod composition;
mod lint;
mod machine;
mod markdown;

pub(crate) use composition::*;
pub(crate) use lint::*;
use machine::*;
pub(crate) use markdown::*;

use super::ids::*;
use super::model::*;
use super::provenance::*;
use super::relations::*;
use super::validation::*;
use super::*;
use crate::core::{
    self, evidence::EvidencePackage, knowledge::KnowledgePackage, pedagogy::PedagogyPackage,
    CorePackages, EVIDENCE_SCHEMA_VERSION, KNOWLEDGE_SCHEMA_VERSION, PEDAGOGY_SCHEMA_VERSION,
};
use crate::profiles::{self, ProjectionRole};
use serde::Serialize;

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

#[derive(Debug, Clone, Default)]
pub struct MachineInputPaths {
    pub knowledge: Option<PathBuf>,
    pub evidence_package: Option<PathBuf>,
    pub pedagogy: Option<PathBuf>,
}

impl MachineInputPaths {
    pub fn is_empty(&self) -> bool {
        self.knowledge.is_none() && self.evidence_package.is_none() && self.pedagogy.is_none()
    }
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
    let evidence_path = evidence_path.as_ref().map(|path| path.as_ref());
    export_markdown_report_impl(
        report_path,
        sources_path,
        evidence_path,
        stage,
        &MachineInputPaths::default(),
    )
}

pub fn export_markdown_report_with_machine_inputs<P>(
    report_path: P,
    sources_path: P,
    evidence_path: Option<P>,
    stage: ExportStage,
    machine_inputs: MachineInputPaths,
) -> Result<ReportDocument>
where
    P: AsRef<Path>,
{
    let report_path = report_path.as_ref();
    let sources_path = sources_path.as_ref();
    let evidence_path = evidence_path.as_ref().map(|path| path.as_ref());
    export_markdown_report_impl(
        report_path,
        sources_path,
        evidence_path,
        stage,
        &machine_inputs,
    )
}

fn export_markdown_report_impl(
    report_path: &Path,
    sources_path: &Path,
    evidence_path: Option<&Path>,
    stage: ExportStage,
    machine_inputs: &MachineInputPaths,
) -> Result<ReportDocument> {
    let markdown = fs::read_to_string(report_path)
        .with_context(|| format!("read report Markdown {}", report_path.display()))?;
    let mut parsed = parse_markdown_report(&markdown);
    let mut machine_packages = collect_machine_inputs(&markdown, machine_inputs)?;

    let normalized_sources = normalize_source_manifest(sources_path)?;
    let mut sources = normalized_sources.sources;
    let mut evidence = normalized_sources.evidence;
    let mut diagnostics = Vec::new();
    diagnostics.append(&mut parsed.diagnostics);
    diagnostics.extend(normalized_sources.diagnostics);
    validate_markdown_report_architecture(&parsed, stage, &mut diagnostics);

    if let Some(path) = evidence_path {
        let mut reviewed_evidence: Vec<EvidenceEntry> = read_jsonl_file(path)?;
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
    let mut relations = build_relations(&parsed, &entity_index, &source_lookup, &mut diagnostics);
    relations.extend(apply_curriculum_prerequisites(
        &parsed,
        &mut curriculum_path,
        &entity_index,
        &mut diagnostics,
    ));
    let relations = deduplicate_relations(relations);
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

    diagnostics.append(&mut machine_packages.diagnostics);

    if !machine_packages.has_input {
        attach_export_diagnostics(&mut document, diagnostics);
        return crate::core::projection::project_report_document(&document);
    }

    project_report_with_machine_inputs(document, machine_packages, diagnostics)
}

fn attach_export_diagnostics(document: &mut ReportDocument, diagnostics: Vec<DiagnosticCheck>) {
    if diagnostics.is_empty() {
        document.diagnostics = None;
        return;
    }
    document.diagnostics = Some(Diagnostics {
        summary: format!(
            "export-json emitted {} diagnostic(s) while converting bounded Markdown.",
            diagnostics.len()
        ),
        checks: diagnostics,
    });
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
