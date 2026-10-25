use super::*;

#[test]
fn render_html_renders_legacy_v1_report_as_self_contained_one_view() {
    let dir = tempfile::tempdir().unwrap();
    let input_path = dir.path().join("legacy-v1-report.json");
    let output_path = dir.path().join("sok-report.html");
    report::write_json_file(&input_path, &legacy_v1_report_value()).unwrap();
    let exit = run_cli(vec![
        "render-html".to_string(),
        "--input".to_string(),
        input_path.display().to_string(),
        "--output".to_string(),
        output_path.display().to_string(),
    ])
    .unwrap();
    assert_eq!(exit, 0);

    let html = fs::read_to_string(&output_path).unwrap();
    assert_contains(&html, "<!doctype html>");
    assert_contains(&html, "<style>");
    assert_contains(
        &html,
        "<script type=\"application/json\" id=\"sok-visual-data\">",
    );
    assert_contains(&html, "Structure of Knowledge Report");
    assert_contains(&html, "Reading Guide");
    assert_contains(&html, "Knowledge Map");
    assert_contains(&html, "Reading Ladder");
    assert_contains(&html, "Domain Profile");
    assert_contains(&html, "Field Elements");
    assert_not_contains(&html, "id=\"core-ideas\"");
    assert_not_contains(&html, "id=\"methods\"");
    assert_not_contains(&html, "id=\"representations\"");
    assert_contains(&html, "Evidence Standards");
    assert_contains(&html, "Source Catalog");
    assert_contains(&html, "Claim Evidence Guide");
    assert_contains(&html, "Curriculum Path");
    assert_contains(&html, "Frontier Guidance");
    assert_contains(&html, "Relation Audit");
    assert_contains(&html, "id=\"visualizations\"");
    assert_contains(&html, "From fragile probe to trustworthy measurement");
    assert_contains(&html, "\"kind\":\"knowledge_spine\"");
    assert_contains(&html, "\"relation_id\":\"rel-measurand-to-encoding\"");
    assert_contains(
        &html,
        "\"relation_id\":\"rel-estimation-grounds-performance\"",
    );
    assert_contains(&html, "class=\"claim-card\"");
    assert_contains(&html, "<summary>Raw claim identifiers</summary>");
    assert_contains(&html, "class=\"visual-fallback\"");
    assert_contains(&html, "Physical quantity to Encoding interaction");
    assert_contains(&html, "class=\"text-fallback\"");
    assert_contains(&html, "Text alternative for curriculum path");
    assert_contains(&html, "role=\"region\"");
    assert_contains(&html, "<noscript>");
    assert_contains(&html, "@media (prefers-color-scheme: dark)");
    assert_contains(&html, "@media (max-width: 720px)");
    assert_contains(&html, "@media print");
    assert_contains(&html, ".table-scroll");
    assert_contains(
        &html,
        ".narrative-body {\n  max-width: 880px;\n  min-width: 0;\n  overflow-x: auto;",
    );
    assert_contains(
        &html,
        "details.structured-appendix > summary {\n    display: none;\n  }\n  \
            details.structured-appendix:not([open]) > :not(summary) {\n    display: block \
            !important;",
    );
    assert_before(&html, "id=\"reading-guide\"", "id=\"visualizations\"");
    assert_before(&html, "id=\"visualizations\"", "id=\"curriculum-path\"");
    assert_before(&html, "id=\"curriculum-path\"", "id=\"reading-ladder\"");
    assert_before(&html, "id=\"reading-ladder\"", "id=\"claims\"");
    assert_before(&html, "id=\"claims\"", "id=\"frontier-debates\"");
    assert_before(&html, "id=\"frontier-debates\"", "id=\"scope\"");
    assert_not_contains(&html, "<script src");
    assert_not_contains(&html, "<link");
    assert_not_contains(&html.to_ascii_lowercase(), "mermaid");
    assert_not_contains(&html.to_ascii_lowercase(), "cdn");
}

