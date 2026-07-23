//! Domain-profile starters used by the Rust CLI.

pub(crate) struct ScaffoldProfile {
    pub(crate) classification: &'static str,
    pub(crate) profile_hypothesis: &'static str,
    pub(crate) scaffold_stance: &'static str,
    pub(crate) scoped_assumption: String,
    pub(crate) evidence_posture: &'static str,
    pub(crate) decomposition: String,
    pub(crate) orientation: String,
    pub(crate) structure_rows: Vec<StructureRow>,
    pub(crate) map_title: &'static str,
    pub(crate) map_caption: &'static str,
    pub(crate) concept_map: String,
    pub(crate) source_probe_rows: Vec<SourceRoleRow>,
    pub(crate) literature_rows: Vec<&'static str>,
    pub(crate) curriculum_rows: Vec<&'static str>,
    pub(crate) practice: &'static str,
    pub(crate) frontier_rows: Vec<&'static str>,
    pub(crate) visual_caption: &'static str,
    pub(crate) visual_map: String,
    pub(crate) source_priorities: &'static str,
    pub(crate) quality_focus: &'static str,
}

pub(crate) struct StructureRow {
    element: &'static str,
    field_pattern: &'static str,
    first_pass: &'static str,
    deeper_synthesis: &'static str,
    why_it_matters: &'static str,
}

pub(crate) struct SourceRoleRow {
    role: &'static str,
    status: &'static str,
    source_pattern: &'static str,
    diagnostic_use: &'static str,
    revision_rule: &'static str,
}

