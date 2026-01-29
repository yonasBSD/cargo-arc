// derived_state.js - Pure functions to derive display state from core state
// Computes highlights and visibility based on selection and collapse state
// No DOM dependencies - operates on Maps/Sets

// Import TreeLogic for visibility calculations
// In browser: TreeLogic is global; in tests: imported
const TreeLogic = (typeof window !== 'undefined' && window.TreeLogic)
  ? window.TreeLogic
  : require('./tree_logic.js').TreeLogic;

const DerivedState = {
  /**
   * Derive highlight roles for nodes and which arcs to highlight.
   * @param {{ mode: string, type: string|null, id: string|null }} selection
   * @param {Object} staticData - StaticData accessor
   * @returns {{ nodeRoles: Map<string, string>, highlightedArcs: Set<string> }}
   */
  deriveHighlights(selection, staticData) {
    const nodeRoles = new Map();
    const highlightedArcs = new Set();

    if (selection.mode === 'none') {
      return { nodeRoles, highlightedArcs };
    }

    if (selection.type === 'node') {
      const selectedNodeId = selection.id;
      nodeRoles.set(selectedNodeId, 'current');

      // Find all arcs connected to this node
      for (const arcId of staticData.getAllArcIds()) {
        const arc = staticData.getArc(arcId);
        if (!arc) continue;

        if (arc.from === selectedNodeId) {
          // Outgoing arc: target is dependency
          highlightedArcs.add(arcId);
          if (!nodeRoles.has(arc.to)) {
            nodeRoles.set(arc.to, 'dependency');
          }
        } else if (arc.to === selectedNodeId) {
          // Incoming arc: source is dependent
          highlightedArcs.add(arcId);
          if (!nodeRoles.has(arc.from)) {
            nodeRoles.set(arc.from, 'dependent');
          }
        }
      }
    } else if (selection.type === 'arc') {
      const arc = staticData.getArc(selection.id);
      if (arc) {
        highlightedArcs.add(selection.id);
        nodeRoles.set(arc.from, 'dependent');
        nodeRoles.set(arc.to, 'dependency');
      }
    }

    return { nodeRoles, highlightedArcs };
  },

  /**
   * Derive which nodes are visible based on collapsed state.
   * A node is visible if getVisibleAncestor(nodeId) === nodeId
   * @param {Set<string>} collapsed - Set of collapsed node IDs
   * @param {Object} staticData - StaticData accessor
   * @returns {Set<string>} - Set of visible node IDs
   */
  deriveNodeVisibility(collapsed, staticData) {
    const parentMap = staticData.buildParentMap();
    const visibleNodes = new Set();

    for (const nodeId of staticData.getAllNodeIds()) {
      const visibleAncestor = TreeLogic.getVisibleAncestor(nodeId, collapsed, parentMap);
      if (visibleAncestor === nodeId) {
        visibleNodes.add(nodeId);
      }
    }

    return visibleNodes;
  },

  /**
   * Derive which arcs are visible (both endpoints visible).
   * @param {Set<string>} visibleNodes - Set of visible node IDs
   * @param {Object} staticData - StaticData accessor
   * @returns {Set<string>} - Set of visible arc IDs
   */
  deriveArcVisibility(visibleNodes, staticData) {
    const visibleArcs = new Set();

    for (const arcId of staticData.getAllArcIds()) {
      const arc = staticData.getArc(arcId);
      if (arc && visibleNodes.has(arc.from) && visibleNodes.has(arc.to)) {
        visibleArcs.add(arcId);
      }
    }

    return visibleArcs;
  },

  /**
   * Determine arc direction based on hierarchy.
   * @private
   * @param {string} fromId
   * @param {string} toId
   * @param {Map<string, string>} parentMap
   * @returns {string} 'upward' | 'downward'
   */
  _determineDirection(fromId, toId, parentMap) {
    // Simple heuristic: check if 'to' is an ancestor of 'from' (upward)
    // Otherwise downward
    let current = fromId;
    while (current) {
      if (current === toId) return 'upward';
      current = parentMap.get(current);
    }
    return 'downward';
  },

  /**
   * Compute current positions for all visible nodes based on collapse state.
   * Replaces extractNodePositions() - no DOM reads needed.
   * @param {Set<string>} collapsed - Set of collapsed node IDs
   * @param {Object} staticData - StaticData accessor
   * @param {number} margin - Top/left margin
   * @param {number} toolbarHeight - Height of toolbar
   * @param {number} rowHeight - Height per row
   * @returns {Map<string, {x: number, y: number, width: number, height: number}>}
   */
  computeCurrentPositions(collapsed, staticData, margin, toolbarHeight, rowHeight) {
    const visibleNodes = this.deriveNodeVisibility(collapsed, staticData);
    const positions = new Map();

    // Sort visible nodes by original Y position
    const sortedIds = [...visibleNodes].sort((a, b) => {
      const posA = staticData.getOriginalPosition(a);
      const posB = staticData.getOriginalPosition(b);
      return posA.y - posB.y;
    });

    // Compute new Y positions based on visible order
    let currentY = margin + toolbarHeight;
    for (const nodeId of sortedIds) {
      const orig = staticData.getOriginalPosition(nodeId);
      positions.set(nodeId, {
        x: orig.x,
        y: currentY,
        width: orig.width,
        height: orig.height
      });
      currentY += rowHeight;
    }

    return positions;
  },

  /**
   * Compute the rightmost X coordinate from positions.
   * @param {Map<string, {x: number, width: number}>} positions
   * @returns {number}
   */
  computeMaxRight(positions) {
    let maxRight = 0;
    for (const pos of positions.values()) {
      maxRight = Math.max(maxRight, pos.x + pos.width);
    }
    return maxRight;
  }
};

// CommonJS export for tests (Node/Bun)
if (typeof module !== 'undefined') {
  module.exports = { DerivedState };
}

// Browser export
if (typeof window !== 'undefined') {
  window.DerivedState = DerivedState;
}
