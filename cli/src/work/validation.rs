use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

use crate::report;

use super::capabilities::missing_capabilities;
use super::model::*;
use super::patch::{
    is_supported_patch_operation, is_supported_patch_path, patch_value_shape_error,
    path_allowed_by_prefixes, path_package_area,
};
use super::permissions::path_within_roots;
use super::{WORK_ORDER_SCHEMA_VERSION, WORK_PATCH_SCHEMA_VERSION, WORK_RESULT_SCHEMA_VERSION};

pub const CHECK_WORK_VERSION: &str = "work.version";
pub const CHECK_WORK_REQUIRED: &str = "work.required";
pub const CHECK_WORK_TASK_KIND: &str = "work.task_kind";
pub const CHECK_WORK_CAPABILITY: &str = "work.capability";
pub const CHECK_WORK_PERMISSION: &str = "work.permission";
pub const CHECK_WORK_PATCH: &str = "work.patch";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkValidationReport {
    pub valid: bool,
    pub diagnostics: report::Diagnostics,
}

impl WorkValidationReport {
    pub fn new(mut checks: Vec<report::DiagnosticCheck>) -> Self {
        sort_diagnostics(&mut checks);
        let error_count = checks
            .iter()
            .filter(|check| check.severity == report::DiagnosticSeverity::Error)
            .count();
        let warning_count = checks
            .iter()
            .filter(|check| check.severity == report::DiagnosticSeverity::Warning)
            .count();
        Self {
            valid: error_count == 0,
            diagnostics: report::Diagnostics {
                summary: format!(
                    "work validation produced {error_count} error(s) and {warning_count} warning(s)"
                ),
                checks,
            },
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.valid
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkJsonlValidationLine {
    pub line: usize,
    pub valid: bool,
    pub diagnostics: report::Diagnostics,
}

pub fn validate_work_order(order: &WorkOrder) -> WorkValidationReport {
    let mut checks = Vec::new();
    if order.schema_version != WORK_ORDER_SCHEMA_VERSION {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_VERSION,
                format!(
                    "unsupported work order schema_version {:?}; expected {}",
                    order.schema_version, WORK_ORDER_SCHEMA_VERSION
                ),
            )
            .with_target("/schema_version", &order.work_order_id),
        );
    }
    validate_stable_id(
        &order.work_order_id,
        "/work_order_id",
        &order.work_order_id,
        &mut checks,
    );
    require_non_empty(
        &order.objective,
        "/objective",
        &order.work_order_id,
        &mut checks,
    );
    validate_capability_set(
        &order.required_capabilities,
        "/required_capabilities",
        &order.work_order_id,
        &mut checks,
    );
    validate_allowed_patch_paths(order, &mut checks);
    validate_permission_envelope(order, &mut checks);
    WorkValidationReport::new(checks)
}

pub fn validate_work_result(order: &WorkOrder, result: &WorkResult) -> WorkValidationReport {
    let mut checks = validate_work_order(order).diagnostics.checks;
    if result.schema_version != WORK_RESULT_SCHEMA_VERSION {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_VERSION,
                format!(
                    "unsupported work result schema_version {:?}; expected {}",
                    result.schema_version, WORK_RESULT_SCHEMA_VERSION
                ),
            )
            .with_target("/schema_version", &result.result_id),
        );
    }
    validate_stable_id(
        &result.result_id,
        "/result_id",
        &result.result_id,
        &mut checks,
    );
    if result.work_order_id != order.work_order_id {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_REQUIRED,
                format!(
                    "work result references work_order_id {:?}; expected {:?}",
                    result.work_order_id, order.work_order_id
                ),
            )
            .with_target("/work_order_id", &result.result_id),
        );
    }
    if result.task_kind != order.task_kind {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_TASK_KIND,
                format!(
                    "work result task_kind {:?} does not match order task_kind {:?}",
                    work_task_kind_label(result.task_kind),
                    work_task_kind_label(order.task_kind)
                ),
            )
            .with_target("/task_kind", &result.result_id),
        );
    }
    validate_capability_set(
        &result.offered_capabilities,
        "/offered_capabilities",
        &result.result_id,
        &mut checks,
    );
    validate_capability_response(order, result, &mut checks);
    validate_result_status(result, &mut checks);
    for (index, patch) in result.patches.iter().enumerate() {
        validate_work_patch(order, patch, index, &result.result_id, &mut checks);
    }
    WorkValidationReport::new(checks)
}

