import { test, expect, describe } from "bun:test";
import { createFakeElement, createMockDomAdapter } from "./dom_adapter.js";

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
