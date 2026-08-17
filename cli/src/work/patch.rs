use anyhow::{anyhow, bail, Context, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::core;

use super::model::*;
use super::validation::validate_work_result;

pub fn accept_work_result_patches(
    order: &WorkOrder,
    result: &WorkResult,
    packages: &core::CorePackages,
) -> Result<core::CorePackages> {
    let work_validation = validate_work_result(order, result);
    if work_validation.has_errors() {
        bail!(
            "work result failed validation: {}",
            work_validation.diagnostics.summary
        );
    }

    let patched = apply_work_patches(packages, result)?;
    let core_validation = core::validation::validate_core_packages(&patched);
    if core_validation.has_errors() {
        let messages = core_validation
            .diagnostics
            .checks
            .iter()
            .filter(|check| check.severity == crate::report::DiagnosticSeverity::Error)
            .map(|check| format!("{}: {}", check.check_id, check.message))
            .collect::<Vec<_>>();
        bail!(
            "patched core packages failed validation: {}",
            messages.join("; ")
        );
    }
    Ok(patched)
}

pub(crate) fn apply_work_patches(
    packages: &core::CorePackages,
    result: &WorkResult,
) -> Result<core::CorePackages> {
    let mut next = packages.clone();
    for patch in &result.patches {
        apply_one_patch(&mut next, patch)
            .with_context(|| format!("apply patch {}", patch.patch_id))?;
    }
    Ok(next)
}

pub fn is_supported_patch_path(path: &str) -> bool {
    if path_package_area(path).is_none() {
        return false;
    }
    let segments = path_segments(path);
    matches!(
        segments.as_slice(),
        ["knowledge", "field"]
            | ["knowledge", "elements"]
            | ["knowledge", "elements", _]
            | ["knowledge", "relations"]
            | ["knowledge", "relations", _]
            | ["evidence", "sources"]
            | ["evidence", "sources", _]
            | ["evidence", "claims"]
            | ["evidence", "claims", _]
            | ["pedagogy", "reading_ladder"]
            | ["pedagogy", "reading_ladder", _]
            | ["pedagogy", "learning_path"]
            | ["pedagogy", "learning_path", _]
    )
}

pub fn is_supported_patch_operation(path: &str, op: PatchOperation) -> bool {
    let segments = path_segments(path);
    match segments.as_slice() {
        ["knowledge", "field"] => op == PatchOperation::Replace,
        ["knowledge", "elements"]
        | ["knowledge", "elements", "-"]
        | ["knowledge", "relations"]
        | ["knowledge", "relations", "-"]
        | ["evidence", "sources"]
        | ["evidence", "sources", "-"]
        | ["evidence", "claims"]
        | ["evidence", "claims", "-"]
        | ["pedagogy", "reading_ladder"]
        | ["pedagogy", "reading_ladder", "-"]
        | ["pedagogy", "learning_path"]
        | ["pedagogy", "learning_path", "-"] => op == PatchOperation::Add,
        ["knowledge", "elements", _]
        | ["knowledge", "relations", _]
        | ["evidence", "sources", _]
        | ["evidence", "claims", _]
        | ["pedagogy", "reading_ladder", _]
        | ["pedagogy", "learning_path", _] => {
            matches!(op, PatchOperation::Replace | PatchOperation::Remove)
        }
        _ => false,
    }
}

pub fn path_package_area(path: &str) -> Option<CorePackageArea> {
    match path_segments(path).first().copied() {
        Some("knowledge") => Some(CorePackageArea::Knowledge),
        Some("evidence") => Some(CorePackageArea::Evidence),
        Some("pedagogy") => Some(CorePackageArea::Pedagogy),
        _ => None,
    }
}

pub fn path_allowed_by_prefixes(path: &str, allowed_prefixes: &[String]) -> bool {
    allowed_prefixes.iter().any(|prefix| {
        path == prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

pub(crate) fn patch_value_shape_error(patch: &WorkPatch) -> Option<String> {
    if patch.op == PatchOperation::Remove {
        return None;
    }
    let segments = path_segments(&patch.path);
    let result = match (patch.target_package, patch.op, segments.as_slice()) {
        (CorePackageArea::Knowledge, PatchOperation::Replace, ["knowledge", "field"]) => {
            decode_for_validation::<String>(patch).map(|_| ())
        }
        (CorePackageArea::Knowledge, PatchOperation::Add, ["knowledge", "elements"])
        | (CorePackageArea::Knowledge, PatchOperation::Add, ["knowledge", "elements", "-"]) => {
            decode_for_validation::<core::knowledge::KnowledgeElement>(patch).map(|_| ())
        }
        (CorePackageArea::Knowledge, PatchOperation::Replace, ["knowledge", "elements", id]) => {
            decode_for_validation::<core::knowledge::KnowledgeElement>(patch)
                .and_then(|item| require_matching_replacement_id(&item.id, id, patch))
        }
        (CorePackageArea::Knowledge, PatchOperation::Add, ["knowledge", "relations"])
        | (CorePackageArea::Knowledge, PatchOperation::Add, ["knowledge", "relations", "-"]) => {
            require_canonical_relation_kind(&patch.value)
                .and_then(|_| decode_for_validation::<core::relations::Relation>(patch))
                .map(|_| ())
        }
        (CorePackageArea::Knowledge, PatchOperation::Replace, ["knowledge", "relations", id]) => {
            require_canonical_relation_kind(&patch.value)
                .and_then(|_| decode_for_validation::<core::relations::Relation>(patch))
                .and_then(|item| require_matching_replacement_id(&item.id, id, patch))
        }
        (CorePackageArea::Evidence, PatchOperation::Add, ["evidence", "sources"])
        | (CorePackageArea::Evidence, PatchOperation::Add, ["evidence", "sources", "-"]) => {
            decode_for_validation::<core::evidence::EvidenceSource>(patch).map(|_| ())
        }
        (CorePackageArea::Evidence, PatchOperation::Replace, ["evidence", "sources", id]) => {
            decode_for_validation::<core::evidence::EvidenceSource>(patch)
                .and_then(|item| require_matching_replacement_id(&item.id, id, patch))
        }
        (CorePackageArea::Evidence, PatchOperation::Add, ["evidence", "claims"])
        | (CorePackageArea::Evidence, PatchOperation::Add, ["evidence", "claims", "-"]) => {
            decode_for_validation::<core::evidence::EvidenceClaim>(patch).map(|_| ())
        }
        (CorePackageArea::Evidence, PatchOperation::Replace, ["evidence", "claims", id]) => {
            decode_for_validation::<core::evidence::EvidenceClaim>(patch)
                .and_then(|item| require_matching_replacement_id(&item.id, id, patch))
        }
        (CorePackageArea::Pedagogy, PatchOperation::Add, ["pedagogy", "reading_ladder"])
        | (CorePackageArea::Pedagogy, PatchOperation::Add, ["pedagogy", "reading_ladder", "-"]) => {
            decode_for_validation::<core::pedagogy::ReadingLadderRow>(patch).map(|_| ())
        }
        (
            CorePackageArea::Pedagogy,
            PatchOperation::Replace,
            ["pedagogy", "reading_ladder", id],
        ) => decode_for_validation::<core::pedagogy::ReadingLadderRow>(patch)
            .and_then(|item| require_matching_replacement_id(&item.id, id, patch)),
        (CorePackageArea::Pedagogy, PatchOperation::Add, ["pedagogy", "learning_path"])
        | (CorePackageArea::Pedagogy, PatchOperation::Add, ["pedagogy", "learning_path", "-"]) => {
            decode_for_validation::<core::pedagogy::LearningStep>(patch).map(|_| ())
        }
        (CorePackageArea::Pedagogy, PatchOperation::Replace, ["pedagogy", "learning_path", id]) => {
            decode_for_validation::<core::pedagogy::LearningStep>(patch)
                .and_then(|item| require_matching_replacement_id(&item.id, id, patch))
        }
        _ => Ok(()),
    };
    result.err()
}

fn apply_one_patch(packages: &mut core::CorePackages, patch: &WorkPatch) -> Result<()> {
    let segments = path_segments(&patch.path);
    if !is_supported_patch_path(&patch.path) {
        bail!("unsupported patch path {:?}", patch.path);
    }
    match (patch.target_package, patch.op, segments.as_slice()) {
        (CorePackageArea::Knowledge, PatchOperation::Replace, ["knowledge", "field"]) => {
            packages.knowledge.field = decode_value::<String>(patch)?;
        }
        (CorePackageArea::Knowledge, PatchOperation::Add, ["knowledge", "elements"])
        | (CorePackageArea::Knowledge, PatchOperation::Add, ["knowledge", "elements", "-"]) => {
            packages
                .knowledge
                .elements
                .push(decode_value::<core::knowledge::KnowledgeElement>(patch)?);
        }
        (CorePackageArea::Knowledge, PatchOperation::Replace, ["knowledge", "elements", id]) => {
            replace_by_id(
                &mut packages.knowledge.elements,
                id,
                decode_value::<core::knowledge::KnowledgeElement>(patch)?,
                |item| item.id.as_str(),
            )?;
        }
        (CorePackageArea::Knowledge, PatchOperation::Remove, ["knowledge", "elements", id]) => {
            remove_by_id(&mut packages.knowledge.elements, id, |item| {
                item.id.as_str()
            })?;
        }
        (CorePackageArea::Knowledge, PatchOperation::Add, ["knowledge", "relations"])
        | (CorePackageArea::Knowledge, PatchOperation::Add, ["knowledge", "relations", "-"]) => {
            packages
                .knowledge
                .relations
                .push(decode_value::<core::relations::Relation>(patch)?);
        }
        (CorePackageArea::Knowledge, PatchOperation::Replace, ["knowledge", "relations", id]) => {
            replace_by_id(
                &mut packages.knowledge.relations,
                id,
                decode_value::<core::relations::Relation>(patch)?,
                |item| item.id.as_str(),
            )?;
        }
        (CorePackageArea::Knowledge, PatchOperation::Remove, ["knowledge", "relations", id]) => {
            remove_by_id(&mut packages.knowledge.relations, id, |item| {
                item.id.as_str()
            })?;
        }
        (CorePackageArea::Evidence, PatchOperation::Add, ["evidence", "sources"])
        | (CorePackageArea::Evidence, PatchOperation::Add, ["evidence", "sources", "-"]) => {
            packages
                .evidence
                .sources
                .push(decode_value::<core::evidence::EvidenceSource>(patch)?);
        }
        (CorePackageArea::Evidence, PatchOperation::Replace, ["evidence", "sources", id]) => {
            replace_by_id(
                &mut packages.evidence.sources,
                id,
                decode_value::<core::evidence::EvidenceSource>(patch)?,
                |item| item.id.as_str(),
            )?;
        }
        (CorePackageArea::Evidence, PatchOperation::Remove, ["evidence", "sources", id]) => {
            remove_by_id(&mut packages.evidence.sources, id, |item| item.id.as_str())?;
        }
        (CorePackageArea::Evidence, PatchOperation::Add, ["evidence", "claims"])
        | (CorePackageArea::Evidence, PatchOperation::Add, ["evidence", "claims", "-"]) => {
            packages
                .evidence
                .claims
                .push(decode_value::<core::evidence::EvidenceClaim>(patch)?);
        }
        (CorePackageArea::Evidence, PatchOperation::Replace, ["evidence", "claims", id]) => {
            replace_by_id(
                &mut packages.evidence.claims,
                id,
                decode_value::<core::evidence::EvidenceClaim>(patch)?,
                |item| item.id.as_str(),
            )?;
        }
        (CorePackageArea::Evidence, PatchOperation::Remove, ["evidence", "claims", id]) => {
            remove_by_id(&mut packages.evidence.claims, id, |item| item.id.as_str())?;
        }
        (CorePackageArea::Pedagogy, PatchOperation::Add, ["pedagogy", "reading_ladder"])
        | (CorePackageArea::Pedagogy, PatchOperation::Add, ["pedagogy", "reading_ladder", "-"]) => {
            packages
                .pedagogy
                .reading_ladder
                .push(decode_value::<core::pedagogy::ReadingLadderRow>(patch)?);
        }
        (
            CorePackageArea::Pedagogy,
            PatchOperation::Replace,
            ["pedagogy", "reading_ladder", id],
        ) => {
            replace_by_id(
                &mut packages.pedagogy.reading_ladder,
                id,
                decode_value::<core::pedagogy::ReadingLadderRow>(patch)?,
                |item| item.id.as_str(),
            )?;
        }
        (CorePackageArea::Pedagogy, PatchOperation::Remove, ["pedagogy", "reading_ladder", id]) => {
            remove_by_id(&mut packages.pedagogy.reading_ladder, id, |item| {
                item.id.as_str()
            })?;
        }
        (CorePackageArea::Pedagogy, PatchOperation::Add, ["pedagogy", "learning_path"])
        | (CorePackageArea::Pedagogy, PatchOperation::Add, ["pedagogy", "learning_path", "-"]) => {
            packages
                .pedagogy
                .learning_path
                .push(decode_value::<core::pedagogy::LearningStep>(patch)?);
        }
        (CorePackageArea::Pedagogy, PatchOperation::Replace, ["pedagogy", "learning_path", id]) => {
            replace_by_id(
                &mut packages.pedagogy.learning_path,
                id,
                decode_value::<core::pedagogy::LearningStep>(patch)?,
                |item| item.id.as_str(),
            )?;
        }
        (CorePackageArea::Pedagogy, PatchOperation::Remove, ["pedagogy", "learning_path", id]) => {
            remove_by_id(&mut packages.pedagogy.learning_path, id, |item| {
                item.id.as_str()
            })?;
        }
        _ => bail!(
            "operation {:?} is not supported for path {:?}",
            patch.op,
            patch.path
        ),
    }
    Ok(())
}

fn decode_value<T: DeserializeOwned>(patch: &WorkPatch) -> Result<T> {
    serde_json::from_value(patch.value.clone())
        .with_context(|| format!("decode patch value at {}", patch.path))
}

fn decode_for_validation<T: DeserializeOwned>(patch: &WorkPatch) -> std::result::Result<T, String> {
    serde_json::from_value(patch.value.clone()).map_err(|err| {
        format!(
            "patch value at {} does not match target shape: {err}",
            patch.path
        )
    })
}

fn require_matching_replacement_id(
    value_id: &str,
    path_id: &str,
    patch: &WorkPatch,
) -> std::result::Result<(), String> {
    if value_id == path_id {
        Ok(())
    } else {
        Err(format!(
            "replace patch {} path id {:?} does not match value id {:?}",
            patch.patch_id, path_id, value_id
        ))
    }
}

fn require_canonical_relation_kind(value: &Value) -> std::result::Result<(), String> {
    let Some(kind) = value.get("kind").and_then(Value::as_str) else {
        return Ok(());
    };
    if matches!(
        kind,
        "requires_before"
            | "introduced_by"
            | "revisits"
            | "deepens"
            | "applies"
            | "assessed_by"
            | "remediates"
    ) {
        Ok(())
    } else {
        Err(format!(
            "work relation patches require canonical relation kind, got {kind:?}"
        ))
    }
}

fn replace_by_id<T, F>(items: &mut [T], id: &str, replacement: T, id_of: F) -> Result<()>
where
    F: Fn(&T) -> &str,
{
    let index = items
        .iter()
        .position(|item| id_of(item) == id)
        .ok_or_else(|| anyhow!("patch target id {id:?} was not found"))?;
    items[index] = replacement;
    Ok(())
}

fn remove_by_id<T, F>(items: &mut Vec<T>, id: &str, id_of: F) -> Result<()>
where
    F: Fn(&T) -> &str,
{
    let index = items
        .iter()
        .position(|item| id_of(item) == id)
        .ok_or_else(|| anyhow!("patch target id {id:?} was not found"))?;
    items.remove(index);
    Ok(())
}

fn path_segments(path: &str) -> Vec<&str> {
    if !path.starts_with('/') || path.contains("//") {
        return Vec::new();
    }
    path.trim_start_matches('/').split('/').collect()
}
