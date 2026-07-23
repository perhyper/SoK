# Structure of Knowledge (SoK)

Structure of Knowledge, or SoK, is a Codex skill and orchestration harness for turning an unfamiliar field into a reusable knowledge scaffold: field structure, source layers, evidence standards, visual maps, and next-step task plans.

SoK was originally developed for research-oriented field mapping. Its more general role is to produce a structured intermediate artifact that other agents, research systems, writing tools, curriculum tools, or visualization layers can adapt for many audiences and formats.

The core scaffold idea is inspired by pedagogical ideas of Jerome Bruner and the structure-of-knowledge curriculum tradition: decompose a domain into the concepts, representations, methods, and warrants that make it intelligible, then recompose that structure into a scaffold for learning, research, and downstream generation.

## Purpose

SoK helps an agent capture questions such as:

- What are the central questions of this field?
- What objects, concepts, models, notations, cases, instruments, or institutions organize its knowledge?
- How does the field decide that a claim is true, valid, persuasive, reproducible, or worth pursuing?
- Which sources function as orientation, foundation, method, synthesis, frontier, or critique?
- What sequence of readings, artifacts, exercises, reproductions, critiques, or briefs supports the target use case?
- What are the current open problems, debates, infrastructure bottlenecks, standards, and research opportunities?

## Installation

The macOS/Linux installer requires Rust with Cargo, `curl`, and `tar`. It downloads `main`, builds the CLI locally, and installs the skill to `${CODEX_HOME:-$HOME/.codex}/skills/structure-of-knowledge`.

```bash
curl -fsSL https://raw.githubusercontent.com/perhyper/SoK/main/install.sh | sh
```

To test a branch, tag, or commit:

```bash
REF=your-branch
curl -fsSL https://raw.githubusercontent.com/perhyper/SoK/main/install.sh |
  sh -s -- --ref "$REF"
```

Rerun the installer to update. From a repository checkout, use `make install`. Set `CODEX_HOME` to change the destination. Windows automated installation is not yet supported.

```bash
SKILL_DIR="${CODEX_HOME:-$HOME/.codex}/skills/structure-of-knowledge"
"$SKILL_DIR/bin/sok" --version
"$SKILL_DIR/bin/sok" --help
```

## Local Development

The repository keeps the two deliverables separate:

- `structure-of-knowledge/` contains only the portable Codex skill, agent metadata, and reference documents.
- `cli/` is the Rust crate.
- `cli/target/` is generated build output and is not tracked.
- `bin/sok` exists only inside an installed skill.

Run the CLI from source:

```bash
cargo run --locked --manifest-path cli/Cargo.toml --bin sok -- --help
```

Build a reusable local binary from the repository root:

```bash
make build
cli/target/release/sok --version
cli/target/release/sok --help
```

Or build directly from the CLI crate:

```bash
cd cli
cargo build --locked --release --bin sok
./target/release/sok --version
./target/release/sok --help
```

Available Make targets:

```bash
make fmt
make test
make test-install
make vet
make build
make package
make install
make clean
```

## Using the Skill

In Codex, invoke the skill explicitly:

```text
Use $structure-of-knowledge to map computational neuroscience into a reusable field scaffold with sources, visual maps, and next-step tasks.
```

The first output can remain deliberately scaffold-like. A later agent or downstream tool can turn it into a deep research report, executive briefing, lecture plan, onboarding guide, interactive map, textbook outline, dataset plan, or audience-specific learning path.

## CLI Commands

The CLI provides deterministic scaffolding, agent handoffs, source manifests, access audits, and safe downloads. It does not replace research; it provides the execution frame an agent can fill with sourced findings.

```bash
cargo run --locked --manifest-path cli/Cargo.toml --bin sok -- --help
```

Current commands:

