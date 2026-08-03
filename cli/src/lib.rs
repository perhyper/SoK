//! Core implementation for the Rust SoK CLI.

mod cli;
mod commands;
mod downloader;
mod profiles;
pub mod report;
mod scaffold;
mod sources;

pub use cli::{print_usage, run_cli, version_string};
pub use downloader::{
    common_extension, filename_from_headers, sanitize_filename, write_download_record,
    DownloadRecord,
};
pub use scaffold::{
    build_agent_brief, build_human_report_handoff, build_report_scaffold,
    build_specificity_checklist, build_tasks, infer_field_from_scaffold,
    infer_scaffold_table_value, infer_weeks_from_scaffold, require_field, today, validate_mode,
};
pub use sources::{
    audit_source, can_download, is_http_url, load_sources, normalize_access_status,
    parse_status_set, source_access, source_identity, source_url, write_sources_csv, AccessInfo,
    Source,
};

#[cfg(test)]
mod tests;
