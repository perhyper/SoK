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
    Supports,
    Qualifies,
    Contradicts,
    PartOf,
    RepresentedBy,
    MapsTo,
    Grounds,
    Motivates,
    UsesMethod,
    Extends,
    Limits,
    Conditions,
    CorrespondsTo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationKindMapping {
    pub kind: RelationKind,
    pub reverse_endpoints: bool,
}

mod record;
pub use record::{Endpoint, RelationRecord};

pub type Relation = RelationRecord<RelationEndpointType>;
pub type RelationEndpoint = Endpoint<RelationEndpointType>;

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
    let (kind, reverse_endpoints) = match normalized.as_str() {
        "requires_before"
        | "precedes"
        | "precedes_in_curriculum"
        | "before"
        | "prior_to"
        | "required_by"
        | "prerequisite_for" => (RelationKind::RequiresBefore, false),
        "depends_on" | "depends" | "requires" | "prerequisite" | "prereq" => {
            (RelationKind::RequiresBefore, true)
        }
        "introduced_by" => (RelationKind::IntroducedBy, false),
        "introduces" | "introduce" => (RelationKind::IntroducedBy, true),
        "revisits" | "revisit" | "returns_to" => (RelationKind::Revisits, false),
        "deepens" | "deepens_understanding_of" | "elaborates" => (RelationKind::Deepens, false),
        "applies" | "apply" => (RelationKind::Applies, false),
        "assessed_by" | "assessed" | "assessment" | "tested_by" | "evaluated_by" => {
            (RelationKind::AssessedBy, false)
        }
        "remediates" | "remediate" | "repairs" | "addresses_gap" => {
            (RelationKind::Remediates, false)
        }
        "supports" | "support" => (RelationKind::Supports, false),
        "qualifies" | "qualify" => (RelationKind::Qualifies, false),
        "contradicts" | "contradict" | "conflicts" => (RelationKind::Contradicts, false),
        "part_of" | "component_of" => (RelationKind::PartOf, false),
        "represented_by" | "represented" | "has_representation" => {
            (RelationKind::RepresentedBy, false)
        }
        "maps_to" | "maps" => (RelationKind::MapsTo, false),
        "grounds" => (RelationKind::Grounds, false),
        "motivates" | "motivation" => (RelationKind::Motivates, false),
        "uses_method" | "uses" | "uses_method_or_warrant" => (RelationKind::UsesMethod, false),
        "extends" => (RelationKind::Extends, false),
        "limits" => (RelationKind::Limits, false),
        "conditions" => (RelationKind::Conditions, false),
        "corresponds_to" => (RelationKind::CorrespondsTo, false),
        "supported_by" => (RelationKind::Supports, true),
        "grounded_by" => (RelationKind::Grounds, true),
        "contains" => (RelationKind::PartOf, true),
        _ => return None,
    };
    let mapping = RelationKindMapping {
        kind,
        reverse_endpoints,
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
        RelationKind::Supports => "supports",
        RelationKind::Qualifies => "qualifies",
        RelationKind::Contradicts => "contradicts",
        RelationKind::PartOf => "part_of",
        RelationKind::RepresentedBy => "represented_by",
        RelationKind::MapsTo => "maps_to",
        RelationKind::Grounds => "grounds",
        RelationKind::Motivates => "motivates",
        RelationKind::UsesMethod => "uses_method",
        RelationKind::Extends => "extends",
        RelationKind::Limits => "limits",
        RelationKind::Conditions => "conditions",
        RelationKind::CorrespondsTo => "corresponds_to",
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
        RelationKind::Supports => "supports",
        RelationKind::Qualifies => "qualifies",
        RelationKind::Contradicts => "contradicts",
        RelationKind::PartOf => "part of",
        RelationKind::RepresentedBy => "represented by",
        RelationKind::MapsTo => "maps to",
        RelationKind::Grounds => "grounds",
        RelationKind::Motivates => "motivates",
        RelationKind::UsesMethod => "uses method",
        RelationKind::Extends => "extends",
        RelationKind::Limits => "limits",
        RelationKind::Conditions => "conditions",
        RelationKind::CorrespondsTo => "corresponds to",
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
    record::deduplicate(relations)
}

pub(crate) fn deduplicate_public_relations(
    relations: Vec<crate::report::Relation>,
) -> Vec<crate::report::Relation> {
    record::deduplicate(relations)
}
