# Architecture

## Project Structure

```
src/
├── analyze.rs   # Workspace & module extraction (cargo_metadata + ra_ap_hir)
├── graph.rs     # Dependency graph building (petgraph)
├── layout.rs    # Tree layout algorithm
├── render.rs    # SVG generation
├── cli.rs       # CLI interface (clap)
├── lib.rs       # Public API exports
└── main.rs      # Entry point
```
