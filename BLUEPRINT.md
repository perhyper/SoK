# SoK Blueprint

## Product Definition

Structure of Knowledge, or SoK, is a Codex skill and research harness that turns a learning or research goal into a bounded, evidence-grounded map of a field. It identifies the field's actual knowledge-bearing elements, their forms and warrants, load-bearing relations, source roles, and a reader-fit artifact architecture.

SoK produces a reusable intermediate scaffold rather than a persistent knowledge base. Other agents and tools can adapt that scaffold into a human-readable report, curriculum, briefing, textbook plan, corpus plan, or interactive view.

## Foundational Idea

SoK's scaffold idea is inspired by Jerome Bruner and the structure-of-knowledge curriculum tradition: decompose a domain into the concepts, representations, methods, and warrants that make it intelligible, then recompose that structure into a scaffold for learning, research, and downstream generation.

## Current Architecture

- `structure-of-knowledge/SKILL.md`: agent workflow, trigger conditions, and quality gates.
- `structure-of-knowledge/references/`: research, output, and collaboration protocols loaded as needed.
- `cli/src/`: Rust CLI for scaffolds, handoffs, source manifests, evidence ingestion, validation, and rendering.
- `specs/sok-harness.yaml`: implementation-neutral stage and command contract.
- `specs/sok-report.schema.json`: JSON-first report contract.
- `cli/tests/fixtures/`: compact bounded inputs and JSON validation fixtures.

Markdown is an authoring input. `sok-report.json` is the structured source of truth for validation and downstream tooling. HTML is a self-contained public view rendered only from validated `human_report` JSON.

New human-report exports use `sok-report/v2` and preserve reviewed inventory rows in `report.field_elements`. The older `core_ideas`, `methods`, and `representations` arrays are compatibility projections rather than a universal ontology; v1 remains readable for legacy artifacts.

## Product Boundaries

- Keep agent-facing scaffolds separate from human-facing reports.
- Keep every run bounded to its explicit Markdown, source manifest, and evidence files.
- Do not create a daemon, database, cross-run evidence ledger, or persistent wiki.
- Treat cataloged sources as discovery metadata until evidence has been reviewed or verified.
- Select visual views only when structured relations justify them; no visualization syntax or graph family is mandatory.
- Preserve technical and evidentiary integrity while allowing downstream tools to adapt depth, tone, and format for a target audience.

## Current Workflow

1. Frame the field, audience, goal, scope, and provisional domain profile.
2. Build a source-role probe and record access metadata.
3. Extract field-native elements, warrants, and load-bearing relations; add frontier questions only when the goal requires them.
4. Compare plausible organizing forms and choose a source-grounded report architecture.
5. Generate an internal scaffold or complete a field-specific ordered human report.
6. Ingest bounded source metadata into evidence JSONL when useful.
7. Lint the selected lane and export `sok-report.json`.
8. Validate final-report reliability gates and render HTML.

## Distribution

- Development checkout: run the CLI with Cargo or `make build`.
- Local Codex installation: run `make install`.
- User testing: `install.sh` downloads a selected Git ref, builds the Rust CLI locally, and installs it with the portable skill sources.
- Plugin distribution: package the skill and tested CLI assets behind a Codex plugin manifest and marketplace entry.

## Development Priorities

- Increase field-structure inference reliability without turning provisional lenses into report templates.
- Improve tool-use guidance for agents executing the skill.
- Evaluate visualization choices against report relations, audience tasks, and information density.
- Keep the public report concise while increasing the specificity and weight of each included claim.
- Add plugin packaging and installation tests without weakening the bounded local workflow.