#[test]
fn render_html_keeps_report_readable_without_visual_shell_when_views_are_absent() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    value["report"]
        .as_object_mut()
        .unwrap()
        .remove("visual_views");
    for section in value["report"]["presentation"]["sections"]
        .as_array_mut()
        .unwrap()
    {
        section.as_object_mut().unwrap().remove("visual_view_ids");
    }
    let html = render_report_value_to_html(dir.path(), "zero-view-report.json", &value);

    assert_contains(&html, "Fragility Becomes Signal");
    assert_contains(&html, "One Measurement Grammar, Many Platforms");
    assert_contains(&html, "Source Catalog");
    assert_contains(&html, "Claim Evidence Guide");
    assert_contains(&html, "Curriculum Path");
    assert_contains(&html, "Frontier Guidance");
    assert_not_contains(&html, "id=\"visualizations\"");
    assert_not_contains(&html, "class=\"visual-card\"");
    assert_not_contains(&html, "data-visual-mount=");
    assert_not_contains(&html, "id=\"sok-visual-data\"");
}

#[test]
fn render_html_instantiates_multiple_declared_supported_views() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    let existing_view = value["report"]["visual_views"][0].clone();
    value["report"]["frontier_debates"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "frontier-field-validation-gap",
            "kind": "uncertainty",
            "title": "Field validation gap",
            "summary": "Laboratory sensitivity does not by itself establish calibrated field utility.",
            "why_it_matters": "Deployment evidence determines whether the measurement chain remains useful outside \
                controlled conditions.",
            "required_background_ids": [
                "element-deployment-environment-eb7bb13b46"
            ],
            "claim_ids": [
                "claim-field-usefulness-requires-performance-to-be-re-evaluated-b493f3b20d"
            ],
            "source_ids": [
                "src-bringing-quantum-sensors-to-fruition-ostp-nstc-2022-6b3219ece3"
            ],
            "temporal": {
                "as_of": "2026-07-27",
                "review_after": "2027-01-26",
                "temporal_status": "current",
                "rationale": "Public translation guidance should be reviewed as deployment evidence changes."
            }
        }));
    value["report"]["relations"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "rel-quantum-sensing-supports-encoding",
            "kind": "supports",
            "from": {
                "entity_type": "source",
                "id": "src-quantum-sensing-doi-10-1103-revmodphys-89-035002-568d4f0855"
            },
            "to": {
                "entity_type": "field_element",
                "id": "element-encoding-interaction-f15ae8c981"
            },
            "description": "The experimental review supports the field's encoding-interaction model."
        }));
    value["report"]["relations"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "rel-deployment-claim-maps-to-gap",
            "kind": "maps_to",
            "from": {
                "entity_type": "claim",
                "id": "claim-field-usefulness-requires-performance-to-be-re-evaluated-b493f3b20d"
            },
            "to": {
                "entity_type": "frontier_debate",
                "id": "frontier-field-validation-gap"
            },
            "description": "The deployment claim identifies the unresolved field-validation gap."
        }));
    value["report"]["relations"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "rel-nstc-supports-field-gap",
            "kind": "supports",
            "from": {
                "entity_type": "source",
                "id": "src-bringing-quantum-sensors-to-fruition-ostp-nstc-2022-6b3219ece3"
            },
            "to": {
                "entity_type": "frontier_debate",
                "id": "frontier-field-validation-gap"
            },
            "description": "The public roadmap documents engineering, validation, and adoption barriers."
        }));
    value["report"]["visual_views"] = json!([
        existing_view,
        {
            "id": "view-concept-source-map",
            "kind": "concept_source",
            "title": "Encoding source map",
            "justification": "A concept-source view is justified because the encoding model is tied to a reviewed \
                experimental source.",
            "nodes": [
                {
                    "id": "vnode-encoding-interaction",
                    "label": "Encoding interaction",
                    "entity_type": "field_element",
                    "ref_id": "element-encoding-interaction-f15ae8c981"
                },
                {
                    "id": "vnode-quantum-sensing-source",
                    "label": "Quantum Sensing",
                    "entity_type": "source",
                    "ref_id": "src-quantum-sensing-doi-10-1103-revmodphys-89-035002-568d4f0855"
                }
            ],
            "edges": [
                {
                    "from": "vnode-quantum-sensing-source",
                    "to": "vnode-encoding-interaction",
                    "kind": "supports",
                    "relation_id": "rel-quantum-sensing-supports-encoding"
                }
            ]
        },
        {
            "id": "view-frontier-debate-map",
            "kind": "frontier_debate",
            "title": "Field validation gap",
            "justification": "A frontier/debate view is justified because field utility depends on a dated public \
                source and deployment claim.",
            "nodes": [
                {
                    "id": "vnode-frontier-field-gap",
                    "label": "Field validation gap",
                    "entity_type": "frontier_debate",
                    "ref_id": "frontier-field-validation-gap"
                },
                {
                    "id": "vnode-claim-field-utility",
                    "label": "Field utility claim",
                    "entity_type": "claim",
                    "ref_id": "claim-field-usefulness-requires-performance-to-be-re-evaluated-b493f3b20d"
                },
                {
                    "id": "vnode-source-nstc",
                    "label": "Bringing Quantum Sensors to Fruition",
                    "entity_type": "source",
                    "ref_id": "src-bringing-quantum-sensors-to-fruition-ostp-nstc-2022-6b3219ece3"
                }
            ],
            "edges": [
                {
                    "from": "vnode-source-nstc",
                    "to": "vnode-frontier-field-gap",
                    "kind": "supports",
                    "relation_id": "rel-nstc-supports-field-gap"
                },
                {
                    "from": "vnode-claim-field-utility",
                    "to": "vnode-frontier-field-gap",
                    "kind": "maps_to",
                    "relation_id": "rel-deployment-claim-maps-to-gap"
                }
            ]
        }
    ]);

    let html = render_report_value_to_html(dir.path(), "multiple-views-report.json", &value);
    assert_eq!(html.matches("class=\"visual-card\"").count(), 3);
    assert_contains(&html, "\"kind\":\"knowledge_spine\"");
    assert_contains(&html, "\"kind\":\"concept_source\"");
    assert_contains(&html, "\"kind\":\"frontier_debate\"");
    assert_contains(&html, "Encoding source map");
    assert_contains(&html, "Field validation gap");
    assert_contains(&html, "Text alternative for Encoding source map");
    assert_contains(&html, "Nodes");
    assert_contains(&html, "Edges");
}

