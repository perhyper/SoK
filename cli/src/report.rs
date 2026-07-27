//! Structured report conversion, validation, and rendering.

use crate::{load_sources, source_access, Source};
use anyhow::{bail, Context, Result};
use chrono::{Duration, NaiveDate, SecondsFormat, Utc};
use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

pub const CHECK_SOURCE_MISSING_ACCESS_STATUS: &str = "source.missing-access-status";
pub const CHECK_SOURCE_MISSING_ACCESS_ROUTE: &str = "source.missing-access-route";
pub const CHECK_SOURCE_MISSING_CURRICULAR_USE: &str = "source.missing-curricular-use";
pub const CHECK_EVIDENCE_CATALOGED_ONLY: &str = "evidence.cataloged-only";
pub const CHECK_EXPORT_UNSUPPORTED_SECTION: &str = "export.unsupported-section";
pub const CHECK_EXPORT_AMBIGUOUS_SECTION: &str = "export.ambiguous-section";
pub const CHECK_EXPORT_UNKNOWN_SURFACE_MARKER: &str = "export.unknown-surface-marker";
pub const CHECK_EXPORT_UNKNOWN_DIRECTIVE: &str = "export.unknown-directive";
pub const CHECK_EXPORT_REPORT_ARCHITECTURE: &str = "export.report-architecture";
pub const CHECK_EXPORT_MISSING_FIELD: &str = "export.missing-field";
pub const CHECK_EXPORT_MISSING_PUBLIC_FIELD: &str = "export.missing-public-field";
pub const CHECK_EXPORT_INTERNAL_SECTION_IN_FINAL: &str = "export.internal-section-in-final";
pub const CHECK_EXPORT_UNKNOWN_EVIDENCE_SOURCE: &str = "export.unknown-evidence-source";
pub const CHECK_EXPORT_CLAIM_NEEDS_EVIDENCE: &str = "export.claim-needs-evidence";
pub const CHECK_EXPORT_UNRESOLVED_REFERENCE: &str = "export.unresolved-reference";
pub const CHECK_EXPORT_AMBIGUOUS_REFERENCE: &str = "export.ambiguous-reference";
pub const CHECK_VALIDATE_SCHEMA_JSON: &str = "validate.schema.json";
pub const CHECK_VALIDATE_SCHEMA_REQUIRED: &str = "validate.schema.required";
pub const CHECK_VALIDATE_SCHEMA_DESERIALIZE: &str = "validate.schema.deserialize";
pub const CHECK_VALIDATE_PUBLIC_BOUNDARY: &str = "validate.public-boundary";
pub const CHECK_VALIDATE_EVIDENCE_REQUIRED: &str = "validate.evidence.required";
pub const CHECK_VALIDATE_EVIDENCE_SOURCE: &str = "validate.evidence.source";
pub const CHECK_VALIDATE_EVIDENCE_SUPPORT: &str = "validate.evidence.support";
pub const CHECK_VALIDATE_SOURCE_ROLE_REQUIRED: &str = "validate.source-role.required";
pub const CHECK_VALIDATE_SOURCE_ROLE_CONDITIONAL: &str = "validate.source-role.conditional";
pub const CHECK_VALIDATE_SOURCE_ROLE_WAIVER: &str = "validate.source-role.waiver";
pub const CHECK_VALIDATE_CURRENTNESS_METADATA: &str = "validate.currentness.metadata";
pub const CHECK_VALIDATE_CURRENTNESS_REVIEW_DUE: &str = "validate.currentness.review-due";
pub const CHECK_VALIDATE_CURRENTNESS_SOURCE_DATE: &str = "validate.currentness.source-date";
pub const CHECK_VALIDATE_CURRENTNESS_PROSE: &str = "validate.currentness.prose";
pub const CHECK_VALIDATE_RELATION_ENDPOINT: &str = "validate.relation.endpoint";
pub const CHECK_VALIDATE_RELATION_KIND: &str = "validate.relation.kind";
pub const CHECK_VALIDATE_CURRICULUM_REFERENCE: &str = "validate.curriculum.reference";
pub const CHECK_VALIDATE_SOURCE_ACCESS: &str = "validate.source-access";
pub const CHECK_VALIDATE_VISUAL_REFERENCE: &str = "validate.visual.reference";
pub const CHECK_VALIDATE_STRUCTURE_REQUIRED: &str = "validate.structure.required";
pub const CHECK_LINT_STRUCTURAL: &str = "lint.structural";
pub const CHECK_LINT_SCAFFOLD_UNRESOLVED: &str = "lint.scaffold-unresolved";
pub const CHECK_LINT_FINAL_PUBLIC_LEAKAGE: &str = "lint.final-public-leakage";

const ID_HASH_LEN: usize = 10;

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

pub fn validate_report_file<P: AsRef<Path>>(path: P) -> Result<ReportValidation> {
    let path = path.as_ref();
    let data = fs::read(path).with_context(|| format!("read JSON {}", path.display()))?;
    let value = match serde_json::from_slice::<Value>(&data) {
        Ok(value) => value,
        Err(err) => {
            return Ok(ReportValidation::new(vec![DiagnosticCheck::error(
                CHECK_VALIDATE_SCHEMA_JSON,
                format!("invalid JSON in {}: {err}", path.display()),
            )
            .with_target("/", "")]));
        }
    };
    Ok(validate_report_value(&value))
}

pub fn validate_report_value(value: &Value) -> ReportValidation {
    let mut checks = Vec::new();
    validate_schema_level_fields(value, &mut checks);
    validate_known_enum_strings(value, &mut checks);

    let document = match serde_json::from_value::<ReportDocument>(value.clone()) {
        Ok(document) => Some(document),
        Err(err) => {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_DESERIALIZE,
                    format!("report JSON does not match the typed report model: {err}"),
                )
                .with_target("/", ""),
            );
            None
        }
    };

    if let Some(document) = document {
        let index = ReportIdIndex::from_report(&document.report, &mut checks);
        validate_embedded_diagnostics(&document, &mut checks);
        validate_public_report_boundary(value, document.metadata.report_type, &mut checks);
        validate_reference_consistency(&document.report, &index, &mut checks);
        validate_claim_evidence_requirements(&document.report, &index, &mut checks);
        validate_source_role_coverage(&document.report, &mut checks);
        validate_currentness(&document, &index, &mut checks);
        validate_relation_consistency(&document.report, &index, &mut checks);
        validate_visual_references(&document.report, &index, &mut checks);
        validate_source_access_metadata(&document.report, &mut checks);
        validate_structure_waivers(&document.report, &mut checks);
        validate_required_structure(&document, &mut checks);
    }

    ReportValidation::new(checks)
}

pub fn lint_markdown_report<R, S, E>(
    report_path: R,
    sources_path: S,
    evidence_path: Option<E>,
    stage: ExportStage,
) -> Result<ReportValidation>
where
    R: AsRef<Path>,
    S: AsRef<Path>,
    E: AsRef<Path>,
{
    let report_path = report_path.as_ref();
    let sources_path = sources_path.as_ref();
    let markdown = fs::read_to_string(report_path)
        .with_context(|| format!("read report Markdown {}", report_path.display()))?;
    let mut parsed = parse_markdown_report(&markdown);
    let normalized_sources = normalize_source_manifest(sources_path)?;
    let sources = normalized_sources.sources;
    let mut evidence = normalized_sources.evidence;
    let mut checks = Vec::new();

    checks.extend(select_lint_parse_diagnostics(&mut parsed.diagnostics));
    checks.extend(normalized_sources.diagnostics);

    if let Some(path) = evidence_path {
        let mut ledger_entries: Vec<EvidenceEntry> = read_jsonl_file(path)?;
        evidence.append(&mut ledger_entries);
    }

    if !parsed_has_any_canonical_content(&parsed) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_LINT_STRUCTURAL,
                "Markdown report has no canonical SoK title or sections",
            )
            .with_target("/report", ""),
        );
    }

    lint_stage_boundary(&markdown, &parsed, stage, &mut checks);
    validate_markdown_report_architecture(&parsed, stage, &mut checks);
    lint_source_role_coverage(&parsed, &sources, &mut checks);
    lint_evidence_sources(&evidence, &sources, &mut checks);
    lint_evidence_semantics(&evidence, &mut checks);
    lint_claim_evidence_support(&parsed, &sources, &evidence, stage, &mut checks);
    lint_markdown_currentness(&markdown, &mut checks);

    Ok(ReportValidation::new(checks))
}

fn validate_schema_level_fields(value: &Value, checks: &mut Vec<DiagnosticCheck>) {
    let Some(root) = value.as_object() else {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_SCHEMA_REQUIRED,
                "root report payload must be a JSON object",
            )
            .with_target("/", ""),
        );
        return;
    };

    for key in root.keys() {
        if !matches!(
            key.as_str(),
            "metadata" | "report" | "internal_context" | "diagnostics"
        ) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("unexpected top-level key {key:?}; expected metadata, report, internal_context, or diagnostics"),
                )
                .with_target(format!("/{key}"), ""),
            );
        }
    }

    let Some(metadata) = require_object_for_validation(root, "metadata", "/", checks) else {
        return;
    };
    let Some(report) = require_object_for_validation(root, "report", "/", checks) else {
        return;
    };

    validate_allowed_keys(
        metadata,
        "/metadata",
        &[
            "schema_version",
            "generated_at",
            "report_type",
            "temporal_review",
            "generator",
        ],
        checks,
    );
    validate_allowed_keys(
        report,
        "/report",
        &[
            "field",
            "scope",
            "domain_profile",
            "presentation",
            "literature_ladder",
            "field_elements",
            "core_ideas",
            "methods",
            "representations",
            "evidence_standards",
            "sources",
            "claims",
            "relations",
            "curriculum_path",
            "frontier_debates",
            "visual_views",
            "structure_waivers",
        ],
        checks,
    );

    for key in [
        "schema_version",
        "generated_at",
        "report_type",
        "temporal_review",
    ] {
        require_key(metadata, key, "/metadata", checks);
    }
    require_non_empty_string(metadata, "schema_version", "/metadata", checks);
    require_non_empty_string(metadata, "generated_at", "/metadata", checks);
    require_non_empty_string(metadata, "report_type", "/metadata", checks);
    if let Some(temporal) =
        require_object_for_validation(metadata, "temporal_review", "/metadata", checks)
    {
        require_temporal_marker_fields(temporal, "/metadata/temporal_review", checks);
    }

    for key in [
        "field",
        "scope",
        "domain_profile",
        "core_ideas",
        "methods",
        "representations",
        "evidence_standards",
        "sources",
        "claims",
        "relations",
        "curriculum_path",
        "frontier_debates",
    ] {
        require_key(report, key, "/report", checks);
    }
    require_non_empty_string(report, "field", "/report", checks);

    if let Some(scope) = require_object_for_validation(report, "scope", "/report", checks) {
        require_non_empty_string(scope, "summary", "/report/scope", checks);
        require_array(scope, "included", "/report/scope", checks);
        require_array(scope, "excluded", "/report/scope", checks);
    }
    if let Some(domain) = require_object_for_validation(report, "domain_profile", "/report", checks)
    {
        require_non_empty_string(domain, "classification", "/report/domain_profile", checks);
        require_non_empty_string(domain, "rationale", "/report/domain_profile", checks);
    }
    if let Some(value) = report.get("presentation") {
        if let Value::Object(presentation) = value {
            validate_allowed_keys(
                presentation,
                "/report/presentation",
                &[
                    "thesis",
                    "organizing_form",
                    "rationale",
                    "alternatives_considered",
                    "sections",
                ],
                checks,
            );
            require_non_empty_string(presentation, "thesis", "/report/presentation", checks);
            require_non_empty_string(
                presentation,
                "organizing_form",
                "/report/presentation",
                checks,
            );
            require_non_empty_string(presentation, "rationale", "/report/presentation", checks);
            require_array(presentation, "sections", "/report/presentation", checks);
            if let Some(Value::Array(sections)) = presentation.get("sections") {
                if sections.is_empty() {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_SCHEMA_REQUIRED,
                            "/report/presentation/sections must contain at least one public narrative section",
                        )
                        .with_target("/report/presentation/sections", ""),
                    );
                }
                for (section_index, section) in sections.iter().enumerate() {
                    let path = format!("/report/presentation/sections/{section_index}");
                    let Some(section) = section.as_object() else {
                        checks.push(
                            DiagnosticCheck::error(
                                CHECK_VALIDATE_SCHEMA_REQUIRED,
                                format!("{path} must be an object"),
                            )
                            .with_target(path, ""),
                        );
                        continue;
                    };
                    validate_allowed_keys(
                        section,
                        &path,
                        &["id", "title", "purpose", "body_markdown", "visual_view_ids"],
                        checks,
                    );
                    require_non_empty_string(section, "id", &path, checks);
                    require_non_empty_string(section, "title", &path, checks);
                    if section.contains_key("visual_view_ids") {
                        require_array(section, "visual_view_ids", &path, checks);
                    }
                }
            }
        } else {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    "/report/presentation must be an object",
                )
                .with_target("/report/presentation", ""),
            );
        }
        match report.get("field_elements").and_then(Value::as_array) {
            Some(field_elements) if !field_elements.is_empty() => {}
            _ => checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_STRUCTURE_REQUIRED,
                    "a report with presentation must include a non-empty /report/field_elements array",
                )
                .with_target("/report/field_elements", ""),
            ),
        }
    }
    if let Some(standards) =
        require_object_for_validation(report, "evidence_standards", "/report", checks)
    {
        require_non_empty_string(standards, "summary", "/report/evidence_standards", checks);
        require_non_empty_string(
            standards,
            "claim_policy",
            "/report/evidence_standards",
            checks,
        );
        require_array(
            standards,
            "source_role_requirements",
            "/report/evidence_standards",
            checks,
        );
    }

    for key in [
        "core_ideas",
        "methods",
        "representations",
        "sources",
        "claims",
        "relations",
        "curriculum_path",
        "frontier_debates",
    ] {
        require_array(report, key, "/report", checks);
    }
    for key in [
        "literature_ladder",
        "field_elements",
        "visual_views",
        "structure_waivers",
    ] {
        if report.contains_key(key) {
            require_array(report, key, "/report", checks);
        }
    }

    validate_required_array_item_fields(
        report,
        "literature_ladder",
        &[
            "id",
            "layer",
            "start_here",
            "read_for",
            "do_not_infer",
            "source_ids",
        ],
        checks,
    );
    validate_required_array_item_fields(
        report,
        "field_elements",
        &[
            "id",
            "element_class",
            "label",
            "actual_form",
            "role",
            "load_bearing_relations",
        ],
        checks,
    );
    validate_required_array_item_fields(
        report,
        "sources",
        &[
            "id",
            "citation",
            "source_type",
            "roles",
            "access",
            "why_it_matters",
            "verification_status",
        ],
        checks,
    );
    validate_required_array_item_fields(
        report,
        "claims",
        &[
            "id",
            "statement",
            "claim_type",
            "evidence_requirement",
            "evidence_links",
            "temporal",
        ],
        checks,
    );
    validate_required_array_item_fields(report, "relations", &["id", "kind", "from", "to"], checks);
    validate_required_array_item_fields(
        report,
        "curriculum_path",
        &[
            "id",
            "sequence",
            "title",
            "learning_goal",
            "practice_artifact",
            "source_ids",
        ],
        checks,
    );
    validate_required_array_item_fields(
        report,
        "frontier_debates",
        &[
            "id",
            "kind",
            "title",
            "summary",
            "why_it_matters",
            "temporal",
        ],
        checks,
    );
    validate_required_array_item_fields(
        report,
        "structure_waivers",
        &["scope", "rationale"],
        checks,
    );
}

fn validate_allowed_keys(
    object: &serde_json::Map<String, Value>,
    path: &str,
    allowed: &[&str],
    checks: &mut Vec<DiagnosticCheck>,
) {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("{path} contains unsupported key {key:?}"),
                )
                .with_target(format!("{path}/{key}"), ""),
            );
        }
    }
}

fn require_object_for_validation<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) -> Option<&'a serde_json::Map<String, Value>> {
    match object.get(key) {
        Some(Value::Object(child)) => Some(child),
        Some(_) => {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("{path}/{key} must be an object"),
                )
                .with_target(format!("{path}/{key}"), ""),
            );
            None
        }
        None => {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("missing required object {path}/{key}"),
                )
                .with_target(format!("{path}/{key}"), ""),
            );
            None
        }
    }
}

fn require_key(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    if !object.contains_key(key) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_SCHEMA_REQUIRED,
                format!("missing required field {path}/{key}"),
            )
            .with_target(format!("{path}/{key}"), ""),
        );
    }
}

fn require_non_empty_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    match object.get(key) {
        Some(Value::String(value)) if !value.trim().is_empty() => {}
        Some(Value::String(_)) => checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_SCHEMA_REQUIRED,
                format!("{path}/{key} must be a non-empty string"),
            )
            .with_target(format!("{path}/{key}"), ""),
        ),
        Some(_) => checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_SCHEMA_REQUIRED,
                format!("{path}/{key} must be a string"),
            )
            .with_target(format!("{path}/{key}"), ""),
        ),
        None => require_key(object, key, path, checks),
    }
}

fn require_array(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    match object.get(key) {
        Some(Value::Array(_)) => {}
        Some(_) => checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_SCHEMA_REQUIRED,
                format!("{path}/{key} must be an array"),
            )
            .with_target(format!("{path}/{key}"), ""),
        ),
        None => require_key(object, key, path, checks),
    }
}

fn require_temporal_marker_fields(
    temporal: &serde_json::Map<String, Value>,
    path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    require_non_empty_string(temporal, "as_of", path, checks);
    require_non_empty_string(temporal, "temporal_status", path, checks);
}

fn validate_required_array_item_fields(
    report: &serde_json::Map<String, Value>,
    section: &str,
    fields: &[&str],
    checks: &mut Vec<DiagnosticCheck>,
) {
    let Some(Value::Array(items)) = report.get(section) else {
        return;
    };
    for (index, item) in items.iter().enumerate() {
        let path = format!("/report/{section}/{index}");
        let Some(object) = item.as_object() else {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("{path} must be an object"),
                )
                .with_target(path, ""),
            );
            continue;
        };
        for field in fields {
            require_key(object, field, &path, checks);
        }
    }
}

fn validate_known_enum_strings(value: &Value, checks: &mut Vec<DiagnosticCheck>) {
    if let Some(metadata) = value.get("metadata").and_then(Value::as_object) {
        validate_string_enum(
            metadata,
            "report_type",
            &["scaffold", "human_report"],
            CHECK_VALIDATE_SCHEMA_REQUIRED,
            "/metadata",
            checks,
        );
    }

    let Some(report) = value.get("report").and_then(Value::as_object) else {
        return;
    };

    if let Some(field_elements) = report.get("field_elements").and_then(Value::as_array) {
        for (index, element) in field_elements.iter().enumerate() {
            let Some(element) = element.as_object() else {
                continue;
            };
            if element.contains_key("confidence") {
                validate_string_enum(
                    element,
                    "confidence",
                    &["high", "medium", "low", "unknown"],
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    &format!("/report/field_elements/{index}"),
                    checks,
                );
            }
        }
    }

    if let Some(sources) = report.get("sources").and_then(Value::as_array) {
        for (index, source) in sources.iter().enumerate() {
            let Some(source) = source.as_object() else {
                continue;
            };
            if let Some(access) = source.get("access").and_then(Value::as_object) {
                validate_string_enum(
                    access,
                    "status",
                    &[
                        "open_access",
                        "free_web",
                        "public_domain",
                        "official_open",
                        "user_provided",
                        "library",
                        "paid_book",
                        "paywalled",
                        "subscription",
                        "restricted",
                        "unknown",
                    ],
                    CHECK_VALIDATE_SOURCE_ACCESS,
                    &format!("/report/sources/{index}/access"),
                    checks,
                );
            }
        }
    }

    if let Some(claims) = report.get("claims").and_then(Value::as_array) {
        for (claim_index, claim) in claims.iter().enumerate() {
            let Some(claim) = claim.as_object() else {
                continue;
            };
            validate_string_enum(
                claim,
                "claim_type",
                &[
                    "structural",
                    "currentness",
                    "frontier",
                    "debate",
                    "curricular",
                    "interpretive",
                    "methodological",
                ],
                CHECK_VALIDATE_EVIDENCE_REQUIRED,
                &format!("/report/claims/{claim_index}"),
                checks,
            );
            validate_string_enum(
                claim,
                "evidence_requirement",
                &[
                    "none",
                    "cataloged_source",
                    "reviewed_source",
                    "verified_source",
                    "multiple_reviewed_sources",
                ],
                CHECK_VALIDATE_EVIDENCE_REQUIRED,
                &format!("/report/claims/{claim_index}"),
                checks,
            );
            if let Some(links) = claim.get("evidence_links").and_then(Value::as_array) {
                for (link_index, link) in links.iter().enumerate() {
                    let Some(link) = link.as_object() else {
                        continue;
                    };
                    validate_string_enum(
                        link,
                        "verification_status",
                        &["cataloged", "reviewed", "verified"],
                        CHECK_VALIDATE_EVIDENCE_SUPPORT,
                        &format!("/report/claims/{claim_index}/evidence_links/{link_index}"),
                        checks,
                    );
                }
            }
        }
    }

    if let Some(relations) = report.get("relations").and_then(Value::as_array) {
        for (index, relation) in relations.iter().enumerate() {
            let Some(relation) = relation.as_object() else {
                continue;
            };
            validate_string_enum(
                relation,
                "kind",
                &[
                    "depends_on",
                    "supports",
                    "qualifies",
                    "contradicts",
                    "precedes",
                    "introduces",
                    "uses_method",
                    "represented_by",
                    "grounds",
                    "motivates",
                    "part_of",
                    "maps_to",
                ],
                CHECK_VALIDATE_RELATION_KIND,
                &format!("/report/relations/{index}"),
                checks,
            );
            for endpoint_key in ["from", "to"] {
                if let Some(endpoint) = relation.get(endpoint_key).and_then(Value::as_object) {
                    validate_string_enum(
                        endpoint,
                        "entity_type",
                        &[
                            "concept",
                            "field_element",
                            "claim",
                            "source",
                            "curriculum_step",
                            "frontier_debate",
                            "method",
                            "representation",
                        ],
                        CHECK_VALIDATE_RELATION_ENDPOINT,
                        &format!("/report/relations/{index}/{endpoint_key}"),
                        checks,
                    );
                }
            }
        }
    }

    if let Some(waivers) = report.get("structure_waivers").and_then(Value::as_array) {
        for (index, waiver) in waivers.iter().enumerate() {
            let Some(waiver) = waiver.as_object() else {
                continue;
            };
            validate_string_enum(
                waiver,
                "scope",
                &["relations", "curriculum_prerequisites", "visual_views"],
                CHECK_VALIDATE_SCHEMA_REQUIRED,
                &format!("/report/structure_waivers/{index}"),
                checks,
            );
        }
    }
}

fn validate_string_enum(
    object: &serde_json::Map<String, Value>,
    key: &str,
    allowed: &[&str],
    check_id: &str,
    path: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    let Some(Value::String(value)) = object.get(key) else {
        return;
    };
    if !allowed.contains(&value.as_str()) {
        checks.push(
            DiagnosticCheck::error(
                check_id,
                format!("{path}/{key} has unknown value {value:?}"),
            )
            .with_target(format!("{path}/{key}"), ""),
        );
    }
}

#[derive(Debug, Default)]
struct ReportIdIndex {
    field_elements: BTreeSet<String>,
    concepts: BTreeSet<String>,
    methods: BTreeSet<String>,
    representations: BTreeSet<String>,
    sources: BTreeSet<String>,
    claims: BTreeSet<String>,
    curriculum_steps: BTreeSet<String>,
    frontier_debates: BTreeSet<String>,
    relations: BTreeSet<String>,
    all_entities: BTreeSet<String>,
}

