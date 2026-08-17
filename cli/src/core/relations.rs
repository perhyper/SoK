use super::*;
use serde::de::{self, Visitor};
use serde::{Deserializer, Serializer};
use std::fmt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RelationKind {
    #[default]
    RequiresBefore,
    IntroducedBy,
    Revisits,
    Deepens,
    Applies,
    AssessedBy,
    Remediates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationKindMapping {
    pub kind: RelationKind,
    pub reverse_endpoints: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct RelationEndpoint {
    pub entity_type: RelationEndpointType,
    pub id: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RelationEndpointType {
    #[default]
    FieldElement,
    Concept,
    Claim,
    Source,
    CurriculumStep,
    FrontierDebate,
    Method,
    Representation,
}

impl Serialize for RelationKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(relation_kind_label(*self))
    }
}

impl<'de> Deserialize<'de> for RelationKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RelationKindVisitor;

        impl Visitor<'_> for RelationKindVisitor {
            type Value = RelationKind;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a canonical or legacy relation kind")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                relation_kind_mapping(value)
                    .map(|mapping| mapping.kind)
                    .ok_or_else(|| E::custom(format!("unknown relation kind {value:?}")))
            }
        }

        deserializer.deserialize_str(RelationKindVisitor)
    }
}

pub fn relation_kind_mapping(raw: &str) -> Option<RelationKindMapping> {
    let normalized = normalize_relation_label(raw);
    let mapping = match normalized.as_str() {
        "requires_before" | "precedes" | "precedes_in_curriculum" | "before" | "prior_to" => {
            RelationKindMapping {
                kind: RelationKind::RequiresBefore,
                reverse_endpoints: false,
            }
        }
        "depends_on" | "depends" | "requires" | "required_by" | "prerequisite" | "prereq"
        | "prerequisite_for" => RelationKindMapping {
            kind: RelationKind::RequiresBefore,
            reverse_endpoints: true,
        },
        "introduced_by" => RelationKindMapping {
            kind: RelationKind::IntroducedBy,
            reverse_endpoints: false,
        },
        "introduces" | "introduce" => RelationKindMapping {
            kind: RelationKind::IntroducedBy,
            reverse_endpoints: true,
        },
        "revisits" | "revisit" | "returns_to" => RelationKindMapping {
            kind: RelationKind::Revisits,
            reverse_endpoints: false,
        },
        "deepens"
        | "deepens_understanding_of"
        | "extends"
        | "elaborates"
        | "qualifies"
        | "qualify"
        | "limits"
        | "conditions" => RelationKindMapping {
            kind: RelationKind::Deepens,
            reverse_endpoints: false,
        },
        "applies"
        | "apply"
        | "uses_method"
        | "uses"
        | "uses_method_or_warrant"
        | "represented_by"
        | "represented"
        | "has_representation"
        | "maps_to"
        | "maps"
        | "corresponds_to"
        | "supports"
        | "support"
        | "supported_by"
        | "grounds"
        | "grounded_by"
        | "motivates"
        | "motivation"
        | "part_of"
        | "contains"
        | "component_of" => RelationKindMapping {
            kind: RelationKind::Applies,
            reverse_endpoints: false,
        },
        "assessed_by" | "assessed" | "assessment" | "tested_by" | "evaluated_by" => {
            RelationKindMapping {
                kind: RelationKind::AssessedBy,
                reverse_endpoints: false,
            }
        }
        "remediates" | "remediate" | "repairs" | "addresses_gap" | "contradicts" | "contradict"
        | "conflicts" => RelationKindMapping {
            kind: RelationKind::Remediates,
            reverse_endpoints: false,
        },
        _ => return None,
    };
    Some(mapping)
}

fn normalize_relation_label(raw: &str) -> String {
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
}

pub fn relation_kind_label(kind: RelationKind) -> &'static str {
    match kind {
        RelationKind::RequiresBefore => "requires_before",
        RelationKind::IntroducedBy => "introduced_by",
        RelationKind::Revisits => "revisits",
        RelationKind::Deepens => "deepens",
        RelationKind::Applies => "applies",
        RelationKind::AssessedBy => "assessed_by",
        RelationKind::Remediates => "remediates",
    }
}

pub fn relation_kind_human_label(kind: RelationKind) -> &'static str {
    match kind {
        RelationKind::RequiresBefore => "requires before",
        RelationKind::IntroducedBy => "is introduced by",
        RelationKind::Revisits => "revisits",
        RelationKind::Deepens => "deepens",
        RelationKind::Applies => "applies",
        RelationKind::AssessedBy => "is assessed by",
        RelationKind::Remediates => "remediates",
    }
}

pub fn is_hard_prerequisite(kind: RelationKind) -> bool {
    kind == RelationKind::RequiresBefore
}

pub fn endpoint_type_label(entity_type: RelationEndpointType) -> &'static str {
    match entity_type {
        RelationEndpointType::FieldElement => "field_element",
        RelationEndpointType::Concept => "concept",
        RelationEndpointType::Claim => "claim",
        RelationEndpointType::Source => "source",
        RelationEndpointType::CurriculumStep => "curriculum_step",
        RelationEndpointType::FrontierDebate => "frontier_debate",
        RelationEndpointType::Method => "method",
        RelationEndpointType::Representation => "representation",
    }
}

pub fn deduplicate_relations(relations: Vec<Relation>) -> Vec<Relation> {
    let mut by_semantic_key = BTreeMap::<RelationSemanticKey, Relation>::new();
    for mut relation in relations {
        relation.source_ids = sorted_unique(relation.source_ids);
        let key = RelationSemanticKey::from(&relation);
        by_semantic_key
            .entry(key)
            .and_modify(|existing| merge_relation(existing, &relation))
            .or_insert(relation);
    }
    by_semantic_key.into_values().collect()
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RelationSemanticKey {
    kind: RelationKind,
    from_type: RelationEndpointType,
    from_id: String,
    to_type: RelationEndpointType,
    to_id: String,
}

impl From<&Relation> for RelationSemanticKey {
    fn from(relation: &Relation) -> Self {
        Self {
            kind: relation.kind,
            from_type: relation.from.entity_type,
            from_id: relation.from.id.clone(),
            to_type: relation.to.entity_type,
            to_id: relation.to.id.clone(),
        }
    }
}

fn merge_relation(existing: &mut Relation, duplicate: &Relation) {
    if duplicate.id < existing.id {
        existing.id = duplicate.id.clone();
    }
    if !duplicate.description.trim().is_empty()
        && (existing.description.trim().is_empty() || duplicate.description < existing.description)
    {
        existing.description = duplicate.description.clone();
    }
    existing
        .source_ids
        .extend(duplicate.source_ids.iter().cloned());
    existing.source_ids = sorted_unique(std::mem::take(&mut existing.source_ids));
}

fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