pub fn validate_work_order_jsonl(input: &str) -> serde_json::Result<(String, bool)> {
    let mut output = String::new();
    let mut all_valid = true;
    for (index, line) in input.lines().enumerate() {
        let line_number = index + 1;
        if line.trim().is_empty() {
            continue;
        }
        let report = match serde_json::from_str::<WorkOrder>(line.trim()) {
            Ok(order) => validate_work_order(&order),
            Err(err) => WorkValidationReport::new(vec![parse_error_check(err, "/")]),
        };
        all_valid &= report.valid;
        let row = WorkJsonlValidationLine {
            line: line_number,
            valid: report.valid,
            diagnostics: report.diagnostics,
        };
        output.push_str(&serde_json::to_string(&row)?);
        output.push('\n');
    }
    Ok((output, all_valid))
}

pub fn validate_work_result_jsonl(
    order: &WorkOrder,
    input: &str,
) -> serde_json::Result<(String, bool)> {
    let mut output = String::new();
    let mut all_valid = true;
    for (index, line) in input.lines().enumerate() {
        let line_number = index + 1;
        if line.trim().is_empty() {
            continue;
        }
        let report = match serde_json::from_str::<WorkResult>(line.trim()) {
            Ok(result) => validate_work_result(order, &result),
            Err(err) => WorkValidationReport::new(vec![parse_error_check(err, "/")]),
        };
        all_valid &= report.valid;
        let row = WorkJsonlValidationLine {
            line: line_number,
            valid: report.valid,
            diagnostics: report.diagnostics,
        };
        output.push_str(&serde_json::to_string(&row)?);
        output.push('\n');
    }
    Ok((output, all_valid))
}

pub(crate) fn validate_work_patch(
    order: &WorkOrder,
    patch: &WorkPatch,
    index: usize,
    result_id: &str,
    checks: &mut Vec<report::DiagnosticCheck>,
) {
    let path = format!("/patches/{index}");
    if patch.schema_version != WORK_PATCH_SCHEMA_VERSION {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_VERSION,
                format!(
                    "unsupported work patch schema_version {:?}; expected {}",
                    patch.schema_version, WORK_PATCH_SCHEMA_VERSION
                ),
            )
            .with_target(format!("{path}/schema_version"), &patch.patch_id),
        );
    }
    validate_stable_id(
        &patch.patch_id,
        &format!("{path}/patch_id"),
        result_id,
        checks,
    );
    if !is_supported_patch_path(&patch.path) {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_PATCH,
                format!("unsupported work patch path {:?}", patch.path),
            )
            .with_target(format!("{path}/path"), &patch.patch_id),
        );
    }
    if is_supported_patch_path(&patch.path) && !is_supported_patch_operation(&patch.path, patch.op)
    {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_PATCH,
                format!(
                    "operation {:?} is not supported for patch path {:?}",
                    patch.op, patch.path
                ),
            )
            .with_target(format!("{path}/op"), &patch.patch_id),
        );
    }
    match path_package_area(&patch.path) {
        Some(area) if area == patch.target_package => {}
        Some(area) => checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_PATCH,
                format!(
                    "patch target_package {:?} does not match path package {:?}",
                    core_package_area_label(patch.target_package),
                    core_package_area_label(area)
                ),
            )
            .with_target(format!("{path}/target_package"), &patch.patch_id),
        ),
        None => checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_PATCH,
                format!("patch path {:?} does not target a core package", patch.path),
            )
            .with_target(format!("{path}/path"), &patch.patch_id),
        ),
    }
    if !path_allowed_by_prefixes(&patch.path, &order.allowed_patch_paths) {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_PATCH,
                format!(
                    "patch path {:?} is outside declared allowed_patch_paths",
                    patch.path
                ),
            )
            .with_target(format!("{path}/path"), &patch.patch_id),
        );
    }
    if !matches!(patch.op, PatchOperation::Remove) && patch.value == Value::Null {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_PATCH,
                "add and replace patches require a non-null value",
            )
            .with_target(format!("{path}/value"), &patch.patch_id),
        );
    }
    if is_supported_patch_path(&patch.path)
        && is_supported_patch_operation(&patch.path, patch.op)
        && patch.value != Value::Null
    {
        if let Some(message) = patch_value_shape_error(patch) {
            checks.push(
                report::DiagnosticCheck::error(CHECK_WORK_PATCH, message)
                    .with_target(format!("{path}/value"), &patch.patch_id),
            );
        }
    }
}