impl ReportIdIndex {
    fn from_report(report: &PublicReport, checks: &mut Vec<DiagnosticCheck>) -> Self {
        let mut index = Self::default();
        let mut field_element_labels = BTreeMap::<String, String>::new();
        for (item_index, item) in report.field_elements.iter().enumerate() {
            index.insert_entity(
                "field_element",
                &item.id,
                format!("/report/field_elements/{item_index}/id"),
                checks,
            );
            for (field, value) in [
                ("element_class", item.element_class.as_str()),
                ("label", item.label.as_str()),
                ("actual_form", item.actual_form.as_str()),
                ("role", item.role.as_str()),
                (
                    "load_bearing_relations",
                    item.load_bearing_relations.as_str(),
                ),
            ] {
                if value.trim().is_empty() || is_placeholder_text(value) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_SCHEMA_REQUIRED,
                            format!(
                                "/report/field_elements/{item_index}/{field} must be non-empty and researched"
                            ),
                        )
                        .with_target(
                            format!("/report/field_elements/{item_index}/{field}"),
                            &item.id,
                        ),
                    );
                }
            }
            let normalized_label = normalize_id_text(&item.label);
            if !normalized_label.is_empty() {
                if let Some(existing_id) =
                    field_element_labels.insert(normalized_label, item.id.clone())
                {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_SCHEMA_REQUIRED,
                            format!(
                                "duplicate field element label {:?} on {} and {}",
                                item.label, existing_id, item.id
                            ),
                        )
                        .with_target(
                            format!("/report/field_elements/{item_index}/label"),
                            &item.id,
                        ),
                    );
                }
            }
        }
        for (item_index, item) in report.core_ideas.iter().enumerate() {
            index.insert_entity(
                "concept",
                &item.id,
                format!("/report/core_ideas/{item_index}/id"),
                checks,
            );
        }
        for (item_index, item) in report.methods.iter().enumerate() {
            index.insert_entity(
                "method",
                &item.id,
                format!("/report/methods/{item_index}/id"),
                checks,
            );
        }
        for (item_index, item) in report.representations.iter().enumerate() {
            index.insert_entity(
                "representation",
                &item.id,
                format!("/report/representations/{item_index}/id"),
                checks,
            );
        }
        for (item_index, source) in report.sources.iter().enumerate() {
            index.insert_entity(
                "source",
                &source.id,
                format!("/report/sources/{item_index}/id"),
                checks,
            );
        }
        for (item_index, claim) in report.claims.iter().enumerate() {
            index.insert_entity(
                "claim",
                &claim.id,
                format!("/report/claims/{item_index}/id"),
                checks,
            );
        }
        for (item_index, step) in report.curriculum_path.iter().enumerate() {
            index.insert_entity(
                "curriculum_step",
                &step.id,
                format!("/report/curriculum_path/{item_index}/id"),
                checks,
            );
        }
        for (item_index, frontier) in report.frontier_debates.iter().enumerate() {
            index.insert_entity(
                "frontier_debate",
                &frontier.id,
                format!("/report/frontier_debates/{item_index}/id"),
                checks,
            );
        }
        for (item_index, relation) in report.relations.iter().enumerate() {
            if !index.relations.insert(relation.id.clone()) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_RELATION_ENDPOINT,
                        format!("duplicate relation id {}", relation.id),
                    )
                    .with_target(format!("/report/relations/{item_index}/id"), &relation.id),
                );
            }
        }
        index
    }

    fn insert_entity(
        &mut self,
        entity_type: &str,
        id: &str,
        path: String,
        checks: &mut Vec<DiagnosticCheck>,
    ) {
        if !is_stable_id(id) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("{path} is not a stable id: {id:?}"),
                )
                .with_target(&path, id),
            );
        }
        if !self.all_entities.insert(id.to_string()) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!("duplicate public entity id {id}"),
                )
                .with_target(path.clone(), id),
            );
        }
        match entity_type {
            "field_element" => {
                self.field_elements.insert(id.to_string());
            }
            "concept" => {
                self.concepts.insert(id.to_string());
            }
            "method" => {
                self.methods.insert(id.to_string());
            }
            "representation" => {
                self.representations.insert(id.to_string());
            }
            "source" => {
                self.sources.insert(id.to_string());
            }
            "claim" => {
                self.claims.insert(id.to_string());
            }
            "curriculum_step" => {
                self.curriculum_steps.insert(id.to_string());
            }
            "frontier_debate" => {
                self.frontier_debates.insert(id.to_string());
            }
            _ => {}
        }
    }

    fn has_entity(&self, entity_type: EntityType, id: &str) -> bool {
        match entity_type {
            EntityType::FieldElement => self.field_elements.contains(id),
            EntityType::Concept => self.concepts.contains(id),
            EntityType::Claim => self.claims.contains(id),
            EntityType::Source => self.sources.contains(id),
            EntityType::CurriculumStep => self.curriculum_steps.contains(id),
            EntityType::FrontierDebate => self.frontier_debates.contains(id),
            EntityType::Method => self.methods.contains(id),
            EntityType::Representation => self.representations.contains(id),
        }
    }

    fn has_any_entity(&self, id: &str) -> bool {
        self.all_entities.contains(id)
    }
}

fn validate_embedded_diagnostics(document: &ReportDocument, checks: &mut Vec<DiagnosticCheck>) {
    let Some(diagnostics) = &document.diagnostics else {
        return;
    };
    for (index, embedded) in diagnostics.checks.iter().enumerate() {
        let mut check = embedded.clone();
        if check.target_path.trim().is_empty() {
            check.target_path = format!("/diagnostics/checks/{index}");
        }
        if embedded_diagnostic_has_accepted_loss_waiver(document, &check) {
            check.severity = DiagnosticSeverity::Info;
        }
        checks.push(check);
    }
}

fn embedded_diagnostic_has_accepted_loss_waiver(
    document: &ReportDocument,
    check: &DiagnosticCheck,
) -> bool {
    if check.status == Some(DiagnosticStatus::NotApplicable)
        && normalize_id_text(&check.message).contains("accepted loss")
    {
        return true;
    }
    embedded_visual_loss_diagnostic(check)
        && has_structure_waiver(&document.report, StructureWaiverScope::VisualViews)
}

fn embedded_visual_loss_diagnostic(check: &DiagnosticCheck) -> bool {
    check.check_id == CHECK_EXPORT_UNSUPPORTED_SECTION && {
        let message = normalize_id_text(&check.message);
        message.contains("visual summary") || message.contains("visual map")
    }
}

fn validate_public_report_boundary(
    value: &Value,
    report_type: ReportType,
    checks: &mut Vec<DiagnosticCheck>,
) {
    if report_type != ReportType::HumanReport {
        return;
    }
    let Some(report) = value.get("report") else {
        return;
    };
    scan_public_boundary(report, "/report", checks);
}

fn scan_public_boundary(value: &Value, path: &str, checks: &mut Vec<DiagnosticCheck>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let child_path = format!("{path}/{key}");
                if is_public_boundary_key(key) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_PUBLIC_BOUNDARY,
                            format!("human_report public payload contains non-public key {key:?}"),
                        )
                        .with_target(&child_path, ""),
                    );
                }
                scan_public_boundary(child, &child_path, checks);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                scan_public_boundary(child, &format!("{path}/{index}"), checks);
            }
        }
        Value::String(text) => {
            let normalized = normalize_id_text(text);
            for phrase in [
                "sentinel internal",
                "raw prompt intent",
                "prompt intent",
                "raw learner profile",
                "learner profile",
                "original goal",
                "scaffold quality notes",
                "internal context",
                "source to verify",
                "date after lookup",
                "placeholder",
                "todo",
            ] {
                if normalized.contains(phrase) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_PUBLIC_BOUNDARY,
                            format!("human_report public payload contains non-public placeholder or internal text {phrase:?}"),
                        )
                        .with_target(path.to_string(), ""),
                    );
                }
            }
        }
        _ => {}
    }
}

fn is_public_boundary_key(key: &str) -> bool {
    matches!(
        key,
        "internal_context"
            | "diagnostics"
            | "check_id"
            | "severity"
            | "target_path"
            | "raw_learner_profile"
            | "original_goal"
            | "prompt_derived_assumptions"
            | "placeholder_state"
            | "handoff_notes"
            | "raw_prompt_intent"
            | "prompt_intent"
            | "research_frame"
            | "scaffold_quality_notes"
            | "quality_gate_diagnostics"
    )
}

fn validate_reference_consistency(
    report: &PublicReport,
    index: &ReportIdIndex,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for (item_index, item) in report.literature_ladder.iter().enumerate() {
        if !is_stable_id(&item.id) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    format!(
                        "/report/literature_ladder/{item_index}/id is not a stable id: {:?}",
                        item.id
                    ),
                )
                .with_target(
                    format!("/report/literature_ladder/{item_index}/id"),
                    &item.id,
                ),
            );
        }
        for (field, value) in [
            ("layer", &item.layer),
            ("start_here", &item.start_here),
            ("read_for", &item.read_for),
            ("do_not_infer", &item.do_not_infer),
        ] {
            if value.trim().is_empty() {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_SCHEMA_REQUIRED,
                        format!("/report/literature_ladder/{item_index}/{field} must be a non-empty string"),
                    )
                    .with_target(
                        format!("/report/literature_ladder/{item_index}/{field}"),
                        &item.id,
                    ),
                );
            }
        }
        if item.source_ids.is_empty() {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_EVIDENCE_SOURCE,
                    format!("literature ladder row {} has no source_ids", item.id),
                )
                .with_target(
                    format!("/report/literature_ladder/{item_index}/source_ids"),
                    &item.id,
                ),
            );
        }
        validate_source_refs(
            &item.source_ids,
            index,
            &format!("/report/literature_ladder/{item_index}/source_ids"),
            &item.id,
            checks,
        );
    }
    for (item_index, item) in report.field_elements.iter().enumerate() {
        validate_source_refs(
            &item.source_ids,
            index,
            &format!("/report/field_elements/{item_index}/source_ids"),
            &item.id,
            checks,
        );
    }
    for (item_index, item) in report.core_ideas.iter().enumerate() {
        validate_source_refs(
            &item.source_ids,
            index,
            &format!("/report/core_ideas/{item_index}/source_ids"),
            &item.id,
            checks,
        );
    }
    for (item_index, item) in report.methods.iter().enumerate() {
        validate_source_refs(
            &item.source_ids,
            index,
            &format!("/report/methods/{item_index}/source_ids"),
            &item.id,
            checks,
        );
    }
    for (item_index, item) in report.representations.iter().enumerate() {
        validate_source_refs(
            &item.source_ids,
            index,
            &format!("/report/representations/{item_index}/source_ids"),
            &item.id,
            checks,
        );
    }
    for (step_index, step) in report.curriculum_path.iter().enumerate() {
        validate_source_refs(
            &step.source_ids,
            index,
            &format!("/report/curriculum_path/{step_index}/source_ids"),
            &step.id,
            checks,
        );
        for prerequisite_id in &step.prerequisite_ids {
            if !index.has_any_entity(prerequisite_id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_CURRICULUM_REFERENCE,
                        format!(
                            "curriculum step {} references missing prerequisite {}",
                            step.id, prerequisite_id
                        ),
                    )
                    .with_target(
                        format!("/report/curriculum_path/{step_index}/prerequisite_ids"),
                        &step.id,
                    ),
                );
            }
            if prerequisite_id.starts_with("step-")
                && !index.curriculum_steps.contains(prerequisite_id)
            {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_CURRICULUM_REFERENCE,
                        format!(
                            "curriculum step {} references missing curriculum step {}",
                            step.id, prerequisite_id
                        ),
                    )
                    .with_target(
                        format!("/report/curriculum_path/{step_index}/prerequisite_ids"),
                        &step.id,
                    ),
                );
            }
        }
    }
    for (item_index, item) in report.frontier_debates.iter().enumerate() {
        validate_source_refs(
            &item.source_ids,
            index,
            &format!("/report/frontier_debates/{item_index}/source_ids"),
            &item.id,
            checks,
        );
        for claim_id in &item.claim_ids {
            if !index.claims.contains(claim_id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_RELATION_ENDPOINT,
                        format!(
                            "frontier/debate item {} references missing claim {}",
                            item.id, claim_id
                        ),
                    )
                    .with_target(
                        format!("/report/frontier_debates/{item_index}/claim_ids"),
                        &item.id,
                    ),
                );
            }
        }
        for background_id in &item.required_background_ids {
            if !index.has_any_entity(background_id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_RELATION_ENDPOINT,
                        format!(
                            "frontier/debate item {} references missing background {}",
                            item.id, background_id
                        ),
                    )
                    .with_target(
                        format!("/report/frontier_debates/{item_index}/required_background_ids"),
                        &item.id,
                    ),
                );
            }
        }
    }
    for (relation_index, relation) in report.relations.iter().enumerate() {
        validate_source_refs(
            &relation.source_ids,
            index,
            &format!("/report/relations/{relation_index}/source_ids"),
            &relation.id,
            checks,
        );
    }
    if let Some(presentation) = &report.presentation {
        let visual_view_ids = report
            .visual_views
            .iter()
            .map(|view| view.id.as_str())
            .collect::<BTreeSet<_>>();
        let mut section_ids = BTreeSet::new();
        let mut placed_visual_ids = BTreeSet::new();
        for (section_index, section) in presentation.sections.iter().enumerate() {
            if !is_stable_id(&section.id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_SCHEMA_REQUIRED,
                        format!(
                            "/report/presentation/sections/{section_index}/id is not a stable id: {:?}",
                            section.id
                        ),
                    )
                    .with_target(
                        format!("/report/presentation/sections/{section_index}/id"),
                        &section.id,
                    ),
                );
            }
            if is_reserved_renderer_id(&section.id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_SCHEMA_REQUIRED,
                        format!(
                            "report presentation section id {} is reserved by the HTML renderer",
                            section.id
                        ),
                    )
                    .with_target(
                        format!("/report/presentation/sections/{section_index}/id"),
                        &section.id,
                    ),
                );
            }
            if !section_ids.insert(section.id.as_str()) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_SCHEMA_REQUIRED,
                        format!("duplicate report presentation section id {}", section.id),
                    )
                    .with_target(
                        format!("/report/presentation/sections/{section_index}/id"),
                        &section.id,
                    ),
                );
            }
            for visual_view_id in &section.visual_view_ids {
                if !is_stable_id(visual_view_id) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_SCHEMA_REQUIRED,
                            format!(
                                "presentation section {} has non-stable visual view id {:?}",
                                section.id, visual_view_id
                            ),
                        )
                        .with_target(
                            format!(
                                "/report/presentation/sections/{section_index}/visual_view_ids"
                            ),
                            visual_view_id,
                        ),
                    );
                }
                if !visual_view_ids.contains(visual_view_id.as_str()) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_VISUAL_REFERENCE,
                            format!(
                                "report section {} references missing visual view {}",
                                section.id, visual_view_id
                            ),
                        )
                        .with_target(
                            format!(
                                "/report/presentation/sections/{section_index}/visual_view_ids"
                            ),
                            &section.id,
                        ),
                    );
                }
                if !placed_visual_ids.insert(visual_view_id.as_str()) {
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_VISUAL_REFERENCE,
                            format!(
                                "visual view {visual_view_id} is placed more than once in report.presentation"
                            ),
                        )
                        .with_target(
                            format!(
                                "/report/presentation/sections/{section_index}/visual_view_ids"
                            ),
                            visual_view_id,
                        ),
                    );
                }
            }
        }
    }
}

fn is_reserved_renderer_id(id: &str) -> bool {
    matches!(
        id,
        "main-report"
            | "evidence-appendix"
            | "unplaced-visualizations"
            | "reading-guide"
            | "visualizations"
            | "curriculum-path"
            | "reading-ladder"
            | "claims"
            | "frontier-debates"
            | "scope"
            | "domain-profile"
            | "field-elements"
            | "core-ideas"
            | "methods"
            | "representations"
            | "evidence-standards"
            | "sources-evidence"
            | "relations"
            | "sok-visual-data"
    ) || id.starts_with("visual-title-")
}

fn validate_source_refs(
    source_ids: &[String],
    index: &ReportIdIndex,
    path: &str,
    entity_id: &str,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for source_id in source_ids {
        if !index.sources.contains(source_id) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_EVIDENCE_SOURCE,
                    format!("{entity_id} references missing source {source_id}"),
                )
                .with_target(path.to_string(), entity_id),
            );
        }
    }
}

fn validate_claim_evidence_requirements(
    report: &PublicReport,
    index: &ReportIdIndex,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for (claim_index, claim) in report.claims.iter().enumerate() {
        let path = format!("/report/claims/{claim_index}");
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
                    checks.push(
                        DiagnosticCheck::warning(
                            CHECK_VALIDATE_EVIDENCE_SUPPORT,
                            format!(
                                "claim {} has contradictory evidence from {}; record a conflict or lower-confidence interpretation",
                                claim.id, link.source_id
                            ),
                        )
                        .with_target(&link_path, &claim.id),
                    );
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
                            "claim {} evidence link {} cannot satisfy claim support without reviewed support metadata",
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
                            "claim {} has cataloged evidence links, but cataloged evidence cannot satisfy reviewed support",
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
                        "claim {} requires reviewed or verified evidence with a locator or support_note based on claim_type {:?} and evidence_requirement {:?}",
                        claim.id, claim.claim_type, claim.evidence_requirement
                    ),
                )
                .with_target(&path, &claim.id),
            );
        }
    }
}

fn validate_reviewed_evidence_link_metadata(
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

fn reviewed_link_can_support_claim(link: &EvidenceLink) -> bool {
    matches!(
        link.verification_status,
        VerificationStatus::Reviewed | VerificationStatus::Verified
    ) && link.support_kind == SupportKind::Supports
        && looks_like_iso_date(&link.reviewed_at)
        && (!link.locator.trim().is_empty() || !link.support_note.trim().is_empty())
}

fn claim_requires_reviewed_or_verified_evidence(claim: &Claim) -> bool {
    claim.evidence_requirement != EvidenceRequirement::None
        || matches!(
            claim.claim_type,
            ClaimType::Currentness | ClaimType::Frontier | ClaimType::Debate
        )
}

fn validate_source_role_coverage(report: &PublicReport, checks: &mut Vec<DiagnosticCheck>) {
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
                                "conditional source role {} has {count} source(s), minimum is {minimum}; record a waiver rationale if the condition is not active",
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

fn has_usable_waiver(waiver: &Option<SourceRoleWaiver>) -> bool {
    waiver
        .as_ref()
        .map(|waiver| !waiver.rationale.trim().is_empty() && !waiver.as_of.trim().is_empty())
        .unwrap_or(false)
}

fn validate_currentness(
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

fn validate_temporal_marker(
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

fn claim_needs_currentness_metadata(claim: &Claim) -> bool {
    matches!(
        claim.claim_type,
        ClaimType::Currentness | ClaimType::Frontier | ClaimType::Debate
    ) || matches!(
        claim.temporal.temporal_status,
        TemporalStatus::Current | TemporalStatus::ReviewDue | TemporalStatus::Stale
    )
}

fn validate_claim_source_dates(
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

fn validate_relation_consistency(
    report: &PublicReport,
    index: &ReportIdIndex,
    checks: &mut Vec<DiagnosticCheck>,
) {
    for (relation_index, relation) in report.relations.iter().enumerate() {
        validate_relation_endpoint_consistency(
            relation,
            "from",
            &relation.from,
            relation_index,
            index,
            checks,
        );
        validate_relation_endpoint_consistency(
            relation,
            "to",
            &relation.to,
            relation_index,
            index,
            checks,
        );
        if matches!(
            relation.kind,
            RelationKind::DependsOn | RelationKind::Precedes
        ) && (relation.from.entity_type == EntityType::CurriculumStep
            || relation.to.entity_type == EntityType::CurriculumStep)
            && (!index.has_entity(relation.from.entity_type, &relation.from.id)
                || !index.has_entity(relation.to.entity_type, &relation.to.id))
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_CURRICULUM_REFERENCE,
                    format!(
                        "dependency/curriculum relation {} references a missing endpoint",
                        relation.id
                    ),
                )
                .with_target(format!("/report/relations/{relation_index}"), &relation.id),
            );
        }
    }
}

fn validate_relation_endpoint_consistency(
    relation: &Relation,
    endpoint_name: &str,
    endpoint: &RelationEndpoint,
    relation_index: usize,
    index: &ReportIdIndex,
    checks: &mut Vec<DiagnosticCheck>,
) {
    if !index.has_entity(endpoint.entity_type, &endpoint.id) {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_RELATION_ENDPOINT,
                format!(
                    "relation {} {} endpoint references missing {}:{}",
                    relation.id,
                    endpoint_name,
                    entity_type_label(endpoint.entity_type),
                    endpoint.id
                ),
            )
            .with_target(
                format!("/report/relations/{relation_index}/{endpoint_name}"),
                &relation.id,
            ),
        );
    }
}

fn validate_visual_references(
    report: &PublicReport,
    index: &ReportIdIndex,
    checks: &mut Vec<DiagnosticCheck>,
) {
    let relation_lookup = report
        .relations
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect::<BTreeMap<_, _>>();
    let mut view_ids = BTreeSet::new();

    for (view_index, view) in report.visual_views.iter().enumerate() {
        let view_path = format!("/report/visual_views/{view_index}");
        if !is_stable_id(&view.id) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_VISUAL_REFERENCE,
                    format!("visual view id is not stable: {:?}", view.id),
                )
                .with_target(format!("{view_path}/id"), &view.id),
            );
        }
        if !view_ids.insert(view.id.as_str()) {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_VISUAL_REFERENCE,
                    format!("duplicate visual view id {}", view.id),
                )
                .with_target(format!("{view_path}/id"), &view.id),
            );
        }
        if visual_view_kind_label(view.kind).is_none() {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_VISUAL_REFERENCE,
                    format!("visual view {} has unsupported kind custom", view.id),
                )
                .with_target(format!("{view_path}/kind"), &view.id),
            );
        }
        if view.justification.trim().is_empty() {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_VISUAL_REFERENCE,
                    format!("visual view {} requires a non-empty justification", view.id),
                )
                .with_target(format!("{view_path}/justification"), &view.id),
            );
        }
        if view.nodes.len() < 2 {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_VISUAL_REFERENCE,
                    format!("visual view {} requires at least two nodes", view.id),
                )
                .with_target(format!("{view_path}/nodes"), &view.id),
            );
        }

        let mut node_lookup = BTreeMap::new();
        let mut duplicate_node_ids = BTreeSet::new();
        for (node_index, node) in view.nodes.iter().enumerate() {
            let node_path = format!("{view_path}/nodes/{node_index}");
            if !is_stable_id(&node.id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!(
                            "visual view {} node id is not stable: {:?}",
                            view.id, node.id
                        ),
                    )
                    .with_target(format!("{node_path}/id"), &view.id),
                );
            }
            if node_lookup.insert(node.id.as_str(), node).is_some() {
                duplicate_node_ids.insert(node.id.as_str());
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!("visual view {} has duplicate node id {}", view.id, node.id),
                    )
                    .with_target(format!("{node_path}/id"), &view.id),
                );
            }
            if node.ref_id.trim().is_empty() {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!(
                            "visual view {} node {} requires an entity ref_id",
                            view.id, node.id
                        ),
                    )
                    .with_target(format!("{node_path}/ref_id"), &view.id),
                );
            } else if !index.has_entity(node.entity_type, &node.ref_id) {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!(
                            "visual view {} node {} references missing {}:{}",
                            view.id,
                            node.id,
                            entity_type_label(node.entity_type),
                            node.ref_id
                        ),
                    )
                    .with_target(format!("{node_path}/ref_id"), &view.id),
                );
            }
        }

        let mut valid_edge_count = 0;
        for (edge_index, edge) in view.edges.iter().enumerate() {
            let edge_path = format!("{view_path}/edges/{edge_index}");
            let mut edge_is_valid = true;
            for (endpoint_name, endpoint) in [("from", &edge.from), ("to", &edge.to)] {
                if !node_lookup.contains_key(endpoint.as_str()) {
                    edge_is_valid = false;
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_VISUAL_REFERENCE,
                            format!(
                                "visual view {} edge references missing node {}",
                                view.id, endpoint
                            ),
                        )
                        .with_target(format!("{edge_path}/{endpoint_name}"), &view.id),
                    );
                } else if duplicate_node_ids.contains(endpoint.as_str()) {
                    edge_is_valid = false;
                }
            }
            if edge.from == edge.to {
                edge_is_valid = false;
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!(
                            "visual view {} edge {} -> {} is a self-edge",
                            view.id, edge.from, edge.to
                        ),
                    )
                    .with_target(&edge_path, &view.id),
                );
            }

            let relation = if edge.relation_id.trim().is_empty() {
                edge_is_valid = false;
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!(
                            "visual view {} edge {} -> {} requires a relation_id",
                            view.id, edge.from, edge.to
                        ),
                    )
                    .with_target(format!("{edge_path}/relation_id"), &view.id),
                );
                None
            } else if !index.relations.contains(&edge.relation_id) {
                edge_is_valid = false;
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_VISUAL_REFERENCE,
                        format!(
                            "visual view {} edge references missing relation {}",
                            view.id, edge.relation_id
                        ),
                    )
                    .with_target(format!("{edge_path}/relation_id"), &view.id),
                );
                None
            } else {
                relation_lookup.get(edge.relation_id.as_str()).copied()
            };

            if let (Some(relation), Some(from_node), Some(to_node)) = (
                relation,
                node_lookup.get(edge.from.as_str()).copied(),
                node_lookup.get(edge.to.as_str()).copied(),
            ) {
                let endpoints_are_valid = [from_node, to_node].into_iter().all(|node| {
                    is_stable_id(&node.id)
                        && !duplicate_node_ids.contains(node.id.as_str())
                        && !node.ref_id.trim().is_empty()
                        && index.has_entity(node.entity_type, &node.ref_id)
                });
                if !endpoints_are_valid {
                    edge_is_valid = false;
                } else if !visual_relation_matches_nodes(relation, from_node, to_node) {
                    edge_is_valid = false;
                    checks.push(
                        DiagnosticCheck::error(
                            CHECK_VALIDATE_VISUAL_REFERENCE,
                            format!(
                                "visual view {} edge {} -> {} relation {} does not match the endpoint entity references",
                                view.id, edge.from, edge.to, edge.relation_id
                            ),
                        )
                        .with_target(format!("{edge_path}/relation_id"), &view.id),
                    );
                }
            } else {
                edge_is_valid = false;
            }

            if edge_is_valid {
                valid_edge_count += 1;
            }
        }
        if valid_edge_count == 0 {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_VISUAL_REFERENCE,
                    format!(
                        "visual view {} requires at least one valid relation-backed edge",
                        view.id
                    ),
                )
                .with_target(format!("{view_path}/edges"), &view.id),
            );
        }
    }
}

