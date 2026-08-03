//! Structured report conversion, validation, and rendering.

use crate::{load_sources, source_access, Source};
use anyhow::{bail, Context, Result};
use chrono::{Duration, NaiveDate, SecondsFormat, Utc};
use pulldown_cmark::{CowStr, Event, Options, Parser, Tag};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

mod html;
mod ids;
mod import;
mod model;
mod provenance;
mod relations;
mod validation;

pub use html::{render_html_report, render_html_report_file};
pub use ids::{
    build_id_migration, content_id, content_id_identity, content_id_map, normalize_alias_text,
    normalize_id_text, normalize_identity_text, IdIdentity, IdMigrationDocument, IdMigrationEntry,
};
pub use import::{
    export_markdown_report, read_json_file, read_jsonl_file, write_json_file, write_jsonl_file,
};
pub use model::*;
pub use provenance::{
    normalize_report_date, normalize_report_source, normalize_report_source_with_id,
    normalize_report_sources, normalize_source_manifest, normalize_source_roles,
    source_curricular_use, source_manifest_diagnostics, source_report_id,
};
pub use validation::{
    lint_markdown_report, validate_report_file, validate_report_value,
    CHECK_EVIDENCE_CATALOGED_ONLY, CHECK_EXPORT_AMBIGUOUS_REFERENCE,
    CHECK_EXPORT_AMBIGUOUS_SECTION, CHECK_EXPORT_CLAIM_NEEDS_EVIDENCE,
    CHECK_EXPORT_INTERNAL_SECTION_IN_FINAL, CHECK_EXPORT_MISSING_FIELD,
    CHECK_EXPORT_MISSING_PUBLIC_FIELD, CHECK_EXPORT_REPORT_ARCHITECTURE,
    CHECK_EXPORT_UNKNOWN_DIRECTIVE, CHECK_EXPORT_UNKNOWN_EVIDENCE_SOURCE,
    CHECK_EXPORT_UNKNOWN_SURFACE_MARKER, CHECK_EXPORT_UNRESOLVED_REFERENCE,
    CHECK_EXPORT_UNSUPPORTED_SECTION, CHECK_LINT_FINAL_PUBLIC_LEAKAGE,
    CHECK_LINT_SCAFFOLD_UNRESOLVED, CHECK_LINT_STRUCTURAL, CHECK_SOURCE_MISSING_ACCESS_ROUTE,
    CHECK_SOURCE_MISSING_ACCESS_STATUS, CHECK_SOURCE_MISSING_CURRICULAR_USE,
    CHECK_VALIDATE_CURRENTNESS_METADATA, CHECK_VALIDATE_CURRENTNESS_PROSE,
    CHECK_VALIDATE_CURRENTNESS_REVIEW_DUE, CHECK_VALIDATE_CURRENTNESS_SOURCE_DATE,
    CHECK_VALIDATE_CURRICULUM_REFERENCE, CHECK_VALIDATE_EVIDENCE_REQUIRED,
    CHECK_VALIDATE_EVIDENCE_SOURCE, CHECK_VALIDATE_EVIDENCE_SUPPORT,
    CHECK_VALIDATE_PUBLIC_BOUNDARY, CHECK_VALIDATE_RELATION_ENDPOINT, CHECK_VALIDATE_RELATION_KIND,
    CHECK_VALIDATE_SCHEMA_DESERIALIZE, CHECK_VALIDATE_SCHEMA_JSON, CHECK_VALIDATE_SCHEMA_REQUIRED,
    CHECK_VALIDATE_SOURCE_ACCESS, CHECK_VALIDATE_SOURCE_ROLE_CONDITIONAL,
    CHECK_VALIDATE_SOURCE_ROLE_REQUIRED, CHECK_VALIDATE_SOURCE_ROLE_WAIVER,
    CHECK_VALIDATE_STRUCTURE_REQUIRED, CHECK_VALIDATE_VISUAL_REFERENCE,
};