#[test]
fn render_html_orders_directed_spines_from_relations_not_node_declaration_order() {
    fn visual_layouts(html: &str) -> std::collections::BTreeMap<String, (String, u64, u64, u64)> {
        let payload = html
            .split_once("<script type=\"application/json\" id=\"sok-visual-data\">")
            .unwrap()
            .1
            .split_once("</script>")
            .unwrap()
            .0;
        let views: Value = serde_json::from_str(payload).unwrap();
        views[0]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| {
                (
                    node["label"].as_str().unwrap().to_string(),
                    (
                        node["layout"]["lane"].as_str().unwrap().to_string(),
                        node["layout"]["order"].as_u64().unwrap(),
                        node["layout"]["x"].as_u64().unwrap(),
                        node["layout"]["y"].as_u64().unwrap(),
                    ),
                )
            })
            .collect()
    }

    let dir = tempfile::tempdir().unwrap();
    let ordered = current_human_report_value();
    let ordered_html =
        render_report_value_to_html(dir.path(), "directed-layout-ordered.json", &ordered);
    let ordered_layouts = visual_layouts(&ordered_html);
    assert_contains(
        &ordered_html,
        "\"label_lines\":[\"Reported\",\"performance vector\"]",
    );
    assert_contains(
        &ordered_html,
        "\"label_lines\":[\"Estimation and\",\"uncertainty\"]",
    );
    assert_contains(
        &ordered_html,
        "\"label_lines\":[\"Calibration and\",\"traceability\"]",
    );
    assert_contains(&ordered_html, "var tspan = el(\"tspan\"");

    let mut reversed = ordered.clone();
    reversed["report"]["visual_views"][0]["nodes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let reversed_html =
        render_report_value_to_html(dir.path(), "directed-layout-reversed.json", &reversed);
    let reversed_layouts = visual_layouts(&reversed_html);
    assert_eq!(ordered_layouts, reversed_layouts);

    let chain = [
        "Physical quantity",
        "Encoding interaction",
        "Readout and transduction",
        "Estimation and uncertainty",
        "Reported performance vector",
    ];
    for (expected_order, label) in chain.iter().enumerate() {
        let (lane, order, _, _) = &ordered_layouts[*label];
        assert_eq!(lane, "primary");
        assert_eq!(*order, expected_order as u64);
    }
    for pair in chain.windows(2) {
        assert!(
            ordered_layouts[pair[0]].2 < ordered_layouts[pair[1]].2,
            "{} should render before {}",
            pair[0],
            pair[1]
        );
    }

    let primary_y = ordered_layouts["Reported performance vector"].3;
    for label in ["Calibration and traceability", "Deployment environment"] {
        let (lane, _, _, y) = &ordered_layouts[label];
        assert_eq!(lane, "secondary");
        assert!(*y > primary_y);
    }
}

