import { test, expect, describe, beforeEach } from "bun:test";
import { ArcLogic } from "./svg_script.js";

// Mock STATIC_DATA for tests
const TEST_STATIC_DATA = {
  nodes: {
    crate: { type: "crate", parent: null, x: 0, y: 0, hasChildren: true },
    mod_a: { type: "module", parent: "crate", x: 20, y: 50, hasChildren: true },
    fn_1: { type: "function", parent: "mod_a", x: 40, y: 60, hasChildren: false },
    fn_2: { type: "function", parent: "mod_a", x: 40, y: 80, hasChildren: false }
  },
  arcs: {
    "fn_1-fn_2": { from: "fn_1", to: "fn_2", usages: ["mod_a.rs:10"] },
    "mod_a-crate": { from: "mod_a", to: "crate", usages: ["lib.rs:5", "lib.rs:10", "lib.rs:15"] }
  }
};

// Inject STATIC_DATA globally for StaticData module
globalThis.STATIC_DATA = TEST_STATIC_DATA;

// Import after STATIC_DATA is set
const { StaticData } = await import("./static_data.js");

describe("StaticData", () => {
  describe("getArcStrokeWidth", () => {
    test("returns stroke width for arc with 1 usage", () => {
      // fn_1-fn_2 has 1 usage
      const width = StaticData.getArcStrokeWidth("fn_1-fn_2");

      // 1 usage -> calculateStrokeWidth(1) = MIN (0.5)
      const expected = ArcLogic.calculateStrokeWidth(1);
      expect(width).toBe(expected);
    });

    test("returns stroke width for arc with multiple usages", () => {
      // mod_a-crate has 3 usages
      const width = StaticData.getArcStrokeWidth("mod_a-crate");

      // 3 usages -> calculateStrokeWidth(3)
      const expected = ArcLogic.calculateStrokeWidth(3);
      expect(width).toBe(expected);
    });

    test("returns minimum stroke width for non-existent arc", () => {
      const width = StaticData.getArcStrokeWidth("nonexistent-arc");

      // 0 usages -> MIN (0.5)
      expect(width).toBe(0.5);
    });

    test("returns correct width for high usage count", () => {
      // Temporarily modify STATIC_DATA for this test
      const originalArcs = { ...TEST_STATIC_DATA.arcs };
      TEST_STATIC_DATA.arcs["heavy-arc"] = {
        from: "fn_1",
        to: "crate",
        usages: Array(50).fill("file.rs:1")
      };

      const width = StaticData.getArcStrokeWidth("heavy-arc");
      const expected = ArcLogic.calculateStrokeWidth(50);
      expect(width).toBe(expected);

      // Restore original
      delete TEST_STATIC_DATA.arcs["heavy-arc"];
    });
  });

  describe("existing functions", () => {
    test("getArcWeight returns usage count", () => {
      expect(StaticData.getArcWeight("fn_1-fn_2")).toBe(1);
      expect(StaticData.getArcWeight("mod_a-crate")).toBe(3);
    });

    test("getAllArcIds returns all arc IDs", () => {
      const ids = StaticData.getAllArcIds();
      expect(ids).toContain("fn_1-fn_2");
      expect(ids).toContain("mod_a-crate");
    });
  });
});
