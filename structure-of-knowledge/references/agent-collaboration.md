# Agent Collaboration Layer

Use this reference when SoK is not only producing a human-readable report, but coordinating agent work: textbook planning, domain research delegation, model-tuning corpus planning, evaluation-set design, or downloadable source-pack assembly.

## CLI

Use the Rust CLI for deterministic setup and safety checks.

Resolve the executable once per run. Prefer the repository build when the current workspace contains `cli/Cargo.toml`; otherwise use the installed skill.

From an installed skill:

```bash
SOK_CLI="${CODEX_HOME:-$HOME/.codex}/skills/structure-of-knowledge/bin/sok"
"$SOK_CLI" --version
"$SOK_CLI" <command> [options]
```

From a repository checkout, run the Rust crate directly:

```bash
cargo run --locked --manifest-path cli/Cargo.toml --bin sok -- <command> [options]
```

For repeated development use, build the CLI from the repository root:

```bash
make build
SOK_CLI="$PWD/cli/target/release/sok"
"$SOK_CLI" --version
"$SOK_CLI" <command> [options]
```

The version output must end with `(rust)`. Do not use an executable that lacks this marker. On Windows, resolve the installed executable as `%CODEX_HOME%\skills\structure-of-knowledge\bin\sok.exe` (or under `%USERPROFILE%\.codex` when `CODEX_HOME` is unset). The repository does not keep a compiled executable inside the skill source tree. `make install` copies the freshly built Rust binary into the installed skill's `bin/sok`.

Before using an unfamiliar command, run `"$SOK_CLI" <command> --help` (or `"$SOK_CLI" ingest last --help`) and use the displayed flags. Do not guess arguments by provoking an error.

Use the CLI when producing persistent workflow artifacts or enforcing deterministic checks. A narrow prose answer does not require it.

Primary commands:

- `init`: Create an agent workspace with `agent-brief.md`, `tasks.md`, `report.md`, `sources.csv`, `downloads/`, `notes/`, and `logs/`.
- `brief`: Produce a standalone agent handoff brief.
- `scaffold`: Produce a discovery canvas with a provisional lens, source-role probe, field-element inventory, and report-architecture decision.
- `handoff-report`: Wrap a scaffold in next-agent instructions for completing a polished human-reader report.
- `source-template`: Create a header-only CSV source manifest; add real rows before `ingest last`.
- `audit-sources`: Check that sources include type, identifier or URL, access status, access route, budget/library guidance, and epistemic or pedagogical role.
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
4. Optionally run `"$SOK_CLI" ingest last --sources sources.csv --output evidence.jsonl` to create a bounded cataloged evidence ledger for the current run.
5. For internal scaffolds, run `"$SOK_CLI" lint --stage scaffold` and `"$SOK_CLI" export-json --stage scaffold`; keep `internal_context` private and do not render scaffold JSON as HTML.
6. For final human-facing reports, confirm or revise the scaffold's narrative architecture after field-structure extraction, add reviewed or verified evidence rows, use canonical headings or recognized `sok:surface` markers for the structured tables the run needs, then lint, export, validate strictly, and render.
7. Run `"$SOK_CLI" audit-sources --strict` before final delivery when source manifests are being delivered independently.
8. Run `"$SOK_CLI" download-sources --dry-run` before downloading.
9. Download only sources with clearly default-allowed access status.
10. Treat paid books, paywalled papers, subscription resources, restricted sources, unknown access, missing access metadata, and unclear licenses as metadata-only unless the user supplies access or permission.
11. Treat existing syllabi and curricula as evidence of pedagogical consensus, not as the boundary of the field.

`sok-report.json` is the structured source of truth for validation, rendering, and downstream tools. Markdown is a bounded input and HTML is a public view. Graph views are chosen from structured relations and `visual_views`; Mermaid is optional and not required. Strict validation is expected to fail on lossy export diagnostics, dangling relation or visual endpoints, missing substantial-report structure without a waiver, and evidence that is only cataloged, qualifying, contradictory, background, or missing review metadata.

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
- `date`: publication, release, or last-updated date when known
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
