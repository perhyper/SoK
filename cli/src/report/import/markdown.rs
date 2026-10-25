use super::*;

pub(crate) fn parse_markdown_report(markdown: &str) -> ParsedMarkdownReport {
    let mut parsed = ParsedMarkdownReport {
        title_field: infer_title_field(markdown),
        placeholder_lines: collect_placeholder_lines(markdown),
        ..ParsedMarkdownReport::default()
    };
    let sections = parse_markdown_sections(markdown, &mut parsed.diagnostics);
    let mut seen = BTreeSet::new();

    for section in sections {
        let public_body = strip_sok_directives(&section.body);
        let visual_view_ids = sok_directive_values(&section.body, "visual-view");
        if is_public_narrative_section(section.canonical) {
            parsed.narrative_sections.push(ParsedNarrativeSection {
                title: section.title.clone(),
                purpose: first_sok_directive_value(&section.body, "purpose"),
                body_markdown: public_body,
                visual_view_ids,
            });
        }

        let Some(canonical) = section.canonical else {
            // Arbitrary H2 sections form the ordered public narrative. The
            // canonical surfaces below remain available for machine extraction
            // without dictating the report's table of contents.
            continue;
        };
        parsed.sections.insert(canonical);
        if !seen.insert(canonical) {
            parsed.diagnostics.push(DiagnosticCheck::warning(
                CHECK_EXPORT_AMBIGUOUS_SECTION,
                format!(
                    "duplicate canonical Markdown section {}; only deterministic table extraction is supported",
                    canonical_section_label(canonical)
                ),
            ));
        }

        match canonical {
            CanonicalSection::ResearchFrame => {
                parsed
                    .research_frame
                    .extend(parse_key_value_table(&section.body));
            }
            CanonicalSection::ReportArchitecture => {
                parsed
                    .report_architecture
                    .extend(parse_key_value_table(&section.body));
            }
            CanonicalSection::DomainDecomposition => {
                parsed.domain_decomposition = public_section_text(&section.body);
            }
            CanonicalSection::Orientation => {
                parsed.orientation = public_section_text(&section.body);
            }
            CanonicalSection::DeepStructure => {
                parsed
                    .deep_structure_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::SourceRoleProbe => {
                parsed
                    .source_role_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::LiteratureLadder => {
                parsed
                    .literature_ladder_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::Relations => {
                parsed
                    .relation_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::CurriculumRoadmap => {
                parsed
                    .curriculum_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::FrontierDebates => {
                parsed
                    .frontier_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::Claims => {
                parsed
                    .claim_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::PracticeAssessment => {
                parsed.practice_text = public_section_text(&section.body);
            }
            CanonicalSection::SoKUsefulnessNotes => {
                parsed.usefulness_notes = bullet_or_paragraph_lines(&section.body);
            }
            CanonicalSection::ScaffoldQualityNotes => {
                parsed.quality_notes = bullet_or_paragraph_lines(&section.body);
            }
            CanonicalSection::VisualSummary => {
                parsed.visual_summary = public_section_text(&section.body);
            }
            CanonicalSection::VisualViews => {
                parsed
                    .visual_view_rows
                    .extend(parse_first_markdown_table(&section.body));
            }
            CanonicalSection::VisualMap => {
                // Mermaid or prose maps are recognized here but intentionally not
                // interpreted as semantic relations. Export diagnostics are emitted
                // later if no structured Visual Views table preserves the intent.
            }
            CanonicalSection::SourcesFurtherReading | CanonicalSection::SourcePointers => {
                parsed.diagnostics.push(DiagnosticCheck::warning(
                    CHECK_EXPORT_UNSUPPORTED_SECTION,
                    format!(
                        "Markdown source section {} was ignored; --sources is the source-of-truth manifest",
                        canonical_section_label(canonical)
                    ),
                ));
            }
        }
    }

    parsed
}

pub(crate) fn parse_markdown_sections(
    markdown: &str,
    diagnostics: &mut Vec<DiagnosticCheck>,
) -> Vec<MarkdownSection> {
    let mut sections = Vec::new();
    let mut current_title = String::new();
    let mut current_body = Vec::new();
    let mut fence = None;

    for line in markdown.lines() {
        let trimmed = line.trim();
        let was_fenced = fence.is_some();
        let is_fence_delimiter = update_markdown_fence(line, &mut fence);
        let is_fenced = was_fenced || is_fence_delimiter || fence.is_some();
        if !is_fenced && trimmed.starts_with("## ") && !trimmed.starts_with("### ") {
            if !current_title.is_empty() {
                let body = current_body.join("\n");
                push_markdown_section(&mut sections, current_title, body, diagnostics);
                current_body.clear();
            }
            current_title = trimmed.trim_start_matches('#').trim().to_string();
        } else if !current_title.is_empty() {
            current_body.push(line.to_string());
        }
    }

    if fence.is_some() {
        diagnostics.push(DiagnosticCheck::warning(
            CHECK_EXPORT_UNSUPPORTED_SECTION,
            "Markdown has an unclosed fenced code block; fenced content was not interpreted",
        ));
    }
    if !current_title.is_empty() {
        let body = current_body.join("\n");
        push_markdown_section(&mut sections, current_title, body, diagnostics);
    }
    sections
}

pub(crate) fn push_markdown_section(
    sections: &mut Vec<MarkdownSection>,
    title: String,
    body: String,
    diagnostics: &mut Vec<DiagnosticCheck>,
) {
    for (key, value) in standalone_sok_directives(&body) {
        if !matches!(key.as_str(), "surface" | "purpose" | "visual-view") {
            diagnostics.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_UNKNOWN_DIRECTIVE,
                    format!("Markdown section {title:?} has unknown sok directive {key:?}"),
                )
                .with_target("/report/sections", &key),
            );
        } else if value.is_empty() {
            diagnostics.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_UNKNOWN_DIRECTIVE,
                    format!("Markdown section {title:?} has empty sok:{key} directive"),
                )
                .with_target("/report/sections", &key),
            );
        }
    }
    let markers = sok_directive_values(&body, "surface");
    let canonical_from_marker = canonical_section_from_marker(&body);
    if markers.len() > 1 {
        diagnostics.push(
            DiagnosticCheck::error(
                CHECK_EXPORT_AMBIGUOUS_SECTION,
                format!(
                    "Markdown section {title:?} has multiple sok:surface markers: {}",
                    markers.join(", ")
                ),
            )
            .with_target("/report/sections", &title),
        );
    }
    if let Some(marker) = markers.first() {
        if canonical_from_marker.is_none() {
            diagnostics.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_UNKNOWN_SURFACE_MARKER,
                    format!("Markdown section {title:?} has unknown sok:surface marker {marker:?}"),
                )
                .with_target("/report/sections", marker),
            );
        }
    }
    sections.push(MarkdownSection {
        canonical: canonical_from_marker.or_else(|| canonical_section(&title)),
        title,
        body,
    });
}

pub(crate) fn canonical_section_from_marker(body: &str) -> Option<CanonicalSection> {
    let surface = first_sok_directive_value(body, "surface");
    match normalize_id_text(&surface).as_str() {
        "report architecture" | "architecture" => Some(CanonicalSection::ReportArchitecture),
        "domain decomposition" => Some(CanonicalSection::DomainDecomposition),
        "orientation" => Some(CanonicalSection::Orientation),
        "deep structure" | "field elements" | "element inventory" => {
            Some(CanonicalSection::DeepStructure)
        }
        "source role probe" => Some(CanonicalSection::SourceRoleProbe),
        "literature ladder" => Some(CanonicalSection::LiteratureLadder),
        "relations" => Some(CanonicalSection::Relations),
        "curriculum roadmap" | "curriculum" => Some(CanonicalSection::CurriculumRoadmap),
        "practice assessment" | "practice" => Some(CanonicalSection::PracticeAssessment),
        "frontier debates" | "frontier" => Some(CanonicalSection::FrontierDebates),
        "visual map" => Some(CanonicalSection::VisualMap),
        "visual summary" => Some(CanonicalSection::VisualSummary),
        "visual views" => Some(CanonicalSection::VisualViews),
        "claims" => Some(CanonicalSection::Claims),
        _ => None,
    }
}

pub(crate) fn is_public_narrative_section(section: Option<CanonicalSection>) -> bool {
    matches!(
        section,
        None | Some(CanonicalSection::DomainDecomposition)
            | Some(CanonicalSection::Orientation)
            | Some(CanonicalSection::DeepStructure)
            | Some(CanonicalSection::CurriculumRoadmap)
            | Some(CanonicalSection::PracticeAssessment)
            | Some(CanonicalSection::FrontierDebates)
    )
}

pub(crate) fn sok_directive_values(body: &str, key: &str) -> Vec<String> {
    standalone_sok_directives(body)
        .into_iter()
        .filter_map(|(candidate, value)| (candidate == key).then_some(value))
        .filter(|value| !value.is_empty())
        .collect()
}

pub(crate) fn standalone_sok_directives(body: &str) -> Vec<(String, String)> {
    let mut fence = None;
    let mut values = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if update_markdown_fence(line, &mut fence) {
            continue;
        }
        if fence.is_some() {
            continue;
        }
        if let Some(directive) = trimmed
            .strip_prefix("<!-- sok:")
            .and_then(|value| value.strip_suffix("-->"))
            .map(str::trim)
        {
            let mut parts = directive.splitn(2, char::is_whitespace);
            let key = parts.next().unwrap_or_default().trim().to_string();
            let value = parts.next().unwrap_or_default().trim().to_string();
            values.push((key, value));
        }
    }
    values
}