```text
init              Create an agent workspace.
brief             Generate an agent handoff brief.
scaffold          Generate an agent-facing SoK report scaffold.
handoff-report    Generate a next-agent brief for completing a human-reader report from a scaffold.
source-template   Create a header-only source manifest CSV template.
audit-sources     Audit source access metadata.
download-sources  Download legally accessible open/free sources from a manifest.
ingest last       Normalize this run's source manifest into cataloged evidence JSONL.
export-json       Convert bounded Markdown plus source/evidence files to sok-report.json.
lint              Check Markdown, source, and evidence inputs before JSON export.
validate-report   Validate sok-report.json reliability gates.
render-html       Render a validated human_report JSON file to self-contained local HTML.
specificity       Generate a concreteness checklist for an underspecified domain task.
```

## JSON-First Bounded Workflow

The report pipeline is bounded to the files from the current run. It does not create a persistent wiki, daemon, database, or cross-run evidence store. Markdown is an input surface: an internal scaffold from `sok scaffold`, or a canonical human report Markdown file. The structured source of truth is `sok-report.json`; HTML is only a rendered public view.

Use the scaffold lane when an agent needs internal working context:

```bash
cli/target/release/sok source-template --output /tmp/sok-sources.csv
# Add real source rows to /tmp/sok-sources.csv before ingesting it.
cli/target/release/sok ingest last \
  --sources /tmp/sok-sources.csv \
  --output /tmp/sok-evidence.jsonl
cli/target/release/sok scaffold \
  --field "topology" \
  --output /tmp/sok-scaffold.md
cli/target/release/sok lint \
  --stage scaffold \
  --scaffold /tmp/sok-scaffold.md \
  --sources /tmp/sok-sources.csv \
  --evidence /tmp/sok-evidence.jsonl
cli/target/release/sok export-json \
  --stage scaffold \
  --scaffold /tmp/sok-scaffold.md \
  --sources /tmp/sok-sources.csv \
  --evidence /tmp/sok-evidence.jsonl \
  --output /tmp/sok-scaffold.json
```

The scaffold JSON may contain `internal_context` and export diagnostics. Do not present it as a finished report and do not render it to HTML.

Use the final-report lane when a human-facing report has reviewed evidence:

```bash
cli/target/release/sok lint \
  --stage final \
  --report reports/examples/json-first-human-report.md \
  --sources reports/examples/json-first-sources.csv \
  --evidence reports/examples/json-first-reviewed-evidence.jsonl
cli/target/release/sok export-json \
  --stage final \
  --report reports/examples/json-first-human-report.md \
  --sources reports/examples/json-first-sources.csv \
  --evidence reports/examples/json-first-reviewed-evidence.jsonl \
  --output /tmp/sok-report.json
cli/target/release/sok validate-report \
  --input /tmp/sok-report.json \
  --strict
cli/target/release/sok render-html \
  --input /tmp/sok-report.json \
  --output /tmp/sok-report.html
```

Only `human_report` JSON that passes validation should be rendered. The renderer uses an explicit public-field allowlist and ignores `internal_context` and `diagnostics`.

## Generate an Agent Scaffold

Use `scaffold` when you want the agent-facing starter only.

```bash
cargo run --locked --manifest-path cli/Cargo.toml --bin sok -- scaffold \
  --field "computational neuroscience" \
  --learner "technical reader new to neuroscience" \
  --goal "build a reusable field map for downstream report generation" \
  --weeks 12 \
  --output /tmp/sok-computational-neuroscience.md
```

The scaffold infers a first-pass domain profile from the field name and adjusts the starter accordingly:

- Formal or well-structured fields foreground prerequisite graphs, proof or validation standards, canonical constructions, and counterexample practice.
- Ill-structured or interpretive fields foreground cases, schools, lenses, debates, and warrant styles.
- Instrument- or infrastructure-bound fields foreground instruments, datasets, collaborations, standards, uncertainty practices, and strategic reports.

This profile is a hypothesis for source review, not a restrictive label. The scaffold includes a source role probe that treats source categories as diagnostics rather than quotas. Existing syllabi and curricula can reveal stabilized sequencing and training conventions, but they are not the boundary of the field. Conditional roles such as recent frontier surveys, datasets, standards, or primary corpora can be required or waived with rationale depending on the field and goal.

