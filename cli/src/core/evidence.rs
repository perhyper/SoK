use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidencePackage {
    pub schema_version: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<EvidenceSource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<EvidenceClaim>,
}

impl Default for EvidencePackage {
    fn default() -> Self {
        Self {
            schema_version: default_evidence_schema_version(),
            sources: Vec::new(),
            claims: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceSource {
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
    pub roles: Vec<report::SourceRole>,
    pub access: report::SourceAccessMetadata,
    pub why_it_matters: String,
    pub verification_status: report::VerificationStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_reviewed: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceClaim {
    pub id: String,
    pub statement: String,
    pub claim_type: report::ClaimType,
    pub evidence_requirement: report::EvidenceRequirement,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_links: Vec<report::EvidenceLink>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<report::ClaimConfidence>,
    pub temporal: report::TemporalMarker,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

pub fn default_evidence_schema_version() -> String {
    EVIDENCE_SCHEMA_VERSION.to_string()
}

impl From<&report::ReportSource> for EvidenceSource {
    fn from(value: &report::ReportSource) -> Self {
        Self {
            id: value.id.clone(),
            citation: value.citation.clone(),
            title: value.title.clone(),
            source_type: value.source_type.clone(),
            identifier: value.identifier.clone(),
            url: value.url.clone(),
            date: value.date.clone(),
            roles: value.roles.clone(),
            access: value.access.clone(),
            why_it_matters: value.why_it_matters.clone(),
            verification_status: value.verification_status,
            last_reviewed: value.last_reviewed.clone(),
            notes: value.notes.clone(),
        }
    }
}

impl From<&EvidenceSource> for report::ReportSource {
    fn from(value: &EvidenceSource) -> Self {
        Self {
            id: value.id.clone(),
            citation: value.citation.clone(),
            title: value.title.clone(),
            source_type: value.source_type.clone(),
            identifier: value.identifier.clone(),
            url: value.url.clone(),
            date: value.date.clone(),
            roles: value.roles.clone(),
            access: value.access.clone(),
            why_it_matters: value.why_it_matters.clone(),
            verification_status: value.verification_status,
            last_reviewed: value.last_reviewed.clone(),
            notes: value.notes.clone(),
        }
    }
}

impl From<&report::Claim> for EvidenceClaim {
    fn from(value: &report::Claim) -> Self {
        Self {
            id: value.id.clone(),
            statement: value.statement.clone(),
            claim_type: value.claim_type,
            evidence_requirement: value.evidence_requirement,
            evidence_links: value.evidence_links.clone(),
            confidence: value.confidence,
            temporal: value.temporal.clone(),
            notes: value.notes.clone(),
        }
    }
}

impl From<&EvidenceClaim> for report::Claim {
    fn from(value: &EvidenceClaim) -> Self {
        Self {
            id: value.id.clone(),
            statement: value.statement.clone(),
            claim_type: value.claim_type,
            evidence_requirement: value.evidence_requirement,
            evidence_links: value.evidence_links.clone(),
            confidence: value.confidence,
            temporal: value.temporal.clone(),
            notes: value.notes.clone(),
        }
    }
}