fn validate_capability_response(
    order: &WorkOrder,
    result: &WorkResult,
    checks: &mut Vec<report::DiagnosticCheck>,
) {
    let actual_missing =
        missing_capabilities(&order.required_capabilities, &result.offered_capabilities);
    let declared_missing = result
        .capability_response
        .missing_capabilities
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if actual_missing != declared_missing {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_CAPABILITY,
                format!(
                    "capability_response missing_capabilities {:?} do not match required/offered mismatch {:?}",
                    capability_labels(&declared_missing),
                    capability_labels(&actual_missing)
                ),
            )
            .with_target("/capability_response/missing_capabilities", &result.result_id),
        );
    }

    if actual_missing.is_empty() {
        if result.capability_response.status != CapabilityResolutionStatus::Accepted {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_WORK_CAPABILITY,
                    "capability_response must be accepted when no required capabilities are missing",
                )
                .with_target("/capability_response/status", &result.result_id),
            );
        }
        return;
    }

    match result.capability_response.status {
        CapabilityResolutionStatus::Accepted => checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_CAPABILITY,
                "capability mismatch cannot be accepted",
            )
            .with_target("/capability_response/status", &result.result_id),
        ),
        CapabilityResolutionStatus::Refusal => {
            if result.capability_response.rationale.trim().is_empty() {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_WORK_CAPABILITY,
                        "typed refusal requires a rationale",
                    )
                    .with_target("/capability_response/rationale", &result.result_id),
                );
            }
        }
        CapabilityResolutionStatus::DegradationPlan => {
            if result.capability_response.degradation_plan.is_empty() {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_WORK_CAPABILITY,
                        "degradation_plan status requires explicit degradation steps",
                    )
                    .with_target("/capability_response/degradation_plan", &result.result_id),
                );
            }
        }
    }
}

fn validate_result_status(result: &WorkResult, checks: &mut Vec<report::DiagnosticCheck>) {
    match result.status {
        WorkResultStatus::PatchProposal => {
            if result.capability_response.status != CapabilityResolutionStatus::Accepted {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_WORK_CAPABILITY,
                        "patch proposals require accepted capability negotiation",
                    )
                    .with_target("/status", &result.result_id),
                );
            }
            if result.patches.is_empty() {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_WORK_PATCH,
                        "patch proposal results require at least one proposed patch",
                    )
                    .with_target("/patches", &result.result_id),
                );
            }
        }
        WorkResultStatus::Refusal => {
            if result.capability_response.status != CapabilityResolutionStatus::Refusal {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_WORK_CAPABILITY,
                        "refusal results require a typed capability refusal",
                    )
                    .with_target("/status", &result.result_id),
                );
            }
            if !result.patches.is_empty() {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_WORK_PATCH,
                        "refusal results cannot include proposed patches",
                    )
                    .with_target("/patches", &result.result_id),
                );
            }
        }
        WorkResultStatus::DegradationPlan => {
            if result.capability_response.status != CapabilityResolutionStatus::DegradationPlan {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_WORK_CAPABILITY,
                        "degradation_plan results require a typed degradation plan",
                    )
                    .with_target("/status", &result.result_id),
                );
            }
            if !result.patches.is_empty() {
                checks.push(
                    report::DiagnosticCheck::error(
                        CHECK_WORK_PATCH,
                        "degradation plan results cannot include proposed patches",
                    )
                    .with_target("/patches", &result.result_id),
                );
            }
        }
    }
}

fn validate_allowed_patch_paths(order: &WorkOrder, checks: &mut Vec<report::DiagnosticCheck>) {
    if order.allowed_patch_paths.is_empty() {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_REQUIRED,
                "allowed_patch_paths must declare at least one core package path",
            )
            .with_target("/allowed_patch_paths", &order.work_order_id),
        );
    }
    for (index, path) in order.allowed_patch_paths.iter().enumerate() {
        if !is_supported_patch_path(path) {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_WORK_PATCH,
                    format!("unsupported allowed patch path {path:?}"),
                )
                .with_target(
                    format!("/allowed_patch_paths/{index}"),
                    &order.work_order_id,
                ),
            );
        }
    }
}

