//! Provisional discovery lenses used by the Rust CLI.
//!
//! These profiles narrow the first source search. They must never determine the
//! final report architecture; source-reviewed field findings do that later.

pub(crate) struct DiscoveryProfile {
    pub(crate) classification_hint: &'static str,
    pub(crate) why_this_hint: &'static str,
    pub(crate) lenses: Vec<DiscoveryLens>,
    pub(crate) questions: Vec<&'static str>,
    pub(crate) source_roles: Vec<SourceRoleProbe>,
}

pub(crate) struct DiscoveryLens {
    name: &'static str,
    inspect: &'static str,
    revise_when: &'static str,
}

pub(crate) struct SourceRoleProbe {
    role: &'static str,
    inspect: &'static str,
    decision_rule: &'static str,
}

impl DiscoveryProfile {
    pub(crate) fn render_lenses(&self) -> String {
        self.lenses
            .iter()
            .map(|lens| {
                format!(
                    "| {} | {} | {} | untested |",
                    lens.name, lens.inspect, lens.revise_when
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn render_questions(&self) -> String {
        self.questions
            .iter()
            .map(|question| format!("- {question}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn render_source_roles(&self) -> String {
        self.source_roles
            .iter()
            .map(|role| {
                format!(
                    "| {} | {} | {} | undecided |",
                    role.role, role.inspect, role.decision_rule
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub(crate) fn infer_discovery_profile(field: &str) -> DiscoveryProfile {
    let normalized = field.to_ascii_lowercase();
    if contains_any(&normalized, INFRASTRUCTURE_DOMAIN_SIGNALS) {
        infrastructure_discovery_profile()
    } else if contains_any(&normalized, ILL_STRUCTURED_DOMAIN_SIGNALS) {
        interpretive_discovery_profile()
    } else if contains_any(&normalized, FORMAL_DOMAIN_SIGNALS) {
        formal_discovery_profile()
    } else {
        open_discovery_profile()
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
    "cryptography",
    "formal methods",
    "geometry",
    "logic",
    "mathematics",
    "number theory",
    "optimization",
    "probability",
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
    "constitutional",
    "critical theory",
    "design",
    "education",
    "ethics",
    "governance",
    "history",
    "law",
    "legal",
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

fn common_source_roles() -> Vec<SourceRoleProbe> {
    vec![
        SourceRoleProbe {
            role: "Boundary / orientation",
            inspect: "How experts delimit the field and name its recurring questions and objects.",
            decision_rule: "Required unless the scope is already explicitly bounded by the user.",
        },
        SourceRoleProbe {
            role: "Foundation / canonical corpus",
            inspect:
                "Which concepts, cases, results, or practices recur across authoritative accounts.",
            decision_rule:
                "Require the source form that actually carries durable knowledge in this field.",
        },
        SourceRoleProbe {
            role: "Method / warrant",
            inspect: "How the field produces, tests, interprets, or rejects claims.",
            decision_rule: "Required; split by community when warrant standards differ.",
        },
        SourceRoleProbe {
            role: "Pedagogical sequence",
            inspect: "Which dependencies or threshold concepts repeatedly shape expert teaching.",
            decision_rule: "Required for learning, curriculum, onboarding, or staged-practice goals; otherwise optional or waived. Never use it as the field boundary.",
        },
        SourceRoleProbe {
            role: "Current synthesis / frontier",
            inspect: "Which questions, methods, standards, or disagreements are changing now.",
            decision_rule: "Required only when the goal includes current state or research entry.",
        },
    ]
}

fn formal_discovery_profile() -> DiscoveryProfile {
    DiscoveryProfile {
        classification_hint: "formal or theory-led candidate",
        why_this_hint:
            "The field name suggests that objects, transformations, invariants, and proof may organize part of the domain.",
        lenses: vec![
            DiscoveryLens {
                name: "Object grammar",
                inspect: "Definitions, canonical examples, non-examples, transformations, and equivalence.",
                revise_when: "Cases, instruments, institutions, or empirical practices carry more explanatory weight than formal objects.",
            },
            DiscoveryLens {
                name: "Dependency and invariance",
                inspect: "Prerequisites, constructions, theorem families, invariants, and counterexamples.",
                revise_when: "The field is better organized by a process, controversy, chronology, or problem ecology.",
            },
            DiscoveryLens {
                name: "Proof and validation",
                inspect: "Accepted proof styles, derivations, reductions, computations, and failure tests.",
                revise_when: "Multiple communities use materially different warrants.",
            },
        ],
        questions: vec![
            "Which objects and transformations recur across independent boundary sources?",
            "Which relations are genuinely load-bearing: prerequisite, equivalence, construction, classification, or limitation?",
            "Which representations make expert work possible, and when do they mislead?",
            "Do applications or computational infrastructure reorganize what appears to be a purely formal field?",
        ],
        source_roles: common_source_roles(),
    }
}

fn interpretive_discovery_profile() -> DiscoveryProfile {
    DiscoveryProfile {
        classification_hint: "interpretive or contested candidate",
        why_this_hint:
            "The field name suggests that cases, institutions, schools, or competing interpretations may organize part of the domain.",
        lenses: vec![
            DiscoveryLens {
                name: "Cases and contexts",
                inspect: "Canonical cases, texts, artifacts, episodes, institutions, and boundary cases.",
                revise_when: "Stable formal objects or a shared causal mechanism better explains the field.",
            },
            DiscoveryLens {
                name: "Schools and disputes",
                inspect: "Competing lenses, normative commitments, interpretive traditions, and live controversies.",
                revise_when: "Apparent schools are historical labels rather than active organizing structures.",
            },
            DiscoveryLens {
                name: "Situated warrants",
                inspect: "Source criticism, precedent, comparison, triangulation, interpretation, and normative argument.",
                revise_when: "One warrant system is broadly shared or the goal concerns only one bounded practice.",
            },
        ],
        questions: vec![
            "Which cases or texts change the meaning of the field's central concepts?",
            "Where do expert communities disagree about evidence rather than merely conclusions?",
            "Which institutions or historical conditions make the current structure intelligible?",
            "Would a debate network, case constellation, genealogy, or comparative matrix reveal more than a hierarchy?",
        ],
        source_roles: common_source_roles(),
    }
}

fn infrastructure_discovery_profile() -> DiscoveryProfile {
    DiscoveryProfile {
        classification_hint: "empirical or infrastructure-bound candidate",
        why_this_hint:
            "The field name suggests that instruments, data products, collaborations, or standards may shape what can be known.",
        lenses: vec![
            DiscoveryLens {
                name: "Phenomenon-to-data chain",
                inspect: "Phenomena, instruments, acquisition, calibration, processing, models, and inference.",
                revise_when: "The field is organized mainly by formal theory or clinical/professional decisions.",
            },
            DiscoveryLens {
                name: "Infrastructure and coordination",
                inspect: "Facilities, datasets, software, standards, collaborations, governance, and access.",
                revise_when: "Infrastructure supports the field but does not shape its central questions or warrants.",
            },
            DiscoveryLens {
                name: "Uncertainty and validation",
                inspect: "Error budgets, benchmarks, controls, replication, sensitivity, and model comparison.",
                revise_when: "Different subfields require separate validation chains.",
            },
        ],
        questions: vec![
            "Where does the object of study become an observation, dataset, or model output?",
            "Which infrastructure choices constrain the questions that can be asked?",
            "Which uncertainty transformations are load-bearing but usually hidden?",
            "Would a pipeline, multiscale system, actor network, or decision flow best expose the field's structure?",
        ],
        source_roles: common_source_roles(),
    }
}

fn open_discovery_profile() -> DiscoveryProfile {
    DiscoveryProfile {
        classification_hint: "open or mixed candidate",
        why_this_hint:
            "The field name alone does not justify a dominant organizing lens.",
        lenses: vec![
            DiscoveryLens {
                name: "Objects and concepts",
                inspect: "Recurring objects, categories, representations, and transformations.",
                revise_when: "They are local vocabulary rather than the field's organizing structure.",
            },
            DiscoveryLens {
                name: "Practices and warrants",
                inspect: "Methods, evidence standards, decision rules, and expert performances.",
                revise_when: "Different communities require separate maps.",
            },
            DiscoveryLens {
                name: "History, institutions, and infrastructure",
                inspect: "Cases, actors, facilities, standards, datasets, and path dependencies.",
                revise_when: "They provide context but do not organize the field's knowledge.",
            },
        ],
        questions: vec![
            "What questions recur across independent descriptions of the field?",
            "What kinds of things are manipulated, compared, interpreted, measured, or built?",
            "Which relations make those elements cohere rather than remain a topic list?",
            "Which candidate organizing form survives comparison against the sources and the user's goal?",
        ],
        source_roles: common_source_roles(),
    }
}
