//! Deterministic conformance scoring over saved SoK fixtures.

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core;
use crate::report;
use crate::run_manifest;
use crate::work;

pub const CONFORMANCE_SUITE_SCHEMA_VERSION: &str = "sok-conformance-suite/v1";
pub const CONFORMANCE_REPORT_SCHEMA_VERSION: &str = "sok-conformance-report/v1";

pub const CHECK_EVAL_ID_DUPLICATE: &str = "eval.id.duplicate";
pub const CHECK_EVAL_ID_FORMAT: &str = "eval.id.format";
pub const CHECK_EVAL_ID_IDENTITY: &str = "eval.id.identity";
pub const CHECK_EVAL_INPUT: &str = "eval.input";
pub const CHECK_EVAL_MULTILINGUAL_RETENTION: &str = "eval.multilingual.retention";
pub const CHECK_EVAL_RUN_REQUIRED: &str = "run.required";
pub const CHECK_EVAL_RUN_VERSION: &str = "run.version";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConformanceSuite {
    pub schema_version: String,
    pub suite_id: String,
    pub cases: Vec<ConformanceCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConformanceCase {
    pub id: String,
    pub contract: String,
    pub kind: ConformanceCaseKind,
    #[serde(default)]
    pub input: ConformanceInput,
    #[serde(default)]
    pub mutations: Vec<FixtureMutation>,
    #[serde(default)]
    pub expect: ConformanceExpect,
    #[serde(default)]
    pub repair_count: u32,
    #[serde(default)]
    pub human_correction: HumanCorrectionMetadata,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConformanceCaseKind {
    CorePackages,
    Report,
    WorkOrder,
    WorkResult,
    RunManifest,
    IdInventory,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConformanceInput {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub knowledge: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub evidence: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pedagogy: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub order: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub result: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identity_checks: Vec<IdentityCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct IdentityCheck {
    pub prefix: String,
    pub parts: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub expected_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub expected_normalized_identity: String,
    #[serde(default)]
    pub require_non_ascii_retention: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FixtureMutation {
    pub op: FixtureMutationOperation,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<FixtureMutationTarget>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub value: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FixtureMutationOperation {
    Set,
    Remove,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FixtureMutationTarget {
    Knowledge,
    Evidence,
    Pedagogy,
    Report,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConformanceExpect {
    pub valid: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_check_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forbidden_check_ids: Vec<String>,
}

impl Default for ConformanceExpect {
    fn default() -> Self {
        Self {
            valid: true,
            required_check_ids: Vec::new(),
            forbidden_check_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HumanCorrectionMetadata {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub status: String,
    #[serde(default)]
    pub count: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConformanceReport {
    pub schema_version: String,
    pub suite_id: String,
    pub total_cases: usize,
    pub expectation_passed: usize,
    pub expectation_failed: usize,
    pub expectation_pass_rate: String,
    pub validation_passed: usize,
    pub validation_failed: usize,
    pub validation_pass_rate: String,
    pub validation_error_count: usize,
    pub validation_warning_count: usize,
    pub repair_count: u32,
    pub human_correction_count: u32,
    pub cases: Vec<ConformanceCaseResult>,
}

impl ConformanceReport {
    pub fn all_expectations_passed(&self) -> bool {
        self.expectation_failed == 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConformanceCaseResult {
    pub id: String,
    pub contract: String,
    pub kind: ConformanceCaseKind,
    pub expected_valid: bool,
    pub actual_valid: bool,
    pub expectation_passed: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub check_ids: Vec<String>,
    pub missing_required_check_ids: Vec<String>,
    pub forbidden_check_ids_present: Vec<String>,
    pub repair_count: u32,
    pub human_correction: HumanCorrectionMetadata,
}

pub fn score_conformance_dir(fixtures_dir: impl AsRef<Path>) -> Result<ConformanceReport> {
    let fixtures_dir = fixtures_dir.as_ref();
    let manifest_path = fixtures_dir.join("manifest.json");
    let suite: ConformanceSuite = report::read_json_file(&manifest_path)?;
    score_conformance_suite(fixtures_dir, suite)
}

pub fn score_conformance_suite(
    fixtures_dir: &Path,
    suite: ConformanceSuite,
) -> Result<ConformanceReport> {
    if suite.schema_version != CONFORMANCE_SUITE_SCHEMA_VERSION {
        bail!(
            "unsupported conformance suite schema_version {:?}; expected {}",
            suite.schema_version,
            CONFORMANCE_SUITE_SCHEMA_VERSION
        );
    }

    let mut results = Vec::new();
    for case in &suite.cases {
        results.push(score_conformance_case(fixtures_dir, case)?);
    }

    let total_cases = results.len();
    let expectation_passed = results
        .iter()
        .filter(|result| result.expectation_passed)
        .count();
    let validation_passed = results.iter().filter(|result| result.actual_valid).count();
    let validation_error_count = results.iter().map(|result| result.error_count).sum();
    let validation_warning_count = results.iter().map(|result| result.warning_count).sum();
    let repair_count = results.iter().map(|result| result.repair_count).sum();
    let human_correction_count = results
        .iter()
        .map(|result| result.human_correction.count)
        .sum();

    Ok(ConformanceReport {
        schema_version: CONFORMANCE_REPORT_SCHEMA_VERSION.to_string(),
        suite_id: suite.suite_id,
        total_cases,
        expectation_passed,
        expectation_failed: total_cases - expectation_passed,
        expectation_pass_rate: ratio_string(expectation_passed, total_cases),
        validation_passed,
        validation_failed: total_cases - validation_passed,
        validation_pass_rate: ratio_string(validation_passed, total_cases),
        validation_error_count,
        validation_warning_count,
        repair_count,
        human_correction_count,
        cases: results,
    })
}

fn score_conformance_case(
    fixtures_dir: &Path,
    case: &ConformanceCase,
) -> Result<ConformanceCaseResult> {
    let diagnostics = match case.kind {
        ConformanceCaseKind::CorePackages => score_core_packages(fixtures_dir, case)?,
        ConformanceCaseKind::Report => score_report(fixtures_dir, case)?,
        ConformanceCaseKind::WorkOrder => score_work_order(fixtures_dir, case)?,
        ConformanceCaseKind::WorkResult => score_work_result(fixtures_dir, case)?,
        ConformanceCaseKind::RunManifest => score_run_manifest(fixtures_dir, case)?,
        ConformanceCaseKind::IdInventory => score_id_inventory(case),
    };
    let error_count = diagnostics
        .checks
        .iter()
        .filter(|check| check.severity == report::DiagnosticSeverity::Error)
        .count();
    let warning_count = diagnostics
        .checks
        .iter()
        .filter(|check| check.severity == report::DiagnosticSeverity::Warning)
        .count();
    let actual_valid = error_count == 0;
    let check_ids = unique_check_ids(&diagnostics.checks);
    let check_id_set = check_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let missing_required_check_ids = case
        .expect
        .required_check_ids
        .iter()
        .filter(|check_id| !check_id_set.contains(check_id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let forbidden_check_ids_present = case
        .expect
        .forbidden_check_ids
        .iter()
        .filter(|check_id| check_id_set.contains(check_id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let expectation_passed = case.expect.valid == actual_valid
        && missing_required_check_ids.is_empty()
        && forbidden_check_ids_present.is_empty();

    Ok(ConformanceCaseResult {
        id: case.id.clone(),
        contract: case.contract.clone(),
        kind: case.kind,
        expected_valid: case.expect.valid,
        actual_valid,
        expectation_passed,
        error_count,
        warning_count,
        check_ids,
        missing_required_check_ids,
        forbidden_check_ids_present,
        repair_count: case.repair_count,
        human_correction: case.human_correction.clone(),
    })
}

fn score_core_packages(fixtures_dir: &Path, case: &ConformanceCase) -> Result<report::Diagnostics> {
    let mut value = if case.input.path.trim().is_empty() {
        json!({
            "knowledge": read_fixture_value(fixtures_dir, &case.input.knowledge)?,
            "evidence": read_fixture_value(fixtures_dir, &case.input.evidence)?,
            "pedagogy": read_fixture_value(fixtures_dir, &case.input.pedagogy)?,
        })
    } else {
        read_fixture_value(fixtures_dir, &case.input.path)?
    };
    apply_case_mutations(&mut value, &case.mutations)?;
    match serde_json::from_value::<core::CorePackages>(value) {
        Ok(packages) => Ok(core::validation::validate_core_packages(&packages).diagnostics),
        Err(err) => Ok(diagnostics_from_error(
            core::validation::CHECK_CORE_REQUIRED,
            format!("core package fixture did not deserialize: {err}"),
        )),
    }
}

fn score_report(fixtures_dir: &Path, case: &ConformanceCase) -> Result<report::Diagnostics> {
    let mut value = read_fixture_value(fixtures_dir, &case.input.path)?;
    apply_case_mutations(&mut value, &case.mutations)?;
    Ok(report::validate_report_value(&value).diagnostics)
}

fn score_work_order(fixtures_dir: &Path, case: &ConformanceCase) -> Result<report::Diagnostics> {
    let mut value = read_fixture_value(fixtures_dir, &case.input.path)?;
    apply_case_mutations(&mut value, &case.mutations)?;
    match serde_json::from_value::<work::WorkOrder>(value) {
        Ok(order) => Ok(work::validate_work_order(&order).diagnostics),
        Err(err) => Ok(diagnostics_from_error(
            work::validation::CHECK_WORK_REQUIRED,
            format!("work order fixture did not deserialize: {err}"),
        )),
    }
}

fn score_work_result(fixtures_dir: &Path, case: &ConformanceCase) -> Result<report::Diagnostics> {
    let order_value = read_fixture_value(fixtures_dir, &case.input.order)?;
    let order = match serde_json::from_value::<work::WorkOrder>(order_value) {
        Ok(order) => order,
        Err(err) => {
            return Ok(diagnostics_from_error(
                work::validation::CHECK_WORK_REQUIRED,
                format!("work order fixture did not deserialize: {err}"),
            ));
        }
    };
    let mut result_value = read_fixture_value(fixtures_dir, &case.input.result)?;
    apply_case_mutations(&mut result_value, &case.mutations)?;
    match serde_json::from_value::<work::WorkResult>(result_value) {
        Ok(result) => Ok(work::validate_work_result(&order, &result).diagnostics),
        Err(err) => Ok(diagnostics_from_error(
            work::validation::CHECK_WORK_REQUIRED,
            format!("work result fixture did not deserialize: {err}"),
        )),
    }
}

fn score_run_manifest(fixtures_dir: &Path, case: &ConformanceCase) -> Result<report::Diagnostics> {
    let mut value = read_fixture_value(fixtures_dir, &case.input.path)?;
    apply_case_mutations(&mut value, &case.mutations)?;
    let manifest = match serde_json::from_value::<run_manifest::RunManifest>(value) {
        Ok(manifest) => manifest,
        Err(err) => {
            return Ok(diagnostics_from_error(
                CHECK_EVAL_RUN_REQUIRED,
                format!("run manifest fixture did not deserialize: {err}"),
            ));
        }
    };
    match run_manifest::validate_run_manifest(&manifest) {
        Ok(()) => Ok(report::Diagnostics {
            summary: "run manifest validation produced 0 error(s) and 0 warning(s)".to_string(),
            checks: Vec::new(),
        }),
        Err(err) => {
            let message = err.to_string();
            let check_id = if message.contains("unsupported run manifest schema_version") {
                CHECK_EVAL_RUN_VERSION
            } else {
                CHECK_EVAL_RUN_REQUIRED
            };
            Ok(diagnostics_from_error(check_id, message))
        }
    }
}

fn score_id_inventory(case: &ConformanceCase) -> report::Diagnostics {
    let mut checks = Vec::new();
    let mut ids = case.input.ids.clone();
    for (index, identity) in case.input.identity_checks.iter().enumerate() {
        let parts = identity
            .parts
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let generated = report::content_id_identity(&identity.prefix, &parts);
        if !identity.expected_id.trim().is_empty() && generated.stable_id != identity.expected_id {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_EVAL_ID_IDENTITY,
                    format!(
                        "identity check {index} generated stable_id {:?}; expected {:?}",
                        generated.stable_id, identity.expected_id
                    ),
                )
                .with_target(format!("/input/identity_checks/{index}/expected_id"), ""),
            );
        }
        if !identity.expected_normalized_identity.trim().is_empty()
            && generated.normalized_identity != identity.expected_normalized_identity
        {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_EVAL_ID_IDENTITY,
                    format!(
                        "identity check {index} normalized identity {:?}; expected {:?}",
                        generated.normalized_identity, identity.expected_normalized_identity
                    ),
                )
                .with_target(
                    format!("/input/identity_checks/{index}/expected_normalized_identity"),
                    "",
                ),
            );
        }
        if identity.require_non_ascii_retention && generated.normalized_identity.is_ascii() {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_EVAL_MULTILINGUAL_RETENTION,
                    format!("identity check {index} lost non-ASCII content in normalized identity"),
                )
                .with_target(format!("/input/identity_checks/{index}"), ""),
            );
        }
        ids.push(generated.stable_id);
    }

    let mut seen = BTreeSet::new();
    for (index, id) in ids.iter().enumerate() {
        if !is_stable_id(id) {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_EVAL_ID_FORMAT,
                    format!("id inventory entry {index} is not ASCII-safe stable id: {id:?}"),
                )
                .with_target(format!("/input/ids/{index}"), id),
            );
        }
        if !seen.insert(id.as_str()) {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_EVAL_ID_DUPLICATE,
                    format!("duplicate stable id {id}"),
                )
                .with_target(format!("/input/ids/{index}"), id),
            );
        }
    }
    diagnostics_from_checks("id inventory", checks)
}

fn read_fixture_value(fixtures_dir: &Path, relative: &str) -> Result<Value> {
    if relative.trim().is_empty() {
        bail!("fixture path is required");
    }
    let path = resolve_fixture_path(fixtures_dir, relative);
    let data = fs::read(&path).with_context(|| format!("read fixture {}", path.display()))?;
    serde_json::from_slice(&data).with_context(|| format!("parse fixture {}", path.display()))
}

fn resolve_fixture_path(fixtures_dir: &Path, relative: &str) -> PathBuf {
    let path = Path::new(relative);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        fixtures_dir.join(path)
    }
}

fn apply_case_mutations(value: &mut Value, mutations: &[FixtureMutation]) -> Result<()> {
    for mutation in mutations {
        let path = mutation_path(mutation)?;
        match mutation.op {
            FixtureMutationOperation::Set => {
                set_json_pointer(value, &path, mutation.value.clone())
                    .with_context(|| format!("apply set mutation {path}"))?;
            }
            FixtureMutationOperation::Remove => {
                remove_json_pointer(value, &path)
                    .with_context(|| format!("apply remove mutation {path}"))?;
            }
        }
    }
    Ok(())
}

fn mutation_path(mutation: &FixtureMutation) -> Result<String> {
    if !mutation.path.starts_with('/') {
        bail!("mutation path {:?} must be a JSON pointer", mutation.path);
    }
    let prefix = match mutation.target {
        Some(FixtureMutationTarget::Knowledge) => "/knowledge",
        Some(FixtureMutationTarget::Evidence) => "/evidence",
        Some(FixtureMutationTarget::Pedagogy) => "/pedagogy",
        Some(FixtureMutationTarget::Report) | None => "",
    };
    Ok(format!("{prefix}{}", mutation.path))
}

fn set_json_pointer(value: &mut Value, pointer: &str, next: Value) -> Result<()> {
    let (parent, last) = pointer_parent_mut(value, pointer)?;
    match parent {
        Value::Object(object) => {
            object.insert(last, next);
            Ok(())
        }
        Value::Array(items) if last == "-" => {
            items.push(next);
            Ok(())
        }
        Value::Array(items) => {
            let index = last
                .parse::<usize>()
                .with_context(|| format!("array pointer segment {last:?} is not an index"))?;
            let slot = items
                .get_mut(index)
                .ok_or_else(|| anyhow!("array index {index} is out of bounds"))?;
            *slot = next;
            Ok(())
        }
        _ => bail!("JSON pointer parent is not an object or array"),
    }
}

fn remove_json_pointer(value: &mut Value, pointer: &str) -> Result<()> {
    let (parent, last) = pointer_parent_mut(value, pointer)?;
    match parent {
        Value::Object(object) => {
            object
                .remove(&last)
                .ok_or_else(|| anyhow!("object key {last:?} is missing"))?;
            Ok(())
        }
        Value::Array(items) => {
            let index = last
                .parse::<usize>()
                .with_context(|| format!("array pointer segment {last:?} is not an index"))?;
            if index >= items.len() {
                bail!("array index {index} is out of bounds");
            }
            items.remove(index);
            Ok(())
        }
        _ => bail!("JSON pointer parent is not an object or array"),
    }
}

fn pointer_parent_mut<'a>(value: &'a mut Value, pointer: &str) -> Result<(&'a mut Value, String)> {
    let segments = pointer_segments(pointer)?;
    let Some(last) = segments.last().cloned() else {
        bail!("cannot mutate the JSON document root");
    };
    let mut current = value;
    for segment in &segments[..segments.len() - 1] {
        match current {
            Value::Object(object) => {
                current = object
                    .get_mut(segment)
                    .ok_or_else(|| anyhow!("object key {segment:?} is missing"))?;
            }
            Value::Array(items) => {
                let index = segment.parse::<usize>().with_context(|| {
                    format!("array pointer segment {segment:?} is not an index")
                })?;
                current = items
                    .get_mut(index)
                    .ok_or_else(|| anyhow!("array index {index} is out of bounds"))?;
            }
            _ => bail!("JSON pointer segment {segment:?} does not address an object or array"),
        }
    }
    Ok((current, last))
}

fn pointer_segments(pointer: &str) -> Result<Vec<String>> {
    if pointer.is_empty() {
        return Ok(Vec::new());
    }
    if !pointer.starts_with('/') {
        bail!("JSON pointer {pointer:?} must start with /");
    }
    Ok(pointer[1..]
        .split('/')
        .map(unescape_pointer_segment)
        .collect())
}

fn unescape_pointer_segment(segment: &str) -> String {
    segment.replace("~1", "/").replace("~0", "~")
}

fn diagnostics_from_error(
    check_id: impl Into<String>,
    message: impl Into<String>,
) -> report::Diagnostics {
    diagnostics_from_checks(
        "conformance input",
        vec![report::DiagnosticCheck::error(check_id, message).with_target("/", "")],
    )
}

fn diagnostics_from_checks(
    label: &str,
    mut checks: Vec<report::DiagnosticCheck>,
) -> report::Diagnostics {
    checks.sort_by(|left, right| {
        severity_sort_key(left.severity)
            .cmp(&severity_sort_key(right.severity))
            .then_with(|| left.check_id.cmp(&right.check_id))
            .then_with(|| left.target_path.cmp(&right.target_path))
            .then_with(|| left.entity_id.cmp(&right.entity_id))
            .then_with(|| left.message.cmp(&right.message))
    });
    let error_count = checks
        .iter()
        .filter(|check| check.severity == report::DiagnosticSeverity::Error)
        .count();
    let warning_count = checks
        .iter()
        .filter(|check| check.severity == report::DiagnosticSeverity::Warning)
        .count();
    report::Diagnostics {
        summary: format!("{label} produced {error_count} error(s) and {warning_count} warning(s)"),
        checks,
    }
}

fn unique_check_ids(checks: &[report::DiagnosticCheck]) -> Vec<String> {
    checks
        .iter()
        .map(|check| check.check_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn ratio_string(numerator: usize, denominator: usize) -> String {
    format!("{numerator}/{denominator}")
}

fn is_stable_id(id: &str) -> bool {
    id.contains('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .chars()
            .next()
            .map(|first| first.is_ascii_lowercase())
            .unwrap_or(false)
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn severity_sort_key(severity: report::DiagnosticSeverity) -> u8 {
    match severity {
        report::DiagnosticSeverity::Error => 0,
        report::DiagnosticSeverity::Warning => 1,
        report::DiagnosticSeverity::Info => 2,
    }
}
