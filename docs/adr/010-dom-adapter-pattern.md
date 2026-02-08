# ADR-010: Encapsulate DOM Access behind DomAdapter

- **Status:** Active
- **Decided:** 2026-01-27

## Context

`svg_script.js` was hard to test: DOM manipulation and business logic were intertwined. Pure function extraction alone would have made only a small portion of the code testable, since the majority contains DOM accesses.

## Decision

We encapsulate all DOM accesses behind a `DomAdapter` interface. `createMockDomAdapter()` for tests (call tracking), `DomAdapter` for the browser (delegates to `document.*`).

## Rationale

- Mock injection makes the majority of the code testable, not just pure functions
- Unit tests run with Bun/Node without a browser
- Lightweight: fake elements instead of a full JSDOM

## Consequences

### Positive
- Significantly more code is unit-testable
- Tests are fast (no browser environment)

### Negative
- Adapter must be extended for new DOM operations
- Mocks could diverge from real DOM behavior
