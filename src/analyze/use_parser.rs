//! Text-based use statement parsing for workspace dependency extraction.

use crate::model::DependencyRef;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(crate) fn normalize_crate_name(name: &str) -> String {
    name.replace('-', "_")
}

/// Find the longest module path prefix from `parts` that exists in `module_paths`.
///
/// Tries from longest to shortest: `["analyze", "use_parser", "normalize"]`
/// checks `"analyze::use_parser"`, then `"analyze"`.
/// Returns `(matched_path, segment_count)`.
/// Fallback: first segment with count 1.
fn find_longest_module_prefix(parts: &[&str], module_paths: &HashSet<String>) -> (String, usize) {
    for end in (1..=parts.len()).rev() {
        let candidate: String = parts[..end].join("::");
        if module_paths.contains(&candidate) {
            return (candidate, end);
        }
    }
    // Fallback: first segment
    (parts[0].to_string(), 1)
}

pub(super) fn is_workspace_member<S: AsRef<str>>(
    name: &str,
    workspace_crates: &HashSet<S>,
) -> bool {
    let normalized = normalize_crate_name(name);
    workspace_crates
        .iter()
        .any(|ws| normalize_crate_name(ws.as_ref()) == normalized)
}

/// Extract the use path from a line like `use crate::module;` → `crate::module`
/// Also handles `pub use`, `pub(crate) use`, `pub(super) use`, `pub(in path) use`.
fn extract_use_path(line: &str) -> Option<&str> {
    let line = line.trim();
    let use_pos = line.find("use ")?;
    let before = line[..use_pos].trim();
    // Only accept if nothing before `use` or a pub-modifier
    if !before.is_empty() && !before.starts_with("pub") {
        return None;
    }
    let after = &line[use_pos + 4..];
    Some(after.trim_end_matches(';').trim())
}

/// Extract an item from path parts at given index, handling trailing `{` and empty strings.
fn extract_item_from_parts(parts: &[&str], index: usize) -> Option<String> {
    let part = parts
        .get(index)?
        .trim_end_matches('{')
        .trim_end_matches(';')
        .trim();
    if part.is_empty() || part.starts_with('{') {
        None
    } else {
        Some(part.to_string())
    }
}

/// Parse crate-local imports: `use crate::module[::item]`
fn parse_crate_local_import(
    path: &str,
    current_crate: &str,
    source_file: &Path,
    line_num: usize,
    all_module_paths: &HashMap<String, HashSet<String>>,
) -> Option<DependencyRef> {
    let after_crate = path.strip_prefix("crate::")?;
    let parts: Vec<&str> = after_crate.split("::").collect();

    let first = parts.first()?.trim_end_matches('{').trim();
    if first.is_empty() {
        return None;
    }

    let empty_set = HashSet::new();
    let module_paths = all_module_paths
        .get(&normalize_crate_name(current_crate))
        .unwrap_or(&empty_set);
    let (target_module, prefix_len) = find_longest_module_prefix(&parts, module_paths);

    Some(DependencyRef {
        target_crate: normalize_crate_name(current_crate),
        target_module,
        target_item: extract_item_from_parts(&parts, prefix_len),
        source_file: source_file.to_path_buf(),
        line: line_num,
    })
}

/// Parse bare module imports: `use cli::Args` where `cli` is a known module of the current crate.
/// Rust 2018+ resolves bare paths from any file, not just the crate root.
fn parse_bare_module_import(
    path: &str,
    current_crate: &str,
    source_file: &Path,
    line_num: usize,
    all_module_paths: &HashMap<String, HashSet<String>>,
) -> Option<DependencyRef> {
    let parts: Vec<&str> = path.split("::").collect();
    let first = parts.first()?.trim_end_matches('{').trim();
    if first.is_empty() {
        return None;
    }

    let empty_set = HashSet::new();
    let module_paths = all_module_paths
        .get(&normalize_crate_name(current_crate))
        .unwrap_or(&empty_set);

    if !module_paths.contains(first) {
        return None;
    }

    let (target_module, prefix_len) = find_longest_module_prefix(&parts, module_paths);

    Some(DependencyRef {
        target_crate: normalize_crate_name(current_crate),
        target_module,
        target_item: extract_item_from_parts(&parts, prefix_len),
        source_file: source_file.to_path_buf(),
        line: line_num,
    })
}

