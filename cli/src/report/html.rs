use super::import::*;
use super::model::*;
use super::provenance::first_non_empty;
use super::relations::{relation_kind_label, visual_relation_matches_nodes};
use super::validation::*;
use super::*;
use pulldown_cmark::html as cmark_html;

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
        push_reading_guide_section(&mut html, document, !views.is_empty());
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

pub(crate) const REPORT_CSS: &str = r#":root {
  color-scheme: light dark;
  --bg: #f6f7f8;
  --paper: #ffffff;
  --panel: #fbfcfd;
  --ink: #1b252f;
  --muted: #5d6b78;
  --line: #d8dee4;
  --accent: #2364aa;
  --accent-soft: #e8f1fb;
  --success: #1f7a4d;
  --warn: #8a5a00;
  --danger: #a33a3a;
  --focus: #9b5de5;
  --shadow: 0 1px 2px rgba(24, 34, 45, 0.08);
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; }
body {
  margin: 0;
  background: var(--bg);
  color: var(--ink);
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 16px;
  line-height: 1.55;
}
.skip-link {
  position: absolute;
  left: 1rem;
  top: -4rem;
  z-index: 10;
  background: var(--paper);
  border: 2px solid var(--focus);
  color: var(--ink);
  padding: 0.5rem 0.75rem;
}
.skip-link:focus { top: 1rem; }
.report-shell {
  width: min(1120px, calc(100% - 32px));
  margin: 0 auto;
  padding: 28px 0 56px;
}
.report-header {
  padding: 28px 0 18px;
  border-bottom: 1px solid var(--line);
}
.eyebrow {
  margin: 0 0 0.45rem;
  color: var(--accent);
  font-size: 0.82rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0;
}
h1, h2, h3, h4 {
  color: var(--ink);
  line-height: 1.2;
  letter-spacing: 0;
}
h1 {
  margin: 0;
  font-size: clamp(2rem, 5vw, 3.7rem);
}
h2 {
  margin: 0 0 0.9rem;
  font-size: 1.55rem;
}
h3 {
  margin: 0 0 0.55rem;
  font-size: 1.08rem;
}
h4 {
  margin: 0.8rem 0 0.4rem;
  font-size: 0.95rem;
}
.summary {
  max-width: 860px;
  margin: 0.9rem 0 0;
  color: var(--muted);
  font-size: 1.08rem;
}
.section-nav {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  margin-top: 1.35rem;
}
.section-nav a {
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--paper);
  color: var(--ink);
  padding: 0.35rem 0.55rem;
  font-size: 0.9rem;
  text-decoration: none;
}
.section-nav a:focus,
.section-nav a:hover {
  border-color: var(--accent);
  outline: 2px solid transparent;
}
.report-section {
  padding: 28px 0;
  border-bottom: 1px solid var(--line);
}
.architecture-note {
  display: grid;
  grid-template-columns: minmax(180px, 0.32fr) minmax(0, 1fr);
  gap: 14px;
  margin: 24px 0 0;
  padding: 14px 16px;
  border-left: 4px solid var(--accent);
  background: var(--accent-soft);
}
.architecture-note p { margin: 0; }
.architecture-note span {
  display: block;
  margin-bottom: 0.2rem;
  color: var(--accent);
  font-size: 0.78rem;
  font-weight: 700;
  text-transform: uppercase;
}
.section-purpose {
  max-width: 820px;
  margin: -0.35rem 0 1rem;
  color: var(--muted);
  font-size: 0.94rem;
}
.narrative-body {
  max-width: 880px;
  min-width: 0;
  overflow-x: auto;
  font-family: ui-serif, Georgia, Cambria, "Times New Roman", serif;
  font-size: 1.04rem;
}
.narrative-body h3,
.narrative-body h4 { margin-top: 1.4rem; }
.narrative-body p,
.narrative-body ul,
.narrative-body ol,
.narrative-body blockquote { max-width: 78ch; }
.narrative-body blockquote {
  margin-left: 0;
  padding-left: 1rem;
  border-left: 3px solid var(--line);
  color: var(--muted);
}
.structured-appendix {
  margin-top: 36px;
  padding: 16px 18px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel);
}
.structured-appendix > summary {
  cursor: pointer;
  color: var(--ink);
  font-weight: 700;
}
.prose {
  max-width: 860px;
  margin: 0.35rem 0 0;
}
.muted { color: var(--muted); }
.guide-list,
.ladder-list,
.curriculum-path {
  display: grid;
  gap: 12px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.guide-list li {
  display: grid;
  grid-template-columns: minmax(170px, 0.34fr) minmax(0, 1fr);
  gap: 12px;
  padding: 12px 14px;
  background: var(--paper);
  border: 1px solid var(--line);
  border-radius: 8px;
  box-shadow: var(--shadow);
}
.guide-list span { color: var(--muted); }
.split-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 18px;
}
.item-grid,
.claim-grid,
.source-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}
.item-card,
.visual-card,
.claim-card,
.path-card,
.ladder-card,
.source-card,
.frontier-card,
.relation-card {
  background: var(--paper);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 16px;
  box-shadow: var(--shadow);
}
.item-card p,
.visual-card p,
.claim-card p,
.path-card p,
.ladder-card p,
.source-card p,
.frontier-card p,
.relation-card p {
  margin: 0.35rem 0 0;
}
.badge-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  margin: 0.55rem 0 0.75rem;
}
.badge {
  display: inline-flex;
  align-items: center;
  max-width: 100%;
  min-height: 1.65rem;
  padding: 0.15rem 0.45rem;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent);
  font-size: 0.78rem;
  font-weight: 700;
  overflow-wrap: anywhere;
}
.meta {
  margin-top: 0.7rem;
  color: var(--muted);
  font-size: 0.92rem;
}
.curriculum-path {
  position: relative;
  gap: 16px;
}
.path-step {
  display: grid;
  grid-template-columns: 44px minmax(0, 1fr);
  gap: 12px;
  align-items: start;
}
.path-marker {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  border: 2px solid var(--accent);
  border-radius: 50%;
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 800;
}
.reader-dl,
.compact-dl {
  display: grid;
  grid-template-columns: minmax(120px, 0.24fr) minmax(0, 1fr);
  gap: 0.35rem 0.8rem;
  margin: 0.65rem 0 0;
}
.reader-dl dt,
.compact-dl dt {
  color: var(--muted);
  font-weight: 700;
}
.reader-dl dd,
.compact-dl dd {
  margin: 0;
  overflow-wrap: anywhere;
}
.reference-block { margin-top: 0.75rem; }
.reference-list,
.evidence-list {
  margin: 0.35rem 0 0 1.1rem;
  padding: 0;
}
.reference-list li,
.evidence-list li {
  margin: 0.35rem 0;
}
.evidence-group {
  margin-top: 0.85rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--line);
}
.evidence-meta {
  display: block;
  margin-top: 0.15rem;
  color: var(--muted);
  font-size: 0.88rem;
}
.text-fallback,
.visual-fallback {
  margin-top: 0.9rem;
  padding: 0.85rem;
  border-left: 4px solid var(--accent);
  background: var(--panel);
}
.raw-identifiers {
  margin-top: 0.85rem;
  color: var(--muted);
}
.raw-identifiers summary {
  cursor: pointer;
  color: var(--ink);
  font-weight: 700;
}
.frontier-list,
.relation-list {
  display: grid;
  gap: 14px;
}
.table-scroll {
  width: 100%;
  max-width: 100%;
  overflow-x: auto;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--paper);
}
table {
  width: 100%;
  min-width: 760px;
  border-collapse: collapse;
}
th,
td {
  padding: 0.7rem 0.75rem;
  border-bottom: 1px solid var(--line);
  text-align: left;
  vertical-align: top;
}
th {
  background: #eef3f7;
  font-size: 0.82rem;
  text-transform: uppercase;
  letter-spacing: 0;
}
td {
  font-size: 0.94rem;
  white-space: pre-wrap;
}
tr:last-child td { border-bottom: 0; }
.visual-canvas {
  min-height: 380px;
  margin-top: 0.9rem;
}
.visual-svg {
  display: block;
  width: 100%;
  height: 360px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fbfcfd;
}
.visual-node circle,
.visual-node rect {
  fill: var(--accent-soft);
  stroke: var(--accent);
  stroke-width: 2;
}
.visual-node text {
  fill: var(--ink);
  font-size: 13px;
  pointer-events: none;
}
.visual-edge {
  stroke: #6f7f8f;
  stroke-width: 2;
}
.visual-node.is-active circle,
.visual-node.is-active rect {
  fill: #fff4cc;
  stroke: #a15c00;
}
.visual-edge.is-active {
  stroke: #a15c00;
  stroke-width: 3;
}
.visual-fallback ul {
  margin: 0.4rem 0 0.8rem 1.1rem;
  padding: 0;
}
.visual-fallback li { margin: 0.2rem 0; }
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #11161c;
    --paper: #18212a;
    --panel: #141c24;
    --ink: #edf2f7;
    --muted: #aeb9c5;
    --line: #344250;
    --accent: #8fc5ff;
    --accent-soft: #17324f;
    --success: #7bd6a3;
    --warn: #f0c36a;
    --danger: #ff9b9b;
    --focus: #d2a8ff;
    --shadow: none;
  }
  th,
  .visual-svg {
    background: var(--panel);
  }
  .visual-edge {
    stroke: #9aa8b6;
  }
  .visual-node.is-active rect {
    fill: #3b2e17;
    stroke: var(--warn);
  }
  .visual-edge.is-active {
    stroke: var(--warn);
  }
}
@media (max-width: 720px) {
  .report-shell {
    width: min(100% - 20px, 1120px);
    padding-top: 16px;
  }
  .report-header { padding-top: 20px; }
  .split-list,
  .item-grid,
  .claim-grid,
  .source-grid {
    grid-template-columns: 1fr;
  }
  .guide-list li,
  .path-step,
  .architecture-note,
  .reader-dl,
  .compact-dl {
    grid-template-columns: 1fr;
  }
  table { min-width: 640px; }
  .visual-canvas { min-height: 320px; }
  .visual-svg { height: 300px; }
}
@media print {
  body {
    background: #ffffff;
    color: #000000;
    font-size: 11pt;
  }
  .report-shell {
    width: 100%;
    padding: 0;
  }
  .section-nav,
  .skip-link,
  .visual-canvas,
  script {
    display: none !important;
  }
  .report-header,
  .report-section {
    border-color: #bbbbbb;
  }
  .item-card,
  .visual-card,
  .claim-card,
  .path-card,
  .ladder-card,
  .source-card,
  .frontier-card,
  .relation-card,
  .table-scroll {
    border-color: #bbbbbb;
    break-inside: avoid;
  }
  .table-scroll {
    overflow: visible;
  }
  details.structured-appendix > summary {
    display: none;
  }
  details.structured-appendix:not([open]) > :not(summary) {
    display: block !important;
  }
  table {
    min-width: 0;
    font-size: 9pt;
  }
  a {
    color: inherit;
    text-decoration: none;
  }
}"#;

