//! Cycle detection for module dependency graphs.

use crate::graph::{ArcGraph, Edge};
use petgraph::algo::tarjan_scc;

/// A cycle in the module dependency graph (SCC with >1 node)
#[derive(Debug, Clone, PartialEq)]
pub struct Cycle {
    /// NodeIndices participating in this cycle
    pub nodes: Vec<petgraph::graph::NodeIndex>,
}

/// Detect cycles in module dependencies using Tarjan's SCC algorithm.
/// Only considers ModuleDep edges (Rust prevents crate-level cycles).
pub fn detect_cycles(graph: &ArcGraph) -> Vec<Cycle> {
    // Build filtered graph with only ModuleDep edges
    let mut filtered = ArcGraph::new();
    let mut node_map = std::collections::HashMap::new();

    // Copy all nodes
    for idx in graph.node_indices() {
        let new_idx = filtered.add_node(graph[idx].clone());
        node_map.insert(idx, new_idx);
    }

    // Copy only Production ModuleDep edges
    for edge_idx in graph.edge_indices() {
        if matches!(&graph[edge_idx], Edge::ModuleDep { context, .. } if context.kind == crate::model::DependencyKind::Production)
        {
            let (src, dst) = graph.edge_endpoints(edge_idx).unwrap();
            filtered.add_edge(
                node_map[&src],
                node_map[&dst],
                Edge::ModuleDep {
                    locations: vec![],
                    context: crate::model::EdgeContext::production(),
                },
            );
        }
    }

    // Run Tarjan's SCC algorithm
    let sccs = tarjan_scc(&filtered);

    // Filter to SCCs with >1 node (actual cycles)
    // Map back to original graph indices
    let reverse_map: std::collections::HashMap<_, _> =
        node_map.iter().map(|(k, v)| (*v, *k)).collect();

    sccs.into_iter()
        .filter(|scc| scc.len() > 1)
        .map(|scc| Cycle {
            nodes: scc.into_iter().map(|idx| reverse_map[&idx]).collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{ArcGraph, Edge, Node};
    use petgraph::graph::NodeIndex;

    // === Cycle Detection Tests ===

    #[test]
    fn test_no_cycles() {
        // A -> B -> C (linear, no cycle)
        let mut graph = ArcGraph::new();
        let a = graph.add_node(Node::Module {
            name: "a".to_string(),
            crate_idx: NodeIndex::new(0),
        });
        let b = graph.add_node(Node::Module {
            name: "b".to_string(),
            crate_idx: NodeIndex::new(0),
        });
        let c = graph.add_node(Node::Module {
            name: "c".to_string(),
            crate_idx: NodeIndex::new(0),
        });
        graph.add_edge(
            a,
            b,
            Edge::ModuleDep {
                locations: vec![],
                context: crate::model::EdgeContext::production(),
            },
        );
        graph.add_edge(
            b,
            c,
            Edge::ModuleDep {
                locations: vec![],
                context: crate::model::EdgeContext::production(),
            },
        );

        let cycles = detect_cycles(&graph);
        assert!(cycles.is_empty(), "Linear graph should have no cycles");
    }

    #[test]
    fn test_direct_cycle() {
        // A <-> B (direct cycle between two modules)
        let mut graph = ArcGraph::new();
        let a = graph.add_node(Node::Module {
            name: "a".to_string(),
            crate_idx: NodeIndex::new(0),
        });
        let b = graph.add_node(Node::Module {
            name: "b".to_string(),
            crate_idx: NodeIndex::new(0),
        });
        graph.add_edge(
            a,
            b,
            Edge::ModuleDep {
                locations: vec![],
                context: crate::model::EdgeContext::production(),
            },
        );
        graph.add_edge(
            b,
            a,
            Edge::ModuleDep {
                locations: vec![],
                context: crate::model::EdgeContext::production(),
            },
        );

        let cycles = detect_cycles(&graph);
        assert_eq!(cycles.len(), 1, "Should detect one cycle");
        assert_eq!(cycles[0].nodes.len(), 2, "Cycle should contain 2 nodes");
        assert!(cycles[0].nodes.contains(&a));
        assert!(cycles[0].nodes.contains(&b));
    }

    #[test]
    fn test_transitive_cycle() {
        // A -> B -> C -> A (transitive cycle)
        let mut graph = ArcGraph::new();
        let a = graph.add_node(Node::Module {
            name: "a".to_string(),
            crate_idx: NodeIndex::new(0),
        });
        let b = graph.add_node(Node::Module {
            name: "b".to_string(),
            crate_idx: NodeIndex::new(0),
        });
        let c = graph.add_node(Node::Module {
            name: "c".to_string(),
            crate_idx: NodeIndex::new(0),
        });
        graph.add_edge(
            a,
            b,
            Edge::ModuleDep {
                locations: vec![],
                context: crate::model::EdgeContext::production(),
            },
        );
        graph.add_edge(
            b,
            c,
            Edge::ModuleDep {
                locations: vec![],
                context: crate::model::EdgeContext::production(),
            },
        );
        graph.add_edge(
            c,
            a,
            Edge::ModuleDep {
                locations: vec![],
                context: crate::model::EdgeContext::production(),
            },
        );

        let cycles = detect_cycles(&graph);
        assert_eq!(cycles.len(), 1, "Should detect one cycle");
        assert_eq!(cycles[0].nodes.len(), 3, "Cycle should contain 3 nodes");
    }

    #[test]
    fn test_multiple_independent_cycles() {
        // A <-> B (cycle 1), C <-> D (cycle 2)
        // Two independent cycles that should both be detected
        let mut graph = ArcGraph::new();
        let a = graph.add_node(Node::Module {
            name: "a".to_string(),
            crate_idx: NodeIndex::new(0),
        });
        let b = graph.add_node(Node::Module {
            name: "b".to_string(),
            crate_idx: NodeIndex::new(0),
        });
        let c = graph.add_node(Node::Module {
            name: "c".to_string(),
            crate_idx: NodeIndex::new(0),
        });
        let d = graph.add_node(Node::Module {
            name: "d".to_string(),
            crate_idx: NodeIndex::new(0),
        });

        // Cycle 1: A <-> B
        graph.add_edge(
            a,
            b,
            Edge::ModuleDep {
                locations: vec![],
                context: crate::model::EdgeContext::production(),
            },
        );
        graph.add_edge(
            b,
            a,
            Edge::ModuleDep {
                locations: vec![],
                context: crate::model::EdgeContext::production(),
            },
        );

        // Cycle 2: C <-> D
        graph.add_edge(
            c,
            d,
            Edge::ModuleDep {
                locations: vec![],
                context: crate::model::EdgeContext::production(),
            },
        );
        graph.add_edge(
            d,
            c,
            Edge::ModuleDep {
                locations: vec![],
                context: crate::model::EdgeContext::production(),
            },
        );

        let cycles = detect_cycles(&graph);
        assert_eq!(cycles.len(), 2, "Should detect two independent cycles");

        // Each cycle should have 2 nodes
        for cycle in &cycles {
            assert_eq!(cycle.nodes.len(), 2, "Each cycle should contain 2 nodes");
        }

        // Verify both cycles are detected (order may vary)
        let all_nodes: Vec<_> = cycles.iter().flat_map(|c| c.nodes.iter()).collect();
        assert!(all_nodes.contains(&&a), "Cycle 1 should contain A");
        assert!(all_nodes.contains(&&b), "Cycle 1 should contain B");
        assert!(all_nodes.contains(&&c), "Cycle 2 should contain C");
        assert!(all_nodes.contains(&&d), "Cycle 2 should contain D");
    }
}
