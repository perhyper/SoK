use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{WORK_ORDER_SCHEMA_VERSION, WORK_PATCH_SCHEMA_VERSION, WORK_RESULT_SCHEMA_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkOrder {
    pub schema_version: String,
    pub work_order_id: String,
    pub task_kind: WorkTaskKind,
    pub objective: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub instructions: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_files: Vec<WorkFileRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_files: Vec<WorkFileRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_capabilities: Vec<WorkCapability>,
    pub allowed_patch_paths: Vec<String>,
    #[serde(default)]
    pub permissions: PermissionEnvelope,
}

impl WorkOrder {
    pub fn new(
        work_order_id: impl Into<String>,
        task_kind: WorkTaskKind,
        objective: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: WORK_ORDER_SCHEMA_VERSION.to_string(),
            work_order_id: work_order_id.into(),
            task_kind,
            objective: objective.into(),
            instructions: String::new(),
            source_text: String::new(),
            input_files: Vec::new(),
            output_files: Vec::new(),
            required_capabilities: vec![WorkCapability::BasicCompletion],
            allowed_patch_paths: default_allowed_patch_paths(task_kind),
            permissions: PermissionEnvelope::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkResult {
    pub schema_version: String,
    pub work_order_id: String,
    pub result_id: String,
    pub task_kind: WorkTaskKind,
    pub status: WorkResultStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub offered_capabilities: Vec<WorkCapability>,
    pub capability_response: CapabilityResponse,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patches: Vec<WorkPatch>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

impl WorkResult {
    pub fn patch_proposal(
        work_order_id: impl Into<String>,
        result_id: impl Into<String>,
        task_kind: WorkTaskKind,
        offered_capabilities: Vec<WorkCapability>,
        patches: Vec<WorkPatch>,
    ) -> Self {
        Self {
            schema_version: WORK_RESULT_SCHEMA_VERSION.to_string(),
            work_order_id: work_order_id.into(),
            result_id: result_id.into(),
            task_kind,
            status: WorkResultStatus::PatchProposal,
            offered_capabilities,
            capability_response: CapabilityResponse::accepted(),
            patches,
            notes: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkPatch {
    pub schema_version: String,
    pub patch_id: String,
    pub target_package: CorePackageArea,
    pub op: PatchOperation,
    pub path: String,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub value: Value,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,
}

impl WorkPatch {
    pub fn add(
        patch_id: impl Into<String>,
        target_package: CorePackageArea,
        path: impl Into<String>,
        value: Value,
    ) -> Self {
        Self {
            schema_version: WORK_PATCH_SCHEMA_VERSION.to_string(),
            patch_id: patch_id.into(),
            target_package,
            op: PatchOperation::Add,
            path: path.into(),
            value,
            rationale: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkFileRef {
    pub kind: String,
    pub path: String,
}

impl WorkFileRef {
    pub fn new(kind: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            path: path.into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PermissionEnvelope {
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub subprocess: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_roots: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub write_roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityResponse {
    pub status: CapabilityResolutionStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_capabilities: Vec<WorkCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub degradation_plan: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,
}

impl CapabilityResponse {
    pub fn accepted() -> Self {
        Self {
            status: CapabilityResolutionStatus::Accepted,
            missing_capabilities: Vec::new(),
            degradation_plan: Vec::new(),
            rationale: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WorkTaskKind {
    Framing,
    SourceRoleClassification,
    FieldElementExtraction,
    ClaimProposals,
    RelationProposals,
    CurriculumPrerequisiteProposals,
    ArchitectureComparison,
    ConsistencyCritique,
    ReportProjection,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WorkCapability {
    BasicCompletion,
    StructuredOutput,
    ToolCapable,
    LongContextAgent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkResultStatus {
    PatchProposal,
    Refusal,
    DegradationPlan,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityResolutionStatus {
    Accepted,
    Refusal,
    DegradationPlan,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CorePackageArea {
    Knowledge,
    Evidence,
    Pedagogy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PatchOperation {
    Add,
    Replace,
    Remove,
}

pub fn work_task_kind_label(kind: WorkTaskKind) -> &'static str {
    match kind {
        WorkTaskKind::Framing => "framing",
        WorkTaskKind::SourceRoleClassification => "source_role_classification",
        WorkTaskKind::FieldElementExtraction => "field_element_extraction",
        WorkTaskKind::ClaimProposals => "claim_proposals",
        WorkTaskKind::RelationProposals => "relation_proposals",
        WorkTaskKind::CurriculumPrerequisiteProposals => "curriculum_prerequisite_proposals",
        WorkTaskKind::ArchitectureComparison => "architecture_comparison",
        WorkTaskKind::ConsistencyCritique => "consistency_critique",
        WorkTaskKind::ReportProjection => "report_projection",
    }
}

pub fn work_capability_label(capability: WorkCapability) -> &'static str {
    match capability {
        WorkCapability::BasicCompletion => "basic_completion",
        WorkCapability::StructuredOutput => "structured_output",
        WorkCapability::ToolCapable => "tool_capable",
        WorkCapability::LongContextAgent => "long_context_agent",
    }
}

pub fn core_package_area_label(area: CorePackageArea) -> &'static str {
    match area {
        CorePackageArea::Knowledge => "knowledge",
        CorePackageArea::Evidence => "evidence",
        CorePackageArea::Pedagogy => "pedagogy",
    }
}

pub fn default_allowed_patch_paths(task_kind: WorkTaskKind) -> Vec<String> {
    let paths = match task_kind {
        WorkTaskKind::Framing => vec!["/knowledge/field"],
        WorkTaskKind::SourceRoleClassification => vec!["/evidence/sources"],
        WorkTaskKind::FieldElementExtraction => vec!["/knowledge/elements"],
        WorkTaskKind::ClaimProposals => vec!["/evidence/claims"],
        WorkTaskKind::RelationProposals => vec!["/knowledge/relations"],
        WorkTaskKind::CurriculumPrerequisiteProposals => {
            vec!["/knowledge/relations", "/pedagogy/learning_path"]
        }
        WorkTaskKind::ArchitectureComparison => vec!["/knowledge/elements", "/knowledge/relations"],
        WorkTaskKind::ConsistencyCritique => vec![
            "/knowledge/elements",
            "/knowledge/relations",
            "/evidence/claims",
            "/pedagogy/learning_path",
        ],
        WorkTaskKind::ReportProjection => vec![
            "/knowledge/elements",
            "/knowledge/relations",
            "/evidence/sources",
            "/evidence/claims",
            "/pedagogy/reading_ladder",
            "/pedagogy/learning_path",
        ],
    };
    paths.into_iter().map(str::to_string).collect()
}
