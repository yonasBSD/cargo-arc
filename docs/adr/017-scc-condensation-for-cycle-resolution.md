# ADR-017: SCC Condensation for Deterministic Cycle Resolution

- **Status:** Active
- **Decided:** 2026-02-11

## Context

`topo_sort` failed on cycles from mixed edge types: a `CrateDep` forward and a `ModuleDep` backward together form a cycle. Simple heuristics (crate-level propagation, fallback sorting) do not solve this reliably.

## Decision

We apply Tarjan's SCC (Strongly Connected Components) algorithm a second time — on the already condensed graph. The resulting DAG (Directed Acyclic Graph) can be trivially topologically sorted, and we expand the SCCs alphabetically.

We resolve mixed-edge cycles silently, without visualizing them: they arise from combining two analysis levels (crate + module), not from real circular dependencies.

## Rationale

- Condensation mathematically guarantees a DAG — no heuristic
- Alphabetical expansion within SCCs produces deterministic output, independent of hash ordering (cf. ADR-001: Deterministic Layout)
- These cycles are analysis artifacts, not an architecture problem — visualizing them would be misleading

## Consequences

### Positive
- Stable sorting for arbitrary workspace topologies

### Negative
- Double SCC computation, though performance overhead is negligible
- Real module-level cycles within an SCC are resolved along with the false positives
