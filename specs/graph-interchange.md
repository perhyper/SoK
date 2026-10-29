# Bounded graph interchange

`export-graph` derives `sok-graph/v1` from a `sok-graph-input/v1` manifest.
The JSON schemas define shape; the CLI additionally enforces reference closure,
strict report validation, review metadata, unique report namespaces, and input
protection. This is a rebuildable snapshot, not an authoritative database.

## Identity and revisions

- Assign a stable `report_id` to each authored report lineage. Paths are resolved
  relative to the manifest. A manifest contains one revision per report ID.
- An entity key is `(report_id, revision, entity_type, id)`. `revision` is the
  lowercase SHA-256 of the exact input file bytes, including metadata and spacing.
  Formatting changes therefore invalidate links, conservatively requiring review.
- Equal labels, local IDs, citations, or hashes across reports do not imply
  equivalence. Source and compatibility-projection nodes also remain separate.
- Optional identity links state `equivalent_to`, `related_to`, or `different_from`.
  They require existing revision-qualified endpoints from different reports,
  a reviewer, date, explanation, and existing source references. They are reviewed
  assertions, not node merges or automatic transitive equivalence.
- Export once without identity links to obtain keys. Review candidate matches,
  add links to the manifest, and export again. A stale revision is rejected.

## Assertions and provenance

Every authored relation and every claim/evidence link remains a separate assertion.
The snapshot carries full public entity data, report scope and metadata, qualified
source references, relation descriptions, evidence locators, review dates, support
kinds, claim confidence, temporal markers, and conflict reviews. A relation citation
alone is not promoted to reviewed evidence. Evidence links without IDs receive a
claim-and-position key scoped to the immutable report revision.

Only identical relation records may collapse. Different assertion IDs, descriptions,
or source sets remain distinct, even with identical endpoints and relation kinds.
Semantic relation kinds are defined in `sok-knowledge.schema.json`. Inverse aliases
reverse endpoints in both core and public import. Previously exported artifacts that
already collapsed distinctions cannot recover them without reviewing original inputs.
Older CLI versions may not understand newly preserved relation kinds.
Newly generated Markdown relation IDs include rationale and source references to
avoid collapsing contextual assertions. Existing explicit IDs remain unchanged;
review references when regenerating older reports.

A claim's optional `conflict_review` acknowledges opposing evidence without treating
it as affirmative support. It includes an explanation, review date, and exactly the
opposing source IDs. Invalid or older-than-evidence reviews fail validation; an
unreviewed contradiction still warns. Review dates do not prove a review happened,
and edits made without updating evidence dates cannot be detected by this check.
When changing evidence, update its review date and reconsider the conflict review.

## Limits

The exporter does not infer missing relations, evaluate scientific truth, check
narrative claims against sources, merge reports, resolve disagreements, or retrieve
context for an agent. Private run context, diagnostics, and local input paths are
excluded. Inputs remain unchanged; failed validation leaves an existing output intact.
Consumers can load snapshots into a graph store, but must preserve qualified keys and
assertions rather than convert all edges into globally accepted facts.
