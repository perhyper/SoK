---
name: structure-of-knowledge
description: Evidence-grounded field-mapping and agent-collaboration harness for producing reusable knowledge scaffolds, literature ladders, curricula, report plans, visual knowledge maps, textbook architectures, and downstream research artifacts. Use when the user asks for SoK, structure of knowledge, a field map, research curriculum, literature ladder, deep research report, domain research delegation, or a guided path into an unfamiliar domain.
---

# Structure of Knowledge

## Overview

Use this skill to turn a broad learning or research goal into a reusable map of a field. The scaffold should show how the field is organized, how claims are warranted, what literature anchors it, and how a learner or downstream system can progress from orientation to purposeful work.

The core idea is inspired by Jerome Bruner and the structure-of-knowledge curriculum tradition: decompose a domain into the concepts, representations, methods, and warrants that make it intelligible, then recompose that structure into a scaffold for learning, research, and downstream generation.

## Reference Loading

For a full SoK task, read these references before drafting the final answer:

- `references/research-protocol.md` for source gathering, evidence ranking, and currentness checks.
- `references/output-contract.md` for the report, curriculum, and visualization package.
- `references/agent-collaboration.md` when coordinating agent work, creating a textbook plan, planning model-tuning data, or downloading source packs.

For a narrow answer, use this file and load only the reference that matches the missing detail.

## Workflow

1. Frame the request.
   - Identify the field, learner profile, target depth, time budget, language preference, and desired output format.
   - If the field is broad, first produce a domain decomposition: major subfields, shared foundations, divergent methods, and possible scoped paths. Ask the user to choose only when the path would materially change the report; otherwise state a scoped assumption.
   - If the field or goal is ambiguous enough to change the requested artifact or report architecture, ask one concise clarifying question. Otherwise state assumptions and continue.
   - Build a preliminary domain profile rather than treating classification as a final label. Estimate the relevant lenses: formal/well-structured, ill-structured, professional/practice-based, empirical, computational, instrument- or infrastructure-bound, emerging, interdisciplinary, or mixed.
   - Treat this profile as a source-review hypothesis. It should reveal which knowledge-production modes must be inspected, not restrict the field to an existing curriculum or a single legacy category.

2. Research the field.
   - Use current web research or user-provided corpora by default because frontier issues, recent surveys, standards, and tools change.
   - Prefer primary sources, field handbooks, authoritative textbooks, major survey/review papers, seminal papers, official standards, respected syllabi, and venue or society materials.
   - For instrument- or infrastructure-bound fields, include official review bodies, facilities, datasets, collaborations, standards, strategic reports, and statistical conventions as part of the source stack.
   - Build a source role probe rather than enforcing a fixed source quota. Required roles usually include orientation or boundary evidence, canonical foundations, and method or warrant evidence. Conditional roles include pedagogical sequence, recent frontier sources, datasets or standards, primary corpora, canonical cases, and infrastructure sources when the domain or user goal requires them.
   - Mark irrelevant source roles as waived with a short rationale. For example, a core Latin-language curriculum may waive recent frontier surveys, while a research-fluency path in number theory should include frontier or problem-list sources.
   - Treat existing syllabi and curricula as evidence of stabilized pedagogical consensus, not as the boundary of the field.
   - Build a source stack across roles and layers: scholarly orientation, canonical foundations, core methods, representative applications or cases, frontier surveys, debates, open problems, standards, datasets, or corpora as appropriate.
   - Track what role each source plays in the scaffold, requested artifact, or learning sequence, not only its bibliographic details.
   - For every recommended source, record type, identifier, access route, free/paid/paywalled status, and budget estimate or `budget unknown; library preferred`.

3. Extract the structure of knowledge.
   - Name the field's generative questions, core objects, representations, transformations, methods, standards of evidence, canonical examples, and failure modes.
   - Record the actual form each element takes in this field. Do not assume that every field is primarily a concept hierarchy: cases, instruments, institutions, datasets, practices, scales, or controversies may be load-bearing.
   - Separate substantive structure (concepts, models, claims, objects) from syntactic structure (how the field warrants, disputes, and revises claims).
   - Treat foundations as structurally central, not necessarily easy. A foundation may be a method, notation, tool, dataset, institution, standard, or debate practice.
   - For formal domains, capture proof techniques, canonical constructions, counterexample practices, classification problems, and problem-list traditions.
   - For empirical or infrastructure-bound domains, capture instruments, measurement regimes, data products, collaborations, uncertainty/statistical practices, and review infrastructures.
   - Distinguish prerequisites from recurring fundamental ideas. A prerequisite is needed before a topic; a fundamental idea returns throughout the spiral at higher sophistication.
   - Identify threshold concepts, common misconceptions, and places where newcomers to the field confuse vocabulary with understanding.

