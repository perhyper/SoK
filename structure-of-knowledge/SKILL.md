---
name: structure-of-knowledge
description: >-
  Evidence-grounded field maps, literature ladders, curricula, textbook and report
  architectures, visual knowledge maps, and research handoffs. Use for SoK,
  structure of knowledge, deep research reports, domain research delegation,
  or guided entry into an unfamiliar field.
---

# Structure of Knowledge

Turn a learning or research goal into a reusable map of a field: its knowledge
structure, warrants, sources, and a path from orientation to purposeful work.
The approach draws on Bruner's structure-of-knowledge curriculum tradition.

## Read as Needed

- For substantial research, read [research-protocol.md](references/research-protocol.md).
- Before composing a substantial report, read [output-contract.md](references/output-contract.md).
- Before the first artifact-producing CLI call, read [cli-workflow.md](references/cli-workflow.md).
- For delegation, textbook or corpus plans, or source downloads, also read
  [agent-collaboration.md](references/agent-collaboration.md).

A narrow conceptual answer needs only the relevant guidance, not a CLI workspace
or a full artifact package.

Keep each user's sources, prompts, reports, and derived examples in their private task workspace. Do not copy them into shared project files, fixtures, documentation, or release artifacts.

## Workflow

1. **Frame the request.** Establish field, audience, entry point, depth, goal,
   time budget, and format. Decompose broad fields into shared
   foundations and divergent paths. Ask one concise question only when ambiguity
   would materially change scope or architecture; otherwise state assumptions.
   Propose a revisable domain profile: formal, interpretive, empirical,
   computational, practice-based, infrastructure-bound, emerging,
   interdisciplinary, or mixed. A field-name lens is a search hypothesis.

2. **Review sources.** Use current web research or a user-provided corpus.
   For scholarly framing, complete the research protocol's
   [scholarly framing gate](references/research-protocol.md#scholarly-framing-gate)
   before choosing an architecture; supplied cases do not replace missing
   foundational or methodological literature.
   Build the role-based source probe and access records defined in the research
   protocol. Include instruments, facilities, collaborations, data, standards,
   and review bodies when they carry knowledge in the field. Treat syllabi as
   evidence of stabilized teaching consensus, not as the field's boundary.
   For long runs, maintain and reload the continuity record in the research protocol.

3. **Extract structure.** Identify generative questions, objects,
   representations, transformations, methods, warrants, examples, and failure
   modes. Record their actual forms: a case, institution, instrument, dataset,
   practice, or dispute may be as central as a concept. Separate substantive
   structure (what is known) from syntactic structure (how claims are warranted,
   disputed, and revised). Capture proof and counterexample practices in formal
   fields, and measurement, uncertainty, and infrastructure in empirical ones.
   Distinguish prerequisites from fundamental ideas revisited at greater depth.
   Identify threshold concepts and misconceptions. Foundations may be difficult.

4. **Choose an architecture.** After source review and element extraction,
   compare at least two plausible organizing forms against the reader's goal.
   Options include dependencies, causal systems, processes, scales, debates,
   cases, taxonomies, chronology, genealogy, or hybrids. Select the executive
   thesis, core ideas, necessary context, section purposes, and visual logic.
   Record the decision, rejected alternatives, and scope limits using the output
   contract. Public headings must follow the field, not the scaffold's sections.

5. **Design a learning path when requested.** Revisit central ideas through
   increasingly sophisticated tasks. Match the learner's entry point while
   preserving scholarly depth. Give modules questions, readings, practice
   artifacts, dependencies, and progress criteria. Use advance organizers,
   worked performances, and cases; contested fields need multiple lenses rather
   than an imposed hierarchy.

6. **Compose and validate.** Keep working scaffolds separate from reader-facing
   reports. Use `sok-report.json` as the downstream source of truth and HTML as
   its public view. Preserve field-native elements; legacy concept/method arrays
   are compatibility projections. Include literature, curriculum, frontier, and
   visual modules only when the goal warrants them. Relations require explicit
   author-owned data; do not infer them from layout, section order, or Mermaid.
   Follow the output contract and CLI lane, with strict validation before final
   delivery. Keep private context, diagnostics, and run/work sidecars out of the
   public narrative.

## Final Review

Confirm that the artifact explains load-bearing relations and inquiry methods,
not merely a topic list. Check dated currentness claims, usable source access,
evidence support, and reader-fit architecture. Structured validation checks
artifact contracts; the agent must still review the sources and reasoning.
