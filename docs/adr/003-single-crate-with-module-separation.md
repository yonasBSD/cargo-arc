# ADR-003: Develop as Single Crate with Module Separation

- **Status:** Active
- **Decided:** 2026-01-20

## Context

Module boundaries vs. crate boundaries: A multi-crate workspace (e.g., analyze/render/cli) enforces stable public APIs between crates. In the current development phase, internal structures change frequently — modules are extracted, merged, APIs relocated.

## Decision

We develop cargo-arc as a single crate with internal module structure (`analyze/`, `graph.rs`, `layout.rs`, `render.rs`, `cli.rs`) and forgo a multi-crate workspace for now.

## Rationale

- Exploration phase: Features and abstractions need to settle before being cemented into crate boundaries
- Modules are easier to refactor than crates (no API contract between crates)
- Crate split remains an option (`analyze/` has no imports from `render.rs`)

## Consequences

### Positive
- Fast iteration without cross-crate API stability
- Simple build setup (one `cargo build`)

### Negative
- No parallel compilation across crate boundaries (single compilation unit)
- Later crate split requires refactoring
