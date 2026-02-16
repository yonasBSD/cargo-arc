//! Topological sorting algorithms for layout ordering.

use petgraph::algo::tarjan_scc;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

/// SCC condensation fallback for stable_toposort when cycles exist.
/// Applies ADR-017 pattern: condense → toposort → expand alphabetically.
fn scc_fallback_sort(
    graph: &DiGraph<NodeIndex, usize>,
    node_to_orig: &HashMap<petgraph::graph::NodeIndex, NodeIndex>,
    get_name: &dyn Fn(NodeIndex) -> String,
) -> Vec<NodeIndex> {
    use std::collections::BinaryHeap;

    let sccs = tarjan_scc(graph);

    // Build condensed DAG: each SCC becomes one node
    let mut condensed: DiGraph<Vec<petgraph::graph::NodeIndex>, ()> = DiGraph::new();
    let mut node_to_scc: HashMap<petgraph::graph::NodeIndex, petgraph::graph::NodeIndex> =
        HashMap::new();

    for scc in &sccs {
        let scc_idx = condensed.add_node(scc.clone());
        for &node in scc {
            node_to_scc.insert(node, scc_idx);
        }
    }

    // Transfer edges between SCCs (skip intra-SCC)
    for edge in graph.edge_references() {
        let src_scc = node_to_scc[&edge.source()];
        let dst_scc = node_to_scc[&edge.target()];
        if src_scc != dst_scc && !condensed.contains_edge(src_scc, dst_scc) {
            condensed.add_edge(src_scc, dst_scc, ());
        }
    }

    // Stable Kahn's toposort on condensed DAG with alphabetical tiebreaker
    let scc_order = {
        let mut in_deg: HashMap<petgraph::graph::NodeIndex, usize> = HashMap::new();
        for n in condensed.node_indices() {
            in_deg.insert(n, 0);
        }
        for e in condensed.edge_references() {
            *in_deg.get_mut(&e.target()).unwrap() += 1;
        }

        // Sort name = alphabetically first member
        let scc_sort_name = |scc_idx: petgraph::graph::NodeIndex| -> String {
            condensed[scc_idx]
                .iter()
                .map(|&n| get_name(node_to_orig[&n]))
                .min()
                .unwrap_or_default()
        };

        // Min-heap by name
        let mut heap: BinaryHeap<std::cmp::Reverse<(String, petgraph::graph::NodeIndex)>> =
            BinaryHeap::new();
        for (&n, &deg) in &in_deg {
            if deg == 0 {
                heap.push(std::cmp::Reverse((scc_sort_name(n), n)));
            }
        }

        let mut order = Vec::new();
        while let Some(std::cmp::Reverse((_, n))) = heap.pop() {
            order.push(n);
            for neighbor in condensed.neighbors(n) {
                let deg = in_deg.get_mut(&neighbor).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    heap.push(std::cmp::Reverse((scc_sort_name(neighbor), neighbor)));
                }
            }
        }
        order
    };

    // Log cycles (SCCs with >1 member)
    for &scc_idx in &scc_order {
        let members = &condensed[scc_idx];
        if members.len() > 1 {
            let names: Vec<String> = members
                .iter()
                .map(|&n| get_name(node_to_orig[&n]))
                .collect();
            tracing::debug!("SCC cycle ({} nodes): {}", members.len(), names.join(", "));
        }
    }

    // Expand: multi-member SCCs via optimal_scc_order, singletons directly
    scc_order
        .into_iter()
        .flat_map(|scc_idx| {
            let members = &condensed[scc_idx];
            if members.len() > 1 {
                optimal_scc_order(members, graph, node_to_orig, get_name)
            } else {
                vec![node_to_orig[&members[0]]]
            }
        })
        .collect()
}

/// Brute-force minimum-upward-permutation for SCC members.
/// Enumerates all permutations and picks the one with the lowest sum of
/// "upward" edge weights (edges from later → earlier in the permutation).
/// Falls back to alphabetical for n > 8 (40320 permutations limit).
fn optimal_scc_order(
    members: &[petgraph::graph::NodeIndex],
    graph: &DiGraph<NodeIndex, usize>,
    node_to_orig: &HashMap<petgraph::graph::NodeIndex, NodeIndex>,
    get_name: &dyn Fn(NodeIndex) -> String,
) -> Vec<NodeIndex> {
    use itertools::Itertools;

    let n = members.len();

    // Guard: too many permutations → fallback alphabetical
    if n > 8 {
        let mut result: Vec<NodeIndex> = members.iter().map(|&m| node_to_orig[&m]).collect();
        result.sort_by_key(|&idx| get_name(idx));
        return result;
    }

    // Extract pairwise weights: weights[i][j] = edge weight from members[i] to members[j]
    let mut weights = vec![vec![0usize; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i != j
                && let Some(edge_idx) = graph.find_edge(members[i], members[j])
            {
                weights[i][j] = graph[edge_idx];
            }
        }
    }

    // Enumerate all permutations, score by upward weight sum
    let mut best_perm: Vec<usize> = (0..n).collect();
    let mut best_score = usize::MAX;
    let mut best_names: Vec<String> = Vec::new();

    for perm in (0..n).permutations(n) {
        // Score: sum of weights[perm[later]][perm[earlier]] for all later > earlier
        let mut score = 0usize;
        for later in 0..n {
            for earlier in 0..later {
                score += weights[perm[later]][perm[earlier]];
            }
        }

        if score < best_score
            || (score == best_score && {
                let names: Vec<String> = perm
                    .iter()
                    .map(|&i| get_name(node_to_orig[&members[i]]))
                    .collect();
                names < best_names
            })
        {
            best_score = score;
            best_names = perm
                .iter()
                .map(|&i| get_name(node_to_orig[&members[i]]))
                .collect();
            best_perm = perm;
        }
    }

    best_perm
        .iter()
        .map(|&i| node_to_orig[&members[i]])
        .collect()
}

