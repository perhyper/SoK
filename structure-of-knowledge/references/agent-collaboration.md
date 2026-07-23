# Agent Collaboration Layer

Use this reference when SoK is not only producing a human-readable report, but coordinating agent work: textbook planning, domain research delegation, model-tuning corpus planning, evaluation-set design, or downloadable source-pack assembly.

## CLI

Use the Rust CLI for deterministic setup and safety checks.

From the skill folder:

```bash
cargo run --bin sok -- <command> [options]
```

For repeated use, build a local binary. From the repository root, use the Makefile:

```bash
make build
structure-of-knowledge/bin/sok <command> [options]
```

From the skill folder, use Cargo directly:

```bash
cargo build --release --bin sok
mkdir -p bin
cp target/release/sok bin/sok
./bin/sok <command> [options]
```

Primary commands:

- `init`: Create an agent workspace with `agent-brief.md`, `tasks.md`, `report.md`, `sources.csv`, `downloads/`, `notes/`, and `logs/`.
- `brief`: Produce a standalone agent handoff brief.
- `scaffold`: Produce a domain-aware SoK report starter with a provisional profile and source role probe.
- `handoff-report`: Wrap a scaffold in next-agent instructions for completing a polished human-reader report.
- `source-template`: Create a CSV source manifest template.
- `audit-sources`: Check that sources include type, identifier or URL, access status, access route, budget/library guidance, and curricular role.
- `download-sources`: Download only sources with default-allowed access statuses by default.
- `ingest last`: Normalize this run's source manifest into cataloged evidence JSONL. This is bounded ingestion, not a persistent knowledge base.
- `lint`: Check scaffold or final Markdown plus source and evidence files before JSON export.
- `export-json`: Convert bounded Markdown plus source/evidence files into `sok-report.json`.
- `validate-report`: Validate the JSON-first report contract and reliability gates.
- `render-html`: Render validated `human_report` JSON into self-contained local HTML.
- `specificity`: Generate questions that make an underspecified domain task executable.

## Recommended Agent Workflow

1. Run `init` for any substantial delegated task.
2. Use the scaffold's source role probe to decide which evidence roles are required, conditional, optional, or waived for this field and goal.
3. Fill `sources.csv` during research, not after writing.
4. Optionally run `sok ingest last --sources sources.csv --output evidence.jsonl` to create a bounded cataloged evidence ledger for the current run.
5. For internal scaffolds, run `sok lint --stage scaffold` and `sok export-json --stage scaffold`; keep `internal_context` private and do not render scaffold JSON as HTML.
6. For final human-facing reports, add reviewed or verified evidence rows, run `sok lint --stage final`, `sok export-json --stage final`, `sok validate-report --strict`, and only then `sok render-html`.
7. Run `sok audit-sources --strict` before final delivery when source manifests are being delivered independently.
8. Run `sok download-sources --dry-run` before downloading.
9. Download only sources with clearly default-allowed access status.
10. Treat paid books, paywalled papers, subscription resources, restricted sources, unknown access, missing access metadata, and unclear licenses as metadata-only unless the user supplies access or permission.
11. Treat existing syllabi and curricula as evidence of pedagogical consensus, not as the boundary of the field.

`sok-report.json` is the structured source of truth for validation, rendering, and downstream tools. Markdown is a bounded input and HTML is a public view. Graph views are chosen from structured relations and `visual_views`; Mermaid is optional and not required.

## Mode Guidance

Use `--mode textbook` when the user asks for a textbook, course book, chapter plan, exercise sequence, or long-form educational artifact. Require a chapter architecture, exercise ladder, worked examples, and citation/permissions plan.

Use `--mode research` when the user asks for a field map, SoK report, literature review, deep research plan, or domain onboarding.

Use `--mode model-tuning` when the user asks for data gathering for model adaptation, fine-tuning, RAG corpus construction, benchmark creation, or domain-specific evaluation. Require license/access audit, corpus mixture, exclusion list, and evaluation blueprint. Do not download or include copyrighted books, paywalled papers, or license-unclear materials as training data.

Use `--mode curriculum` when the user wants a learning path, seminar plan, graduate module, or assessment sequence.

## Source Manifest Contract

Each source row should include:

- `title`
- `type`: book, paper, preprint, notes, course, standard, dataset, software, archive, report, or syllabus
- `identifier`: DOI, arXiv ID, ISBN, official URL, or stable catalog entry
- `url`
- `access_status`: open_access, free_web, public_domain, cc_by, cc_by_sa, official_open, user_provided, paid_book, paywalled, subscription, unknown, or restricted
- `access_route`: how to find or obtain it
- `budget_estimate`: `$0`, visible price, or `budget unknown; library preferred`
- `license`
- `layer`
- `why_it_matters`
- `use_in_curriculum`

## Download Safety

Default downloadable statuses are:

- open_access
- free_web
- public_domain
- cc_by
- cc_by_sa
- official_open
- user_provided

The downloader normalizes access status values by trimming whitespace and lowercasing before comparison. It skips paid, paywalled, subscription, restricted, unknown, and missing access status by default. `--include-unknown` is required before missing or `unknown` access status can be considered downloadable. Never use random shadow-library PDFs as substitutes for paid books or paywalled papers.
