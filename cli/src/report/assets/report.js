(function () {
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
})();