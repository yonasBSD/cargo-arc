import { test, expect, describe, beforeEach } from "bun:test";
import { createFakeElement } from "./dom_adapter.js";
import { SidebarLogic } from "./sidebar.js";

// Mock Selectors (sidebar.js uses _getMaxArcRightX → Selectors.allArcPaths)
globalThis.Selectors = {
  allArcPaths: () => ".dep-arc, .cycle-arc, .virtual-arc",
};

// Mock STATIC_DATA for buildContent tests
globalThis.STATIC_DATA = {
  arcs: {
    "crate_a-crate_b": {
      from: "crate_a",
      to: "crate_b",
      usages: [
        "ModuleInfo  <- src/cli.rs:7",
        "             <- src/render.rs:12",
        "analyze  <- src/cli.rs:7",
      ],
    },
    "empty_arc": {
      from: "x",
      to: "y",
      usages: [],
    },
  },
};

describe("SidebarLogic", () => {
  describe("parseUsages", () => {
    test("single symbol with one location", () => {
      const result = SidebarLogic.parseUsages(["ModuleInfo  <- src/cli.rs:7"]);
      expect(result).toEqual([
        { symbol: "ModuleInfo", locations: ["src/cli.rs:7"] },
      ]);
    });

    test("single symbol with continuation lines", () => {
      const result = SidebarLogic.parseUsages([
        "ModuleInfo  <- src/cli.rs:7",
        "             <- src/render.rs:12",
      ]);
      expect(result).toEqual([
        { symbol: "ModuleInfo", locations: ["src/cli.rs:7", "src/render.rs:12"] },
      ]);
    });

    test("multiple symbols", () => {
      const result = SidebarLogic.parseUsages([
        "ModuleInfo  <- src/cli.rs:7",
        "             <- src/render.rs:12",
        "analyze  <- src/cli.rs:7",
      ]);
      expect(result).toEqual([
        { symbol: "ModuleInfo", locations: ["src/cli.rs:7", "src/render.rs:12"] },
        { symbol: "analyze", locations: ["src/cli.rs:7"] },
      ]);
    });

    test("empty input", () => {
      expect(SidebarLogic.parseUsages([])).toEqual([]);
      expect(SidebarLogic.parseUsages(undefined)).toEqual([]);
      expect(SidebarLogic.parseUsages(null)).toEqual([]);
    });

    test("bare locations (no symbol prefix)", () => {
      const result = SidebarLogic.parseUsages([
        "  <- src/lib.rs:1",
      ]);
      expect(result).toEqual([
        { symbol: "", locations: ["src/lib.rs:1"] },
      ]);
    });
  });

  describe("buildContent", () => {
    test("header shows from → to from STATIC_DATA", () => {
      const html = SidebarLogic.buildContent("crate_a-crate_b");
      expect(html).toContain("crate_a");
      expect(html).toContain("crate_b");
      expect(html).toContain("sidebar-header");
    });

    test("contains close button", () => {
      const html = SidebarLogic.buildContent("crate_a-crate_b");
      expect(html).toContain("sidebar-close");
      expect(html).toContain("&#x2715;");
    });

    test("contains usage groups", () => {
      const html = SidebarLogic.buildContent("crate_a-crate_b");
      expect(html).toContain("sidebar-usage-group");
      expect(html).toContain("sidebar-symbol");
      expect(html).toContain("ModuleInfo");
      expect(html).toContain("src/cli.rs:7");
    });

    test("empty usages shows message", () => {
      const html = SidebarLogic.buildContent("empty_arc");
      expect(html).toContain("sidebar-header");
      expect(html).toContain("No usages");
    });

    test("uses overrideData instead of STATIC_DATA when provided", () => {
      const override = {
        from: "parent_crate",
        to: "dep_crate",
        usages: ["VirtSymbol  <- src/virt.rs:42"],
      };
      const html = SidebarLogic.buildContent("nonexistent-id", override);
      expect(html).toContain("parent_crate");
      expect(html).toContain("dep_crate");
      expect(html).toContain("VirtSymbol");
      expect(html).toContain("src/virt.rs:42");
    });

    test("overrideData with empty usages shows No usages", () => {
      const override = { from: "a", to: "b", usages: [] };
      const html = SidebarLogic.buildContent("whatever", override);
      expect(html).toContain("No usages");
    });
  });

  describe("show/hide/isVisible", () => {
    let fakeEl;

    function makeSvgMock(rectTop) {
      return {
        getBoundingClientRect() {
          return { left: 0, top: rectTop ?? 0, width: 1000, height: 800 };
        },
        viewBox: { baseVal: { width: 2000, height: 1600 } },
      };
    }

    beforeEach(() => {
      fakeEl = createFakeElement("foreignObject");
      fakeEl.innerHTML = "";
      const innerDiv = createFakeElement("div");
      innerDiv._innerHTML = "";
      Object.defineProperty(innerDiv, "innerHTML", {
        get() { return this._innerHTML; },
        set(v) { this._innerHTML = v; },
      });
      fakeEl._innerDiv = innerDiv;
      fakeEl.querySelector = () => fakeEl._innerDiv;
      const svgMock = makeSvgMock(0);
      globalThis.DomAdapter = {
        getElementById(id) {
          if (id === "relation-sidebar") return fakeEl;
          return null;
        },
        getSvgRoot() { return svgMock; },
        querySelector(sel) { if (sel === "svg") return svgMock; return null; },
        querySelectorAll() { return []; },
      };
      globalThis.window = globalThis.window || {};
      globalThis.window.innerWidth = 1000;
      globalThis.window.innerHeight = 800;
    });

    test("show sets display to block and sets content", () => {
      SidebarLogic.show("crate_a-crate_b");
      expect(fakeEl.style.display).toBe("block");
      expect(fakeEl._innerDiv.innerHTML).toContain("sidebar-header");
    });

    test("hide sets display to none", () => {
      SidebarLogic.show("crate_a-crate_b");
      SidebarLogic.hide();
      expect(fakeEl.style.display).toBe("none");
    });

    test("isVisible returns correct state", () => {
      expect(SidebarLogic.isVisible()).toBe(false);
      SidebarLogic.show("crate_a-crate_b");
      expect(SidebarLogic.isVisible()).toBe(true);
      SidebarLogic.hide();
      expect(SidebarLogic.isVisible()).toBe(false);
    });
  });

  describe("updatePosition", () => {
    test("positions right of arcs with fallback to viewport edge", () => {
      const fakeEl = createFakeElement("foreignObject");
      const innerDiv = createFakeElement("div");
      Object.defineProperty(innerDiv, "innerHTML", {
        get() { return this._innerHTML || ""; },
        set(v) { this._innerHTML = v; },
      });
      fakeEl.querySelector = () => innerDiv;
      const svgMock = {
        getBoundingClientRect() {
          return { left: 0, top: -300, width: 1000, height: 800 };
        },
        viewBox: { baseVal: { width: 2000, height: 1600 } },
      };
      globalThis.DomAdapter = {
        getElementById(id) {
          if (id === "relation-sidebar") return fakeEl;
          return null;
        },
        getSvgRoot() { return svgMock; },
        querySelectorAll() { return []; },
      };
      globalThis.window = globalThis.window || {};
      globalThis.window.innerWidth = 1000;
      globalThis.window.innerHeight = 800;
      SidebarLogic.show("crate_a-crate_b");
      // No arcs → maxArcRight=0, x=0+24=24
      // viewportRight = (1000-0)*2 = 2000, 24+280=304 < 2000 → no fallback
      expect(fakeEl.getAttribute("x")).toBe("24");
      // scaleY = 1600/800 = 2, scrollTop = max(0,300)*2 = 600
      // y = 600 + TOOLBAR_HEIGHT(0 in test) + GAP_TOP(20) = 620
      expect(fakeEl.getAttribute("y")).toBe("620");
    });

    test("falls back to viewport edge when arcs are too wide", () => {
      const fakeEl = createFakeElement("foreignObject");
      const innerDiv = createFakeElement("div");
      Object.defineProperty(innerDiv, "innerHTML", {
        get() { return this._innerHTML || ""; },
        set(v) { this._innerHTML = v; },
      });
      fakeEl.querySelector = () => innerDiv;
      const svgMock = {
        getBoundingClientRect() {
          return { left: 0, top: 0, width: 1000, height: 800 };
        },
        viewBox: { baseVal: { width: 2000, height: 1600 } },
      };
      // Mock an arc at x=1800, width=100 → right edge at 1900
      const fakeArc = { style: { display: '' }, getBBox() { return { x: 1800, width: 100 }; } };
      globalThis.DomAdapter = {
        getElementById(id) {
          if (id === "relation-sidebar") return fakeEl;
          return null;
        },
        getSvgRoot() { return svgMock; },
        querySelectorAll() { return [fakeArc]; },
      };
      globalThis.window = globalThis.window || {};
      globalThis.window.innerWidth = 1000;
      globalThis.window.innerHeight = 800;
      SidebarLogic.show("crate_a-crate_b");
      // maxArcRight=1900, x=1900+24=1924
      // viewportRight = (1000-0)*2 = 2000, 1924+280=2204 > 2000
      // fallback: x = 2000-280-16 = 1704
      expect(fakeEl.getAttribute("x")).toBe("1704");
    });

    test("height is capped at MAX_HEIGHT", () => {
      const fakeEl = createFakeElement("foreignObject");
      const innerDiv = createFakeElement("div");
      Object.defineProperty(innerDiv, "innerHTML", {
        get() { return this._innerHTML || ""; },
        set(v) { this._innerHTML = v; },
      });
      fakeEl.querySelector = () => innerDiv;
      // Large viewport: innerHeight=2000 * scaleY=2 = 4000 SVG units
      const svgMock = {
        getBoundingClientRect() {
          return { left: 0, top: 0, width: 1000, height: 800 };
        },
        viewBox: { baseVal: { width: 2000, height: 1600 } },
      };
      globalThis.DomAdapter = {
        getElementById(id) {
          if (id === "relation-sidebar") return fakeEl;
          return null;
        },
        getSvgRoot() { return svgMock; },
        querySelectorAll() { return []; },
      };
      globalThis.window = globalThis.window || {};
      globalThis.window.innerWidth = 1000;
      globalThis.window.innerHeight = 2000;
      SidebarLogic.show("crate_a-crate_b");
      // vpHeight = 2000 * (1600/800) = 4000, 4000 - 0 - 20 = 3980, capped at 500
      // Inner div gets content height, foreignObject gets +12 for shadow padding
      expect(parseInt(innerDiv.style.height)).toBeLessThanOrEqual(500);
    });
  });
});
