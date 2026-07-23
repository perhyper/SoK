# Agent Collaboration Extension Report

Date: 2026-06-19

## Purpose

This extension turns SoK from a human-facing report skill into a reusable collaboration harness for agents. The motivating use cases are:

- A user asks an agent to design a textbook for a field.
- A user delegates research on a knowledge domain to an agent team.
- A user asks for source discovery before model tuning, RAG corpus construction, or evaluation-set design.
- A user wants all legally downloadable open/free materials collected into a local source pack.

## Design Principle

The agent layer should not replace judgment. It should make the workflow executable:

1. Create a task workspace.
2. Produce an agent brief.
3. Maintain a source manifest with access metadata.
4. Audit sources before use.
5. Download only legally accessible open/free/user-provided materials.
6. Leave paid, paywalled, subscription, restricted, or unknown-license materials as metadata-only unless the user supplies access or permission.

## Implemented CLI

The canonical CLI is implemented in Rust and lives at:

```text
cli/src/
```

Build it with:

```bash
make build
cli/target/release/sok --version
```

Commands:

- `init`: Create a workspace containing `agent-brief.md`, `tasks.md`, `report.md`, `sources.csv`, `downloads/`, `notes/`, and `logs/`.
- `brief`: Produce a standalone agent handoff brief.
- `scaffold`: Produce a SoK report scaffold.
- `handoff-report`: Wrap a scaffold in instructions for completing a human-facing report.
- `source-template`: Create a header-only CSV source manifest; add real rows before ingestion or audit.
- `audit-sources`: Validate required source metadata.
- `download-sources`: Download only sources with allowed open/free/user-provided access status by default.
- `ingest last`: Normalize the current run's source manifest into cataloged evidence JSONL.
- `lint`: Check scaffold or final-report inputs before JSON export.
- `export-json`: Convert bounded Markdown, source, and evidence inputs to `sok-report.json`.
- `validate-report`: Enforce report structure, evidence, currentness, and public-boundary gates.
- `render-html`: Render validated `human_report` JSON as self-contained HTML.
- `specificity`: Generate a checklist that turns vague domain work into a concrete agent task.

Modes:

- `research`: SoK report, literature ladder, frontier map, curriculum path.
- `textbook`: textbook thesis, chapter architecture, exercise ladder, permissions plan.
- `model-tuning`: legal corpus plan, access audit, exclusion list, evaluation blueprint.
- `curriculum`: module roadmap, practice artifacts, scholarly assessment.

## Source Manifest Contract

The source manifest is deliberately plain CSV so that agents, scripts, spreadsheets, and humans can edit it. Required fields include source title, type, identifier, URL, access status, access route, budget estimate, license, layer, curricular role, and notes.

This is important for model-tuning workflows. A source that is useful for learning may still be unusable as training data. The CLI therefore separates metadata collection from download and treats copyright/licensing as an explicit quality gate.

## Safety Boundary

The downloader is conservative by default. It downloads only these statuses:

- `open_access`
- `free_web`
- `public_domain`
- `cc_by`
- `cc_by_sa`
- `official_open`
- `user_provided`

It skips:

- `paid_book`
- `paywalled`
- `subscription`
- `restricted`
- `unknown`

The downloader is not a piracy tool and should not be pointed at shadow-library PDFs.

## Future Extensions

- Add license classifiers for common open-access and Creative Commons pages.
- Add DOI/arXiv/ISBN enrichment hooks.
- Add source deduplication and checksum manifests.
- Add external evaluation adapters that score reports against SoK quality gates.
- Add optional interactive exploration for relation-backed visual views.
