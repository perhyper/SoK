//! Scaffold, brief, handoff, and task text generation.

use anyhow::{bail, Context, Result};
use chrono::Local;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::profiles;

pub(crate) const SCHEMA_VERSION: &str = "sok-agent-pack-v0.3";

#[derive(Debug, Serialize)]
pub(crate) struct AgentPack {
    pub(crate) field: String,
    pub(crate) learner: String,
    pub(crate) goal: String,
    pub(crate) mode: String,
    #[serde(skip_serializing_if = "is_zero")]
    pub(crate) weeks: i32,
    pub(crate) created: String,
    pub(crate) schema: String,
}

pub(crate) fn is_zero(value: &i32) -> bool {
    *value == 0
}

pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

pub fn require_field(field: &str) -> Result<()> {
    if field.trim().is_empty() {
        bail!("--field is required");
    }
    Ok(())
}

pub fn validate_mode(mode: &str) -> Result<()> {
    if mode_descriptions().contains_key(mode) {
        return Ok(());
    }
    let modes = mode_descriptions().keys().copied().collect::<Vec<_>>();
    bail!(
        "invalid --mode {mode:?}; choose one of {}",
        modes.join(", ")
    );
}

pub(crate) fn mode_descriptions() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (
            "curriculum",
            "Design an evidence-grounded learning sequence and assessment path.",
        ),
        (
            "model-tuning",
            "Plan a legally usable domain corpus and evaluation set for model adaptation.",
        ),
        (
            "research",
            "Produce a field-structured research report with only the evidence modules the goal requires.",
        ),
        (
            "textbook",
            "Plan a textbook or long-form course artifact from the field structure.",
        ),
    ])
}

pub(crate) fn write_text<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
    }
    fs::write(path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub(crate) fn absolute_path(path: &str) -> Result<PathBuf> {
    let raw = PathBuf::from(path);
    let joined = if raw.is_absolute() {
        raw
    } else {
        std::env::current_dir()?.join(raw)
    };
    Ok(clean_path(&joined))
}

pub(crate) fn clean_path(path: &Path) -> PathBuf {
    let mut cleaned = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                cleaned.pop();
            }
            _ => cleaned.push(component.as_os_str()),
        }
    }
    cleaned
}

pub fn infer_field_from_scaffold(scaffold: &str) -> String {
    for raw_line in scaffold.lines() {
        let line = raw_line.trim();
        if let Some(value) = line.strip_prefix("# Structure of Knowledge:") {
            return value.trim().to_string();
        }
        if line.starts_with("# SoK") && line.contains(':') {
            if let Some((_, value)) = line.split_once(':') {
                if !value.trim().is_empty() {
                    return value.trim().to_string();
                }
            }
        }
    }
    infer_scaffold_table_value(scaffold, &["Field"])
}

pub fn infer_scaffold_table_value(scaffold: &str, labels: &[&str]) -> String {
    for raw_line in scaffold.lines() {
        let line = raw_line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<_> = line.trim_matches('|').split('|').collect();
        if cells.len() < 2 {
            continue;
        }
        let key = cells[0].trim();
        let value = cells[1].trim();
        if !value.is_empty() && labels.iter().any(|label| key.eq_ignore_ascii_case(label)) {
            return value.to_string();
        }
    }
    String::new()
}

pub fn infer_weeks_from_scaffold(scaffold: &str) -> i32 {
    let value = infer_scaffold_table_value(scaffold, &["Time budget"]);
    value
        .trim()
        .strip_suffix(" weeks")
        .and_then(|weeks| weeks.parse::<i32>().ok())
        .unwrap_or(0)
}