4. Choose the report architecture.
   - Do this only after the source review and field-element inventory. Compare at least two plausible organizing forms against the observed load-bearing relations and the reader's goal.
   - Possible forms include dependency graph, causal system, process or pipeline, multiscale map, debate network, case constellation, taxonomy, chronology, genealogy, or a justified hybrid. These are comparison options, not templates.
   - Identify the core ideas to foreground and the important surrounding elements needed to make them intelligible. Omit peripheral inventory that does not serve the thesis.
   - Record an executive thesis, chosen organizing form, rationale, ordered section purposes, visual logic, rejected alternatives, and scope limits in `Report Architecture`.
   - Give the public report field-specific headings and ordering. Never copy the scaffold's working sections as an automatic final table of contents.

5. Design a learning spiral when the goal requires one.
   - Organize the curriculum into goal-fit repeated passes over core ideas. Orientation, core grammar, research fluency, and frontier participation are examples, not mandatory stages.
   - Use advance organizers, concept maps, and worked scholarly performances to make the field's structure visible before asking for independent synthesis.
   - For each module, include purpose, essential questions, readings, practice tasks, artifacts to produce, dependencies, and criteria for progress.
   - For ill-structured domains, use cases, schools of thought, debates, and interpretive lenses rather than forcing a single hierarchy.

6. Deliver the SoK package.
   - Keep internal scaffold artifacts and human-facing reports separate. Scaffolds may include learner profile, original goal, assumptions, placeholders, and quality notes; final reports must not expose that machinery.
   - Treat `sok-report.json` as the structured source of truth for downstream tooling. Preserve reviewed inventory rows in `report.field_elements`; `core_ideas`, `methods`, and `representations` are compatibility projections, not the field's ontology. Markdown scaffolds and field-specific reports with structured surfaces are bounded inputs; HTML is a rendered public view.
   - Use typed `sok-knowledge/v1`, `sok-evidence/v1`, and `sok-pedagogy/v1` sidecars only as additive machine inputs. Keep public `sok-report/v1` and `sok-report/v2` compatibility intact; do not embed core packages, run manifests, or work sidecars in the public report.
   - When private reproducibility data is useful, request `--run-manifest <sok-run.json>`. Treat `sok-run/v1`, `sok-work-order/v1`, and `sok-work-result/v1` as private sidecars that can be removed without changing the public report payload.
   - Provide a narrative report. Add a visualization only when structured relations justify it; select graph views from `relations` and `visual_views`. Mermaid is optional and not required.
   - Structured surfaces support export and validation; they do not define public section order. A surface may keep its canonical heading or use a field-specific heading followed by `<!-- sok:surface report-architecture -->`, `field-elements`, `claims`, `relations`, `literature-ladder`, `curriculum`, or `visual-views`.
   - Treat unknown or empty `sok:` directives and duplicate or conflicting `sok:surface` markers as blocking errors. Directives are standalone control lines, never examples inside fenced code.
   - Put curriculum dependencies in a `Prerequisite IDs` or `Prerequisites` column, and put graph intent in `Visual Views` rows that reference explicit relation IDs. Include only the surfaces that serve the requested artifact, with a structure waiver when strict validation requires one.
   - Place a declared visual inside a narrative section with `<!-- sok:visual-view view-id -->`. Unplaced views remain in the collapsed evidence appendix.
   - Do not infer semantic relations from Mermaid arrows, node proximity, or section order. Relations come from structured relation rows, curriculum prerequisites, or another explicit author-owned surface.

7. Quality check.
   - Verify that claims about current literature, active debates, standards, or tools are sourced with dates.
   - Verify that recommended readings include access metadata and usable acquisition routes.
   - When a curriculum is included, ensure it starts from the learner's actual entry point but still preserves the field's intellectual integrity.
   - Confirm the output is not a flat topic list: it must explain the field's load-bearing relations, organizing form, and inquiry methods; include progression only when the artifact needs it.
   - For final-report publication, treat `sok validate-report --strict` as blocking. Final Markdown must contain a non-empty field-element inventory, report thesis, chosen organizing form, architecture rationale, and at least one field-specific public section. Strict validation also fails on embedded export warnings or errors, missing relation/prerequisite/visual structure without a waiver, dangling endpoints, public/internal boundary leaks, weak evidence support semantics, and stale or undated currentness claims.

