use super::*;

#[test]
fn report_scaffold_uses_domain_hints_for_discovery_not_final_sections() {
    let formal = build_report_scaffold(
        "algebraic topology",
        "doctoral mathematician",
        "enter current research",
        16,
    );
    assert_contains(
        &formal,
        "| Provisional lens | formal or theory-led candidate |",
    );
    assert_contains(&formal, "| Object grammar |");
    assert_contains(&formal, "Do not draft the final table of contents yet");
    assert_contains(&formal, "## Organizing Form Comparison");
    assert_contains(&formal, "## Report Architecture Decision");
    assert_contains(&formal, "| Architecture rationale |  |");
    assert_contains(&formal, "Rejected alternatives and why");
    assert_not_contains(&formal, "| Generative question |  |");
    assert_not_contains(&formal, "| Dependency or prerequisite graph |");
    assert_not_contains(&formal, "## 4. Concept and Prerequisite Map");
    assert_not_contains(&formal, "## Literature Ladder");
    assert_not_contains(&formal, "## Curriculum Roadmap");
    assert_not_contains(&formal, "```mermaid");
    assert_not_contains(&formal, "TODO");

    let ill = build_report_scaffold(
        "comparative constitutional law",
        "policy researcher",
        "read scholarly debates",
        10,
    );
    assert_contains(
        &ill,
        "| Provisional lens | interpretive or contested candidate |",
    );
    assert_contains(&ill, "| Cases and contexts |");
    assert_contains(&ill, "debate network, case constellation, genealogy");
    assert_not_contains(&ill, "## 4. Debate and Case Network");

    let infrastructure = build_report_scaffold(
        "particle physics",
        "physics-adjacent engineer",
        "map the field",
        12,
    );
    assert_contains(
        &infrastructure,
        "| Provisional lens | empirical or infrastructure-bound candidate |",
    );
    assert_contains(&infrastructure, "| Phenomenon-to-data chain |");
    assert_contains(
        &infrastructure,
        "Where does the object of study become an observation",
    );

    let mixed = build_report_scaffold("causal inference", "research lead", "build a path", 6);
    assert_contains(&mixed, "| Provisional lens | open or mixed candidate |");
    assert_contains(&mixed, "| Practices and warrants |");
    assert_contains(
        &mixed,
        "Which candidate organizing form survives comparison against the sources",
    );
}

#[test]
fn declarative_profiles_support_korean_signals_and_open_fallback() {
    assert_eq!(
        profiles::embedded_profile_schema_version(),
        "sok-discovery-profiles/v1"
    );

    let korean_formal = build_report_scaffold("위상수학", "doctoral learner", "map proofs", 8);
    assert_contains(
        &korean_formal,
        "| Provisional lens | formal or theory-led candidate |",
    );
    assert_contains(&korean_formal, "| Profile proposal | formal |");
    assert_contains(&korean_formal, "| Profile locale | ko |");
    assert_contains(
        &korean_formal,
        "Matched field-name signal \"위상수학\" for locale ko; provisional until source review.",
    );

    let korean_interpretive =
        build_report_scaffold("비교 헌법", "legal scholar", "compare cases", 4);
    assert_contains(
        &korean_interpretive,
        "| Provisional lens | interpretive or contested candidate |",
    );
    assert_contains(&korean_interpretive, "| Profile proposal | interpretive |");
    assert_contains(&korean_interpretive, "| Profile locale | ko |");

    let unknown = build_report_scaffold("zzqv untranslated domain", "reader", "orient", 0);
    assert_contains(&unknown, "| Provisional lens | open or mixed candidate |");
    assert_contains(&unknown, "| Profile proposal | open |");
    assert_contains(&unknown, "| Profile locale | und |");
    assert_contains(&unknown, "| Profile confidence | low |");
    assert_contains(
        &unknown,
        "No profile signal matched or the locale is unknown; keep the profile open and untyped \
            until source review.",
    );

    let korean_unmatched = build_report_scaffold("방법론 일반론", "reader", "orient", 0);
    assert_contains(
        &korean_unmatched,
        "| Provisional lens | open or mixed candidate |",
    );
    assert_contains(&korean_unmatched, "| Profile locale | und |");
}

#[test]
fn explicit_profile_proposal_and_korean_projection_terms_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let sources_path = dir.path().join("sources.csv");
    fs::write(
        &sources_path,
        "title,type,identifier,url,date,access_status,access_route,budget_estimate,license,layer,why_it_matters,use_in_curriculum,notes\n\
Open Korean Source,review,fixture:open-korean-source,https://example.test/korean,2026-01-01,open_access,Fixture URL,$0,CC BY,foundation,Supports Korean profile fixture.,Use for profile fixture,Fixture row\n",
    )
    .unwrap();

    let fixture_path = repo_path("cli/tests/fixtures/profiles/explicit-profile-scaffold.md");
    let exported = report::export_markdown_report(
        &fixture_path,
        &sources_path,
        None::<&PathBuf>,
        report::ExportStage::Scaffold,
    )
    .unwrap();

    let internal = exported.internal_context.as_ref().unwrap();
    assert_eq!(internal.profile_proposals.len(), 1);
    let proposal = &internal.profile_proposals[0];
    assert_eq!(proposal.proposer_type, report::ProfileProposerType::User);
    assert_eq!(proposal.profile_id, "interpretive");
    assert_eq!(proposal.profile_version, "1.0.0");
    assert_eq!(proposal.locale, "ko");
    assert_eq!(proposal.confidence, "0.72");
    assert_contains(
        &proposal.rationale,
        "case comparison and schools of interpretation",
    );

    assert!(exported
        .report
        .methods
        .iter()
        .any(|item| item.label == "판례 비교"));
    assert!(exported
        .report
        .representations
        .iter()
        .any(|item| item.label == "권리 분석 틀"));
    let compatibility_labels = exported
        .report
        .core_ideas
        .iter()
        .chain(exported.report.methods.iter())
        .chain(exported.report.representations.iter())
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert!(!compatibility_labels.contains(&"사법 적극주의 논쟁"));

    let validation = report::validate_report_value(&serde_json::to_value(&exported).unwrap());
    assert_eq!(validation.error_count(), 0);
}