fn validate_permission_envelope(order: &WorkOrder, checks: &mut Vec<report::DiagnosticCheck>) {
    for (index, root) in order.permissions.read_roots.iter().enumerate() {
        require_non_empty(
            root,
            &format!("/permissions/read_roots/{index}"),
            &order.work_order_id,
            checks,
        );
    }
    for (index, root) in order.permissions.write_roots.iter().enumerate() {
        require_non_empty(
            root,
            &format!("/permissions/write_roots/{index}"),
            &order.work_order_id,
            checks,
        );
    }
    for (index, file) in order.input_files.iter().enumerate() {
        validate_file_ref(
            file,
            &format!("/input_files/{index}"),
            &order.work_order_id,
            checks,
        );
        match path_within_roots(Path::new(&file.path), &order.permissions.read_roots) {
            Ok(true) => {}
            Ok(false) => checks.push(
                report::DiagnosticCheck::error(
                    CHECK_WORK_PERMISSION,
                    format!("input file {} is outside declared read_roots", file.path),
                )
                .with_target(format!("/input_files/{index}/path"), &order.work_order_id),
            ),
            Err(err) => checks.push(
                report::DiagnosticCheck::error(
                    CHECK_WORK_PERMISSION,
                    format!("cannot validate input file root for {}: {err}", file.path),
                )
                .with_target(format!("/input_files/{index}/path"), &order.work_order_id),
            ),
        }
    }
    for (index, file) in order.output_files.iter().enumerate() {
        validate_file_ref(
            file,
            &format!("/output_files/{index}"),
            &order.work_order_id,
            checks,
        );
        match path_within_roots(Path::new(&file.path), &order.permissions.write_roots) {
            Ok(true) => {}
            Ok(false) => checks.push(
                report::DiagnosticCheck::error(
                    CHECK_WORK_PERMISSION,
                    format!("output file {} is outside declared write_roots", file.path),
                )
                .with_target(format!("/output_files/{index}/path"), &order.work_order_id),
            ),
            Err(err) => checks.push(
                report::DiagnosticCheck::error(
                    CHECK_WORK_PERMISSION,
                    format!("cannot validate output file root for {}: {err}", file.path),
                )
                .with_target(format!("/output_files/{index}/path"), &order.work_order_id),
            ),
        }
    }
}

fn validate_file_ref(
    file: &WorkFileRef,
    path: &str,
    entity_id: &str,
    checks: &mut Vec<report::DiagnosticCheck>,
) {
    require_non_empty(&file.kind, &format!("{path}/kind"), entity_id, checks);
    require_non_empty(&file.path, &format!("{path}/path"), entity_id, checks);
}

fn validate_capability_set(
    capabilities: &[WorkCapability],
    path: &str,
    entity_id: &str,
    checks: &mut Vec<report::DiagnosticCheck>,
) {
    let mut seen = BTreeSet::new();
    for capability in capabilities {
        if !seen.insert(*capability) {
            checks.push(
                report::DiagnosticCheck::error(
                    CHECK_WORK_CAPABILITY,
                    format!(
                        "duplicate capability {}",
                        work_capability_label(*capability)
                    ),
                )
                .with_target(path, entity_id),
            );
        }
    }
}

fn parse_error_check(err: serde_json::Error, path: &str) -> report::DiagnosticCheck {
    report::DiagnosticCheck::error(CHECK_WORK_REQUIRED, format!("parse error: {err}"))
        .with_target(path, "")
}

pub(crate) fn require_non_empty(
    value: &str,
    path: &str,
    entity_id: &str,
    checks: &mut Vec<report::DiagnosticCheck>,
) {
    if value.trim().is_empty() {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_REQUIRED,
                format!("{path} must be a non-empty string"),
            )
            .with_target(path, entity_id),
        );
    }
}

pub(crate) fn validate_stable_id(
    id: &str,
    path: &str,
    entity_id: &str,
    checks: &mut Vec<report::DiagnosticCheck>,
) {
    if !is_stable_id(id) {
        checks.push(
            report::DiagnosticCheck::error(
                CHECK_WORK_REQUIRED,
                format!("{path} is not a stable id: {id:?}"),
            )
            .with_target(path, entity_id),
        );
    }
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

fn capability_labels(capabilities: &[WorkCapability]) -> Vec<&'static str> {
    capabilities
        .iter()
        .map(|capability| work_capability_label(*capability))
        .collect()
}

fn sort_diagnostics(checks: &mut [report::DiagnosticCheck]) {
    checks.sort_by(|left, right| {
        severity_sort_key(left.severity)
            .cmp(&severity_sort_key(right.severity))
            .then_with(|| left.check_id.cmp(&right.check_id))
            .then_with(|| left.target_path.cmp(&right.target_path))
            .then_with(|| left.entity_id.cmp(&right.entity_id))
            .then_with(|| left.message.cmp(&right.message))
    });
}

fn severity_sort_key(severity: report::DiagnosticSeverity) -> u8 {
    match severity {
        report::DiagnosticSeverity::Error => 0,
        report::DiagnosticSeverity::Warning => 1,
        report::DiagnosticSeverity::Info => 2,
    }
}