pub(crate) fn first_sok_directive_value(body: &str, key: &str) -> String {
    sok_directive_values(body, key)
        .into_iter()
        .next()
        .unwrap_or_default()
}

pub(crate) fn strip_sok_directives(body: &str) -> String {
    let mut fence = None;
    let mut skipped_sok_json_fence = None;
    let mut lines = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(active) = skipped_sok_json_fence {
            if markdown_fence_closer(line, active) {
                skipped_sok_json_fence = None;
            }
            continue;
        }
        if fence.is_none() {
            if let Some((candidate, info)) = markdown_fence_opener(line) {
                if is_sok_json_info(&info) {
                    skipped_sok_json_fence = Some(candidate);
                    continue;
                }
            }
        }
        if update_markdown_fence(line, &mut fence) {
            lines.push(line);
            continue;
        }
        if fence.is_none() && trimmed.starts_with("<!-- sok:") && trimmed.ends_with("-->") {
            continue;
        }
        lines.push(line);
    }
    lines.join("\n").trim().to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MarkdownFence {
    marker: char,
    length: usize,
}

pub(crate) fn update_markdown_fence(line: &str, fence: &mut Option<MarkdownFence>) -> bool {
    let trimmed = line.trim();
    let Some(marker) = trimmed.chars().next() else {
        return false;
    };
    if !matches!(marker, '`' | '~') {
        return false;
    }
    let length = trimmed.chars().take_while(|ch| *ch == marker).count();
    if length < 3 {
        return false;
    }
    let remainder = &trimmed[length..];

    if let Some(active) = *fence {
        if marker == active.marker && length >= active.length && remainder.trim().is_empty() {
            *fence = None;
            return true;
        }
        return false;
    }

    // CommonMark does not allow a backtick in the info string of a
    // backtick-delimited code fence.
    if marker == '`' && remainder.contains('`') {
        return false;
    }
    *fence = Some(MarkdownFence { marker, length });
    true
}