/// Parse workspace crate imports: `use other_crate::module[::item]`
fn parse_workspace_import(
    path: &str,
    workspace_crates: &HashSet<String>,
    source_file: &Path,
    line_num: usize,
    all_module_paths: &HashMap<String, HashSet<String>>,
    crate_exports: &HashMap<String, HashSet<String>>,
) -> Option<DependencyRef> {
    let parts: Vec<&str> = path.split("::").collect();
    let crate_name = parts.first()?.trim();

    if !is_workspace_member(crate_name, workspace_crates) || parts.len() < 2 {
        return None;
    }

    let module_segment = parts[1].trim_end_matches('{').trim_end_matches(';').trim();
    if module_segment.is_empty() {
        return None;
    }

    let empty_set = HashSet::new();
    let target_crate_name = normalize_crate_name(crate_name);
    let module_paths = all_module_paths
        .get(&target_crate_name)
        .unwrap_or(&empty_set);
    let (target_module, prefix_len) = find_longest_module_prefix(&parts[1..], module_paths);

    // Entry-point detection: if the resolved target_module is not a known module
    // and the first segment after the crate name is a known export, treat it as
    // an entry-point dependency (target_module = "").
    if !module_paths.contains(&target_module)
        && crate_exports
            .get(&target_crate_name)
            .is_some_and(|e| e.contains(module_segment))
    {
        return Some(DependencyRef {
            target_crate: crate_name.to_string(),
            target_module: String::new(),
            target_item: Some(module_segment.to_string()),
            source_file: source_file.to_path_buf(),
            line: line_num,
        });
    }

    Some(DependencyRef {
        target_crate: crate_name.to_string(),
        target_module,
        target_item: extract_item_from_parts(&parts, 1 + prefix_len),
        source_file: source_file.to_path_buf(),
        line: line_num,
    })
}

/// Process a single use statement line, returning a DependencyRef if it's a relevant import.
///
/// Handles:
/// - `use crate::module;` - crate-local imports
/// - `use crate::module::item;` - crate-local item imports
/// - `use workspace_crate::module;` - workspace crate imports (when in workspace_crates set)
///
/// Returns None for:
/// - `use self::*` or `use super::*` - relative imports
/// - External crate imports (not in workspace_crates)
fn process_use_statement(
    line: &str,
    line_num: usize,
    current_crate: &str,
    workspace_crates: &HashSet<String>,
    source_file: &Path,
    all_module_paths: &HashMap<String, HashSet<String>>,
    crate_exports: &HashMap<String, HashSet<String>>,
) -> Option<DependencyRef> {
    let path = extract_use_path(line)?;

    parse_crate_local_import(path, current_crate, source_file, line_num, all_module_paths)
        .or_else(|| {
            parse_bare_module_import(path, current_crate, source_file, line_num, all_module_paths)
        })
        .or_else(|| {
            parse_workspace_import(
                path,
                workspace_crates,
                source_file,
                line_num,
                all_module_paths,
                crate_exports,
            )
        })
}

/// Process a use statement that may contain multiple symbols (`{A, B, C}`) or glob (`*`).
/// Returns a Vec of DependencyRefs, one per symbol.
///
/// Handles:
/// - `use crate::module::{A, B, C}` → 3 DependencyRefs
/// - `use crate::module::*` → 1 DependencyRef with target_item = "*"
/// - `use crate::module::Item` → 1 DependencyRef (simple import)
fn process_use_statement_multi(
    line: &str,
    line_num: usize,
    current_crate: &str,
    workspace_crates: &HashSet<String>,
    source_file: &Path,
    all_module_paths: &HashMap<String, HashSet<String>>,
    crate_exports: &HashMap<String, HashSet<String>>,
) -> Vec<DependencyRef> {
    let path = match extract_use_path(line) {
        Some(p) => p,
        None => return vec![],
    };

    // Check for multi-symbol import: `use path::{A, B, C}`
    if let Some(brace_start) = path.find('{')
        && let Some(brace_end) = path.find('}')
    {
        let base_path = path[..brace_start].trim_end_matches(':');
        let symbols_str = &path[brace_start + 1..brace_end];
        let symbols: Vec<&str> = symbols_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        // Parse base path to get crate and module
        if let Some((target_crate, target_module)) = parse_base_path(
            base_path,
            current_crate,
            workspace_crates,
            all_module_paths,
            crate_exports,
        ) {
            return symbols
                .into_iter()
                .map(|sym| DependencyRef {
                    target_crate: target_crate.clone(),
                    target_module: target_module.clone(),
                    target_item: Some(sym.to_string()),
                    source_file: source_file.to_path_buf(),
                    line: line_num,
                })
                .collect();
        }
        return vec![];
    }

    // Check for glob import: `use path::*`
    if path.ends_with("::*") {
        let base_path = path.trim_end_matches("::*");
        if let Some((target_crate, target_module)) = parse_base_path(
            base_path,
            current_crate,
            workspace_crates,
            all_module_paths,
            crate_exports,
        ) {
            return vec![DependencyRef {
                target_crate,
                target_module,
                target_item: Some("*".to_string()),
                source_file: source_file.to_path_buf(),
                line: line_num,
            }];
        }
        return vec![];
    }

    // Fall back to simple import
    if let Some(dep) = process_use_statement(
        line,
        line_num,
        current_crate,
        workspace_crates,
        source_file,
        all_module_paths,
        crate_exports,
    ) {
        return vec![dep];
    }

    vec![]
}

