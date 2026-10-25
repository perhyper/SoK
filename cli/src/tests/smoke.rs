use super::*;

pub(super) fn validate_sok_report_smoke(value: &Value) -> std::result::Result<(), String> {
    let metadata = object_field(value, "metadata")?;
    let report = object_field(value, "report")?;
    require_string(metadata, "schema_version", "metadata")?;
    require_string(metadata, "generated_at", "metadata")?;
    let report_type = require_string(metadata, "report_type", "metadata")?;
    if !matches!(report_type, "scaffold" | "human_report") {
        return Err(format!("invalid report_type {report_type:?}"));
    }

    let temporal = object_child(metadata, "temporal_review", "metadata")?;
    require_string(temporal, "as_of", "metadata.temporal_review")?;
    let temporal_status = require_string(temporal, "temporal_status", "metadata.temporal_review")?;
    if !matches!(
        temporal_status,
        "durable" | "current" | "review_due" | "stale" | "unknown"
    ) {
        return Err(format!("invalid temporal_status {temporal_status:?}"));
    }

    for key in [
        "field",
        "scope",
        "domain_profile",
        "core_ideas",
        "methods",
        "representations",
        "evidence_standards",
        "sources",
        "claims",
        "relations",
        "curriculum_path",
        "frontier_debates",
    ] {
        if !report.contains_key(key) {
            return Err(format!("missing report.{key}"));
        }
    }

    for key in [
        "internal_context",
        "diagnostics",
        "raw_learner_profile",
        "original_goal",
        "prompt_derived_assumptions",
        "placeholder_state",
        "profile_proposals",
        "handoff_notes",
        "research_frame",
        "scaffold_quality_notes",
    ] {
        if report.contains_key(key) {
            return Err(format!("public report contains internal key {key}"));
        }
    }

    let mut ids: HashMap<String, BTreeSet<String>> = HashMap::new();
    collect_item_ids(report, "core_ideas", "concept", &mut ids)?;
    collect_item_ids(report, "methods", "method", &mut ids)?;
    collect_item_ids(report, "representations", "representation", &mut ids)?;
    if report.contains_key("field_elements") {
        collect_item_ids(report, "field_elements", "field_element", &mut ids)?;
    }
    collect_item_ids(report, "sources", "source", &mut ids)?;
    collect_item_ids(report, "claims", "claim", &mut ids)?;
    collect_item_ids(report, "curriculum_path", "curriculum_step", &mut ids)?;
    collect_item_ids(report, "frontier_debates", "frontier_debate", &mut ids)?;

    validate_sources(report)?;
    validate_literature_ladder_refs(report, &ids)?;
    validate_knowledge_source_refs(report, &ids)?;
    validate_claim_evidence(report, &ids)?;
    validate_curriculum_refs(report, &ids)?;
    validate_frontier_refs(report, &ids)?;
    let relation_ids = validate_relations(report, &ids)?;
    validate_visual_views(report, &ids, &relation_ids)?;
    validate_structure_waivers(report)?;
    Ok(())
}

pub(super) fn object_field<'a>(
    value: &'a Value,
    key: &str,
) -> std::result::Result<&'a serde_json::Map<String, Value>, String> {
    value
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("missing {key} object"))
}

pub(super) fn object_child<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> std::result::Result<&'a serde_json::Map<String, Value>, String> {
    object
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("missing {path}.{key} object"))
}

pub(super) fn array_child<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> std::result::Result<&'a Vec<Value>, String> {
    object
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing {path}.{key} array"))
}

pub(super) fn require_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> std::result::Result<&'a str, String> {
    let value = object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing {path}.{key} string"))?;
    if value.trim().is_empty() {
        return Err(format!("empty {path}.{key}"));
    }
    Ok(value)
}