pub(super) fn markdown_fence_opener(line: &str) -> Option<(MarkdownFence, String)> {
    let trimmed = line.trim();
    let marker = trimmed.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let length = trimmed.chars().take_while(|ch| *ch == marker).count();
    if length < 3 {
        return None;
    }
    let info = trimmed[length..].trim().to_string();
    if marker == '`' && info.contains('`') {
        return None;
    }
    Some((MarkdownFence { marker, length }, info))
}

pub(super) fn markdown_fence_closer(line: &str, fence: MarkdownFence) -> bool {
    let trimmed = line.trim();
    let Some(marker) = trimmed.chars().next() else {
        return false;
    };
    if marker != fence.marker {
        return false;
    }
    let length = trimmed.chars().take_while(|ch| *ch == marker).count();
    length >= fence.length && trimmed[length..].trim().is_empty()
}

pub(super) fn is_sok_json_info(info: &str) -> bool {
    info.split_whitespace().next() == Some("sok-json")
}

pub(super) fn declared_sok_json_schema_version(info: &str) -> Option<&str> {
    info.split_whitespace()
        .skip(1)
        .find(|token| token.starts_with("sok-") && token.contains("/v"))
}

pub(crate) fn markdown_lines_outside_fences(markdown: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut fence = None;
    markdown.lines().enumerate().filter(move |(_, line)| {
        if update_markdown_fence(line, &mut fence) {
            return false;
        }
        fence.is_none()
    })
}

