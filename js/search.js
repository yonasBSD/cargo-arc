// @module SearchLogic
// @deps StaticData, DomAdapter, AppState
// @config
// search.js - Substring search with scope selector and highlight dimming

const SearchLogic = {
  _state: {
    active: false,
    query: '',
    scope: 'all',
    matchedNodeIds: new Set(),
    matchParentIds: new Set(),
    debounceTimer: null,
  },

  /**
   * Initialize search event listeners.
   * @param {Object} appState - AppState instance (for collapse checks)
   */
  init(appState) {
    this._appState = appState;

    const input = DomAdapter.querySelector('#search-input');
    const clearBtn = DomAdapter.querySelector('#search-clear');
    const scopeSelector = DomAdapter.querySelector('#scope-selector');

    if (input) {
      input.addEventListener('input', (e) => this._onInput(e));
    }
    if (clearBtn) {
      clearBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        const inp = DomAdapter.querySelector('#search-input');
        if (inp) inp.value = '';
        clearBtn.style.display = 'none';
        this.clearSearch();
      });
    }
    if (scopeSelector) {
      scopeSelector.addEventListener('click', (e) => {
        const btn = e.target.closest('[data-scope]');
        if (!btn) return;
        e.stopPropagation();
        this.setScope(btn.dataset.scope);
      });
    }
  },

  _onInput(e) {
    const query = e.target.value;
    const clearBtn = DomAdapter.querySelector('#search-clear');
    if (clearBtn) clearBtn.style.display = query ? 'block' : 'none';

    clearTimeout(this._state.debounceTimer);

    if (!query.trim()) {
      this.clearSearch();
      return;
    }

    this._state.debounceTimer = setTimeout(() => {
      this.executeSearch(query, this._state.scope);
    }, 150);
  },

  /**
   * Execute search with given query and scope.
   * @param {string} query
   * @param {string} scope - 'all', 'crate', 'module', 'symbol'
   * @returns {number} Number of matches
   */
  executeSearch(query, scope) {
    const q = query.toLowerCase().trim();
    if (!q) { this.clearSearch(); return 0; }

    this._state.query = q;
    this._state.scope = scope;

    const matchedNodes = new Set();

    if (scope === 'all' || scope === 'crate' || scope === 'module') {
      for (const nodeId of StaticData.getAllNodeIds()) {
        const node = StaticData.getNode(nodeId);
        if (!node) continue;
        if (scope === 'crate' && node.type !== 'crate') continue;
        if (scope === 'module' && node.type !== 'module') continue;
        if (node.name.toLowerCase().includes(q)) {
          matchedNodes.add(nodeId);
        }
      }
    }

    if (scope === 'all' || scope === 'symbol') {
      for (const arcId of StaticData.getAllArcIds()) {
        const arc = StaticData.getArc(arcId);
        if (!arc || !arc.usages) continue;
        for (const group of arc.usages) {
          if (group.symbol && group.symbol.toLowerCase().includes(q)) {
            matchedNodes.add(arc.from);
            matchedNodes.add(arc.to);
            break;
          }
        }
      }
    }

    // Collapsed-parent-resolution
    const directMatches = new Set();
    const parentMatches = new Set();

    for (const nodeId of matchedNodes) {
      const visible = this._resolveVisibleAncestor(nodeId);
      if (visible === nodeId) {
        directMatches.add(nodeId);
      } else {
        parentMatches.add(visible);
      }
    }

    // Apply DOM updates
    this._clearDom();

    const C = STATIC_DATA.classes;
    const svg = DomAdapter.getSvgRoot();
    if (svg) svg.classList.add(C.searchActive);

    for (const nodeId of directMatches) {
      const rect = DomAdapter.getNode(nodeId);
      if (rect) {
        rect.classList.add(C.searchMatch);
        const label = rect.nextElementSibling;
        if (label && label.classList.contains(C.label)) {
          label.classList.add(C.searchMatch);
        }
      }
    }

    for (const nodeId of parentMatches) {
      if (directMatches.has(nodeId)) continue;
      const rect = DomAdapter.getNode(nodeId);
      if (rect) {
        rect.classList.add(C.searchMatchParent);
        const label = rect.nextElementSibling;
        if (label && label.classList.contains(C.label)) {
          label.classList.add(C.searchMatchParent);
        }
      }
    }

    this._state.matchedNodeIds = directMatches;
    this._state.matchParentIds = parentMatches;
    this._state.active = true;

    const total = directMatches.size + parentMatches.size;
    const countEl = DomAdapter.getElementById('search-result-count');
    if (countEl) countEl.textContent = total + ' match' + (total !== 1 ? 'es' : '');

    // Update scope button active state
    this._updateScopeButtons(scope);

    return total;
  },

  /**
   * Clear all search highlights.
   */
  clearSearch() {
    this._clearDom();
    this._state.active = false;
    this._state.query = '';
    this._state.matchedNodeIds = new Set();
    this._state.matchParentIds = new Set();

    const countEl = DomAdapter.getElementById('search-result-count');
    if (countEl) countEl.textContent = '';
  },

  /**
   * Change scope and re-execute current query.
   * @param {string} scope
   */
  setScope(scope) {
    this._state.scope = scope;
    this._updateScopeButtons(scope);
    if (this._state.query) {
      this.executeSearch(this._state.query, scope);
    }
  },

  isActive() { return this._state.active; },

  refresh() {
    if (this._state.active && this._state.query) {
      this.executeSearch(this._state.query, this._state.scope);
    }
  },

  getMatchedNodeIds() { return this._state.matchedNodeIds; },

  // --- Internal helpers ---

  _clearDom() {
    const C = STATIC_DATA.classes;
    const svg = DomAdapter.getSvgRoot();
    if (svg) svg.classList.remove(C.searchActive);

    for (const nodeId of this._state.matchedNodeIds) {
      const rect = DomAdapter.getNode(nodeId);
      if (rect) {
        rect.classList.remove(C.searchMatch);
        const label = rect.nextElementSibling;
        if (label) label.classList.remove(C.searchMatch);
      }
    }
    for (const nodeId of this._state.matchParentIds) {
      const rect = DomAdapter.getNode(nodeId);
      if (rect) {
        rect.classList.remove(C.searchMatchParent);
        const label = rect.nextElementSibling;
        if (label) label.classList.remove(C.searchMatchParent);
      }
    }
  },

  _resolveVisibleAncestor(nodeId) {
    let current = nodeId;
    while (current) {
      const node = StaticData.getNode(current);
      if (!node || node.parent === null) return current;
      if (AppState.isCollapsed(this._appState, node.parent)) {
        current = node.parent;
      } else {
        return current;
      }
    }
    return nodeId;
  },

  _updateScopeButtons(scope) {
    const C = STATIC_DATA.classes;
    const selector = DomAdapter.querySelector('#scope-selector');
    if (!selector) return;
    const buttons = selector.querySelectorAll('[data-scope]');
    buttons.forEach(btn => {
      if (btn.dataset.scope === scope) {
        btn.classList.add(C.toolbarScopeActive);
      } else {
        btn.classList.remove(C.toolbarScopeActive);
      }
    });
  },
};

// CommonJS export for tests (Node/Bun)
if (typeof module !== 'undefined') {
  module.exports = { SearchLogic };
}
