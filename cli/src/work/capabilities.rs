use std::collections::BTreeSet;

use super::model::{
    work_capability_label, CapabilityResolutionStatus, CapabilityResponse, WorkCapability,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegotiationMode {
    Refusal,
    DegradationPlan,
}

pub fn missing_capabilities(
    required: &[WorkCapability],
    offered: &[WorkCapability],
) -> Vec<WorkCapability> {
    let offered = offered.iter().copied().collect::<BTreeSet<_>>();
    let mut missing = required
        .iter()
        .copied()
        .filter(|capability| !offered.contains(capability))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    missing.sort();
    missing
}

pub fn negotiate_capabilities(
    required: &[WorkCapability],
    offered: &[WorkCapability],
    mode: NegotiationMode,
) -> CapabilityResponse {
    let missing = missing_capabilities(required, offered);
    if missing.is_empty() {
        return CapabilityResponse::accepted();
    }

    let labels = missing
        .iter()
        .map(|capability| work_capability_label(*capability))
        .collect::<Vec<_>>();
    match mode {
        NegotiationMode::Refusal => CapabilityResponse {
            status: CapabilityResolutionStatus::Refusal,
            missing_capabilities: missing,
            degradation_plan: Vec::new(),
            rationale: format!("missing required capabilities: {}", labels.join(", ")),
        },
        NegotiationMode::DegradationPlan => CapabilityResponse {
            status: CapabilityResolutionStatus::DegradationPlan,
            missing_capabilities: missing,
            degradation_plan: labels
                .iter()
                .map(|label| format!("defer or narrow work requiring {label}"))
                .collect(),
            rationale: format!(
                "degrade deterministically because required capabilities are missing: {}",
                labels.join(", ")
            ),
        },
    }
}
