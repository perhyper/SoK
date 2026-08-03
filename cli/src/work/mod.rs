//! Provider-neutral WorkOrder and WorkResult exchange for bounded SoK work.

pub mod capabilities;
pub mod model;
pub mod patch;
pub mod permissions;
pub mod validation;

pub use capabilities::{missing_capabilities, negotiate_capabilities, NegotiationMode};
pub use model::*;
pub use patch::accept_work_result_patches;
pub use validation::{
    validate_work_order, validate_work_order_jsonl, validate_work_result,
    validate_work_result_jsonl, WorkJsonlValidationLine, WorkValidationReport,
};

pub const WORK_ORDER_SCHEMA_VERSION: &str = "sok-work-order/v1";
pub const WORK_RESULT_SCHEMA_VERSION: &str = "sok-work-result/v1";
pub const WORK_PATCH_SCHEMA_VERSION: &str = "sok-work-patch/v1";
