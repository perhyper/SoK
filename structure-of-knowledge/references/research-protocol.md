# SoK Research Protocol

## Research Loop

1. Seed the field.
   - Search for handbooks, graduate syllabi, introductory textbooks, field encyclopedias, professional societies, and major venues.
   - Capture alternate names, subfield names, and historical terms.
   - Create a preliminary domain profile: formal, interpretive, empirical, computational, infrastructure-bound, professional, emerging, interdisciplinary, or mixed. Treat it as a hypothesis to revise, not a final label.

2. Find canonical foundations.
   - Look for seminal papers, foundational books, classic experiments, canonical cases, standards, and widely taught methods.
   - Confirm centrality through citations, syllabi recurrence, review papers, and expert retrospectives.

3. Find the current frontier when the goal requires current state, research entry, active tools, standards, or debates.
   - Search for recent survey papers, annual reviews, "open problems", "grand challenges", "state of the art", benchmark reports, standards updates, and major conference keynotes or tutorials.
   - Use absolute dates for current claims.

4. Compare perspectives when disagreements, cases, schools, or warrant differences organize the field.
   - Identify schools, debates, methodological splits, and critiques.
   - Include dissent where it changes the curriculum or interpretation of evidence.

5. Convert sources into artifact roles.
   - For each important source, decide whether it is best used as orientation, foundation, method, canonical case, synthesis, frontier, critique, or pedagogical sequence when the goal calls for one.
   - Record how the user can actually obtain the source: book, paper, preprint, lecture notes, standard, dataset, software, DOI, arXiv ID, ISBN, publisher page, library route, open URL, or expected purchase/subscription cost.
   - Treat syllabi and existing curricula as evidence of pedagogical consensus, not as the field boundary.

## Source Role Probe

Use source roles as diagnostics, not quotas. The goal is to discover which evidence roles are necessary for this field and user goal.

Usually required:

- Orientation / boundary: identifies field scope, subfields, vocabulary, and adjacent-field boundaries.
- Canonical foundation: anchors durable concepts, objects, examples, cases, methods, or standards.
- Method / warrant: shows how the field validates claims, performances, interpretations, proofs, measurements, or designs.
Conditional:

- Pedagogical sequence evidence: required when the goal includes learning, curriculum, onboarding, or staged practice.
- Recent survey / frontier: required for research-entry, current-tool, active-debate, recent-standard, or open-problem claims. It may be waived for stable core-skill paths such as basic language competence, with rationale.
- Dataset / standard / infrastructure: required when instruments, repositories, standards, software, benchmarks, or institutions mediate the field's knowledge.
- Primary corpus / canonical case: required for philology, law, history, literature, design, interpretive domains, case-based professions, and fields where expert judgment is taught through exemplars.
- Review / synthesis: useful when subfields are fragmented or when the learner needs a bridge across parent disciplines.

For every conditional role, mark it as `required`, `conditional`, `optional`, or `waived`, and explain the rationale. Do not fill source baskets by count alone.

## Search Query Patterns

Use combinations like:

- `[field] handbook core concepts`
- `[field] graduate syllabus foundational papers`
- `[field] survey paper open problems`
- `[field] annual review recent advances`
- `[field] textbook prerequisites`
- `[field] history seminal paper`
- `[field] methods benchmark standard`
- `[field] controversy debate critique`
- `[field] roadmap grand challenges`

For interdisciplinary fields, repeat searches for each parent discipline and then search the bridge terms.

## Source Ranking

Prefer sources in this order when available:

1. Primary literature, foundational books, standards, official documentation, or field society publications.
2. Peer-reviewed reviews, surveys, annual reviews, and major handbooks.
3. University syllabi from recognized programs, lecture notes by field experts, and publisher textbook pages.
4. High-quality technical blogs or essays by recognized researchers, mainly for context or recent tool practice.
5. General web explainers only for orientation, never as sole support for key claims.

## Evidence Matrix

Maintain a compact matrix while researching:

| Source | Date | Type | Identifier | Access route | Budget | Layer | Why it matters | Confidence | Use in curriculum |
|---|---:|---|---|---|---:|---|---|---|---|
|  |  | book/paper/preprint/review/syllabus/standard | DOI/arXiv/ISBN/URL | open/publisher/library/purchase/subscription |  | orientation/foundation/method/frontier/debate |  | high/medium/low |  |

In the JSON-first workflow, keep `sources.csv` and reviewed evidence JSONL distinct. `sok ingest last` may normalize source rows into `cataloged` evidence candidates for the current run, but cataloged entries do not satisfy claim evidence. A claim is supported only by reviewed or verified evidence with a usable locator or support note.

Reviewed evidence must also record support semantics. Use `support_kind: supports` only for evidence that affirmatively supports the claim. Use `qualifies` for scope limits or conditions, `contradicts` for conflicts, and `background` or `example` for context. Only reviewed or verified `supports` evidence with `reviewed_at` plus a locator or support note can satisfy final-report claim support.

Typed core sidecars may carry reviewed source roles, evidence links, currentness metadata, relations, and learning-path prerequisites directly into validation. They do not replace source review. WorkResult sidecars are proposed patches, not authoritative edits; accept them only after core validation, and record human corrections separately from validation failures when scoring conformance fixtures.

## Currentness Rules

- Browse or otherwise verify any claim about "current", "recent", "latest", active standards, tools, datasets, leaderboards, regulations, or open problems.
- Use absolute dates for time-sensitive claims.
- Distinguish durable foundations from current frontier items.
- If sources conflict, report the disagreement and explain what would change the learning path.

## Inclusion Criteria

Include a source when it satisfies at least one criterion:

- It defines a core object, method, or standard of evidence.
- It is a canonical case through which experts teach the field.
- It marks a historical turn or conceptual reorganization.
- It synthesizes scattered work for learners.
- It represents a live frontier, open problem, or major disagreement.
- It provides a practical artifact: dataset, tool, protocol, benchmark, or standard.

## Citation Expectations

For final reports:

- Cite every non-obvious factual claim about literature, history, current state, and controversies.
- Do not overquote. Prefer paraphrase with links.
- Give enough bibliographic detail for the user to find the source.
- Separate "recommended first reading" from "historically important but difficult."
- For every recommended reading, say how to find it. If it is a book, mark it as a book and provide ISBN or publisher/library route when available. If it is a paper, provide DOI, arXiv ID, stable URL, or venue information. If it is paywalled or paid, estimate the budget or state that institutional/library access is the preferred route.
- Before rendering or publication, export the bounded final report to `sok-report.json` and run `sok validate-report --strict`. Treat that JSON as the structured source of truth; HTML is only a view of validated public fields. Strict validation also checks embedded export diagnostics, relation and visual endpoints, literature-ladder source references, curriculum prerequisites, currentness metadata, and public/internal boundary separation.
- For regression or migration checks, run `sok eval conformance --fixtures <directory>` over saved fixtures. This scores deterministic validators only; it must not browse, call models, or repair artifacts in place.
