use super::*;
use pulldown_cmark::html as cmark_html;

pub(crate) fn push_presentation_nav(html: &mut String, presentation: &ReportPresentation) {
    html.push_str("<nav class=\"section-nav\" aria-label=\"Report sections\">\n");
    for section in &presentation.sections {
        html.push_str("<a href=\"#");
        push_escaped_attr(html, &section.id);
        html.push_str("\">");
        push_escaped(html, &section.title);
        html.push_str("</a>\n");
    }
    html.push_str("<a href=\"#evidence-appendix\">Evidence trail</a>\n</nav>\n");
}

pub(crate) fn push_presentation_report(
    html: &mut String,
    presentation: &ReportPresentation,
    views: &[RenderVisualView],
    report: &PublicReport,
    entity_labels: &BTreeMap<String, String>,
    source_labels: &BTreeMap<String, String>,
) {
    if !presentation.organizing_form.trim().is_empty() || !presentation.rationale.trim().is_empty()
    {
        html.push_str("<aside class=\"architecture-note\" aria-label=\"Report architecture\">\n");
        if !presentation.organizing_form.trim().is_empty() {
            html.push_str("<p><span>Organizing form</span>");
            push_escaped(html, &presentation.organizing_form);
            html.push_str("</p>\n");
        }
        if !presentation.rationale.trim().is_empty() {
            html.push_str("<p>");
            push_escaped(html, &presentation.rationale);
            html.push_str("</p>\n");
        }
        html.push_str("</aside>\n");
    }

    let views_by_id = views
        .iter()
        .map(|view| (view.id.as_str(), view))
        .collect::<BTreeMap<_, _>>();
    let assigned_view_ids = presentation
        .sections
        .iter()
        .flat_map(|section| section.visual_view_ids.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();

    for section in &presentation.sections {
        push_section_open(html, &section.id, &section.title);
        if !section.purpose.trim().is_empty() {
            html.push_str("<p class=\"section-purpose\">");
            push_escaped(html, &section.purpose);
            html.push_str("</p>\n");
        }
        push_safe_markdown(html, &section.body_markdown, &section.id);
        for view_id in &section.visual_view_ids {
            if let Some(view) = views_by_id.get(view_id.as_str()) {
                push_visual_card(html, view);
            }
        }
        html.push_str("</section>\n");
    }

    html.push_str("<details class=\"structured-appendix\" id=\"evidence-appendix\">\n");
    html.push_str("<summary>Evidence and structured data</summary>\n");
    html.push_str("<p class=\"muted\">Machine-verifiable support for the narrative above. It is kept separate \
        so the evidence contract does not dictate the report's reading order.</p>\n");
    let unassigned_views = views
        .iter()
        .filter(|view| !assigned_view_ids.contains(view.id.as_str()))
        .collect::<Vec<_>>();
    if !unassigned_views.is_empty() {
        push_section_open(html, "unplaced-visualizations", "Additional Visual Views");
        for view in unassigned_views {
            push_visual_card(html, view);
        }
        html.push_str("</section>\n");
    }
    push_scope_section(html, &report.scope);
    push_domain_profile_section(html, &report.domain_profile);
    push_field_elements_or_legacy(html, report, source_labels);
    push_curriculum_section(html, &report.curriculum_path, entity_labels, source_labels);
    push_reading_ladder_section(html, report, source_labels);
    push_frontier_section(html, report, entity_labels, source_labels);
    push_evidence_standards_section(html, &report.evidence_standards);
    push_claims_section(html, &report.claims, source_labels);
    push_sources_and_evidence_section(html, report);
    push_relations_section(html, &report.relations, entity_labels, source_labels);
    html.push_str("</details>\n");
}

pub(crate) fn push_safe_markdown(
    html_output: &mut String,
    markdown: &str,
    footnote_namespace: &str,
) {
    if markdown.trim().is_empty() {
        return;
    }
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let events = Parser::new_ext(markdown, options).map(|event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(CowStr::from(raw.into_string())),
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: safe_markdown_destination(dest_url),
            title,
            id,
        }),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Image {
            link_type,
            dest_url: safe_markdown_image_destination(dest_url),
            title,
            id,
        }),
        Event::Start(Tag::FootnoteDefinition(label)) => Event::Start(Tag::FootnoteDefinition(
            namespaced_footnote_label(footnote_namespace, label),
        )),
        Event::FootnoteReference(label) => {
            Event::FootnoteReference(namespaced_footnote_label(footnote_namespace, label))
        }
        other => other,
    });
    html_output.push_str("<div class=\"narrative-body\">\n");
    cmark_html::push_html(html_output, events);
    html_output.push_str("</div>\n");
}

pub(crate) fn safe_markdown_destination(destination: CowStr<'_>) -> CowStr<'_> {
    let normalized = destination
        .trim()
        .chars()
        .filter(|character| !character.is_ascii_control() && !character.is_ascii_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    let first_path_delimiter = normalized.find(['/', '?', '#']).unwrap_or(normalized.len());
    let scheme = normalized
        .find(':')
        .filter(|colon| *colon < first_path_delimiter)
        .map(|colon| &normalized[..colon]);
    if scheme.is_none() || matches!(scheme, Some("http" | "https" | "mailto")) {
        destination
    } else {
        CowStr::Borrowed("#blocked-unsafe-link")
    }
}

pub(crate) fn safe_markdown_image_destination(destination: CowStr<'_>) -> CowStr<'_> {
    let trimmed = destination.trim();
    let Some((metadata, payload)) = trimmed.split_once(',') else {
        return CowStr::Borrowed("#blocked-external-image");
    };
    let metadata = metadata.to_ascii_lowercase();
    let safe_raster_type = matches!(
        metadata.as_str(),
        "data:image/png;base64"
            | "data:image/jpeg;base64"
            | "data:image/jpg;base64"
            | "data:image/gif;base64"
            | "data:image/webp;base64"
    );
    let safe_base64_payload = !payload.is_empty()
        && payload
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='));
    if safe_raster_type && safe_base64_payload {
        destination
    } else {
        CowStr::Borrowed("#blocked-external-image")
    }
}

pub(crate) fn namespaced_footnote_label<'a>(namespace: &str, label: CowStr<'a>) -> CowStr<'a> {
    CowStr::from(format!("{namespace}-{label}"))
}
