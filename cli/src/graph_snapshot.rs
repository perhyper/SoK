//! Rebuildable, report-scoped graph interchange. Never merges entity identities.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::report;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: String,
    reports: Vec<ReportInput>,
    #[serde(default)]
    identity_links: Vec<IdentityLink>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportInput {
    report_id: String,
    path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct EntityRef {
    report_id: String,
    revision: String,
    entity_type: String,
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityLink {
    from: EntityRef,
    to: EntityRef,
    kind: IdentityKind,
    explanation: String,
    reviewed_at: String,
    reviewer: String,
    source_refs: Vec<EntityRef>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum IdentityKind {
    EquivalentTo,
    RelatedTo,
    DifferentFrom,
}

/// Input paths are relative to the manifest. Revisions hash exact source bytes.
/// Validation finishes before writing, and source artifacts cannot be overwritten.
pub fn export(manifest_path: &Path, output: &Path) -> Result<()> {
    let manifest: Manifest = serde_json::from_slice(&fs::read(manifest_path)?)?;
    if manifest.schema_version != "sok-graph-input/v1" || manifest.reports.is_empty() {
        bail!("expected sok-graph-input/v1 with at least one report");
    }
    let parent = manifest_path.parent().unwrap_or(Path::new("."));
    let output_existing = output.canonicalize().ok();
    let mut input_paths = vec![manifest_path.canonicalize()?];
    let mut reports = Vec::new();
    let mut nodes = Vec::new();
    let mut assertions = Vec::new();
    let mut known = BTreeSet::new();
    let mut namespaces = BTreeSet::new();

    for input in &manifest.reports {
        if input.report_id.trim().is_empty() || !namespaces.insert(&input.report_id) {
            bail!(
                "report_id must be nonempty and unique: {:?}",
                input.report_id
            );
        }
        let path = parent.join(&input.path);
        input_paths.push(path.canonicalize()?);
        let bytes = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
        let value: Value = serde_json::from_slice(&bytes)?;
        let validation = report::validate_report_value(&value);
        if validation.has_errors() || validation.has_warnings() {
            bail!(
                "{} requires strict validation: {} errors, {} warnings",
                input.report_id,
                validation.error_count(),
                validation.warning_count()
            );
        }
        let document: report::ReportDocument = serde_json::from_value(value)?;
        let revision = hex::encode(Sha256::digest(&bytes));
        let reference = |entity_type: &str, id: &str| EntityRef {
            report_id: input.report_id.clone(),
            revision: revision.clone(),
            entity_type: entity_type.into(),
            id: id.into(),
        };
        let public = serde_json::to_value(&document.report)?;
        reports.push(json!({
            "report_id": input.report_id, "revision": revision,
            "metadata": document.metadata, "field": public["field"], "scope": public["scope"]
        }));
        for (collection, entity_type) in [
            ("field_elements", "field_element"),
            ("core_ideas", "concept"),
            ("methods", "method"),
            ("representations", "representation"),
            ("sources", "source"),
            ("claims", "claim"),
            ("curriculum_path", "curriculum_step"),
            ("frontier_debates", "frontier_debate"),
        ] {
            for item in public[collection].as_array().into_iter().flatten() {
                let key = reference(entity_type, item["id"].as_str().context("entity ID")?);
                if !known.insert(key.clone()) {
                    bail!("duplicate graph entity: {key:?}");
                }
                nodes.push(json!({"key": key, "data": item}));
            }
        }
        for relation in &document.report.relations {
            let from = serde_json::to_value(&relation.from)?;
            let to = serde_json::to_value(&relation.to)?;
            assertions.push(json!({
                "key": reference("relation", &relation.id),
                "kind": relation.kind,
                "from": reference(from["entity_type"].as_str().unwrap(), &relation.from.id),
                "to": reference(to["entity_type"].as_str().unwrap(), &relation.to.id),
                "data": relation,
                "source_refs": relation.source_ids.iter().map(|id| reference("source", id))
                    .collect::<Vec<_>>()
            }));
        }
        for claim in &document.report.claims {
            for (index, link) in claim.evidence_links.iter().enumerate() {
                // Position is unique within the immutable revision, including absent evidence IDs.
                assertions.push(json!({
                    "key": reference("evidence_link", &format!("{}/{}", claim.id, index)),
                    "kind": link.support_kind,
                    "from": reference("source", &link.source_id),
                    "to": reference("claim", &claim.id), "data": link,
                    "source_refs": [reference("source", &link.source_id)]
                }));
            }
        }
    }
    for link in &manifest.identity_links {
        if !known.contains(&link.from) || !known.contains(&link.to) {
            bail!("identity link has a missing entity or stale report revision");
        }
        if link.from.report_id == link.to.report_id
            || link.explanation.trim().is_empty()
            || link.reviewer.trim().is_empty()
            || chrono::NaiveDate::parse_from_str(&link.reviewed_at, "%Y-%m-%d")
                .map(|date| date.to_string() != link.reviewed_at)
                .unwrap_or(true)
            || link.source_refs.is_empty()
            || link
                .source_refs
                .iter()
                .any(|source| source.entity_type != "source" || !known.contains(source))
        {
            bail!("cross-report identity links need distinct reports, review metadata and valid sources");
        }
    }
    // No implicit equivalence, transitive closure, or claim-conflict resolution.
    if output_existing.is_some_and(|path| input_paths.contains(&path)) {
        bail!("graph output must not overwrite an input artifact");
    }
    report::write_json_file(
        output,
        &json!({
            "schema_version": "sok-graph/v1", "reports": reports, "nodes": nodes,
            "assertions": assertions, "identity_links": manifest.identity_links
        }),
    )
}
