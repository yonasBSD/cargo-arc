# ADR-011: Separate State and Derived Data in Frontend

- **Status:** Active
- **Decided:** 2026-01-29

## Context

The DOM served simultaneously as state container and view. Code read e.g. `arc.style.strokeWidth` as the "original" — but the value could already have been changed by highlighting. Recurring bugs: highlight growth, timing dependencies between clear/apply.

## Decision

Three-layer model for the JavaScript frontend architecture:

1. **Static** (immutable, from Rust via `STATIC_DATA`): graph structure, positions, styles
2. **State** (user interaction): only `collapsed: Set<nodeId>` + `selection: {mode, type, id}`
3. **DerivedState** (computed via pure functions from Static+State): visibility, highlights, virtual arcs

The DOM is never read as a data source. DerivedState is recomputed on every state change.

## Rationale

- We limit state to two fields — this reduces the bug surface
- We compute DerivedState as pure functions that we can test deterministically
- Selection needs only one ID — dependencies/dependents are derived from StaticData

## Consequences

### Positive
- No divergence between data and presentation
- DerivedState functions can be tested with pure unit tests
- Highlight growth bugs no longer occur because the DOM never serves as a data source

### Negative
- Every state change triggers a full DerivedState recomputation