The scaffold is an internal working artifact. It may include learner profile, original goal, scoped assumptions, placeholder source rows, source-role waiver notes, and quality-gate notes. Keep it rough when roughness is useful: its job is to preserve structure and handoff context, not to read like a finished publication.

For downstream tooling, export the scaffold with `sok export-json --stage scaffold`. That internal-stage JSON preserves scaffold context separately from the public `report` payload and should not be rendered as a final human report.

## Handoff a Scaffold for a Human-Reader Report

Use `handoff-report` when you want to pass a scaffold to a next agent that will complete a polished report for a specified human audience.

```bash
cargo run --locked --manifest-path cli/Cargo.toml --bin sok -- handoff-report \
  --field "computational neuroscience" \
  --learner "technical reader new to neuroscience" \
  --goal "complete a polished audience-specific report" \
  --weeks 12 \
  --output /tmp/sok-computational-neuroscience-handoff.md
```

If you already generated a scaffold, pass it directly:

```bash
cargo run --locked --manifest-path cli/Cargo.toml --bin sok -- handoff-report \
  --scaffold /tmp/sok-computational-neuroscience.md \
  --output /tmp/sok-computational-neuroscience-handoff.md
```

The handoff preserves internal context for the next agent while instructing it not to expose prompt intent, learner profile, raw assumptions, scaffold quality notes, or placeholder rows in the final report.

After the next agent completes the human-facing Markdown and evidence review, run `sok lint --stage final`, `sok export-json --stage final`, `sok validate-report --strict`, and then `sok render-html`. Final reports must not contain scaffold sections such as `Research Frame`, `Scaffold Quality Notes`, raw prompt intent, or placeholder source rows.

## Use Cases and Extension Patterns

SoK is most useful when treated as an upstream knowledge-structure layer. The same scaffold can be recomposed for different users and products:

- Independent scholarly learning: build a self-directed path from field structure and actionable source routes without reducing the field to a beginner summary.
- Interdisciplinary research preparation: map a target field's concepts, methods, evidence standards, and limitations before attempting cross-domain transfer.
- Research briefing: expand the scaffold into a source-grounded report with dated frontier claims and evidence notes.
- Executive or policy briefing: compress the scaffold into decisions, risks, institutions, and current constraints.
- Course or workshop plan: turn the concept map, literature ladder, and practice tasks into sessions and assessments.
- Team onboarding guide: convert field structure into vocabulary, canonical examples, failure modes, and first tasks.
- Interactive visual map: render concept graphs, debate maps, timelines, or instrument-data pipelines in a UI.
- Textbook or long-form outline: use the scaffold to derive chapter dependencies, examples, exercises, and permissions plans.
- Corpus or model-tuning plan: use source layers and access metadata to define legally usable data mixtures and evaluation sets.

## Create an Agent Workspace

Use `init` for delegated research, textbook planning, curriculum design, model-tuning corpus planning, or source-pack workflows.

```bash
cargo run --locked --manifest-path cli/Cargo.toml --bin sok -- init \
  --field "particle physics" \
  --learner "physics-adjacent authoring team" \
  --goal "plan a textbook outline and source pack" \
  --mode textbook \
  --weeks 16 \
  --out /tmp/sok-particle-physics-textbook
```

This creates:

- `agent-brief.md`
- `tasks.md`
- `report.md`
- `sources.csv`
- `downloads/`
- `notes/`
- `logs/`

## Source Access and Downloads

Downloads are intentionally conservative. By default, `download-sources` only downloads sources whose access status is explicitly open or user-provided:

- `open_access`
- `free_web`
- `public_domain`
- `cc_by`
- `cc_by_sa`
- `official_open`
- `user_provided`

Paid, paywalled, subscription-only, restricted, unknown, and blank access statuses are metadata-only by default.

