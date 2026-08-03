use super::validation::sort_diagnostics;
use super::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReportDocument {
    pub metadata: ReportMetadata,
    pub report: PublicReport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_context: Option<InternalContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Diagnostics>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReportMetadata {
    #[serde(default)]
    pub schema_version: String,
    #[serde(default)]
    pub generated_at: String,
    #[serde(default)]
    pub report_type: ReportType,
    pub temporal_review: TemporalMarker,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<GeneratorInfo>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneratorInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    #[default]
    Scaffold,
    HumanReport,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicReport {
    #[serde(default)]
    pub field: String,
    pub scope: Scope,
    pub domain_profile: DomainProfile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<ReportPresentation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub literature_ladder: Vec<LiteratureLadderRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_elements: Vec<FieldElement>,
    pub core_ideas: Vec<KnowledgeItem>,
    pub methods: Vec<KnowledgeItem>,
    pub representations: Vec<KnowledgeItem>,
    pub evidence_standards: EvidenceStandards,
    pub sources: Vec<ReportSource>,
    pub claims: Vec<Claim>,
    pub relations: Vec<Relation>,
    pub curriculum_path: Vec<CurriculumStep>,
    pub frontier_debates: Vec<FrontierDebateItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visual_views: Vec<VisualView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub structure_waivers: Vec<StructureWaiver>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReportPresentation {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub thesis: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub organizing_form: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub alternatives_considered: String,
    pub sections: Vec<ReportSection>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReportSection {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub purpose: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body_markdown: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visual_view_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scope {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub included: Vec<String>,
    #[serde(default)]
    pub excluded: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assumptions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interpretive_notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomainProfile {
    #[serde(default)]
    pub classification: DomainClassification,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary_characteristics: Vec<DomainClassification>,
    #[serde(default)]
    pub rationale: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failure_modes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DomainClassification {
    WellStructured,
    Formal,
    IllStructured,
    ProfessionalPractice,
    InstrumentBound,
    InfrastructureBound,
    Emerging,
    Interdisciplinary,
    #[default]
    Mixed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporal: Option<TemporalMarker>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldElement {
    pub id: String,
    pub element_class: String,
    pub label: String,
    pub actual_form: String,
    pub role: String,
    pub load_bearing_relations: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ClaimConfidence>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiteratureLadderRow {
    pub id: String,
    pub layer: String,
    pub start_here: String,
    pub read_for: String,
    pub do_not_infer: String,
    pub source_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceStandards {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub claim_policy: String,
    #[serde(default)]
    pub source_role_requirements: Vec<SourceRoleRequirement>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRoleRequirement {
    #[serde(default)]
    pub role: SourceRole,
    #[serde(default)]
    pub requirement: SourceRoleRequirementKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_sources: Option<u32>,
    #[serde(default)]
    pub rationale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiver: Option<SourceRoleWaiver>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceRoleRequirementKind {
    #[default]
    Required,
    Conditional,
    Waived,
    NotApplicable,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRoleWaiver {
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub as_of: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub review_after: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReportSource {
    pub id: String,
    pub citation: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    pub source_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub identifier: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub date: String,
    pub roles: Vec<SourceRole>,
    pub access: SourceAccessMetadata,
    pub why_it_matters: String,
    pub verification_status: VerificationStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_reviewed: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    Orientation,
    Foundation,
    Method,
    Representation,
    Evidence,
    Synthesis,
    Frontier,
    Debate,
    Standard,
    Dataset,
    Infrastructure,
    Critique,
    Curriculum,
    #[default]
    Other,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceAccessMetadata {
    pub status: AccessStatus,
    pub route: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub budget_estimate: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub license: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_only: Option<bool>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessStatus {
    OpenAccess,
    FreeWeb,
    PublicDomain,
    OfficialOpen,
    UserProvided,
    Library,
    PaidBook,
    Paywalled,
    Subscription,
    Restricted,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    #[default]
    Cataloged,
    Reviewed,
    Verified,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claim {
    pub id: String,
    pub statement: String,
    pub claim_type: ClaimType,
    pub evidence_requirement: EvidenceRequirement,
    pub evidence_links: Vec<EvidenceLink>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ClaimConfidence>,
    pub temporal: TemporalMarker,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimType {
    Structural,
    Currentness,
    Frontier,
    Debate,
    Curricular,
    #[default]
    Interpretive,
    Methodological,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRequirement {
    None,
    CatalogedSource,
    #[default]
    ReviewedSource,
    VerifiedSource,
    MultipleReviewedSources,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimConfidence {
    High,
    Medium,
    Low,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceLink {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub evidence_id: String,
    pub source_id: String,
    pub verification_status: VerificationStatus,
    pub support_kind: SupportKind,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub locator: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub support_note: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reviewed_at: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SupportKind {
    #[default]
    Supports,
    Qualifies,
    Contradicts,
    Background,
    Example,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Relation {
    pub id: String,
    pub kind: RelationKind,
    pub from: RelationEndpoint,
    pub to: RelationEndpoint,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    DependsOn,
    #[default]
    Supports,
    Qualifies,
    Contradicts,
    Precedes,
    Introduces,
    UsesMethod,
    RepresentedBy,
    Grounds,
    Motivates,
    PartOf,
    MapsTo,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelationEndpoint {
    pub entity_type: EntityType,
    pub id: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    #[default]
    Concept,
    FieldElement,
    Claim,
    Source,
    CurriculumStep,
    FrontierDebate,
    Method,
    Representation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurriculumStep {
    pub id: String,
    pub sequence: u32,
    pub title: String,
    pub learning_goal: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prerequisite_ids: Vec<String>,
    pub practice_artifact: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub progress_criteria: Vec<String>,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrontierDebateItem {
    pub id: String,
    pub kind: FrontierDebateKind,
    pub title: String,
    pub summary: String,
    pub why_it_matters: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_background_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claim_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_ids: Vec<String>,
    pub temporal: TemporalMarker,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FrontierDebateKind {
    #[default]
    Frontier,
    Debate,
    OpenProblem,
    Uncertainty,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemporalMarker {
    #[serde(default)]
    pub as_of: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub review_after: String,
    #[serde(default)]
    pub temporal_status: TemporalStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TemporalStatus {
    Durable,
    Current,
    ReviewDue,
    Stale,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostics {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub summary: String,
    pub checks: Vec<DiagnosticCheck>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticCheck {
    pub check_id: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<DiagnosticStatus>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub entity_id: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Info,
    #[default]
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStatus {
    Passed,
    Failed,
    NotApplicable,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisualView {
    pub id: String,
    pub kind: VisualViewKind,
    pub title: String,
    pub justification: String,
    pub nodes: Vec<VisualViewNode>,
    pub edges: Vec<VisualViewEdge>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualViewKind {
    KnowledgeSpine,
    ConceptSource,
    DependencyPath,
    FrontierDebate,
    #[default]
    Custom,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisualViewNode {
    pub id: String,
    pub label: String,
    pub entity_type: EntityType,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ref_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisualViewEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub relation_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructureWaiver {
    pub scope: StructureWaiverScope,
    pub rationale: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructureWaiverScope {
    Relations,
    CurriculumPrerequisites,
    #[default]
    VisualViews,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InternalContext {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub raw_learner_profile: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub original_goal: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompt_derived_assumptions: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub placeholder_state: BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handoff_notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceEntry {
    pub evidence_id: String,
    pub source_id: String,
    pub input_provenance: BoundedInputProvenance,
    pub verification_status: VerificationStatus,
    pub support_kind: SupportKind,
    pub locator: String,
    pub support_note: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub observed_at: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reviewed_at: String,
    pub notes: String,
    pub claim_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_roles: Vec<SourceRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_access: Option<SourceAccessMetadata>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_citation: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_identifier: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub curricular_use: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoundedInputProvenance {
    pub input_path: String,
    pub input_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_number: Option<usize>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub row_hash: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedSourceManifest {
    pub sources: Vec<ReportSource>,
    pub evidence: Vec<EvidenceEntry>,
    pub diagnostics: Vec<DiagnosticCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportValidation {
    pub diagnostics: Diagnostics,
}

impl ReportValidation {
    pub fn new(mut checks: Vec<DiagnosticCheck>) -> Self {
        sort_diagnostics(&mut checks);
        let error_count = checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Error)
            .count();
        let warning_count = checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Warning)
            .count();
        let summary = format!(
            "report validation produced {error_count} error(s) and {warning_count} warning(s)"
        );
        Self {
            diagnostics: Diagnostics { summary, checks },
        }
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Warning)
            .count()
    }

    pub fn info_count(&self) -> usize {
        self.diagnostics
            .checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Info)
            .count()
    }

    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }

    pub fn has_warnings(&self) -> bool {
        self.warning_count() > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportStage {
    Scaffold,
    Final,
}