impl ScaffoldProfile {
    pub(crate) fn render_structure_rows(&self) -> String {
        self.structure_rows
            .iter()
            .map(|row| {
                format!(
                    "| {} | {} | {} | {} | {} |",
                    row.element,
                    row.field_pattern,
                    row.first_pass,
                    row.deeper_synthesis,
                    row.why_it_matters
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn render_source_role_rows(&self) -> String {
        self.source_probe_rows
            .iter()
            .map(|row| {
                format!(
                    "| {} | {} | {} | {} | {} |",
                    row.role, row.status, row.source_pattern, row.diagnostic_use, row.revision_rule
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub(crate) fn infer_scaffold_profile(field: &str) -> ScaffoldProfile {
    let normalized = field.to_ascii_lowercase();
    if contains_any(&normalized, INFRASTRUCTURE_DOMAIN_SIGNALS) {
        infrastructure_scaffold_profile(field)
    } else if contains_any(&normalized, ILL_STRUCTURED_DOMAIN_SIGNALS) {
        ill_structured_scaffold_profile(field)
    } else if contains_any(&normalized, FORMAL_DOMAIN_SIGNALS) {
        formal_scaffold_profile(field)
    } else {
        mixed_scaffold_profile(field)
    }
}

fn contains_any(value: &str, signals: &[&str]) -> bool {
    signals.iter().any(|signal| value.contains(signal))
}

const FORMAL_DOMAIN_SIGNALS: &[&str] = &[
    "algebra",
    "analysis",
    "automata",
    "category theory",
    "complexity",
    "compiler",
    "cryptography",
    "formal methods",
    "geometry",
    "logic",
    "mathematics",
    "number theory",
    "optimization",
    "probability",
    "programming language",
    "proof",
    "quantum information",
    "set theory",
    "statistics",
    "theoretical computer science",
    "topology",
    "type theory",
];

const ILL_STRUCTURED_DOMAIN_SIGNALS: &[&str] = &[
    "anthropology",
    "archival",
    "constitutional",
    "critical theory",
    "design",
    "education",
    "ethics",
    "governance",
    "history",
    "law",
    "legal",
    "literary",
    "literature",
    "management",
    "media studies",
    "policy",
    "political",
    "sociology",
    "strategy",
    "urban planning",
];

const INFRASTRUCTURE_DOMAIN_SIGNALS: &[&str] = &[
    "accelerator",
    "astronomy",
    "astrophysics",
    "bioinformatics",
    "climate",
    "collider",
    "earth observation",
    "epidemiology",
    "genomics",
    "gravitational wave",
    "high energy physics",
    "materials characterization",
    "metabolomics",
    "neuroscience",
    "oceanography",
    "particle physics",
    "proteomics",
    "remote sensing",
    "seismology",
    "synchrotron",
];

fn formal_scaffold_profile(field: &str) -> ScaffoldProfile {
    ScaffoldProfile {
        classification: "formal / well-structured",
        profile_hypothesis: "formal/theoretical: primary; pedagogical consensus: high; frontier-volatility: low for core foundations and conditional for research-entry paths; infrastructure/data-bound: usually low unless the field is computational or applied.",
        scaffold_stance: "Use prerequisite and proof graphs; make definitions, canonical examples, constructions, counterexamples, and validation standards visible before broad coverage.",
        scoped_assumption: format!("Treat {field} as a formal domain unless research shows that institutional or empirical infrastructure should dominate the path."),
        evidence_posture: "Durable claims can come from canonical texts and proof traditions; frontier claims still need dated surveys, problem lists, or recent papers.",
        decomposition: [
            format!("- Object families: identify the structures, maps, invariants, models, or languages that organize {field}."),
            "- Representation stack: name the definitions, notation, diagrams, equations, normal forms, or proof objects a reader must manipulate.".to_string(),
            "- Method strands: separate proof techniques, constructions, reductions, computations, counterexample practice, and classification programs.".to_string(),
            "- Boundary fields: mark prerequisites imported from adjacent mathematics, theory, statistics, physics, or computer science.".to_string(),
            "- Candidate scoped path: start with object grammar and canonical examples, then add methods, then frontier problems.".to_string(),
        ].join("\n"),
        orientation: format!("Read {field} as a domain where competence is shown by moving between precise definitions, canonical examples, transformations, and accepted proofs or validations. The scaffold should teach the learner what experts regard as a legitimate object, a meaningful construction, a decisive counterexample, and a research-level problem."),
        structure_rows: vec![
            StructureRow { element: "Generative questions", field_pattern: "Classification, equivalence, representation, existence, uniqueness, computability, or optimality questions.", first_pass: "State the central question as a relation among objects.", deeper_synthesis: "Connect the question to theorem families, problem lists, conjectures, or impossibility results.", why_it_matters: "Prevents the report from becoming a list of named topics." },
            StructureRow { element: "Core objects", field_pattern: "Definitions, structures, morphisms or transformations, canonical examples, and counterexamples.", first_pass: "Build a small gallery of examples and non-examples.", deeper_synthesis: "Use examples to test hypotheses, generalize definitions, and expose boundary cases.", why_it_matters: "Formal understanding depends on knowing what the definitions permit and forbid." },
            StructureRow { element: "Substantive structure", field_pattern: "Definitions, invariants, constructions, theorems, classifications, and canonical problem families.", first_pass: "Organize concepts by dependency and recurrence.", deeper_synthesis: "Explain which constructions generate later theory and which results reorganize the field.", why_it_matters: "Gives the learner a map of durable intellectual load-bearing points." },
            StructureRow { element: "Syntactic structure", field_pattern: "Proof, derivation, formal verification, reduction, counterexample, and reproducible computation where applicable.", first_pass: "Name the standard warrant for each claim type.", deeper_synthesis: "Compare proof styles and validation norms across subfields.", why_it_matters: "Teaches how the field knows, not only what it names." },
            StructureRow { element: "Representations", field_pattern: "Notation, diagrams, equations, normal forms, models, algorithms, or type/signature systems.", first_pass: "Translate one canonical example across two representations.", deeper_synthesis: "Show how representation choice changes proof difficulty or research tractability.", why_it_matters: "Many formal bottlenecks are representation bottlenecks." },
            StructureRow { element: "Methods", field_pattern: "Definition chasing, construction, induction, diagonalization, compactness, reduction, approximation, simulation, or classification.", first_pass: "Pair each method with a worked canonical problem.", deeper_synthesis: "Identify when a method fails and what replacement technique experts use.", why_it_matters: "Methods are the learner's route from reading to doing." },
            StructureRow { element: "Evidence standards", field_pattern: "Correct proof, verified derivation, accepted formal argument, benchmarked computation, or decisive counterexample.", first_pass: "Distinguish plausibility from proof or validation.", deeper_synthesis: "Track assumptions, lemmas, edge cases, and reproducibility constraints.", why_it_matters: "Keeps intellectual honesty at scholar level." },
            StructureRow { element: "Threshold concepts", field_pattern: "Abstraction, equivalence, invariance, duality, limiting behavior, universality, or complexity barriers.", first_pass: "Choose the concept that first reorganizes novice intuition.", deeper_synthesis: "Return to it through harder examples and frontier uses.", why_it_matters: "These concepts make later literature readable." },
            StructureRow { element: "Failure modes", field_pattern: "Memorizing definitions, ignoring counterexamples, confusing notation with structure, or treating a proof sketch as proof.", first_pass: "List the most tempting false shortcuts.", deeper_synthesis: "Design tasks that expose and repair the shortcut.", why_it_matters: "Smart entrants often fail through overconfident analogy." },
        ],
        map_title: "Concept and Prerequisite Map",
        map_caption: "Use this as the first dependency/proof graph; revise node labels after source review.",
        concept_map: [
            "flowchart LR",
            "  A[\"Core definitions\"] --> B[\"Canonical examples\"]",
            "  B --> C[\"Representations and notation\"]",
            "  C --> D[\"Proof or validation methods\"]",
            "  B --> E[\"Counterexamples\"]",
            "  D --> F[\"Classification or frontier problems\"]",
            "  E --> F",
        ].join("\n"),
        source_probe_rows: vec![
            SourceRoleRow { role: "Orientation / boundary", status: "Required", source_pattern: "Graduate notes, handbook chapter, expert overview, or field guide", diagnostic_use: "Tests the field boundary, subfield split, and entry vocabulary.", revision_rule: "Do not treat one syllabus or book as the whole field; revise scope if sources disagree." },
            SourceRoleRow { role: "Canonical foundation", status: "Required", source_pattern: "Canonical textbook, monograph, or seminal paper sequence", diagnostic_use: "Identifies stable definitions, constructions, theorem families, and recurring examples.", revision_rule: "Mark historically central sources that are poor first reads and pair them with a better entry source." },
            SourceRoleRow { role: "Method / warrant", status: "Required", source_pattern: "Proof-technique source, problem set, formalization notes, algorithms source, or methods chapter", diagnostic_use: "Shows how claims are warranted: proof, derivation, counterexample, computation, or validation.", revision_rule: "If method sources differ by subfield, split the curriculum path instead of averaging them." },
            SourceRoleRow { role: "Pedagogical sequence evidence", status: "Required", source_pattern: "Syllabi, lecture notes, problem ladders, qualifying exam lists, or expert reading lists", diagnostic_use: "Reveals stabilized teaching order and prerequisite expectations.", revision_rule: "Use curricula as evidence of pedagogical consensus, not as a boundary that excludes newer research." },
            SourceRoleRow { role: "Canonical examples / counterexamples", status: "Required", source_pattern: "Example-rich notes, counterexample collection, problem book, or canonical construction source", diagnostic_use: "Tests whether learners can distinguish definitions, examples, non-examples, and edge cases.", revision_rule: "If no dedicated source exists, extract examples from foundation and method sources." },
            SourceRoleRow { role: "Recent survey / frontier", status: "Conditional", source_pattern: "Problem list, recent survey, conference tutorial, preprint cluster, or expert roadmap", diagnostic_use: "Needed for research-entry, open-problem, or current-method claims.", revision_rule: "Waive for a core-foundations-only curriculum with rationale; include if the goal mentions research fluency or frontier participation." },
            SourceRoleRow { role: "Dataset / standard / infrastructure", status: "Conditional / usually waived", source_pattern: "Software benchmark, formal library, dataset, standard, or reproducibility source", diagnostic_use: "Tests whether the formal field has computational, empirical, or tooling-mediated knowledge practices.", revision_rule: "Waive when irrelevant; require when the field is computational, applied, benchmarked, or formalized in tools." },
        ],
        literature_rows: vec![
            "| Orientation | Graduate notes, handbook chapter, or survey that names the object grammar | notes/review/book | official URL/DOI/ISBN | open or library route | $0 or library preferred | 2 | Build an advance organizer before proofs | Prefer sources with canonical examples |",
            "| Foundation | Canonical monograph or seminal paper sequence establishing definitions and theorem families | book/paper | DOI/arXiv/ISBN/URL | publisher, preprint, or library | budget unknown; library preferred | 4 | Learn definitions, constructions, and proof idioms | Mark historically important sources that are poor first reads |",
            "| Core method | Source centered on proof techniques, algorithms, reductions, or model construction | book/paper/notes | DOI/arXiv/URL | official open or library route | $0 or library preferred | 4 | Practice expert moves on worked problems | Pair with exercises or reproduced proofs |",
            "| Canonical examples | Text or notes organized around examples, counterexamples, or problem sets | book/notes | ISBN/URL | library or official open URL | $0 or library preferred | 3 | Build example judgment and non-example detection | Use before frontier reading |",
            "| Synthesis | Recent survey or graduate-level overview connecting subfields | review/book | DOI/arXiv/URL | official route | $0 or library preferred | 3 | See how local methods compose into field structure | Check publication date for frontier claims |",
            "| Frontier | Problem list, recent survey, preprint cluster, or conference tutorial | paper/review/preprint | DOI/arXiv/URL | official route | $0 or library preferred | 5 | Locate open questions and method bottlenecks | Date every claim |",
        ],
        curriculum_rows: vec![
            "| Orientation | Object grammar and canonical examples | What counts as an object, map, invariant, or valid transformation here? | orientation source plus example-rich notes | Annotated example and non-example gallery | Can explain why each example satisfies or violates the definitions |",
            "| Core grammar | Representations, transformations, and proof idioms | How do experts move between notation, diagrams, models, and proof? | foundation plus core method sources | Reconstructed proof or validated derivation with dependency notes | Can identify assumptions, lemmas, and failure points |",
            "| Research fluency | Theorem family, construction family, or classification program | What makes a result central rather than merely technical? | synthesis source plus canonical papers | Theorem/problem map with method annotations | Can compare methods and choose one for a new problem |",
            "| Frontier participation | Open problems and counterexample practice | Where do current methods stop working? | dated frontier sources | Problem brief with prerequisites, known partial results, and possible entry routes | Can state a research question without flattening it into a topic |",
        ],
        practice: "Use proof reconstructions, counterexample logs, theorem dependency maps, small formalizations or reproducible computations, and problem briefs. Assessment should reward precision about assumptions, representation choice, and the difference between intuition, proof, and verified computation.",
        frontier_rows: vec![
            "| Classification or generality boundary | Find dated surveys or problem lists that show where current techniques stop | recent surveys, problem lists, conference tutorials | core definitions, canonical examples, proof methods | Boundaries often depend on subtle counterexamples or hidden assumptions |",
            "| Representation bottleneck | Identify cases where a change of notation, model, or invariant made progress possible | method papers, expert notes | representation stack and examples | The hard part may be choosing the right object language |",
            "| Constructive versus nonconstructive tension | Track whether existence, computability, or explicit construction matters in this field | foundational and frontier papers | proof standards and examples | Results can be true without giving usable constructions |",
        ],
        visual_caption: "Spiral the learner through the same formal objects at increasing precision.",
        visual_map: [
            "flowchart TB",
            "  P1[\"Pass 1: examples and object grammar\"] --> P2[\"Pass 2: definitions and proof moves\"]",
            "  P2 --> P3[\"Pass 3: theorem families and methods\"]",
            "  P3 --> P4[\"Pass 4: open problems and counterexamples\"]",
            "  P4 -. refine definitions .-> P2",
        ].join("\n"),
        source_priorities: "Prioritize canonical textbooks, proof-rich lecture notes, survey papers, problem lists, and expert-curated examples.",
        quality_focus: "Formal-field audit: the final report should include prerequisites, proof or validation standards, canonical constructions, counterexample practice, and a dated frontier/problem source.",
    }
}

fn ill_structured_scaffold_profile(field: &str) -> ScaffoldProfile {
    ScaffoldProfile {
        classification: "ill-structured / interpretive",
        profile_hypothesis: "interpretive/case-based: primary; methodological plurality: high; frontier-volatility: tied to live debates, institutions, or editions; curriculum-codification: uneven across schools and regions.",
        scaffold_stance: "Use cases, schools, interpretive lenses, debate maps, and methodological pluralism instead of forcing a single hierarchy.",
        scoped_assumption: format!("Treat {field} as a contested domain where expert judgment depends on cases, contexts, evidence traditions, and argument styles."),
        evidence_posture: "Durable claims need canonical cases and methods sources; current claims need dated debates, policy changes, venue discussions, or recent reviews.",
        decomposition: [
            format!("- Case field: identify the canonical cases, texts, sites, institutions, artifacts, or episodes through which {field} is taught."),
            "- Schools and lenses: map competing theories, interpretive traditions, normative commitments, and methodological camps.".to_string(),
            "- Evidence practices: separate doctrinal, archival, ethnographic, comparative, quantitative, critical, design, or practice-based warrants as relevant.".to_string(),
            "- Boundary disputes: name what adjacent fields would include or exclude differently.".to_string(),
            "- Candidate scoped path: start with cases and lenses, then teach how experts argue across disagreement.".to_string(),
        ].join("\n"),
        orientation: format!("Read {field} as a domain where structure is often a network of cases, concepts, institutions, and arguments. The scaffold should teach the learner how experts frame a problem, select evidence, compare interpretations, notice exceptions, and make defensible judgments under contestation."),
        structure_rows: vec![
            StructureRow { element: "Generative questions", field_pattern: "Meaning, authority, causation, legitimacy, interpretation, value conflict, design tradeoff, or institutional consequence questions.", first_pass: "State the central controversy and why reasonable experts disagree.", deeper_synthesis: "Trace how the question changes across cases, periods, jurisdictions, schools, or contexts.", why_it_matters: "Ill-structured fields move through argued judgment, not only answers." },
            StructureRow { element: "Core objects", field_pattern: "Cases, texts, artifacts, practices, actors, institutions, events, categories, and interpretive lenses.", first_pass: "Build a case/text/artifact constellation.", deeper_synthesis: "Use the constellation to compare concepts, exceptions, and institutional stakes.", why_it_matters: "The field's objects are often relational and context-bound." },
            StructureRow { element: "Substantive structure", field_pattern: "Concepts, schools, typologies, cases, histories, norms, and institutional arrangements.", first_pass: "Name the major lenses and the cases that reveal them.", deeper_synthesis: "Show how lenses reorganize evidence and what each lens obscures.", why_it_matters: "Prevents false neutrality and one-school summaries." },
            StructureRow { element: "Syntactic structure", field_pattern: "Interpretation, precedent, critique, triangulation, comparison, thick description, design rationale, or normative argument.", first_pass: "Pair each claim type with its warrant.", deeper_synthesis: "Compare warrant standards across schools and methods.", why_it_matters: "Teaches what counts as a good argument in context." },
            StructureRow { element: "Representations", field_pattern: "Case briefs, timelines, debate maps, actor maps, typologies, genealogies, or design rationales.", first_pass: "Represent one case through two lenses.", deeper_synthesis: "Show how representation changes what becomes salient or invisible.", why_it_matters: "Network representations fit contested domains better than strict trees." },
            StructureRow { element: "Methods", field_pattern: "Close reading, archival work, doctrinal analysis, comparative case analysis, ethnography, critique, stakeholder analysis, or reflective design practice.", first_pass: "Attach each method to an example artifact.", deeper_synthesis: "Name method limits and when mixed evidence is needed.", why_it_matters: "Method choice is often itself part of the argument." },
            StructureRow { element: "Evidence standards", field_pattern: "Coherence, fit with cases, source criticism, triangulation, precedent, explanatory power, practical consequence, or normative defensibility.", first_pass: "Distinguish evidence, interpretation, and value claim.", deeper_synthesis: "Mark what would persuade different expert communities.", why_it_matters: "A claim can be strong inside one warrant system and weak inside another." },
            StructureRow { element: "Threshold concepts", field_pattern: "Context dependence, contested categories, situated judgment, reflexivity, institutional path dependence, or plural validity.", first_pass: "Choose the first concept that disrupts naive rule-seeking.", deeper_synthesis: "Return to it through harder cases and live disputes.", why_it_matters: "The learner must tolerate principled ambiguity without becoming vague." },
            StructureRow { element: "Failure modes", field_pattern: "Forcing a single hierarchy, treating one school as the field, cherry-picking cases, moralizing before analysis, or confusing description with justification.", first_pass: "List the shortcut most likely for this learner.", deeper_synthesis: "Use paired cases or lens-switching tasks to expose it.", why_it_matters: "These failures produce confident but brittle reports." },
        ],
        map_title: "Debate and Case Network",
        map_caption: "Use this as the first debate and case network; replace generic school labels with field-specific traditions after research.",
        concept_map: [
            "flowchart LR",
            "  Q[\"Central contested question\"] --> C[\"Canonical cases or texts\"]",
            "  Q --> S1[\"School or lens A\"]",
            "  Q --> S2[\"School or lens B\"]",
            "  C --> E[\"Evidence and interpretation\"]",
            "  S1 --> D[\"Debate map\"]",
            "  S2 --> D",
            "  E --> J[\"Reflective judgment\"]",
            "  D --> J",
        ].join("\n"),
        source_probe_rows: vec![
            SourceRoleRow { role: "Orientation / boundary", status: "Required", source_pattern: "Handbook, companion, field guide, syllabus cluster, or expert overview", diagnostic_use: "Tests competing boundaries, schools, cases, and methodological camps.", revision_rule: "If sources center different regions, schools, or periods, state the scoped path rather than collapsing them." },
            SourceRoleRow { role: "Canonical case / corpus", status: "Required", source_pattern: "Primary texts, casebook, archive guide, legal cases, artifacts, sites, or curated corpus", diagnostic_use: "Identifies the objects experts repeatedly interpret, compare, or argue through.", revision_rule: "For language or philology paths, this role may become the central foundation source." },
            SourceRoleRow { role: "Method / warrant", status: "Required", source_pattern: "Methods text, doctrinal guide, archival method source, ethnographic source, critique, or interpretive theory", diagnostic_use: "Shows what counts as evidence, interpretation, critique, precedent, or persuasive judgment.", revision_rule: "If methods conflict, represent the conflict as part of the knowledge structure." },
            SourceRoleRow { role: "Pedagogical sequence evidence", status: "Required", source_pattern: "Syllabi, reading lists, seminar sequences, casebooks, or studio/fieldwork progressions", diagnostic_use: "Reveals how experts stage cases, lenses, and methods for learners.", revision_rule: "Use curricula as evidence of stabilized teaching practice, not as a boundary around valid inquiry." },
            SourceRoleRow { role: "School / lens source", status: "Required", source_pattern: "Foundational work from a major school plus critique or rival lens", diagnostic_use: "Tests whether the scaffold represents disagreement fairly.", revision_rule: "Do not let one school become the field unless the user explicitly scopes it that way." },
            SourceRoleRow { role: "Recent debate / institutional change", status: "Conditional", source_pattern: "Recent symposium, policy update, review essay, court decision, conference exchange, or field controversy", diagnostic_use: "Needed for live-debate, policy, institutional, or current-practice claims.", revision_rule: "Waive for historical or core-language competence with rationale; include if the goal includes contemporary research or practice." },
            SourceRoleRow { role: "Dataset / standard / infrastructure", status: "Conditional", source_pattern: "Archive metadata standard, digital corpus, repository, institutional protocol, or governance source", diagnostic_use: "Tests whether access, editions, metadata, or institutions shape what can be known.", revision_rule: "Waive when not relevant; require for digital humanities, law databases, policy, archives, or institutional fields." },
        ],
        literature_rows: vec![
            "| Orientation | Handbook, syllabus, or field guide that names schools, cases, and methods | handbook/syllabus/book | official URL/ISBN | open or library route | $0 or library preferred | 2 | Build a map of the contested terrain | Prefer sources that show plural approaches |",
            "| Canonical cases | Casebook, archive guide, primary texts, or curated artifact set | book/archive/primary source | ISBN/URL/archive ID | library, official archive, or open URL | $0 or library preferred | 3 | Learn the cases experts argue with | Track access and edition/version details |",
            "| Method and warrant | Methods text explaining interpretation, comparison, critique, or field evidence | book/paper/notes | DOI/ISBN/URL | official or library route | budget unknown; library preferred | 3 | Learn what counts as evidence and argument | Use to avoid one-school flattening |",
            "| School or lens | Foundational work from a major school or theoretical tradition | book/paper | DOI/ISBN/URL | library or publisher route | budget unknown; library preferred | 4 | Understand a strong internal view | Pair with critique or rival lens |",
            "| Synthesis | Review, companion, or annual review connecting schools and debates | review/book | DOI/ISBN/URL | official route | $0 or library preferred | 3 | Compare debates without losing nuance | Check whether it centers one geography or tradition |",
            "| Current debate | Recent article, symposium, policy report, or conference exchange | paper/report/symposium | DOI/URL | official route | $0 or library preferred | 4 | Date the live controversy and its stakes | Record who would disagree and why |",
        ],
        curriculum_rows: vec![
            "| Orientation | Cases, boundaries, and vocabulary | What is the field arguing about, and what counts as a case? | orientation plus canonical cases | Case constellation with boundary notes | Can state scope without pretending the field is settled |",
            "| Core grammar | Schools, lenses, and warrant styles | How do different experts make the same object mean different things? | method source plus school sources | Lens-switching memo on one case | Can separate evidence, interpretation, and value claim |",
            "| Research fluency | Comparative judgment across cases | Which interpretation survives comparison, exception, or countercase? | synthesis plus critique/debate sources | Debate matrix with rival warrants | Can argue a position while representing alternatives fairly |",
            "| Frontier participation | Live dispute or institutional consequence | What changed recently, and whose framework handles it best? | dated current debate sources | Position brief with evidence map and dissent | Can identify what further evidence would change the judgment |",
        ],
        practice: "Use case briefs, lens-switching memos, debate matrices, annotated primary-source notes, stakeholder or actor maps, and critique essays. Assessment should reward fair representation of rival views, explicit warrant standards, and the ability to revise a claim when a case or context changes.",
        frontier_rows: vec![
            "| Conceptual boundary dispute | Identify where experts disagree about what belongs inside the field | recent symposia, review essays, professional debates | core cases and schools | Boundary decisions change which evidence and values count |",
            "| Methodological tension | Compare what different methods reveal or hide about the same case | methods sources, critiques, paired studies | warrant styles and case knowledge | No single method exhausts the object |",
            "| Institutional or normative conflict | Track how a live controversy changes practice, law, policy, design, or interpretation | current reports, cases, articles, official records | concepts, institutions, and debate history | The hard part is linking analysis to consequence without collapsing nuance |",
        ],
        visual_caption: "Spiral by returning to the same cases through stronger lenses and harder objections.",
        visual_map: [
            "flowchart TB",
            "  P1[\"Pass 1: cases and vocabulary\"] --> P2[\"Pass 2: schools and warrant styles\"]",
            "  P2 --> P3[\"Pass 3: comparative judgment\"]",
            "  P3 --> P4[\"Pass 4: live debate and consequence\"]",
            "  P4 -. reinterprets cases .-> P1",
        ].join("\n"),
        source_priorities: "Prioritize handbooks, canonical cases or primary texts, methods sources, rival schools, critiques, and dated current debates.",
        quality_focus: "Ill-structured-field audit: the final report should include cases, schools, interpretive lenses, debate maps, warrant standards, and explicit handling of disagreement.",
    }
}

fn infrastructure_scaffold_profile(field: &str) -> ScaffoldProfile {
    ScaffoldProfile {
        classification: "infrastructure-bound / instrument-bound",
        profile_hypothesis: "empirical/infrastructure-bound: primary; data and standards mediation: high; frontier-volatility: high where instruments, datasets, standards, or collaborations change; formal/modeling lens: conditional and often important.",
        scaffold_stance: "Use an instrument-data-standards map; include facilities, datasets, collaborations, review bodies, statistical conventions, and strategic reports.",
        scoped_assumption: format!("Treat {field} as a field where knowledge is mediated by instruments, measurement regimes, data products, standards, and institutions."),
        evidence_posture: "Durable claims need canonical methods and instrumentation sources; current claims require dated facility, dataset, collaboration, standards, or strategic-report sources.",
        decomposition: [
            format!("- Measurement chain: identify the instruments, facilities, sensors, assays, protocols, or platforms that make {field} observable."),
            "- Data products: map raw observations, processed datasets, metadata conventions, benchmarks, catalogs, and uncertainty models.".to_string(),
            "- Collaborations and review infrastructure: name major consortia, facilities, standards bodies, review panels, and governance processes.".to_string(),
            "- Analysis stack: separate calibration, statistical inference, simulation, software, reproducibility, and validation practices.".to_string(),
            "- Candidate scoped path: follow the pipeline from instrument to data to analysis to collaboration-level frontier decisions.".to_string(),
        ].join("\n"),
        orientation: format!("Read {field} as a domain where the structure of knowledge is partly built into instruments, protocols, datasets, standards, and collaborations. The scaffold should teach the learner how observations are produced, cleaned, modeled, reviewed, and turned into claims or strategic priorities."),
        structure_rows: vec![
            StructureRow { element: "Generative questions", field_pattern: "What can be measured, at what resolution, with what uncertainty, and through which institutional pipeline?", first_pass: "State the question as a measurement or infrastructure problem.", deeper_synthesis: "Connect it to facility limits, dataset design, standards, and collaboration-level decisions.", why_it_matters: "The field's questions are constrained by what the infrastructure can make knowable." },
            StructureRow { element: "Core objects", field_pattern: "Instruments, facilities, protocols, samples, data products, simulations, standards, collaborations, and review reports.", first_pass: "Draw the pipeline from observation to claim.", deeper_synthesis: "Track dependencies among hardware, software, metadata, uncertainty, and governance.", why_it_matters: "Objects of knowledge include the systems that produce the evidence." },
            StructureRow { element: "Substantive structure", field_pattern: "Phenomena, measurement regimes, data models, benchmark objects, protocols, and strategic priorities.", first_pass: "Name the stable entities and the infrastructure that reveals them.", deeper_synthesis: "Explain how infrastructure changes reorganize what the field can ask.", why_it_matters: "Foundations may be datasets or instruments, not only texts." },
            StructureRow { element: "Syntactic structure", field_pattern: "Calibration, statistical inference, reproducibility, validation, peer review, collaboration review, and standards compliance.", first_pass: "Pair each claim with the validation path it requires.", deeper_synthesis: "Compare uncertainty practices and review conventions across data products or facilities.", why_it_matters: "Teaches how the field warrants claims at scale." },
            StructureRow { element: "Representations", field_pattern: "Instrument diagrams, data lineage maps, uncertainty budgets, schemas, simulation workflows, benchmark tables, and timelines.", first_pass: "Represent one result as a data lineage.", deeper_synthesis: "Show where uncertainty, selection effects, and standards enter the claim.", why_it_matters: "Infrastructure-bound fields become legible through pipelines." },
            StructureRow { element: "Methods", field_pattern: "Experimental design, observation planning, calibration, preprocessing, statistical modeling, simulation, software pipelines, and replication.", first_pass: "Attach methods to stages in the pipeline.", deeper_synthesis: "Identify method failure modes and infrastructure constraints.", why_it_matters: "Method and infrastructure are intertwined." },
            StructureRow { element: "Evidence standards", field_pattern: "Validated measurement, calibrated uncertainty, reproducible pipeline, standard-compliant metadata, collaboration review, and external cross-check.", first_pass: "Distinguish observation, processed data, inference, and claim.", deeper_synthesis: "Audit uncertainty, provenance, and review status.", why_it_matters: "Prevents treating data products as raw facts." },
            StructureRow { element: "Threshold concepts", field_pattern: "Instrument mediation, data provenance, uncertainty propagation, selection bias, standardization, collaboration governance, or simulation-model dependence.", first_pass: "Choose the first concept that changes how the learner reads results.", deeper_synthesis: "Return to it through datasets, standards, and frontier reports.", why_it_matters: "These concepts turn users of data into critical investigators." },
            StructureRow { element: "Failure modes", field_pattern: "Ignoring calibration, confusing dataset with phenomenon, overlooking metadata, treating official standards as optional, or missing collaboration governance.", first_pass: "List the infrastructure shortcut most tempting to newcomers.", deeper_synthesis: "Use a data lineage or uncertainty audit to expose it.", why_it_matters: "Infrastructure ignorance creates false confidence." },
        ],
        map_title: "Instrument, Data, and Standards Map",
        map_caption: "Use this as the first instrument/data/standards map; replace node labels with field-specific facilities, datasets, and review bodies after research.",
        concept_map: [
            "flowchart LR",
            "  I[\"Instruments or facilities\"] --> M[\"Measurement protocols\"]",
            "  M --> D[\"Data products and metadata\"]",
            "  D --> A[\"Analysis and statistical conventions\"]",
            "  A --> R[\"Collaboration or review bodies\"]",
            "  R --> S[\"Standards and strategic reports\"]",
            "  S --> F[\"Frontier decisions\"]",
        ].join("\n"),
        source_probe_rows: vec![
            SourceRoleRow { role: "Orientation / boundary", status: "Required", source_pattern: "Field overview, handbook, review, or strategic overview that explains the evidence pipeline", diagnostic_use: "Tests the scope, major phenomena, and infrastructure that make the field observable.", revision_rule: "If the field also has a strong formal/modeling branch, add a separate formal method path." },
            SourceRoleRow { role: "Canonical foundation", status: "Required", source_pattern: "Methods text, instrumentation source, canonical experiment, textbook, or foundational dataset paper", diagnostic_use: "Identifies durable concepts, measurement regimes, protocols, and benchmark objects.", revision_rule: "Do not reduce foundations to textbooks when datasets, protocols, or instruments carry the field." },
            SourceRoleRow { role: "Method / warrant", status: "Required", source_pattern: "Calibration, preprocessing, statistical, simulation, validation, or reproducibility source", diagnostic_use: "Shows how observations become claims and where uncertainty enters.", revision_rule: "Split methods by data product or instrument when one method source cannot cover the warrant structure." },
            SourceRoleRow { role: "Pedagogical sequence evidence", status: "Required", source_pattern: "Graduate syllabus, training school, workshop, collaboration tutorial, or methods course", diagnostic_use: "Reveals how learners are staged through instruments, data products, and analysis tasks.", revision_rule: "Use curricula as evidence of training consensus, not as a boundary excluding new tools or facilities." },
            SourceRoleRow { role: "Dataset / standard / infrastructure", status: "Required", source_pattern: "Official dataset, facility documentation, standard, schema, repository, collaboration report, or technical design report", diagnostic_use: "Tests the data lineage, access constraints, governance, and measurement limits.", revision_rule: "Cannot be waived for infrastructure-bound claims; if missing, downgrade confidence and mark source gap." },
            SourceRoleRow { role: "Recent survey / roadmap / frontier", status: "Conditional / often required", source_pattern: "Roadmap, decadal survey, standards update, collaboration review, or recent survey", diagnostic_use: "Needed for active instruments, datasets, standards, strategic priorities, and frontier bottlenecks.", revision_rule: "Waive only for historical or core-foundations paths; include for research fluency or current infrastructure claims." },
            SourceRoleRow { role: "Primary corpus / canonical case", status: "Conditional", source_pattern: "Benchmark task, exemplar dataset, canonical experiment, field site, or representative result", diagnostic_use: "Makes abstract infrastructure constraints inspectable through one concrete scholarly performance.", revision_rule: "Require when the learner needs practice artifacts; waive only for a high-level orientation." },
        ],
        literature_rows: vec![
            "| Orientation | Field overview that explains the infrastructure and evidence pipeline | review/handbook/report | DOI/URL/ISBN | official or library route | $0 or library preferred | 2 | See the measurement system before details | Prefer sources with diagrams or data-flow descriptions |",
            "| Instrument or facility | Official facility, instrument, protocol, or platform documentation | documentation/report/standard | official URL | official open URL | $0 | 3 | Understand what can be observed and what limits the observation | Track version and date |",
            "| Dataset or standard | Official dataset, catalog, schema, benchmark, or metadata standard | dataset/standard/software | DOI/URL/version | repository or standards body | $0 or license terms | 3 | Learn data provenance, access, and constraints | Record license and reuse limits |",
            "| Core method | Calibration, preprocessing, statistical, simulation, or workflow source | paper/notes/software | DOI/arXiv/URL | official route | $0 or library preferred | 4 | Understand how observations become claims | Pair with reproducibility task |",
            "| Collaboration review | Collaboration paper, technical design report, annual review, or review-board report | paper/report | DOI/URL | official route | $0 or library preferred | 4 | Learn review and consensus practices | Note authoring body and date |",
            "| Strategic frontier | Roadmap, decadal survey, standards update, or recent survey | report/review | DOI/URL | official route | $0 | 4 | Date priorities, bottlenecks, and upcoming infrastructure | Essential for currentness |",
        ],
        curriculum_rows: vec![
            "| Orientation | Infrastructure and observable phenomena | What can this field know only because of specific instruments or datasets? | overview plus facility source | Instrument-to-claim pipeline sketch | Can explain each pipeline stage without treating data as raw fact |",
            "| Core grammar | Data products, standards, and uncertainty | How do metadata, calibration, and uncertainty shape valid claims? | dataset/standard plus method source | Data lineage and uncertainty memo | Can distinguish observation, processed data, inference, and claim |",
            "| Research fluency | Reproducible analysis and collaboration review | What must be reproduced, audited, or reviewed before a result is credible? | method plus collaboration review sources | Reproduction or protocol audit plan | Can identify bottlenecks, assumptions, and governance checks |",
            "| Frontier participation | Strategic reports and infrastructure limits | Which next questions require new instruments, datasets, standards, or collaborations? | dated roadmap and frontier survey | Frontier brief tied to infrastructure constraints | Can connect open problems to concrete infrastructure requirements |",
        ],
        practice: "Use data lineage diagrams, calibration and uncertainty memos, standards comparisons, small reproducibility plans, protocol audits, and strategic-roadmap summaries. Assessment should reward provenance tracking, uncertainty reasoning, license/access awareness, and the ability to connect frontier claims to infrastructure constraints.",
        frontier_rows: vec![
            "| Instrument or facility limit | Identify the capability ceiling that blocks a current question | technical design reports, facility roadmaps, strategic reports | measurement chain and uncertainty concepts | Progress may require new hardware, protocols, or allocation decisions |",
            "| Dataset or standard bottleneck | Track where metadata, benchmark design, interoperability, or access limits inference | standards, dataset papers, repository docs | schemas, provenance, and licensing | Data reuse can fail for institutional reasons as much as technical ones |",
            "| Collaboration and review tension | Map how large collaborations, review bodies, or standards groups settle claims | collaboration papers, review procedures, policy documents | evidence standards and governance | Credibility depends on social and technical review infrastructure |",
        ],
        visual_caption: "Spiral by revisiting the same claim through infrastructure, data, analysis, and governance.",
        visual_map: [
            "flowchart TB",
            "  P1[\"Pass 1: infrastructure orientation\"] --> P2[\"Pass 2: data provenance and standards\"]",
            "  P2 --> P3[\"Pass 3: analysis, uncertainty, and review\"]",
            "  P3 --> P4[\"Pass 4: strategic frontier and infrastructure limits\"]",
            "  P4 -. changes what can be measured .-> P1",
        ].join("\n"),
        source_priorities: "Prioritize official facilities, datasets, standards, collaboration papers, technical design reports, statistical conventions, software repositories, and strategic roadmaps.",
        quality_focus: "Infrastructure-bound-field audit: the final report must include instruments, datasets, collaborations, standards, and strategic reports, plus uncertainty practices and official access routes.",
    }
}

fn mixed_scaffold_profile(field: &str) -> ScaffoldProfile {
    ScaffoldProfile {
        classification: "mixed / classification to verify",
        profile_hypothesis: "hybrid profile unresolved: estimate formal, empirical, interpretive, computational, practice, and infrastructure lenses during source review; use the strongest two to four lenses to shape the curriculum.",
        scaffold_stance: "Start with a mixed map, then choose whether the field behaves more like a formal prerequisite graph, an ill-structured debate network, an infrastructure pipeline, or a hybrid.",
        scoped_assumption: format!("Treat {field} as mixed until initial source review identifies the dominant knowledge structure."),
        evidence_posture: "Use canonical sources for durable structure and dated sources for frontier claims, standards, tools, datasets, or debates.",
        decomposition: [
            format!("- Subfields: identify major branches of {field} and whether each is formal, interpretive, practice-based, empirical, or infrastructure-bound."),
            "- Shared foundations: name concepts, methods, representations, and evidence standards that recur across branches.".to_string(),
            "- Divergent methods: mark where proof, interpretation, experiment, design, data infrastructure, or institutional review changes the curriculum.".to_string(),
            "- Boundary assumptions: state what this scaffold includes and what it leaves to an adjacent-field path.".to_string(),
            "- Candidate scoped path: pick the dominant structure after reviewing orientation sources.".to_string(),
        ].join("\n"),
        orientation: format!("Use this starter to discover the governing structure of {field} before committing to a single curriculum shape. The first research pass should decide which parts need prerequisite graphs, which need debate or case networks, and which need infrastructure or standards maps."),
        structure_rows: vec![
            StructureRow { element: "Generative questions", field_pattern: "Questions may involve classification, interpretation, design, measurement, explanation, or intervention.", first_pass: "Name the question types by subfield.", deeper_synthesis: "Explain which question type organizes the chosen path and which remain secondary.", why_it_matters: "Prevents an arbitrary one-size-fits-all scaffold." },
            StructureRow { element: "Core objects", field_pattern: "Concepts, cases, models, artifacts, datasets, instruments, institutions, or texts depending on subfield.", first_pass: "Build a typed inventory of objects.", deeper_synthesis: "Show relations among object types and which sources warrant claims about each.", why_it_matters: "Mixed fields often fail when object types are blurred." },
            StructureRow { element: "Substantive structure", field_pattern: "Core concepts, models, schools, cases, methods, tools, and institutions.", first_pass: "Group content by the structure it uses.", deeper_synthesis: "Identify load-bearing concepts that recur across subfields.", why_it_matters: "Keeps the report relational rather than encyclopedic." },
            StructureRow { element: "Syntactic structure", field_pattern: "Proof, interpretation, experiment, validation, critique, design review, or standards compliance.", first_pass: "Pair claim types with warrant types.", deeper_synthesis: "Compare how credibility differs across subfields.", why_it_matters: "The learner must know how each kind of claim is judged." },
            StructureRow { element: "Representations", field_pattern: "Concept maps, prerequisite graphs, debate maps, timelines, data pipelines, typologies, or system diagrams.", first_pass: "Choose one representation per major branch.", deeper_synthesis: "Integrate them into a single visual package with labeled relations.", why_it_matters: "The visual form should fit the field rather than dominate it." },
            StructureRow { element: "Methods", field_pattern: "Formal, empirical, interpretive, computational, design, or institutional methods.", first_pass: "Attach methods to representative tasks.", deeper_synthesis: "Name method limits and transfer conditions.", why_it_matters: "Method transfer is powerful but risky across boundaries." },
            StructureRow { element: "Evidence standards", field_pattern: "Correctness, coherence, explanatory power, reproducibility, external validity, practical effect, or compliance.", first_pass: "List standards by claim type.", deeper_synthesis: "Design assessment tasks that require using the right standard.", why_it_matters: "Mixed domains require evidence pluralism with discipline." },
            StructureRow { element: "Threshold concepts", field_pattern: "The first concepts that reorganize how an outsider sees the field's boundaries and warrants.", first_pass: "Identify candidate thresholds during orientation.", deeper_synthesis: "Test thresholds against readings and practice artifacts.", why_it_matters: "Thresholds determine the spiral sequence." },
            StructureRow { element: "Failure modes", field_pattern: "Flat topic lists, imported assumptions from one parent field, ignoring access and currentness, or using the wrong warrant standard.", first_pass: "Name likely errors for the learner profile.", deeper_synthesis: "Create tasks that reveal and correct the errors.", why_it_matters: "Mixed fields punish unexamined transfer." },
        ],
        map_title: "Domain Structure Map",
        map_caption: "Use this mixed map for the first research pass; split it into formal, debate, or infrastructure maps when the dominant structure becomes clear.",
        concept_map: [
            "flowchart LR",
            "  F[\"Frame field boundaries\"] --> O[\"Typed core objects\"]",
            "  O --> W[\"Warrant standards by claim type\"]",
            "  W --> M[\"Methods and representations\"]",
            "  M --> C[\"Curriculum path\"]",
            "  C --> R[\"Frontier or debate map\"]",
        ].join("\n"),
        source_probe_rows: vec![
            SourceRoleRow { role: "Orientation / boundary", status: "Required", source_pattern: "Handbook, syllabus cluster, survey, companion, field guide, or expert overview", diagnostic_use: "Tests whether the field is formal, interpretive, empirical, practice-based, infrastructure-bound, or hybrid.", revision_rule: "Revise the profile after source review; do not let the initial keyword guess settle the structure." },
            SourceRoleRow { role: "Canonical foundation", status: "Required", source_pattern: "Textbook, monograph, seminal paper, standard, primary corpus, canonical case, or foundational dataset", diagnostic_use: "Identifies the load-bearing concepts, objects, examples, and stabilized vocabulary.", revision_rule: "Choose foundations that fit the dominant lens rather than forcing a generic textbook model." },
            SourceRoleRow { role: "Method / warrant", status: "Required", source_pattern: "Methods source, proof source, empirical protocol, interpretive guide, design rationale, or standards documentation", diagnostic_use: "Shows how the field validates claims and what competent performance looks like.", revision_rule: "If warrant styles differ by subfield, preserve the plurality in the output." },
            SourceRoleRow { role: "Pedagogical sequence evidence", status: "Required", source_pattern: "Syllabi, lecture notes, course maps, reading lists, training programs, or assessment artifacts", diagnostic_use: "Reveals teachable order, prerequisites, bottlenecks, and recurring fundamental ideas.", revision_rule: "Use curricula as evidence of consensus, not as a conservative boundary around the field." },
            SourceRoleRow { role: "Recent survey / frontier", status: "Conditional", source_pattern: "Recent review, open-problems source, symposium, roadmap, standards update, or conference tutorial", diagnostic_use: "Needed only when making current, active-debate, tool, standard, or research-entry claims.", revision_rule: "Waive with rationale for stable core-skill paths; include for research fluency and frontier participation." },
            SourceRoleRow { role: "Dataset / standard / infrastructure", status: "Conditional", source_pattern: "Dataset, software, standard, facility, protocol, benchmark, repository, or governance source", diagnostic_use: "Tests whether material infrastructure mediates the field's knowledge.", revision_rule: "Require if the domain is data-, tool-, instrument-, or standard-mediated; otherwise mark waived." },
            SourceRoleRow { role: "Primary corpus / canonical case", status: "Conditional", source_pattern: "Primary source corpus, legal case, artifact set, canonical experiment, exemplar proof, or worked system", diagnostic_use: "Tests whether learners need concrete cases to see the field's structure.", revision_rule: "Require for interpretive, legal, historical, language, design, and case-based domains." },
        ],
        literature_rows: vec![
            "| Orientation | Handbook, syllabus, or survey that names subfields and methods | handbook/syllabus/review | DOI/URL/ISBN | official or library route | $0 or library preferred | 2 | Decide the dominant domain structure | Prefer sources that compare subfields |",
            "| Foundation | Canonical source for the most load-bearing concepts | book/paper/standard | DOI/arXiv/ISBN/URL | official or library route | budget unknown; library preferred | 3 | Anchor durable concepts and vocabulary | Verify whether it is still the right entry point |",
            "| Methods | Source explaining the field's main warrant or practice style | book/paper/notes | DOI/URL/ISBN | official route | $0 or library preferred | 3 | Learn how claims are judged | Pair with representative task |",
            "| Canonical case or example | Case, dataset, proof, artifact, system, or episode experts repeatedly use | case/dataset/paper/book | DOI/URL/ISBN | official route | $0 or library preferred | 3 | Make abstract structure inspectable | Choose one that reveals method and evidence |",
            "| Synthesis | Review or companion that connects branches | review/book | DOI/URL/ISBN | official or library route | $0 or library preferred | 3 | Prevent subfield siloing | Check date and scope |",
            "| Conditional frontier or debate | Recent survey, roadmap, symposium, standard, problem list, or waived role rationale | paper/report/preprint/waiver | DOI/arXiv/URL or rationale | official route, $0, or waived | $0 or library preferred | 4 | Date current priorities only when the goal requires current claims | Waive for stable core-skill paths with rationale |",
        ],
        curriculum_rows: vec![
            "| Orientation | Field boundary and domain-type decision | What kind of knowledge structure does this field actually use? | orientation source plus one canonical example | Domain-type memo with scoped path | Can justify the chosen scaffold shape |",
            "| Core grammar | Objects, representations, and warrants | What are the field's objects, and how are claims about them judged? | foundation plus methods sources | Core structure matrix with examples | Can match claim types to evidence standards |",
            "| Research fluency | Representative scholarly performance | What does competent work look like in this domain? | canonical case/example plus synthesis | Reproduction, critique, proof, case, or design artifact | Can perform a partial expert task with explicit assumptions |",
            "| Conditional currentness check | Live problem, debate, standard, infrastructure shift, or explicit waiver | Does this goal require current research, active tools, standards, or frontier participation? | dated frontier source or waiver rationale | Source-role decision memo with currentness audit | Can distinguish durable foundations from current claims, or justify why no frontier source is needed |",
        ],
        practice: "Use a domain-type memo, source role matrix, concept map with labeled relations, representative task artifact, and frontier currentness audit. Assessment should reward choosing the right structure for the field instead of forcing one template.",
        frontier_rows: vec![
            "| Dominant-structure uncertainty | Initial research should decide whether the frontier is formal, interpretive, infrastructure-bound, or hybrid | orientation sources, recent surveys, field handbooks | field boundaries and warrant standards | The wrong structure will produce the wrong curriculum |",
            "| Cross-subfield translation | Identify concepts or methods that travel poorly across branches | synthesis sources, critiques, methods papers | core object inventory | Transfer can hide incompatible assumptions |",
            "| Currentness-sensitive claim | Track active tools, standards, datasets, debates, or problem lists with absolute dates | recent reports, standards, surveys, preprints | source access and date audit | Mixed fields often change at uneven speeds |",
        ],
        visual_caption: "Start with a domain-classification spiral, then specialize the map after source review.",
        visual_map: [
            "flowchart TB",
            "  P1[\"Pass 1: classify domain structure\"] --> P2[\"Pass 2: core objects and warrants\"]",
            "  P2 --> P3[\"Pass 3: representative scholarly task\"]",
            "  P3 --> P4[\"Pass 4: dated frontier or debate\"]",
            "  P4 -. revises scope .-> P1",
        ].join("\n"),
        source_priorities: "Prioritize orientation sources that reveal domain type, then add foundations, methods, canonical examples, synthesis, and dated frontier material.",
        quality_focus: "Mixed-field audit: the final report should explicitly justify the chosen domain structure and avoid importing the wrong evidence standard from an adjacent field.",
    }
}
