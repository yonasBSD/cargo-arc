//! Graph Types & Builder

use petgraph::graph::{DiGraph, NodeIndex};
use std::path::PathBuf;

pub enum Node {
    Crate { name: String, path: PathBuf },
    Module { name: String, crate_idx: NodeIndex },
}

pub enum Edge {
    CrateDep,
    ModuleDep,
    Contains,
}

pub type ArcGraph = DiGraph<Node, Edge>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_node_creation() {
        let crate_node = Node::Crate {
            name: "my_crate".to_string(),
            path: PathBuf::from("/path/to/crate"),
        };
        let module_node = Node::Module {
            name: "my_module".to_string(),
            crate_idx: NodeIndex::new(0),
        };
        // Nodes should be creatable
        match crate_node {
            Node::Crate { name, .. } => assert_eq!(name, "my_crate"),
            _ => panic!("Expected Crate node"),
        }
        match module_node {
            Node::Module { name, .. } => assert_eq!(name, "my_module"),
            _ => panic!("Expected Module node"),
        }
    }

    #[test]
    fn test_edge_types() {
        let edges = [Edge::CrateDep, Edge::ModuleDep, Edge::Contains];
        assert_eq!(edges.len(), 3);
    }
}