pub(super) fn collect_item_ids(
    report: &serde_json::Map<String, Value>,
    key: &str,
    entity_type: &str,
    ids: &mut HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    for item in array_child(report, key, "report")? {
        let object = item
            .as_object()
            .ok_or_else(|| format!("report.{key} item should be object"))?;
        let id = require_string(object, "id", &format!("report.{key}[]"))?;
        validate_stable_id(id)?;
        let inserted = ids
            .entry(entity_type.to_string())
            .or_default()
            .insert(id.to_string());
        if !inserted {
            return Err(format!("duplicate {entity_type} id {id}"));
        }
    }
    Ok(())
}

pub(super) fn validate_sources(
    report: &serde_json::Map<String, Value>,
) -> std::result::Result<(), String> {
    for source in array_child(report, "sources", "report")? {
        let source = source
            .as_object()
            .ok_or_else(|| "report.sources item should be object".to_string())?;
        let id = require_string(source, "id", "report.sources[]")?;
        validate_stable_id(id)?;
        let access = object_child(source, "access", "report.sources[]")?;
        require_string(access, "status", "report.sources[].access")?;
        require_string(access, "route", "report.sources[].access")?;
        let verification = require_string(source, "verification_status", "report.sources[]")?;
        validate_verification_status(verification)?;
    }
    Ok(())
}

pub(super) fn validate_knowledge_source_refs(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    for section in ["core_ideas", "methods", "representations", "field_elements"] {
        if !report.contains_key(section) {
            continue;
        }
        for item in array_child(report, section, "report")? {
            let item = item
                .as_object()
                .ok_or_else(|| format!("report.{section} item should be object"))?;
            validate_source_id_array(item, "source_ids", ids, &format!("report.{section}[]"))?;
        }
    }
    Ok(())
}

pub(super) fn validate_literature_ladder_refs(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    let Some(rows) = report.get("literature_ladder") else {
        return Ok(());
    };
    let rows = rows
        .as_array()
        .ok_or_else(|| "report.literature_ladder should be array".to_string())?;
    for row in rows {
        let row = row
            .as_object()
            .ok_or_else(|| "report.literature_ladder item should be object".to_string())?;
        validate_stable_id(require_string(row, "id", "report.literature_ladder[]")?)?;
        require_string(row, "layer", "report.literature_ladder[]")?;
        require_string(row, "start_here", "report.literature_ladder[]")?;
        require_string(row, "read_for", "report.literature_ladder[]")?;
        require_string(row, "do_not_infer", "report.literature_ladder[]")?;
        validate_source_id_array(row, "source_ids", ids, "report.literature_ladder[]")?;
    }
    Ok(())
}

pub(super) fn validate_claim_evidence(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    for claim in array_child(report, "claims", "report")? {
        let claim = claim
            .as_object()
            .ok_or_else(|| "report.claims item should be object".to_string())?;
        let links = array_child(claim, "evidence_links", "report.claims[]")?;
        for link in links {
            let link = link
                .as_object()
                .ok_or_else(|| "claim evidence link should be object".to_string())?;
            let source_id = require_string(link, "source_id", "claim.evidence_links[]")?;
            validate_stable_id(source_id)?;
            if !entity_id_exists(ids, "source", source_id) {
                return Err(format!("unknown evidence source_id {source_id}"));
            }
            let verification =
                require_string(link, "verification_status", "claim.evidence_links[]")?;
            validate_verification_status(verification)?;
        }
    }
    Ok(())
}

pub(super) fn validate_curriculum_refs(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    for step in array_child(report, "curriculum_path", "report")? {
        let step = step
            .as_object()
            .ok_or_else(|| "report.curriculum_path item should be object".to_string())?;
        validate_source_id_array(step, "source_ids", ids, "report.curriculum_path[]")?;
        validate_known_id_array(step, "prerequisite_ids", ids, "report.curriculum_path[]")?;
    }
    Ok(())
}

