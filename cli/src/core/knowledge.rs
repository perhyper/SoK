use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KnowledgePackage {
    pub schema_version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub field: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<KnowledgeElement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<relations::Relation>,
}

impl Default for KnowledgePackage {
    fn default() -> Self {
        Self {
            schema_version: default_knowledge_schema_version(),
            field: String::new(),
            elements: Vec::new(),
            relations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeElement {
    pub id: String,
    pub element_class: String,
    pub label: LocalizedText,
    pub actual_form: LocalizedText,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_roles: Vec<SemanticRole>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role_note: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relation_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub load_bearing_relations: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LocalizedText {
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub locale: String,
}

impl LocalizedText {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            locale: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRole {
    Core,
    Surrounding,
    Context,
    Concept,
    Method,
    Representation,
    Object,
    Practice,
    Evidence,
    Frontier,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
    Unknown,
}

pub fn default_knowledge_schema_version() -> String {
    KNOWLEDGE_SCHEMA_VERSION.to_string()
}

impl From<report::ClaimConfidence> for Confidence {
    fn from(value: report::ClaimConfidence) -> Self {
        match value {
            report::ClaimConfidence::High => Self::High,
            report::ClaimConfidence::Medium => Self::Medium,
            report::ClaimConfidence::Low => Self::Low,
            report::ClaimConfidence::Unknown => Self::Unknown,
        }
    }
}

impl From<Confidence> for report::ClaimConfidence {
    fn from(value: Confidence) -> Self {
        match value {
            Confidence::High => Self::High,
            Confidence::Medium => Self::Medium,
            Confidence::Low => Self::Low,
            Confidence::Unknown => Self::Unknown,
        }
    }
}
