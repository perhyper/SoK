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