pub(super) fn validate_frontier_refs(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    for item in array_child(report, "frontier_debates", "report")? {
        let item = item
            .as_object()
            .ok_or_else(|| "report.frontier_debates item should be object".to_string())?;
        validate_source_id_array(item, "source_ids", ids, "report.frontier_debates[]")?;
        validate_typed_id_array(item, "claim_ids", "claim", ids, "report.frontier_debates[]")?;
        validate_known_id_array(
            item,
            "required_background_ids",
            ids,
            "report.frontier_debates[]",
        )?;
    }
    Ok(())
}

pub(super) fn validate_source_id_array(
    object: &serde_json::Map<String, Value>,
    key: &str,
    ids: &HashMap<String, BTreeSet<String>>,
    path: &str,
) -> std::result::Result<(), String> {
    validate_typed_id_array(object, key, "source", ids, path)
}

pub(super) fn validate_typed_id_array(
    object: &serde_json::Map<String, Value>,
    key: &str,
    entity_type: &str,
    ids: &HashMap<String, BTreeSet<String>>,
    path: &str,
) -> std::result::Result<(), String> {
    let Some(values) = object.get(key) else {
        return Ok(());
    };
    let values = values
        .as_array()
        .ok_or_else(|| format!("{path}.{key} should be array"))?;
    for value in values {
        let id = value
            .as_str()
            .ok_or_else(|| format!("{path}.{key} item should be string"))?;
        validate_stable_id(id)?;
        if !entity_id_exists(ids, entity_type, id) {
            return Err(format!("unknown {entity_type} id {id}"));
        }
    }
    Ok(())
}

pub(super) fn validate_known_id_array(
    object: &serde_json::Map<String, Value>,
    key: &str,
    ids: &HashMap<String, BTreeSet<String>>,
    path: &str,
) -> std::result::Result<(), String> {
    let Some(values) = object.get(key) else {
        return Ok(());
    };
    let values = values
        .as_array()
        .ok_or_else(|| format!("{path}.{key} should be array"))?;
    for value in values {
        let id = value
            .as_str()
            .ok_or_else(|| format!("{path}.{key} item should be string"))?;
        validate_stable_id(id)?;
        if !known_id_exists(ids, id) {
            return Err(format!("unknown referenced id {id}"));
        }
    }
    Ok(())
}