/// Stable topological sort using Kahn's algorithm.
/// Preserves alphabetical order for nodes without dependency relationships.
pub(super) fn stable_toposort(
    sibling_deps: &DiGraph<NodeIndex, usize>,
    children: &[NodeIndex],
    get_name: impl Fn(NodeIndex) -> String,
) -> Vec<NodeIndex> {
    use std::collections::BinaryHeap;

    if children.is_empty() {
        return vec![];
    }

    // Map sibling_deps node indices to original NodeIndex
    let node_to_orig: HashMap<petgraph::graph::NodeIndex, NodeIndex> = sibling_deps
        .node_indices()
        .map(|n| (n, sibling_deps[n]))
        .collect();

    // Compute in-degrees
    let mut in_degree: HashMap<petgraph::graph::NodeIndex, usize> = HashMap::new();
    for n in sibling_deps.node_indices() {
        in_degree.insert(n, 0);
    }
    for edge in sibling_deps.edge_references() {
        *in_degree.get_mut(&edge.target()).unwrap() += 1;
    }

    // Barycenter score: average position of already-placed dependents.
    // Nodes closer to their dependents get lower scores → fewer arc crossings.
    let barycenter = |n: petgraph::graph::NodeIndex,
                      positions: &HashMap<petgraph::graph::NodeIndex, usize>|
     -> f64 {
        let placed: Vec<usize> = sibling_deps
            .neighbors_directed(n, petgraph::Direction::Incoming)
            .filter_map(|pred| positions.get(&pred).copied())
            .collect();
        if placed.is_empty() {
            f64::INFINITY
        } else {
            placed.iter().sum::<usize>() as f64 / placed.len() as f64
        }
    };

    // Use BinaryHeap with barycenter score as primary key, name as tiebreaker
    struct Item(f64, String, petgraph::graph::NodeIndex);
    impl PartialEq for Item {
        fn eq(&self, other: &Self) -> bool {
            self.0.to_bits() == other.0.to_bits() && self.1 == other.1
        }
    }
    impl Eq for Item {}
    impl Ord for Item {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            // Reversed for min-heap: lower score = higher priority
            other
                .0
                .partial_cmp(&self.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| other.1.cmp(&self.1))
        }
    }
    impl PartialOrd for Item {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    // Position tracking: maps sibling_deps node index → placement order in result
    let mut positions: HashMap<petgraph::graph::NodeIndex, usize> = HashMap::new();

    // Initialize with nodes having in-degree 0 (all INFINITY — nothing placed yet)
    let mut heap: BinaryHeap<Item> = BinaryHeap::new();
    for (&n, &deg) in &in_degree {
        if deg == 0 {
            let orig = node_to_orig[&n];
            heap.push(Item(f64::INFINITY, get_name(orig), n));
        }
    }

    let mut result = Vec::new();
    while let Some(Item(_, _, n)) = heap.pop() {
        positions.insert(n, result.len());
        result.push(node_to_orig[&n]);

        // Decrease in-degree of neighbors
        for neighbor in sibling_deps.neighbors(n) {
            let deg = in_degree.get_mut(&neighbor).unwrap();
            *deg -= 1;
            if *deg == 0 {
                let orig = node_to_orig[&neighbor];
                let score = barycenter(neighbor, &positions);
                heap.push(Item(score, get_name(orig), neighbor));
            }
        }
    }

    // If not all nodes processed, there's a cycle — use SCC fallback
    if result.len() != children.len() {
        return scc_fallback_sort(sibling_deps, &node_to_orig, &get_name);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{ArcGraph, Node};
    use petgraph::graph::NodeIndex;
    use std::collections::HashMap;

    // === stable_toposort Cycle Tests ===

    #[test]
    fn test_stable_toposort_with_cycle_returns_sorted() {
        use std::path::PathBuf;

        // Build a graph with a cycle (A⇄B) — stable_toposort should return
        // non-empty result via SCC fallback instead of vec![]
        let mut graph = ArcGraph::new();
        let crate_idx = graph.add_node(Node::Crate {
            name: "test".to_string(),
            path: PathBuf::from("/test"),
        });
        let a = graph.add_node(Node::Module {
            name: "a".to_string(),
            crate_idx,
        });
        let b = graph.add_node(Node::Module {
            name: "b".to_string(),
            crate_idx,
        });

        // Build sibling_deps DiGraph with cycle
        let mut sibling_deps: DiGraph<NodeIndex, usize> = DiGraph::new();
        let sa = sibling_deps.add_node(a);
        let sb = sibling_deps.add_node(b);
        sibling_deps.add_edge(sa, sb, 1);
        sibling_deps.add_edge(sb, sa, 1); // cycle

        let children = vec![a, b];
        let result = stable_toposort(&sibling_deps, &children, |idx| match &graph[idx] {
            Node::Module { name, .. } => name.clone(),
            Node::Crate { name, .. } => name.clone(),
        });

        assert_eq!(result.len(), 2, "Should return all nodes, not empty vec");
        // Both nodes present
        assert!(result.contains(&a), "Should contain a");
        assert!(result.contains(&b), "Should contain b");
        // Alphabetical within SCC
        let pos_a = result.iter().position(|&n| n == a).unwrap();
        let pos_b = result.iter().position(|&n| n == b).unwrap();
        assert!(pos_a < pos_b, "a before b (alphabetical in SCC)");
    }

    // === SCC Fallback Tests ===

    #[test]
    fn test_scc_fallback_three_node_cycle() {
        // Pure cycle: A→B→C→A — all nodes in one SCC
        // Expected: alphabetical order [A, B, C]
        let mut graph: DiGraph<NodeIndex, usize> = DiGraph::new();
        let a_orig = NodeIndex::new(10);
        let b_orig = NodeIndex::new(11);
        let c_orig = NodeIndex::new(12);
        let a = graph.add_node(a_orig);
        let b = graph.add_node(b_orig);
        let c = graph.add_node(c_orig);
        graph.add_edge(a, b, 1);
        graph.add_edge(b, c, 1);
        graph.add_edge(c, a, 1);

        let node_to_orig: HashMap<petgraph::graph::NodeIndex, NodeIndex> =
            graph.node_indices().map(|n| (n, graph[n])).collect();

        let get_name = |idx: NodeIndex| -> String {
            match idx.index() {
                10 => "a".to_string(),
                11 => "b".to_string(),
                12 => "c".to_string(),
                _ => panic!("unexpected index"),
            }
        };

        let result = scc_fallback_sort(&graph, &node_to_orig, &get_name);
        let names: Vec<String> = result.iter().map(|&idx| get_name(idx)).collect();
        assert_eq!(
            names,
            vec!["a", "b", "c"],
            "SCC members sorted alphabetically"
        );
    }

    #[test]
    fn test_scc_fallback_with_external_node() {
        // SCC {A,B} (A⇄B) + standalone C, C→A edge
        // Condensed: SCC{A,B} → C (C depends on SCC)
        // Expected: [A, B, C] — SCC first (expanded alphabetically), C after
        let mut graph: DiGraph<NodeIndex, usize> = DiGraph::new();
        let a_orig = NodeIndex::new(10);
        let b_orig = NodeIndex::new(11);
        let c_orig = NodeIndex::new(12);
        let a = graph.add_node(a_orig);
        let b = graph.add_node(b_orig);
        let c = graph.add_node(c_orig);
        graph.add_edge(a, b, 1);
        graph.add_edge(b, a, 1); // cycle A⇄B
        graph.add_edge(c, a, 1); // C depends on A

        let node_to_orig: HashMap<petgraph::graph::NodeIndex, NodeIndex> =
            graph.node_indices().map(|n| (n, graph[n])).collect();

        let get_name = |idx: NodeIndex| -> String {
            match idx.index() {
                10 => "a".to_string(),
                11 => "b".to_string(),
                12 => "c".to_string(),
                _ => panic!("unexpected index"),
            }
        };

        let result = scc_fallback_sort(&graph, &node_to_orig, &get_name);
        let names: Vec<String> = result.iter().map(|&idx| get_name(idx)).collect();
        // C depends on SCC{A,B}, so C comes first (dependent before dependency)
        // Wait — in our convention: edge C→A means C depends on A.
        // toposort: dependents come before dependencies → C first, then SCC{A,B}
        assert_eq!(
            names,
            vec!["c", "a", "b"],
            "Dependent C first, then SCC{{A,B}} alphabetically"
        );
    }

    // === Barycenter Direction Test ===

    #[test]
    fn test_barycenter_incoming_gives_dependents() {
        // Verify edge direction: A→B means "A depends on B"
        // neighbors_directed(B, Incoming) should return A (the dependent)
        let mut g: DiGraph<NodeIndex, usize> = DiGraph::new();
        let a_orig = NodeIndex::new(0);
        let b_orig = NodeIndex::new(1);
        let a = g.add_node(a_orig);
        let b = g.add_node(b_orig);
        g.add_edge(a, b, 1); // A depends on B

        let incoming: Vec<_> = g
            .neighbors_directed(b, petgraph::Direction::Incoming)
            .collect();
        assert_eq!(incoming, vec![a], "Incoming neighbors of B should be [A]");
    }
}
