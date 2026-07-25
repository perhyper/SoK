# SoK Harness Review: Solid-State Battery Trial

Date: 2026-07-25

Artifact refresh note: the regenerated evaluation bundle now preserves the report's machine-readable structure. The original review findings remain useful as the rationale for the repair plan, but the result rows below reflect the refreshed local artifacts.

## Verdict

The **research artifact is useful**, and the refreshed bundle now exercises the repaired structure-preservation path.

The Markdown report succeeds as a field scaffold: it replaces a flat materials list with a coupled chain from bulk ion transport to interfaces, mechanics, cell metrics, and manufacturing; it separates durable foundations from dated frontier claims; and it gives each source an epistemic and curricular role.

The deterministic harness succeeds at:

- keeping internal scaffold material out of the final public payload;
- separating cataloged sources from reviewed evidence;
- requiring evidence cardinality for claims;
- checking source identifiers, access metadata, dates, and source-role coverage;
- producing stable IDs and a schema-valid JSON report;
- rejecting missing or dangling references when structured references exist.

The current bundle establishes that this substantial report contains a machine-readable knowledge structure: exported relations, visual views, the literature ladder, and curriculum prerequisites are populated, and strict validation passes without embedded export warnings.

Release readiness still depends on the remaining plan-level documentation and regression coverage, but this evaluation artifact no longer demonstrates a structure-loss blocker.

## Trial Evidence

| Check | Result |
|---|---|
| Sources | 15 authoritative papers, reviews, an official roadmap, and an open course |
| Public claims | 15 |
| Reviewed evidence entries | 15 |
| Curriculum modules | 10 |
| Source audit | 15 sources; no metadata problems |
| Final lint | 0 errors, 0 warnings |
| JSON strict validation | 0 errors, 0 warnings |
| Unit tests | 60 passed |
| Clippy | Passed with warnings denied |
| Exported literature ladder rows | **9** |
| Exported relations | **13** |
| Exported visual views | **2** |
| Exported curriculum prerequisites | **10 modules with prerequisite references** |
| Export diagnostics | 0 warnings |

The refreshed export reports no embedded diagnostics. `validate-report --strict` reports a clean result on the regenerated JSON.

## What Worked

### 1. The source and evidence boundary is real

`sources.csv` alone produces only cataloged evidence, which cannot support a final claim. A claim needs a reviewed or verified evidence link with a locator or support note. Claims marked `multiple_reviewed_sources` correctly required at least two distinct reviewed sources.

This is the strongest part of the harness. It prevents a bibliography from being presented as evidence merely because it exists.

### 2. Public and internal lanes are separated

The generated scaffold can contain assumptions, placeholders, and agent-facing notes. Final lint rejects those sections in a public report, and the final JSON omits `internal_context`. This matches the intended scaffold-versus-report boundary.

### 3. Access and currentness metadata are actionable

The manifest records access status, route, budget guidance, date, role, and curricular use. Current and frontier claims acquire structured `as_of` and `review_after` markers, and dated sources cannot postdate the claim's review date.

### 4. The field map itself is substantively good

The report extracts:

- a generative problem rather than a topic list;
- field objects and representations;
- measurement and evidentiary warrants;
- threshold concepts;
- common reasoning failures;
- a learning sequence that culminates in a claim-to-cell audit.

This is a plausible demonstration of what SoK should produce.

## Original Release-Blocking Gaps

The following gaps describe the original trial that motivated the repair plan. The refreshed artifact rows above supersede the old zero-count export facts for this evaluation bundle.

### 1. The structured source of truth drops the most important structure

The output contract says `sok-report.json` is the structured source of truth and that visual views should be relation-backed. In this trial, `export-json` explicitly initializes `relations` and `visual_views` as empty and does not convert the Markdown knowledge map.

The human Markdown contains a meaningful relation chain:

`material chemistry → transport → interface evolution → contact and fracture → cell metrics → manufacturing`

None of that chain exists in the JSON. Strict validation therefore certifies a container, not the central knowledge structure.

Required change:

- add a canonical relation input surface, such as a `Relations` table or a separate structured relations file;
- resolve its endpoints to report entities during export;
- require either a non-empty relation set for substantial reports or an explicit, reasoned waiver;
- generate or accept `visual_views` only from those validated relations.

### 2. Strict validation ignores export warnings embedded in the report

`export-json` emitted four warnings because it discarded public sections. `validate-report --strict` returned no warnings because it validates the report payload but not its embedded export diagnostics.

This makes the strict lane give a misleading green result after lossy conversion.

Required change:

- in strict mode, fail a final human report when embedded export diagnostics contain warnings or errors, or
- classify explicitly accepted information loss as a recorded waiver that the validator can inspect.

### 3. Curriculum dependencies are not exported

The curriculum is ordered from electrochemical grammar to a frontier-paper audit, but every exported `prerequisite_ids` value is empty. Validation checks prerequisite references only when they already exist; it does not check that the sequence has any dependency structure.

Required change:

- add prerequisite references to the canonical curriculum table;
- resolve module aliases or stable IDs during export;
- require dependency edges when the curriculum claims a prerequisite or spiral structure.

### 4. Evidence validation is syntactic, not semantic

The harness trusts an agent-authored `verification_status`, locator, and support note. It cannot determine whether the source was actually read or whether the note accurately represents it. More seriously, every non-background support kind is counted as usable evidence, including `contradicts`. A claim can therefore satisfy its evidence requirement without any supporting source.

Required change:

