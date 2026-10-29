//! Shared relation records for core and public endpoint vocabularies.
use super::*;

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename = "Relation")]
pub struct RelationRecord<T> {
    pub id: String,
    pub kind: RelationKind,
    pub from: Endpoint<T>,
    pub to: Endpoint<T>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename = "RelationEndpoint", deny_unknown_fields)]
pub struct Endpoint<T> {
    pub entity_type: T,
    pub id: String,
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for RelationRecord<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RelationWire<T> {
            id: String,
            kind: String,
            from: Endpoint<T>,
            to: Endpoint<T>,
            #[serde(default)]
            description: String,
            #[serde(default)]
            source_ids: Vec<String>,
        }
        let wire = RelationWire::<T>::deserialize(deserializer)?;
        let mapping = relation_kind_mapping(&wire.kind)
            .ok_or_else(|| de::Error::custom(format!("unknown relation kind {:?}", wire.kind)))?;
        let (from, to) = if mapping.reverse_endpoints {
            (wire.to, wire.from)
        } else {
            (wire.from, wire.to)
        };
        Ok(Self {
            id: wire.id,
            kind: mapping.kind,
            from,
            to,
            description: wire.description,
            source_ids: wire.source_ids,
        })
    }
}

pub(super) fn deduplicate<T: Clone + Ord>(
    relations: Vec<RelationRecord<T>>,
) -> Vec<RelationRecord<T>> {
    let mut by_assertion = BTreeMap::new();
    for mut relation in relations {
        relation.source_ids = relation
            .source_ids
            .into_iter()
            .filter(|value| !value.trim().is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        // Keep the established ordering, including each endpoint vocabulary's order.
        let key = (
            relation.id.clone(),
            relation.description.clone(),
            relation.source_ids.clone(),
            relation.kind,
            relation.from.clone(),
            relation.to.clone(),
        );
        by_assertion.entry(key).or_insert(relation);
    }
    by_assertion.into_values().collect()
}