pub(crate) const REPORT_JS: &str = r##"(function () {
  var dataNode = document.getElementById("sok-visual-data");
  if (!dataNode) return;
  var views;
  try {
    views = JSON.parse(dataNode.textContent || "[]");
  } catch (error) {
    return;
  }
  if (!Array.isArray(views) || views.length === 0) return;

  var svgNs = "http://www.w3.org/2000/svg";
  function el(name, attrs) {
    var node = document.createElementNS(svgNs, name);
    Object.keys(attrs || {}).forEach(function (key) {
      node.setAttribute(key, attrs[key]);
    });
    return node;
  }
  function textNode(value) {
    return document.createTextNode(value == null ? "" : String(value));
  }
  function labelLines(node) {
    if (Array.isArray(node.label_lines) && node.label_lines.length > 0) {
      return node.label_lines.slice(0, 2);
    }
    var label = String(node.label || node.id || "");
    return [label.length > 20 ? label.slice(0, 19) + "\u2026" : label];
  }
  function layout(view) {
    var nodes = view.nodes || [];
    if (view.kind === "knowledge_spine" || view.kind === "dependency_path") {
      var ranked = {};
      var hasCompleteLayout = nodes.length > 0;
      nodes.forEach(function (node) {
        if (!node.layout || typeof node.layout.x !== "number" || typeof node.layout.y !== "number") {
          hasCompleteLayout = false;
          return;
        }
        ranked[node.id] = { x: node.layout.x, y: node.layout.y };
      });
      if (hasCompleteLayout) return ranked;
    }
    if (view.kind === "concept_source") {
      var left = nodes.filter(function (node) { return node.entity_type === "source"; });
      var right = nodes.filter(function (node) { return node.entity_type !== "source"; });
      var positions = {};
      left.forEach(function (node, index) {
        positions[node.id] = { x: 190, y: 85 + index * (210 / Math.max(1, left.length - 1)) };
      });
      right.forEach(function (node, index) {
        positions[node.id] = { x: 720, y: 85 + index * (210 / Math.max(1, right.length - 1)) };
      });
      return positions;
    }
    if (view.kind === "frontier_debate") {
      var cx = 480;
      var cy = 180;
      var radius = 125;
      var out = {};
      nodes.forEach(function (node, index) {
        if (index === 0) {
          out[node.id] = { x: cx, y: cy };
        } else {
          var angle = -Math.PI / 2 + (2 * Math.PI * (index - 1)) / Math.max(1, nodes.length - 1);
          out[node.id] = { x: cx + Math.cos(angle) * radius, y: cy + Math.sin(angle) * radius };
        }
      });
      return out;
    }
    var step = 720 / Math.max(1, nodes.length - 1);
    var base = {};
    nodes.forEach(function (node, index) {
      base[node.id] = { x: 120 + index * step, y: 180 + (index % 2 === 0 ? -38 : 38) };
    });
    return base;
  }
  function connectedIds(view, nodeId) {
    var ids = {};
    (view.edges || []).forEach(function (edge) {
      if (edge.from === nodeId) ids[edge.to] = true;
      if (edge.to === nodeId) ids[edge.from] = true;
    });
    ids[nodeId] = true;
    return ids;
  }
  function render(mount, view) {
    var positions = layout(view);
    var markerId = "arrowhead-" + view.id;
    var svg = el("svg", {
      "class": "visual-svg",
      "role": "img",
      "aria-labelledby": "svg-title-" + view.id + " svg-desc-" + view.id,
      "viewBox": "0 0 960 360",
      "preserveAspectRatio": "xMidYMid meet"
    });
    var title = el("title", { "id": "svg-title-" + view.id });
    title.appendChild(textNode(view.title || "Visual view"));
    svg.appendChild(title);
    var desc = el("desc", { "id": "svg-desc-" + view.id });
    desc.appendChild(textNode(view.alt || view.justification || ""));
    svg.appendChild(desc);
    var defs = el("defs", {});
    var marker = el("marker", {
      "id": markerId,
      "viewBox": "0 0 10 10",
      "refX": "9",
      "refY": "5",
      "markerWidth": "7",
      "markerHeight": "7",
      "orient": "auto-start-reverse"
    });
    marker.appendChild(el("path", { "d": "M 0 0 L 10 5 L 0 10 z", "fill": "#6f7f8f" }));
    defs.appendChild(marker);
    svg.appendChild(defs);

    (view.edges || []).forEach(function (edge, index) {
      var from = positions[edge.from];
      var to = positions[edge.to];
      if (!from || !to) return;
      var line = el("line", {
        "class": "visual-edge",
        "tabindex": "0",
        "role": "img",
        "data-from": edge.from,
        "data-to": edge.to,
        "x1": from.x,
        "y1": from.y,
        "x2": to.x,
        "y2": to.y,
        "marker-end": "url(#" + markerId + ")",
        "aria-label": (edge.label || edge.kind || "edge") + ": " + (edge.from_label || edge.from) + " to " + (edge.to_label || edge.to)
      });
      line.setAttribute("data-edge-index", String(index));
      var edgeTitle = el("title", {});
      edgeTitle.appendChild(textNode((edge.label || edge.kind || "edge") + ": " + (edge.from_label || edge.from) + " to " + (edge.to_label || edge.to)));
      line.appendChild(edgeTitle);
      svg.appendChild(line);
    });

    (view.nodes || []).forEach(function (node) {
      var point = positions[node.id];
      if (!point) return;
      var group = el("g", {
        "class": "visual-node",
        "tabindex": "0",
        "role": "img",
        "aria-label": node.label + ". " + (node.description || node.entity_type || "node"),
        "data-node-id": node.id
      });
      group.appendChild(el("rect", {
        "x": point.x - 82,
        "y": point.y - 24,
        "width": "164",
        "height": "48",
        "rx": "7"
      }));
      var text = el("text", {
        "x": point.x,
        "y": point.y,
        "text-anchor": "middle"
      });
      var lines = labelLines(node);
      lines.forEach(function (line, index) {
        var tspan = el("tspan", {
          "x": point.x,
          "dy": index === 0 ? (lines.length === 1 ? "4" : "-4") : "15"
        });
        tspan.appendChild(textNode(line));
        text.appendChild(tspan);
      });
      group.appendChild(text);
      group.addEventListener("mouseenter", function () { activate(svg, view, node.id); });
      group.addEventListener("focus", function () { activate(svg, view, node.id); });
      group.addEventListener("mouseleave", function () { clearActive(svg); });
      group.addEventListener("blur", function () { clearActive(svg); });
      svg.appendChild(group);
    });
    mount.innerHTML = "";
    mount.appendChild(svg);
  }
  function activate(svg, view, nodeId) {
    var active = connectedIds(view, nodeId);
    Array.prototype.forEach.call(svg.querySelectorAll(".visual-node"), function (node) {
      node.classList.toggle("is-active", !!active[node.getAttribute("data-node-id")]);
    });
    Array.prototype.forEach.call(svg.querySelectorAll(".visual-edge"), function (edge) {
      var from = edge.getAttribute("data-from");
      var to = edge.getAttribute("data-to");
      edge.classList.toggle("is-active", from === nodeId || to === nodeId);
    });
  }
  function clearActive(svg) {
    Array.prototype.forEach.call(svg.querySelectorAll(".is-active"), function (node) {
      node.classList.remove("is-active");
    });
  }
  Array.prototype.forEach.call(document.querySelectorAll("[data-visual-mount]"), function (mount) {
    var id = mount.getAttribute("data-visual-mount");
    var view = views.find(function (item) { return item.id === id; });
    if (view) render(mount, view);
  });
})();"##;

