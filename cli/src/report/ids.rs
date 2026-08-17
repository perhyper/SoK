use super::html::{verification_status_label, visual_view_kind_label};
use super::model::*;
use super::relations::{relation_kind_label, visual_view_kind_export_label};
use super::validation::entity_type_label;
use super::*;
use icu_normalizer::ComposingNormalizerBorrowed;
use unicase::UniCase;

pub(crate) const ID_HASH_LEN: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdIdentity {
    pub display_slug: String,
    pub normalized_identity: String,
    pub identity_hash: String,
    pub stable_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdMigrationDocument {
    pub schema_version: String,
    pub source_schema_version: String,
    pub mappings: Vec<IdMigrationEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdMigrationEntry {
    pub path: String,
    pub entity_type: String,
    pub old_id: String,
    pub new_id: String,
    pub changed: bool,
    pub display_slug: String,
    pub identity_hash: String,
    pub normalized_identity: String,
}

pub fn content_id(prefix: &str, parts: &[&str]) -> String {
    content_id_identity(prefix, parts).stable_id
}

pub fn content_id_identity(prefix: &str, parts: &[&str]) -> IdIdentity {
    identity_for_text(prefix, &parts.join("\n"))
}

pub fn content_id_map<'a, I>(prefix: &str, inputs: I) -> BTreeMap<String, String>
where
    I: IntoIterator<Item = &'a str>,
{
    let prefix = id_prefix(prefix);
    let mut by_identity = BTreeMap::new();
    for input in inputs {
        let identity = identity_for_text_with_prefix(&prefix, input);
        by_identity
            .entry(identity.normalized_identity.clone())
            .or_insert(identity);
    }

    let mut groups: BTreeMap<String, Vec<IdIdentity>> = BTreeMap::new();
    for identity in by_identity.into_values() {
        let candidate = identity.stable_id.clone();
        groups.entry(candidate).or_default().push(identity);
    }

    let mut ids = BTreeMap::new();
    for (candidate, mut group) in groups {
        if group.len() == 1 {
            let identity = group.remove(0);
            ids.insert(identity.normalized_identity, candidate);
            continue;
        }
        group.sort_by(|left, right| {
            (
                &left.normalized_identity,
                &left.display_slug,
                &left.identity_hash,
            )
                .cmp(&(
                    &right.normalized_identity,
                    &right.display_slug,
                    &right.identity_hash,
                ))
        });
        for identity in &group {
            let unique_len = shortest_unique_identity_hash_prefix(&identity.identity_hash, &group);
            ids.insert(
                identity.normalized_identity.clone(),
                format!(
                    "{prefix}-{}-{}",
                    identity.display_slug,
                    &identity.identity_hash[..unique_len]
                ),
            );
        }
    }
    ids
}

pub fn normalize_identity_text(raw: &str) -> String {
    if raw.is_ascii() {
        let normalized = normalize_id_text(raw);
        return if normalized.is_empty() {
            "item".to_string()
        } else {
            normalized
        };
    }

    let nfc = ComposingNormalizerBorrowed::new_nfc();
    let collapsed = collapse_unicode_identity_whitespace(raw.trim());
    let canonical = nfc.normalize(&collapsed);
    let folded = UniCase::new(canonical.as_ref()).to_folded_case();
    let canonical_folded = nfc.normalize(&folded);
    let normalized = collapse_unicode_identity_whitespace(canonical_folded.trim());
    if normalized.is_empty() {
        "item".to_string()
    } else {
        normalized
    }
}

pub fn normalize_alias_text(raw: &str) -> String {
    normalize_identity_text(raw)
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

pub(crate) fn identity_for_text(prefix: &str, raw: &str) -> IdIdentity {
    identity_for_text_with_prefix(&id_prefix(prefix), raw)
}

pub(crate) fn identity_for_text_with_prefix(prefix: &str, raw: &str) -> IdIdentity {
    let normalized_identity = normalize_identity_text(raw);
    let display_normalized = normalize_id_text(raw);
    let display_slug = slug_from_normalized(&if display_normalized.is_empty() {
        "item".to_string()
    } else {
        display_normalized
    });
    let identity_hash = full_hash(&normalized_identity);
    let stable_id = format!(
        "{prefix}-{display_slug}-{}",
        &identity_hash[..ID_HASH_LEN.min(identity_hash.len())]
    );
    IdIdentity {
        display_slug,
        normalized_identity,
        identity_hash,
        stable_id,
    }
}

pub(crate) fn id_prefix(prefix: &str) -> String {
    let normalized = normalize_id_text(prefix).replace(' ', "-");
    if normalized.is_empty() {
        "item".to_string()
    } else {
        normalized
    }
}

pub(crate) fn slug_from_normalized(normalized: &str) -> String {
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

pub(crate) fn full_hash(normalized: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    hex::encode(hasher.finalize())
}

pub(crate) fn short_hash(normalized: &str, len: usize) -> String {
    let hash = full_hash(normalized);
    hash[..len.min(hash.len())].to_string()
}

pub(crate) fn shortest_unique_identity_hash_prefix(hash: &str, group: &[IdIdentity]) -> usize {
    for len in (ID_HASH_LEN + 1)..=hash.len() {
        let prefix = &hash[..len];
        if group
            .iter()
            .filter(|candidate| candidate.identity_hash.starts_with(prefix))
            .count()
            == 1
        {
            return len;
        }
    }
    hash.len()
}

pub(crate) fn collapse_unicode_identity_whitespace(raw: &str) -> String {
    let mut output = String::new();
    let mut previous_space = true;
    for ch in raw.chars() {
        if ch.is_whitespace() {
            if !previous_space {
                output.push(' ');
                previous_space = true;
            }
        } else {
            output.push(ch);
            previous_space = false;
        }
    }
    output.trim().to_string()
}

pub fn build_id_migration(document: &ReportDocument) -> IdMigrationDocument {
    let mut mappings = Vec::new();
    let mut entity_new_ids = BTreeMap::new();

    for (index, item) in document.report.literature_ladder.iter().enumerate() {
        push_migration(
            &mut mappings,
            format!("/report/literature_ladder/{index}/id"),
            "literature_ladder",
            &item.id,
            content_id_identity("ladder", &[&item.layer, &item.start_here]),
        );
    }
    for (index, item) in document.report.field_elements.iter().enumerate() {
        let identity = content_id_identity("element", &[&item.label]);
        entity_new_ids.insert(item.id.clone(), identity.stable_id.clone());
        push_migration(
            &mut mappings,
            format!("/report/field_elements/{index}/id"),
            "field_element",
            &item.id,
            identity,
        );
    }
    for (index, item) in document.report.core_ideas.iter().enumerate() {
        let identity = content_id_identity("concept", &[&item.label]);
        entity_new_ids.insert(item.id.clone(), identity.stable_id.clone());
        push_migration(
            &mut mappings,
            format!("/report/core_ideas/{index}/id"),
            "concept",
            &item.id,
            identity,
        );
    }
    for (index, item) in document.report.methods.iter().enumerate() {
        let identity = content_id_identity("method", &[&item.label]);
        entity_new_ids.insert(item.id.clone(), identity.stable_id.clone());
        push_migration(
            &mut mappings,
            format!("/report/methods/{index}/id"),
            "method",
            &item.id,
            identity,
        );
    }
    for (index, item) in document.report.representations.iter().enumerate() {
        let identity = content_id_identity("rep", &[&item.label]);
        entity_new_ids.insert(item.id.clone(), identity.stable_id.clone());
        push_migration(
            &mut mappings,
            format!("/report/representations/{index}/id"),
            "representation",
            &item.id,
            identity,
        );
    }
    for (index, source) in document.report.sources.iter().enumerate() {
        let identity = content_id_identity("src", &[&report_source_identity(source)]);
        entity_new_ids.insert(source.id.clone(), identity.stable_id.clone());
        push_migration(
            &mut mappings,
            format!("/report/sources/{index}/id"),
            "source",
            &source.id,
            identity,
        );
    }
    for (index, claim) in document.report.claims.iter().enumerate() {
        let identity = content_id_identity("claim", &[&claim.statement]);
        entity_new_ids.insert(claim.id.clone(), identity.stable_id.clone());
        push_migration(
            &mut mappings,
            format!("/report/claims/{index}/id"),
            "claim",
            &claim.id,
            identity,
        );
        for (link_index, link) in claim.evidence_links.iter().enumerate() {
            if link.evidence_id.trim().is_empty() {
                continue;
            }
            let source_id = migrated_id(&entity_new_ids, &link.source_id);
            let claim_id = migrated_id(&entity_new_ids, &claim.id);
            push_migration(
                &mut mappings,
                format!("/report/claims/{index}/evidence_links/{link_index}/evidence_id"),
                "evidence",
                &link.evidence_id,
                content_id_identity(
                    "ev",
                    &[
                        &source_id,
                        &claim_id,
                        verification_status_label(link.verification_status),
                    ],
                ),
            );
        }
    }
    for (index, step) in document.report.curriculum_path.iter().enumerate() {
        let identity = content_id_identity("step", &[&step.title]);
        entity_new_ids.insert(step.id.clone(), identity.stable_id.clone());
        push_migration(
            &mut mappings,
            format!("/report/curriculum_path/{index}/id"),
            "curriculum_step",
            &step.id,
            identity,
        );
    }
    for (index, item) in document.report.frontier_debates.iter().enumerate() {
        let identity = content_id_identity("frontier", &[&item.title]);
        entity_new_ids.insert(item.id.clone(), identity.stable_id.clone());
        push_migration(
            &mut mappings,
            format!("/report/frontier_debates/{index}/id"),
            "frontier_debate",
            &item.id,
            identity,
        );
    }

    let mut relation_new_ids = BTreeMap::new();
    for (index, relation) in document.report.relations.iter().enumerate() {
        let from_id = migrated_id(&entity_new_ids, &relation.from.id);
        let to_id = migrated_id(&entity_new_ids, &relation.to.id);
        let identity = content_id_identity(
            "rel",
            &[
                relation_kind_label(relation.kind),
                entity_type_label(relation.from.entity_type),
                &from_id,
                entity_type_label(relation.to.entity_type),
                &to_id,
            ],
        );
        relation_new_ids.insert(relation.id.clone(), identity.stable_id.clone());
        push_migration(
            &mut mappings,
            format!("/report/relations/{index}/id"),
            "relation",
            &relation.id,
            identity,
        );
    }

    if let Some(presentation) = &document.report.presentation {
        for (index, section) in presentation.sections.iter().enumerate() {
            push_migration(
                &mut mappings,
                format!("/report/presentation/sections/{index}/id"),
                "report_section",
                &section.id,
                content_id_identity("section", &[&index.to_string(), &section.title]),
            );
        }
    }

    for (view_index, view) in document.report.visual_views.iter().enumerate() {
        let relation_id_text = view
            .edges
            .iter()
            .filter_map(|edge| relation_new_ids.get(&edge.relation_id))
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        let kind = visual_view_kind_label(view.kind)
            .unwrap_or_else(|| visual_view_kind_export_label(view.kind));
        push_migration(
            &mut mappings,
            format!("/report/visual_views/{view_index}/id"),
            "visual_view",
            &view.id,
            content_id_identity("view", &[&view.title, kind, &relation_id_text]),
        );
        for (node_index, node) in view.nodes.iter().enumerate() {
            let ref_id = migrated_id(&entity_new_ids, &node.ref_id);
            push_migration(
                &mut mappings,
                format!("/report/visual_views/{view_index}/nodes/{node_index}/id"),
                "visual_node",
                &node.id,
                content_id_identity("vnode", &[entity_type_label(node.entity_type), &ref_id]),
            );
        }
    }

    mappings.sort_by(|left, right| {
        (&left.old_id, &left.entity_type, &left.path).cmp(&(
            &right.old_id,
            &right.entity_type,
            &right.path,
        ))
    });

    IdMigrationDocument {
        schema_version: "sok-id-migration/v1".to_string(),
        source_schema_version: document.metadata.schema_version.clone(),
        mappings,
    }
}

fn push_migration(
    mappings: &mut Vec<IdMigrationEntry>,
    path: String,
    entity_type: &str,
    old_id: &str,
    identity: IdIdentity,
) {
    mappings.push(IdMigrationEntry {
        path,
        entity_type: entity_type.to_string(),
        old_id: old_id.to_string(),
        new_id: identity.stable_id.clone(),
        changed: old_id != identity.stable_id,
        display_slug: identity.display_slug,
        identity_hash: identity.identity_hash,
        normalized_identity: identity.normalized_identity,
    });
}

fn migrated_id(migrations: &BTreeMap<String, String>, id: &str) -> String {
    migrations
        .get(id)
        .cloned()
        .unwrap_or_else(|| id.to_string())
}

fn report_source_identity(source: &ReportSource) -> String {
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