pub(crate) fn canonical_section(title: &str) -> Option<CanonicalSection> {
    let title = normalize_heading_title(title);
    match title.as_str() {
        "research frame" | "internal context" => Some(CanonicalSection::ResearchFrame),
        "report architecture" | "report architecture decision" | "narrative architecture" => {
            Some(CanonicalSection::ReportArchitecture)
        }
        "domain decomposition" => Some(CanonicalSection::DomainDecomposition),
        "orientation" | "executive orientation" => Some(CanonicalSection::Orientation),
        "the field s deep structure"
        | "field s deep structure"
        | "deep structure"
        | "field element inventory" => Some(CanonicalSection::DeepStructure),
        "source role probe" => Some(CanonicalSection::SourceRoleProbe),
        "literature ladder" | "literature ladder with access metadata" => {
            Some(CanonicalSection::LiteratureLadder)
        }
        "relations" | "relation table" | "semantic relations" | "knowledge relations" => {
            Some(CanonicalSection::Relations)
        }
        "curriculum roadmap" | "research roadmap" => Some(CanonicalSection::CurriculumRoadmap),
        "practice and assessment" => Some(CanonicalSection::PracticeAssessment),
        "frontier debates and open problems"
        | "frontier debate map"
        | "frontier and debate map"
        | "frontier and debates"
        | "frontier debates"
        | "frontier and debate"
        | "frontier open problems and debates" => Some(CanonicalSection::FrontierDebates),
        "visual summary" => Some(CanonicalSection::VisualSummary),
        "visual views" | "visual view declarations" | "visual declarations" => {
            Some(CanonicalSection::VisualViews)
        }
        "concept and prerequisite map"
        | "debate and case network"
        | "instrument data and standards map"
        | "concept map" => Some(CanonicalSection::VisualMap),
        "sources and further reading" => Some(CanonicalSection::SourcesFurtherReading),
        "source pointers" => Some(CanonicalSection::SourcePointers),
        "sok usefulness notes" => Some(CanonicalSection::SoKUsefulnessNotes),
        "scaffold quality notes" => Some(CanonicalSection::ScaffoldQualityNotes),
        "claims" | "evidence backed claims" | "public claims" => Some(CanonicalSection::Claims),
        _ => None,
    }
}

pub(crate) fn canonical_section_label(section: CanonicalSection) -> &'static str {
    match section {
        CanonicalSection::ResearchFrame => "Research Frame",
        CanonicalSection::ReportArchitecture => "Report Architecture",
        CanonicalSection::DomainDecomposition => "Domain Decomposition",
        CanonicalSection::Orientation => "Orientation",
        CanonicalSection::DeepStructure => "Deep Structure",
        CanonicalSection::SourceRoleProbe => "Source Role Probe",
        CanonicalSection::LiteratureLadder => "Literature Ladder",
        CanonicalSection::Relations => "Relations",
        CanonicalSection::CurriculumRoadmap => "Curriculum Roadmap",
        CanonicalSection::PracticeAssessment => "Practice and Assessment",
        CanonicalSection::FrontierDebates => "Frontier, Debates, and Open Problems",
        CanonicalSection::VisualMap => "Visual Map",
        CanonicalSection::VisualSummary => "Visual Summary",
        CanonicalSection::VisualViews => "Visual Views",
        CanonicalSection::SourcesFurtherReading => "Sources and Further Reading",
        CanonicalSection::SourcePointers => "Source Pointers",
        CanonicalSection::SoKUsefulnessNotes => "SoK Usefulness Notes",
        CanonicalSection::ScaffoldQualityNotes => "Scaffold Quality Notes",
        CanonicalSection::Claims => "Claims",
    }
}

pub(crate) fn normalize_heading_title(title: &str) -> String {
    let normalized = normalize_id_text(title);
    normalized
        .trim_start_matches(|ch: char| ch.is_ascii_digit() || ch.is_whitespace())
        .trim()
        .to_string()
}

pub(crate) fn parse_key_value_table(markdown: &str) -> BTreeMap<String, String> {
    parse_first_markdown_table(markdown)
        .into_iter()
        .filter_map(|row| {
            let key = lookup_cell(&row, &["item", "key", "field", "label"]);
            let value = lookup_cell(
                &row,
                &["value", "decision", "so k extraction", "in this field"],
            );
            if key.is_empty() || value.is_empty() {
                None
            } else {
                Some((key, value))
            }
        })
        .collect()
}

