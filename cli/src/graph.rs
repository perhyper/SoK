//! Deterministic cycle detection for prerequisite graphs.

use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn directed_cycle(
    edges: impl IntoIterator<Item = (String, String)>,
) -> Option<Vec<String>> {
    let mut graph = BTreeMap::<String, BTreeSet<String>>::new();
    for (from, to) in edges {
        graph.entry(from).or_default().insert(to.clone());
        graph.entry(to).or_default();
    }

    let mut visited = BTreeSet::new();
    let mut stack = Vec::<String>::new();
    let mut in_stack = BTreeSet::new();
    for node in graph.keys() {
        if visited.contains(node) {
            continue;
        }
        if let Some(cycle) =
            hard_prerequisite_cycle_from(node, &graph, &mut visited, &mut stack, &mut in_stack)
        {
            return Some(cycle);
        }
    }
    None
}

fn hard_prerequisite_cycle_from(
    node: &str,
    graph: &BTreeMap<String, BTreeSet<String>>,
    visited: &mut BTreeSet<String>,
    stack: &mut Vec<String>,
    in_stack: &mut BTreeSet<String>,
) -> Option<Vec<String>> {
    visited.insert(node.to_string());
    stack.push(node.to_string());
    in_stack.insert(node.to_string());

    if let Some(next_nodes) = graph.get(node) {
        for next in next_nodes {
            if !visited.contains(next.as_str()) {
                if let Some(cycle) =
                    hard_prerequisite_cycle_from(next, graph, visited, stack, in_stack)
                {
                    return Some(cycle);
                }
            } else if in_stack.contains(next.as_str()) {
                if let Some(position) = stack.iter().position(|item| item == next) {
                    let mut cycle = stack[position..].to_vec();
                    cycle.push(next.clone());
                    return Some(cycle);
                }
            }
        }
    }

    in_stack.remove(node);
    stack.pop();
    None
}
