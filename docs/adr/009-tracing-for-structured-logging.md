# ADR-009: Use tracing for Structured Logging

- **Status:** Active
- **Decided:** 2026-01-26

## Context

Debug output consisted of scattered `eprintln!()` calls without context, filterability, or structured fields.

## Decision

We log via the `tracing` crate with `#[instrument]` attribute for automatic function context. The CLI flag `--debug` activates debug level.

## Rationale

- `tracing` provides `#[instrument]` (automatic span with function name and parameters) — this was the deciding factor over `log`
- Structured fields instead of string interpolation

## Consequences

### Positive
- Filterable, context-rich debug output
- `#[instrument]` reduces boilerplate

### Negative
- Additional dependency (tracing + tracing-subscriber)
