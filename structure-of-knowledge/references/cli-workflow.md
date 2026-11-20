# CLI Workflow

Use the Rust CLI for workspaces, scaffolds, source/evidence files, validated JSON,
and HTML. It supplies deterministic checks; source review remains the agent's job.

## Resolve the Executable Once

Prefer a repository build when the workspace contains `cli/Cargo.toml`:

```bash
make build
SOK_CLI="$PWD/cli/target/release/sok"
"$SOK_CLI" --version
```

Otherwise locate the directory containing the installed skill's `SKILL.md` and
use its `bin/sok` executable (`bin/sok.exe` on Windows). For the default Codex
installation, run the following command to verify the executable.

```bash
SOK_CLI="${CODEX_HOME:-$HOME/.codex}/skills/structure-of-knowledge/bin/sok"
"$SOK_CLI" --version
```

Require the version suffix `(rust)`. If verification fails, report the limitation
instead of using an unverified or stale executable.
Installed skills contain only the compiled CLI, not Cargo sources.

Before an unfamiliar command, run `"$SOK_CLI" <command> --help`; for ingestion,
use `"$SOK_CLI" ingest last --help`. Follow the displayed flags rather than guessing.
Use the resolved executable consistently. From a checkout, a single command can
also run through `cargo run --locked --manifest-path cli/Cargo.toml --bin sok --`.

## Artifact Lanes

`init` creates a workspace with brief, tasks, a separate `scaffold.md`, a compatible
`report.md` starting canvas, source manifest,
downloads, notes, and logs. For example:

```bash
"$SOK_CLI" init --field "computational neuroscience" \
  --learner "software engineer new to neuroscience" \
  --goal "build a source-backed path into the field" \
  --mode research --weeks 12 --out /tmp/sok-neuroscience
```

`source-template` creates a header-only CSV. Add real rows before optional
`ingest last`, which writes cataloged, non-supporting evidence candidates and
**overwrites its output**. Keep reviewed evidence separately to avoid losing it.

For file-producing runs, create `scaffold.md` with `scaffold` or `init` before
synthesis. Populate and consult its domain judgment layer, then write the requested
result separately. Preserve `scaffold.md` while replacing the `report.md` starting
canvas with the final narrative. For older workspaces with only `report.md`, save
the working scaffold separately before composing the report. Short chat-only runs
may use the inline scaffold defined in the output contract.

For a working scaffold, use `scaffold`, maintain `sources.csv`, optionally
ingest sources, then `lint --stage scaffold` and `export-json --stage scaffold`.
Scaffold JSON may contain `internal_context`; do not render or deliver it as a
finished report. Deliver the populated Markdown scaffold as a labeled companion
under the output contract; private scaffold JSON is not a substitute.
`handoff-report` prepares it for another agent's completion.

For a final report, supply field-specific Markdown, sources, and reviewed evidence:

```bash
"$SOK_CLI" lint --stage final --report report.md \
  --sources sources.csv --evidence reviewed-evidence.jsonl --strict
"$SOK_CLI" export-json --stage final --report report.md \
  --sources sources.csv --evidence reviewed-evidence.jsonl --output sok-report.json
"$SOK_CLI" validate-report --input sok-report.json --strict
"$SOK_CLI" render-html --input sok-report.json --output report.html
```

Omit evidence input only when no claims require reviewed support. Render HTML
only when needed. Strict validation is blocking; resolve diagnostics or use an
explicitly supported waiver under the [output contract](output-contract.md).

## Specialized Commands

- `brief` and `specificity`: handoff context or a concreteness checklist.
- `audit-sources --strict`: check standalone manifests before delivery.
- `download-sources`: follow [download rules](agent-collaboration.md#source-downloads).
- `work`: create, validate, negotiate, and accept bounded proposed core patches.
- `migrate-ids`: write a reviewable ID map without rewriting the source report.
- `eval conformance --fixtures <directory>`: deterministic saved-fixture checks,
  without browsing, model calls, or in-place repairs.

Before using optional machine inputs, compatibility migration, or private
`--run-manifest` sidecars, read [machine-contract.md](machine-contract.md).