#[test]
fn render_html_renders_literature_ladder_as_reader_path() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    value["report"]["literature_ladder"] = json!([
        {
            "id": "ladder-measurement-chain-quantum-sensing",
            "layer": "Experimental grammar",
            "start_here": "Start with the full measurement chain before comparing probe platforms.",
            "read_for": "Read for encoding, control, readout, estimation, and dominant noise.",
            "do_not_infer": "Do not infer that laboratory sensitivity alone establishes field utility.",
            "source_ids": ["src-quantum-sensing-doi-10-1103-revmodphys-89-035002-568d4f0855"],
            "notes": "Use this before comparing nonclassical resources."
        }
    ]);

    let html = render_report_value_to_html(dir.path(), "ladder-report.json", &value);
    assert_contains(&html, "Start point");
    assert_contains(&html, "Read for");
    assert_contains(&html, "Do not infer");
    assert_contains(
        &html,
        "Start with the full measurement chain before comparing probe platforms.",
    );
    assert_contains(&html, "Quantum Sensing");
    assert_contains(&html, "<summary>Raw ladder identifiers</summary>");
    assert_before(
        &html,
        "Start with the full measurement chain",
        "<summary>Raw ladder identifiers</summary>",
    );
}

#[test]
fn render_html_claim_cards_group_support_qualifiers_and_contradictions() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    value["report"]["claims"][0]["evidence_links"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "source_id": "src-jcgm-100-2008-guide-to-the-expression-of-8b71053703",
            "verification_status": "reviewed",
            "support_kind": "qualifies",
            "locator": "Clauses 4-8",
            "support_note": "Qualifies the chain interpretation with explicit uncertainty and reporting \
                requirements.",
            "reviewed_at": "2026-07-27"
        }));
    value["report"]["claims"][0]["evidence_links"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "source_id": "src-doe-quantum-information-science-applications-roadmap-doe-qis-5749cee471",
            "verification_status": "reviewed",
            "support_kind": "contradicts",
            "locator": "Quantum sensing applications and cross-cutting needs",
            "support_note": "Challenges any reading that treats one laboratory measurement chain as sufficient \
                for every deployment.",
            "reviewed_at": "2026-07-27"
        }));

    let html = render_report_value_to_html(dir.path(), "claim-card-report.json", &value);
    assert_contains(&html, "class=\"claim-card\"");
    assert_contains(&html, "Supporting Evidence");
    assert_contains(&html, "Qualifying Evidence");
    assert_contains(&html, "Contradictory Evidence");
    assert_contains(
        &html,
        "Qualifies the chain interpretation with explicit uncertainty and reporting requirements.",
    );
    assert_contains(
        &html,
        "Challenges any reading that treats one laboratory measurement chain as sufficient for every deployment.",
    );
    assert_contains(
        &html,
        "JCGM 100:2008 Guide to the Expression of Uncertainty in Measurement",
    );
    assert_contains(
        &html,
        "DOE Quantum Information Science Applications Roadmap",
    );
    assert_contains(&html, "<summary>Raw claim identifiers</summary>");
}

