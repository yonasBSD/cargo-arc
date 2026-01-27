import { test, expect, describe } from "bun:test";
import { Selectors } from "./selectors.js";

describe("Selectors", () => {
  describe("IDs", () => {
    test("nodeId generates node-prefixed ID", () => {
      expect(Selectors.nodeId("foo")).toBe("node-foo");
      expect(Selectors.nodeId("crate::module")).toBe("node-crate::module");
    });

    test("countId generates count-prefixed ID", () => {
      expect(Selectors.countId("bar")).toBe("count-bar");
    });
  });

  describe("CSS Selectors", () => {
    test("visibleArc selects dep-arc or cycle-arc by arc-id", () => {
      expect(Selectors.visibleArc("a-b")).toBe(
        '.dep-arc[data-arc-id="a-b"], .cycle-arc[data-arc-id="a-b"]'
      );
    });

    test("hitarea selects arc-hitarea by arc-id", () => {
      expect(Selectors.hitarea("x-y")).toBe('.arc-hitarea[data-arc-id="x-y"]');
    });

    test("arrows selects by data-edge attribute", () => {
      expect(Selectors.arrows("from-to")).toBe('[data-edge="from-to"]');
    });

    test("virtualArrows selects data-vedge excluding arc-count", () => {
      expect(Selectors.virtualArrows("v-edge")).toBe(
        '[data-vedge="v-edge"]:not(.arc-count)'
      );
    });

    test("virtualArc selects by from and to attributes", () => {
      expect(Selectors.virtualArc("nodeA", "nodeB")).toBe(
        '.virtual-arc[data-from="nodeA"][data-to="nodeB"]'
      );
    });

    test("connectedHitareas selects hitareas connected to a node", () => {
      expect(Selectors.connectedHitareas("myNode")).toBe(
        '.arc-hitarea[data-from="myNode"], .arc-hitarea[data-to="myNode"]'
      );
    });

    test("labelGroup selects arc-count-group by vedge", () => {
      expect(Selectors.labelGroup("edge-id")).toBe(
        '.arc-count-group[data-vedge="edge-id"]'
      );
    });
  });

  describe("Layer Selectors", () => {
    test("highlightedArcs returns selector for highlight-arcs-layer children", () => {
      expect(Selectors.highlightedArcs()).toBe("#highlight-arcs-layer > *");
    });

    test("highlightedLabels returns selector for highlight-labels-layer children", () => {
      expect(Selectors.highlightedLabels()).toBe("#highlight-labels-layer > *");
    });

    test("highlightedHitareas returns selector for highlight-hitareas-layer children", () => {
      expect(Selectors.highlightedHitareas()).toBe("#highlight-hitareas-layer > *");
    });
  });

  describe("Edge cases", () => {
    test("handles empty string IDs", () => {
      expect(Selectors.nodeId("")).toBe("node-");
      expect(Selectors.visibleArc("")).toBe(
        '.dep-arc[data-arc-id=""], .cycle-arc[data-arc-id=""]'
      );
    });

    test("handles IDs with special characters", () => {
      expect(Selectors.nodeId("crate::mod::sub")).toBe("node-crate::mod::sub");
      expect(Selectors.virtualArc("a::b", "c::d")).toBe(
        '.virtual-arc[data-from="a::b"][data-to="c::d"]'
      );
    });
  });
});
