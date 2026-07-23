# Evaluation of SoK v0 on Sample Domains

Date: 2026-06-19

Samples evaluated:

- `reports/examples/topology-sok-v0.md`
- `reports/examples/particle-physics-sok-v0.md`

## Evaluation Questions

1. Does the SoK frame produce something more useful than a reading list?
2. Does the frame preserve scholar-level rigor?
3. Does the substantive/syntactic distinction reveal the field's mode of inquiry?
4. Does the literature ladder include actionable access metadata?
5. Does the same framework adapt to different domain structures?

## Topology Findings

The framework is useful for topology because it prevents a flat list of books from masquerading as a curriculum. It surfaces invariance, functoriality, quotient construction, counterexample culture, exactness, and classification as the real structure.

Strengths:

- The substantive/syntactic distinction worked well.
- The curriculum naturally became a sequence of proof practices and invariant-building practices.
- The access metadata was actionable because several high-quality resources are free.

Weaknesses:

- The keyword "topology" is too broad. Without early decomposition, the curriculum risks mixing point-set topology, algebraic topology, differential topology, low-dimensional topology, HoTT, and TDA.
- The v0 scaffold should force a "domain decomposition" step for broad fields before committing to a path.
- Frontier mapping in pure mathematics needs problem-list culture, seminar culture, and subfield taxonomies, not only recent papers.

Improvement:

- Add a workflow rule: if the field is broad, first produce a subfield decomposition and either ask the user to choose or state a scoped assumption.
- Add a formal-domain rule: include proof techniques, counterexample practices, canonical constructions, and problem-list traditions.

## Particle Physics Findings

The framework is useful for particle physics, but only after expanding "syntactic structure" to include experimental and institutional knowledge infrastructure. Particle physics is not just QFT plus particles; it is QFT, detectors, statistical inference, collaborations, global reviews, and strategic planning.

Strengths:

- The framework identified the field's dual grammar: theoretical formalism and empirical event inference.
- Access metadata forced inclusion of PDG, CERN, Snowmass, and P5, which are essential for research orientation.
- The frontier map connected unsolved scientific questions to infrastructure and community planning.

Weaknesses:

- The original skill did not explicitly mention instrument-bound or infrastructure-bound sciences.
- A reading ladder alone is insufficient; learners need to understand collaborations, detectors, datasets, and statistical conventions.
- Budget metadata can be uncertain for textbooks. The skill should require "budget unknown; library preferred" rather than leaving paid sources vague.

Improvement:

- Add an infrastructure-bound science rule: map instruments, collaborations, datasets, standards, review bodies, and strategic reports.
- Strengthen source access metadata as a quality gate.

## Cross-Domain Conclusion

The SoK frame is promising because it made two very different domains legible:

- Topology became a curriculum of invariance, construction, proof, and subfield branching.
- Particle physics became a curriculum of fields, symmetries, prediction, detectors, statistical warrant, and frontier planning.

The key improvement is to make domain classification more operational. The skill should not merely label a domain as well-structured or interdisciplinary; it should change required artifacts accordingly.

## Implemented Changes Recommended

1. Add "broad field decomposition" to `SKILL.md`.
2. Add formal-domain and infrastructure-bound-domain clauses to `SKILL.md`.
3. Keep access metadata mandatory in `research-protocol.md`, `output-contract.md`, `sok scaffold`, and `sok-report.schema.json`.
4. In future, add an evaluation rubric script that scores sample reports against domain fit, source access, syntactic structure, and curriculum usefulness.