pub(crate) fn parse_first_markdown_table(markdown: &str) -> Vec<BTreeMap<String, String>> {
    let mut header: Vec<String> = Vec::new();
    let mut rows = Vec::new();
    let mut in_table = false;

    for (_, line) in markdown_lines_outside_fences(markdown) {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            if in_table && !trimmed.is_empty() {
                break;
            }
            continue;
        }
        let cells = parse_table_cells(trimmed);
        if cells.is_empty() || is_table_separator(&cells) {
            continue;
        }
        if header.is_empty() {
            header = cells
                .into_iter()
                .map(|cell| normalize_id_text(&cell))
                .collect();
            in_table = true;
            continue;
        }
        let mut row = BTreeMap::new();
        for (index, value) in cells.into_iter().enumerate() {
            let key = header
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("column_{index}"));
            row.insert(key, clean_inline_markdown(&value));
        }
        if !row.values().all(|value| value.trim().is_empty()) {
            rows.push(row);
        }
    }

    rows
}

pub(crate) fn parse_table_cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

pub(crate) fn is_table_separator(cells: &[String]) -> bool {
    cells.iter().all(|cell| {
        let value = cell.trim();
        !value.is_empty() && value.chars().all(|ch| matches!(ch, '-' | ':' | ' ' | '\t'))
    })
}

pub(crate) fn public_section_text(markdown: &str) -> String {
    collapse_whitespace(
        &markdown_lines_outside_fences(markdown)
            .filter_map(|(_, line)| {
                let trimmed = line.trim();
                (!trimmed.is_empty() && !trimmed.starts_with('|') && !is_placeholder_text(trimmed))
                    .then_some(line)
            })
            .map(clean_inline_markdown)
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub(crate) fn bullet_or_paragraph_lines(markdown: &str) -> Vec<String> {
    markdown_lines_outside_fences(markdown)
        .map(|(_, line)| {
            line.trim()
                .trim_start_matches("- ")
                .trim_start_matches("* ")
                .trim()
                .to_string()
        })
        .filter(|line| !line.is_empty() && !line.starts_with('|'))
        .map(|line| clean_inline_markdown(&line))
        .collect()
}

pub(crate) fn clean_inline_markdown(raw: &str) -> String {
    collapse_whitespace(
        &raw.replace('`', "")
            .replace("**", "")
            .replace("__", "")
            .replace("<br>", "; ")
            .replace("<br/>", "; ")
            .replace("<br />", "; "),
    )
}

pub(crate) fn collapse_whitespace(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn collect_placeholder_lines(markdown: &str) -> Vec<String> {
    markdown_lines_outside_fences(markdown)
        .map(|(_, line)| line.trim())
        .filter(|line| is_placeholder_text(line))
        .map(clean_inline_markdown)
        .collect()
}

pub(crate) fn is_placeholder_text(text: &str) -> bool {
    let normalized = normalize_id_text(text);
    normalized.contains("source to verify")
        || normalized.contains("date after lookup")
        || normalized.contains("budget unknown")
        || normalized.contains("until verified")
        || normalized.contains("replace starter source")
        || normalized.contains("scaffold quality")
        || normalized.contains("sentinel internal")
}

pub(crate) fn lookup_cell(row: &BTreeMap<String, String>, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| row.get(*key))
        .cloned()
        .unwrap_or_default()
        .trim()
        .to_string()
}

pub(crate) fn lookup_cell_any(row: &BTreeMap<String, String>, keys: &[&str]) -> String {
    for key in keys {
        let normalized = normalize_id_text(key);
        if let Some(value) = row.get(&normalized) {
            return value.trim().to_string();
        }
    }
    String::new()
}

pub(crate) fn infer_title_field(markdown: &str) -> String {
    for (_, line) in markdown_lines_outside_fences(markdown) {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("# Structure of Knowledge:") {
            return clean_inline_markdown(value);
        }
        if let Some(value) = line.strip_prefix("# SoK v0 Sample:") {
            return clean_inline_markdown(value);
        }
        if line.starts_with("# SoK") && line.contains(':') {
            if let Some((_, value)) = line.split_once(':') {
                return clean_inline_markdown(value);
            }
        }
    }
    String::new()
}
