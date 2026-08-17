//! Artifact-independent SoK core packages and compatibility projections.

use crate::report;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod evidence;
pub mod knowledge;
pub mod pedagogy;
pub mod projection;
pub mod relations;
pub mod validation;

pub const KNOWLEDGE_SCHEMA_VERSION: &str = "sok-knowledge/v1";
pub const EVIDENCE_SCHEMA_VERSION: &str = "sok-evidence/v1";
pub const PEDAGOGY_SCHEMA_VERSION: &str = "sok-pedagogy/v1";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorePackages {
    pub knowledge: knowledge::KnowledgePackage,
    pub evidence: evidence::EvidencePackage,
    pub pedagogy: pedagogy::PedagogyPackage,
}
