# ADR-007: Filter Features via cargo_metadata resolve

- **Status:** Active
- **Decided:** 2026-01-24

## Context

cargo_metadata offers two ways to read dependencies: `packages[].dependencies` (static declaration from Cargo.toml) and `resolve.nodes[].deps` (resolved graph after feature resolution). Static dependencies contain all possible dependencies regardless of active features.

## Decision

We extract dependencies from the resolve section of cargo_metadata. `resolve.nodes[].deps` provides the actually resolved dependency graph after feature activation.

## Rationale

- Cargo already performs feature resolution — we use the result
- Verified on the test project: `--features server` yields different dependencies than `--features web` — resolve reflects this correctly
- Static dependencies would ignore features and show incorrect edges

## Consequences

### Positive
- Correct representation of feature-dependent dependencies
- No custom feature resolution logic needed

### Negative
- Dependency on cargo_metadata resolve format
- Unresolved (optional) dependencies are invisible
