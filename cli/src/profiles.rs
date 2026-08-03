//! Provisional discovery lenses used by the Rust CLI.
//!
//! These profiles narrow the first source search. They must never determine the
//! final report architecture; source-reviewed field findings do that later.

use serde::Deserialize;
use std::sync::OnceLock;

const DISCOVERY_PROFILES_JSON: &str =
    include_str!("../../structure-of-knowledge/references/discovery-profiles.v1.json");
const DISCOVERY_PROFILES_SCHEMA_VERSION: &str = "sok-discovery-profiles/v1";

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DiscoveryProfile {
    pub(crate) profile_id: String,
    pub(crate) profile_version: String,
    pub(crate) locale: String,
    pub(crate) classification_hint: String,
    pub(crate) why_this_hint: String,
    #[serde(default)]
    signal_terms: Vec<ProfileTerm>,
    #[serde(default, rename = "lens_prompts")]
    lenses: Vec<DiscoveryLens>,
    #[serde(default, rename = "scaffold_questions")]
    questions: Vec<String>,
    #[serde(default, rename = "source_role_probes")]
    source_roles: Vec<SourceRoleProbe>,
    #[serde(default)]
    projection_keyword_mappings: Vec<ProjectionKeywordMapping>,
    #[serde(default)]
    fallback_behavior: ProfileFallbackBehavior,
    #[serde(skip)]
    matched_signal: Option<ProfileTerm>,
}

