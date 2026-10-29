use super::*;

#[derive(Debug, Serialize)]
pub(crate) struct RenderVisualView {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) title: String,
    pub(super) justification: String,
    pub(super) alt: String,
    pub(super) nodes: Vec<RenderVisualNode>,
    pub(super) edges: Vec<RenderVisualEdge>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RenderVisualNode {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) label_lines: Vec<String>,
    pub(super) entity_type: String,
    pub(super) description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) layout: Option<RenderVisualNodeLayout>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RenderVisualNodeLayout {
    pub(super) rank: usize,
    pub(super) lane: &'static str,
    pub(super) order: usize,
    pub(super) x: u16,
    pub(super) y: u16,
}

#[derive(Debug, Serialize)]
pub(crate) struct RenderVisualEdge {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) kind: String,
    pub(super) label: String,
    pub(super) relation_id: String,
    pub(super) from_label: String,
    pub(super) to_label: String,
}

#[derive(Debug, Clone)]
pub(crate) struct VisualLayoutEdge {
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) secondary: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct VisualLayoutPath {
    pub(super) nodes: Vec<usize>,
    pub(super) secondary_edges: usize,
    pub(super) auxiliary_nodes: usize,
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
