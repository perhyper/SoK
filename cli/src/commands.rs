//! Shared command plumbing; handlers are grouped by responsibility.
mod reports;
#[path = "commands/sources.rs"]
mod source_commands;
#[path = "commands/work.rs"]
mod work_commands;
mod workspace;

pub(crate) use reports::*;
pub(crate) use source_commands::*;
pub(crate) use work_commands::*;
pub(crate) use workspace::*;

use anyhow::{anyhow, bail, Context, Result};
use reqwest::blocking::Client;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::downloader::*;
use crate::eval;
use crate::report;
use crate::run_manifest::{self, RunFile, RunManifestSpec, SemanticPayload};
use crate::scaffold::*;
use crate::sources::*;
use crate::work;

fn read_stdin() -> Result<String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .context("read standard input")?;
    Ok(input)
}

fn ensure_distinct_paths(input: &str, output: &str) -> Result<()> {
    let input = work::permissions::resolved_normalized(Path::new(input))?;
    let output = work::permissions::resolved_normalized(Path::new(output))?;
    if input == output {
        bail!("--output must be a new file; refusing to mutate --input in place");
    }
    if fs::symlink_metadata(output).is_ok() {
        bail!("--output must be a new file; refusing to overwrite an existing path");
    }
    Ok(())
}

fn diagnostic_target_label(check: &report::DiagnosticCheck) -> String {
    match (
        check.target_path.trim().is_empty(),
        check.entity_id.trim().is_empty(),
    ) {
        (true, true) => "-".to_string(),
        (false, true) => check.target_path.clone(),
        (true, false) => check.entity_id.clone(),
        (false, false) => format!("{}#{}", check.target_path, check.entity_id),
    }
}

fn diagnostic_severity_label(severity: report::DiagnosticSeverity) -> &'static str {
    match severity {
        report::DiagnosticSeverity::Info => "info",
        report::DiagnosticSeverity::Warning => "warning",
        report::DiagnosticSeverity::Error => "error",
    }
}

fn run_file(kind: &str, path: impl Into<PathBuf>) -> RunFile {
    RunFile::new(kind, path)
}

fn input_file(kind: &str, path: &str) -> Vec<RunFile> {
    if path.trim().is_empty() {
        Vec::new()
    } else {
        vec![run_file(kind, path)]
    }
}

use input_file as output_file;

struct OptionalRunManifest<'a> {
    path: &'a str,
    command: &'a str,
    args: &'a [String],
    stage: &'a str,
    profile_version: String,
    started_at: String,
    inputs: Vec<RunFile>,
    outputs: Vec<RunFile>,
    semantic_payloads: Vec<SemanticPayload>,
    network: bool,
}

fn write_optional_run_manifest(request: OptionalRunManifest<'_>) -> Result<()> {
    if request.path.trim().is_empty() {
        return Ok(());
    }

    let mut spec = RunManifestSpec::new(request.command);
    spec.command_args = request.args.to_vec();
    spec.stage = request.stage.to_string();
    spec.profile_version = request.profile_version;
    spec.started_at = request.started_at;
    spec.finished_at = run_manifest::timestamp_now();
    spec.declared_permissions = run_manifest::declared_permissions_for_files(
        &request.inputs,
        &request.outputs,
        request.network,
    );
    spec.bounded_input_files = request.inputs;
    spec.output_files = request.outputs;
    spec.semantic_payloads = request.semantic_payloads;
    let manifest = run_manifest::build_run_manifest(spec)?;
    run_manifest::write_run_manifest(request.path, &manifest)?;
    println!("Wrote {}", request.path);
    Ok(())
}

fn export_stage_label(stage: report::ExportStage) -> &'static str {
    match stage {
        report::ExportStage::Scaffold => "scaffold",
        report::ExportStage::Final => "final",
    }
}

#[derive(Debug, Default)]
struct ParsedFlags {
    values: HashMap<String, String>,
    bools: HashSet<String>,
}

impl ParsedFlags {
    fn string(&self, key: &str, default: &str) -> String {
        self.values
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    fn int(&self, key: &str, default: i32) -> Result<i32> {
        match self.values.get(key) {
            Some(value) => value
                .parse::<i32>()
                .with_context(|| format!("invalid --{key} value {value:?}")),
            None => Ok(default),
        }
    }

    fn bool(&self, key: &str) -> bool {
        self.bools.contains(key)
    }
}

fn parse_flags(args: &[String], bool_flags: &[&str]) -> Result<ParsedFlags> {
    let bool_flags: HashSet<&str> = bool_flags.iter().copied().collect();
    let mut parsed = ParsedFlags::default();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "-h" || arg == "--help" {
            bail!("subcommand help is not implemented");
        }
        if !arg.starts_with('-') {
            bail!("unexpected argument {arg:?}");
        }

        let raw = arg.trim_start_matches('-');
        let (key, inline_value) = match raw.split_once('=') {
            Some((key, value)) => (key, Some(value.to_string())),
            None => (raw, None),
        };
        if key.is_empty() {
            bail!("invalid flag {arg:?}");
        }

        if bool_flags.contains(key) {
            let value = inline_value.unwrap_or_else(|| "true".to_string());
            if value == "true" {
                parsed.bools.insert(key.to_string());
            } else if value != "false" {
                bail!("invalid boolean value for --{key}: {value:?}");
            }
            index += 1;
            continue;
        }

        let value = match inline_value {
            Some(value) => value,
            None => {
                index += 1;
                args.get(index)
                    .cloned()
                    .ok_or_else(|| anyhow!("flag needs an argument: -{key}"))?
            }
        };
        parsed.values.insert(key.to_string(), value);
        index += 1;
    }
    Ok(parsed)
}
