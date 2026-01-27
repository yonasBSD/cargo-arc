function createFakeElement(tagName) {
  const attrs = new Map();
  const classes = new Set();
  const styleData = {};
  const children = [];
  return {
    tagName,
    children,
    getAttribute(name) { return attrs.get(name) ?? null; },
    setAttribute(name, value) { attrs.set(name, value); },
    classList: {
      add(c) { classes.add(c); },
      remove(c) { classes.delete(c); },
      contains(c) { return classes.has(c); },
    },
    style: new Proxy(styleData, {
      get(target, prop) { return target[prop]; },
      set(target, prop, value) { target[prop] = value; return true; },
    }),
    appendChild(child) { children.push(child); },
    removeChild(child) {
      const idx = children.indexOf(child);
      if (idx !== -1) children.splice(idx, 1);
    },
  };
}

function createMockDomAdapter() {
  const calls = new Map();
  const elements = new Map();
  function track(method, args) {
    if (!calls.has(method)) calls.set(method, []);
    calls.get(method).push([...args]);
  }
  return {
    getElementById(id) { track("getElementById", [id]); return elements.get(id) ?? null; },
    querySelector(sel) { track("querySelector", [sel]); return null; },
    querySelectorAll(sel) { track("querySelectorAll", [sel]); return []; },
    createSvgElement(tag) { track("createSvgElement", [tag]); return createFakeElement(tag); },
    _getCalls(method) { return calls.get(method) ?? []; },
    _registerElement(id, el) { elements.set(id, el); },
  };
}

const SVG_NS = "http://www.w3.org/2000/svg";

const DomAdapter = {
  getElementById(id) { return document.getElementById(id); },
  querySelector(sel) { return document.querySelector(sel); },
  querySelectorAll(sel) { return document.querySelectorAll(sel); },
  createSvgElement(tag) { return document.createElementNS(SVG_NS, tag); },
};

// Export for Browser
if (typeof window !== "undefined") {
  window.DomAdapter = DomAdapter;
}

// Export for Bun/Node
if (typeof module !== "undefined" && module.exports) {
  module.exports = { DomAdapter, createMockDomAdapter, createFakeElement };
}