## Helper Script

Use the Rust CLI when the task creates an agent workspace, scaffold, source manifest, evidence ledger, validated JSON report, or rendered HTML. Do not invoke it for a narrow conceptual answer that does not need an artifact.

Before the first CLI call, resolve and verify the executable. If the current workspace contains `cli/Cargo.toml`, prefer the repository build so the CLI matches the checked-out sources. Otherwise use the installed skill.

From an installed skill:

```bash
SOK_CLI="${CODEX_HOME:-$HOME/.codex}/skills/structure-of-knowledge/bin/sok"
"$SOK_CLI" --version
```

The output must end with `(rust)`. The installed skill does not contain Cargo sources. From a repository checkout:

```bash
make build
SOK_CLI="$PWD/cli/target/release/sok"
"$SOK_CLI" --version
```

On Windows, use `%CODEX_HOME%\skills\structure-of-knowledge\bin\sok.exe` (or `%USERPROFILE%\.codex\skills\structure-of-knowledge\bin\sok.exe` when `CODEX_HOME` is unset) and require the same `(rust)` version marker.

Use the resolved path consistently:

```bash
"$SOK_CLI" init \
  --field "computational neuroscience" \
  --learner "software engineer new to neuroscience" \
  --goal "build a source-backed path into the field" \
  --mode research \
  --weeks 12 \
  --out /tmp/sok-computational-neuroscience
```

Before using an unfamiliar command, run `"$SOK_CLI" <command> --help` (or `"$SOK_CLI" ingest last --help`) and follow the displayed flags. Do not infer required flags from command names.

If the verified Rust CLI is unavailable, report that limitation instead of silently using a stale executable. The installed skill keeps only the compiled executable at `bin/sok`; Rust sources live separately under `cli/`.

Useful subcommands:

- `brief`: create an agent handoff brief.
- `scaffold`: create a source-review and report-architecture discovery canvas; its field-name lens is provisional.
- `handoff-report`: wrap a scaffold in next-agent instructions for completing a polished human-reader report.
- `source-template`: create a header-only source manifest; add real source rows before `ingest last`.
- `audit-sources`: verify access metadata and source role or relevance fields.
- `download-sources`: download only sources with default-allowed access statuses by default.
- `ingest last`: normalize the current run's source manifest into cataloged evidence JSONL without treating it as reviewed claim support.
- `lint`: check scaffold or final Markdown plus source/evidence files before JSON export.
- `export-json`: convert bounded scaffold or final Markdown plus source/evidence files into `sok-report.json`.
- `validate-report`: validate `sok-report.json` reliability gates; use `--strict` before rendering or publication.
- `migrate-ids`: write a deterministic old-to-new ID map for `sok-report/v1` or `sok-report/v2` without rewriting the input.
- `work`: validate provider-neutral WorkOrder and WorkResult sidecars; accept only proposed patches into a new core output file.
- `eval`: score saved provider-neutral conformance fixtures without model or network calls.
- `render-html`: render a validated `human_report` JSON file to self-contained local HTML.
- `specificity`: make an underspecified domain task concrete enough to execute.

The CLI is not a substitute for research. It provides a stable execution frame so the agent can fill it with sourced findings.

Recommended bounded lanes:

- Scaffold lane: create or receive scaffold Markdown, maintain `sources.csv`, optionally run `"$SOK_CLI" ingest last`, run `"$SOK_CLI" lint --stage scaffold`, then `"$SOK_CLI" export-json --stage scaffold`. This JSON may contain `internal_context`; do not present it as a finished report and do not render it to HTML.
- Final-report lane: complete a field-specific human report Markdown file plus its structured evidence surfaces, review evidence into JSONL entries, run `"$SOK_CLI" lint --stage final`, `"$SOK_CLI" export-json --stage final`, `"$SOK_CLI" validate-report --strict`, then `"$SOK_CLI" render-html`. Optional core sidecars may be supplied to `export-json`; optional run manifests remain private. Only this lane produces human-facing HTML. Reviewed or verified evidence with `support_kind: supports`, `reviewed_at`, and a locator or support note can satisfy claims; `qualifies` narrows an already supported claim, `contradicts` records conflict, and `cataloged` source rows remain non-supporting.
