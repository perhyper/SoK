//! Private run sidecar manifests for reproducibility metadata.

use anyhow::{bail, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{profiles, report};

pub const RUN_SCHEMA_VERSION: &str = "sok-run/v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunManifest {
    pub schema_version: String,
    pub run_id: String,
    pub sok_revision: RevisionInfo,
    pub skill_revision: RevisionInfo,
    pub profile_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor: Option<ExecutorMetadata>,
    pub command: RunCommand,
    pub stage: String,
    pub declared_permissions: DeclaredPermissions,
    pub bounded_input_files: Vec<FileHash>,
    pub work_order_hashes: Vec<ContentHash>,
    pub work_result_hashes: Vec<ContentHash>,
    pub output_files: Vec<FileHash>,
    pub semantic_digest: SemanticDigest,
    pub timestamps: RunTimestamps,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevisionInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutorMetadata {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub executor_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub executor_kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub executor_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunCommand {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclaredPermissions {
    pub network: bool,
    pub subprocess: bool,
    pub read_roots: Vec<String>,
    pub write_roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileHash {
    pub kind: String,
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentHash {
    pub kind: String,
    pub label: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticDigest {
    pub algorithm: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunTimestamps {
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFile {
    pub kind: String,
    pub path: PathBuf,
}

impl RunFile {
    pub fn new(kind: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            kind: kind.into(),
            path: path.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticPayload {
    pub kind: String,
    pub label: String,
    pub sha256: String,
}

impl SemanticPayload {
    pub fn from_text(kind: impl Into<String>, label: impl Into<String>, text: &str) -> Self {
        Self {
            kind: kind.into(),
            label: label.into(),
            sha256: sha256_hex(text.as_bytes()),
        }
    }

    pub fn from_json_value(
        kind: impl Into<String>,
        label: impl Into<String>,
        value: &Value,
    ) -> Result<Self> {
        let mut semantic = value.clone();
        strip_report_timestamp_metadata(&mut semantic);
        let data = serde_json::to_vec(&semantic).context("encode semantic JSON payload")?;
        Ok(Self {
            kind: kind.into(),
            label: label.into(),
            sha256: sha256_hex(&data),
        })
    }

    pub fn from_report_document(
        label: impl Into<String>,
        document: &report::ReportDocument,
    ) -> Result<Self> {
        Self::from_json_value("sok-report", label, &serde_json::to_value(document)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunManifestSpec {
    pub command_name: String,
    pub command_args: Vec<String>,
    pub stage: String,
    pub profile_version: String,
    pub started_at: String,
    pub finished_at: String,
    pub declared_permissions: DeclaredPermissions,
    pub bounded_input_files: Vec<RunFile>,
    pub work_order_files: Vec<RunFile>,
    pub work_result_files: Vec<RunFile>,
    pub output_files: Vec<RunFile>,
    pub semantic_payloads: Vec<SemanticPayload>,
}

impl RunManifestSpec {
    pub fn new(command_name: impl Into<String>) -> Self {
        Self {
            command_name: command_name.into(),
            command_args: Vec::new(),
            stage: String::new(),
            profile_version: "unknown".to_string(),
            started_at: timestamp_now(),
            finished_at: timestamp_now(),
            declared_permissions: DeclaredPermissions::default(),
            bounded_input_files: Vec::new(),
            work_order_files: Vec::new(),
            work_result_files: Vec::new(),
            output_files: Vec::new(),
            semantic_payloads: Vec::new(),
        }
    }
}

pub fn timestamp_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn default_profile_version_for_field(field: &str) -> String {
    profiles::infer_discovery_profile(field).profile_version
}

pub fn profile_version_from_report(document: &report::ReportDocument) -> String {
    document
        .internal_context
        .as_ref()
        .and_then(|context| {
            context
                .profile_proposals
                .iter()
                .find_map(|proposal| non_empty(&proposal.profile_version))
        })
        .unwrap_or_else(|| default_profile_version_for_field(&document.report.field))
}

pub fn hash_file(file: &RunFile) -> Result<FileHash> {
    let data = fs::read(&file.path).with_context(|| format!("read {}", file.path.display()))?;
    Ok(FileHash {
        kind: file.kind.clone(),
        path: display_path(&file.path),
        sha256: sha256_hex(&data),
        bytes: data.len() as u64,
    })
}

pub fn build_run_manifest(spec: RunManifestSpec) -> Result<RunManifest> {
    let bounded_input_files = spec
        .bounded_input_files
        .iter()
        .map(hash_file)
        .collect::<Result<Vec<_>>>()?;
    let work_order_hashes = spec
        .work_order_files
        .iter()
        .map(content_hash_file)
        .collect::<Result<Vec<_>>>()?;
    let work_result_hashes = spec
        .work_result_files
        .iter()
        .map(content_hash_file)
        .collect::<Result<Vec<_>>>()?;
    let output_files = spec
        .output_files
        .iter()
        .map(hash_file)
        .collect::<Result<Vec<_>>>()?;

    let semantic_digest = semantic_digest_for_run(SemanticDigestInput {
        command_name: &spec.command_name,
        stage: &spec.stage,
        profile_version: &spec.profile_version,
        permissions: &spec.declared_permissions,
        bounded_input_files: &bounded_input_files,
        work_order_hashes: &work_order_hashes,
        work_result_hashes: &work_result_hashes,
        output_files: &output_files,
        semantic_payloads: &spec.semantic_payloads,
    })?;
    let run_id = run_id_for_run(
        &spec.command_name,
        &spec.stage,
        &spec.started_at,
        &spec.finished_at,
        &semantic_digest.value,
    )?;
    let manifest = RunManifest {
        schema_version: RUN_SCHEMA_VERSION.to_string(),
        run_id,
        sok_revision: RevisionInfo {
            name: "sok-cli-rust".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        skill_revision: RevisionInfo {
            name: "structure-of-knowledge".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        profile_version: spec.profile_version,
        executor: None,
        command: RunCommand {
            name: spec.command_name,
            args: spec.command_args,
        },
        stage: spec.stage,
        declared_permissions: spec.declared_permissions,
        bounded_input_files,
        work_order_hashes,
        work_result_hashes,
        output_files,
        semantic_digest,
        timestamps: RunTimestamps {
            started_at: spec.started_at,
            finished_at: spec.finished_at,
        },
    };
    validate_run_manifest(&manifest)?;
    Ok(manifest)
}

pub fn write_run_manifest(path: impl AsRef<Path>, manifest: &RunManifest) -> Result<()> {
    validate_run_manifest(manifest)?;
    let path = path.as_ref();
    ensure_parent_dir(path)?;
    let mut data = serde_json::to_vec_pretty(manifest)
        .with_context(|| format!("encode run manifest {}", path.display()))?;
    data.push(b'\n');
    fs::write(path, data).with_context(|| format!("write run manifest {}", path.display()))?;
    Ok(())
}

pub fn read_run_manifest(path: impl AsRef<Path>) -> Result<RunManifest> {
    let path = path.as_ref();
    let data = fs::read(path).with_context(|| format!("read run manifest {}", path.display()))?;
    let manifest: RunManifest = serde_json::from_slice(&data)
        .with_context(|| format!("parse run manifest {}", path.display()))?;
    validate_run_manifest(&manifest)?;
    Ok(manifest)
}

pub fn validate_run_manifest(manifest: &RunManifest) -> Result<()> {
    if manifest.schema_version != RUN_SCHEMA_VERSION {
        bail!(
            "unsupported run manifest schema_version {:?}; expected {}",
            manifest.schema_version,
            RUN_SCHEMA_VERSION
        );
    }
    require_run_id(&manifest.run_id)?;
    require_non_empty("sok_revision.name", &manifest.sok_revision.name)?;
    require_non_empty("sok_revision.version", &manifest.sok_revision.version)?;
    require_non_empty("skill_revision.name", &manifest.skill_revision.name)?;
    require_non_empty("skill_revision.version", &manifest.skill_revision.version)?;
    require_non_empty("profile_version", &manifest.profile_version)?;
    require_non_empty("command.name", &manifest.command.name)?;
    require_non_empty("stage", &manifest.stage)?;
    require_non_empty(
        "semantic_digest.algorithm",
        &manifest.semantic_digest.algorithm,
    )?;
    if manifest.semantic_digest.algorithm != "sha256" {
        bail!(
            "unsupported semantic digest algorithm {:?}",
            manifest.semantic_digest.algorithm
        );
    }
    require_sha256("semantic_digest.value", &manifest.semantic_digest.value)?;
    require_non_empty("timestamps.started_at", &manifest.timestamps.started_at)?;
    require_non_empty("timestamps.finished_at", &manifest.timestamps.finished_at)?;

    for (index, file) in manifest.bounded_input_files.iter().enumerate() {
        validate_file_hash("bounded_input_files", index, file)?;
    }
    for (index, file) in manifest.output_files.iter().enumerate() {
        validate_file_hash("output_files", index, file)?;
    }
    for (index, hash) in manifest.work_order_hashes.iter().enumerate() {
        validate_content_hash("work_order_hashes", index, hash)?;
    }
    for (index, hash) in manifest.work_result_hashes.iter().enumerate() {
        validate_content_hash("work_result_hashes", index, hash)?;
    }
    Ok(())
}

pub fn declared_permissions_for_files(
    inputs: &[RunFile],
    outputs: &[RunFile],
    network: bool,
) -> DeclaredPermissions {
    DeclaredPermissions {
        network,
        subprocess: false,
        read_roots: parent_roots(inputs),
        write_roots: parent_roots(outputs),
    }
}

fn content_hash_file(file: &RunFile) -> Result<ContentHash> {
    let hashed = hash_file(file)?;
    Ok(ContentHash {
        kind: hashed.kind,
        label: hashed.path,
        sha256: hashed.sha256,
    })
}

struct SemanticDigestInput<'a> {
    command_name: &'a str,
    stage: &'a str,
    profile_version: &'a str,
    permissions: &'a DeclaredPermissions,
    bounded_input_files: &'a [FileHash],
    work_order_hashes: &'a [ContentHash],
    work_result_hashes: &'a [ContentHash],
    output_files: &'a [FileHash],
    semantic_payloads: &'a [SemanticPayload],
}

fn semantic_digest_for_run(input: SemanticDigestInput<'_>) -> Result<SemanticDigest> {
    let semantic_output_hashes = if input.semantic_payloads.is_empty() {
        input
            .output_files
            .iter()
            .map(|file| {
                json!({
                    "kind": file.kind,
                    "sha256": file.sha256,
                })
            })
            .collect::<Vec<_>>()
    } else {
        input
            .semantic_payloads
            .iter()
            .map(|payload| {
                json!({
                    "kind": payload.kind,
                    "label": payload.label,
                    "sha256": payload.sha256,
                })
            })
            .collect::<Vec<_>>()
    };
    let value = json!({
        "command": input.command_name,
        "stage": input.stage,
        "profile_version": input.profile_version,
        "declared_permissions": {
            "network": input.permissions.network,
            "subprocess": input.permissions.subprocess,
        },
        "bounded_input_file_hashes": file_digest_items(input.bounded_input_files),
        "work_order_hashes": content_digest_items(input.work_order_hashes),
        "work_result_hashes": content_digest_items(input.work_result_hashes),
        "semantic_output_hashes": semantic_output_hashes,
    });
    let data = serde_json::to_vec(&value).context("encode semantic run digest input")?;
    Ok(SemanticDigest {
        algorithm: "sha256".to_string(),
        value: sha256_hex(&data),
    })
}

fn file_digest_items(files: &[FileHash]) -> Vec<Value> {
    files
        .iter()
        .map(|file| {
            json!({
                "kind": file.kind,
                "sha256": file.sha256,
                "bytes": file.bytes,
            })
        })
        .collect()
}

fn content_digest_items(hashes: &[ContentHash]) -> Vec<Value> {
    hashes
        .iter()
        .map(|hash| {
            json!({
                "kind": hash.kind,
                "sha256": hash.sha256,
            })
        })
        .collect()
}

fn run_id_for_run(
    command_name: &str,
    stage: &str,
    started_at: &str,
    finished_at: &str,
    semantic_digest: &str,
) -> Result<String> {
    let seed = json!({
        "command": command_name,
        "stage": stage,
        "started_at": started_at,
        "finished_at": finished_at,
        "semantic_digest": semantic_digest,
    });
    let data = serde_json::to_vec(&seed).context("encode run id seed")?;
    Ok(format!("run-{}", &sha256_hex(&data)[..16]))
}

fn strip_report_timestamp_metadata(value: &mut Value) {
    if let Some(metadata) = value.get_mut("metadata").and_then(Value::as_object_mut) {
        metadata.remove("generated_at");
        if let Some(temporal) = metadata
            .get_mut("temporal_review")
            .and_then(Value::as_object_mut)
        {
            temporal.remove("as_of");
            temporal.remove("review_after");
        }
    }
}

fn validate_file_hash(collection: &str, index: usize, file: &FileHash) -> Result<()> {
    require_non_empty(&format!("{collection}[{index}].kind"), &file.kind)?;
    require_non_empty(&format!("{collection}[{index}].path"), &file.path)?;
    require_sha256(&format!("{collection}[{index}].sha256"), &file.sha256)?;
    Ok(())
}

fn validate_content_hash(collection: &str, index: usize, hash: &ContentHash) -> Result<()> {
    require_non_empty(&format!("{collection}[{index}].kind"), &hash.kind)?;
    require_non_empty(&format!("{collection}[{index}].label"), &hash.label)?;
    require_sha256(&format!("{collection}[{index}].sha256"), &hash.sha256)?;
    Ok(())
}

fn require_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("run manifest {field} must not be empty");
    }
    Ok(())
}

fn require_run_id(value: &str) -> Result<()> {
    let suffix = value.strip_prefix("run-").unwrap_or_default();
    if suffix.len() == 16 && suffix.bytes().all(is_lower_hex) {
        Ok(())
    } else {
        bail!("run manifest run_id must match run-[0-9a-f]{{16}}")
    }
}

fn require_sha256(field: &str, value: &str) -> Result<()> {
    if value.len() == 64 && value.bytes().all(is_lower_hex) {
        Ok(())
    } else {
        bail!("run manifest {field} must be a full SHA-256 hex digest")
    }
}

fn is_lower_hex(byte: u8) -> bool {
    byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn display_path(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| absolutize(path))
        .display()
        .to_string()
}

fn parent_roots(files: &[RunFile]) -> Vec<String> {
    let mut roots = BTreeSet::new();
    for file in files {
        if let Some(parent) = file.path.parent() {
            roots.insert(display_path(parent));
        }
    }
    roots.into_iter().collect()
}

fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
    }
    Ok(())
}