#[derive(Debug, Clone, Deserialize)]
struct DiscoveryLens {
    name: String,
    inspect: String,
    revise_when: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SourceRoleProbe {
    role: String,
    inspect: String,
    decision_rule: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ProfileFallbackBehavior {
    #[serde(default)]
    advisory_message: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DiscoveryProfilesFile {
    schema_version: String,
    fallback_profile_id: String,
    inference_order: Vec<String>,
    profiles: Vec<DiscoveryProfile>,
    #[serde(default)]
    domain_classification_terms: Vec<DomainClassificationTerm>,
}

#[derive(Debug, Clone, Deserialize)]
struct DomainClassificationTerm {
    classification: String,
    terms: Vec<ProfileTerm>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProjectionKeywordMapping {
    projection: String,
    item_prefix: String,
    terms: Vec<ProfileTerm>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProfileTerm {
    term: String,
    locale: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectionRole {
    CoreIdea,
    Method,
    Representation,
    Omit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectionMatch {
    pub(crate) role: ProjectionRole,
    pub(crate) item_prefix: String,
}

impl DiscoveryProfile {
    pub(crate) fn render_lenses(&self) -> String {
        self.lenses
            .iter()
            .map(|lens| {
                format!(
                    "| {} | {} | {} | untested |",
                    lens.name, lens.inspect, lens.revise_when
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn render_questions(&self) -> String {
        self.questions
            .iter()
            .map(|question| format!("- {question}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn render_source_roles(&self) -> String {
        self.source_roles
            .iter()
            .map(|role| {
                format!(
                    "| {} | {} | {} | undecided |",
                    role.role, role.inspect, role.decision_rule
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn inference_confidence(&self) -> &'static str {
        if self.matched_signal.is_some() {
            "medium"
        } else {
            "low"
        }
    }

    pub(crate) fn inference_rationale(&self) -> String {
        match &self.matched_signal {
            Some(signal) => format!(
                "Matched field-name signal {:?} for locale {}; provisional until source review.",
                signal.term, signal.locale
            ),
            None => first_non_empty([
                self.fallback_behavior.advisory_message.as_str(),
                "No profile signal matched; using an open advisory fallback until source review.",
            ]),
        }
    }
}

pub(crate) fn infer_discovery_profile(field: &str) -> DiscoveryProfile {
    let data = discovery_profiles();
    let normalized_field = normalize_profile_text(field);

    for profile_id in &data.inference_order {
        for profile in data
            .profiles
            .iter()
            .filter(|profile| profile.profile_id == *profile_id)
        {
            if let Some(signal) = first_matching_term(&normalized_field, &profile.signal_terms) {
                let mut matched = profile.clone();
                matched.locale = signal.locale.clone();
                matched.matched_signal = Some(signal.clone());
                return matched;
            }
        }
    }

    fallback_profile(data)
}

pub(crate) fn domain_classification_ids_from_text(raw: &str) -> Vec<String> {
    let data = discovery_profiles();
    let text = normalize_profile_text(raw);
    let mut out = Vec::new();
    for mapping in &data.domain_classification_terms {
        if first_matching_term(&text, &mapping.terms).is_some()
            && !out.contains(&mapping.classification)
        {
            out.push(mapping.classification.clone());
        }
    }
    out
}

pub(crate) fn projection_for_element_class(raw: &str) -> ProjectionMatch {
    let text = normalize_profile_text(raw);
    for profile in &discovery_profiles().profiles {
        for mapping in &profile.projection_keyword_mappings {
            if first_matching_term(&text, &mapping.terms).is_some() {
                return ProjectionMatch {
                    role: projection_role(&mapping.projection),
                    item_prefix: mapping.item_prefix.clone(),
                };
            }
        }
    }

    ProjectionMatch {
        role: ProjectionRole::CoreIdea,
        item_prefix: "concept".to_string(),
    }
}

#[cfg(test)]
pub(crate) fn embedded_profile_schema_version() -> &'static str {
    discovery_profiles().schema_version.as_str()
}

fn discovery_profiles() -> &'static DiscoveryProfilesFile {
    static DATA: OnceLock<DiscoveryProfilesFile> = OnceLock::new();
    DATA.get_or_init(|| {
        let data: DiscoveryProfilesFile = serde_json::from_str(DISCOVERY_PROFILES_JSON)
            .expect("embedded discovery profiles JSON must parse");
        assert_eq!(
            data.schema_version, DISCOVERY_PROFILES_SCHEMA_VERSION,
            "embedded discovery profiles schema version changed without loader update"
        );
        data
    })
}

fn fallback_profile(data: &DiscoveryProfilesFile) -> DiscoveryProfile {
    let mut profile = data
        .profiles
        .iter()
        .find(|profile| profile.profile_id == data.fallback_profile_id)
        .cloned()
        .or_else(|| data.profiles.first().cloned())
        .expect("embedded discovery profiles must define at least one profile");
    profile.locale = "und".to_string();
    profile.matched_signal = None;
    profile
}

fn first_matching_term<'a>(text: &str, terms: &'a [ProfileTerm]) -> Option<&'a ProfileTerm> {
    terms
        .iter()
        .filter_map(|signal| {
            let needle = normalize_profile_text(&signal.term);
            (!needle.is_empty() && text.contains(&needle))
                .then_some((needle.chars().count(), signal))
        })
        .max_by_key(|(length, _)| *length)
        .map(|(_, signal)| signal)
}

fn projection_role(value: &str) -> ProjectionRole {
    match normalize_profile_text(value).as_str() {
        "method" => ProjectionRole::Method,
        "representation" => ProjectionRole::Representation,
        "omit" => ProjectionRole::Omit,
        _ => ProjectionRole::CoreIdea,
    }
}

fn normalize_profile_text(raw: &str) -> String {
    let mut output = String::new();
    let mut previous_space = true;
    for ch in raw.chars() {
        if ch.is_alphanumeric() {
            for folded in ch.to_lowercase() {
                output.push(folded);
            }
            previous_space = false;
        } else if !previous_space {
            output.push(' ');
            previous_space = true;
        }
    }
    output.trim().to_string()
}

fn first_non_empty(values: [&str; 2]) -> String {
    values
        .into_iter()
        .find(|value| !value.trim().is_empty())
        .unwrap_or("")
        .to_string()
}