```bash
cargo run --locked --manifest-path cli/Cargo.toml --bin sok -- audit-sources \
  --manifest /tmp/sok-particle-physics-textbook/sources.csv \
  --strict

cargo run --locked --manifest-path cli/Cargo.toml --bin sok -- download-sources \
  --manifest /tmp/sok-particle-physics-textbook/sources.csv \
  --out-dir /tmp/sok-particle-physics-textbook/downloads \
  --dry-run
```

`download-sources` normalizes `access_status` and `--allow-status` by trimming whitespace and lowercasing before comparison. Missing or `unknown` access statuses are downloaded only when `--include-unknown` is passed. `--allow-status` and `--include-unknown` change only the download decision; they do not grant legal access to restricted material.

Create a source manifest template:

```bash
cargo run --locked --manifest-path cli/Cargo.toml --bin sok -- source-template --output /tmp/sok-sources.csv
# Add real source rows before auditing or downloading.
cargo run --locked --manifest-path cli/Cargo.toml --bin sok -- audit-sources --manifest /tmp/sok-sources.csv --strict
cargo run --locked --manifest-path cli/Cargo.toml --bin sok -- download-sources --manifest /tmp/sok-sources.csv --out-dir /tmp/sok-downloads --dry-run
```

## Expected Outputs

A substantial SoK package usually includes:

- Narrative report: the field's knowledge structure and research path.
- Literature ladder: orientation, foundation, methods, synthesis, frontier, and critique sources.
- Access metadata: source type, DOI/arXiv/ISBN/URL, access route, paid/free status, and budget estimate.
- Curriculum roadmap: modules, tasks, or checkpoints adapted to the target use case.
- Visual package: structured graph views selected from report relations when useful; Mermaid is optional and not required.
- Frontier map: live problems, debates, schools, infrastructure constraints, and required background.
- Source matrix: source date, type, role, reliability, and curricular use.
- Agent packet: brief, task queue, source manifest, download log, notes, and report scaffold.
- JSON report: `sok-report.json`, the structured source of truth for validation, rendering, and downstream tooling.
- HTML report: a self-contained public view rendered only from validated `human_report` JSON.

## Design Notes

- The scaffold should preserve field structure, evidence boundaries, and source roles. Downstream tools can then simplify, specialize, translate, or reformat it for a target audience.
- Foundations are not always easy. In some fields, a foundation may be a method, notation, tool, dataset, institution, standard, or debate style.
- Current claims require dates and sources. Durable foundations and frontier claims should not be mixed without marking the difference.
- Source recommendations must be actionable. Books should include publisher or ISBN where available and a library or purchase route. Papers should include DOI, arXiv ID, or stable URL. Paid or paywalled sources need visible price estimates or `budget unknown; library preferred`.
- Source-role coverage is not a fixed quota. Require orientation, foundation, method/warrant, and sequence evidence for most substantial tasks, then add or waive frontier, dataset, standard, infrastructure, primary-corpus, or canonical-case roles with rationale.
- Source-role requirements are intended to be decided from the field, goal, and reviewed evidence. They are categorical requirements, not numeric weights. The current release does not implement a dynamic weighting engine.
- Different fields require different structures. Some need prerequisite graphs, some need debate networks, and some need instrument-data-governance pipelines.
- A good SoK-derived artifact connects concepts, evidence, sources, and next actions into an explicit structure.
- Graph views are selected from structured `relations` and `visual_views`; Mermaid diagrams are optional inspection aids, not a required runtime or source of authority.

## Non-Skill Specifications

The Codex skill defines agent behavior. The repository also includes implementation-neutral specifications:

- `specs/sok-harness.yaml`: research harness stages, required artifacts, quality gates, and CLI command registry.
- `specs/sok-report.schema.json`: structured report contract for future validators, visualization tools, or datasets.
- `cli/src`: Rust CLI for agent workspaces, source manifests, access audits, safe downloads, scaffolds, and handoff briefs.

This keeps SoK extensible beyond a single prompt: CLI workflows, web reports, evaluation benchmarks, source-ingestion tools, and visualization apps can share the same contracts.

## License

MIT License. See `LICENSE`.
