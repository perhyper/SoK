//! Report rendering entry points and shared embedded assets.
mod formatting;
mod layout;
mod presentation;
mod sections;

pub(crate) use formatting::*;
use layout::*;
use presentation::*;
use sections::*;

use super::import::*;
use super::model::*;
use super::provenance::first_non_empty;
use super::relations::{relation_kind_label, visual_relation_matches_nodes};
use super::validation::*;
use super::*;

pub fn render_html_report_file<I, O>(input: I, output: O) -> Result<()>
where
    I: AsRef<Path>,
    O: AsRef<Path>,
{
    let input = input.as_ref();
    let output = output.as_ref();
    let validation = validate_report_file(input)?;
    if validation.has_errors() {
        bail!(
            "cannot render HTML from {}: report validation has {} error(s); run sok validate-report --input {}",
            input.display(),
            validation.error_count(),
            input.display()
        );
    }

    let document: ReportDocument = read_json_file(input)?;
    let html = render_html_report(&document)?;
    ensure_parent_dir(output)?;
    fs::write(output, html).with_context(|| format!("write HTML {}", output.display()))?;
    Ok(())
}

pub fn render_html_report(document: &ReportDocument) -> Result<String> {
    let document = crate::core::projection::project_report_document(document)?;
    if document.metadata.report_type != ReportType::HumanReport {
        bail!("sok render-html requires metadata.report_type to be human_report");
    }

    let report = &document.report;
    let views = renderable_visual_views(report);
    let visual_json = if views.is_empty() {
        None
    } else {
        Some(safe_script_json(&views)?)
    };
    let source_labels = source_label_lookup(report);
    let entity_labels = entity_label_lookup(report);
    let title = format!("Structure of Knowledge: {}", report.field);
    let mut html = String::new();

    html.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("  <meta charset=\"utf-8\">\n");
    html.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str("  <title>");
    push_escaped(&mut html, &title);
    html.push_str("</title>\n");
    html.push_str("  <style>\n");
    html.push_str(REPORT_CSS);
    html.push_str("\n  </style>\n</head>\n<body>\n");
    html.push_str("<a class=\"skip-link\" href=\"#main-report\">Skip to report content</a>\n");
    html.push_str("<div class=\"report-shell\">\n<header class=\"report-header\">\n");
    html.push_str("<p class=\"eyebrow\">Structure of Knowledge Report</p>\n<h1>");
    push_escaped(&mut html, &report.field);
    html.push_str("</h1>\n");
    let presentation = report
        .presentation
        .as_ref()
        .filter(|presentation| !presentation.sections.is_empty());
    let header_summary = presentation
        .map(|presentation| presentation.thesis.as_str())
        .filter(|thesis| !thesis.trim().is_empty())
        .unwrap_or(report.scope.summary.as_str());
    if !header_summary.trim().is_empty() {
        html.push_str("<p class=\"summary\">");
        push_escaped(&mut html, header_summary);
        html.push_str("</p>\n");
    }
    if let Some(presentation) = presentation {
        push_presentation_nav(&mut html, presentation);
    } else {
        push_nav(&mut html, report, !views.is_empty());
    }
    html.push_str("</header>\n<main id=\"main-report\">\n");

    if let Some(presentation) = presentation {
        push_presentation_report(
            &mut html,
            presentation,
            &views,
            report,
            &entity_labels,
            &source_labels,
        );
    } else {
        push_reading_guide_section(&mut html, &document, !views.is_empty());
        if !views.is_empty() {
            push_visual_section(&mut html, &views);
        }
        push_curriculum_section(
            &mut html,
            &report.curriculum_path,
            &entity_labels,
            &source_labels,
        );
        push_reading_ladder_section(&mut html, report, &source_labels);
        push_claims_section(&mut html, &report.claims, &source_labels);
        push_frontier_section(&mut html, report, &entity_labels, &source_labels);

        push_scope_section(&mut html, &report.scope);
        push_domain_profile_section(&mut html, &report.domain_profile);
        push_field_elements_or_legacy(&mut html, report, &source_labels);
        push_evidence_standards_section(&mut html, &report.evidence_standards);
        push_sources_and_evidence_section(&mut html, report);
        push_relations_section(&mut html, &report.relations, &entity_labels, &source_labels);
    }

    html.push_str("</main>\n</div>\n");
    if let Some(visual_json) = visual_json {
        html.push_str("<script type=\"application/json\" id=\"sok-visual-data\">");
        html.push_str(&visual_json);
        html.push_str("</script>\n<script>\n");
        html.push_str(REPORT_JS);
        html.push_str("\n</script>\n");
    }
    html.push_str("</body>\n</html>\n");
    Ok(html)
}

pub(crate) const REPORT_CSS: &str = include_str!("assets/report.css");

pub(crate) const REPORT_JS: &str = include_str!("assets/report.js");
