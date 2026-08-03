use super::*;

pub(crate) const ID_HASH_LEN: usize = 10;

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

pub(crate) fn shortest_unique_hash_prefix(hash: &str, group: &[(String, String, String)]) -> usize {
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
