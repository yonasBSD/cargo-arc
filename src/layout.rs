//! Layout IR & Algorithms

/// Index into LayoutIR.items
pub type NodeId = usize;

#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    Crate,
    Module { nesting: u32, parent: NodeId },
}

#[derive(Debug, Clone)]
pub struct LayoutItem {
    pub id: NodeId,
    pub kind: ItemKind,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeKind {
    Normal,
    DirectCycle,
    TransitiveCycle,
}

#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
}

#[derive(Debug, Default)]
pub struct LayoutIR {
    pub items: Vec<LayoutItem>,
    pub edges: Vec<LayoutEdge>,
}

impl LayoutIR {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_item(&mut self, kind: ItemKind, label: String) -> NodeId {
        let id = self.items.len();
        self.items.push(LayoutItem { id, kind, label });
        id
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId, kind: EdgeKind) {
        self.edges.push(LayoutEdge { from, to, kind });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_item_creation() {
        let crate_item = LayoutItem {
            id: 0,
            kind: ItemKind::Crate,
            label: "my_crate".to_string(),
        };
        let module_item = LayoutItem {
            id: 1,
            kind: ItemKind::Module {
                nesting: 1,
                parent: 0,
            },
            label: "my_module".to_string(),
        };
        assert_eq!(crate_item.label, "my_crate");
        assert_eq!(module_item.id, 1);
        match module_item.kind {
            ItemKind::Module { nesting, parent } => {
                assert_eq!(nesting, 1);
                assert_eq!(parent, 0);
            }
            _ => panic!("Expected Module"),
        }
    }

    #[test]
    fn test_layout_edge_kinds() {
        let normal = LayoutEdge {
            from: 0,
            to: 1,
            kind: EdgeKind::Normal,
        };
        let direct = LayoutEdge {
            from: 1,
            to: 0,
            kind: EdgeKind::DirectCycle,
        };
        let trans = LayoutEdge {
            from: 2,
            to: 3,
            kind: EdgeKind::TransitiveCycle,
        };

        assert_eq!(normal.from, 0);
        assert!(matches!(direct.kind, EdgeKind::DirectCycle));
        assert!(matches!(trans.kind, EdgeKind::TransitiveCycle));
    }

    #[test]
    fn test_layout_ir_builder() {
        let mut ir = LayoutIR::new();

        let crate_id = ir.add_item(ItemKind::Crate, "my_crate".to_string());
        let mod_id = ir.add_item(
            ItemKind::Module {
                nesting: 1,
                parent: crate_id,
            },
            "my_module".to_string(),
        );
        ir.add_edge(crate_id, mod_id, EdgeKind::Normal);

        assert_eq!(ir.items.len(), 2);
        assert_eq!(ir.edges.len(), 1);
        assert_eq!(ir.items[crate_id].label, "my_crate");
    }
}