pub fn build_human_report_handoff(
    field: &str,
    learner: &str,
    goal: &str,
    weeks: i32,
    scaffold: &str,
) -> String {
    let duration = if weeks > 0 {
        format!("{weeks} weeks")
    } else {
        "not fixed".to_string()
    };
    format!(
        r#"# SoK Human-Report Handoff: {field}

Generated: {generated}

## Purpose

Use the scaffold below as internal planning material for a next agent. Complete it into a polished, source-grounded deep research report for a human reader.

## Internal Context

- Field: {field}
- Learner profile: {learner}
- Original goal: {goal}
- Time budget: {duration}

Do not render this internal context as a front-matter table in the final report. Use it only to calibrate depth, scope, source selection, and sequencing.

## Visibility Rules

- Do not include sections named `Research Frame`, `Scaffold Quality Notes`, `SoK Usefulness Notes`, or raw prompt-intent notes in the human-facing report.
- Do not expose the learner profile, original goal, prompt wording, or time budget unless the user explicitly asks for a curriculum plan where that context belongs.
- Convert `Scoped assumption` into a concise public scope note only when it affects interpretation.
- Replace scaffold placeholders such as `source to verify`, `date after lookup`, and generic source rows with researched citations or remove them.
- Keep access metadata and verification limits, but move them into source notes or a short final limitations section.

## Human-Reader Report Contract

1. Review sources before confirming the report's shape. Treat the scaffold's profile, lenses, and any architecture decision as hypotheses to preserve, revise, or reject against the evidence.
2. Inventory the field-native elements that actually carry its knowledge: questions, objects, cases, representations, methods, warrants, institutions, instruments, data, disputes, or other forms found in the sources.
3. Identify the few load-bearing relations among those elements and distinguish core ideas from supporting context.
4. Compare plausible organizing forms against those findings and the reader's goal, then confirm or revise the scaffold's decision: for example a dependency graph, causal system, process, multiscale model, debate network, case constellation, genealogy, chronology, or a justified hybrid.
5. Record the final `Report Architecture` decision with a thesis, organizing form, and rationale. Then write ordered, field-specific `##` sections; do not reuse the scaffold's working sections as the final table of contents.
6. Keep machine-verifiable Claims, Relations, Literature Ladder, Curriculum Roadmap, and Visual Views surfaces only when they serve this run. A report does not need a public chapter for every structured surface.
7. Date current claims, preserve evidence limits, and select only relation-backed visuals that clarify the chosen architecture.

## Quality Gate

The final report should read like a field-specific deep research briefing, not a filled worksheet. Its narrative sequence must follow the discovered structure of the field. Structured evidence surfaces support validation; they do not dictate the public table of contents.

## Input Scaffold

````markdown
{scaffold}
````
"#,
        generated = today()
    )
}

