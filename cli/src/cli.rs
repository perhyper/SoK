//! CLI dispatch and usage text.

use anyhow::{bail, Result};

use crate::commands::*;

pub fn version_string() -> String {
    format!("sok {} (rust)", env!("CARGO_PKG_VERSION"))
}

pub fn run_cli(args: Vec<String>) -> Result<i32> {
    if args.is_empty() {
        print_usage();
        return Ok(2);
    }

    if args
        .get(1)
        .is_some_and(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_command_usage(&args[0])?;
        return Ok(0);
    }

    match args[0].as_str() {
        "-h" | "--help" => {
            print_usage();
            Ok(0)
        }
        "help" => {
            if let Some(command) = args.get(1) {
                print_command_usage(command)?;
            } else {
                print_usage();
            }
            Ok(0)
        }
        "-V" | "--version" | "version" => {
            println!("{}", version_string());
            Ok(0)
        }
        "init" => run_init(&args[1..]).map(|_| 0),
        "brief" => run_brief(&args[1..]).map(|_| 0),
        "scaffold" => run_scaffold(&args[1..]).map(|_| 0),
        "handoff-report" => run_handoff_report(&args[1..]).map(|_| 0),
        "source-template" => run_source_template(&args[1..]).map(|_| 0),
        "audit-sources" => run_audit_sources(&args[1..]).map(|_| 0),
        "download-sources" => run_download_sources(&args[1..]).map(|_| 0),
        "ingest" => run_ingest(&args[1..]).map(|_| 0),
        "export-json" => run_export_json(&args[1..]).map(|_| 0),
        "lint" => run_lint(&args[1..]),
        "validate-report" => run_validate_report(&args[1..]),
        "migrate-ids" => run_migrate_ids(&args[1..]).map(|_| 0),
        "render-html" => run_render_html(&args[1..]).map(|_| 0),
        "specificity" => run_specificity(&args[1..]).map(|_| 0),
        "work" => run_work(&args[1..]),
        command => bail!("unknown command {command:?}"),
    }
}

pub fn print_usage() {
    println!(
        r#"SoK agent CLI

Usage:
  sok <command> [options]

Global options:
  -h, --help        Show this help.
  -V, --version     Show the CLI version and implementation.

Commands:
  init              Create an agent workspace.
  brief             Generate an agent handoff brief.
  scaffold          Generate a field-discovery and report-architecture canvas.
  handoff-report    Generate a next-agent brief for completing a human-reader report from a scaffold.
  source-template   Create a header-only source manifest CSV template.
  audit-sources     Audit source access metadata.
  download-sources  Download legally accessible open/free sources from a manifest.
  ingest last       Normalize a current source manifest into cataloged evidence JSONL; overwrites --output.
  export-json       Convert bounded Markdown plus source/evidence files to sok-report.json.
  lint              Check Markdown, source, and evidence inputs before JSON export.
  validate-report   Validate sok-report.json reliability gates.
  migrate-ids       Write a deterministic old-to-new stable ID migration map.
  render-html       Render a validated human_report JSON file to self-contained local HTML.
  specificity       Generate a concreteness checklist for an underspecified domain task.
  work              Validate and exchange bounded WorkOrder/WorkResult sidecars.

Run "sok <command> -h" for command options."#
    );
}

pub(crate) fn print_command_usage(command: &str) -> Result<()> {
    match command {
        "init" => print_init_usage(),
        "brief" => print_brief_usage(),
        "scaffold" => print_scaffold_usage(),
        "handoff-report" => print_handoff_report_usage(),
        "source-template" => print_source_template_usage(),
        "audit-sources" => print_audit_sources_usage(),
        "download-sources" => print_download_sources_usage(),
        "ingest" => print_ingest_usage(),
        "export-json" => print_export_json_usage(),
        "lint" => print_lint_usage(),
        "validate-report" => print_validate_report_usage(),
        "migrate-ids" => print_migrate_ids_usage(),
        "render-html" => print_render_html_usage(),
        "specificity" => print_specificity_usage(),
        "work" => print_work_usage(),
        command => bail!("unknown command {command:?}"),
    }
    Ok(())
}

fn print_init_usage() {
    println!(
        r#"Usage:
  sok init --field <field> --out <directory> [--learner <description>] [--goal <goal>] [--mode research|textbook|model-tuning|curriculum] [--weeks <number>] [--output-format <format>] [--run-manifest <sok-run.json>]

Creates an agent workspace with a brief, task list, report scaffold, header-only source manifest, and working directories."#
    );
}

fn print_brief_usage() {
    println!(
        r#"Usage:
  sok brief --field <field> [--learner <description>] [--goal <goal>] [--mode research|textbook|model-tuning|curriculum] [--weeks <number>] [--output-format <format>] [--output <brief.md>] [--run-manifest <sok-run.json>]

Writes the brief to --output or prints it to standard output."#
    );
}

fn print_scaffold_usage() {
    println!(
        r#"Usage:
  sok scaffold --field <field> [--learner <description>] [--goal <goal>] [--weeks <number>] [--output <scaffold.md>] [--run-manifest <sok-run.json>]

Writes a provisional source-review and report-architecture canvas to --output or prints it to standard output."#
    );
}

fn print_handoff_report_usage() {
    println!(
        r#"Usage:
  sok handoff-report --scaffold <scaffold.md> [--field <field>] [--learner <description>] [--goal <goal>] [--weeks <number>] [--output <handoff.md>] [--run-manifest <sok-run.json>]
  sok handoff-report --field <field> [--learner <description>] [--goal <goal>] [--weeks <number>] [--output <handoff.md>] [--run-manifest <sok-run.json>]

Uses the supplied scaffold when present; otherwise creates one from --field. Writes to --output or standard output."#
    );
}

fn print_source_template_usage() {
    println!(
        r#"Usage:
  sok source-template --output <sources.csv> [--run-manifest <sok-run.json>]

Creates a header-only source manifest. Add real source rows before ingesting it; the template never inserts placeholder evidence."#
    );
}

fn print_audit_sources_usage() {
    println!(
        r#"Usage:
  sok audit-sources --manifest <sources.csv|sources.tsv|sources.json> [--strict]

Checks source identity, access, and source-role metadata. --strict returns an error when problems are found."#
    );
}

fn print_download_sources_usage() {
    println!(
        r#"Usage:
  sok download-sources --manifest <sources.csv|sources.tsv|sources.json> --out-dir <directory> [--allow-status <comma-separated-statuses>] [--include-unknown] [--dry-run] [--max-mb <number>] [--timeout <seconds>] [--run-manifest <sok-run.json>]

Downloads only sources allowed by access status. Inspect a --dry-run before permitting network downloads."#
    );
}

fn print_specificity_usage() {
    println!(
        r#"Usage:
  sok specificity --field <field> [--output <checklist.md>] [--run-manifest <sok-run.json>]

Writes a domain-task specificity checklist to --output or prints it to standard output."#
    );
}
