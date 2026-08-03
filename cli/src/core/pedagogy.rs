use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PedagogyPackage {
    pub schema_version: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reading_ladder: Vec<ReadingLadderRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub learning_path: Vec<LearningStep>,
}

impl Default for PedagogyPackage {
    fn default() -> Self {
        Self {
            schema_version: default_pedagogy_schema_version(),
            reading_ladder: Vec::new(),
            learning_path: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadingLadderRow {
    pub id: String,
    pub layer: String,
    pub start_here: String,
    pub read_for: String,
    pub do_not_infer: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LearningStep {
    pub id: String,
    pub sequence: u32,
    pub title: String,
    pub learning_goal: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prerequisite_ids: Vec<String>,
    pub practice_artifact: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub progress_criteria: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_ids: Vec<String>,
}

pub fn default_pedagogy_schema_version() -> String {
    PEDAGOGY_SCHEMA_VERSION.to_string()
}

impl From<&report::LiteratureLadderRow> for ReadingLadderRow {
    fn from(value: &report::LiteratureLadderRow) -> Self {
        Self {
            id: value.id.clone(),
            layer: value.layer.clone(),
            start_here: value.start_here.clone(),
            read_for: value.read_for.clone(),
            do_not_infer: value.do_not_infer.clone(),
            source_ids: value.source_ids.clone(),
            notes: value.notes.clone(),
        }
    }
}

impl From<&ReadingLadderRow> for report::LiteratureLadderRow {
    fn from(value: &ReadingLadderRow) -> Self {
        Self {
            id: value.id.clone(),
            layer: value.layer.clone(),
            start_here: value.start_here.clone(),
            read_for: value.read_for.clone(),
            do_not_infer: value.do_not_infer.clone(),
            source_ids: value.source_ids.clone(),
            notes: value.notes.clone(),
        }
    }
}

impl From<&report::CurriculumStep> for LearningStep {
    fn from(value: &report::CurriculumStep) -> Self {
        Self {
            id: value.id.clone(),
            sequence: value.sequence,
            title: value.title.clone(),
            learning_goal: value.learning_goal.clone(),
            prerequisite_ids: value.prerequisite_ids.clone(),
            practice_artifact: value.practice_artifact.clone(),
            progress_criteria: value.progress_criteria.clone(),
            source_ids: value.source_ids.clone(),
        }
    }
}

impl From<&LearningStep> for report::CurriculumStep {
    fn from(value: &LearningStep) -> Self {
        Self {
            id: value.id.clone(),
            sequence: value.sequence,
            title: value.title.clone(),
            learning_goal: value.learning_goal.clone(),
            prerequisite_ids: value.prerequisite_ids.clone(),
            practice_artifact: value.practice_artifact.clone(),
            progress_criteria: value.progress_criteria.clone(),
            source_ids: value.source_ids.clone(),
        }
    }
}