/// Parse a base path (before `::*` or `::{...}`) into (crate, module).
fn parse_base_path(
    base_path: &str,
    current_crate: &str,
    workspace_crates: &HashSet<String>,
    all_module_paths: &HashMap<String, HashSet<String>>,
    crate_exports: &HashMap<String, HashSet<String>>,
) -> Option<(String, String)> {
    let empty_set = HashSet::new();

    // Handle crate-local: `crate::module`
    if let Some(after_crate) = base_path.strip_prefix("crate::") {
        let parts: Vec<&str> = after_crate.split("::").collect();
        if parts.is_empty() || parts[0].is_empty() {
            return None;
        }
        let module_paths = all_module_paths
            .get(&normalize_crate_name(current_crate))
            .unwrap_or(&empty_set);
        let (module, _prefix_len) = find_longest_module_prefix(&parts, module_paths);
        return Some((normalize_crate_name(current_crate), module));
    }

    // Handle bare module paths: `cli` or `cli::sub` where `cli` is own module
    let parts: Vec<&str> = base_path.split("::").collect();
    let first = parts[0].trim();
    let module_paths = all_module_paths
        .get(&normalize_crate_name(current_crate))
        .unwrap_or(&empty_set);
    if !first.is_empty() && module_paths.contains(first) {
        let (module, _) = find_longest_module_prefix(&parts, module_paths);
        return Some((normalize_crate_name(current_crate), module));
    }

    // Handle workspace crate imports
    // Crate-root imports: `use other_crate::{Foo, Bar}` or `use other_crate::*`
    // base_path is just "other_crate" (no :: after crate name)
    if parts.len() == 1 {
        let segment = parts[0].trim();
        if is_workspace_member(segment, workspace_crates) {
            return Some((segment.to_string(), String::new()));
        }
    }

    if parts.len() >= 2 {
        let first_segment = parts[0].trim();
        let is_workspace_crate = is_workspace_member(first_segment, workspace_crates);

        if is_workspace_crate {
            let target_crate = normalize_crate_name(first_segment);
            let module_paths = all_module_paths.get(&target_crate).unwrap_or(&empty_set);
            let (module, _prefix_len) = find_longest_module_prefix(&parts[1..], module_paths);

            // Entry-point detection: if resolved module is not a known module
            // and the segment is a known export, return entry-point (empty module)
            if !module_paths.contains(&module) {
                let segment = parts[1].trim();
                if crate_exports
                    .get(&target_crate)
                    .is_some_and(|e| e.contains(segment))
                {
                    return Some((first_segment.to_string(), String::new()));
                }
            }

            return Some((first_segment.to_string(), module));
        }
    }

    None
}