pub(super) fn validate_relations(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<BTreeSet<String>, String> {
    let mut relation_ids = BTreeSet::new();
    for relation in array_child(report, "relations", "report")? {
        let relation = relation
            .as_object()
            .ok_or_else(|| "report.relations item should be object".to_string())?;
        let id = require_string(relation, "id", "report.relations[]")?;
        validate_stable_id(id)?;
        if !relation_ids.insert(id.to_string()) {
            return Err(format!("duplicate relation id {id}"));
        }
        validate_relation_endpoint(relation, "from", ids)?;
        validate_relation_endpoint(relation, "to", ids)?;
    }
    Ok(relation_ids)
}

pub(super) fn validate_relation_endpoint(
    relation: &serde_json::Map<String, Value>,
    key: &str,
    ids: &HashMap<String, BTreeSet<String>>,
) -> std::result::Result<(), String> {
    let endpoint = object_child(relation, key, "report.relations[]")?;
    let entity_type = require_string(endpoint, "entity_type", "relation endpoint")?;
    if !valid_entity_type(entity_type) {
        return Err(format!("invalid relation entity_type {entity_type}"));
    }
    let id = require_string(endpoint, "id", "relation endpoint")?;
    validate_stable_id(id)?;
    if !entity_id_exists(ids, entity_type, id) {
        return Err(format!("unknown relation endpoint {entity_type}:{id}"));
    }
    Ok(())
}

pub(super) fn validate_visual_views(
    report: &serde_json::Map<String, Value>,
    ids: &HashMap<String, BTreeSet<String>>,
    relation_ids: &BTreeSet<String>,
) -> std::result::Result<(), String> {
    let Some(views) = report.get("visual_views") else {
        return Ok(());
    };
    let views = views
        .as_array()
        .ok_or_else(|| "report.visual_views should be array".to_string())?;
    for view in views {
        let view = view
            .as_object()
            .ok_or_else(|| "visual view should be object".to_string())?;
        validate_stable_id(require_string(view, "id", "visual_views[]")?)?;
        let mut visual_node_ids = BTreeSet::new();
        for node in array_child(view, "nodes", "visual_views[]")? {
            let node = node
                .as_object()
                .ok_or_else(|| "visual view node should be object".to_string())?;
            let node_id = require_string(node, "id", "visual_view.nodes[]")?;
            validate_stable_id(node_id)?;
            visual_node_ids.insert(node_id.to_string());
            if let Some(ref_id) = node.get("ref_id").and_then(Value::as_str) {
                validate_stable_id(ref_id)?;
                let entity_type = require_string(node, "entity_type", "visual_view.nodes[]")?;
                if !entity_id_exists(ids, entity_type, ref_id) {
                    return Err(format!("unknown visual ref_id {entity_type}:{ref_id}"));
                }
            }
        }
        for edge in array_child(view, "edges", "visual_views[]")? {
            let edge = edge
                .as_object()
                .ok_or_else(|| "visual view edge should be object".to_string())?;
            for key in ["from", "to"] {
                let endpoint = require_string(edge, key, "visual_view.edges[]")?;
                validate_stable_id(endpoint)?;
                if !visual_node_ids.contains(endpoint) {
                    return Err(format!("unknown visual edge endpoint {endpoint}"));
                }
            }
            if let Some(relation_id) = edge.get("relation_id").and_then(Value::as_str) {
                validate_stable_id(relation_id)?;
                if !relation_ids.contains(relation_id) {
                    return Err(format!("unknown visual relation_id {relation_id}"));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn validate_structure_waivers(
    report: &serde_json::Map<String, Value>,
) -> std::result::Result<(), String> {
    let Some(waivers) = report.get("structure_waivers") else {
        return Ok(());
    };
    let waivers = waivers
        .as_array()
        .ok_or_else(|| "report.structure_waivers should be array".to_string())?;
    for waiver in waivers {
        let waiver = waiver
            .as_object()
            .ok_or_else(|| "structure waiver should be object".to_string())?;
        let scope = require_string(waiver, "scope", "report.structure_waivers[]")?;
        if !matches!(
            scope,
            "relations" | "curriculum_prerequisites" | "visual_views"
        ) {
            return Err(format!("invalid structure waiver scope {scope:?}"));
        }
        require_string(waiver, "rationale", "report.structure_waivers[]")?;
    }
    Ok(())
}

pub(super) fn validate_stable_id(id: &str) -> std::result::Result<(), String> {
    let valid = id.contains('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .chars()
            .next()
            .map(|first| first.is_ascii_lowercase())
            .unwrap_or(false)
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
    if valid {
        Ok(())
    } else {
        Err(format!("invalid stable id {id:?}"))
    }
}

pub(super) fn validate_verification_status(status: &str) -> std::result::Result<(), String> {
    if matches!(status, "cataloged" | "reviewed" | "verified") {
        Ok(())
    } else {
        Err(format!("invalid verification status {status:?}"))
    }
}

pub(super) fn entity_id_exists(
    ids: &HashMap<String, BTreeSet<String>>,
    entity_type: &str,
    id: &str,
) -> bool {
    ids.get(entity_type)
        .map(|known| known.contains(id))
        .unwrap_or(false)
}

pub(super) fn known_id_exists(ids: &HashMap<String, BTreeSet<String>>, id: &str) -> bool {
    ids.values().any(|known| known.contains(id))
}

pub(super) fn valid_entity_type(entity_type: &str) -> bool {
    matches!(
        entity_type,
        "concept"
            | "claim"
            | "source"
            | "curriculum_step"
            | "frontier_debate"
            | "method"
            | "representation"
            | "field_element"
    )
}
