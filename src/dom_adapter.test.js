import { test, expect, describe } from "bun:test";
import { createFakeElement, createMockDomAdapter, Selectors } from "./dom_adapter.js";

describe("createFakeElement", () => {
  test("setAttribute/getAttribute roundtrip", () => {
    const el = createFakeElement("rect");
    el.setAttribute("id", "my-rect");
    el.setAttribute("width", "100");
    expect(el.getAttribute("id")).toBe("my-rect");
    expect(el.getAttribute("width")).toBe("100");
  });

  test("classList.add/contains/remove", () => {
    const el = createFakeElement("g");
    expect(el.classList.contains("active")).toBe(false);
    el.classList.add("active");
    expect(el.classList.contains("active")).toBe(true);
    el.classList.remove("active");
    expect(el.classList.contains("active")).toBe(false);
  });

  test("style property get/set", () => {
    const el = createFakeElement("path");
    el.style.strokeWidth = "5px";
    el.style.fill = "red";
    expect(el.style.strokeWidth).toBe("5px");
    expect(el.style.fill).toBe("red");
  });

  test("appendChild/removeChild", () => {
    const parent = createFakeElement("g");
    const child = createFakeElement("rect");
    parent.appendChild(child);
    expect(parent.children).toContain(child);
    parent.removeChild(child);
    expect(parent.children).not.toContain(child);
  });
});

describe("createMockDomAdapter", () => {
  test("getElementById tracks calls", () => {
    const mock = createMockDomAdapter();
    mock.getElementById("foo");
    mock.getElementById("bar");
    expect(mock._getCalls("getElementById")).toEqual([["foo"], ["bar"]]);
  });

  test("_registerElement makes getElementById return element", () => {
    const mock = createMockDomAdapter();
    const el = createFakeElement("g");
    mock._registerElement("my-id", el);
    expect(mock.getElementById("my-id")).toBe(el);
  });

  test("querySelector, querySelectorAll, createSvgElement track calls", () => {
    const mock = createMockDomAdapter();
    mock.querySelector(".node");
    mock.querySelectorAll("rect");
    const svgEl = mock.createSvgElement("path");
    expect(mock._getCalls("querySelector")).toEqual([[".node"]]);
    expect(mock._getCalls("querySelectorAll")).toEqual([["rect"]]);
    expect(mock._getCalls("createSvgElement")).toEqual([["path"]]);
    expect(svgEl.tagName).toBe("path");
  });
});

describe("Convenience methods", () => {
  test("getNode uses Selectors.nodeId", () => {
    const mock = createMockDomAdapter();
    const el = createFakeElement("rect");
    mock._registerElement("node-foo", el);
    expect(mock.getNode("foo")).toBe(el);
    expect(mock._getCalls("getElementById")).toContainEqual(["node-foo"]);
  });

  test("getVisibleArc uses Selectors.visibleArc", () => {
    const mock = createMockDomAdapter();
    const el = createFakeElement("path");
    mock._registerSelector(Selectors.visibleArc("a-b"), el);
    expect(mock.getVisibleArc("a-b")).toBe(el);
    expect(mock._getCalls("querySelector")).toContainEqual([Selectors.visibleArc("a-b")]);
  });

  test("getHitarea uses Selectors.hitarea", () => {
    const mock = createMockDomAdapter();
    const el = createFakeElement("path");
    mock._registerSelector(Selectors.hitarea("x-y"), el);
    expect(mock.getHitarea("x-y")).toBe(el);
    expect(mock._getCalls("querySelector")).toContainEqual([Selectors.hitarea("x-y")]);
  });

  test("getArrows uses Selectors.arrows", () => {
    const mock = createMockDomAdapter();
    const arrows = [createFakeElement("polygon"), createFakeElement("polygon")];
    mock._registerSelector(Selectors.arrows("e-id"), arrows);
    expect(mock.getArrows("e-id")).toEqual(arrows);
    expect(mock._getCalls("querySelectorAll")).toContainEqual([Selectors.arrows("e-id")]);
  });

  test("getVirtualArrows uses Selectors.virtualArrows", () => {
    const mock = createMockDomAdapter();
    const arrows = [createFakeElement("polygon")];
    mock._registerSelector(Selectors.virtualArrows("v-id"), arrows);
    expect(mock.getVirtualArrows("v-id")).toEqual(arrows);
  });

  test("getConnectedHitareas uses Selectors.connectedHitareas", () => {
    const mock = createMockDomAdapter();
    const hitareas = [createFakeElement("path"), createFakeElement("path")];
    mock._registerSelector(Selectors.connectedHitareas("node1"), hitareas);
    expect(mock.getConnectedHitareas("node1")).toEqual(hitareas);
  });

  test("getLabelGroup uses Selectors.labelGroup", () => {
    const mock = createMockDomAdapter();
    const group = createFakeElement("g");
    mock._registerSelector(Selectors.labelGroup("arc-1"), group);
    expect(mock.getLabelGroup("arc-1")).toBe(group);
  });
});