fn validate_source_access_metadata(report: &PublicReport, checks: &mut Vec<DiagnosticCheck>) {
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

fn validate_structure_waivers(report: &PublicReport, checks: &mut Vec<DiagnosticCheck>) {
    for (waiver_index, waiver) in report.structure_waivers.iter().enumerate() {
        if waiver.rationale.trim().is_empty() {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_SCHEMA_REQUIRED,
                    "structure waiver rationale must be a non-empty string",
                )
                .with_target(
                    format!("/report/structure_waivers/{waiver_index}/rationale"),
                    "",
                ),
            );
        }
    }
}

fn validate_required_structure(document: &ReportDocument, checks: &mut Vec<DiagnosticCheck>) {
    let report = &document.report;
    if report.presentation.is_some() && report.field_elements.is_empty() {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_STRUCTURE_REQUIRED,
                "a declared report presentation needs non-empty report.field_elements",
            )
            .with_target("/report/field_elements", ""),
        );
    }
    if document.metadata.report_type != ReportType::HumanReport {
        return;
    }
    if document.metadata.schema_version == "sok-report/v2" {
        if report
            .presentation
            .as_ref()
            .map(|presentation| presentation.sections.is_empty())
            .unwrap_or(true)
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_STRUCTURE_REQUIRED,
                    "sok-report/v2 human_report needs a non-empty report.presentation",
                )
                .with_target("/report/presentation", ""),
            );
        }
        if report.field_elements.is_empty() {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_STRUCTURE_REQUIRED,
                    "sok-report/v2 human_report needs non-empty report.field_elements",
                )
                .with_target("/report/field_elements", ""),
            );
        }
        if report
            .presentation
            .as_ref()
            .is_some_and(|presentation| presentation.alternatives_considered.trim().is_empty())
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_STRUCTURE_REQUIRED,
                    "sok-report/v2 human_report needs a non-empty presentation.alternatives_considered decision trace",
                )
                .with_target("/report/presentation/alternatives_considered", ""),
            );
        }
        for (index, element) in report.field_elements.iter().enumerate() {
            let role = normalize_id_text(&element.role);
            if (role.contains("core") || role.contains("surrounding"))
                && element.source_ids.is_empty()
            {
                checks.push(
                    DiagnosticCheck::error(
                        CHECK_VALIDATE_EVIDENCE_SOURCE,
                        format!(
                            "field element {} has role {:?} but no source_ids",
                            element.id, element.role
                        ),
                    )
                    .with_target(
                        format!("/report/field_elements/{index}/source_ids"),
                        &element.id,
                    ),
                );
            }
        }
    }
    if substantial_final_report(report) {
        if report.relations.is_empty()
            && !has_structure_waiver(report, StructureWaiverScope::Relations)
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_STRUCTURE_REQUIRED,
                    "substantial human_report needs non-empty report.relations or a structure waiver",
                )
                .with_target("/report/relations", ""),
            );
        }
        let has_curriculum_prerequisite = report
            .curriculum_path
            .iter()
            .any(|step| !step.prerequisite_ids.is_empty());
        if report.curriculum_path.len() > 1
            && !has_curriculum_prerequisite
            && !has_structure_waiver(report, StructureWaiverScope::CurriculumPrerequisites)
        {
            checks.push(
                DiagnosticCheck::error(
                    CHECK_VALIDATE_STRUCTURE_REQUIRED,
                    "a multi-step curriculum needs at least one prerequisite or a structure waiver",
                )
                .with_target("/report/curriculum_path", ""),
            );
        }
    }

    if report.visual_views.is_empty()
        && embedded_visual_intent_was_declared(document)
        && !has_structure_waiver(report, StructureWaiverScope::VisualViews)
    {
        checks.push(
            DiagnosticCheck::error(
                CHECK_VALIDATE_STRUCTURE_REQUIRED,
                "source Markdown declared a visual map or visual summary, but report.visual_views is empty",
            )
            .with_target("/report/visual_views", ""),
        );
    }
}

fn substantial_final_report(report: &PublicReport) -> bool {
    if report.curriculum_path.len() > 1 {
        return true;
    }
    if report.claims.len() > 2 || report.frontier_debates.len() > 1 {
        return true;
    }
    let field_structure_count = if report.field_elements.is_empty() {
        report.core_ideas.len() + report.methods.len() + report.representations.len()
    } else {
        report.field_elements.len()
    };
    let structural_count = field_structure_count
        + report.literature_ladder.len()
        + report.curriculum_path.len()
        + report.frontier_debates.len();
    structural_count >= 7
}

fn embedded_visual_intent_was_declared(document: &ReportDocument) -> bool {
    document
        .diagnostics
        .as_ref()
        .map(|diagnostics| {
            diagnostics
                .checks
                .iter()
                .any(embedded_visual_loss_diagnostic)
        })
        .unwrap_or(false)
}

fn has_structure_waiver(report: &PublicReport, scope: StructureWaiverScope) -> bool {
    report
        .structure_waivers
        .iter()
        .any(|waiver| waiver.scope == scope && !waiver.rationale.trim().is_empty())
}

fn source_access_requires_notes(status: AccessStatus) -> bool {
    matches!(
        status,
        AccessStatus::PaidBook
            | AccessStatus::Paywalled
            | AccessStatus::Subscription
            | AccessStatus::Restricted
            | AccessStatus::Unknown
    )
}

fn source_access_requires_metadata_only(status: AccessStatus) -> bool {
    matches!(
        status,
        AccessStatus::PaidBook
            | AccessStatus::Paywalled
            | AccessStatus::Subscription
            | AccessStatus::Restricted
            | AccessStatus::Unknown
    )
}

fn structured_temporal_metadata_absent(marker: &TemporalMarker) -> bool {
    marker.as_of.trim().is_empty() || marker.temporal_status == TemporalStatus::Unknown
}

fn has_currentness_words(text: &str) -> bool {
    let normalized = normalize_id_text(text);
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    if words
        .iter()
        .any(|word| matches!(*word, "currently" | "recent" | "recently" | "latest"))
    {
        return true;
    }
    if contains_word_pair(&words, "as", "of") {
        return true;
    }
    if contains_review_date_construction(&words) {
        return true;
    }
    for index in 0..words.len().saturating_sub(1) {
        if words[index] != "current" {
            continue;
        }
        let next = words[index + 1];
        if is_exempt_current_compound(next) {
            continue;
        }
        if matches!(
            next,
            "state"
                | "status"
                | "evidence"
                | "literature"
                | "consensus"
                | "practice"
                | "recommendation"
                | "recommendations"
                | "guidance"
                | "frontier"
                | "standard"
                | "standards"
                | "version"
                | "review"
                | "snapshot"
        ) {
            return true;
        }
    }
    false
}

fn contains_word_pair(words: &[&str], left: &str, right: &str) -> bool {
    words
        .windows(2)
        .any(|pair| pair.first() == Some(&left) && pair.get(1) == Some(&right))
}

fn contains_review_date_construction(words: &[&str]) -> bool {
    contains_word_pair(words, "reviewed", "on")
        || contains_word_pair(words, "reviewed", "at")
        || contains_word_pair(words, "review", "date")
        || contains_word_pair(words, "review", "dated")
        || contains_word_pair(words, "last", "reviewed")
}

fn is_exempt_current_compound(next_word: &str) -> bool {
    matches!(
        next_word,
        "density" | "collector" | "focusing" | "stripping" | "critical"
    )
}

fn source_date_after_as_of(source_date: &str, as_of: &str) -> bool {
    if !looks_like_iso_date(as_of) {
        return false;
    }
    if looks_like_iso_date(source_date) {
        return source_date > as_of;
    }
    let Some(source_year) = leading_year(source_date) else {
        return false;
    };
    let Some(as_of_year) = leading_year(as_of) else {
        return false;
    };
    source_year > as_of_year
}

fn leading_year(value: &str) -> Option<i32> {
    let year = value.get(0..4)?;
    if year.chars().all(|ch| ch.is_ascii_digit()) {
        year.parse().ok()
    } else {
        None
    }
}

fn looks_like_iso_date(value: &str) -> bool {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
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

fn source_role_label(role: SourceRole) -> &'static str {
    match role {
        SourceRole::Orientation => "orientation",
        SourceRole::Foundation => "foundation",
        SourceRole::Method => "method",
        SourceRole::Representation => "representation",
        SourceRole::Evidence => "evidence",
        SourceRole::Synthesis => "synthesis",
        SourceRole::Frontier => "frontier",
        SourceRole::Debate => "debate",
        SourceRole::Standard => "standard",
        SourceRole::Dataset => "dataset",
        SourceRole::Infrastructure => "infrastructure",
        SourceRole::Critique => "critique",
        SourceRole::Curriculum => "curriculum",
        SourceRole::Other => "other",
    }
}

fn access_status_label(status: AccessStatus) -> &'static str {
    match status {
        AccessStatus::OpenAccess => "open_access",
        AccessStatus::FreeWeb => "free_web",
        AccessStatus::PublicDomain => "public_domain",
        AccessStatus::OfficialOpen => "official_open",
        AccessStatus::UserProvided => "user_provided",
        AccessStatus::Library => "library",
        AccessStatus::PaidBook => "paid_book",
        AccessStatus::Paywalled => "paywalled",
        AccessStatus::Subscription => "subscription",
        AccessStatus::Restricted => "restricted",
        AccessStatus::Unknown => "unknown",
    }
}

fn entity_type_label(entity_type: EntityType) -> &'static str {
    match entity_type {
        EntityType::Concept => "concept",
        EntityType::FieldElement => "field_element",
        EntityType::Claim => "claim",
        EntityType::Source => "source",
        EntityType::CurriculumStep => "curriculum_step",
        EntityType::FrontierDebate => "frontier_debate",
        EntityType::Method => "method",
        EntityType::Representation => "representation",
    }
}