- require at least one `supports` link for an affirmative claim;
- let `qualifies` narrow a supported claim but not satisfy it alone;
- ensure `contradicts` creates a conflict or confidence diagnostic rather than support;
- require `reviewed_at` for reviewed evidence;
- reserve `verified` for a stronger, explicitly defined process.

### 5. Structural items are not evidence-linked at claim granularity

Core ideas, methods, and representations contain source IDs, but validation only checks that those source records exist. It does not require reviewed evidence that supports the specific structural extraction.

Required change:

- give knowledge items evidence links, or
- express every load-bearing structural item as an evidence-backed claim and connect it through explicit relations.

## Efficiency and Autonomy Findings

### 1. The initial scaffold added little domain value

`sok scaffold` classified “solid-state batteries” as generic `mixed` because profile inference is a small keyword list. Its 130-line output was mostly a reusable checklist and was largely replaced during research.

This is acceptable if the command is described as a generic framing template, but inefficient if it is expected to save substantive work.

Improvement:

- add `--profile` and `--profile-hint` overrides;
- expand only high-confidence domain signals;
- keep the generated scaffold shorter when classification confidence is low.

### 2. Final evidence authoring requires a brittle two-pass workflow

Evidence JSONL must reference content-derived source and claim IDs. The practical workflow was:

1. export a draft without reviewed evidence;
2. inspect generated IDs;
3. hand-author the evidence ledger;
4. re-export;
5. update evidence IDs if a claim sentence changes.

One terminology edit changed a claim ID during this trial. This makes normal editorial iteration fragile.

Improvement:

- add an `evidence-template` or `evidence link` command that resolves source and claim aliases;
- allow stable author-assigned aliases in Markdown and CSV;
- report orphaned evidence links with a suggested replacement after claim edits.

### 3. The currentness linter confuses time with electricity

The linter treats the word `current` as a temporal marker. It flagged phrases such as `current collector`, `current focusing`, and a paper title containing `critical stripping current`. Eight warnings had to be removed through terminology changes or date annotations.

This is both noisy and scientifically distorting.

Improvement:

- detect phrases such as `currently`, `latest`, `recent`, and explicit temporal constructions;
- exempt established technical compounds such as `current density`, `current collector`, and `current focusing`;
- add domain-neutral regression tests for polysemous words.

### 4. Source references are punctuation-fragile

Markdown source references are split on commas and semicolons, while scholarly titles and citations routinely contain commas. The run avoided author-year citations and comma-heavy titles to keep resolution stable.

Improvement:

- use an explicit source alias column;
- reference aliases with a syntax such as `[src:ohno-2020]`;
- keep human citation text separate from machine references.

### 5. Too much semantic authority remains with the LLM

The LLM still decides:

- the field boundary and dominant lenses;
- which sources are canonical or frontier;
- which text counts as reviewed;
- whether a source supports a claim;
- the structure, curriculum, and visual relations;
- the confidence assigned to each claim.

The harness constrains the format and catches missing metadata, but it does not independently challenge these judgments. It is therefore an orchestration and integrity frame, not a semantic verifier.

That role is reasonable, but the public description should state it clearly unless stronger checks are added. For high-confidence outputs, the workflow needs at least one of:

- a separate critic pass that searches for counterevidence;
- human review of the evidence ledger;
- reproducible source snapshots and locators;
- domain-specific machine checks for quantitative claims;
- explicit uncertainty and disagreement gates.

### 6. Installed skill instructions can drift from the repository

The installed `SKILL.md` used in this session differs from the checked-out version by one newer instruction about foundations. The repository CLI was correctly preferred and matched the source code, but the active skill instructions were stale.

Improvement:

- add a `sok doctor` or version command that reports CLI version, skill version, and source commit or content hash;
- make local development instructions explicitly reinstall the skill after instruction changes;
- include a parity test in the install workflow.

## Is the Harness Performing Its Intended Role?

| Intended role | Assessment |
|---|---|
| Prevent a flat topic list | **Partial.** The LLM-produced Markdown succeeds, but the validator does not require relations. |
| Ground claims in reviewed sources | **Partial pass.** Cardinality and metadata are enforced; semantic support is trusted. |
| Separate durable and frontier claims | **Pass.** Dates and temporal markers are present and checked. |
| Keep source access actionable | **Pass.** The manifest and audit work well. |
| Produce a reusable structured scaffold | **Pass for the refreshed fixture.** Concepts, claims, relations, visuals, literature ladder, and prerequisites survive. |
| Render a faithful human report | **Pass for the refreshed fixture.** HTML includes the literature ladder and relation-backed maps. |
| Reduce agent autonomy with deterministic gates | **Partial.** It reduces formatting and bookkeeping autonomy, but not the main semantic judgments. |

## Recommended Acceptance Test Before Release

Use this solid-state battery fixture as an end-to-end release gate. A release candidate should fail unless:

1. the final JSON contains a non-empty relation graph or an explicit waiver;
2. at least one visual view is backed by those relations;
3. curriculum steps after the first contain validated prerequisite links where the report claims a sequence;
4. the HTML contains the literature ladder and the relation-backed knowledge map;
5. a claim supported only by `contradicts` evidence fails;
6. an embedded export warning makes strict validation fail;
7. a source alias resolves even when the title contains commas;
8. `current density` does not trigger a temporal warning, while `latest` without a date does;
9. the installed skill and repository skill versions or hashes match.

## README Decision

The solid-state battery report is a good **content** use case, but it should not yet be used to advertise the validated visual-report pipeline. Keep it as an evaluation fixture. Update the README only after the JSON and HTML artifacts preserve the same structure shown in the Markdown report.
