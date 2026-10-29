# Research Protocol

## Research Loop

1. Establish names and boundaries through handbooks, textbooks, field
   encyclopedias, societies, venues, and graduate syllabi. Capture alternate
   names, subfields, historical terms, and a revisable domain profile.
2. Find canonical foundations: seminal papers, books, experiments, cases,
   standards, and methods. Check centrality through independent reviews,
   syllabi recurrence, citations, and expert retrospectives.
3. When currentness matters, inspect recent surveys, annual reviews, open-problem
   lists, benchmarks, standards, and conference material. Distinguish established
   foundations from changing frontier claims and record absolute dates.
4. Compare schools, methodological splits, critiques, and dissent when they
   affect interpretation or learning. For interdisciplinary work, inspect each
   parent field and the bridge literature.
5. Assign each source a role in the scaffold and requested artifact. Record
   access while researching; choose the report architecture after reviewing the
   field's elements and relations.

Useful query terms include `[field] handbook`, `foundational papers`, `methods`,
`graduate syllabus`, `survey open problems`, `history`, `benchmark standard`,
and `controversy`. Adapt searches to gaps in the role probe below.

## Scholarly Framing Gate

For a substantial task that asks for academic perspectives, core ideas, a field
map, or theory-informed analysis, complete this gate **before selecting the
report architecture or evaluating the target corpus**. Initial corpus inspection
may identify questions; it cannot by itself validate the evaluative framework.

### Separate the object from the warrant

Distinguish sources that are **objects of analysis** (local rules, meeting records,
datasets, project documents, or cases) from sources that **justify the analytical
lens** (scholarly foundations, methods, syntheses, or professional standards).
One source may serve both roles, but explain why. A document's authority over a
local practice does not establish the scholarly adequacy of that practice.

User-provided files normally add evidence; they do not silently narrow an existing
request for scholarly research to internal consistency checking. Inspect whether
the supplied corpus covers orientation, foundations, and warrants. If it does not,
research credible external literature for those missing roles. External here means
independent of the object under evaluation, not necessarily downloaded in this run.
Do not require new browsing merely to duplicate adequate supplied scholarship.

Honor explicit corpus-only or no-browsing constraints. Record the resulting limits
and label the product accordingly; do not claim a literature-grounded field map
when its foundations remain unreviewed. Lack of time, a concise output target, or
the presence of attachments is not a sufficient reason to waive foundational
research. Keep research depth separate from presentation length.

### Review competing lenses and transfer limits

Record a compact matrix in the working notes:

| Candidate core idea or lens | Foundational/method source and reviewed locator | Competing explanation or limiting evidence | Why it changes this analysis | Transfer limits |
|---|---|---|---|---|

Read the relevant source passages. Search snippets, abstracts, publisher metadata,
and citation counts can establish discovery or bibliographic facts, but cannot
stand in for review of arguments they do not expose. Mark inaccessible foundations
as unreviewed; find an accessible substantive substitute or narrow the claim.
Check the centrality of purported canonical works through independent scholarly
uptake rather than declaring a convenient source canonical.

Compare plausible lenses and consequential counterarguments, then select, revise,
or reject them. Distinguish descriptive findings, explanatory theory, normative
criteria, binding local rules, and the analyst's proposals. When transferring a
framework between settings, state the mechanism that carries over, relevant
differences, and what the source cannot establish. In particular, a professional
standard or state-law principle is not automatically binding on a local association.

### Check scholarly sufficiency before delivery

Record the gate outcome in `notes/research-state.md` (or an equivalent existing
research log): required source roles and reviewed coverage; missing roles and
justified waivers; selected and rejected lenses; material counterevidence; and
which conclusions changed because of the literature. Trace central analytical
claims through **source passage → concept or warrant → application → limitation**.
Bibliography appended after the argument, without changing or testing it, does not
satisfy this gate. Use role coverage and argumentative relevance, not source quotas.

If a required role remains uncovered, continue research or explicitly report an
incomplete/preliminary product; do not silently relabel the gap as out of scope.
Before delivery, check the actual evidence record against this gate and the user's
original goal. CLI/schema validation checks artifact structure and declared support;
it does **not** certify adequate literature discovery, competing perspectives,
sound interpretation, or completion of the research protocol. Do not describe a
passing CLI check as proof that all research requirements were fulfilled.

## Source Role Probe

Roles diagnose missing evidence; they are not source-count quotas.