#[test]
fn render_html_rejects_visuals_when_edges_are_not_relation_backed() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    for edge in value["report"]["visual_views"][0]["edges"]
        .as_array_mut()
        .unwrap()
    {
        edge.as_object_mut().unwrap().remove("relation_id");
    }

    let validation = report::validate_report_value(&value);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );
    assert!(validation
        .diagnostics
        .checks
        .iter()
        .any(|check| check.message.contains("requires a relation_id")));
    assert_validation_message(
        &validation,
        "requires at least one valid relation-backed edge",
    );

    let input_path = dir.path().join("relationless-view-report.json");
    let output_path = dir.path().join("relationless-view-report.html");
    report::write_json_file(&input_path, &value).unwrap();
    assert!(report::render_html_report_file(&input_path, &output_path).is_err());
    assert!(!output_path.exists());
}

#[test]
fn render_html_rejects_visuals_when_relation_does_not_match_node_refs() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    value["report"]["visual_views"][0]["edges"][0]["relation_id"] =
        json!("rel-encoding-before-readout");

    let validation = report::validate_report_value(&value);
    assert_validation_check(
        &validation,
        report::CHECK_VALIDATE_VISUAL_REFERENCE,
        report::DiagnosticSeverity::Error,
    );
    assert!(validation.diagnostics.checks.iter().any(|check| check
        .message
        .contains("does not match the endpoint entity references")));

    let input_path = dir.path().join("mismatched-view-report.json");
    let output_path = dir.path().join("mismatched-view-report.html");
    report::write_json_file(&input_path, &value).unwrap();
    assert!(report::render_html_report_file(&input_path, &output_path).is_err());
    assert!(!output_path.exists());
}