pub fn build_report_scaffold(field: &str, learner: &str, goal: &str, weeks: i32) -> String {
    let duration = if weeks > 0 {
        format!("{weeks} weeks")
    } else {
        "unspecified duration".to_string()
    };
    let profile = profiles::infer_discovery_profile(field);
    format!(
        r#"# SoK Working Scaffold: {field}

## Research Frame

| Item | Value |
|---|---|
| Field | {field} |
| Learner | {learner} |
| Goal | {goal} |
| Time budget | {duration} |
| Provisional lens | {classification_hint} |
| Why only provisional | {why_this_hint} |
| Profile proposal | {profile_id} |
| Profile proposer type | inference |
| Profile locale | {profile_locale} |
| Profile version | {profile_version} |
| Profile confidence | {profile_confidence} |
| Profile rationale | {profile_rationale} |

## Discovery Rule

Do not draft the final table of contents yet. Review boundary, foundation, and method sources first; then replace or reject the provisional lenses below. The field name is only a search hint and must not determine the final report architecture.

## Candidate Lenses to Test

| Lens | Inspect in sources | Revise or reject when | Status |
|---|---|---|---|
{lenses}

## Questions for Field Discovery

{questions}

## Field Element Inventory

Create element classes from what the sources actually reveal. Questions, objects, cases, representations, methods, warrants, institutions, instruments, datasets, practices, and disputes are prompts, not required slots. Do not add a row merely to cover a generic category.

| Element class | Observed element | Actual form in this field | Role: core / surrounding / context | Load-bearing relations | Source IDs | Confidence |
|---|---|---|---|---|---|---|

## Source Role Probe

Source roles are diagnostic, not quotas. Decide each role after the first source pass and record a reason for every waiver.

| Source role | Inspect | Decision rule | Decision and rationale |
|---|---|---|---|
{source_roles}

## Organizing Form Comparison

Add at least two plausible forms suggested by the reviewed element inventory. Compare their explanatory gain and distortion risk; invent a field-specific or hybrid form when standard graph, process, case, debate, taxonomy, scale, or historical forms do not fit.

| Candidate form | Use when the load-bearing relation is | What it foregrounds | Main distortion risk | Evidence from this run | Decision |
|---|---|---|---|---|---|

## Report Architecture Decision

Complete this only after the element inventory and organizing-form comparison are source-grounded.

| Item | Decision |
|---|---|
| Executive thesis |  |
| Primary reader question |  |
| Chosen organizing form |  |
| Architecture rationale |  |
| Core elements to foreground |  |
| Important surrounding elements |  |
| Narrative sequence and section purposes |  |
| Visual logic and view IDs |  |
| Rejected alternatives and why |  |
| Evidence or scope limits |  |

## Structured Export Workspace

Claims, relations, reading ladders, curricula, and visual declarations are machine-verifiable support surfaces, not a mandatory public chapter sequence. Add only the surfaces this run needs, using the contracts in `structure-of-knowledge/references/output-contract.md`.

## Scaffold Quality Notes

- The field name selected a provisional discovery lens only; source findings must control the final structure.
- Profile proposal metadata is advisory until source review revises or confirms it.
- A completed run identifies core and surrounding elements, compares organizing forms, and records why the chosen report architecture fits.
- Existing curricula are evidence of pedagogical consensus, not the boundary or automatic table of contents of the field.
"#,
        classification_hint = profile.classification_hint.as_str(),
        why_this_hint = profile.why_this_hint.as_str(),
        profile_id = profile.profile_id.as_str(),
        profile_locale = profile.locale.as_str(),
        profile_version = profile.profile_version.as_str(),
        profile_confidence = profile.inference_confidence(),
        profile_rationale = profile.inference_rationale(),
        lenses = profile.render_lenses(),
        questions = profile.render_questions(),
        source_roles = profile.render_source_roles()
    )
}

pub(crate) fn mode_deliverables(mode: &str) -> Vec<&'static str> {
    let mut common = vec![
        "Field boundary and scoped assumptions",
        "Field-native element inventory with actual forms",
        "Load-bearing relations and core-versus-surrounding distinction",
        "Source manifest with access metadata",
        "Role-based evidence plan",
        "Compared organizing forms and report-architecture decision",
        "Quality-gate self-audit",
    ];
    match mode {
        "textbook" => common.extend([
            "Textbook thesis and reader model",
            "Chapter architecture with dependencies",
            "Exercise ladder and worked examples plan",
            "Citation and permissions plan",
        ]),
        "model-tuning" => common.extend([
            "Legally usable corpus plan with license/access audit",
            "Data mixture by source type and subdomain",
            "Exclusion list for paywalled or license-unclear materials",
            "Evaluation set blueprint and contamination controls",
        ]),
        "curriculum" => common.extend([
            "Module-by-module curriculum roadmap",
            "Practice artifacts and progress criteria",
            "Assessment rubrics for scholarly performance",
        ]),
        _ => common.extend([
            "Field-specific narrative SoK report",
            "Literature, frontier, or curriculum modules only when the goal requires them",
            "Relation-backed visual views when useful",
        ]),
    }
    common
}