- **Orientation/boundary:** expert scope, vocabulary, questions, and interfaces.
- **Canonical foundation:** durable concepts, results, cases, or practices.
- **Method/warrant:** how claims, proofs, performances, or designs are validated.

These are normally required. Conditional roles depend on the field and goal:

- **Pedagogical sequence:** learning, curriculum, onboarding, or staged practice.
  Syllabi show stabilized teaching consensus, not the limits of the field.
- **Frontier:** research entry, current tools, active debates, standards, or open
  problems. Stable core-skill paths may waive it with a goal-specific rationale.
- **Data/standards/infrastructure:** instruments, repositories, collaborations,
  facilities, statistical conventions, and review bodies that mediate knowledge.
- **Primary corpus/canonical case:** interpretive and case-based fields such as
  history, philology, literature, law, design, and professional practice.
- **Synthesis:** bridges across fragmented subfields or parent disciplines.

Mark conditional roles required, conditional, optional, or waived, with rationale.
Split roles by community when warrant standards differ.

## Ranking and Inclusion

Prefer primary literature, foundational books, standards, official documentation,
and field societies; then peer-reviewed syntheses and handbooks; then expert
university notes, syllabi, and publisher pages. Use expert blogs for context or
recent practice, and general explainers only for orientation, never sole support
for key claims.

Include sources that define core objects or warrants, supply canonical cases,
mark conceptual changes, synthesize scattered work, represent meaningful disputes
or frontiers, or provide usable datasets, tools, protocols, and standards.
Separate good first readings from historically important but difficult works.

## Source Manifest and Access

Keep these fields in `sources.csv` (the CLI supplies a header-only template):

- Identity: `title`, `type`, `identifier`, `url`, publication/update `date`.
- Access: `access_status`, `access_route`, `budget_estimate`, `license`.
- Role: `layer`, `why_it_matters`, `use_in_curriculum`.

Types include book, paper, preprint, notes, course, standard, dataset, software,
archive, report, and syllabus. Use DOI, arXiv ID, ISBN, official URL, or a stable
catalog record. Mark books as books and provide publisher/library routes;
provide direct official links for open notes, standards, datasets, and software.

Record free, paid, paywalled, subscription, restricted, or unknown access using
the CLI's statuses. Give visible prices or `budget unknown; library preferred`,
with institutional access or interlibrary loan guidance where appropriate.
Explain accessible substitutes for inaccessible recommendations. Download policy
is in [agent-collaboration.md](agent-collaboration.md#source-downloads).

A research matrix may add confidence to these fields. Record why each source
matters and how it will be used, rather than merely collecting bibliography.

## Evidence and Currentness

Keep source metadata separate from reviewed evidence JSONL. Ingestion produces
`cataloged` candidates; it does not establish claim support. Apply the precise
support-link requirements in [output-contract.md](output-contract.md#strict-final-validation).
Typed sidecars carry reviewed findings but do not replace source review.

Browse or otherwise verify claims about current literature, recent results,
active debates, tools, standards, datasets, leaderboards, regulations, and open
problems. Record absolute dates and distinguish publication status and durable
foundations from frontier findings. Report conflicting evidence and its effect
on interpretation or the learning path.

Cite every non-obvious factual claim about literature, history, current state,
and controversies. Prefer paraphrase with links, enough bibliographic detail to
find the source, and explicit uncertainty. Before final delivery, use the
[CLI validation lane](cli-workflow.md#artifact-lanes).

## Continuity During Long Research

Maintain a compact `notes/research-state.md` as the run grows. Record the current
question and scope, stable claim/relation IDs, assumptions and boundary conditions,
competing interpretations, unresolved contradictions, source locators, decisions
with brief justifications, and the next evidence gaps. Link to full artifacts instead
of replacing them with successive summaries. This is an evidence and decision record,
not a transcript of private reasoning.

Before a major synthesis, handoff, or resumption after context loss, reread that record
and the relevant claims, sources, and opposing evidence. Check which conclusions rely
on changed assumptions. Preserve counterexamples and conditions when shortening prose;
if they no longer fit, link to their full treatment. Update the record after a material
finding or decision. Before delivery, compare the synthesis against these distinctions
and explain any omissions. Validation cannot perform this scholarly review.

Across reports, treat matching terms as candidates for review. Use report-qualified
identities and explicit equivalence assertions; never merge by label alone. Graph
snapshots preserve evidence for retrieval but do not establish its truth or relevance.
