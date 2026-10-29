use super::*;

pub(crate) fn push_section_open(html: &mut String, id: &str, title: &str) {
    html.push_str("<section class=\"report-section\" id=\"");
    push_escaped_attr(html, id);
    html.push_str("\">\n<h2>");
    push_escaped(html, title);
    html.push_str("</h2>\n");
}

pub(crate) fn push_paragraph(html: &mut String, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    html.push_str("<p class=\"prose\">");
    push_escaped(html, text);
    html.push_str("</p>\n");
}

pub(crate) fn push_string_list(html: &mut String, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    html.push_str("<div>\n<h3>");
    push_escaped(html, title);
    html.push_str("</h3>\n<ul>\n");
    for item in items {
        html.push_str("<li>");
        push_escaped(html, item);
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n</div>\n");
}

pub(crate) fn push_nested_string_list(html: &mut String, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    html.push_str("<div class=\"reference-block\">\n<h4>");
    push_escaped(html, title);
    html.push_str("</h4>\n<ul class=\"reference-list\">\n");
    for item in items {
        html.push_str("<li>");
        push_escaped(html, item);
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n</div>\n");
}

pub(crate) fn push_empty_note(html: &mut String) {
    html.push_str("<p class=\"muted\">No entries.</p>\n");
}

pub(crate) fn push_table(html: &mut String, headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        push_empty_note(html);
        return;
    }
    html.push_str(
        "<div class=\"table-scroll\" role=\"region\" aria-label=\"Scrollable table\" \
        tabindex=\"0\">\n<table>\n<thead>\n<tr>",
    );
    for header in headers {
        html.push_str("<th scope=\"col\">");
        push_escaped(html, header);
        html.push_str("</th>");
    }
    html.push_str("</tr>\n</thead>\n<tbody>\n");
    for row in rows {
        html.push_str("<tr>");
        for index in 0..headers.len() {
            html.push_str("<td>");
            if let Some(cell) = row.get(index) {
                push_escaped(html, cell);
            }
            html.push_str("</td>");
        }
        html.push_str("</tr>\n");
    }
    html.push_str("</tbody>\n</table>\n</div>\n");
}

pub(crate) fn push_badge(html: &mut String, label: &str) {
    if label.trim().is_empty() {
        return;
    }
    html.push_str("<span class=\"badge\">");
    push_escaped(html, label);
    html.push_str("</span>");
}

pub(crate) fn push_dl_item(html: &mut String, term: &str, value: &str) {
    if value.trim().is_empty() {
        return;
    }
    html.push_str("<dt>");
    push_escaped(html, term);
    html.push_str("</dt><dd>");
    push_escaped(html, value);
    html.push_str("</dd>\n");
}

pub(crate) fn push_source_reference_list(
    html: &mut String,
    title: &str,
    source_ids: &[String],
    source_labels: &BTreeMap<String, String>,
) {
    if source_ids.is_empty() {
        return;
    }
    html.push_str("<div class=\"reference-block\">\n<h4>");
    push_escaped(html, title);
    html.push_str("</h4>\n<ul class=\"reference-list\">\n");
    for source_id in source_ids {
        html.push_str("<li>");
        push_escaped(html, &label_for_id(source_id, source_labels));
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n</div>\n");
}

pub(crate) fn push_entity_reference_list(
    html: &mut String,
    title: &str,
    ids: &[String],
    entity_labels: &BTreeMap<String, String>,
) {
    if ids.is_empty() {
        return;
    }
    html.push_str("<div class=\"reference-block\">\n<h4>");
    push_escaped(html, title);
    html.push_str("</h4>\n<ul class=\"reference-list\">\n");
    for id in ids {
        html.push_str("<li>");
        push_escaped(html, &label_for_id(id, entity_labels));
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n</div>\n");
}

pub(crate) fn push_evidence_group<F>(
    html: &mut String,
    title: &str,
    links: &[EvidenceLink],
    source_labels: &BTreeMap<String, String>,
    include: F,
) where
    F: Fn(SupportKind) -> bool,
{
    html.push_str("<section class=\"evidence-group\" aria-label=\"");
    push_escaped_attr(html, title);
    html.push_str("\">\n<h4>");
    push_escaped(html, title);
    html.push_str("</h4>\n");
    let matching = links
        .iter()
        .filter(|link| include(link.support_kind))
        .collect::<Vec<_>>();
    if matching.is_empty() {
        html.push_str("<p class=\"muted\">None recorded.</p>\n</section>\n");
        return;
    }
    html.push_str("<ul class=\"evidence-list\">\n");
    for link in matching {
        let source_label = label_for_id(&link.source_id, source_labels);
        html.push_str("<li>\n<strong>");
        push_escaped(html, &source_label);
        html.push_str("</strong>\n<span class=\"evidence-meta\">");
        push_escaped(
            html,
            &format!(
                "{}; {}",
                verification_status_label(link.verification_status),
                support_kind_label(link.support_kind)
            ),
        );
        html.push_str("</span>\n");
        if !link.locator.trim().is_empty() {
            html.push_str("<p><strong>Locator:</strong> ");
            push_escaped(html, &link.locator);
            html.push_str("</p>\n");
        }
        if !link.support_note.trim().is_empty() {
            html.push_str("<p>");
            push_escaped(html, &link.support_note);
            html.push_str("</p>\n");
        }
        if !link.reviewed_at.trim().is_empty() {
            html.push_str("<p class=\"meta\">Reviewed ");
            push_escaped(html, &link.reviewed_at);
            html.push_str("</p>\n");
        }
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n</section>\n");
}

pub(crate) fn push_claim_raw_details(html: &mut String, claim: &Claim) {
    let evidence_ids = claim
        .evidence_links
        .iter()
        .map(|link| first_non_empty([link.evidence_id.as_str(), link.source_id.as_str()]))
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    let source_ids = claim
        .evidence_links
        .iter()
        .map(|link| link.source_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(", ");
    push_raw_details(
        html,
        "Raw claim identifiers",
        &[
            ("Claim ID", claim.id.clone()),
            ("Evidence IDs", evidence_ids),
            ("Source IDs", source_ids),
            (
                "Evidence links",
                format_claim_evidence_links(&claim.evidence_links),
            ),
            ("Temporal fields", format_temporal(&claim.temporal)),
        ],
    );
}

pub(crate) fn push_raw_details(html: &mut String, summary: &str, rows: &[(&str, String)]) {
    if rows.iter().all(|(_, value)| value.trim().is_empty()) {
        return;
    }
    html.push_str("<details class=\"raw-identifiers\">\n<summary>");
    push_escaped(html, summary);
    html.push_str("</summary>\n<dl class=\"compact-dl\">\n");
    for (term, value) in rows {
        push_dl_item(html, term, value);
    }
    html.push_str("</dl>\n</details>\n");
}

pub(crate) fn source_label_lookup(report: &PublicReport) -> BTreeMap<String, String> {
    report
        .sources
        .iter()
        .map(|source| (source.id.clone(), source_display_label(source)))
        .collect()
}

pub(crate) fn entity_label_lookup(report: &PublicReport) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    for source in &report.sources {
        labels.insert(source.id.clone(), source_display_label(source));
    }
    for item in &report.field_elements {
        labels.insert(
            item.id.clone(),
            first_non_empty([item.label.as_str(), item.id.as_str()]),
        );
    }
    for item in &report.core_ideas {
        labels.insert(
            item.id.clone(),
            first_non_empty([item.label.as_str(), item.id.as_str()]),
        );
    }
    for item in &report.methods {
        labels.insert(
            item.id.clone(),
            first_non_empty([item.label.as_str(), item.id.as_str()]),
        );
    }
    for item in &report.representations {
        labels.insert(
            item.id.clone(),
            first_non_empty([item.label.as_str(), item.id.as_str()]),
        );
    }
    for claim in &report.claims {
        labels.insert(
            claim.id.clone(),
            first_non_empty([claim.statement.as_str(), claim.id.as_str()]),
        );
    }
    for step in &report.curriculum_path {
        labels.insert(
            step.id.clone(),
            first_non_empty([step.title.as_str(), step.id.as_str()]),
        );
    }
    for item in &report.frontier_debates {
        labels.insert(
            item.id.clone(),
            first_non_empty([item.title.as_str(), item.id.as_str()]),
        );
    }
    labels
}

pub(crate) fn source_display_label(source: &ReportSource) -> String {
    first_non_empty([
        source.title.as_str(),
        source.citation.as_str(),
        source.identifier.as_str(),
        source.id.as_str(),
    ])
}

pub(crate) fn labels_for_ids(ids: &[String], labels: &BTreeMap<String, String>) -> String {
    ids.iter()
        .map(|id| label_for_id(id, labels))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn label_for_id(id: &str, labels: &BTreeMap<String, String>) -> String {
    labels
        .get(id)
        .cloned()
        .unwrap_or_else(|| id.trim().to_string())
}

pub(crate) fn format_source_roles(roles: &[SourceRole]) -> String {
    roles
        .iter()
        .map(|role| source_role_label(*role))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn format_source_access(access: &SourceAccessMetadata) -> String {
    let mut parts = vec![
        access_status_label(access.status).to_string(),
        access.route.clone(),
    ];
    if !access.budget_estimate.trim().is_empty() {
        parts.push(format!("budget: {}", access.budget_estimate));
    }
    if !access.license.trim().is_empty() {
        parts.push(format!("license: {}", access.license));
    }
    if access.metadata_only == Some(true) {
        parts.push("metadata only".to_string());
    }
    if !access.notes.trim().is_empty() {
        parts.push(access.notes.clone());
    }
    parts.join("\n")
}

pub(crate) fn format_source_role_waiver(waiver: &SourceRoleWaiver) -> String {
    let mut parts = vec![waiver.rationale.clone(), format!("as_of: {}", waiver.as_of)];
    if !waiver.review_after.trim().is_empty() {
        parts.push(format!("review_after: {}", waiver.review_after));
    }
    parts.join("\n")
}

pub(crate) fn format_temporal(marker: &TemporalMarker) -> String {
    let mut parts = Vec::new();
    if !marker.as_of.trim().is_empty() {
        parts.push(format!("as_of: {}", marker.as_of));
    }
    if !marker.review_after.trim().is_empty() {
        parts.push(format!("review_after: {}", marker.review_after));
    }
    parts.push(format!(
        "status: {}",
        temporal_status_label(marker.temporal_status)
    ));
    if !marker.rationale.trim().is_empty() {
        parts.push(marker.rationale.clone());
    }
    parts.join("\n")
}

pub(crate) fn format_temporal_inline(marker: &TemporalMarker) -> String {
    let mut parts = Vec::new();
    parts.push(format!(
        "status: {}",
        temporal_status_label(marker.temporal_status)
    ));
    if !marker.as_of.trim().is_empty() {
        parts.push(format!("as of {}", marker.as_of));
    }
    if !marker.review_after.trim().is_empty() {
        parts.push(format!("review after {}", marker.review_after));
    }
    if !marker.rationale.trim().is_empty() {
        parts.push(marker.rationale.clone());
    }
    parts.join("; ")
}

pub(crate) fn format_claim_evidence_links(links: &[EvidenceLink]) -> String {
    links
        .iter()
        .map(|link| {
            let support = first_non_empty([link.locator.as_str(), link.support_note.as_str()]);
            format!(
                "{} [{}; {}; {}]",
                link.source_id,
                verification_status_label(link.verification_status),
                support_kind_label(link.support_kind),
                support
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn safe_script_json<T: Serialize>(value: &T) -> Result<String> {
    let json = serde_json::to_string(value).context("encode visual view JSON")?;
    Ok(json
        .replace('&', "\\u0026")
        .replace('<', "\\u003C")
        .replace('>', "\\u003E")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029"))
}

pub(crate) fn push_escaped(html: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '&' => html.push_str("&amp;"),
            '<' => html.push_str("&lt;"),
            '>' => html.push_str("&gt;"),
            '"' => html.push_str("&quot;"),
            '\'' => html.push_str("&#39;"),
            _ => html.push(ch),
        }
    }
}

pub(crate) fn push_escaped_attr(html: &mut String, value: &str) {
    push_escaped(html, value);
}

pub(crate) fn domain_classification_label(value: DomainClassification) -> &'static str {
    match value {
        DomainClassification::WellStructured => "well_structured",
        DomainClassification::Formal => "formal",
        DomainClassification::IllStructured => "ill_structured",
        DomainClassification::ProfessionalPractice => "professional_practice",
        DomainClassification::InstrumentBound => "instrument_bound",
        DomainClassification::InfrastructureBound => "infrastructure_bound",
        DomainClassification::Emerging => "emerging",
        DomainClassification::Interdisciplinary => "interdisciplinary",
        DomainClassification::Mixed => "mixed",
    }
}

pub(crate) fn source_role_requirement_label(value: SourceRoleRequirementKind) -> &'static str {
    match value {
        SourceRoleRequirementKind::Required => "required",
        SourceRoleRequirementKind::Conditional => "conditional",
        SourceRoleRequirementKind::Waived => "waived",
        SourceRoleRequirementKind::NotApplicable => "not_applicable",
    }
}

pub(crate) fn verification_status_label(value: VerificationStatus) -> &'static str {
    match value {
        VerificationStatus::Cataloged => "cataloged",
        VerificationStatus::Reviewed => "reviewed",
        VerificationStatus::Verified => "verified",
    }
}

pub(crate) fn claim_type_label(value: ClaimType) -> &'static str {
    match value {
        ClaimType::Structural => "structural",
        ClaimType::Currentness => "currentness",
        ClaimType::Frontier => "frontier",
        ClaimType::Debate => "debate",
        ClaimType::Curricular => "curricular",
        ClaimType::Interpretive => "interpretive",
        ClaimType::Methodological => "methodological",
    }
}

pub(crate) fn evidence_requirement_label(value: EvidenceRequirement) -> &'static str {
    match value {
        EvidenceRequirement::None => "none",
        EvidenceRequirement::CatalogedSource => "cataloged_source",
        EvidenceRequirement::ReviewedSource => "reviewed_source",
        EvidenceRequirement::VerifiedSource => "verified_source",
        EvidenceRequirement::MultipleReviewedSources => "multiple_reviewed_sources",
    }
}

pub(crate) fn claim_confidence_label(value: ClaimConfidence) -> &'static str {
    match value {
        ClaimConfidence::High => "high",
        ClaimConfidence::Medium => "medium",
        ClaimConfidence::Low => "low",
        ClaimConfidence::Unknown => "unknown",
    }
}

pub(crate) fn support_kind_label(value: SupportKind) -> &'static str {
    match value {
        SupportKind::Supports => "supports",
        SupportKind::Qualifies => "qualifies",
        SupportKind::Contradicts => "contradicts",
        SupportKind::Background => "background",
        SupportKind::Example => "example",
    }
}

pub(crate) fn frontier_debate_kind_label(value: FrontierDebateKind) -> &'static str {
    match value {
        FrontierDebateKind::Frontier => "frontier",
        FrontierDebateKind::Debate => "debate",
        FrontierDebateKind::OpenProblem => "open_problem",
        FrontierDebateKind::Uncertainty => "uncertainty",
    }
}

pub(crate) fn temporal_status_label(value: TemporalStatus) -> &'static str {
    match value {
        TemporalStatus::Durable => "durable",
        TemporalStatus::Current => "current",
        TemporalStatus::ReviewDue => "review_due",
        TemporalStatus::Stale => "stale",
        TemporalStatus::Unknown => "unknown",
    }
}

pub(crate) fn visual_view_kind_label(value: VisualViewKind) -> Option<&'static str> {
    match value {
        VisualViewKind::KnowledgeSpine => Some("knowledge_spine"),
        VisualViewKind::ConceptSource => Some("concept_source"),
        VisualViewKind::DependencyPath => Some("dependency_path"),
        VisualViewKind::FrontierDebate => Some("frontier_debate"),
        VisualViewKind::Custom => None,
    }
}