#[derive(Debug, Clone)]
struct MarkdownSection {
    title: String,
    canonical: Option<CanonicalSection>,
    body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CanonicalSection {
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
struct ParsedNarrativeSection {
    title: String,
    purpose: String,
    body_markdown: String,
    visual_view_ids: Vec<String>,
}

#[derive(Debug, Default)]
struct ParsedMarkdownReport {
    title_field: String,
    sections: BTreeSet<CanonicalSection>,
    research_frame: BTreeMap<String, String>,
    report_architecture: BTreeMap<String, String>,
    narrative_sections: Vec<ParsedNarrativeSection>,
    domain_decomposition: String,
    orientation: String,
    deep_structure_rows: Vec<BTreeMap<String, String>>,
    source_role_rows: Vec<BTreeMap<String, String>>,
    literature_ladder_rows: Vec<BTreeMap<String, String>>,
    relation_rows: Vec<BTreeMap<String, String>>,
    curriculum_rows: Vec<BTreeMap<String, String>>,
    frontier_rows: Vec<BTreeMap<String, String>>,
    visual_view_rows: Vec<BTreeMap<String, String>>,
    visual_summary: String,
    claim_rows: Vec<BTreeMap<String, String>>,
    practice_text: String,
    usefulness_notes: Vec<String>,
    quality_notes: Vec<String>,
    placeholder_lines: Vec<String>,
    diagnostics: Vec<DiagnosticCheck>,
}

#[derive(Debug, Clone)]
struct ClaimSeed {
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
    let field_elements = build_field_elements(&parsed, &source_lookup);
    let (core_ideas, methods, representations) = build_knowledge_items(&field_elements, &parsed);
    let evidence_standards = build_evidence_standards(&parsed);
    let literature_ladder = build_literature_ladder(&parsed, &source_lookup, &mut diagnostics);
    let mut curriculum_path = build_curriculum_path(&parsed, &source_lookup);
    let (frontier_debates, frontier_claims) =
        build_frontier_debates(&parsed, &source_lookup, &as_of, &review_after);
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

fn parse_markdown_report(markdown: &str) -> ParsedMarkdownReport {
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

fn select_lint_parse_diagnostics(checks: &mut Vec<DiagnosticCheck>) -> Vec<DiagnosticCheck> {
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

fn parsed_has_any_canonical_content(parsed: &ParsedMarkdownReport) -> bool {
    !parsed.title_field.trim().is_empty()
        || !parsed.sections.is_empty()
        || !parsed.narrative_sections.is_empty()
}

fn lint_stage_boundary(
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

fn validate_markdown_report_architecture(
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

fn contains_non_public_scaffold_text(text: &str) -> bool {
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

fn lint_source_role_coverage(
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

fn lint_evidence_sources(
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

fn lint_evidence_semantics(evidence: &[EvidenceEntry], checks: &mut Vec<DiagnosticCheck>) {
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

fn lint_claim_evidence_support(
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
    let (_, frontier_claims) =
        build_frontier_debates(parsed, &source_lookup, lint_as_of, &lint_review_after);
    seeds.extend(frontier_claims);

    for seed in seeds {
        let claim_id = content_id("claim", &[&seed.statement]);
        let intended_source_ids = source_lookup.resolve_ref_list(&seed.source_refs);
        if intended_source_ids.is_empty() && !seed.source_refs.is_empty() {
            checks.push(
                DiagnosticCheck::warning(
                    CHECK_VALIDATE_EVIDENCE_SOURCE,
                    format!(
                        "claim {} references source text that does not match the source manifest",
                        claim_id
                    ),
                )
                .with_target("/report/claims", &claim_id),
            );
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

fn claim_seed_requires_evidence(seed: &ClaimSeed, stage: ExportStage) -> bool {
    stage == ExportStage::Final
        || seed.evidence_requirement != EvidenceRequirement::None
        || matches!(
            seed.claim_type,
            ClaimType::Currentness | ClaimType::Frontier | ClaimType::Debate
        )
}

fn lint_markdown_currentness(markdown: &str, checks: &mut Vec<DiagnosticCheck>) {
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

fn is_likely_currentness_table_header(line: &str) -> bool {
    let normalized = normalize_id_text(line);
    line.starts_with('|')
        && !line.contains('.')
        && !line.contains(':')
        && (normalized.contains("current state") || normalized.contains("temporal status"))
}

fn line_contains_iso_date(line: &str) -> bool {
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

fn parse_markdown_sections(
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

fn push_markdown_section(
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

fn canonical_section_from_marker(body: &str) -> Option<CanonicalSection> {
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

fn is_public_narrative_section(section: Option<CanonicalSection>) -> bool {
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

fn sok_directive_values(body: &str, key: &str) -> Vec<String> {
    standalone_sok_directives(body)
        .into_iter()
        .filter_map(|(candidate, value)| (candidate == key).then_some(value))
        .filter(|value| !value.is_empty())
        .collect()
}

fn standalone_sok_directives(body: &str) -> Vec<(String, String)> {
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

fn first_sok_directive_value(body: &str, key: &str) -> String {
    sok_directive_values(body, key)
        .into_iter()
        .next()
        .unwrap_or_default()
}

fn strip_sok_directives(body: &str) -> String {
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
struct MarkdownFence {
    marker: char,
    length: usize,
}

fn update_markdown_fence(line: &str, fence: &mut Option<MarkdownFence>) -> bool {
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

fn markdown_lines_outside_fences(markdown: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut fence = None;
    markdown.lines().enumerate().filter(move |(_, line)| {
        if update_markdown_fence(line, &mut fence) {
            return false;
        }
        fence.is_none()
    })
}

fn canonical_section(title: &str) -> Option<CanonicalSection> {
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

fn canonical_section_label(section: CanonicalSection) -> &'static str {
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

fn normalize_heading_title(title: &str) -> String {
    let normalized = normalize_id_text(title);
    normalized
        .trim_start_matches(|ch: char| ch.is_ascii_digit() || ch.is_whitespace())
        .trim()
        .to_string()
}

fn parse_key_value_table(markdown: &str) -> BTreeMap<String, String> {
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

fn parse_first_markdown_table(markdown: &str) -> Vec<BTreeMap<String, String>> {
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

fn parse_table_cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn is_table_separator(cells: &[String]) -> bool {
    cells.iter().all(|cell| {
        let value = cell.trim();
        !value.is_empty() && value.chars().all(|ch| matches!(ch, '-' | ':' | ' ' | '\t'))
    })
}

fn public_section_text(markdown: &str) -> String {
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

fn bullet_or_paragraph_lines(markdown: &str) -> Vec<String> {
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

fn clean_inline_markdown(raw: &str) -> String {
    collapse_whitespace(
        &raw.replace('`', "")
            .replace("**", "")
            .replace("__", "")
            .replace("<br>", "; ")
            .replace("<br/>", "; ")
            .replace("<br />", "; "),
    )
}

fn collapse_whitespace(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect_placeholder_lines(markdown: &str) -> Vec<String> {
    markdown_lines_outside_fences(markdown)
        .map(|(_, line)| line.trim())
        .filter(|line| is_placeholder_text(line))
        .map(clean_inline_markdown)
        .collect()
}

fn is_placeholder_text(text: &str) -> bool {
    let normalized = normalize_id_text(text);
    normalized.contains("source to verify")
        || normalized.contains("date after lookup")
        || normalized.contains("budget unknown")
        || normalized.contains("until verified")
        || normalized.contains("replace starter source")
        || normalized.contains("scaffold quality")
        || normalized.contains("sentinel internal")
}

fn lookup_cell(row: &BTreeMap<String, String>, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| row.get(*key))
        .cloned()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn lookup_cell_any(row: &BTreeMap<String, String>, keys: &[&str]) -> String {
    for key in keys {
        let normalized = normalize_id_text(key);
        if let Some(value) = row.get(&normalized) {
            return value.trim().to_string();
        }
    }
    String::new()
}

fn infer_title_field(markdown: &str) -> String {
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

fn infer_report_field(parsed: &ParsedMarkdownReport) -> String {
    first_non_empty([
        parsed
            .research_frame
            .get("Field")
            .map_or("", String::as_str),
        parsed.title_field.as_str(),
        "Unknown field",
    ])
}

fn build_scope(parsed: &ParsedMarkdownReport, diagnostics: &mut Vec<DiagnosticCheck>) -> Scope {
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

fn build_domain_profile(
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

fn domain_classifications_from_text(raw: &str) -> Vec<DomainClassification> {
    let text = normalize_id_text(raw);
    let mut out = Vec::new();
    for (needle, classification) in [
        ("formal", DomainClassification::Formal),
        ("well structured", DomainClassification::WellStructured),
        ("ill structured", DomainClassification::IllStructured),
        (
            "professional practice",
            DomainClassification::ProfessionalPractice,
        ),
        ("instrument bound", DomainClassification::InstrumentBound),
        (
            "infrastructure bound",
            DomainClassification::InfrastructureBound,
        ),
        ("emerging", DomainClassification::Emerging),
        ("interdisciplinary", DomainClassification::Interdisciplinary),
        ("mixed", DomainClassification::Mixed),
    ] {
        if text.contains(needle) && !out.contains(&classification) {
            out.push(classification);
        }
    }
    out
}

fn build_field_elements(
    parsed: &ParsedMarkdownReport,
    source_lookup: &SourceLookup,
) -> Vec<FieldElement> {
    parsed
        .deep_structure_rows
        .iter()
        .filter(|row| is_usable_deep_structure_row(row))
        .map(|row| {
            let label = lookup_cell_any(row, &["Observed element", "Element", "Name", "Title"]);
            let explicit_id =
                clean_inline_markdown(&lookup_cell_any(row, &["Element ID", "Element Id", "ID"]));
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
            FieldElement {
                id: if explicit_id.trim().is_empty() {
                    content_id("element", &[&label])
                } else {
                    explicit_id
                },
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
                source_ids: source_lookup.resolve_refs(&lookup_cell_any(
                    row,
                    &["Source IDs", "Sources", "Key sources", "Readings"],
                )),
                confidence: parse_claim_confidence(&lookup_cell_any(row, &["Confidence"])),
            }
        })
        .collect()
}

fn build_knowledge_items(
    field_elements: &[FieldElement],
    parsed: &ParsedMarkdownReport,
) -> (Vec<KnowledgeItem>, Vec<KnowledgeItem>, Vec<KnowledgeItem>) {
    let mut core_ideas = Vec::new();
    let mut methods = Vec::new();
    let mut representations = Vec::new();
    for element in field_elements {
        let normalized_class = normalize_id_text(&first_non_empty([
            element.element_class.as_str(),
            element.label.as_str(),
        ]));
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
            id: content_id(knowledge_prefix(&normalized_class), &[&element.label]),
            label: element.label.clone(),
            description,
            source_ids: element.source_ids.clone(),
            ..KnowledgeItem::default()
        };
        if normalized_class.contains("representation")
            || normalized_class.contains("model")
            || normalized_class.contains("notation")
        {
            representations.push(item);
        } else if normalized_class.contains("syntactic")
            || normalized_class.contains("method")
            || normalized_class.contains("operation")
            || normalized_class.contains("practice")
            || normalized_class.contains("proof")
            || normalized_class.contains("warrant")
            || normalized_class.contains("evidence standard")
        {
            methods.push(item);
        } else if normalized_class.contains("failure mode")
            || normalized_class.contains("frontier")
            || normalized_class.contains("dispute")
            || normalized_class.contains("debate")
            || normalized_class.contains("open problem")
        {
            continue;
        } else {
            core_ideas.push(item);
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

fn knowledge_prefix(normalized_element: &str) -> &'static str {
    if normalized_element.contains("representation")
        || normalized_element.contains("model")
        || normalized_element.contains("notation")
    {
        "rep"
    } else if normalized_element.contains("method")
        || normalized_element.contains("operation")
        || normalized_element.contains("practice")
        || normalized_element.contains("syntactic")
        || normalized_element.contains("proof")
        || normalized_element.contains("warrant")
        || normalized_element.contains("evidence standard")
    {
        "method"
    } else {
        "concept"
    }
}

fn deep_structure_description(row: &BTreeMap<String, String>) -> String {
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

fn is_usable_deep_structure_row(row: &BTreeMap<String, String>) -> bool {
    let observed_element = lookup_cell_any(row, &["Observed element", "Element", "Name", "Title"]);
    let description = deep_structure_description(row);
    !observed_element.is_empty() && !description.is_empty() && !is_placeholder_text(&description)
}

fn build_evidence_standards(parsed: &ParsedMarkdownReport) -> EvidenceStandards {
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

fn build_literature_ladder(
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
        let source_ids = source_lookup.resolve_refs(&source_refs);
        if source_ids.is_empty() {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_UNRESOLVED_REFERENCE,
                    format!(
                        "Literature Ladder row {} could not resolve source reference(s) {:?}",
                        index + 1,
                        source_refs
                    ),
                )
                .with_target(format!("/report/literature_ladder/{index}/source_ids"), &id),
            );
        }

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

fn parse_source_requirement(raw: &str) -> SourceRoleRequirementKind {
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

fn build_curriculum_path(
    parsed: &ParsedMarkdownReport,
    source_lookup: &SourceLookup,
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
        let source_ids = source_lookup.resolve_refs(&lookup_cell_any(
            row,
            &["Readings", "Sources", "Source IDs"],
        ));
        steps.push(CurriculumStep {
            id: content_id("step", &[&title]),
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

fn curriculum_row_title(row: &BTreeMap<String, String>) -> String {
    first_non_empty([
        lookup_cell_any(row, &["Module"]).as_str(),
        lookup_cell_any(row, &["Phase"]).as_str(),
        lookup_cell_any(row, &["Title"]).as_str(),
    ])
}

fn apply_curriculum_prerequisites(
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
                    DiagnosticCheck::warning(
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

fn build_frontier_debates(
    parsed: &ParsedMarkdownReport,
    source_lookup: &SourceLookup,
    as_of: &str,
    review_after: &str,
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
        seen_titles.insert(normalize_id_text(&title));
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
        let source_ids = source_lookup.resolve_refs(&source_refs);
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
            id: content_id("frontier", &[&title]),
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
            || !seen_titles.insert(normalize_id_text(&title))
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
        let source_ids = source_lookup.resolve_refs(&source_refs);
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
            id: content_id("frontier", &[&title]),
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

fn build_claim_seeds(parsed: &ParsedMarkdownReport) -> Vec<ClaimSeed> {
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

fn build_claims(
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
        let intended_source_ids = source_lookup.resolve_ref_list(&seed.source_refs);
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

fn duplicate_claim_evidence_link(existing: &EvidenceLink, next: &EvidenceLink) -> bool {
    if existing.verification_status == VerificationStatus::Cataloged
        && next.verification_status == VerificationStatus::Cataloged
    {
        return existing.source_id == next.source_id;
    }
    existing.evidence_id == next.evidence_id && existing.source_id == next.source_id
}

fn build_relations(
    parsed: &ParsedMarkdownReport,
    entity_index: &ExportEntityIndex,
    source_lookup: &SourceLookup,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<Relation> {
    let mut relations = Vec::new();
    let mut seen_ids = BTreeSet::new();
    for (index, row) in parsed.relation_rows.iter().enumerate() {
        let kind_text = lookup_cell_any(row, &["Relation kind", "Kind", "Type"]);
        let Some(kind) = parse_relation_kind_input(&kind_text) else {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_MISSING_PUBLIC_FIELD,
                    format!(
                        "Relations row {} has unsupported or missing relation kind {:?}",
                        index + 1,
                        kind_text
                    ),
                )
                .with_target(format!("/report/relations/{index}/kind"), ""),
            );
            continue;
        };

        let Some(from) = resolve_relation_endpoint_from_row(
            row,
            index,
            "from",
            &["From type", "Source type", "Subject type"],
            &[
                "From reference",
                "From ref",
                "From",
                "Source reference",
                "Subject",
            ],
            entity_index,
            diagnostics,
        ) else {
            continue;
        };
        let Some(to) = resolve_relation_endpoint_from_row(
            row,
            index,
            "to",
            &["To type", "Target type", "Object type"],
            &["To reference", "To ref", "To", "Target reference", "Object"],
            entity_index,
            diagnostics,
        ) else {
            continue;
        };

        let explicit_id = lookup_cell_any(row, &["Relation ID", "Relation id", "ID"]);
        let id = if explicit_id.trim().is_empty() {
            content_id(
                "rel",
                &[
                    relation_kind_label(kind),
                    entity_type_label(from.entity_type),
                    &from.id,
                    entity_type_label(to.entity_type),
                    &to.id,
                ],
            )
        } else if is_stable_id(&explicit_id) {
            explicit_id
        } else {
            content_id("rel", &[&explicit_id])
        };
        if !seen_ids.insert(id.clone()) {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_AMBIGUOUS_REFERENCE,
                    format!("Relations row {} repeats relation id {}", index + 1, id),
                )
                .with_target(format!("/report/relations/{index}/id"), &id),
            );
            continue;
        }

        let source_refs = lookup_cell_any(
            row,
            &[
                "Source IDs",
                "Source ids",
                "Sources",
                "Evidence sources",
                "Source references",
            ],
        );
        let source_ids = source_lookup.resolve_refs(&source_refs);
        if !source_refs.trim().is_empty() && source_ids.is_empty() {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_UNRESOLVED_REFERENCE,
                    format!(
                        "relation {} could not resolve source reference(s) {:?}",
                        id, source_refs
                    ),
                )
                .with_target(format!("/report/relations/{index}/source_ids"), &id),
            );
        }

        relations.push(Relation {
            id,
            kind,
            from,
            to,
            description: lookup_cell_any(row, &["Rationale", "Description", "Why"]),
            source_ids,
        });
    }
    relations
}

fn resolve_relation_endpoint_from_row(
    row: &BTreeMap<String, String>,
    row_index: usize,
    endpoint_name: &str,
    type_keys: &[&str],
    reference_keys: &[&str],
    entity_index: &ExportEntityIndex,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Option<RelationEndpoint> {
    let type_text = lookup_cell_any(row, type_keys);
    let reference = lookup_cell_any(row, reference_keys);
    let Some(entity_type) = parse_entity_type_input(&type_text) else {
        diagnostics.push(
            DiagnosticCheck::warning(
                CHECK_EXPORT_MISSING_PUBLIC_FIELD,
                format!(
                    "Relations row {} has unsupported or missing {endpoint_name} entity type {:?}",
                    row_index + 1,
                    type_text
                ),
            )
            .with_target(
                format!("/report/relations/{row_index}/{endpoint_name}/entity_type"),
                "",
            ),
        );
        return None;
    };
    if reference.trim().is_empty() {
        diagnostics.push(
            DiagnosticCheck::warning(
                CHECK_EXPORT_MISSING_PUBLIC_FIELD,
                format!(
                    "Relations row {} is missing {endpoint_name} reference",
                    row_index + 1
                ),
            )
            .with_target(
                format!("/report/relations/{row_index}/{endpoint_name}/id"),
                "",
            ),
        );
        return None;
    }
    match entity_index.resolve_typed(entity_type, &reference) {
        ExportReferenceResolution::Resolved(entity) => Some(RelationEndpoint {
            entity_type,
            id: entity.id,
        }),
        ExportReferenceResolution::Missing => {
            let type_hint = match entity_index.resolve_any(&reference) {
                ExportReferenceResolution::Resolved(candidate)
                    if candidate.entity_type != entity_type =>
                {
                    format!(
                        "; the field-element inventory exports this reference as {}, so revise the declared endpoint type if that classification is intended",
                        entity_type_label(candidate.entity_type)
                    )
                }
                _ => String::new(),
            };
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_UNRESOLVED_REFERENCE,
                    format!(
                        "Relations row {} could not resolve {endpoint_name} endpoint {}:{:?}{type_hint}",
                        row_index + 1,
                        entity_type_label(entity_type),
                        reference
                    ),
                )
                .with_target(
                    format!("/report/relations/{row_index}/{endpoint_name}/id"),
                    "",
                ),
            );
            None
        }
        ExportReferenceResolution::Ambiguous(candidates) => {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_EXPORT_AMBIGUOUS_REFERENCE,
                    format!(
                        "Relations row {} {endpoint_name} endpoint {:?} is ambiguous for {}: {}",
                        row_index + 1,
                        reference,
                        entity_type_label(entity_type),
                        candidates.join(", ")
                    ),
                )
                .with_target(
                    format!("/report/relations/{row_index}/{endpoint_name}/id"),
                    "",
                ),
            );
            None
        }
    }
}

fn build_visual_views(
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

fn insert_visual_node(
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

fn apply_visual_node_emphasis(
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
        let normalized_reference = normalize_id_text(&reference);
        let matching_existing_nodes = nodes_by_entity_id
            .iter()
            .filter(|(_, node)| {
                node.ref_id == reference || normalize_id_text(&node.label) == normalized_reference
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
                DiagnosticCheck::warning(
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

fn visual_node_id(entity_type: EntityType, entity_id: &str) -> String {
    content_id("vnode", &[entity_type_label(entity_type), entity_id])
}

fn visual_view_justification(purpose: &str, visual_summary: &str) -> String {
    match (purpose.trim().is_empty(), visual_summary.trim().is_empty()) {
        (false, false) => format!("{purpose} Visual guidance: {visual_summary}"),
        (false, true) => purpose.trim().to_string(),
        (true, false) => visual_summary.trim().to_string(),
        (true, true) => "Relation-backed visual view declared in Markdown.".to_string(),
    }
}

fn warn_if_unpreserved_visual_sections(
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

fn evidence_source_diagnostics(
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

fn apply_evidence_review_status(sources: &mut [ReportSource], evidence: &[EvidenceEntry]) {
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

fn max_verification_status(
    left: VerificationStatus,
    right: VerificationStatus,
) -> VerificationStatus {
    if verification_rank(right) > verification_rank(left) {
        right
    } else {
        left
    }
}

fn verification_rank(status: VerificationStatus) -> u8 {
    match status {
        VerificationStatus::Cataloged => 0,
        VerificationStatus::Reviewed => 1,
        VerificationStatus::Verified => 2,
    }
}

fn build_report_presentation(
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

fn architecture_value(values: &BTreeMap<String, String>, keys: &[&str]) -> String {
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

fn build_internal_context(parsed: &ParsedMarkdownReport) -> InternalContext {
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
        handoff_notes,
    }
}

fn warn_if_empty_public_sections(
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

fn parsed_has_section(parsed: &ParsedMarkdownReport, section: CanonicalSection) -> bool {
    parsed.sections.contains(&section)
}

#[derive(Debug)]
struct SourceLookup {
    aliases: BTreeMap<String, String>,
}

impl SourceLookup {
    fn new(sources: &[ReportSource]) -> Self {
        let mut aliases = BTreeMap::new();
        for (index, source) in sources.iter().enumerate() {
            for alias in [
                source.id.as_str(),
                source.title.as_str(),
                source.citation.as_str(),
                source.identifier.as_str(),
                source.url.as_str(),
            ] {
                let normalized = normalize_id_text(alias);
                if !normalized.is_empty() {
                    aliases.insert(normalized, source.id.clone());
                }
            }
            aliases.insert(format!("s{}", index + 1), source.id.clone());
        }
        Self { aliases }
    }

    fn resolve_refs(&self, refs: &str) -> Vec<String> {
        self.resolve_ref_list(&split_ref_list(refs))
    }

    fn resolve_ref_list(&self, refs: &[String]) -> Vec<String> {
        let mut ids = BTreeSet::new();
        for reference in refs {
            let normalized = normalize_id_text(reference);
            if normalized.is_empty() {
                continue;
            }
            if let Some(id) = self.aliases.get(&normalized) {
                ids.insert(id.clone());
                continue;
            }
            for (alias, id) in &self.aliases {
                if alias.contains(&normalized) || normalized.contains(alias) {
                    ids.insert(id.clone());
                }
            }
        }
        ids.into_iter().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExportEntityRef {
    entity_type: EntityType,
    id: String,
    label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExportReferenceResolution {
    Resolved(ExportEntityRef),
    Missing,
    Ambiguous(Vec<String>),
}

#[derive(Debug, Default)]
struct ExportEntityIndex {
    by_exact_id: BTreeMap<String, ExportEntityRef>,
    aliases_by_type: BTreeMap<EntityType, BTreeMap<String, BTreeSet<String>>>,
    aliases_all: BTreeMap<String, BTreeSet<String>>,
}

impl ExportEntityIndex {
    fn new(
        core_ideas: &[KnowledgeItem],
        methods: &[KnowledgeItem],
        representations: &[KnowledgeItem],
        sources: &[ReportSource],
        claims: &[Claim],
        curriculum_path: &[CurriculumStep],
        frontier_debates: &[FrontierDebateItem],
    ) -> Self {
        let mut index = Self::default();
        for item in core_ideas {
            index.insert(
                EntityType::Concept,
                &item.id,
                &item.label,
                [&item.id, &item.label],
            );
            for alias in &item.aliases {
                index.insert_alias(EntityType::Concept, &item.id, alias);
            }
        }
        for item in methods {
            index.insert(
                EntityType::Method,
                &item.id,
                &item.label,
                [&item.id, &item.label],
            );
            for alias in &item.aliases {
                index.insert_alias(EntityType::Method, &item.id, alias);
            }
        }
        for item in representations {
            index.insert(
                EntityType::Representation,
                &item.id,
                &item.label,
                [&item.id, &item.label],
            );
            for alias in &item.aliases {
                index.insert_alias(EntityType::Representation, &item.id, alias);
            }
        }
        for (source_index, source) in sources.iter().enumerate() {
            index.insert(
                EntityType::Source,
                &source.id,
                &first_non_empty([source.title.as_str(), source.citation.as_str(), &source.id]),
                [
                    source.id.as_str(),
                    source.title.as_str(),
                    source.citation.as_str(),
                    source.identifier.as_str(),
                    source.url.as_str(),
                ],
            );
            index.insert_alias(
                EntityType::Source,
                &source.id,
                &format!("S{}", source_index + 1),
            );
        }
        for claim in claims {
            index.insert(
                EntityType::Claim,
                &claim.id,
                &claim.statement,
                [&claim.id, &claim.statement],
            );
        }
        for step in curriculum_path {
            index.insert(
                EntityType::CurriculumStep,
                &step.id,
                &step.title,
                [&step.id, &step.title],
            );
        }
        for item in frontier_debates {
            index.insert(
                EntityType::FrontierDebate,
                &item.id,
                &item.title,
                [&item.id, &item.title],
            );
        }
        index
    }

    fn add_field_elements(&mut self, field_elements: &[FieldElement]) {
        for item in field_elements {
            self.insert(
                EntityType::FieldElement,
                &item.id,
                &item.label,
                [&item.id, &item.label],
            );
        }
    }

    fn insert<const N: usize>(
        &mut self,
        entity_type: EntityType,
        id: &str,
        label: &str,
        aliases: [&str; N],
    ) {
        if id.trim().is_empty() {
            return;
        }
        let entity = ExportEntityRef {
            entity_type,
            id: id.to_string(),
            label: first_non_empty([label, id]),
        };
        self.by_exact_id.insert(id.to_string(), entity);
        for alias in aliases {
            self.insert_alias(entity_type, id, alias);
        }
    }

    fn insert_alias(&mut self, entity_type: EntityType, id: &str, alias: &str) {
        let normalized = normalize_id_text(alias);
        if normalized.is_empty() {
            return;
        }
        self.aliases_by_type
            .entry(entity_type)
            .or_default()
            .entry(normalized.clone())
            .or_default()
            .insert(id.to_string());
        self.aliases_all
            .entry(normalized)
            .or_default()
            .insert(id.to_string());
    }

    fn resolve_typed(&self, entity_type: EntityType, reference: &str) -> ExportReferenceResolution {
        let reference = reference.trim();
        if reference.is_empty() {
            return ExportReferenceResolution::Missing;
        }
        if let Some(entity) = self.by_exact_id.get(reference) {
            if entity.entity_type == entity_type {
                return ExportReferenceResolution::Resolved(entity.clone());
            }
            return ExportReferenceResolution::Missing;
        }
        let normalized = normalize_id_text(reference);
        let Some(candidates) = self
            .aliases_by_type
            .get(&entity_type)
            .and_then(|aliases| aliases.get(&normalized))
        else {
            return ExportReferenceResolution::Missing;
        };
        self.resolve_candidate_set(candidates)
    }

    fn resolve_any(&self, reference: &str) -> ExportReferenceResolution {
        let reference = reference.trim();
        if reference.is_empty() {
            return ExportReferenceResolution::Missing;
        }
        if let Some(entity) = self.by_exact_id.get(reference) {
            return ExportReferenceResolution::Resolved(entity.clone());
        }
        let normalized = normalize_id_text(reference);
        let Some(candidates) = self.aliases_all.get(&normalized) else {
            return ExportReferenceResolution::Missing;
        };
        self.resolve_candidate_set(candidates)
    }

    fn resolve_any_legacy_projection(&self, reference: &str) -> ExportReferenceResolution {
        let reference = reference.trim();
        if reference.is_empty() {
            return ExportReferenceResolution::Missing;
        }
        if let Some(entity) = self.by_exact_id.get(reference) {
            return ExportReferenceResolution::Resolved(entity.clone());
        }
        let normalized = normalize_id_text(reference);
        let Some(candidates) = self.aliases_all.get(&normalized) else {
            return ExportReferenceResolution::Missing;
        };
        let legacy_candidates = candidates
            .iter()
            .filter(|id| {
                self.by_exact_id
                    .get(*id)
                    .map(|entity| entity.entity_type != EntityType::FieldElement)
                    .unwrap_or(false)
            })
            .cloned()
            .collect::<BTreeSet<_>>();
        if legacy_candidates.is_empty() {
            self.resolve_candidate_set(candidates)
        } else {
            self.resolve_candidate_set_without_field_preference(&legacy_candidates)
        }
    }

    fn resolve_candidate_set(&self, candidates: &BTreeSet<String>) -> ExportReferenceResolution {
        if candidates.len() == 1 {
            let id = candidates
                .iter()
                .next()
                .expect("candidate set is not empty");
            self.by_exact_id
                .get(id)
                .cloned()
                .map(ExportReferenceResolution::Resolved)
                .unwrap_or(ExportReferenceResolution::Missing)
        } else {
            let field_elements = candidates
                .iter()
                .filter_map(|id| self.by_exact_id.get(id))
                .filter(|entity| entity.entity_type == EntityType::FieldElement)
                .collect::<Vec<_>>();
            if field_elements.len() == 1 {
                let field_element = field_elements[0];
                let field_label = normalize_id_text(&field_element.label);
                let only_derived_projections = candidates
                    .iter()
                    .filter_map(|id| self.by_exact_id.get(id))
                    .filter(|entity| entity.id != field_element.id)
                    .all(|entity| {
                        matches!(
                            entity.entity_type,
                            EntityType::Concept | EntityType::Method | EntityType::Representation
                        ) && normalize_id_text(&entity.label) == field_label
                    });
                if only_derived_projections {
                    return ExportReferenceResolution::Resolved(field_element.clone());
                }
            }
            self.resolve_candidate_set_without_field_preference(candidates)
        }
    }

    fn resolve_candidate_set_without_field_preference(
        &self,
        candidates: &BTreeSet<String>,
    ) -> ExportReferenceResolution {
        if candidates.len() == 1 {
            let id = candidates
                .iter()
                .next()
                .expect("candidate set is not empty");
            self.by_exact_id
                .get(id)
                .cloned()
                .map(ExportReferenceResolution::Resolved)
                .unwrap_or(ExportReferenceResolution::Missing)
        } else {
            ExportReferenceResolution::Ambiguous(candidates.iter().cloned().collect())
        }
    }

    fn record(&self, id: &str) -> Option<&ExportEntityRef> {
        self.by_exact_id.get(id)
    }
}

fn split_ref_list(raw: &str) -> Vec<String> {
    raw.split([',', ';', '\n'])
        .map(clean_inline_markdown)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty() && !is_placeholder_text(item))
        .collect()
}

fn split_listish(raw: &str) -> Vec<String> {
    raw.split([';', '\n'])
        .flat_map(|part| part.split(" / "))
        .map(clean_inline_markdown)
        .map(|item| item.trim().trim_start_matches("- ").to_string())
        .filter(|item| !item.is_empty() && !is_placeholder_text(item))
        .collect()
}

fn parse_entity_type_input(raw: &str) -> Option<EntityType> {
    match normalize_access_status_label(raw).as_str() {
        "field_element" | "field_elements" | "element" | "elements" => {
            Some(EntityType::FieldElement)
        }
        "concept" | "core_idea" | "core_ideas" | "idea" | "threshold_concept" => {
            Some(EntityType::Concept)
        }
        "claim" | "claims" => Some(EntityType::Claim),
        "source" | "sources" | "src" | "reading" => Some(EntityType::Source),
        "curriculum_step" | "curriculum" | "step" | "module" => Some(EntityType::CurriculumStep),
        "frontier_debate" | "frontier" | "debate" | "open_problem" => {
            Some(EntityType::FrontierDebate)
        }
        "method" | "methods" | "warrant" | "warrants" => Some(EntityType::Method),
        "representation" | "representations" | "rep" => Some(EntityType::Representation),
        _ => None,
    }
}

fn parse_relation_kind_input(raw: &str) -> Option<RelationKind> {
    match normalize_access_status_label(raw).as_str() {
        "depends_on" | "depends" | "requires" | "prerequisite" | "prerequisite_for" => {
            Some(RelationKind::DependsOn)
        }
        "supports" | "support" | "supported_by" => Some(RelationKind::Supports),
        "qualifies" | "qualify" | "limits" | "conditions" => Some(RelationKind::Qualifies),
        "contradicts" | "contradict" | "conflicts" => Some(RelationKind::Contradicts),
        "precedes" | "precedes_in_curriculum" | "before" | "prior_to" => {
            Some(RelationKind::Precedes)
        }
        "introduces" | "introduce" => Some(RelationKind::Introduces),
        "uses_method" | "uses" | "uses_method_or_warrant" => Some(RelationKind::UsesMethod),
        "represented_by" | "represented" | "has_representation" => {
            Some(RelationKind::RepresentedBy)
        }
        "grounds" | "grounded_by" => Some(RelationKind::Grounds),
        "motivates" | "motivation" => Some(RelationKind::Motivates),
        "part_of" | "contains" | "component_of" => Some(RelationKind::PartOf),
        "maps_to" | "maps" | "corresponds_to" => Some(RelationKind::MapsTo),
        _ => None,
    }
}

fn relation_kind_label(kind: RelationKind) -> &'static str {
    match kind {
        RelationKind::DependsOn => "depends_on",
        RelationKind::Supports => "supports",
        RelationKind::Qualifies => "qualifies",
        RelationKind::Contradicts => "contradicts",
        RelationKind::Precedes => "precedes",
        RelationKind::Introduces => "introduces",
        RelationKind::UsesMethod => "uses_method",
        RelationKind::RepresentedBy => "represented_by",
        RelationKind::Grounds => "grounds",
        RelationKind::Motivates => "motivates",
        RelationKind::PartOf => "part_of",
        RelationKind::MapsTo => "maps_to",
    }
}

fn parse_visual_view_kind_input(raw: &str) -> VisualViewKind {
    match normalize_access_status_label(raw).as_str() {
        "knowledge_spine" | "spine" => VisualViewKind::KnowledgeSpine,
        "concept_source" | "source_map" | "concept_source_map" => VisualViewKind::ConceptSource,
        "dependency_path" | "curriculum_path" | "prerequisite_path" => {
            VisualViewKind::DependencyPath
        }
        "frontier_debate" | "frontier_debate_map" | "debate_map" => VisualViewKind::FrontierDebate,
        _ => VisualViewKind::Custom,
    }
}

fn visual_view_kind_export_label(kind: VisualViewKind) -> &'static str {
    match kind {
        VisualViewKind::KnowledgeSpine => "knowledge_spine",
        VisualViewKind::ConceptSource => "concept_source",
        VisualViewKind::DependencyPath => "dependency_path",
        VisualViewKind::FrontierDebate => "frontier_debate",
        VisualViewKind::Custom => "custom",
    }
}

fn parse_source_role(raw: &str) -> SourceRole {
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

fn parse_claim_type(raw: &str) -> ClaimType {
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

fn parse_evidence_requirement(raw: &str) -> EvidenceRequirement {
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

fn parse_claim_confidence(raw: &str) -> Option<ClaimConfidence> {
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

fn parse_temporal_status(raw: &str) -> Option<TemporalStatus> {
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

impl DiagnosticCheck {
    pub fn new(
        check_id: impl Into<String>,
        severity: DiagnosticSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            check_id: check_id.into(),
            severity,
            message: message.into(),
            ..Self::default()
        }
    }

    pub fn warning(check_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(check_id, DiagnosticSeverity::Warning, message)
    }

    pub fn error(check_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(check_id, DiagnosticSeverity::Error, message)
    }

    pub fn with_target(
        mut self,
        target_path: impl Into<String>,
        entity_id: impl Into<String>,
    ) -> Self {
        self.target_path = target_path.into();
        self.entity_id = entity_id.into();
        self
    }
}

fn sort_diagnostics(checks: &mut [DiagnosticCheck]) {
    checks.sort_by(|left, right| {
        severity_sort_key(left.severity)
            .cmp(&severity_sort_key(right.severity))
            .then_with(|| left.check_id.cmp(&right.check_id))
            .then_with(|| left.target_path.cmp(&right.target_path))
            .then_with(|| left.entity_id.cmp(&right.entity_id))
            .then_with(|| left.message.cmp(&right.message))
    });
}

fn severity_sort_key(severity: DiagnosticSeverity) -> u8 {
    match severity {
        DiagnosticSeverity::Error => 0,
        DiagnosticSeverity::Warning => 1,
        DiagnosticSeverity::Info => 2,
    }
}

impl EvidenceEntry {
    pub fn cataloged_from_source(
        source: &Source,
        report_source: &ReportSource,
        input_path: &Path,
        row_number: usize,
    ) -> Self {
        let source_key = source_content_key(source);
        Self {
            evidence_id: content_id("ev", &[&report_source.id, &source_key, "cataloged"]),
            source_id: report_source.id.clone(),
            input_provenance: BoundedInputProvenance {
                input_path: input_path.display().to_string(),
                input_kind: input_kind_for_path(input_path),
                row_number: Some(row_number),
                row_hash: short_hash(&normalize_id_text(&source_key), 16),
            },
            verification_status: VerificationStatus::Cataloged,
            support_kind: SupportKind::Background,
            locator: String::new(),
            support_note: String::new(),
            notes: "Cataloged from source manifest only; this row has not been read, crawled, or externally verified and cannot satisfy claim evidence.".to_string(),
            claim_ids: Vec::new(),
            source_roles: report_source.roles.clone(),
            source_access: Some(report_source.access.clone()),
            source_citation: report_source.citation.clone(),
            source_identifier: report_source.identifier.clone(),
            source_url: report_source.url.clone(),
            curricular_use: source_curricular_use(source),
            ..Self::default()
        }
    }

    pub fn has_usable_support_metadata(&self) -> bool {
        !self.locator.trim().is_empty() || !self.support_note.trim().is_empty()
    }

    pub fn can_satisfy_claim_link(&self) -> bool {
        matches!(
            self.verification_status,
            VerificationStatus::Reviewed | VerificationStatus::Verified
        ) && self.support_kind == SupportKind::Supports
            && self.has_usable_support_metadata()
            && looks_like_iso_date(&self.reviewed_at)
    }

    pub fn to_evidence_link(&self) -> Option<EvidenceLink> {
        if !self.can_satisfy_claim_link() {
            return None;
        }
        Some(self.as_evidence_link())
    }

    pub fn to_visible_evidence_link(&self) -> Option<EvidenceLink> {
        match self.verification_status {
            VerificationStatus::Cataloged => Some(self.as_evidence_link()),
            VerificationStatus::Reviewed | VerificationStatus::Verified
                if self.has_usable_support_metadata() && looks_like_iso_date(&self.reviewed_at) =>
            {
                Some(self.as_evidence_link())
            }
            _ => None,
        }
    }

    fn as_evidence_link(&self) -> EvidenceLink {
        EvidenceLink {
            evidence_id: self.evidence_id.clone(),
            source_id: self.source_id.clone(),
            verification_status: self.verification_status,
            support_kind: self.support_kind,
            locator: self.locator.clone(),
            support_note: self.support_note.clone(),
            reviewed_at: self.reviewed_at.clone(),
        }
    }
}

impl AccessStatus {
    pub fn from_manifest_status(raw: &str) -> Self {
        match normalize_access_status_label(raw).as_str() {
            "open" | "open_access" | "oa" | "cc_by" | "cc_by_sa" => Self::OpenAccess,
            "free" | "free_web" | "free_to_read" => Self::FreeWeb,
            "public_domain" => Self::PublicDomain,
            "official" | "official_open" => Self::OfficialOpen,
            "user" | "user_provided" => Self::UserProvided,
            "library" | "library_access" => Self::Library,
            "paid" | "paid_book" | "book_purchase" => Self::PaidBook,
            "paywall" | "paywalled" => Self::Paywalled,
            "subscription" | "institutional_subscription" => Self::Subscription,
            "restricted" => Self::Restricted,
            _ => Self::Unknown,
        }
    }

    pub fn metadata_only_default(self) -> bool {
        matches!(
            self,
            Self::PaidBook
                | Self::Paywalled
                | Self::Subscription
                | Self::Restricted
                | Self::Unknown
        )
    }
}

pub fn normalize_source_manifest<P: AsRef<Path>>(path: P) -> Result<NormalizedSourceManifest> {
    let path = path.as_ref();
    let raw_sources = load_sources(path)?;
    let sources = normalize_report_sources(&raw_sources);
    let evidence = raw_sources
        .iter()
        .zip(sources.iter())
        .enumerate()
        .map(|(index, (source, report_source))| {
            EvidenceEntry::cataloged_from_source(source, report_source, path, index + 1)
        })
        .collect::<Vec<_>>();
    let diagnostics = source_manifest_diagnostics(&sources);
    Ok(NormalizedSourceManifest {
        sources,
        evidence,
        diagnostics,
    })
}

pub fn normalize_report_sources(sources: &[Source]) -> Vec<ReportSource> {
    let keys = sources.iter().map(source_content_key).collect::<Vec<_>>();
    let id_map = content_id_map("src", keys.iter().map(String::as_str));
    sources
        .iter()
        .zip(keys.iter())
        .map(|(source, key)| {
            let normalized = normalize_id_text(key);
            let id = id_map
                .get(&normalized)
                .cloned()
                .unwrap_or_else(|| content_id("src", &[key]));
            normalize_report_source_with_id(source, id)
        })
        .collect()
}

pub fn normalize_report_source(source: &Source) -> ReportSource {
    normalize_report_source_with_id(source, source_report_id(source))
}

pub fn source_report_id(source: &Source) -> String {
    content_id("src", &[&source_content_key(source)])
}

pub fn normalize_report_source_with_id(source: &Source, id: String) -> ReportSource {
    let access = SourceAccessMetadata::from_source(source);
    let citation = first_non_empty([
        source.citation.as_str(),
        source.title.as_str(),
        source.identifier.as_str(),
        source.url.as_str(),
        "Untitled source",
    ]);
    let why_it_matters = first_non_empty([
        source.why_it_matters.as_str(),
        source.use_in_curriculum.as_str(),
        source.notes.as_str(),
        "Cataloged source; source role or relevance not yet specified.",
    ]);
    ReportSource {
        id,
        citation,
        title: source.title.trim().to_string(),
        source_type: first_non_empty([source.source_type.as_str(), "unknown"]),
        identifier: source.identifier.trim().to_string(),
        url: source.url.trim().to_string(),
        date: normalize_report_date(&source.date),
        roles: normalize_source_roles(&source.layer, &source.source_type),
        access,
        why_it_matters,
        verification_status: VerificationStatus::Cataloged,
        notes: source.notes.trim().to_string(),
        ..ReportSource::default()
    }
}

impl SourceAccessMetadata {
    pub fn from_source(source: &Source) -> Self {
        let access = source_access(source);
        let status = AccessStatus::from_manifest_status(&access.status);
        let route = first_non_empty([
            access.route.as_str(),
            source.access_route.as_str(),
            source.url.as_str(),
            "unknown",
        ]);
        Self {
            status,
            route,
            budget_estimate: first_non_empty([
                access.budget_estimate.as_str(),
                source.budget_estimate.as_str(),
            ]),
            license: source.license.trim().to_string(),
            metadata_only: Some(status.metadata_only_default()),
            notes: first_non_empty([access.notes.as_str(), source.notes.as_str()]),
        }
    }
}

pub fn normalize_source_roles(layer: &str, source_type: &str) -> Vec<SourceRole> {
    let haystack = format!(
        " {} {} ",
        normalize_access_status_label(layer),
        normalize_access_status_label(source_type)
    );
    let mut roles = BTreeSet::new();
    for (needle, role) in [
        ("orientation", SourceRole::Orientation),
        ("foundation", SourceRole::Foundation),
        ("method", SourceRole::Method),
        ("representation", SourceRole::Representation),
        ("evidence", SourceRole::Evidence),
        ("synthesis", SourceRole::Synthesis),
        ("survey", SourceRole::Synthesis),
        ("frontier", SourceRole::Frontier),
        ("debate", SourceRole::Debate),
        ("standard", SourceRole::Standard),
        ("dataset", SourceRole::Dataset),
        ("data", SourceRole::Dataset),
        ("infrastructure", SourceRole::Infrastructure),
        ("critique", SourceRole::Critique),
        ("curriculum", SourceRole::Curriculum),
        ("syllabus", SourceRole::Curriculum),
    ] {
        if haystack.contains(needle) {
            roles.insert(role);
        }
    }
    if roles.is_empty() {
        roles.insert(SourceRole::Other);
    }
    roles.into_iter().collect()
}

pub fn source_curricular_use(source: &Source) -> String {
    first_non_empty([
        source.use_in_curriculum.as_str(),
        source.why_it_matters.as_str(),
        source.layer.as_str(),
    ])
}

pub fn normalize_report_date(raw: &str) -> String {
    let value = raw.trim();
    let lowered = value.to_ascii_lowercase();
    if value.is_empty()
        || matches!(
            lowered.as_str(),
            "unknown" | "n/a" | "na" | "date after lookup" | "to verify"
        )
    {
        String::new()
    } else {
        value.to_string()
    }
}

fn review_after_date(as_of: &str) -> String {
    NaiveDate::parse_from_str(as_of, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.checked_add_signed(Duration::days(183)))
        .map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

pub fn source_manifest_diagnostics(sources: &[ReportSource]) -> Vec<DiagnosticCheck> {
    let mut diagnostics = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let target_path = format!("/sources/{index}");
        if source.access.status == AccessStatus::Unknown {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_SOURCE_MISSING_ACCESS_STATUS,
                    format!("source {} has unknown access status", source.id),
                )
                .with_target(&target_path, &source.id),
            );
        }
        if source.access.route.trim().is_empty() || source.access.route == "unknown" {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_SOURCE_MISSING_ACCESS_ROUTE,
                    format!("source {} has no actionable access route", source.id),
                )
                .with_target(&target_path, &source.id),
            );
        }
        if source
            .why_it_matters
            .starts_with("Cataloged source; source role or relevance")
        {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_SOURCE_MISSING_CURRICULAR_USE,
                    format!("source {} has no curricular use note", source.id),
                )
                .with_target(&target_path, &source.id),
            );
        }
    }
    diagnostics
}

pub fn content_id(prefix: &str, parts: &[&str]) -> String {
    let text = normalize_id_text(&parts.join("\n"));
    let text = if text.is_empty() {
        "item".to_string()
    } else {
        text
    };
    let prefix = id_prefix(prefix);
    let slug = slug_from_normalized(&text);
    let hash = short_hash(&text, ID_HASH_LEN);
    format!("{prefix}-{slug}-{hash}")
}

pub fn content_id_map<'a, I>(prefix: &str, inputs: I) -> BTreeMap<String, String>
where
    I: IntoIterator<Item = &'a str>,
{
    let prefix = id_prefix(prefix);
    let mut by_text = BTreeMap::new();
    for input in inputs {
        let mut normalized = normalize_id_text(input);
        if normalized.is_empty() {
            normalized = "item".to_string();
        }
        by_text
            .entry(normalized.clone())
            .or_insert_with(|| (slug_from_normalized(&normalized), full_hash(&normalized)));
    }

    let mut groups: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
    for (normalized, (slug, hash)) in by_text {
        let candidate = format!("{prefix}-{slug}-{}", &hash[..ID_HASH_LEN]);
        groups
            .entry(candidate)
            .or_default()
            .push((normalized, slug, hash));
    }

    let mut ids = BTreeMap::new();
    for (candidate, mut group) in groups {
        if group.len() == 1 {
            let (normalized, _, _) = group.remove(0);
            ids.insert(normalized, candidate);
            continue;
        }
        group.sort();
        for (normalized, slug, hash) in &group {
            let unique_len = shortest_unique_hash_prefix(hash, &group);
            ids.insert(
                normalized.clone(),
                format!("{prefix}-{slug}-{}", &hash[..unique_len]),
            );
        }
    }
    ids
}

pub fn normalize_id_text(raw: &str) -> String {
    let mut output = String::new();
    let mut previous_space = true;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
            previous_space = false;
        } else if !previous_space {
            output.push(' ');
            previous_space = true;
        }
    }
    output.trim().to_string()
}

pub fn render_html_report_file<I, O>(input: I, output: O) -> Result<()>
where
    I: AsRef<Path>,
    O: AsRef<Path>,
{
    let input = input.as_ref();
    let output = output.as_ref();
    let validation = validate_report_file(input)?;
    if validation.has_errors() {
        bail!(
            "cannot render HTML from {}: report validation has {} error(s); run sok validate-report --input {}",
            input.display(),
            validation.error_count(),
            input.display()
        );
    }

    let document: ReportDocument = read_json_file(input)?;
    let html = render_html_report(&document)?;
    ensure_parent_dir(output)?;
    fs::write(output, html).with_context(|| format!("write HTML {}", output.display()))?;
    Ok(())
}

pub fn render_html_report(document: &ReportDocument) -> Result<String> {
    if document.metadata.report_type != ReportType::HumanReport {
        bail!("sok render-html requires metadata.report_type to be human_report");
    }

    let report = &document.report;
    let views = renderable_visual_views(report);
    let visual_json = if views.is_empty() {
        None
    } else {
        Some(safe_script_json(&views)?)
    };
    let source_labels = source_label_lookup(report);
    let entity_labels = entity_label_lookup(report);
    let title = format!("Structure of Knowledge: {}", report.field);
    let mut html = String::new();

    html.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("  <meta charset=\"utf-8\">\n");
    html.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str("  <title>");
    push_escaped(&mut html, &title);
    html.push_str("</title>\n");
    html.push_str("  <style>\n");
    html.push_str(REPORT_CSS);
    html.push_str("\n  </style>\n</head>\n<body>\n");
    html.push_str("<a class=\"skip-link\" href=\"#main-report\">Skip to report content</a>\n");
    html.push_str("<div class=\"report-shell\">\n<header class=\"report-header\">\n");
    html.push_str("<p class=\"eyebrow\">Structure of Knowledge Report</p>\n<h1>");
    push_escaped(&mut html, &report.field);
    html.push_str("</h1>\n");
    let presentation = report
        .presentation
        .as_ref()
        .filter(|presentation| !presentation.sections.is_empty());
    let header_summary = presentation
        .map(|presentation| presentation.thesis.as_str())
        .filter(|thesis| !thesis.trim().is_empty())
        .unwrap_or(report.scope.summary.as_str());
    if !header_summary.trim().is_empty() {
        html.push_str("<p class=\"summary\">");
        push_escaped(&mut html, header_summary);
        html.push_str("</p>\n");
    }
    if let Some(presentation) = presentation {
        push_presentation_nav(&mut html, presentation);
    } else {
        push_nav(&mut html, report, !views.is_empty());
    }
    html.push_str("</header>\n<main id=\"main-report\">\n");

    if let Some(presentation) = presentation {
        push_presentation_report(
            &mut html,
            presentation,
            &views,
            report,
            &entity_labels,
            &source_labels,
        );
    } else {
        push_reading_guide_section(&mut html, document, !views.is_empty());
        if !views.is_empty() {
            push_visual_section(&mut html, &views);
        }
        push_curriculum_section(
            &mut html,
            &report.curriculum_path,
            &entity_labels,
            &source_labels,
        );
        push_reading_ladder_section(&mut html, report, &source_labels);
        push_claims_section(&mut html, &report.claims, &source_labels);
        push_frontier_section(&mut html, report, &entity_labels, &source_labels);

        push_scope_section(&mut html, &report.scope);
        push_domain_profile_section(&mut html, &report.domain_profile);
        push_field_elements_or_legacy(&mut html, report, &source_labels);
        push_evidence_standards_section(&mut html, &report.evidence_standards);
        push_sources_and_evidence_section(&mut html, report);
        push_relations_section(&mut html, &report.relations, &entity_labels, &source_labels);
    }

    html.push_str("</main>\n</div>\n");
    if let Some(visual_json) = visual_json {
        html.push_str("<script type=\"application/json\" id=\"sok-visual-data\">");
        html.push_str(&visual_json);
        html.push_str("</script>\n<script>\n");
        html.push_str(REPORT_JS);
        html.push_str("\n</script>\n");
    }
    html.push_str("</body>\n</html>\n");
    Ok(html)
}

const REPORT_CSS: &str = r#":root {
  color-scheme: light dark;
  --bg: #f6f7f8;
  --paper: #ffffff;
  --panel: #fbfcfd;
  --ink: #1b252f;
  --muted: #5d6b78;
  --line: #d8dee4;
  --accent: #2364aa;
  --accent-soft: #e8f1fb;
  --success: #1f7a4d;
  --warn: #8a5a00;
  --danger: #a33a3a;
  --focus: #9b5de5;
  --shadow: 0 1px 2px rgba(24, 34, 45, 0.08);
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; }
body {
  margin: 0;
  background: var(--bg);
  color: var(--ink);
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 16px;
  line-height: 1.55;
}
.skip-link {
  position: absolute;
  left: 1rem;
  top: -4rem;
  z-index: 10;
  background: var(--paper);
  border: 2px solid var(--focus);
  color: var(--ink);
  padding: 0.5rem 0.75rem;
}
.skip-link:focus { top: 1rem; }
.report-shell {
  width: min(1120px, calc(100% - 32px));
  margin: 0 auto;
  padding: 28px 0 56px;
}
.report-header {
  padding: 28px 0 18px;
  border-bottom: 1px solid var(--line);
}
.eyebrow {
  margin: 0 0 0.45rem;
  color: var(--accent);
  font-size: 0.82rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0;
}
h1, h2, h3, h4 {
  color: var(--ink);
  line-height: 1.2;
  letter-spacing: 0;
}
h1 {
  margin: 0;
  font-size: clamp(2rem, 5vw, 3.7rem);
}
h2 {
  margin: 0 0 0.9rem;
  font-size: 1.55rem;
}
h3 {
  margin: 0 0 0.55rem;
  font-size: 1.08rem;
}
h4 {
  margin: 0.8rem 0 0.4rem;
  font-size: 0.95rem;
}
.summary {
  max-width: 860px;
  margin: 0.9rem 0 0;
  color: var(--muted);
  font-size: 1.08rem;
}
.section-nav {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  margin-top: 1.35rem;
}
.section-nav a {
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--paper);
  color: var(--ink);
  padding: 0.35rem 0.55rem;
  font-size: 0.9rem;
  text-decoration: none;
}
.section-nav a:focus,
.section-nav a:hover {
  border-color: var(--accent);
  outline: 2px solid transparent;
}
.report-section {
  padding: 28px 0;
  border-bottom: 1px solid var(--line);
}
.architecture-note {
  display: grid;
  grid-template-columns: minmax(180px, 0.32fr) minmax(0, 1fr);
  gap: 14px;
  margin: 24px 0 0;
  padding: 14px 16px;
  border-left: 4px solid var(--accent);
  background: var(--accent-soft);
}
.architecture-note p { margin: 0; }
.architecture-note span {
  display: block;
  margin-bottom: 0.2rem;
  color: var(--accent);
  font-size: 0.78rem;
  font-weight: 700;
  text-transform: uppercase;
}
.section-purpose {
  max-width: 820px;
  margin: -0.35rem 0 1rem;
  color: var(--muted);
  font-size: 0.94rem;
}
.narrative-body {
  max-width: 880px;
  min-width: 0;
  overflow-x: auto;
  font-family: ui-serif, Georgia, Cambria, "Times New Roman", serif;
  font-size: 1.04rem;
}
.narrative-body h3,
.narrative-body h4 { margin-top: 1.4rem; }
.narrative-body p,
.narrative-body ul,
.narrative-body ol,
.narrative-body blockquote { max-width: 78ch; }
.narrative-body blockquote {
  margin-left: 0;
  padding-left: 1rem;
  border-left: 3px solid var(--line);
  color: var(--muted);
}
.structured-appendix {
  margin-top: 36px;
  padding: 16px 18px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel);
}
.structured-appendix > summary {
  cursor: pointer;
  color: var(--ink);
  font-weight: 700;
}
.prose {
  max-width: 860px;
  margin: 0.35rem 0 0;
}
.muted { color: var(--muted); }
.guide-list,
.ladder-list,
.curriculum-path {
  display: grid;
  gap: 12px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.guide-list li {
  display: grid;
  grid-template-columns: minmax(170px, 0.34fr) minmax(0, 1fr);
  gap: 12px;
  padding: 12px 14px;
  background: var(--paper);
  border: 1px solid var(--line);
  border-radius: 8px;
  box-shadow: var(--shadow);
}
.guide-list span { color: var(--muted); }
.split-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 18px;
}
.item-grid,
.claim-grid,
.source-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}
.item-card,
.visual-card,
.claim-card,
.path-card,
.ladder-card,
.source-card,
.frontier-card,
.relation-card {
  background: var(--paper);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 16px;
  box-shadow: var(--shadow);
}
.item-card p,
.visual-card p,
.claim-card p,
.path-card p,
.ladder-card p,
.source-card p,
.frontier-card p,
.relation-card p {
  margin: 0.35rem 0 0;
}
.badge-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  margin: 0.55rem 0 0.75rem;
}
.badge {
  display: inline-flex;
  align-items: center;
  max-width: 100%;
  min-height: 1.65rem;
  padding: 0.15rem 0.45rem;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent);
  font-size: 0.78rem;
  font-weight: 700;
  overflow-wrap: anywhere;
}
.meta {
  margin-top: 0.7rem;
  color: var(--muted);
  font-size: 0.92rem;
}
.curriculum-path {
  position: relative;
  gap: 16px;
}
.path-step {
  display: grid;
  grid-template-columns: 44px minmax(0, 1fr);
  gap: 12px;
  align-items: start;
}
.path-marker {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  border: 2px solid var(--accent);
  border-radius: 50%;
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 800;
}
.reader-dl,
.compact-dl {
  display: grid;
  grid-template-columns: minmax(120px, 0.24fr) minmax(0, 1fr);
  gap: 0.35rem 0.8rem;
  margin: 0.65rem 0 0;
}
.reader-dl dt,
.compact-dl dt {
  color: var(--muted);
  font-weight: 700;
}
.reader-dl dd,
.compact-dl dd {
  margin: 0;
  overflow-wrap: anywhere;
}
.reference-block { margin-top: 0.75rem; }
.reference-list,
.evidence-list {
  margin: 0.35rem 0 0 1.1rem;
  padding: 0;
}
.reference-list li,
.evidence-list li {
  margin: 0.35rem 0;
}
.evidence-group {
  margin-top: 0.85rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--line);
}
.evidence-meta {
  display: block;
  margin-top: 0.15rem;
  color: var(--muted);
  font-size: 0.88rem;
}
.text-fallback,
.visual-fallback {
  margin-top: 0.9rem;
  padding: 0.85rem;
  border-left: 4px solid var(--accent);
  background: var(--panel);
}
.raw-identifiers {
  margin-top: 0.85rem;
  color: var(--muted);
}
.raw-identifiers summary {
  cursor: pointer;
  color: var(--ink);
  font-weight: 700;
}
.frontier-list,
.relation-list {
  display: grid;
  gap: 14px;
}
.table-scroll {
  width: 100%;
  max-width: 100%;
  overflow-x: auto;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--paper);
}
table {
  width: 100%;
  min-width: 760px;
  border-collapse: collapse;
}
th,
td {
  padding: 0.7rem 0.75rem;
  border-bottom: 1px solid var(--line);
  text-align: left;
  vertical-align: top;
}
th {
  background: #eef3f7;
  font-size: 0.82rem;
  text-transform: uppercase;
  letter-spacing: 0;
}
td {
  font-size: 0.94rem;
  white-space: pre-wrap;
}
tr:last-child td { border-bottom: 0; }
.visual-canvas {
  min-height: 380px;
  margin-top: 0.9rem;
}
.visual-svg {
  display: block;
  width: 100%;
  height: 360px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fbfcfd;
}
.visual-node circle,
.visual-node rect {
  fill: var(--accent-soft);
  stroke: var(--accent);
  stroke-width: 2;
}
.visual-node text {
  fill: var(--ink);
  font-size: 13px;
  pointer-events: none;
}
.visual-edge {
  stroke: #6f7f8f;
  stroke-width: 2;
}
.visual-node.is-active circle,
.visual-node.is-active rect {
  fill: #fff4cc;
  stroke: #a15c00;
}
.visual-edge.is-active {
  stroke: #a15c00;
  stroke-width: 3;
}
.visual-fallback ul {
  margin: 0.4rem 0 0.8rem 1.1rem;
  padding: 0;
}
.visual-fallback li { margin: 0.2rem 0; }
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #11161c;
    --paper: #18212a;
    --panel: #141c24;
    --ink: #edf2f7;
    --muted: #aeb9c5;
    --line: #344250;
    --accent: #8fc5ff;
    --accent-soft: #17324f;
    --success: #7bd6a3;
    --warn: #f0c36a;
    --danger: #ff9b9b;
    --focus: #d2a8ff;
    --shadow: none;
  }
  th,
  .visual-svg {
    background: var(--panel);
  }
  .visual-edge {
    stroke: #9aa8b6;
  }
  .visual-node.is-active rect {
    fill: #3b2e17;
    stroke: var(--warn);
  }
  .visual-edge.is-active {
    stroke: var(--warn);
  }
}
@media (max-width: 720px) {
  .report-shell {
    width: min(100% - 20px, 1120px);
    padding-top: 16px;
  }
  .report-header { padding-top: 20px; }
  .split-list,
  .item-grid,
  .claim-grid,
  .source-grid {
    grid-template-columns: 1fr;
  }
  .guide-list li,
  .path-step,
  .architecture-note,
  .reader-dl,
  .compact-dl {
    grid-template-columns: 1fr;
  }
  table { min-width: 640px; }
  .visual-canvas { min-height: 320px; }
  .visual-svg { height: 300px; }
}
@media print {
  body {
    background: #ffffff;
    color: #000000;
    font-size: 11pt;
  }
  .report-shell {
    width: 100%;
    padding: 0;
  }
  .section-nav,
  .skip-link,
  .visual-canvas,
  script {
    display: none !important;
  }
  .report-header,
  .report-section {
    border-color: #bbbbbb;
  }
  .item-card,
  .visual-card,
  .claim-card,
  .path-card,
  .ladder-card,
  .source-card,
  .frontier-card,
  .relation-card,
  .table-scroll {
    border-color: #bbbbbb;
    break-inside: avoid;
  }
  .table-scroll {
    overflow: visible;
  }
  details.structured-appendix > summary {
    display: none;
  }
  details.structured-appendix:not([open]) > :not(summary) {
    display: block !important;
  }
  table {
    min-width: 0;
    font-size: 9pt;
  }
  a {
    color: inherit;
    text-decoration: none;
  }
}"#;

const REPORT_JS: &str = r##"(function () {
  var dataNode = document.getElementById("sok-visual-data");
  if (!dataNode) return;
  var views;
  try {
    views = JSON.parse(dataNode.textContent || "[]");
  } catch (error) {
    return;
  }
  if (!Array.isArray(views) || views.length === 0) return;

  var svgNs = "http://www.w3.org/2000/svg";
  function el(name, attrs) {
    var node = document.createElementNS(svgNs, name);
    Object.keys(attrs || {}).forEach(function (key) {
      node.setAttribute(key, attrs[key]);
    });
    return node;
  }
  function textNode(value) {
    return document.createTextNode(value == null ? "" : String(value));
  }
  function labelLines(node) {
    if (Array.isArray(node.label_lines) && node.label_lines.length > 0) {
      return node.label_lines.slice(0, 2);
    }
    var label = String(node.label || node.id || "");
    return [label.length > 20 ? label.slice(0, 19) + "\u2026" : label];
  }
  function layout(view) {
    var nodes = view.nodes || [];
    if (view.kind === "knowledge_spine" || view.kind === "dependency_path") {
      var ranked = {};
      var hasCompleteLayout = nodes.length > 0;
      nodes.forEach(function (node) {
        if (!node.layout || typeof node.layout.x !== "number" || typeof node.layout.y !== "number") {
          hasCompleteLayout = false;
          return;
        }
        ranked[node.id] = { x: node.layout.x, y: node.layout.y };
      });
      if (hasCompleteLayout) return ranked;
    }
    if (view.kind === "concept_source") {
      var left = nodes.filter(function (node) { return node.entity_type === "source"; });
      var right = nodes.filter(function (node) { return node.entity_type !== "source"; });
      var positions = {};
      left.forEach(function (node, index) {
        positions[node.id] = { x: 190, y: 85 + index * (210 / Math.max(1, left.length - 1)) };
      });
      right.forEach(function (node, index) {
        positions[node.id] = { x: 720, y: 85 + index * (210 / Math.max(1, right.length - 1)) };
      });
      return positions;
    }
    if (view.kind === "frontier_debate") {
      var cx = 480;
      var cy = 180;
      var radius = 125;
      var out = {};
      nodes.forEach(function (node, index) {
        if (index === 0) {
          out[node.id] = { x: cx, y: cy };
        } else {
          var angle = -Math.PI / 2 + (2 * Math.PI * (index - 1)) / Math.max(1, nodes.length - 1);
          out[node.id] = { x: cx + Math.cos(angle) * radius, y: cy + Math.sin(angle) * radius };
        }
      });
      return out;
    }
    var step = 720 / Math.max(1, nodes.length - 1);
    var base = {};
    nodes.forEach(function (node, index) {
      base[node.id] = { x: 120 + index * step, y: 180 + (index % 2 === 0 ? -38 : 38) };
    });
    return base;
  }
  function connectedIds(view, nodeId) {
    var ids = {};
    (view.edges || []).forEach(function (edge) {
      if (edge.from === nodeId) ids[edge.to] = true;
      if (edge.to === nodeId) ids[edge.from] = true;
    });
    ids[nodeId] = true;
    return ids;
  }
  function render(mount, view) {
    var positions = layout(view);
    var markerId = "arrowhead-" + view.id;
    var svg = el("svg", {
      "class": "visual-svg",
      "role": "img",
      "aria-labelledby": "svg-title-" + view.id + " svg-desc-" + view.id,
      "viewBox": "0 0 960 360",
      "preserveAspectRatio": "xMidYMid meet"
    });
    var title = el("title", { "id": "svg-title-" + view.id });
    title.appendChild(textNode(view.title || "Visual view"));
    svg.appendChild(title);
    var desc = el("desc", { "id": "svg-desc-" + view.id });
    desc.appendChild(textNode(view.alt || view.justification || ""));
    svg.appendChild(desc);
    var defs = el("defs", {});
    var marker = el("marker", {
      "id": markerId,
      "viewBox": "0 0 10 10",
      "refX": "9",
      "refY": "5",
      "markerWidth": "7",
      "markerHeight": "7",
      "orient": "auto-start-reverse"
    });
    marker.appendChild(el("path", { "d": "M 0 0 L 10 5 L 0 10 z", "fill": "#6f7f8f" }));
    defs.appendChild(marker);
    svg.appendChild(defs);

    (view.edges || []).forEach(function (edge, index) {
      var from = positions[edge.from];
      var to = positions[edge.to];
      if (!from || !to) return;
      var line = el("line", {
        "class": "visual-edge",
        "tabindex": "0",
        "role": "img",
        "data-from": edge.from,
        "data-to": edge.to,
        "x1": from.x,
        "y1": from.y,
        "x2": to.x,
        "y2": to.y,
        "marker-end": "url(#" + markerId + ")",
        "aria-label": (edge.label || edge.kind || "edge") + ": " + (edge.from_label || edge.from) + " to " + (edge.to_label || edge.to)
      });
      line.setAttribute("data-edge-index", String(index));
      var edgeTitle = el("title", {});
      edgeTitle.appendChild(textNode((edge.label || edge.kind || "edge") + ": " + (edge.from_label || edge.from) + " to " + (edge.to_label || edge.to)));
      line.appendChild(edgeTitle);
      svg.appendChild(line);
    });

    (view.nodes || []).forEach(function (node) {
      var point = positions[node.id];
      if (!point) return;
      var group = el("g", {
        "class": "visual-node",
        "tabindex": "0",
        "role": "img",
        "aria-label": node.label + ". " + (node.description || node.entity_type || "node"),
        "data-node-id": node.id
      });
      group.appendChild(el("rect", {
        "x": point.x - 82,
        "y": point.y - 24,
        "width": "164",
        "height": "48",
        "rx": "7"
      }));
      var text = el("text", {
        "x": point.x,
        "y": point.y,
        "text-anchor": "middle"
      });
      var lines = labelLines(node);
      lines.forEach(function (line, index) {
        var tspan = el("tspan", {
          "x": point.x,
          "dy": index === 0 ? (lines.length === 1 ? "4" : "-4") : "15"
        });
        tspan.appendChild(textNode(line));
        text.appendChild(tspan);
      });
      group.appendChild(text);
      group.addEventListener("mouseenter", function () { activate(svg, view, node.id); });
      group.addEventListener("focus", function () { activate(svg, view, node.id); });
      group.addEventListener("mouseleave", function () { clearActive(svg); });
      group.addEventListener("blur", function () { clearActive(svg); });
      svg.appendChild(group);
    });
    mount.innerHTML = "";
    mount.appendChild(svg);
  }
  function activate(svg, view, nodeId) {
    var active = connectedIds(view, nodeId);
    Array.prototype.forEach.call(svg.querySelectorAll(".visual-node"), function (node) {
      node.classList.toggle("is-active", !!active[node.getAttribute("data-node-id")]);
    });
    Array.prototype.forEach.call(svg.querySelectorAll(".visual-edge"), function (edge) {
      var from = edge.getAttribute("data-from");
      var to = edge.getAttribute("data-to");
      edge.classList.toggle("is-active", from === nodeId || to === nodeId);
    });
  }
  function clearActive(svg) {
    Array.prototype.forEach.call(svg.querySelectorAll(".is-active"), function (node) {
      node.classList.remove("is-active");
    });
  }
  Array.prototype.forEach.call(document.querySelectorAll("[data-visual-mount]"), function (mount) {
    var id = mount.getAttribute("data-visual-mount");
    var view = views.find(function (item) { return item.id === id; });
    if (view) render(mount, view);
  });
})();"##;

#[derive(Debug, Serialize)]
struct RenderVisualView {
    id: String,
    kind: String,
    title: String,
    justification: String,
    alt: String,
    nodes: Vec<RenderVisualNode>,
    edges: Vec<RenderVisualEdge>,
}

#[derive(Debug, Serialize)]
struct RenderVisualNode {
    id: String,
    label: String,
    label_lines: Vec<String>,
    entity_type: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    layout: Option<RenderVisualNodeLayout>,
}

#[derive(Debug, Serialize)]
struct RenderVisualNodeLayout {
    rank: usize,
    lane: &'static str,
    order: usize,
    x: u16,
    y: u16,
}

#[derive(Debug, Serialize)]
struct RenderVisualEdge {
    from: String,
    to: String,
    kind: String,
    label: String,
    relation_id: String,
    from_label: String,
    to_label: String,
}

#[derive(Debug, Clone)]
struct VisualLayoutEdge {
    from: usize,
    to: usize,
    secondary: bool,
}

#[derive(Debug, Clone)]
struct VisualLayoutPath {
    nodes: Vec<usize>,
    secondary_edges: usize,
    auxiliary_nodes: usize,
}

fn renderable_visual_views(report: &PublicReport) -> Vec<RenderVisualView> {
    let relation_lookup = report
        .relations
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect::<BTreeMap<_, _>>();
    report
        .visual_views
        .iter()
        .filter_map(|view| {
            let kind = visual_view_kind_label(view.kind)?;
            if view.justification.trim().is_empty() {
                return None;
            }
            let mut node_ids = BTreeSet::new();
            let mut nodes = view
                .nodes
                .iter()
                .filter_map(|node| {
                    if node.id.trim().is_empty() {
                        return None;
                    }
                    node_ids.insert(node.id.clone());
                    let label = first_non_empty([node.label.as_str(), node.id.as_str()]);
                    Some(RenderVisualNode {
                        id: node.id.clone(),
                        label_lines: visual_node_label_lines(&label),
                        label,
                        entity_type: entity_type_label(node.entity_type).to_string(),
                        description: node.description.clone(),
                        layout: None,
                    })
                })
                .collect::<Vec<_>>();
            let node_labels = nodes
                .iter()
                .map(|node| (node.id.as_str(), node.label.as_str()))
                .collect::<BTreeMap<_, _>>();
            let node_refs = view
                .nodes
                .iter()
                .filter(|node| !node.id.trim().is_empty() && !node.ref_id.trim().is_empty())
                .map(|node| (node.id.as_str(), node))
                .collect::<BTreeMap<_, _>>();
            let edges = view
                .edges
                .iter()
                .filter_map(|edge| {
                    let relation = relation_lookup.get(edge.relation_id.as_str())?;
                    let from_node = node_refs.get(edge.from.as_str())?;
                    let to_node = node_refs.get(edge.to.as_str())?;
                    if node_ids.contains(&edge.from)
                        && node_ids.contains(&edge.to)
                        && edge.from != edge.to
                        && visual_relation_matches_nodes(relation, from_node, to_node)
                    {
                        Some(RenderVisualEdge {
                            from: edge.from.clone(),
                            to: edge.to.clone(),
                            kind: first_non_empty([
                                edge.kind.as_str(),
                                relation_kind_label(relation.kind),
                            ]),
                            label: first_non_empty([
                                edge.label.as_str(),
                                relation.description.as_str(),
                                relation_kind_label(relation.kind),
                            ]),
                            relation_id: edge.relation_id.clone(),
                            from_label: node_labels
                                .get(edge.from.as_str())
                                .copied()
                                .unwrap_or(edge.from.as_str())
                                .to_string(),
                            to_label: node_labels
                                .get(edge.to.as_str())
                                .copied()
                                .unwrap_or(edge.to.as_str())
                                .to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if nodes.len() < 2 || edges.is_empty() {
                return None;
            }
            if matches!(
                view.kind,
                VisualViewKind::KnowledgeSpine | VisualViewKind::DependencyPath
            ) {
                apply_directed_visual_layout(&mut nodes, &edges);
            }
            let title = first_non_empty([view.title.as_str(), kind]);
            Some(RenderVisualView {
                id: view.id.clone(),
                kind: kind.to_string(),
                title: title.clone(),
                justification: view.justification.clone(),
                alt: visual_alt_text(&title, kind, &nodes, &edges),
                nodes,
                edges,
            })
        })
        .collect()
}

fn visual_node_label_lines(label: &str) -> Vec<String> {
    const MAX_LINE_CHARS: usize = 20;

    fn char_len(value: &str) -> usize {
        value.chars().count()
    }

    fn ellipsize(value: &str) -> String {
        if char_len(value) <= MAX_LINE_CHARS {
            return value.to_string();
        }
        value
            .chars()
            .take(MAX_LINE_CHARS - 1)
            .chain(std::iter::once('…'))
            .collect()
    }

    let normalized = label.split_whitespace().collect::<Vec<_>>().join(" ");
    if char_len(&normalized) <= MAX_LINE_CHARS {
        return vec![normalized];
    }

    let words = normalized.split_whitespace().collect::<Vec<_>>();
    if words.len() == 1 {
        let first = normalized.chars().take(MAX_LINE_CHARS).collect::<String>();
        let rest = normalized.chars().skip(MAX_LINE_CHARS).collect::<String>();
        return vec![first, ellipsize(&rest)];
    }

    let split = (1..words.len())
        .min_by_key(|split| {
            let left_len = char_len(&words[..*split].join(" "));
            let right_len = char_len(&words[*split..].join(" "));
            (
                left_len.saturating_sub(MAX_LINE_CHARS) + right_len.saturating_sub(MAX_LINE_CHARS),
                left_len.max(right_len),
                left_len.abs_diff(right_len),
                *split,
            )
        })
        .unwrap_or(1);
    vec![
        ellipsize(&words[..split].join(" ")),
        ellipsize(&words[split..].join(" ")),
    ]
}

fn apply_directed_visual_layout(nodes: &mut [RenderVisualNode], edges: &[RenderVisualEdge]) {
    const MIN_X: i32 = 100;
    const MAX_X: i32 = 860;
    const PRIMARY_Y: u16 = 110;
    const SECONDARY_Y: u16 = 275;
    const SECONDARY_SPACING: i32 = 176;

    let node_index = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let mut layout_edges = edges
        .iter()
        .filter_map(|edge| {
            let from = node_index.get(edge.from.as_str()).copied()?;
            let to = node_index.get(edge.to.as_str()).copied()?;
            Some(VisualLayoutEdge {
                from,
                to,
                secondary: visual_layout_edge_is_secondary(edge, &nodes[from], &nodes[to]),
            })
        })
        .collect::<Vec<_>>();
    layout_edges.sort_by(|left, right| {
        (
            nodes[left.from].id.as_str(),
            nodes[left.to].id.as_str(),
            left.secondary,
        )
            .cmp(&(
                nodes[right.from].id.as_str(),
                nodes[right.to].id.as_str(),
                right.secondary,
            ))
    });
    if layout_edges.is_empty() {
        return;
    }

    let (topological_order, ranks) = directed_visual_order_and_ranks(nodes, &layout_edges);
    let topological_positions = topological_order
        .iter()
        .enumerate()
        .map(|(position, node)| (*node, position))
        .collect::<BTreeMap<_, _>>();
    let primary_eligible = nodes
        .iter()
        .map(|node| !visual_layout_node_is_auxiliary(node))
        .collect::<Vec<_>>();
    let mut primary_path = longest_directed_visual_path(
        nodes,
        &layout_edges,
        &topological_order,
        &topological_positions,
        &primary_eligible,
        true,
    )
    .unwrap_or_default();

    if primary_path.len() == 1 {
        primary_path = vec![
            most_connected_visual_node(nodes, &layout_edges, &primary_eligible)
                .unwrap_or(primary_path[0]),
        ];
    } else if primary_path.is_empty() {
        let all_nodes = vec![true; nodes.len()];
        primary_path = longest_directed_visual_path(
            nodes,
            &layout_edges,
            &topological_order,
            &topological_positions,
            &all_nodes,
            false,
        )
        .unwrap_or_default();
    }
    if primary_path.is_empty() {
        return;
    }

    let primary_set = primary_path.iter().copied().collect::<BTreeSet<_>>();
    let mut primary_x = BTreeMap::new();
    for (order, node_index) in primary_path.iter().copied().enumerate() {
        let x = if primary_path.len() == 1 {
            (MIN_X + MAX_X) / 2
        } else {
            MIN_X + ((MAX_X - MIN_X) * order as i32) / (primary_path.len() as i32 - 1)
        };
        primary_x.insert(node_index, x);
        nodes[node_index].layout = Some(RenderVisualNodeLayout {
            rank: ranks[node_index],
            lane: "primary",
            order,
            x: x as u16,
            y: PRIMARY_Y,
        });
    }

    let mut adjacency = vec![Vec::new(); nodes.len()];
    for edge in &layout_edges {
        adjacency[edge.from].push(edge.to);
        adjacency[edge.to].push(edge.from);
    }
    for neighbors in &mut adjacency {
        neighbors.sort_by(|left, right| nodes[*left].id.cmp(&nodes[*right].id));
        neighbors.dedup();
    }

    let mut groups = BTreeMap::<i32, Vec<usize>>::new();
    for node_index in 0..nodes.len() {
        if primary_set.contains(&node_index) {
            continue;
        }
        let anchor_x = closest_primary_anchor_x(node_index, &primary_x, &adjacency).unwrap_or(480);
        groups.entry(anchor_x).or_default().push(node_index);
    }

    let mut secondary = Vec::<(usize, i32)>::new();
    for (anchor_x, mut group) in groups {
        group.sort_by(|left, right| {
            (ranks[*left], nodes[*left].id.as_str())
                .cmp(&(ranks[*right], nodes[*right].id.as_str()))
        });
        let width = group.len() as i32 - 1;
        for (position, node_index) in group.into_iter().enumerate() {
            let offset = (position as i32 * 2 - width) * (SECONDARY_SPACING / 2);
            secondary.push((node_index, anchor_x + offset));
        }
    }
    secondary.sort_by(|(left_node, left_x), (right_node, right_x)| {
        (left_x, ranks[*left_node], nodes[*left_node].id.as_str()).cmp(&(
            right_x,
            ranks[*right_node],
            nodes[*right_node].id.as_str(),
        ))
    });

    let spacing = if secondary.len() <= 1 {
        0
    } else {
        SECONDARY_SPACING.min((MAX_X - MIN_X) / (secondary.len() as i32 - 1))
    };
    let mut secondary_x = secondary
        .iter()
        .map(|(_, desired_x)| (*desired_x).clamp(MIN_X, MAX_X))
        .collect::<Vec<_>>();
    for index in 1..secondary_x.len() {
        secondary_x[index] = secondary_x[index].max(secondary_x[index - 1] + spacing);
    }
    if secondary_x.last().copied().unwrap_or(MAX_X) > MAX_X {
        if let Some(last) = secondary_x.last_mut() {
            *last = MAX_X;
        }
        for index in (0..secondary_x.len().saturating_sub(1)).rev() {
            secondary_x[index] = secondary_x[index].min(secondary_x[index + 1] - spacing);
        }
    }
    if secondary_x.first().copied().unwrap_or(MIN_X) < MIN_X {
        if let Some(first) = secondary_x.first_mut() {
            *first = MIN_X;
        }
        for index in 1..secondary_x.len() {
            secondary_x[index] = secondary_x[index].max(secondary_x[index - 1] + spacing);
        }
    }

    for (order, ((node_index, _), x)) in secondary.into_iter().zip(secondary_x).enumerate() {
        nodes[node_index].layout = Some(RenderVisualNodeLayout {
            rank: ranks[node_index],
            lane: "secondary",
            order,
            x: x as u16,
            y: SECONDARY_Y,
        });
    }
}

fn visual_layout_node_is_auxiliary(node: &RenderVisualNode) -> bool {
    matches!(
        node.entity_type.as_str(),
        "source" | "claim" | "frontier_debate"
    )
}

fn visual_layout_edge_is_secondary(
    edge: &RenderVisualEdge,
    from: &RenderVisualNode,
    to: &RenderVisualNode,
) -> bool {
    let kind = edge.kind.trim().to_ascii_lowercase();
    visual_layout_node_is_auxiliary(from)
        || visual_layout_node_is_auxiliary(to)
        || kind.contains("qualif")
        || kind.contains("contradict")
}

fn directed_visual_order_and_ranks(
    nodes: &[RenderVisualNode],
    edges: &[VisualLayoutEdge],
) -> (Vec<usize>, Vec<usize>) {
    let mut outgoing = vec![Vec::new(); nodes.len()];
    let mut indegree = vec![0usize; nodes.len()];
    for edge in edges {
        outgoing[edge.from].push(edge.to);
        indegree[edge.to] += 1;
    }
    for neighbors in &mut outgoing {
        neighbors.sort_by(|left, right| nodes[*left].id.cmp(&nodes[*right].id));
    }

    let mut remaining = vec![true; nodes.len()];
    let mut ready = BTreeSet::<(String, usize)>::new();
    for (index, node) in nodes.iter().enumerate() {
        if indegree[index] == 0 {
            ready.insert((node.id.clone(), index));
        }
    }
    let mut order = Vec::with_capacity(nodes.len());
    let mut ranks = vec![0usize; nodes.len()];

    while order.len() < nodes.len() {
        let next = ready.iter().next().cloned().or_else(|| {
            nodes
                .iter()
                .enumerate()
                .filter(|(index, _)| remaining[*index])
                .min_by(|(_, left), (_, right)| left.id.cmp(&right.id))
                .map(|(index, node)| (node.id.clone(), index))
        });
        let Some((id, current)) = next else {
            break;
        };
        ready.remove(&(id, current));
        if !remaining[current] {
            continue;
        }
        remaining[current] = false;
        order.push(current);

        for &target in &outgoing[current] {
            if !remaining[target] {
                continue;
            }
            ranks[target] = ranks[target].max(ranks[current] + 1);
            indegree[target] = indegree[target].saturating_sub(1);
            if indegree[target] == 0 {
                ready.insert((nodes[target].id.clone(), target));
            }
        }
    }
    (order, ranks)
}

fn longest_directed_visual_path(
    nodes: &[RenderVisualNode],
    edges: &[VisualLayoutEdge],
    topological_order: &[usize],
    topological_positions: &BTreeMap<usize, usize>,
    eligible_nodes: &[bool],
    structural_only: bool,
) -> Option<Vec<usize>> {
    let mut best = vec![None::<VisualLayoutPath>; nodes.len()];
    for &current in topological_order {
        if !eligible_nodes[current] {
            continue;
        }
        let mut current_best = VisualLayoutPath {
            nodes: vec![current],
            secondary_edges: 0,
            auxiliary_nodes: usize::from(visual_layout_node_is_auxiliary(&nodes[current])),
        };
        for edge in edges.iter().filter(|edge| edge.to == current) {
            if structural_only && edge.secondary {
                continue;
            }
            if !eligible_nodes[edge.from]
                || topological_positions.get(&edge.from) >= topological_positions.get(&current)
            {
                continue;
            }
            let Some(prefix) = best[edge.from].as_ref() else {
                continue;
            };
            let mut candidate = prefix.clone();
            candidate.nodes.push(current);
            candidate.secondary_edges += usize::from(edge.secondary);
            candidate.auxiliary_nodes +=
                usize::from(visual_layout_node_is_auxiliary(&nodes[current]));
            if visual_layout_path_is_better(&candidate, &current_best, nodes) {
                current_best = candidate;
            }
        }
        best[current] = Some(current_best);
    }

    best.into_iter()
        .flatten()
        .reduce(|current, candidate| {
            if visual_layout_path_is_better(&candidate, &current, nodes) {
                candidate
            } else {
                current
            }
        })
        .map(|path| path.nodes)
}

fn visual_layout_path_is_better(
    candidate: &VisualLayoutPath,
    current: &VisualLayoutPath,
    nodes: &[RenderVisualNode],
) -> bool {
    if candidate.nodes.len() != current.nodes.len() {
        return candidate.nodes.len() > current.nodes.len();
    }
    if candidate.secondary_edges != current.secondary_edges {
        return candidate.secondary_edges < current.secondary_edges;
    }
    if candidate.auxiliary_nodes != current.auxiliary_nodes {
        return candidate.auxiliary_nodes < current.auxiliary_nodes;
    }
    candidate
        .nodes
        .iter()
        .map(|index| nodes[*index].id.as_str())
        .cmp(current.nodes.iter().map(|index| nodes[*index].id.as_str()))
        .is_lt()
}

fn most_connected_visual_node(
    nodes: &[RenderVisualNode],
    edges: &[VisualLayoutEdge],
    eligible_nodes: &[bool],
) -> Option<usize> {
    (0..nodes.len())
        .filter(|index| eligible_nodes[*index])
        .min_by(|left, right| {
            let left_degree = edges
                .iter()
                .filter(|edge| edge.from == *left || edge.to == *left)
                .count();
            let right_degree = edges
                .iter()
                .filter(|edge| edge.from == *right || edge.to == *right)
                .count();
            let left_indegree = edges.iter().filter(|edge| edge.to == *left).count();
            let right_indegree = edges.iter().filter(|edge| edge.to == *right).count();
            right_degree
                .cmp(&left_degree)
                .then_with(|| right_indegree.cmp(&left_indegree))
                .then_with(|| nodes[*left].id.cmp(&nodes[*right].id))
        })
}

fn closest_primary_anchor_x(
    start: usize,
    primary_x: &BTreeMap<usize, i32>,
    adjacency: &[Vec<usize>],
) -> Option<i32> {
    let mut queue = VecDeque::from([(start, 0usize)]);
    let mut visited = vec![false; adjacency.len()];
    visited[start] = true;
    let mut closest_distance = None;
    let mut anchors = Vec::new();

    while let Some((current, distance)) = queue.pop_front() {
        if closest_distance.is_some_and(|closest| distance > closest) {
            break;
        }
        if let Some(x) = primary_x.get(&current) {
            closest_distance = Some(distance);
            anchors.push(*x);
            continue;
        }
        for &neighbor in &adjacency[current] {
            if !visited[neighbor] {
                visited[neighbor] = true;
                queue.push_back((neighbor, distance + 1));
            }
        }
    }
    (!anchors.is_empty()).then(|| anchors.iter().sum::<i32>() / anchors.len() as i32)
}

fn visual_relation_matches_nodes(
    relation: &Relation,
    from_node: &VisualViewNode,
    to_node: &VisualViewNode,
) -> bool {
    (visual_endpoint_matches_node(&relation.from, from_node)
        && visual_endpoint_matches_node(&relation.to, to_node))
        || (visual_endpoint_matches_node(&relation.from, to_node)
            && visual_endpoint_matches_node(&relation.to, from_node))
}

fn visual_endpoint_matches_node(endpoint: &RelationEndpoint, node: &VisualViewNode) -> bool {
    endpoint.entity_type == node.entity_type && endpoint.id == node.ref_id
}

fn visual_alt_text(
    title: &str,
    kind: &str,
    nodes: &[RenderVisualNode],
    edges: &[RenderVisualEdge],
) -> String {
    let edge_summary = edges
        .iter()
        .map(|edge| {
            format!(
                "{} to {} ({})",
                edge.from_label,
                edge.to_label,
                first_non_empty([edge.label.as_str(), edge.kind.as_str(), "related"])
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "{title} is a {kind} view with {} nodes and {} edges: {edge_summary}",
        nodes.len(),
        edges.len()
    )
}

fn push_presentation_nav(html: &mut String, presentation: &ReportPresentation) {
    html.push_str("<nav class=\"section-nav\" aria-label=\"Report sections\">\n");
    for section in &presentation.sections {
        html.push_str("<a href=\"#");
        push_escaped_attr(html, &section.id);
        html.push_str("\">");
        push_escaped(html, &section.title);
        html.push_str("</a>\n");
    }
    html.push_str("<a href=\"#evidence-appendix\">Evidence trail</a>\n</nav>\n");
}

fn push_presentation_report(
    html: &mut String,
    presentation: &ReportPresentation,
    views: &[RenderVisualView],
    report: &PublicReport,
    entity_labels: &BTreeMap<String, String>,
    source_labels: &BTreeMap<String, String>,
) {
    if !presentation.organizing_form.trim().is_empty() || !presentation.rationale.trim().is_empty()
    {
        html.push_str("<aside class=\"architecture-note\" aria-label=\"Report architecture\">\n");
        if !presentation.organizing_form.trim().is_empty() {
            html.push_str("<p><span>Organizing form</span>");
            push_escaped(html, &presentation.organizing_form);
            html.push_str("</p>\n");
        }
        if !presentation.rationale.trim().is_empty() {
            html.push_str("<p>");
            push_escaped(html, &presentation.rationale);
            html.push_str("</p>\n");
        }
        html.push_str("</aside>\n");
    }

    let views_by_id = views
        .iter()
        .map(|view| (view.id.as_str(), view))
        .collect::<BTreeMap<_, _>>();
    let assigned_view_ids = presentation
        .sections
        .iter()
        .flat_map(|section| section.visual_view_ids.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();

    for section in &presentation.sections {
        push_section_open(html, &section.id, &section.title);
        if !section.purpose.trim().is_empty() {
            html.push_str("<p class=\"section-purpose\">");
            push_escaped(html, &section.purpose);
            html.push_str("</p>\n");
        }
        push_safe_markdown(html, &section.body_markdown, &section.id);
        for view_id in &section.visual_view_ids {
            if let Some(view) = views_by_id.get(view_id.as_str()) {
                push_visual_card(html, view);
            }
        }
        html.push_str("</section>\n");
    }

    html.push_str("<details class=\"structured-appendix\" id=\"evidence-appendix\">\n");
    html.push_str("<summary>Evidence and structured data</summary>\n");
    html.push_str("<p class=\"muted\">Machine-verifiable support for the narrative above. It is kept separate so the evidence contract does not dictate the report's reading order.</p>\n");
    let unassigned_views = views
        .iter()
        .filter(|view| !assigned_view_ids.contains(view.id.as_str()))
        .collect::<Vec<_>>();
    if !unassigned_views.is_empty() {
        push_section_open(html, "unplaced-visualizations", "Additional Visual Views");
        for view in unassigned_views {
            push_visual_card(html, view);
        }
        html.push_str("</section>\n");
    }
    push_scope_section(html, &report.scope);
    push_domain_profile_section(html, &report.domain_profile);
    push_field_elements_or_legacy(html, report, source_labels);
    push_curriculum_section(html, &report.curriculum_path, entity_labels, source_labels);
    push_reading_ladder_section(html, report, source_labels);
    push_frontier_section(html, report, entity_labels, source_labels);
    push_evidence_standards_section(html, &report.evidence_standards);
    push_claims_section(html, &report.claims, source_labels);
    push_sources_and_evidence_section(html, report);
    push_relations_section(html, &report.relations, entity_labels, source_labels);
    html.push_str("</details>\n");
}

fn push_safe_markdown(html_output: &mut String, markdown: &str, footnote_namespace: &str) {
    if markdown.trim().is_empty() {
        return;
    }
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let events = Parser::new_ext(markdown, options).map(|event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(CowStr::from(raw.into_string())),
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: safe_markdown_destination(dest_url),
            title,
            id,
        }),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Image {
            link_type,
            dest_url: safe_markdown_image_destination(dest_url),
            title,
            id,
        }),
        Event::Start(Tag::FootnoteDefinition(label)) => Event::Start(Tag::FootnoteDefinition(
            namespaced_footnote_label(footnote_namespace, label),
        )),
        Event::FootnoteReference(label) => {
            Event::FootnoteReference(namespaced_footnote_label(footnote_namespace, label))
        }
        other => other,
    });
    html_output.push_str("<div class=\"narrative-body\">\n");
    html::push_html(html_output, events);
    html_output.push_str("</div>\n");
}

fn safe_markdown_destination(destination: CowStr<'_>) -> CowStr<'_> {
    let normalized = destination
        .trim()
        .chars()
        .filter(|character| !character.is_ascii_control() && !character.is_ascii_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    let first_path_delimiter = normalized.find(['/', '?', '#']).unwrap_or(normalized.len());
    let scheme = normalized
        .find(':')
        .filter(|colon| *colon < first_path_delimiter)
        .map(|colon| &normalized[..colon]);
    if scheme.is_none() || matches!(scheme, Some("http" | "https" | "mailto")) {
        destination
    } else {
        CowStr::Borrowed("#blocked-unsafe-link")
    }
}

fn safe_markdown_image_destination(destination: CowStr<'_>) -> CowStr<'_> {
    let trimmed = destination.trim();
    let Some((metadata, payload)) = trimmed.split_once(',') else {
        return CowStr::Borrowed("#blocked-external-image");
    };
    let metadata = metadata.to_ascii_lowercase();
    let safe_raster_type = matches!(
        metadata.as_str(),
        "data:image/png;base64"
            | "data:image/jpeg;base64"
            | "data:image/jpg;base64"
            | "data:image/gif;base64"
            | "data:image/webp;base64"
    );
    let safe_base64_payload = !payload.is_empty()
        && payload
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='));
    if safe_raster_type && safe_base64_payload {
        destination
    } else {
        CowStr::Borrowed("#blocked-external-image")
    }
}

fn namespaced_footnote_label<'a>(namespace: &str, label: CowStr<'a>) -> CowStr<'a> {
    CowStr::from(format!("{namespace}-{label}"))
}

fn push_nav(html: &mut String, report: &PublicReport, has_visuals: bool) {
    let mut items = vec![
        ("reading-guide", "Reading Guide"),
        ("curriculum-path", "Curriculum Path"),
        ("reading-ladder", "Reading Ladder"),
        ("claims", "Claim Evidence"),
        ("frontier-debates", "Frontier Guidance"),
        ("scope", "Scope"),
        ("domain-profile", "Domain Profile"),
        ("evidence-standards", "Evidence Standards"),
        ("sources-evidence", "Source Catalog"),
        ("relations", "Relation Audit"),
    ];
    let structure_index = items
        .iter()
        .position(|(id, _)| *id == "evidence-standards")
        .unwrap_or(items.len());
    if report.field_elements.is_empty() {
        for item in [
            ("representations", "Representations"),
            ("methods", "Methods"),
            ("core-ideas", "Core Ideas"),
        ] {
            items.insert(structure_index, item);
        }
    } else {
        items.insert(structure_index, ("field-elements", "Field Elements"));
    }
    if has_visuals {
        items.insert(1, ("visualizations", "Knowledge Map"));
    }
    html.push_str("<nav class=\"section-nav\" aria-label=\"Report sections\">\n");
    for (id, label) in items {
        html.push_str("<a href=\"#");
        push_escaped_attr(html, id);
        html.push_str("\">");
        push_escaped(html, label);
        html.push_str("</a>\n");
    }
    html.push_str("</nav>\n");
}

fn push_reading_guide_section(html: &mut String, document: &ReportDocument, has_visuals: bool) {
    let report = &document.report;
    push_section_open(html, "reading-guide", "Reading Guide");
    push_paragraph(html, &report.scope.summary);
    html.push_str("<ol class=\"guide-list\">\n");
    if has_visuals {
        push_guide_item(
            html,
            "Orient with the knowledge map",
            "Use the relation-backed visual view first; every drawn edge comes from a validated public relation.",
        );
    }
    push_guide_item(
        html,
        "Follow the curriculum path",
        "Read the steps in sequence and check prerequisites before moving to each new layer.",
    );
    push_guide_item(
        html,
        "Use the reading ladder",
        "Start from the layer guidance and read sources for the stated purpose, including the explicit do-not-infer caveats.",
    );
    push_guide_item(
        html,
        "Check claims against evidence",
        "Read each claim with its supporting, qualifying, and contradictory evidence before treating it as settled.",
    );
    if !report.frontier_debates.is_empty() {
        push_guide_item(
            html,
            "Handle frontier guidance last",
            "Treat current or debate-facing material as time-bound and review the stated background and source support.",
        );
    }
    html.push_str("</ol>\n");
    html.push_str("<p class=\"meta\"><strong>Report review:</strong> ");
    push_escaped(
        html,
        &format_temporal_inline(&document.metadata.temporal_review),
    );
    html.push_str("</p>\n</section>\n");
}

fn push_guide_item(html: &mut String, title: &str, body: &str) {
    html.push_str("<li><strong>");
    push_escaped(html, title);
    html.push_str("</strong><span>");
    push_escaped(html, body);
    html.push_str("</span></li>\n");
}

fn push_scope_section(html: &mut String, scope: &Scope) {
    push_section_open(html, "scope", "Scope");
    push_paragraph(html, &scope.summary);
    html.push_str("<div class=\"split-list\">\n");
    push_string_list(html, "Included", &scope.included);
    push_string_list(html, "Excluded", &scope.excluded);
    push_string_list(html, "Assumptions", &scope.assumptions);
    push_string_list(html, "Interpretive Notes", &scope.interpretive_notes);
    html.push_str("</div>\n</section>\n");
}

fn push_domain_profile_section(html: &mut String, profile: &DomainProfile) {
    push_section_open(html, "domain-profile", "Domain Profile");
    let mut classifications = vec![domain_classification_label(profile.classification).to_string()];
    classifications.extend(
        profile
            .secondary_characteristics
            .iter()
            .map(|classification| domain_classification_label(*classification).to_string()),
    );
    html.push_str("<p class=\"meta\"><strong>Classification:</strong> ");
    push_escaped(html, &classifications.join(", "));
    html.push_str("</p>\n");
    push_paragraph(html, &profile.rationale);
    push_string_list(html, "Failure Modes", &profile.failure_modes);
    html.push_str("</section>\n");
}

fn push_field_elements_or_legacy(
    html: &mut String,
    report: &PublicReport,
    source_labels: &BTreeMap<String, String>,
) {
    if report.field_elements.is_empty() {
        push_knowledge_section(
            html,
            "core-ideas",
            "Core Ideas",
            &report.core_ideas,
            source_labels,
        );
        push_knowledge_section(html, "methods", "Methods", &report.methods, source_labels);
        push_knowledge_section(
            html,
            "representations",
            "Representations",
            &report.representations,
            source_labels,
        );
    } else {
        push_field_elements_section(html, &report.field_elements, source_labels);
    }
}

fn push_field_elements_section(
    html: &mut String,
    elements: &[FieldElement],
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "field-elements", "Field Elements");
    html.push_str("<div class=\"item-grid\">\n");
    for element in elements {
        html.push_str("<article class=\"item-card\">\n<h3>");
        push_escaped(html, &element.label);
        html.push_str("</h3>\n<div class=\"badge-row\">");
        push_badge(html, &element.element_class);
        push_badge(html, &element.role);
        if let Some(confidence) = element.confidence {
            push_badge(html, claim_confidence_label(confidence));
        }
        html.push_str("</div>\n<dl class=\"compact-dl\">\n");
        push_dl_item(html, "Actual form", &element.actual_form);
        push_dl_item(
            html,
            "Load-bearing relations",
            &element.load_bearing_relations,
        );
        html.push_str("</dl>\n");
        push_source_reference_list(html, "Sources", &element.source_ids, source_labels);
        push_raw_details(
            html,
            "Raw field element identifier",
            &[
                ("Element ID", element.id.clone()),
                ("Source IDs", element.source_ids.join(", ")),
            ],
        );
        html.push_str("</article>\n");
    }
    html.push_str("</div>\n</section>\n");
}

fn push_knowledge_section(
    html: &mut String,
    id: &str,
    title: &str,
    items: &[KnowledgeItem],
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, id, title);
    if items.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"item-grid\">\n");
        for item in items {
            html.push_str("<article class=\"item-card\">\n<h3>");
            push_escaped(html, &item.label);
            html.push_str("</h3>\n");
            push_paragraph(html, &item.description);
            push_nested_string_list(html, "Aliases", &item.aliases);
            push_source_reference_list(html, "Sources", &item.source_ids, source_labels);
            push_raw_details(
                html,
                "Raw item identifiers",
                &[
                    ("Item ID", item.id.clone()),
                    ("Source IDs", item.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

fn push_evidence_standards_section(html: &mut String, standards: &EvidenceStandards) {
    push_section_open(html, "evidence-standards", "Evidence Standards");
    push_paragraph(html, &standards.summary);
    push_paragraph(html, &standards.claim_policy);
    let rows = standards
        .source_role_requirements
        .iter()
        .map(|requirement| {
            vec![
                source_role_label(requirement.role).to_string(),
                source_role_requirement_label(requirement.requirement).to_string(),
                requirement
                    .minimum_sources
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                requirement.rationale.clone(),
                requirement
                    .waiver
                    .as_ref()
                    .map(format_source_role_waiver)
                    .unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    push_table(
        html,
        &["Role", "Requirement", "Minimum", "Rationale", "Waiver"],
        &rows,
    );
    html.push_str("</section>\n");
}

fn push_sources_and_evidence_section(html: &mut String, report: &PublicReport) {
    push_section_open(html, "sources-evidence", "Source Catalog");
    if report.sources.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"source-grid\">\n");
        for source in &report.sources {
            html.push_str("<article class=\"source-card\">\n<h3>");
            push_escaped(html, &source_display_label(source));
            html.push_str("</h3>\n<div class=\"badge-row\">");
            push_badge(html, &source.source_type);
            push_badge(html, verification_status_label(source.verification_status));
            for role in &source.roles {
                push_badge(html, source_role_label(*role));
            }
            html.push_str("</div>\n");
            push_paragraph(html, &source.why_it_matters);
            html.push_str("<dl class=\"compact-dl\">\n");
            push_dl_item(html, "Roles", &format_source_roles(&source.roles));
            push_dl_item(html, "Access", &format_source_access(&source.access));
            push_dl_item(html, "Identifier", &source.identifier);
            push_dl_item(html, "Date", &source.date);
            push_dl_item(html, "Last reviewed", &source.last_reviewed);
            push_dl_item(html, "Notes", &source.notes);
            html.push_str("</dl>\n");
            push_raw_details(
                html,
                "Raw source fields",
                &[
                    ("Source ID", source.id.clone()),
                    ("Citation", source.citation.clone()),
                    ("URL", source.url.clone()),
                ],
            );
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

fn push_claims_section(
    html: &mut String,
    claims: &[Claim],
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "claims", "Claim Evidence Guide");
    if claims.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"claim-grid\">\n");
        for claim in claims {
            html.push_str("<article class=\"claim-card\">\n<h3>");
            push_escaped(html, &claim.statement);
            html.push_str("</h3>\n<div class=\"badge-row\">");
            push_badge(html, claim_type_label(claim.claim_type));
            push_badge(html, evidence_requirement_label(claim.evidence_requirement));
            push_badge(
                html,
                claim
                    .confidence
                    .map(claim_confidence_label)
                    .unwrap_or("unknown"),
            );
            push_badge(html, temporal_status_label(claim.temporal.temporal_status));
            html.push_str("</div>\n");
            html.push_str("<p class=\"meta\"><strong>Temporal status:</strong> ");
            push_escaped(html, &format_temporal_inline(&claim.temporal));
            html.push_str("</p>\n");
            push_evidence_group(
                html,
                "Supporting Evidence",
                &claim.evidence_links,
                source_labels,
                |kind| kind == SupportKind::Supports,
            );
            push_evidence_group(
                html,
                "Qualifying Evidence",
                &claim.evidence_links,
                source_labels,
                |kind| kind == SupportKind::Qualifies,
            );
            push_evidence_group(
                html,
                "Contradictory Evidence",
                &claim.evidence_links,
                source_labels,
                |kind| kind == SupportKind::Contradicts,
            );
            push_evidence_group(
                html,
                "Context Evidence",
                &claim.evidence_links,
                source_labels,
                |kind| matches!(kind, SupportKind::Background | SupportKind::Example),
            );
            push_paragraph(html, &claim.notes);
            push_claim_raw_details(html, claim);
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

fn push_curriculum_section(
    html: &mut String,
    steps: &[CurriculumStep],
    entity_labels: &BTreeMap<String, String>,
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "curriculum-path", "Curriculum Path");
    if steps.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<ol class=\"curriculum-path\" aria-label=\"Visual curriculum path\">\n");
        for step in steps {
            html.push_str(
                "<li class=\"path-step\">\n<div class=\"path-marker\" aria-hidden=\"true\">",
            );
            push_escaped(html, &step.sequence.to_string());
            html.push_str("</div>\n<article class=\"path-card\">\n<h3>");
            push_escaped(html, &step.title);
            html.push_str("</h3>\n");
            push_paragraph(html, &step.learning_goal);
            if step.prerequisite_ids.is_empty() {
                html.push_str(
                    "<p class=\"meta\"><strong>Prerequisites:</strong> Entry point</p>\n",
                );
            } else {
                html.push_str("<p class=\"meta\"><strong>Prerequisites:</strong> ");
                push_escaped(html, &labels_for_ids(&step.prerequisite_ids, entity_labels));
                html.push_str("</p>\n");
            }
            push_paragraph(html, &step.practice_artifact);
            push_nested_string_list(html, "Progress Criteria", &step.progress_criteria);
            push_source_reference_list(html, "Sources", &step.source_ids, source_labels);
            push_raw_details(
                html,
                "Raw curriculum identifiers",
                &[
                    ("Step ID", step.id.clone()),
                    ("Prerequisite IDs", step.prerequisite_ids.join(", ")),
                    ("Source IDs", step.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n</li>\n");
        }
        html.push_str("</ol>\n");
        html.push_str("<div class=\"text-fallback\" role=\"group\" aria-label=\"Text alternative for curriculum path\">\n<h3>Text Alternative</h3>\n<ol>\n");
        for step in steps {
            html.push_str("<li>");
            push_escaped(html, &step.title);
            if step.prerequisite_ids.is_empty() {
                html.push_str(": entry point.");
            } else {
                html.push_str(": follows ");
                push_escaped(html, &labels_for_ids(&step.prerequisite_ids, entity_labels));
                html.push('.');
            }
            html.push_str("</li>\n");
        }
        html.push_str("</ol>\n</div>\n");
    }
    html.push_str("</section>\n");
}

fn push_frontier_section(
    html: &mut String,
    report: &PublicReport,
    entity_labels: &BTreeMap<String, String>,
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "frontier-debates", "Frontier Guidance");
    if report.frontier_debates.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"frontier-list\">\n");
        for item in &report.frontier_debates {
            html.push_str("<article class=\"frontier-card\">\n<h3>");
            push_escaped(html, &item.title);
            html.push_str("</h3>\n<div class=\"badge-row\">");
            push_badge(html, frontier_debate_kind_label(item.kind));
            push_badge(html, temporal_status_label(item.temporal.temporal_status));
            html.push_str("</div>\n");
            push_paragraph(html, &item.summary);
            push_paragraph(html, &item.why_it_matters);
            push_entity_reference_list(
                html,
                "Required Background",
                &item.required_background_ids,
                entity_labels,
            );
            push_entity_reference_list(html, "Related Claims", &item.claim_ids, entity_labels);
            push_source_reference_list(html, "Sources", &item.source_ids, source_labels);
            html.push_str("<p class=\"meta\"><strong>Temporal status:</strong> ");
            push_escaped(html, &format_temporal_inline(&item.temporal));
            html.push_str("</p>\n");
            push_raw_details(
                html,
                "Raw frontier identifiers",
                &[
                    ("Frontier ID", item.id.clone()),
                    ("Background IDs", item.required_background_ids.join(", ")),
                    ("Claim IDs", item.claim_ids.join(", ")),
                    ("Source IDs", item.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

fn push_reading_ladder_section(
    html: &mut String,
    report: &PublicReport,
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "reading-ladder", "Reading Ladder");
    if report.literature_ladder.is_empty() {
        push_source_ladder(html, &report.sources);
    } else {
        html.push_str("<ol class=\"ladder-list\">\n");
        for row in &report.literature_ladder {
            html.push_str("<li class=\"ladder-step\">\n<article class=\"ladder-card\">\n<h3>");
            push_escaped(html, &row.layer);
            html.push_str("</h3>\n<dl class=\"reader-dl\">\n");
            push_dl_item(html, "Start point", &row.start_here);
            push_dl_item(html, "Read for", &row.read_for);
            push_dl_item(html, "Do not infer", &row.do_not_infer);
            push_dl_item(html, "Notes", &row.notes);
            html.push_str("</dl>\n");
            push_source_reference_list(html, "Sources", &row.source_ids, source_labels);
            push_raw_details(
                html,
                "Raw ladder identifiers",
                &[
                    ("Ladder row ID", row.id.clone()),
                    ("Source IDs", row.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n</li>\n");
        }
        html.push_str("</ol>\n");
    }
    html.push_str("</section>\n");
}

fn push_source_ladder(html: &mut String, sources: &[ReportSource]) {
    if sources.is_empty() {
        push_empty_note(html);
        return;
    }
    html.push_str("<p class=\"prose\">No explicit literature ladder is present; use the source metadata below as the public source ladder.</p>\n");
    html.push_str("<ol class=\"ladder-list\">\n");
    for source in sources {
        let start_point = source_display_label(source);
        html.push_str("<li class=\"ladder-step\">\n<article class=\"ladder-card\">\n<h3>");
        push_escaped(html, &start_point);
        html.push_str("</h3>\n<div class=\"badge-row\">");
        for role in &source.roles {
            push_badge(html, source_role_label(*role));
        }
        push_badge(html, verification_status_label(source.verification_status));
        html.push_str("</div>\n<dl class=\"reader-dl\">\n");
        push_dl_item(html, "Start point", &start_point);
        push_dl_item(html, "Read for", &source.why_it_matters);
        push_dl_item(html, "Do not infer", &source.notes);
        html.push_str("</dl>\n");
        push_raw_details(
            html,
            "Raw source identifiers",
            &[
                ("Source ID", source.id.clone()),
                ("Identifier", source.identifier.clone()),
                ("URL", source.url.clone()),
            ],
        );
        html.push_str("</article>\n</li>\n");
    }
    html.push_str("</ol>\n");
}

fn push_relations_section(
    html: &mut String,
    relations: &[Relation],
    entity_labels: &BTreeMap<String, String>,
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "relations", "Relation Audit");
    if relations.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"relation-list\">\n");
        for relation in relations {
            let from = label_for_id(&relation.from.id, entity_labels);
            let to = label_for_id(&relation.to.id, entity_labels);
            html.push_str("<article class=\"relation-card\">\n<h3>");
            push_escaped(html, &from);
            html.push_str(" -> ");
            push_escaped(html, &to);
            html.push_str("</h3>\n<div class=\"badge-row\">");
            push_badge(html, relation_kind_label(relation.kind));
            push_badge(html, entity_type_label(relation.from.entity_type));
            push_badge(html, entity_type_label(relation.to.entity_type));
            html.push_str("</div>\n");
            push_paragraph(html, &relation.description);
            push_source_reference_list(html, "Sources", &relation.source_ids, source_labels);
            push_raw_details(
                html,
                "Raw relation identifiers",
                &[
                    ("Relation ID", relation.id.clone()),
                    ("From", relation.from.id.clone()),
                    ("To", relation.to.id.clone()),
                    ("Source IDs", relation.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

fn push_visual_section(html: &mut String, views: &[RenderVisualView]) {
    push_section_open(html, "visualizations", "Knowledge Map");
    for view in views {
        push_visual_card(html, view);
    }
    html.push_str("</section>\n");
}

fn push_visual_card(html: &mut String, view: &RenderVisualView) {
    html.push_str("<article class=\"visual-card\" aria-labelledby=\"visual-title-");
    push_escaped_attr(html, &view.id);
    html.push_str("\">\n<h3 id=\"visual-title-");
    push_escaped_attr(html, &view.id);
    html.push_str("\">");
    push_escaped(html, &view.title);
    html.push_str("</h3>\n<p class=\"meta\">");
    push_escaped(html, &view.justification);
    html.push_str("</p>\n<div class=\"visual-canvas\" data-visual-mount=\"");
    push_escaped_attr(html, &view.id);
    html.push_str("\" role=\"region\" aria-label=\"Interactive visual view: ");
    push_escaped_attr(html, &view.title);
    html.push_str("\"></div>\n");
    html.push_str("<noscript><p class=\"muted\">The text alternative below contains the same graph nodes and edges.</p></noscript>\n");
    html.push_str(
        "<div class=\"visual-fallback\" role=\"group\" aria-label=\"Text alternative for ",
    );
    push_escaped_attr(html, &view.title);
    html.push_str("\">\n<p>");
    push_escaped(html, &view.alt);
    html.push_str("</p>\n<h4>Nodes</h4>\n<ul>\n");
    for node in &view.nodes {
        html.push_str("<li><strong>");
        push_escaped(html, &node.label);
        html.push_str("</strong> ");
        push_escaped(html, &format!("({})", node.entity_type));
        if !node.description.trim().is_empty() {
            html.push_str(": ");
            push_escaped(html, &node.description);
        }
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n<h4>Edges</h4>\n<ul>\n");
    for edge in &view.edges {
        html.push_str("<li>");
        push_escaped(
            html,
            &format!(
                "{} -> {} ({})",
                edge.from_label,
                edge.to_label,
                first_non_empty([edge.label.as_str(), edge.kind.as_str(), "related"])
            ),
        );
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n");
    push_raw_details(
        html,
        "Raw visual identifiers",
        &[
            ("Visual view ID", view.id.clone()),
            (
                "Relation IDs",
                view.edges
                    .iter()
                    .map(|edge| edge.relation_id.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        ],
    );
    html.push_str("</div>\n</article>\n");
}

fn push_section_open(html: &mut String, id: &str, title: &str) {
    html.push_str("<section class=\"report-section\" id=\"");
    push_escaped_attr(html, id);
    html.push_str("\">\n<h2>");
    push_escaped(html, title);
    html.push_str("</h2>\n");
}

fn push_paragraph(html: &mut String, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    html.push_str("<p class=\"prose\">");
    push_escaped(html, text);
    html.push_str("</p>\n");
}

fn push_string_list(html: &mut String, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    html.push_str("<div>\n<h3>");
    push_escaped(html, title);
    html.push_str("</h3>\n<ul>\n");
    for item in items {
        html.push_str("<li>");
        push_escaped(html, item);
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n</div>\n");
}

fn push_nested_string_list(html: &mut String, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    html.push_str("<div class=\"reference-block\">\n<h4>");
    push_escaped(html, title);
    html.push_str("</h4>\n<ul class=\"reference-list\">\n");
    for item in items {
        html.push_str("<li>");
        push_escaped(html, item);
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n</div>\n");
}

fn push_empty_note(html: &mut String) {
    html.push_str("<p class=\"muted\">No entries.</p>\n");
}

fn push_table(html: &mut String, headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        push_empty_note(html);
        return;
    }
    html.push_str("<div class=\"table-scroll\" role=\"region\" aria-label=\"Scrollable table\" tabindex=\"0\">\n<table>\n<thead>\n<tr>");
    for header in headers {
        html.push_str("<th scope=\"col\">");
        push_escaped(html, header);
        html.push_str("</th>");
    }
    html.push_str("</tr>\n</thead>\n<tbody>\n");
    for row in rows {
        html.push_str("<tr>");
        for index in 0..headers.len() {
            html.push_str("<td>");
            if let Some(cell) = row.get(index) {
                push_escaped(html, cell);
            }
            html.push_str("</td>");
        }
        html.push_str("</tr>\n");
    }
    html.push_str("</tbody>\n</table>\n</div>\n");
}

fn push_badge(html: &mut String, label: &str) {
    if label.trim().is_empty() {
        return;
    }
    html.push_str("<span class=\"badge\">");
    push_escaped(html, label);
    html.push_str("</span>");
}

fn push_dl_item(html: &mut String, term: &str, value: &str) {
    if value.trim().is_empty() {
        return;
    }
    html.push_str("<dt>");
    push_escaped(html, term);
    html.push_str("</dt><dd>");
    push_escaped(html, value);
    html.push_str("</dd>\n");
}

fn push_source_reference_list(
    html: &mut String,
    title: &str,
    source_ids: &[String],
    source_labels: &BTreeMap<String, String>,
) {
    if source_ids.is_empty() {
        return;
    }
    html.push_str("<div class=\"reference-block\">\n<h4>");
    push_escaped(html, title);
    html.push_str("</h4>\n<ul class=\"reference-list\">\n");
    for source_id in source_ids {
        html.push_str("<li>");
        push_escaped(html, &label_for_id(source_id, source_labels));
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n</div>\n");
}

fn push_entity_reference_list(
    html: &mut String,
    title: &str,
    ids: &[String],
    entity_labels: &BTreeMap<String, String>,
) {
    if ids.is_empty() {
        return;
    }
    html.push_str("<div class=\"reference-block\">\n<h4>");
    push_escaped(html, title);
    html.push_str("</h4>\n<ul class=\"reference-list\">\n");
    for id in ids {
        html.push_str("<li>");
        push_escaped(html, &label_for_id(id, entity_labels));
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n</div>\n");
}

fn push_evidence_group<F>(
    html: &mut String,
    title: &str,
    links: &[EvidenceLink],
    source_labels: &BTreeMap<String, String>,
    include: F,
) where
    F: Fn(SupportKind) -> bool,
{
    html.push_str("<section class=\"evidence-group\" aria-label=\"");
    push_escaped_attr(html, title);
    html.push_str("\">\n<h4>");
    push_escaped(html, title);
    html.push_str("</h4>\n");
    let matching = links
        .iter()
        .filter(|link| include(link.support_kind))
        .collect::<Vec<_>>();
    if matching.is_empty() {
        html.push_str("<p class=\"muted\">None recorded.</p>\n</section>\n");
        return;
    }
    html.push_str("<ul class=\"evidence-list\">\n");
    for link in matching {
        let source_label = label_for_id(&link.source_id, source_labels);
        html.push_str("<li>\n<strong>");
        push_escaped(html, &source_label);
        html.push_str("</strong>\n<span class=\"evidence-meta\">");
        push_escaped(
            html,
            &format!(
                "{}; {}",
                verification_status_label(link.verification_status),
                support_kind_label(link.support_kind)
            ),
        );
        html.push_str("</span>\n");
        if !link.locator.trim().is_empty() {
            html.push_str("<p><strong>Locator:</strong> ");
            push_escaped(html, &link.locator);
            html.push_str("</p>\n");
        }
        if !link.support_note.trim().is_empty() {
            html.push_str("<p>");
            push_escaped(html, &link.support_note);
            html.push_str("</p>\n");
        }
        if !link.reviewed_at.trim().is_empty() {
            html.push_str("<p class=\"meta\">Reviewed ");
            push_escaped(html, &link.reviewed_at);
            html.push_str("</p>\n");
        }
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n</section>\n");
}

fn push_claim_raw_details(html: &mut String, claim: &Claim) {
    let evidence_ids = claim
        .evidence_links
        .iter()
        .map(|link| first_non_empty([link.evidence_id.as_str(), link.source_id.as_str()]))
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    let source_ids = claim
        .evidence_links
        .iter()
        .map(|link| link.source_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(", ");
    push_raw_details(
        html,
        "Raw claim identifiers",
        &[
            ("Claim ID", claim.id.clone()),
            ("Evidence IDs", evidence_ids),
            ("Source IDs", source_ids),
            (
                "Evidence links",
                format_claim_evidence_links(&claim.evidence_links),
            ),
            ("Temporal fields", format_temporal(&claim.temporal)),
        ],
    );
}

fn push_raw_details(html: &mut String, summary: &str, rows: &[(&str, String)]) {
    if rows.iter().all(|(_, value)| value.trim().is_empty()) {
        return;
    }
    html.push_str("<details class=\"raw-identifiers\">\n<summary>");
    push_escaped(html, summary);
    html.push_str("</summary>\n<dl class=\"compact-dl\">\n");
    for (term, value) in rows {
        push_dl_item(html, term, value);
    }
    html.push_str("</dl>\n</details>\n");
}

fn source_label_lookup(report: &PublicReport) -> BTreeMap<String, String> {
    report
        .sources
        .iter()
        .map(|source| (source.id.clone(), source_display_label(source)))
        .collect()
}

fn entity_label_lookup(report: &PublicReport) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    for source in &report.sources {
        labels.insert(source.id.clone(), source_display_label(source));
    }
    for item in &report.field_elements {
        labels.insert(
            item.id.clone(),
            first_non_empty([item.label.as_str(), item.id.as_str()]),
        );
    }
    for item in &report.core_ideas {
        labels.insert(
            item.id.clone(),
            first_non_empty([item.label.as_str(), item.id.as_str()]),
        );
    }
    for item in &report.methods {
        labels.insert(
            item.id.clone(),
            first_non_empty([item.label.as_str(), item.id.as_str()]),
        );
    }
    for item in &report.representations {
        labels.insert(
            item.id.clone(),
            first_non_empty([item.label.as_str(), item.id.as_str()]),
        );
    }
    for claim in &report.claims {
        labels.insert(
            claim.id.clone(),
            first_non_empty([claim.statement.as_str(), claim.id.as_str()]),
        );
    }
    for step in &report.curriculum_path {
        labels.insert(
            step.id.clone(),
            first_non_empty([step.title.as_str(), step.id.as_str()]),
        );
    }
    for item in &report.frontier_debates {
        labels.insert(
            item.id.clone(),
            first_non_empty([item.title.as_str(), item.id.as_str()]),
        );
    }
    labels
}

fn source_display_label(source: &ReportSource) -> String {
    first_non_empty([
        source.title.as_str(),
        source.citation.as_str(),
        source.identifier.as_str(),
        source.id.as_str(),
    ])
}

fn labels_for_ids(ids: &[String], labels: &BTreeMap<String, String>) -> String {
    ids.iter()
        .map(|id| label_for_id(id, labels))
        .collect::<Vec<_>>()
        .join(", ")
}

fn label_for_id(id: &str, labels: &BTreeMap<String, String>) -> String {
    labels
        .get(id)
        .cloned()
        .unwrap_or_else(|| id.trim().to_string())
}

fn format_source_roles(roles: &[SourceRole]) -> String {
    roles
        .iter()
        .map(|role| source_role_label(*role))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_source_access(access: &SourceAccessMetadata) -> String {
    let mut parts = vec![
        access_status_label(access.status).to_string(),
        access.route.clone(),
    ];
    if !access.budget_estimate.trim().is_empty() {
        parts.push(format!("budget: {}", access.budget_estimate));
    }
    if !access.license.trim().is_empty() {
        parts.push(format!("license: {}", access.license));
    }
    if access.metadata_only == Some(true) {
        parts.push("metadata only".to_string());
    }
    if !access.notes.trim().is_empty() {
        parts.push(access.notes.clone());
    }
    parts.join("\n")
}

fn format_source_role_waiver(waiver: &SourceRoleWaiver) -> String {
    let mut parts = vec![waiver.rationale.clone(), format!("as_of: {}", waiver.as_of)];
    if !waiver.review_after.trim().is_empty() {
        parts.push(format!("review_after: {}", waiver.review_after));
    }
    parts.join("\n")
}

fn format_temporal(marker: &TemporalMarker) -> String {
    let mut parts = Vec::new();
    if !marker.as_of.trim().is_empty() {
        parts.push(format!("as_of: {}", marker.as_of));
    }
    if !marker.review_after.trim().is_empty() {
        parts.push(format!("review_after: {}", marker.review_after));
    }
    parts.push(format!(
        "status: {}",
        temporal_status_label(marker.temporal_status)
    ));
    if !marker.rationale.trim().is_empty() {
        parts.push(marker.rationale.clone());
    }
    parts.join("\n")
}

fn format_temporal_inline(marker: &TemporalMarker) -> String {
    let mut parts = Vec::new();
    parts.push(format!(
        "status: {}",
        temporal_status_label(marker.temporal_status)
    ));
    if !marker.as_of.trim().is_empty() {
        parts.push(format!("as of {}", marker.as_of));
    }
    if !marker.review_after.trim().is_empty() {
        parts.push(format!("review after {}", marker.review_after));
    }
    if !marker.rationale.trim().is_empty() {
        parts.push(marker.rationale.clone());
    }
    parts.join("; ")
}

fn format_claim_evidence_links(links: &[EvidenceLink]) -> String {
    links
        .iter()
        .map(|link| {
            let support = first_non_empty([link.locator.as_str(), link.support_note.as_str()]);
            format!(
                "{} [{}; {}; {}]",
                link.source_id,
                verification_status_label(link.verification_status),
                support_kind_label(link.support_kind),
                support
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn safe_script_json<T: Serialize>(value: &T) -> Result<String> {
    let json = serde_json::to_string(value).context("encode visual view JSON")?;
    Ok(json
        .replace('&', "\\u0026")
        .replace('<', "\\u003C")
        .replace('>', "\\u003E")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029"))
}

fn push_escaped(html: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '&' => html.push_str("&amp;"),
            '<' => html.push_str("&lt;"),
            '>' => html.push_str("&gt;"),
            '"' => html.push_str("&quot;"),
            '\'' => html.push_str("&#39;"),
            _ => html.push(ch),
        }
    }
}

fn push_escaped_attr(html: &mut String, value: &str) {
    push_escaped(html, value);
}

fn domain_classification_label(value: DomainClassification) -> &'static str {
    match value {
        DomainClassification::WellStructured => "well_structured",
        DomainClassification::Formal => "formal",
        DomainClassification::IllStructured => "ill_structured",
        DomainClassification::ProfessionalPractice => "professional_practice",
        DomainClassification::InstrumentBound => "instrument_bound",
        DomainClassification::InfrastructureBound => "infrastructure_bound",
        DomainClassification::Emerging => "emerging",
        DomainClassification::Interdisciplinary => "interdisciplinary",
        DomainClassification::Mixed => "mixed",
    }
}

fn source_role_requirement_label(value: SourceRoleRequirementKind) -> &'static str {
    match value {
        SourceRoleRequirementKind::Required => "required",
        SourceRoleRequirementKind::Conditional => "conditional",
        SourceRoleRequirementKind::Waived => "waived",
        SourceRoleRequirementKind::NotApplicable => "not_applicable",
    }
}

fn verification_status_label(value: VerificationStatus) -> &'static str {
    match value {
        VerificationStatus::Cataloged => "cataloged",
        VerificationStatus::Reviewed => "reviewed",
        VerificationStatus::Verified => "verified",
    }
}

fn claim_type_label(value: ClaimType) -> &'static str {
    match value {
        ClaimType::Structural => "structural",
        ClaimType::Currentness => "currentness",
        ClaimType::Frontier => "frontier",
        ClaimType::Debate => "debate",
        ClaimType::Curricular => "curricular",
        ClaimType::Interpretive => "interpretive",
        ClaimType::Methodological => "methodological",
    }
}

fn evidence_requirement_label(value: EvidenceRequirement) -> &'static str {
    match value {
        EvidenceRequirement::None => "none",
        EvidenceRequirement::CatalogedSource => "cataloged_source",
        EvidenceRequirement::ReviewedSource => "reviewed_source",
        EvidenceRequirement::VerifiedSource => "verified_source",
        EvidenceRequirement::MultipleReviewedSources => "multiple_reviewed_sources",
    }
}

fn claim_confidence_label(value: ClaimConfidence) -> &'static str {
    match value {
        ClaimConfidence::High => "high",
        ClaimConfidence::Medium => "medium",
        ClaimConfidence::Low => "low",
        ClaimConfidence::Unknown => "unknown",
    }
}

fn support_kind_label(value: SupportKind) -> &'static str {
    match value {
        SupportKind::Supports => "supports",
        SupportKind::Qualifies => "qualifies",
        SupportKind::Contradicts => "contradicts",
        SupportKind::Background => "background",
        SupportKind::Example => "example",
    }
}

fn frontier_debate_kind_label(value: FrontierDebateKind) -> &'static str {
    match value {
        FrontierDebateKind::Frontier => "frontier",
        FrontierDebateKind::Debate => "debate",
        FrontierDebateKind::OpenProblem => "open_problem",
        FrontierDebateKind::Uncertainty => "uncertainty",
    }
}

fn temporal_status_label(value: TemporalStatus) -> &'static str {
    match value {
        TemporalStatus::Durable => "durable",
        TemporalStatus::Current => "current",
        TemporalStatus::ReviewDue => "review_due",
        TemporalStatus::Stale => "stale",
        TemporalStatus::Unknown => "unknown",
    }
}

fn visual_view_kind_label(value: VisualViewKind) -> Option<&'static str> {
    match value {
        VisualViewKind::KnowledgeSpine => Some("knowledge_spine"),
        VisualViewKind::ConceptSource => Some("concept_source"),
        VisualViewKind::DependencyPath => Some("dependency_path"),
        VisualViewKind::FrontierDebate => Some("frontier_debate"),
        VisualViewKind::Custom => None,
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

fn source_content_key(source: &Source) -> String {
    [
        source.citation.trim(),
        source.title.trim(),
        source.identifier.trim(),
        source.url.trim(),
        source.source_type.trim(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

fn first_non_empty<const N: usize>(values: [&str; N]) -> String {
    values
        .into_iter()
        .find(|value| !value.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string()
}

fn normalize_access_status_label(raw: &str) -> String {
    normalize_id_text(raw).replace(' ', "_")
}

fn input_kind_for_path(path: &Path) -> String {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "csv" => "source_manifest_csv",
        "tsv" => "source_manifest_tsv",
        "json" => "source_manifest_json",
        _ => "source_manifest",
    }
    .to_string()
}

fn id_prefix(prefix: &str) -> String {
    let normalized = normalize_id_text(prefix).replace(' ', "-");
    if normalized.is_empty() {
        "item".to_string()
    } else {
        normalized
    }
}

fn slug_from_normalized(normalized: &str) -> String {
    let slug = normalized
        .split_whitespace()
        .take(8)
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "item".to_string()
    } else {
        slug
    }
}

fn full_hash(normalized: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    hex::encode(hasher.finalize())
}

fn short_hash(normalized: &str, len: usize) -> String {
    let hash = full_hash(normalized);
    hash[..len.min(hash.len())].to_string()
}

fn shortest_unique_hash_prefix(hash: &str, group: &[(String, String, String)]) -> usize {
    for len in (ID_HASH_LEN + 1)..=hash.len() {
        let prefix = &hash[..len];
        if group
            .iter()
            .filter(|(_, _, candidate_hash)| candidate_hash.starts_with(prefix))
            .count()
            == 1
        {
            return len;
        }
    }
    hash.len()
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
    }
    Ok(())
}