/// Parse use statements from source code, extracting workspace-relevant dependencies.
///
/// Returns DependencyRefs for:
/// - Crate-local imports (`use crate::module`)
/// - Workspace crate imports (`use other_crate::module` where other_crate is in workspace)
///
/// Deduplicates by full_target() to keep distinct symbols but avoid duplicates.
pub(crate) fn parse_workspace_dependencies(
    source: &str,
    current_crate: &str,
    workspace_crates: &HashSet<String>,
    source_file: &Path,
    all_module_paths: &HashMap<String, HashSet<String>>,
    crate_exports: &HashMap<String, HashSet<String>>,
) -> Vec<DependencyRef> {
    let mut deps: Vec<DependencyRef> = Vec::new();
    let mut seen_targets: HashSet<String> = HashSet::new();

    for (line_idx, line) in source.lines().enumerate() {
        let line_num = line_idx + 1;
        for dep in process_use_statement_multi(
            line,
            line_num,
            current_crate,
            workspace_crates,
            source_file,
            all_module_paths,
            crate_exports,
        ) {
            let target_key = dep.full_target();
            if !seen_targets.contains(&target_key) {
                seen_targets.insert(target_key);
                deps.push(dep);
            }
        }
    }

    deps
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    mod normalize_tests {
        use super::*;

        #[test]
        fn test_normalize_crate_name() {
            assert_eq!(normalize_crate_name("my-lib"), "my_lib");
            assert_eq!(normalize_crate_name("already_valid"), "already_valid");
            assert_eq!(normalize_crate_name("a-b-c"), "a_b_c");
        }

        #[test]
        fn test_process_use_statement_crate_local() {
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let dep = process_use_statement(
                "use crate::graph::build;",
                1,
                "my_crate",
                &ws,
                Path::new("src/cli.rs"),
                &mp,
                &HashMap::new(),
            );
            let dep = dep.expect("should parse crate-local import");
            assert_eq!(dep.target_crate, "my_crate");
            assert_eq!(dep.target_module, "graph");
            assert_eq!(dep.target_item, Some("build".to_string()));
        }

        #[test]
        fn test_process_use_statement_crate_local_module_only() {
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let dep = process_use_statement(
                "use crate::graph;",
                5,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            let dep = dep.expect("should parse crate-local module import");
            assert_eq!(dep.target_crate, "my_crate");
            assert_eq!(dep.target_module, "graph");
            assert!(dep.target_item.is_none());
            assert_eq!(dep.line, 5);
        }

        #[test]
        fn test_process_use_statement_workspace_crate() {
            let ws: HashSet<String> = HashSet::from(["other_crate".to_string()]);
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let dep = process_use_statement(
                "use other_crate::utils;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            let dep = dep.expect("should parse workspace crate import");
            assert_eq!(dep.target_crate, "other_crate");
            assert_eq!(dep.target_module, "utils");
        }

        #[test]
        fn test_process_use_statement_workspace_crate_with_hyphen() {
            let ws: HashSet<String> = HashSet::from(["my-lib".to_string()]);
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let dep = process_use_statement(
                "use my_lib::feature;",
                1,
                "app",
                &ws,
                Path::new("src/main.rs"),
                &mp,
                &HashMap::new(),
            );
            let dep = dep.expect("should parse workspace crate with hyphen");
            assert_eq!(dep.target_crate, "my_lib");
            assert_eq!(dep.target_module, "feature");
        }

        #[test]
        fn test_process_use_statement_relative_self_ignored() {
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let dep = process_use_statement(
                "use self::helper;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            assert!(dep.is_none(), "self:: imports should be ignored");
        }

        #[test]
        fn test_process_use_statement_relative_super_ignored() {
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let dep = process_use_statement(
                "use super::parent;",
                1,
                "my_crate",
                &ws,
                Path::new("src/sub/mod.rs"),
                &mp,
                &HashMap::new(),
            );
            assert!(dep.is_none(), "super:: imports should be ignored");
        }

        #[test]
        fn test_process_use_statement_external_filtered() {
            let ws: HashSet<String> = HashSet::from(["my_crate".to_string()]);
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let dep = process_use_statement(
                "use serde::Serialize;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            assert!(dep.is_none(), "external crate imports should be filtered");
        }

        #[test]
        fn test_process_use_statement_std_filtered() {
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let dep = process_use_statement(
                "use std::collections::HashMap;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            assert!(dep.is_none(), "std imports should be filtered");
        }
    }

    mod longest_prefix_tests {
        use super::*;

        #[test]
        fn test_find_longest_prefix_submodule() {
            let paths: HashSet<String> =
                HashSet::from(["analyze".into(), "analyze::use_parser".into()]);
            let (prefix, len) =
                find_longest_module_prefix(&["analyze", "use_parser", "normalize"], &paths);
            assert_eq!(prefix, "analyze::use_parser");
            assert_eq!(len, 2);
        }

        #[test]
        fn test_find_longest_prefix_parent_only() {
            let paths: HashSet<String> = HashSet::from(["analyze".into()]);
            let (prefix, len) = find_longest_module_prefix(&["analyze", "SomeItem"], &paths);
            assert_eq!(prefix, "analyze");
            assert_eq!(len, 1);
        }

        #[test]
        fn test_find_longest_prefix_no_match() {
            let paths: HashSet<String> = HashSet::from(["analyze".into()]);
            let (prefix, len) = find_longest_module_prefix(&["unknown", "item"], &paths);
            assert_eq!(prefix, "unknown");
            assert_eq!(len, 1);
        }

        #[test]
        fn test_find_longest_prefix_single_segment() {
            let paths: HashSet<String> = HashSet::from(["graph".into()]);
            let (prefix, len) = find_longest_module_prefix(&["graph"], &paths);
            assert_eq!(prefix, "graph");
            assert_eq!(len, 1);
        }

        #[test]
        fn test_find_longest_prefix_empty_module_paths() {
            let paths: HashSet<String> = HashSet::new();
            let (prefix, len) = find_longest_module_prefix(&["analyze", "foo"], &paths);
            assert_eq!(prefix, "analyze");
            assert_eq!(len, 1);
        }
    }

    mod submodule_tests {
        use super::*;

        #[test]
        fn test_crate_local_submodule() {
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> = HashMap::from([(
                "my_crate".to_string(),
                HashSet::from(["analyze".into(), "analyze::use_parser".into()]),
            )]);
            let dep = process_use_statement(
                "use crate::analyze::use_parser::normalize;",
                1,
                "my_crate",
                &ws,
                Path::new("src/cli.rs"),
                &mp,
                &HashMap::new(),
            );
            let dep = dep.expect("should parse crate-local submodule import");
            assert_eq!(dep.target_module, "analyze::use_parser");
            assert_eq!(dep.target_item, Some("normalize".to_string()));
        }

        #[test]
        fn test_workspace_import_submodule() {
            let ws: HashSet<String> = HashSet::from(["other_crate".to_string()]);
            let mp: HashMap<String, HashSet<String>> = HashMap::from([(
                "other_crate".to_string(),
                HashSet::from(["foo".into(), "foo::bar".into()]),
            )]);
            let dep = process_use_statement(
                "use other_crate::foo::bar::Baz;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            let dep = dep.expect("should parse workspace submodule import");
            assert_eq!(dep.target_module, "foo::bar");
            assert_eq!(dep.target_item, Some("Baz".to_string()));
        }

        #[test]
        fn test_workspace_import_cross_crate_deep() {
            let ws: HashSet<String> = HashSet::from(["other_crate".to_string()]);
            let mp: HashMap<String, HashSet<String>> = HashMap::from([(
                "other_crate".to_string(),
                HashSet::from(["sub".into(), "sub::deep".into()]),
            )]);
            let dep = process_use_statement(
                "use other_crate::sub::deep::Item;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            let dep = dep.expect("should parse deep cross-crate import");
            assert_eq!(dep.target_module, "sub::deep");
            assert_eq!(dep.target_item, Some("Item".to_string()));
        }

        #[test]
        fn test_cross_crate_no_paths_fallback() {
            let ws: HashSet<String> = HashSet::from(["other_crate".to_string()]);
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let dep = process_use_statement(
                "use other_crate::foo::bar::Baz;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            let dep = dep.expect("should parse with fallback");
            assert_eq!(dep.target_module, "foo");
            assert_eq!(dep.target_item, Some("bar".to_string()));
        }

        #[test]
        fn test_parse_base_path_with_submodule() {
            let mp: HashMap<String, HashSet<String>> = HashMap::from([(
                "my_crate".to_string(),
                HashSet::from(["analyze::use_parser".into()]),
            )]);
            let ws: HashSet<String> = HashSet::new();
            let result = parse_base_path(
                "crate::analyze::use_parser",
                "my_crate",
                &ws,
                &mp,
                &HashMap::new(),
            );
            assert_eq!(
                result,
                Some(("my_crate".to_string(), "analyze::use_parser".to_string()))
            );
        }

        #[test]
        fn test_parse_base_path_workspace_submodule() {
            let ws: HashSet<String> = HashSet::from(["other_crate".to_string()]);
            let mp: HashMap<String, HashSet<String>> = HashMap::from([(
                "other_crate".to_string(),
                HashSet::from(["foo::bar".into()]),
            )]);
            let result = parse_base_path(
                "other_crate::foo::bar",
                "my_crate",
                &ws,
                &mp,
                &HashMap::new(),
            );
            assert_eq!(
                result,
                Some(("other_crate".to_string(), "foo::bar".to_string()))
            );
        }

        #[test]
        fn test_parse_base_path_cross_crate_no_paths() {
            let ws: HashSet<String> = HashSet::from(["other_crate".to_string()]);
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let result = parse_base_path(
                "other_crate::foo::bar",
                "my_crate",
                &ws,
                &mp,
                &HashMap::new(),
            );
            assert_eq!(result, Some(("other_crate".to_string(), "foo".to_string())));
        }

        #[test]
        fn test_parse_base_path_glob_stays_parent() {
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let ws: HashSet<String> = HashSet::new();
            let result = parse_base_path("crate::analyze", "my_crate", &ws, &mp, &HashMap::new());
            assert_eq!(
                result,
                Some(("my_crate".to_string(), "analyze".to_string()))
            );
        }

        #[test]
        fn test_multi_symbol_with_submodule() {
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> = HashMap::from([(
                "my_crate".to_string(),
                HashSet::from(["analyze".into(), "analyze::use_parser".into()]),
            )]);
            let deps = process_use_statement_multi(
                "use crate::analyze::use_parser::{normalize, is_workspace_member};",
                1,
                "my_crate",
                &ws,
                Path::new("src/cli.rs"),
                &mp,
                &HashMap::new(),
            );
            assert_eq!(deps.len(), 2, "should return 2 deps: {:?}", deps);
            assert_eq!(deps[0].target_module, "analyze::use_parser");
            assert!(
                deps.iter()
                    .any(|d| d.target_item == Some("normalize".to_string()))
            );
            assert!(
                deps.iter()
                    .any(|d| d.target_item == Some("is_workspace_member".to_string()))
            );
        }
    }

    mod parsing_tests {
        use super::*;

        #[test]
        fn test_parse_workspace_dependencies_mixed() {
            let source = r#"
use crate::graph;
use other_crate::utils;
use serde::Serialize;
use std::collections::HashMap;
"#;
            let ws: HashSet<String> = HashSet::from(["my_crate".into(), "other_crate".into()]);
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let deps = parse_workspace_dependencies(
                source,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );

            assert_eq!(deps.len(), 2, "found: {:?}", deps);
            assert!(
                deps.iter()
                    .any(|d| d.target_crate == "my_crate" && d.target_module == "graph")
            );
            assert!(
                deps.iter()
                    .any(|d| d.target_crate == "other_crate" && d.target_module == "utils")
            );
        }

        #[test]
        fn test_parse_workspace_dependencies_dedup_by_full_target() {
            let source = r#"
use crate::graph::build;
use crate::graph::Node;
use crate::graph;
"#;
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let deps = parse_workspace_dependencies(
                source,
                "my_crate",
                &ws,
                Path::new("src/cli.rs"),
                &mp,
                &HashMap::new(),
            );

            assert_eq!(deps.len(), 3, "should keep distinct symbols: {:?}", deps);
            assert!(
                deps.iter()
                    .any(|d| d.target_item == Some("build".to_string()))
            );
            assert!(
                deps.iter()
                    .any(|d| d.target_item == Some("Node".to_string()))
            );
            assert!(deps.iter().any(|d| d.target_item.is_none()));
        }

        #[test]
        fn test_process_use_multi_symbol() {
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let deps = process_use_statement_multi(
                "use crate::graph::{Node, Edge};",
                1,
                "my_crate",
                &ws,
                Path::new("src/cli.rs"),
                &mp,
                &HashMap::new(),
            );
            assert_eq!(deps.len(), 2, "should return 2 deps: {:?}", deps);
            assert!(
                deps.iter()
                    .any(|d| d.target_item == Some("Node".to_string()))
            );
            assert!(
                deps.iter()
                    .any(|d| d.target_item == Some("Edge".to_string()))
            );
        }

        #[test]
        fn test_process_use_glob() {
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let deps = process_use_statement_multi(
                "use crate::analyze::*;",
                1,
                "my_crate",
                &ws,
                Path::new("src/cli.rs"),
                &mp,
                &HashMap::new(),
            );
            assert_eq!(deps.len(), 1, "glob should return 1 dep: {:?}", deps);
            assert_eq!(deps[0].target_item, Some("*".to_string()));
            assert_eq!(deps[0].target_module, "analyze");
        }
    }

    mod integration_tests {
        use super::*;

        #[test]
        fn test_bare_module_pub_use_integration() {
            let source = r#"
pub use cli::{Args, Cargo, run};
use crate::graph::build;
pub(crate) use model::Node;
use other_crate::utils;
use serde::Serialize;
"#;
            let ws: HashSet<String> = HashSet::from(["my_crate".into(), "other_crate".into()]);
            let mp: HashMap<String, HashSet<String>> = HashMap::from([(
                "my_crate".to_string(),
                HashSet::from(["cli".into(), "graph".into(), "model".into()]),
            )]);
            let deps = parse_workspace_dependencies(
                source,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );

            // Expected: 6 DependencyRefs
            // - cli::Args, cli::Cargo, cli::run (bare module, pub use multi)
            // - graph::build (crate:: prefix)
            // - model::Node (bare module, pub(crate))
            // - other_crate::utils (workspace crate)
            assert_eq!(deps.len(), 6, "expected 6 deps, got: {:?}", deps);

            // Bare module multi-import (pub use)
            assert!(
                deps.iter().any(|d| d.target_crate == "my_crate"
                    && d.target_module == "cli"
                    && d.target_item == Some("Args".to_string())),
                "missing cli::Args"
            );
            assert!(
                deps.iter().any(|d| d.target_crate == "my_crate"
                    && d.target_module == "cli"
                    && d.target_item == Some("Cargo".to_string())),
                "missing cli::Cargo"
            );
            assert!(
                deps.iter().any(|d| d.target_crate == "my_crate"
                    && d.target_module == "cli"
                    && d.target_item == Some("run".to_string())),
                "missing cli::run"
            );

            // crate:: prefix
            assert!(
                deps.iter().any(|d| d.target_crate == "my_crate"
                    && d.target_module == "graph"
                    && d.target_item == Some("build".to_string())),
                "missing graph::build"
            );

            // Bare module pub(crate)
            assert!(
                deps.iter().any(|d| d.target_crate == "my_crate"
                    && d.target_module == "model"
                    && d.target_item == Some("Node".to_string())),
                "missing model::Node"
            );

            // Workspace crate
            assert!(
                deps.iter()
                    .any(|d| d.target_crate == "other_crate" && d.target_module == "utils"),
                "missing other_crate::utils"
            );
        }
    }

    mod visibility_tests {
        use super::*;

        #[test]
        fn test_extract_use_path_pub_use() {
            assert_eq!(extract_use_path("pub use crate::cli;"), Some("crate::cli"));
        }

        #[test]
        fn test_extract_use_path_pub_crate_use() {
            assert_eq!(
                extract_use_path("pub(crate) use cli::Args;"),
                Some("cli::Args")
            );
        }

        #[test]
        fn test_extract_use_path_pub_super_use() {
            assert_eq!(
                extract_use_path("pub(super) use cli::Args;"),
                Some("cli::Args")
            );
        }

        #[test]
        fn test_extract_use_path_pub_in_use() {
            assert_eq!(
                extract_use_path("pub(in crate::foo) use cli::Args;"),
                Some("cli::Args")
            );
        }

        #[test]
        fn test_extract_use_path_plain_use_still_works() {
            assert_eq!(extract_use_path("use crate::graph;"), Some("crate::graph"));
        }

        #[test]
        fn test_extract_use_path_not_use() {
            assert_eq!(extract_use_path("let x = 5;"), None);
        }

        #[test]
        fn test_extract_use_path_comment_with_use() {
            assert_eq!(extract_use_path("// use crate::foo;"), None);
        }
    }

    mod bare_module_tests {
        use super::*;

        #[test]
        fn test_bare_module_simple() {
            let mp: HashMap<String, HashSet<String>> =
                HashMap::from([("my_crate".to_string(), HashSet::from(["cli".into()]))]);
            let dep =
                parse_bare_module_import("cli::Args", "my_crate", Path::new("src/lib.rs"), 1, &mp);
            let dep = dep.expect("should parse bare module import");
            assert_eq!(dep.target_crate, "my_crate");
            assert_eq!(dep.target_module, "cli");
            assert_eq!(dep.target_item, Some("Args".to_string()));
        }

        #[test]
        fn test_bare_module_no_match() {
            let mp: HashMap<String, HashSet<String>> =
                HashMap::from([("my_crate".to_string(), HashSet::from(["cli".into()]))]);
            let dep = parse_bare_module_import(
                "serde::Serialize",
                "my_crate",
                Path::new("src/lib.rs"),
                1,
                &mp,
            );
            assert!(dep.is_none(), "external crate should not match");
        }

        #[test]
        fn test_bare_module_deep_path() {
            let mp: HashMap<String, HashSet<String>> = HashMap::from([(
                "my_crate".to_string(),
                HashSet::from(["analyze".into(), "analyze::use_parser".into()]),
            )]);
            let dep = parse_bare_module_import(
                "analyze::use_parser::normalize",
                "my_crate",
                Path::new("src/lib.rs"),
                1,
                &mp,
            );
            let dep = dep.expect("should parse deep bare module import");
            assert_eq!(dep.target_crate, "my_crate");
            assert_eq!(dep.target_module, "analyze::use_parser");
            assert_eq!(dep.target_item, Some("normalize".to_string()));
        }

        #[test]
        fn test_bare_module_module_only() {
            let mp: HashMap<String, HashSet<String>> =
                HashMap::from([("my_crate".to_string(), HashSet::from(["cli".into()]))]);
            let dep = parse_bare_module_import("cli", "my_crate", Path::new("src/lib.rs"), 1, &mp);
            let dep = dep.expect("should parse module-only bare import");
            assert_eq!(dep.target_crate, "my_crate");
            assert_eq!(dep.target_module, "cli");
            assert!(dep.target_item.is_none());
        }

        #[test]
        fn test_bare_module_via_process_use_statement() {
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> =
                HashMap::from([("my_crate".to_string(), HashSet::from(["cli".into()]))]);
            let dep = process_use_statement(
                "use cli::Args;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            let dep = dep.expect("should parse bare module via process_use_statement");
            assert_eq!(dep.target_crate, "my_crate");
            assert_eq!(dep.target_module, "cli");
            assert_eq!(dep.target_item, Some("Args".to_string()));
        }

        #[test]
        fn test_bare_module_pub_use_via_process_use_statement() {
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> =
                HashMap::from([("my_crate".to_string(), HashSet::from(["cli".into()]))]);
            let dep = process_use_statement(
                "pub(crate) use cli::Args;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            let dep = dep.expect("should parse pub(crate) bare module import");
            assert_eq!(dep.target_crate, "my_crate");
            assert_eq!(dep.target_module, "cli");
            assert_eq!(dep.target_item, Some("Args".to_string()));
        }

        #[test]
        fn test_parse_base_path_bare_module() {
            let mp: HashMap<String, HashSet<String>> =
                HashMap::from([("my_crate".to_string(), HashSet::from(["cli".into()]))]);
            let ws: HashSet<String> = HashSet::new();
            let result = parse_base_path("cli", "my_crate", &ws, &mp, &HashMap::new());
            assert_eq!(result, Some(("my_crate".to_string(), "cli".to_string())));
        }

        #[test]
        fn test_parse_base_path_bare_module_deep() {
            let mp: HashMap<String, HashSet<String>> = HashMap::from([(
                "my_crate".to_string(),
                HashSet::from(["analyze".into(), "analyze::use_parser".into()]),
            )]);
            let ws: HashSet<String> = HashSet::new();
            let result =
                parse_base_path("analyze::use_parser", "my_crate", &ws, &mp, &HashMap::new());
            assert_eq!(
                result,
                Some(("my_crate".to_string(), "analyze::use_parser".to_string()))
            );
        }

        #[test]
        fn test_parse_base_path_bare_no_match() {
            let mp: HashMap<String, HashSet<String>> =
                HashMap::from([("my_crate".to_string(), HashSet::from(["cli".into()]))]);
            let ws: HashSet<String> = HashSet::new();
            let result = parse_base_path("serde", "my_crate", &ws, &mp, &HashMap::new());
            assert!(result.is_none(), "non-module should not match");
        }

        #[test]
        fn test_bare_module_multi_import() {
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> =
                HashMap::from([("my_crate".to_string(), HashSet::from(["cli".into()]))]);
            let deps = process_use_statement_multi(
                "use cli::{Args, Cargo, run};",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            assert_eq!(deps.len(), 3, "should return 3 deps: {:?}", deps);
            assert!(deps.iter().all(|d| d.target_crate == "my_crate"));
            assert!(deps.iter().all(|d| d.target_module == "cli"));
            assert!(
                deps.iter()
                    .any(|d| d.target_item == Some("Args".to_string()))
            );
            assert!(
                deps.iter()
                    .any(|d| d.target_item == Some("Cargo".to_string()))
            );
            assert!(
                deps.iter()
                    .any(|d| d.target_item == Some("run".to_string()))
            );
        }

        #[test]
        fn test_bare_module_glob_import() {
            let ws: HashSet<String> = HashSet::new();
            let mp: HashMap<String, HashSet<String>> =
                HashMap::from([("my_crate".to_string(), HashSet::from(["cli".into()]))]);
            let deps = process_use_statement_multi(
                "use cli::*;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            assert_eq!(deps.len(), 1, "glob should return 1 dep: {:?}", deps);
            assert_eq!(deps[0].target_crate, "my_crate");
            assert_eq!(deps[0].target_module, "cli");
            assert_eq!(deps[0].target_item, Some("*".to_string()));
        }
    }

    mod entry_point_tests {
        use super::*;

        #[test]
        fn test_entry_point_export_detected() {
            let ws: HashSet<String> = HashSet::from(["other_crate".to_string()]);
            let mp: HashMap<String, HashSet<String>> =
                HashMap::from([("other_crate".to_string(), HashSet::from(["sub_mod".into()]))]);
            let exports: HashMap<String, HashSet<String>> = HashMap::from([(
                "other_crate".to_string(),
                HashSet::from(["MyStruct".into()]),
            )]);
            let dep = process_use_statement(
                "use other_crate::MyStruct;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &exports,
            );
            let dep = dep.expect("should detect entry-point export");
            assert_eq!(dep.target_crate, "other_crate");
            assert_eq!(dep.target_module, "");
            assert_eq!(dep.target_item, Some("MyStruct".to_string()));
        }

        #[test]
        fn test_non_export_stays_fallback() {
            let ws: HashSet<String> = HashSet::from(["other_crate".to_string()]);
            let mp: HashMap<String, HashSet<String>> =
                HashMap::from([("other_crate".to_string(), HashSet::from(["sub_mod".into()]))]);
            let exports: HashMap<String, HashSet<String>> =
                HashMap::from([("other_crate".to_string(), HashSet::new())]);
            let dep = process_use_statement(
                "use other_crate::Unknown;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &exports,
            );
            let dep = dep.expect("should fall back to module interpretation");
            assert_eq!(dep.target_module, "Unknown");
            assert!(dep.target_item.is_none());
        }

        #[test]
        fn test_real_module_not_affected() {
            let ws: HashSet<String> = HashSet::from(["other_crate".to_string()]);
            let mp: HashMap<String, HashSet<String>> =
                HashMap::from([("other_crate".to_string(), HashSet::from(["sub_mod".into()]))]);
            let exports: HashMap<String, HashSet<String>> =
                HashMap::from([("other_crate".to_string(), HashSet::from(["sub_mod".into()]))]);
            let dep = process_use_statement(
                "use other_crate::sub_mod::Foo;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &exports,
            );
            let dep = dep.expect("real module should take priority");
            assert_eq!(dep.target_module, "sub_mod");
            assert_eq!(dep.target_item, Some("Foo".to_string()));
        }

        #[test]
        fn test_entry_point_multi_import() {
            let ws: HashSet<String> = HashSet::from(["other_crate".to_string()]);
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let deps = process_use_statement_multi(
                "use other_crate::{Foo, Bar};",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            assert_eq!(deps.len(), 2, "should return 2 deps: {:?}", deps);
            assert!(deps.iter().all(|d| d.target_module.is_empty()));
            assert!(
                deps.iter()
                    .any(|d| d.target_item == Some("Foo".to_string()))
            );
            assert!(
                deps.iter()
                    .any(|d| d.target_item == Some("Bar".to_string()))
            );
        }

        #[test]
        fn test_entry_point_glob_import() {
            let ws: HashSet<String> = HashSet::from(["other_crate".to_string()]);
            let mp: HashMap<String, HashSet<String>> = HashMap::new();
            let deps = process_use_statement_multi(
                "use other_crate::*;",
                1,
                "my_crate",
                &ws,
                Path::new("src/lib.rs"),
                &mp,
                &HashMap::new(),
            );
            assert_eq!(deps.len(), 1, "glob should return 1 dep: {:?}", deps);
            assert_eq!(deps[0].target_module, "");
            assert_eq!(deps[0].target_item, Some("*".to_string()));
        }
    }
}