#[derive(Debug, Serialize)]
pub(crate) struct RenderVisualView {
    id: String,
    kind: String,
    title: String,
    justification: String,
    alt: String,
    nodes: Vec<RenderVisualNode>,
    edges: Vec<RenderVisualEdge>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RenderVisualNode {
    id: String,
    label: String,
    label_lines: Vec<String>,
    entity_type: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    layout: Option<RenderVisualNodeLayout>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RenderVisualNodeLayout {
    rank: usize,
    lane: &'static str,
    order: usize,
    x: u16,
    y: u16,
}

#[derive(Debug, Serialize)]
pub(crate) struct RenderVisualEdge {
    from: String,
    to: String,
    kind: String,
    label: String,
    relation_id: String,
    from_label: String,
    to_label: String,
}

#[derive(Debug, Clone)]
pub(crate) struct VisualLayoutEdge {
    from: usize,
    to: usize,
    secondary: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct VisualLayoutPath {
    nodes: Vec<usize>,
    secondary_edges: usize,
    auxiliary_nodes: usize,
}

pub(crate) fn renderable_visual_views(report: &PublicReport) -> Vec<RenderVisualView> {
    let relation_lookup = report
        .relations
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect::<BTreeMap<_, _>>();
    report
        .visual_views
        .iter()
        .filter_map(|view| {
            let kind = visual_view_kind_label(view.kind)?;
            if view.justification.trim().is_empty() {
                return None;
            }
            let mut node_ids = BTreeSet::new();
            let mut nodes = view
                .nodes
                .iter()
                .filter_map(|node| {
                    if node.id.trim().is_empty() {
                        return None;
                    }
                    node_ids.insert(node.id.clone());
                    let label = first_non_empty([node.label.as_str(), node.id.as_str()]);
                    Some(RenderVisualNode {
                        id: node.id.clone(),
                        label_lines: visual_node_label_lines(&label),
                        label,
                        entity_type: entity_type_label(node.entity_type).to_string(),
                        description: node.description.clone(),
                        layout: None,
                    })
                })
                .collect::<Vec<_>>();
            let node_labels = nodes
                .iter()
                .map(|node| (node.id.as_str(), node.label.as_str()))
                .collect::<BTreeMap<_, _>>();
            let node_refs = view
                .nodes
                .iter()
                .filter(|node| !node.id.trim().is_empty() && !node.ref_id.trim().is_empty())
                .map(|node| (node.id.as_str(), node))
                .collect::<BTreeMap<_, _>>();
            let edges = view
                .edges
                .iter()
                .filter_map(|edge| {
                    let relation = relation_lookup.get(edge.relation_id.as_str())?;
                    let from_node = node_refs.get(edge.from.as_str())?;
                    let to_node = node_refs.get(edge.to.as_str())?;
                    if node_ids.contains(&edge.from)
                        && node_ids.contains(&edge.to)
                        && edge.from != edge.to
                        && visual_relation_matches_nodes(relation, from_node, to_node)
                    {
                        Some(RenderVisualEdge {
                            from: edge.from.clone(),
                            to: edge.to.clone(),
                            kind: first_non_empty([
                                edge.kind.as_str(),
                                relation_kind_label(relation.kind),
                            ]),
                            label: first_non_empty([
                                edge.label.as_str(),
                                relation.description.as_str(),
                                relation_kind_label(relation.kind),
                            ]),
                            relation_id: edge.relation_id.clone(),
                            from_label: node_labels
                                .get(edge.from.as_str())
                                .copied()
                                .unwrap_or(edge.from.as_str())
                                .to_string(),
                            to_label: node_labels
                                .get(edge.to.as_str())
                                .copied()
                                .unwrap_or(edge.to.as_str())
                                .to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if nodes.len() < 2 || edges.is_empty() {
                return None;
            }
            if matches!(
                view.kind,
                VisualViewKind::KnowledgeSpine | VisualViewKind::DependencyPath
            ) {
                apply_directed_visual_layout(&mut nodes, &edges);
            }
            let title = first_non_empty([view.title.as_str(), kind]);
            Some(RenderVisualView {
                id: view.id.clone(),
                kind: kind.to_string(),
                title: title.clone(),
                justification: view.justification.clone(),
                alt: visual_alt_text(&title, kind, &nodes, &edges),
                nodes,
                edges,
            })
        })
        .collect()
}

pub(crate) fn visual_node_label_lines(label: &str) -> Vec<String> {
    const MAX_LINE_CHARS: usize = 20;

    fn char_len(value: &str) -> usize {
        value.chars().count()
    }

    fn ellipsize(value: &str) -> String {
        if char_len(value) <= MAX_LINE_CHARS {
            return value.to_string();
        }
        value
            .chars()
            .take(MAX_LINE_CHARS - 1)
            .chain(std::iter::once('…'))
            .collect()
    }

    let normalized = label.split_whitespace().collect::<Vec<_>>().join(" ");
    if char_len(&normalized) <= MAX_LINE_CHARS {
        return vec![normalized];
    }

    let words = normalized.split_whitespace().collect::<Vec<_>>();
    if words.len() == 1 {
        let first = normalized.chars().take(MAX_LINE_CHARS).collect::<String>();
        let rest = normalized.chars().skip(MAX_LINE_CHARS).collect::<String>();
        return vec![first, ellipsize(&rest)];
    }

    let split = (1..words.len())
        .min_by_key(|split| {
            let left_len = char_len(&words[..*split].join(" "));
            let right_len = char_len(&words[*split..].join(" "));
            (
                left_len.saturating_sub(MAX_LINE_CHARS) + right_len.saturating_sub(MAX_LINE_CHARS),
                left_len.max(right_len),
                left_len.abs_diff(right_len),
                *split,
            )
        })
        .unwrap_or(1);
    vec![
        ellipsize(&words[..split].join(" ")),
        ellipsize(&words[split..].join(" ")),
    ]
}

pub(crate) fn apply_directed_visual_layout(
    nodes: &mut [RenderVisualNode],
    edges: &[RenderVisualEdge],
) {
    const MIN_X: i32 = 100;
    const MAX_X: i32 = 860;
    const PRIMARY_Y: u16 = 110;
    const SECONDARY_Y: u16 = 275;
    const SECONDARY_SPACING: i32 = 176;

    let node_index = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let mut layout_edges = edges
        .iter()
        .filter_map(|edge| {
            let from = node_index.get(edge.from.as_str()).copied()?;
            let to = node_index.get(edge.to.as_str()).copied()?;
            Some(VisualLayoutEdge {
                from,
                to,
                secondary: visual_layout_edge_is_secondary(edge, &nodes[from], &nodes[to]),
            })
        })
        .collect::<Vec<_>>();
    layout_edges.sort_by(|left, right| {
        (
            nodes[left.from].id.as_str(),
            nodes[left.to].id.as_str(),
            left.secondary,
        )
            .cmp(&(
                nodes[right.from].id.as_str(),
                nodes[right.to].id.as_str(),
                right.secondary,
            ))
    });
    if layout_edges.is_empty() {
        return;
    }

    let (topological_order, ranks) = directed_visual_order_and_ranks(nodes, &layout_edges);
    let topological_positions = topological_order
        .iter()
        .enumerate()
        .map(|(position, node)| (*node, position))
        .collect::<BTreeMap<_, _>>();
    let primary_eligible = nodes
        .iter()
        .map(|node| !visual_layout_node_is_auxiliary(node))
        .collect::<Vec<_>>();
    let mut primary_path = longest_directed_visual_path(
        nodes,
        &layout_edges,
        &topological_order,
        &topological_positions,
        &primary_eligible,
        true,
    )
    .unwrap_or_default();

    if primary_path.len() == 1 {
        primary_path = vec![
            most_connected_visual_node(nodes, &layout_edges, &primary_eligible)
                .unwrap_or(primary_path[0]),
        ];
    } else if primary_path.is_empty() {
        let all_nodes = vec![true; nodes.len()];
        primary_path = longest_directed_visual_path(
            nodes,
            &layout_edges,
            &topological_order,
            &topological_positions,
            &all_nodes,
            false,
        )
        .unwrap_or_default();
    }
    if primary_path.is_empty() {
        return;
    }

    let primary_set = primary_path.iter().copied().collect::<BTreeSet<_>>();
    let mut primary_x = BTreeMap::new();
    for (order, node_index) in primary_path.iter().copied().enumerate() {
        let x = if primary_path.len() == 1 {
            (MIN_X + MAX_X) / 2
        } else {
            MIN_X + ((MAX_X - MIN_X) * order as i32) / (primary_path.len() as i32 - 1)
        };
        primary_x.insert(node_index, x);
        nodes[node_index].layout = Some(RenderVisualNodeLayout {
            rank: ranks[node_index],
            lane: "primary",
            order,
            x: x as u16,
            y: PRIMARY_Y,
        });
    }

    let mut adjacency = vec![Vec::new(); nodes.len()];
    for edge in &layout_edges {
        adjacency[edge.from].push(edge.to);
        adjacency[edge.to].push(edge.from);
    }
    for neighbors in &mut adjacency {
        neighbors.sort_by(|left, right| nodes[*left].id.cmp(&nodes[*right].id));
        neighbors.dedup();
    }

    let mut groups = BTreeMap::<i32, Vec<usize>>::new();
    for node_index in 0..nodes.len() {
        if primary_set.contains(&node_index) {
            continue;
        }
        let anchor_x = closest_primary_anchor_x(node_index, &primary_x, &adjacency).unwrap_or(480);
        groups.entry(anchor_x).or_default().push(node_index);
    }

    let mut secondary = Vec::<(usize, i32)>::new();
    for (anchor_x, mut group) in groups {
        group.sort_by(|left, right| {
            (ranks[*left], nodes[*left].id.as_str())
                .cmp(&(ranks[*right], nodes[*right].id.as_str()))
        });
        let width = group.len() as i32 - 1;
        for (position, node_index) in group.into_iter().enumerate() {
            let offset = (position as i32 * 2 - width) * (SECONDARY_SPACING / 2);
            secondary.push((node_index, anchor_x + offset));
        }
    }
    secondary.sort_by(|(left_node, left_x), (right_node, right_x)| {
        (left_x, ranks[*left_node], nodes[*left_node].id.as_str()).cmp(&(
            right_x,
            ranks[*right_node],
            nodes[*right_node].id.as_str(),
        ))
    });

    let spacing = if secondary.len() <= 1 {
        0
    } else {
        SECONDARY_SPACING.min((MAX_X - MIN_X) / (secondary.len() as i32 - 1))
    };
    let mut secondary_x = secondary
        .iter()
        .map(|(_, desired_x)| (*desired_x).clamp(MIN_X, MAX_X))
        .collect::<Vec<_>>();
    for index in 1..secondary_x.len() {
        secondary_x[index] = secondary_x[index].max(secondary_x[index - 1] + spacing);
    }
    if secondary_x.last().copied().unwrap_or(MAX_X) > MAX_X {
        if let Some(last) = secondary_x.last_mut() {
            *last = MAX_X;
        }
        for index in (0..secondary_x.len().saturating_sub(1)).rev() {
            secondary_x[index] = secondary_x[index].min(secondary_x[index + 1] - spacing);
        }
    }
    if secondary_x.first().copied().unwrap_or(MIN_X) < MIN_X {
        if let Some(first) = secondary_x.first_mut() {
            *first = MIN_X;
        }
        for index in 1..secondary_x.len() {
            secondary_x[index] = secondary_x[index].max(secondary_x[index - 1] + spacing);
        }
    }

    for (order, ((node_index, _), x)) in secondary.into_iter().zip(secondary_x).enumerate() {
        nodes[node_index].layout = Some(RenderVisualNodeLayout {
            rank: ranks[node_index],
            lane: "secondary",
            order,
            x: x as u16,
            y: SECONDARY_Y,
        });
    }
}

pub(crate) fn visual_layout_node_is_auxiliary(node: &RenderVisualNode) -> bool {
    matches!(
        node.entity_type.as_str(),
        "source" | "claim" | "frontier_debate"
    )
}

pub(crate) fn visual_layout_edge_is_secondary(
    edge: &RenderVisualEdge,
    from: &RenderVisualNode,
    to: &RenderVisualNode,
) -> bool {
    let kind = edge.kind.trim().to_ascii_lowercase();
    visual_layout_node_is_auxiliary(from)
        || visual_layout_node_is_auxiliary(to)
        || kind.contains("qualif")
        || kind.contains("contradict")
}

pub(crate) fn directed_visual_order_and_ranks(
    nodes: &[RenderVisualNode],
    edges: &[VisualLayoutEdge],
) -> (Vec<usize>, Vec<usize>) {
    let mut outgoing = vec![Vec::new(); nodes.len()];
    let mut indegree = vec![0usize; nodes.len()];
    for edge in edges {
        outgoing[edge.from].push(edge.to);
        indegree[edge.to] += 1;
    }
    for neighbors in &mut outgoing {
        neighbors.sort_by(|left, right| nodes[*left].id.cmp(&nodes[*right].id));
    }

    let mut remaining = vec![true; nodes.len()];
    let mut ready = BTreeSet::<(String, usize)>::new();
    for (index, node) in nodes.iter().enumerate() {
        if indegree[index] == 0 {
            ready.insert((node.id.clone(), index));
        }
    }
    let mut order = Vec::with_capacity(nodes.len());
    let mut ranks = vec![0usize; nodes.len()];

    while order.len() < nodes.len() {
        let next = ready.iter().next().cloned().or_else(|| {
            nodes
                .iter()
                .enumerate()
                .filter(|(index, _)| remaining[*index])
                .min_by(|(_, left), (_, right)| left.id.cmp(&right.id))
                .map(|(index, node)| (node.id.clone(), index))
        });
        let Some((id, current)) = next else {
            break;
        };
        ready.remove(&(id, current));
        if !remaining[current] {
            continue;
        }
        remaining[current] = false;
        order.push(current);

        for &target in &outgoing[current] {
            if !remaining[target] {
                continue;
            }
            ranks[target] = ranks[target].max(ranks[current] + 1);
            indegree[target] = indegree[target].saturating_sub(1);
            if indegree[target] == 0 {
                ready.insert((nodes[target].id.clone(), target));
            }
        }
    }
    (order, ranks)
}

pub(crate) fn longest_directed_visual_path(
    nodes: &[RenderVisualNode],
    edges: &[VisualLayoutEdge],
    topological_order: &[usize],
    topological_positions: &BTreeMap<usize, usize>,
    eligible_nodes: &[bool],
    structural_only: bool,
) -> Option<Vec<usize>> {
    let mut best = vec![None::<VisualLayoutPath>; nodes.len()];
    for &current in topological_order {
        if !eligible_nodes[current] {
            continue;
        }
        let mut current_best = VisualLayoutPath {
            nodes: vec![current],
            secondary_edges: 0,
            auxiliary_nodes: usize::from(visual_layout_node_is_auxiliary(&nodes[current])),
        };
        for edge in edges.iter().filter(|edge| edge.to == current) {
            if structural_only && edge.secondary {
                continue;
            }
            if !eligible_nodes[edge.from]
                || topological_positions.get(&edge.from) >= topological_positions.get(&current)
            {
                continue;
            }
            let Some(prefix) = best[edge.from].as_ref() else {
                continue;
            };
            let mut candidate = prefix.clone();
            candidate.nodes.push(current);
            candidate.secondary_edges += usize::from(edge.secondary);
            candidate.auxiliary_nodes +=
                usize::from(visual_layout_node_is_auxiliary(&nodes[current]));
            if visual_layout_path_is_better(&candidate, &current_best, nodes) {
                current_best = candidate;
            }
        }
        best[current] = Some(current_best);
    }

    best.into_iter()
        .flatten()
        .reduce(|current, candidate| {
            if visual_layout_path_is_better(&candidate, &current, nodes) {
                candidate
            } else {
                current
            }
        })
        .map(|path| path.nodes)
}

pub(crate) fn visual_layout_path_is_better(
    candidate: &VisualLayoutPath,
    current: &VisualLayoutPath,
    nodes: &[RenderVisualNode],
) -> bool {
    if candidate.nodes.len() != current.nodes.len() {
        return candidate.nodes.len() > current.nodes.len();
    }
    if candidate.secondary_edges != current.secondary_edges {
        return candidate.secondary_edges < current.secondary_edges;
    }
    if candidate.auxiliary_nodes != current.auxiliary_nodes {
        return candidate.auxiliary_nodes < current.auxiliary_nodes;
    }
    candidate
        .nodes
        .iter()
        .map(|index| nodes[*index].id.as_str())
        .cmp(current.nodes.iter().map(|index| nodes[*index].id.as_str()))
        .is_lt()
}

pub(crate) fn most_connected_visual_node(
    nodes: &[RenderVisualNode],
    edges: &[VisualLayoutEdge],
    eligible_nodes: &[bool],
) -> Option<usize> {
    (0..nodes.len())
        .filter(|index| eligible_nodes[*index])
        .min_by(|left, right| {
            let left_degree = edges
                .iter()
                .filter(|edge| edge.from == *left || edge.to == *left)
                .count();
            let right_degree = edges
                .iter()
                .filter(|edge| edge.from == *right || edge.to == *right)
                .count();
            let left_indegree = edges.iter().filter(|edge| edge.to == *left).count();
            let right_indegree = edges.iter().filter(|edge| edge.to == *right).count();
            right_degree
                .cmp(&left_degree)
                .then_with(|| right_indegree.cmp(&left_indegree))
                .then_with(|| nodes[*left].id.cmp(&nodes[*right].id))
        })
}

pub(crate) fn closest_primary_anchor_x(
    start: usize,
    primary_x: &BTreeMap<usize, i32>,
    adjacency: &[Vec<usize>],
) -> Option<i32> {
    let mut queue = VecDeque::from([(start, 0usize)]);
    let mut visited = vec![false; adjacency.len()];
    visited[start] = true;
    let mut closest_distance = None;
    let mut anchors = Vec::new();

    while let Some((current, distance)) = queue.pop_front() {
        if closest_distance.is_some_and(|closest| distance > closest) {
            break;
        }
        if let Some(x) = primary_x.get(&current) {
            closest_distance = Some(distance);
            anchors.push(*x);
            continue;
        }
        for &neighbor in &adjacency[current] {
            if !visited[neighbor] {
                visited[neighbor] = true;
                queue.push_back((neighbor, distance + 1));
            }
        }
    }
    (!anchors.is_empty()).then(|| anchors.iter().sum::<i32>() / anchors.len() as i32)
}
pub(crate) fn visual_alt_text(
    title: &str,
    kind: &str,
    nodes: &[RenderVisualNode],
    edges: &[RenderVisualEdge],
) -> String {
    let edge_summary = edges
        .iter()
        .map(|edge| {
            format!(
                "{} to {} ({})",
                edge.from_label,
                edge.to_label,
                first_non_empty([edge.label.as_str(), edge.kind.as_str(), "related"])
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "{title} is a {kind} view with {} nodes and {} edges: {edge_summary}",
        nodes.len(),
        edges.len()
    )
}

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
    html.push_str("<p class=\"muted\">Machine-verifiable support for the narrative above. It is kept separate so the evidence contract does not dictate the report's reading order.</p>\n");
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

pub(crate) fn push_nav(html: &mut String, report: &PublicReport, has_visuals: bool) {
    let mut items = vec![
        ("reading-guide", "Reading Guide"),
        ("curriculum-path", "Curriculum Path"),
        ("reading-ladder", "Reading Ladder"),
        ("claims", "Claim Evidence"),
        ("frontier-debates", "Frontier Guidance"),
        ("scope", "Scope"),
        ("domain-profile", "Domain Profile"),
        ("evidence-standards", "Evidence Standards"),
        ("sources-evidence", "Source Catalog"),
        ("relations", "Relation Audit"),
    ];
    let structure_index = items
        .iter()
        .position(|(id, _)| *id == "evidence-standards")
        .unwrap_or(items.len());
    if report.field_elements.is_empty() {
        for item in [
            ("representations", "Representations"),
            ("methods", "Methods"),
            ("core-ideas", "Core Ideas"),
        ] {
            items.insert(structure_index, item);
        }
    } else {
        items.insert(structure_index, ("field-elements", "Field Elements"));
    }
    if has_visuals {
        items.insert(1, ("visualizations", "Knowledge Map"));
    }
    html.push_str("<nav class=\"section-nav\" aria-label=\"Report sections\">\n");
    for (id, label) in items {
        html.push_str("<a href=\"#");
        push_escaped_attr(html, id);
        html.push_str("\">");
        push_escaped(html, label);
        html.push_str("</a>\n");
    }
    html.push_str("</nav>\n");
}

pub(crate) fn push_reading_guide_section(
    html: &mut String,
    document: &ReportDocument,
    has_visuals: bool,
) {
    let report = &document.report;
    push_section_open(html, "reading-guide", "Reading Guide");
    push_paragraph(html, &report.scope.summary);
    html.push_str("<ol class=\"guide-list\">\n");
    if has_visuals {
        push_guide_item(
            html,
            "Orient with the knowledge map",
            "Use the relation-backed visual view first; every drawn edge comes from a validated public relation.",
        );
    }
    push_guide_item(
        html,
        "Follow the curriculum path",
        "Read the steps in sequence and check prerequisites before moving to each new layer.",
    );
    push_guide_item(
        html,
        "Use the reading ladder",
        "Start from the layer guidance and read sources for the stated purpose, including the explicit do-not-infer caveats.",
    );
    push_guide_item(
        html,
        "Check claims against evidence",
        "Read each claim with its supporting, qualifying, and contradictory evidence before treating it as settled.",
    );
    if !report.frontier_debates.is_empty() {
        push_guide_item(
            html,
            "Handle frontier guidance last",
            "Treat current or debate-facing material as time-bound and review the stated background and source support.",
        );
    }
    html.push_str("</ol>\n");
    html.push_str("<p class=\"meta\"><strong>Report review:</strong> ");
    push_escaped(
        html,
        &format_temporal_inline(&document.metadata.temporal_review),
    );
    html.push_str("</p>\n</section>\n");
}

pub(crate) fn push_guide_item(html: &mut String, title: &str, body: &str) {
    html.push_str("<li><strong>");
    push_escaped(html, title);
    html.push_str("</strong><span>");
    push_escaped(html, body);
    html.push_str("</span></li>\n");
}

pub(crate) fn push_scope_section(html: &mut String, scope: &Scope) {
    push_section_open(html, "scope", "Scope");
    push_paragraph(html, &scope.summary);
    html.push_str("<div class=\"split-list\">\n");
    push_string_list(html, "Included", &scope.included);
    push_string_list(html, "Excluded", &scope.excluded);
    push_string_list(html, "Assumptions", &scope.assumptions);
    push_string_list(html, "Interpretive Notes", &scope.interpretive_notes);
    html.push_str("</div>\n</section>\n");
}

pub(crate) fn push_domain_profile_section(html: &mut String, profile: &DomainProfile) {
    push_section_open(html, "domain-profile", "Domain Profile");
    let mut classifications = vec![domain_classification_label(profile.classification).to_string()];
    classifications.extend(
        profile
            .secondary_characteristics
            .iter()
            .map(|classification| domain_classification_label(*classification).to_string()),
    );
    html.push_str("<p class=\"meta\"><strong>Classification:</strong> ");
    push_escaped(html, &classifications.join(", "));
    html.push_str("</p>\n");
    push_paragraph(html, &profile.rationale);
    push_string_list(html, "Failure Modes", &profile.failure_modes);
    html.push_str("</section>\n");
}

pub(crate) fn push_field_elements_or_legacy(
    html: &mut String,
    report: &PublicReport,
    source_labels: &BTreeMap<String, String>,
) {
    if report.field_elements.is_empty() {
        push_knowledge_section(
            html,
            "core-ideas",
            "Core Ideas",
            &report.core_ideas,
            source_labels,
        );
        push_knowledge_section(html, "methods", "Methods", &report.methods, source_labels);
        push_knowledge_section(
            html,
            "representations",
            "Representations",
            &report.representations,
            source_labels,
        );
    } else {
        push_field_elements_section(html, &report.field_elements, source_labels);
    }
}

pub(crate) fn push_field_elements_section(
    html: &mut String,
    elements: &[FieldElement],
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "field-elements", "Field Elements");
    html.push_str("<div class=\"item-grid\">\n");
    for element in elements {
        html.push_str("<article class=\"item-card\">\n<h3>");
        push_escaped(html, &element.label);
        html.push_str("</h3>\n<div class=\"badge-row\">");
        push_badge(html, &element.element_class);
        push_badge(html, &element.role);
        if let Some(confidence) = element.confidence {
            push_badge(html, claim_confidence_label(confidence));
        }
        html.push_str("</div>\n<dl class=\"compact-dl\">\n");
        push_dl_item(html, "Actual form", &element.actual_form);
        push_dl_item(
            html,
            "Load-bearing relations",
            &element.load_bearing_relations,
        );
        html.push_str("</dl>\n");
        push_source_reference_list(html, "Sources", &element.source_ids, source_labels);
        push_raw_details(
            html,
            "Raw field element identifier",
            &[
                ("Element ID", element.id.clone()),
                ("Source IDs", element.source_ids.join(", ")),
            ],
        );
        html.push_str("</article>\n");
    }
    html.push_str("</div>\n</section>\n");
}

pub(crate) fn push_knowledge_section(
    html: &mut String,
    id: &str,
    title: &str,
    items: &[KnowledgeItem],
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, id, title);
    if items.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"item-grid\">\n");
        for item in items {
            html.push_str("<article class=\"item-card\">\n<h3>");
            push_escaped(html, &item.label);
            html.push_str("</h3>\n");
            push_paragraph(html, &item.description);
            push_nested_string_list(html, "Aliases", &item.aliases);
            push_source_reference_list(html, "Sources", &item.source_ids, source_labels);
            push_raw_details(
                html,
                "Raw item identifiers",
                &[
                    ("Item ID", item.id.clone()),
                    ("Source IDs", item.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_evidence_standards_section(html: &mut String, standards: &EvidenceStandards) {
    push_section_open(html, "evidence-standards", "Evidence Standards");
    push_paragraph(html, &standards.summary);
    push_paragraph(html, &standards.claim_policy);
    let rows = standards
        .source_role_requirements
        .iter()
        .map(|requirement| {
            vec![
                source_role_label(requirement.role).to_string(),
                source_role_requirement_label(requirement.requirement).to_string(),
                requirement
                    .minimum_sources
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                requirement.rationale.clone(),
                requirement
                    .waiver
                    .as_ref()
                    .map(format_source_role_waiver)
                    .unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    push_table(
        html,
        &["Role", "Requirement", "Minimum", "Rationale", "Waiver"],
        &rows,
    );
    html.push_str("</section>\n");
}

pub(crate) fn push_sources_and_evidence_section(html: &mut String, report: &PublicReport) {
    push_section_open(html, "sources-evidence", "Source Catalog");
    if report.sources.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"source-grid\">\n");
        for source in &report.sources {
            html.push_str("<article class=\"source-card\">\n<h3>");
            push_escaped(html, &source_display_label(source));
            html.push_str("</h3>\n<div class=\"badge-row\">");
            push_badge(html, &source.source_type);
            push_badge(html, verification_status_label(source.verification_status));
            for role in &source.roles {
                push_badge(html, source_role_label(*role));
            }
            html.push_str("</div>\n");
            push_paragraph(html, &source.why_it_matters);
            html.push_str("<dl class=\"compact-dl\">\n");
            push_dl_item(html, "Roles", &format_source_roles(&source.roles));
            push_dl_item(html, "Access", &format_source_access(&source.access));
            push_dl_item(html, "Identifier", &source.identifier);
            push_dl_item(html, "Date", &source.date);
            push_dl_item(html, "Last reviewed", &source.last_reviewed);
            push_dl_item(html, "Notes", &source.notes);
            html.push_str("</dl>\n");
            push_raw_details(
                html,
                "Raw source fields",
                &[
                    ("Source ID", source.id.clone()),
                    ("Citation", source.citation.clone()),
                    ("URL", source.url.clone()),
                ],
            );
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_claims_section(
    html: &mut String,
    claims: &[Claim],
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "claims", "Claim Evidence Guide");
    if claims.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"claim-grid\">\n");
        for claim in claims {
            html.push_str("<article class=\"claim-card\">\n<h3>");
            push_escaped(html, &claim.statement);
            html.push_str("</h3>\n<div class=\"badge-row\">");
            push_badge(html, claim_type_label(claim.claim_type));
            push_badge(html, evidence_requirement_label(claim.evidence_requirement));
            push_badge(
                html,
                claim
                    .confidence
                    .map(claim_confidence_label)
                    .unwrap_or("unknown"),
            );
            push_badge(html, temporal_status_label(claim.temporal.temporal_status));
            html.push_str("</div>\n");
            html.push_str("<p class=\"meta\"><strong>Temporal status:</strong> ");
            push_escaped(html, &format_temporal_inline(&claim.temporal));
            html.push_str("</p>\n");
            push_evidence_group(
                html,
                "Supporting Evidence",
                &claim.evidence_links,
                source_labels,
                |kind| kind == SupportKind::Supports,
            );
            push_evidence_group(
                html,
                "Qualifying Evidence",
                &claim.evidence_links,
                source_labels,
                |kind| kind == SupportKind::Qualifies,
            );
            push_evidence_group(
                html,
                "Contradictory Evidence",
                &claim.evidence_links,
                source_labels,
                |kind| kind == SupportKind::Contradicts,
            );
            push_evidence_group(
                html,
                "Context Evidence",
                &claim.evidence_links,
                source_labels,
                |kind| matches!(kind, SupportKind::Background | SupportKind::Example),
            );
            push_paragraph(html, &claim.notes);
            push_claim_raw_details(html, claim);
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_curriculum_section(
    html: &mut String,
    steps: &[CurriculumStep],
    entity_labels: &BTreeMap<String, String>,
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "curriculum-path", "Curriculum Path");
    if steps.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<ol class=\"curriculum-path\" aria-label=\"Visual curriculum path\">\n");
        for step in steps {
            html.push_str(
                "<li class=\"path-step\">\n<div class=\"path-marker\" aria-hidden=\"true\">",
            );
            push_escaped(html, &step.sequence.to_string());
            html.push_str("</div>\n<article class=\"path-card\">\n<h3>");
            push_escaped(html, &step.title);
            html.push_str("</h3>\n");
            push_paragraph(html, &step.learning_goal);
            if step.prerequisite_ids.is_empty() {
                html.push_str(
                    "<p class=\"meta\"><strong>Prerequisites:</strong> Entry point</p>\n",
                );
            } else {
                html.push_str("<p class=\"meta\"><strong>Prerequisites:</strong> ");
                push_escaped(html, &labels_for_ids(&step.prerequisite_ids, entity_labels));
                html.push_str("</p>\n");
            }
            push_paragraph(html, &step.practice_artifact);
            push_nested_string_list(html, "Progress Criteria", &step.progress_criteria);
            push_source_reference_list(html, "Sources", &step.source_ids, source_labels);
            push_raw_details(
                html,
                "Raw curriculum identifiers",
                &[
                    ("Step ID", step.id.clone()),
                    ("Prerequisite IDs", step.prerequisite_ids.join(", ")),
                    ("Source IDs", step.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n</li>\n");
        }
        html.push_str("</ol>\n");
        html.push_str("<div class=\"text-fallback\" role=\"group\" aria-label=\"Text alternative for curriculum path\">\n<h3>Text Alternative</h3>\n<ol>\n");
        for step in steps {
            html.push_str("<li>");
            push_escaped(html, &step.title);
            if step.prerequisite_ids.is_empty() {
                html.push_str(": entry point.");
            } else {
                html.push_str(": follows ");
                push_escaped(html, &labels_for_ids(&step.prerequisite_ids, entity_labels));
                html.push('.');
            }
            html.push_str("</li>\n");
        }
        html.push_str("</ol>\n</div>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_frontier_section(
    html: &mut String,
    report: &PublicReport,
    entity_labels: &BTreeMap<String, String>,
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "frontier-debates", "Frontier Guidance");
    if report.frontier_debates.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"frontier-list\">\n");
        for item in &report.frontier_debates {
            html.push_str("<article class=\"frontier-card\">\n<h3>");
            push_escaped(html, &item.title);
            html.push_str("</h3>\n<div class=\"badge-row\">");
            push_badge(html, frontier_debate_kind_label(item.kind));
            push_badge(html, temporal_status_label(item.temporal.temporal_status));
            html.push_str("</div>\n");
            push_paragraph(html, &item.summary);
            push_paragraph(html, &item.why_it_matters);
            push_entity_reference_list(
                html,
                "Required Background",
                &item.required_background_ids,
                entity_labels,
            );
            push_entity_reference_list(html, "Related Claims", &item.claim_ids, entity_labels);
            push_source_reference_list(html, "Sources", &item.source_ids, source_labels);
            html.push_str("<p class=\"meta\"><strong>Temporal status:</strong> ");
            push_escaped(html, &format_temporal_inline(&item.temporal));
            html.push_str("</p>\n");
            push_raw_details(
                html,
                "Raw frontier identifiers",
                &[
                    ("Frontier ID", item.id.clone()),
                    ("Background IDs", item.required_background_ids.join(", ")),
                    ("Claim IDs", item.claim_ids.join(", ")),
                    ("Source IDs", item.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_reading_ladder_section(
    html: &mut String,
    report: &PublicReport,
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "reading-ladder", "Reading Ladder");
    if report.literature_ladder.is_empty() {
        push_source_ladder(html, &report.sources);
    } else {
        html.push_str("<ol class=\"ladder-list\">\n");
        for row in &report.literature_ladder {
            html.push_str("<li class=\"ladder-step\">\n<article class=\"ladder-card\">\n<h3>");
            push_escaped(html, &row.layer);
            html.push_str("</h3>\n<dl class=\"reader-dl\">\n");
            push_dl_item(html, "Start point", &row.start_here);
            push_dl_item(html, "Read for", &row.read_for);
            push_dl_item(html, "Do not infer", &row.do_not_infer);
            push_dl_item(html, "Notes", &row.notes);
            html.push_str("</dl>\n");
            push_source_reference_list(html, "Sources", &row.source_ids, source_labels);
            push_raw_details(
                html,
                "Raw ladder identifiers",
                &[
                    ("Ladder row ID", row.id.clone()),
                    ("Source IDs", row.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n</li>\n");
        }
        html.push_str("</ol>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_source_ladder(html: &mut String, sources: &[ReportSource]) {
    if sources.is_empty() {
        push_empty_note(html);
        return;
    }
    html.push_str("<p class=\"prose\">No explicit literature ladder is present; use the source metadata below as the public source ladder.</p>\n");
    html.push_str("<ol class=\"ladder-list\">\n");
    for source in sources {
        let start_point = source_display_label(source);
        html.push_str("<li class=\"ladder-step\">\n<article class=\"ladder-card\">\n<h3>");
        push_escaped(html, &start_point);
        html.push_str("</h3>\n<div class=\"badge-row\">");
        for role in &source.roles {
            push_badge(html, source_role_label(*role));
        }
        push_badge(html, verification_status_label(source.verification_status));
        html.push_str("</div>\n<dl class=\"reader-dl\">\n");
        push_dl_item(html, "Start point", &start_point);
        push_dl_item(html, "Read for", &source.why_it_matters);
        push_dl_item(html, "Do not infer", &source.notes);
        html.push_str("</dl>\n");
        push_raw_details(
            html,
            "Raw source identifiers",
            &[
                ("Source ID", source.id.clone()),
                ("Identifier", source.identifier.clone()),
                ("URL", source.url.clone()),
            ],
        );
        html.push_str("</article>\n</li>\n");
    }
    html.push_str("</ol>\n");
}

pub(crate) fn push_relations_section(
    html: &mut String,
    relations: &[Relation],
    entity_labels: &BTreeMap<String, String>,
    source_labels: &BTreeMap<String, String>,
) {
    push_section_open(html, "relations", "Relation Audit");
    if relations.is_empty() {
        push_empty_note(html);
    } else {
        html.push_str("<div class=\"relation-list\">\n");
        for relation in relations {
            let from = label_for_id(&relation.from.id, entity_labels);
            let to = label_for_id(&relation.to.id, entity_labels);
            html.push_str("<article class=\"relation-card\">\n<h3>");
            push_escaped(html, &from);
            html.push_str(" -> ");
            push_escaped(html, &to);
            html.push_str("</h3>\n<div class=\"badge-row\">");
            push_badge(html, relation_kind_label(relation.kind));
            push_badge(html, entity_type_label(relation.from.entity_type));
            push_badge(html, entity_type_label(relation.to.entity_type));
            html.push_str("</div>\n");
            push_paragraph(html, &relation.description);
            push_source_reference_list(html, "Sources", &relation.source_ids, source_labels);
            push_raw_details(
                html,
                "Raw relation identifiers",
                &[
                    ("Relation ID", relation.id.clone()),
                    ("From", relation.from.id.clone()),
                    ("To", relation.to.id.clone()),
                    ("Source IDs", relation.source_ids.join(", ")),
                ],
            );
            html.push_str("</article>\n");
        }
        html.push_str("</div>\n");
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_visual_section(html: &mut String, views: &[RenderVisualView]) {
    push_section_open(html, "visualizations", "Knowledge Map");
    for view in views {
        push_visual_card(html, view);
    }
    html.push_str("</section>\n");
}

pub(crate) fn push_visual_card(html: &mut String, view: &RenderVisualView) {
    html.push_str("<article class=\"visual-card\" aria-labelledby=\"visual-title-");
    push_escaped_attr(html, &view.id);
    html.push_str("\">\n<h3 id=\"visual-title-");
    push_escaped_attr(html, &view.id);
    html.push_str("\">");
    push_escaped(html, &view.title);
    html.push_str("</h3>\n<p class=\"meta\">");
    push_escaped(html, &view.justification);
    html.push_str("</p>\n<div class=\"visual-canvas\" data-visual-mount=\"");
    push_escaped_attr(html, &view.id);
    html.push_str("\" role=\"region\" aria-label=\"Interactive visual view: ");
    push_escaped_attr(html, &view.title);
    html.push_str("\"></div>\n");
    html.push_str("<noscript><p class=\"muted\">The text alternative below contains the same graph nodes and edges.</p></noscript>\n");
    html.push_str(
        "<div class=\"visual-fallback\" role=\"group\" aria-label=\"Text alternative for ",
    );
    push_escaped_attr(html, &view.title);
    html.push_str("\">\n<p>");
    push_escaped(html, &view.alt);
    html.push_str("</p>\n<h4>Nodes</h4>\n<ul>\n");
    for node in &view.nodes {
        html.push_str("<li><strong>");
        push_escaped(html, &node.label);
        html.push_str("</strong> ");
        push_escaped(html, &format!("({})", node.entity_type));
        if !node.description.trim().is_empty() {
            html.push_str(": ");
            push_escaped(html, &node.description);
        }
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n<h4>Edges</h4>\n<ul>\n");
    for edge in &view.edges {
        html.push_str("<li>");
        push_escaped(
            html,
            &format!(
                "{} -> {} ({})",
                edge.from_label,
                edge.to_label,
                first_non_empty([edge.label.as_str(), edge.kind.as_str(), "related"])
            ),
        );
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n");
    push_raw_details(
        html,
        "Raw visual identifiers",
        &[
            ("Visual view ID", view.id.clone()),
            (
                "Relation IDs",
                view.edges
                    .iter()
                    .map(|edge| edge.relation_id.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        ],
    );
    html.push_str("</div>\n</article>\n");
}

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
    html.push_str("<div class=\"table-scroll\" role=\"region\" aria-label=\"Scrollable table\" tabindex=\"0\">\n<table>\n<thead>\n<tr>");
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
