//! Shared identity and diagnostic ordering rules.

use crate::report::{DiagnosticCheck, DiagnosticSeverity};

pub(crate) fn is_stable_id(id: &str) -> bool {
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

pub(crate) fn sort_diagnostics(checks: &mut [DiagnosticCheck]) {
    checks.sort_by(|left, right| {
        severity_sort_key(left.severity)
            .cmp(&severity_sort_key(right.severity))
            .then_with(|| left.check_id.cmp(&right.check_id))
            .then_with(|| left.target_path.cmp(&right.target_path))
            .then_with(|| left.entity_id.cmp(&right.entity_id))
            .then_with(|| left.message.cmp(&right.message))
    });
}

pub(crate) fn severity_sort_key(severity: DiagnosticSeverity) -> u8 {
    match severity {
        DiagnosticSeverity::Error => 0,
        DiagnosticSeverity::Warning => 1,
        DiagnosticSeverity::Info => 2,
    }
}
