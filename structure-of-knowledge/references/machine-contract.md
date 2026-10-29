# Optional Machine Inputs and Compatibility

When reproducibility metadata is needed, artifact-producing commands may also write a
private `sok-run/v1` sidecar with `--run-manifest <sok-run.json>` . The run manifest
records command metadata, declared permissions, bounded input file hashes, output file
hashes, optional executor metadata, and a timestamp-independent semantic digest. Treat
it as disposable private run metadata; do not embed run manifests, executor details, or
private paths in `sok-report.json` or rendered HTML.

Machine input is opt-in. `sok export-json` may also read typed core sidecars with
`--knowledge <sok-knowledge.json>` , `--evidence-package <sok-evidence.json>` , and
`--pedagogy <sok-pedagogy.json>` , or typed fenced blocks in the Markdown:

````markdown
```sok-json sok-knowledge/v1
{
  "schema_version": "sok-knowledge/v1",
  "elements": []
}
```
````

Typed machine inputs feed directly into the `sok-knowledge/v1` , `sok-evidence/v1` , and
`sok-pedagogy/v1` core validators. They do not depend on localized Markdown table
headers. Markdown tables, `sok:surface` markers, and narrative prose remain supported
for human-authored inputs. When human and machine lanes provide different values for the
same stable ID or package field, the exporter keeps the first value deterministically
and emits an `export.machine-input` diagnostic instead of overwriting silently.

## Compatibility, Migration, and Rollback

Public `sok-report/v1` and `sok-report/v2` remain readable. New final human-report
exports use `sok-report/v2` ; unknown report, core, run, and work schema versions must
be rejected by validators rather than guessed.

ASCII stable IDs remain stable. Existing non-ASCII IDs do not need an automatic rewrite;
use `sok migrate-ids --input <sok-report.json> --output <id-map.json>` when an explicit,
reviewable old-to-new map is needed. Reference rewriting is a separate deliberate step
outside the command.

Core packages, run manifests, and work protocol files are additive sidecars.
`sok-knowledge/v1` , `sok-evidence/v1` , and `sok-pedagogy/v1` can be supplied to
`export-json` or omitted. `sok-run/v1` , `sok-work-order/v1` , and `sok-work-result/v1`
are private files for reproducibility and bounded exchange; removing them rolls the
workflow back to Markdown/source/evidence inputs without changing public report
payloads.

For bounded collaboration, a WorkOrder declares the current SoK task kind, required
capabilities, allowed core patch paths, and a default-deny permission envelope. A
WorkResult returns a typed refusal, degradation plan, or proposed patches against core
packages. Accepting a result writes a new validated core output file and never mutates
source inputs in place.
