//! Source manifest loading, normalization, and access policy helpers.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fs::{self, File};
use std::path::Path;

const SOURCE_FIELDS: [&str; 13] = [
    "title",
    "type",
    "identifier",
    "url",
    "date",
    "access_status",
    "access_route",
    "budget_estimate",
    "license",
    "layer",
    "why_it_matters",
    "use_in_curriculum",
    "notes",
];

const OPEN_ACCESS_STATUSES: [&str; 7] = [
    "open_access",
    "free_web",
    "public_domain",
    "cc_by",
    "cc_by_sa",
    "official_open",
    "user_provided",
];

const BLOCKED_ACCESS_STATUSES: [&str; 5] = [
    "paid_book",
    "paywalled",
    "subscription",
    "unknown",
    "restricted",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccessInfo {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub route: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub budget_estimate: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Source {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub citation: String,
    #[serde(default, rename = "type", skip_serializing_if = "String::is_empty")]
    pub source_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub identifier: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub date: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub access_status: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub access_route: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub budget_estimate: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub license: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub layer: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub why_it_matters: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub use_in_curriculum: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<AccessInfo>,
}

pub fn write_sources_csv<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
    }
    let file = File::create(path).with_context(|| format!("create {}", path.display()))?;
    let mut writer = csv::Writer::from_writer(file);
    writer.write_record(SOURCE_FIELDS)?;
    writer.flush()?;
    Ok(())
}

pub fn load_sources<P: AsRef<Path>>(path: P) -> Result<Vec<Source>> {
    let path = path.as_ref();
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "csv" => load_sources_table(path, b','),
        "tsv" => load_sources_table(path, b'\t'),
        "json" => load_sources_json(path),
        _ => bail!("manifest must be .json, .csv, or .tsv"),
    }
}

pub(crate) fn load_sources_table(path: &Path, delimiter: u8) -> Result<Vec<Source>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(delimiter)
        .from_path(path)
        .with_context(|| format!("open {}", path.display()))?;
    let mut rows = reader.records();
    let Some(headers) = rows.next() else {
        return Ok(Vec::new());
    };
    let headers = headers?;
    let headers = headers.iter().map(str::to_string).collect::<Vec<_>>();
    let mut sources = Vec::new();
    for row in rows {
        let row = row?;
        let values = headers
            .iter()
            .enumerate()
            .map(|(index, header)| (header.as_str(), row.get(index).unwrap_or("")))
            .collect::<HashMap<_, _>>();
        sources.push(Source {
            title: value_for(&values, "title"),
            source_type: value_for(&values, "type"),
            identifier: value_for(&values, "identifier"),
            url: value_for(&values, "url"),
            date: value_for(&values, "date"),
            access_status: value_for(&values, "access_status"),
            access_route: value_for(&values, "access_route"),
            budget_estimate: value_for(&values, "budget_estimate"),
            license: value_for(&values, "license"),
            layer: value_for(&values, "layer"),
            why_it_matters: value_for(&values, "why_it_matters"),
            use_in_curriculum: value_for(&values, "use_in_curriculum"),
            notes: value_for(&values, "notes"),
            ..Source::default()
        });
    }
    Ok(sources)
}

pub(crate) fn value_for(values: &HashMap<&str, &str>, key: &str) -> String {
    values.get(key).copied().unwrap_or("").to_string()
}

pub(crate) fn load_sources_json(path: &Path) -> Result<Vec<Source>> {
    let data = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    if let Ok(list) = serde_json::from_slice::<Vec<Source>>(&data) {
        return Ok(list);
    }
    #[derive(Deserialize)]
    struct WrappedSources {
        sources: Vec<Source>,
    }
    let wrapped: WrappedSources = serde_json::from_slice(&data)?;
    Ok(wrapped.sources)
}

pub fn source_access(item: &Source) -> AccessInfo {
    if let Some(access) = &item.access {
        let mut access = access.clone();
        access.status = normalize_access_status(&access.status);
        return access;
    }
    AccessInfo {
        status: normalize_access_status(&first_non_empty([&item.access_status])),
        route: first_non_empty([&item.access_route, &item.url]),
        budget_estimate: item.budget_estimate.clone(),
        notes: String::new(),
    }
}

pub(crate) fn first_non_empty<const N: usize>(values: [&str; N]) -> String {
    for value in values {
        if !value.trim().is_empty() {
            return value.to_string();
        }
    }
    String::new()
}

pub fn source_identity(item: &Source, index: usize) -> String {
    first_non_empty([
        &item.title,
        &item.citation,
        &item.url,
        &format!("source-{}", index + 1),
    ])
}

pub fn source_url(item: &Source) -> String {
    let access = source_access(item);
    first_non_empty([&item.url, &access.route])
}

pub fn audit_source(item: &Source, index: usize) -> Vec<String> {
    let label = source_identity(item, index);
    let access = source_access(item);
    let mut problems = Vec::new();
    if first_non_empty([&item.title, &item.citation]).is_empty() {
        problems.push(format!("{label}: missing title/citation"));
    }
    if item.source_type.trim().is_empty() {
        problems.push(format!("{label}: missing type"));
    }
    if first_non_empty([&item.identifier, &item.url]).is_empty() {
        problems.push(format!("{label}: missing identifier or url"));
    }
    if access.status.trim().is_empty() {
        problems.push(format!("{label}: missing access status"));
    }
    if access.route.trim().is_empty() {
        problems.push(format!("{label}: missing access route"));
    }
    if blocked_access_statuses().contains(access.status.as_str())
        && access.budget_estimate.trim().is_empty()
    {
        problems.push(format!(
            "{label}: paid/paywalled/unknown source needs budget estimate or library guidance"
        ));
    }
    if first_non_empty([&item.why_it_matters, &item.use_in_curriculum]).is_empty() {
        problems.push(format!("{label}: missing source role or relevance note"));
    }
    problems
}

pub fn parse_status_set(raw: &str) -> BTreeSet<String> {
    raw.split(',')
        .map(normalize_access_status)
        .filter(|status| !status.is_empty())
        .collect()
}

pub fn normalize_access_status(status: &str) -> String {
    status.trim().to_ascii_lowercase()
}

pub fn can_download(
    item: &Source,
    allowed: &BTreeSet<String>,
    include_unknown: bool,
) -> (bool, String) {
    let status = source_access(item).status;
    if status.is_empty() {
        if include_unknown {
            return (true, "unknown allowed by flag".to_string());
        }
        return (false, "blocked access status: missing".to_string());
    }
    if status == "unknown" {
        if include_unknown {
            return (true, "unknown allowed by flag".to_string());
        }
        return (false, "blocked access status: unknown".to_string());
    }
    if allowed.contains(&status) {
        return (true, "allowed".to_string());
    }
    if blocked_access_statuses().contains(status.as_str()) {
        return (false, format!("blocked access status: {status}"));
    }
    (false, format!("not in allowed statuses: {status}"))
}

pub(crate) fn blocked_access_statuses() -> BTreeSet<&'static str> {
    BLOCKED_ACCESS_STATUSES.into_iter().collect()
}

pub(crate) fn sorted_open_access_statuses() -> Vec<&'static str> {
    let mut statuses = OPEN_ACCESS_STATUSES.to_vec();
    statuses.sort_unstable();
    statuses
}

pub fn is_http_url(raw: &str) -> bool {
    url::Url::parse(raw)
        .map(|parsed| matches!(parsed.scheme(), "http" | "https"))
        .unwrap_or(false)
}
