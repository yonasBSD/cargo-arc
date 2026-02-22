import { test, expect, describe, beforeEach } from "bun:test";

// Minimal STATIC_DATA.classes mock
if (!globalThis.STATIC_DATA) globalThis.STATIC_DATA = {};
if (!globalThis.STATIC_DATA.classes) globalThis.STATIC_DATA.classes = {};
Object.assign(globalThis.STATIC_DATA.classes, {
  searchActive: "search-active",
  searchMatch: "search-match",
  searchMatchParent: "search-match-parent",
  label: "label",
  toolbarScopeActive: "scope-active",
});

// Minimal DomAdapter mock
globalThis.DomAdapter = {
  querySelector: () => null,
  querySelectorAll: () => [],
  getElementById: () => null,
  getSvgRoot: () => null,
  getNode: () => null,
};

// Minimal StaticData mock
globalThis.StaticData = {
  getAllNodeIds: () => [],
  getNode: () => null,
  getAllArcIds: () => [],
  getArc: () => null,
};

// Minimal AppState mock
globalThis.AppState = {
  isCollapsed: () => false,
};

const { SearchLogic } = require("./search.js");

describe("SearchLogic", () => {
  beforeEach(() => {
    // Reset internal state between tests
    SearchLogic.clearSearch();
  });

  describe("refresh", () => {
    test("does nothing when search is not active", () => {
      let executeCalled = false;
      const origExecute = SearchLogic.executeSearch;
      SearchLogic.executeSearch = () => { executeCalled = true; return 0; };

      SearchLogic.refresh();
      expect(executeCalled).toBe(false);

      SearchLogic.executeSearch = origExecute;
    });

    test("re-executes search when active with query", () => {
      let capturedArgs = null;
      const origExecute = SearchLogic.executeSearch;

      // First call sets up state, then we intercept subsequent calls
      SearchLogic.executeSearch = function(query, scope) {
        // Use original to set up active state
        return origExecute.call(this, query, scope);
      };

      // Set up nodes for a match
      const origGetAllNodeIds = StaticData.getAllNodeIds;
      const origGetNode = StaticData.getNode;
      StaticData.getAllNodeIds = () => ["crate-1"];
      StaticData.getNode = (id) => id === "crate-1" ? { name: "my-crate", type: "crate", parent: null } : null;

      // Execute a real search to set active state
      SearchLogic.executeSearch("my-crate", "all");
      expect(SearchLogic.isActive()).toBe(true);

      // Now intercept to verify refresh calls executeSearch
      SearchLogic.executeSearch = (query, scope) => {
        capturedArgs = { query, scope };
        return 0;
      };

      SearchLogic.refresh();
      expect(capturedArgs).toEqual({ query: "my-crate", scope: "all" });

      // Restore
      SearchLogic.executeSearch = origExecute;
      StaticData.getAllNodeIds = origGetAllNodeIds;
      StaticData.getNode = origGetNode;
    });
  });

  describe("isActive", () => {
    test("returns false initially", () => {
      expect(SearchLogic.isActive()).toBe(false);
    });

    test("returns true after executeSearch", () => {
      const origGetAllNodeIds = StaticData.getAllNodeIds;
      StaticData.getAllNodeIds = () => ["n1"];
      const origGetNode = StaticData.getNode;
      StaticData.getNode = () => ({ name: "foo", type: "crate", parent: null });

      SearchLogic.executeSearch("foo", "all");
      expect(SearchLogic.isActive()).toBe(true);

      StaticData.getAllNodeIds = origGetAllNodeIds;
      StaticData.getNode = origGetNode;
    });

    test("returns false after clearSearch", () => {
      const origGetAllNodeIds = StaticData.getAllNodeIds;
      StaticData.getAllNodeIds = () => ["n1"];
      const origGetNode = StaticData.getNode;
      StaticData.getNode = () => ({ name: "foo", type: "crate", parent: null });

      SearchLogic.executeSearch("foo", "all");
      SearchLogic.clearSearch();
      expect(SearchLogic.isActive()).toBe(false);

      StaticData.getAllNodeIds = origGetAllNodeIds;
      StaticData.getNode = origGetNode;
    });
  });

  describe("executeSearch", () => {
    test("clears and returns 0 for empty query", () => {
      const result = SearchLogic.executeSearch("", "all");
      expect(result).toBe(0);
      expect(SearchLogic.isActive()).toBe(false);
    });

    test("clears and returns 0 for whitespace-only query", () => {
      const result = SearchLogic.executeSearch("   ", "all");
      expect(result).toBe(0);
    });

    test("matches nodes by name substring", () => {
      const origGetAllNodeIds = StaticData.getAllNodeIds;
      const origGetNode = StaticData.getNode;
      StaticData.getAllNodeIds = () => ["a", "b", "c"];
      StaticData.getNode = (id) => ({
        a: { name: "auth-service", type: "crate", parent: null },
        b: { name: "core", type: "module", parent: null },
        c: { name: "auth-utils", type: "crate", parent: null },
      }[id]);

      const result = SearchLogic.executeSearch("auth", "all");
      expect(result).toBe(2);
      expect(SearchLogic.getMatchedNodeIds().has("a")).toBe(true);
      expect(SearchLogic.getMatchedNodeIds().has("c")).toBe(true);

      StaticData.getAllNodeIds = origGetAllNodeIds;
      StaticData.getNode = origGetNode;
    });

    test("respects scope filter for crate", () => {
      const origGetAllNodeIds = StaticData.getAllNodeIds;
      const origGetNode = StaticData.getNode;
      StaticData.getAllNodeIds = () => ["a", "b"];
      StaticData.getNode = (id) => ({
        a: { name: "foo", type: "crate", parent: null },
        b: { name: "foo-mod", type: "module", parent: null },
      }[id]);

      const result = SearchLogic.executeSearch("foo", "crate");
      expect(result).toBe(1);
      expect(SearchLogic.getMatchedNodeIds().has("a")).toBe(true);
      expect(SearchLogic.getMatchedNodeIds().has("b")).toBe(false);

      StaticData.getAllNodeIds = origGetAllNodeIds;
      StaticData.getNode = origGetNode;
    });
  });
});