#[test]
fn render_html_escapes_report_content_and_omits_internal_payloads() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    value["report"]["field"] = json!("Topology <img src=x onerror=alert(1)> \"quoted\"");
    value["report"]["scope"]["summary"] =
        json!("Summary </script><script>alert(1)</script> & <b>bold</b>");
    value["report"]["visual_views"][0]["nodes"][0]["label"] =
        json!("Node </script><script>alert(2)</script>");
    value["report"]["presentation"] = json!({
        "thesis": "Narrative Markdown must remain safe to render.",
        "organizing_form": "Security test",
        "rationale": "Exercise the safe Markdown renderer.",
        "alternatives_considered": "A raw HTML rendering path was rejected because it would bypass sanitization.",
        "sections": [
            {
                "id": "section-security-test",
                "title": "Narrative safety",
                "body_markdown": "Safe prose. <script>alert(3)</script>\n\n[unsafe](javascript:alert(4)) \
                    [safe](https://example.org/reference)\n\n![remote](https://assets.example.org/remote.png) ![relative](assets/local.png) ![svg](data:image/svg+xml;base64,PHN2Zz4=) ![embedded](data:image/png;base64,iVBORw0KGgo=)"
            }
        ]
    });
    value["internal_context"] = json!({
        "raw_learner_profile": "SENTINEL_INTERNAL_RENDER",
        "original_goal": "SENTINEL_INTERNAL_GOAL"
    });
    value["diagnostics"] = json!({
        "summary": "SENTINEL_DIAGNOSTIC_RENDER",
        "checks": [
            {
                "check_id": "sentinel.diagnostic",
                "severity": "warning",
                "message": "SENTINEL_DIAGNOSTIC_MESSAGE"
            }
        ]
    });

    let html = render_report_value_to_html(dir.path(), "escaped-report.json", &value);
    assert_contains(&html, "&lt;img src=x onerror=alert(1)&gt;");
    assert_contains(
        &html,
        "Summary &lt;/script&gt;&lt;script&gt;alert(1)&lt;/script&gt; &amp; &lt;b&gt;bold&lt;/b&gt;",
    );
    assert_contains(&html, "\\u003C/script\\u003E\\u003Cscript\\u003Ealert(2)");
    assert_not_contains(&html, "<img src=x");
    assert_not_contains(&html, "</script><script>alert(1)</script>");
    assert_not_contains(&html, "</script><script>alert(2)</script>");
    assert_contains(&html, "&lt;script&gt;alert(3)&lt;/script&gt;");
    assert_not_contains(&html, "<script>alert(3)</script>");
    assert_contains(&html, "href=\"#blocked-unsafe-link\"");
    assert_not_contains(&html, "href=\"javascript:");
    assert_contains(&html, "href=\"https://example.org/reference\"");
    assert_not_contains(&html, "https://assets.example.org/remote.png");
    assert_not_contains(&html, "assets/local.png");
    assert_not_contains(&html, "data:image/svg+xml");
    assert_contains(&html, "src=\"#blocked-external-image\"");
    assert_contains(&html, "src=\"data:image/png;base64,iVBORw0KGgo=\"");
    assert_not_contains(&html, "SENTINEL_INTERNAL_RENDER");
    assert_not_contains(&html, "SENTINEL_INTERNAL_GOAL");
    assert_not_contains(&html, "SENTINEL_DIAGNOSTIC_RENDER");
    assert_not_contains(&html, "SENTINEL_DIAGNOSTIC_MESSAGE");
    assert_not_contains(&html, "sentinel.diagnostic");
}

#[test]
fn render_html_namespaces_markdown_footnotes_by_presentation_section() {
    let dir = tempfile::tempdir().unwrap();
    let mut value = current_human_report_value();
    value["report"]["presentation"] = json!({
        "thesis": "Repeated footnote labels remain local to their narrative sections.",
        "organizing_form": "Two-section test",
        "rationale": "Exercise generated DOM identifiers.",
        "alternatives_considered": "A single combined narrative was rejected because it would not exercise section-local \
            identifiers.",
        "sections": [
            {
                "id": "section-first",
                "title": "First section",
                "body_markdown": "First claim.[^note]\n\n[^note]: First section note."
            },
            {
                "id": "section-second",
                "title": "Second section",
                "body_markdown": "Second claim.[^note]\n\n[^note]: Second section note."
            }
        ]
    });

    let html = render_report_value_to_html(dir.path(), "footnote-report.json", &value);
    assert_contains(&html, "href=\"#section-first-note\"");
    assert_contains(&html, "id=\"section-first-note\"");
    assert_contains(&html, "href=\"#section-second-note\"");
    assert_contains(&html, "id=\"section-second-note\"");
}

#[test]
fn render_html_rejects_non_human_or_invalid_reports() {
    let dir = tempfile::tempdir().unwrap();
    let scaffold_path = dir.path().join("scaffold-report.json");
    let output_path = dir.path().join("scaffold.html");
    let mut scaffold = current_human_report_value();
    scaffold["metadata"]["report_type"] = json!("scaffold");
    report::write_json_file(&scaffold_path, &scaffold).unwrap();
    assert_contains(
        &report::render_html_report_file(&scaffold_path, &output_path)
            .unwrap_err()
            .to_string(),
        "human_report",
    );

    let invalid_path = dir.path().join("invalid-report.json");
    let mut invalid = current_human_report_value();
    invalid["report"]["claims"][2]["evidence_links"] = json!([]);
    report::write_json_file(&invalid_path, &invalid).unwrap();
    assert_contains(
        &report::render_html_report_file(&invalid_path, &output_path)
            .unwrap_err()
            .to_string(),
        "validation has",
    );
}
