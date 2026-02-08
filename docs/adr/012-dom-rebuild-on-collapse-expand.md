# ADR-012: Rebuild DOM on Collapse Instead of Hide/Show

- **Status:** Active
- **Decided:** 2026-01-29

## Context

When collapsing a node, virtual arcs (aggregated edges for hidden child nodes) needed updating. The original approach hid elements via `display: none` and tried to preserve their state. Hidden elements accumulated stale state (e.g. highlight stroke widths), and inconsistencies appeared on expand.

## Decision

We delete all dynamic elements (virtual arcs, labels, hitareas) on every collapse/expand and recreate them from `STATIC_DATA` + current collapse state. Delete/recreate instead of hide/show.

The flow on each state change:

1. `cleanupVirtualElements()` — remove all dynamic DOM elements
2. `DerivedState.deriveNodeVisibility()` — compute visible nodes
3. `renderVirtualElements()` — create new elements from computed data

## Rationale

- No accumulated state in DOM elements — each element is freshly computed from `STATIC_DATA`
- No timing dependencies between clear and apply — the rebuild is idempotent and self-contained
- Complements ADR-011 (State+Derived separation): state change -> derive -> render is the consistent data flow

## Consequences

### Positive
- No hidden state bits in DOM elements
- Idempotent: each render produces the same result for the same state
- Highlight reapplication after rebuild is reliable (no stale values)

### Negative
- CPU cost for delete/recreate on every collapse/expand
- Pinned highlights must be explicitly reapplied after rebuild