pub fn build_agent_brief(
    field: &str,
    learner: &str,
    goal: &str,
    mode: &str,
    weeks: i32,
    output_format: &str,
) -> String {
    let duration = if weeks > 0 {
        format!("{weeks} weeks")
    } else {
        "not fixed".to_string()
    };
    let deliverables = mode_deliverables(mode)
        .into_iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mode_description = mode_descriptions()
        .get(mode)
        .copied()
        .unwrap_or("Produce a field-structured SoK research report.");
    format!(
        r#"# SoK Agent Brief: {field}

Generated: {generated}
Mode: {mode}
Mode description: {mode_description}

## Objective

Map {field} as a structured knowledge domain for {learner}.

Goal: {goal}
Time budget: {duration}
Requested output format: {output_format}

## Operating Rules

- Preserve domain precision while calibrating explanations, sequence, representations, and tasks to the intended audience.
- Keep the domain profile provisional until source review establishes the field's actual knowledge-bearing forms.
- If the field is broad, decompose it into subfields and state the scoped path.
- Separate substantive structure from syntactic structure.
- Assign every source an epistemic or pedagogical role and actionable access route.
- Treat paywalled, paid, subscription, and license-unclear sources as metadata-only unless the user supplies access or permission.
- Use current sources for frontier claims, active standards, tools, datasets, and strategic reports.

## Required Deliverables

{deliverables}

## Suggested Execution Stages

1. Frame: define field, learner, scope, assumptions, and a provisional domain profile.
2. Decompose: identify subfields, shared foundations, divergent methods, and path options.
3. Source: probe the boundary, foundation, warrant, and any goal-dependent evidence roles without imposing a fixed quota.
4. Extract: identify the questions and field-native elements that carry knowledge, their actual forms, and their load-bearing relations.
5. Design: compare plausible organizing forms, choose the reader-fit architecture, and add only the capability modules the goal requires.
6. Verify: check currentness, access metadata, domain fit, and whether the result is more than a topic list.

## Handoff Files

- sources.csv: add candidate sources with access metadata.
- report.md: fill with the SoK report or artifact blueprint.
- tasks.md: use as an agent work queue.
- downloads/: place only legally downloadable open/free/user-provided materials.
"#,
        generated = today()
    )
}

pub fn build_tasks(mode: &str) -> String {
    let deliverables = mode_deliverables(mode)
        .into_iter()
        .map(|item| format!("- [ ] {item}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"# SoK Agent Tasks

## Setup

- [ ] Confirm field, learner, goal, and scope.
- [ ] Record a provisional discovery lens without choosing the final report or visual form.
- [ ] Decide whether broad-field decomposition is required.

## Sources

- [ ] Fill sources.csv.
- [ ] Run sok audit-sources --manifest sources.csv.
- [ ] Download only legally available open/free/user-provided sources.
- [ ] Mark paid/paywalled/subscription/unknown sources as metadata-only.

## Deliverables

{deliverables}

## Final Audit

- [ ] Source access metadata is complete.
- [ ] Current claims use current sources and dates.
- [ ] Output explains relations and warrants, not just topics.
- [ ] The requested artifact and any goal-dependent learning path are useful for scholar-level work.
"#
    )
}

pub fn build_specificity_checklist(field: &str) -> String {
    format!(
        r#"# Specificity Checklist: {field}

Use this to turn an underspecified domain task into an executable agent brief.

## Scope
- What is the target artifact: report, textbook, curriculum, corpus, evaluation set, or interactive map?
- What is excluded from the field for this run?
- Which subfield path should be primary if the field is broad?

## Learner or Model Target
- What prior knowledge is assumed?
- What performance should be possible after completion?
- What artifacts should demonstrate mastery?

## Sources
- Which source types are mandatory: books, surveys, papers, standards, syllabi, datasets, software, archives?
- Which sources are legally downloadable?
- Which paid or paywalled sources are metadata-only?

## Domain Structure
- What is the substantive structure?
- What is the syntactic structure?
- What are the threshold concepts and bottlenecks?
- After extraction, which load-bearing relations would benefit from a visual, and which form would distort them least?

## Quality Gates
- Does every source have access metadata?
- Are current claims dated and sourced?
- Is the result more than a topic list?
"#
    )
}
