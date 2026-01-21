//! Workspace & Module Analysis

use anyhow::{Context, Result};
use cargo_metadata::{MetadataCommand, Package};
use ra_ap_hir as hir;
use ra_ap_ide as ide;
use ra_ap_load_cargo as load_cargo;
use ra_ap_paths as paths;
use ra_ap_project_model as project_model;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CrateInfo {
    pub name: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
}

/// Analyzes a workspace and returns all member crates.
/// `manifest_path` should point to a Cargo.toml.
pub fn analyze_workspace(manifest_path: &Path) -> Result<Vec<CrateInfo>> {
    let metadata = MetadataCommand::new()
        .manifest_path(manifest_path)
        .exec()
        .context("Failed to run cargo metadata")?;

    // Collect workspace member names for dependency filtering
    let workspace_members: HashSet<&str> = metadata
        .workspace_packages()
        .iter()
        .map(|p| p.name.as_str())
        .collect();

    let crates: Vec<CrateInfo> = metadata
        .workspace_packages()
        .into_iter()
        .map(|pkg| package_to_crate_info(pkg, &workspace_members))
        .collect();

    Ok(crates)
}

fn package_to_crate_info(pkg: &Package, workspace_members: &HashSet<&str>) -> CrateInfo {
    let dependencies: Vec<String> = pkg
        .dependencies
        .iter()
        .filter(|dep| workspace_members.contains(dep.name.as_str()))
        .map(|dep| dep.name.clone())
        .collect();

    CrateInfo {
        name: pkg.name.clone(),
        path: pkg.manifest_path.parent().unwrap().into(),
        dependencies,
    }
}

// ============================================================================
// Module Hierarchy Analysis (via ra_ap_hir)
// ============================================================================

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub children: Vec<ModuleInfo>,
}

#[derive(Debug, Clone)]
pub struct ModuleTree {
    pub root: ModuleInfo,
}

/// Analyzes the module hierarchy of a crate using rust-analyzer's HIR.
pub fn analyze_modules(crate_info: &CrateInfo) -> Result<ModuleTree> {
    let manifest_path = crate_info.path.join("Cargo.toml");

    // Load workspace into rust-analyzer
    let (krate, host, _vfs) = load_crate(&manifest_path)?;
    let db = host.raw_database();

    // Walk module tree starting from crate root
    let root_module = krate.root_module();
    let root = walk_module(root_module, db);

    Ok(ModuleTree { root })
}

fn walk_module(module: hir::Module, db: &ide::RootDatabase) -> ModuleInfo {
    let name = if module.is_crate_root() {
        module
            .krate()
            .display_name(db)
            .map(|n| n.to_string().replace('-', "_"))
            .unwrap_or_else(|| "crate".to_string())
    } else {
        module
            .name(db)
            .map(|n| n.as_str().to_string())
            .unwrap_or_else(|| "<anonymous>".to_string())
    };

    let children: Vec<ModuleInfo> = module
        .declarations(db)
        .into_iter()
        .filter_map(|decl| {
            if let hir::ModuleDef::Module(child_module) = decl {
                Some(walk_module(child_module, db))
            } else {
                None
            }
        })
        .collect();

    ModuleInfo { name, children }
}

fn load_crate(manifest_path: &Path) -> Result<(hir::Crate, ide::AnalysisHost, ra_ap_vfs::Vfs)> {
    let project_path = manifest_path.canonicalize()?;
    let project_path = dunce::simplified(&project_path).to_path_buf();

    // Minimal cargo config (no sysroot for speed)
    let cargo_config = project_model::CargoConfig {
        sysroot: Some(project_model::RustLibSource::Discover),
        ..Default::default()
    };

    // Load config - minimal for faster loading
    let load_config = load_cargo::LoadCargoConfig {
        load_out_dirs_from_check: false,
        prefill_caches: false,
        with_proc_macro_server: load_cargo::ProcMacroServerChoice::None,
    };

    // Discover project manifest - convert PathBuf -> Utf8PathBuf -> AbsPathBuf
    let utf8_path = paths::Utf8PathBuf::from_path_buf(project_path.clone())
        .map_err(|_| anyhow::anyhow!("Invalid UTF-8 path"))?;
    let root = paths::AbsPathBuf::assert(utf8_path);
    let manifest = project_model::ProjectManifest::discover_single(root.as_path())?;

    // Load project workspace
    let project_workspace =
        project_model::ProjectWorkspace::load(manifest, &cargo_config, &|_| {})?;

    // Load into analysis database
    let (db, vfs, _proc_macro) =
        load_cargo::load_workspace(project_workspace, &Default::default(), &load_config)?;

    let host = ide::AnalysisHost::with_database(db);

    // Find our crate by matching manifest path
    let crates = hir::Crate::all(host.raw_database());
    let parent_path = project_path.parent().unwrap_or(&project_path);
    let parent_utf8 = paths::Utf8PathBuf::from_path_buf(parent_path.to_path_buf())
        .map_err(|_| anyhow::anyhow!("Invalid UTF-8 path"))?;
    let parent_dir = paths::AbsPathBuf::assert(parent_utf8);

    let krate = crates
        .into_iter()
        .find(|k| {
            let root_file = k.root_file(host.raw_database());
            let vfs_path = vfs.file_path(root_file);
            vfs_path
                .as_path()
                .map(|p| p.starts_with(&parent_dir))
                .unwrap_or(false)
        })
        .context("Crate not found in loaded workspace")?;

    Ok((krate, host, vfs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_analyze_workspace_self() {
        // Test with cargo-arc itself as workspace
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let crates = analyze_workspace(&manifest).expect("should analyze");

        // cargo-arc should find itself
        assert!(!crates.is_empty());
        let cargo_arc = crates.iter().find(|c| c.name == "cargo-arc");
        assert!(cargo_arc.is_some(), "should find cargo-arc");
    }

    #[test]
    fn test_crate_info_fields() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let crates = analyze_workspace(&manifest).expect("should analyze");

        let cargo_arc = crates.iter().find(|c| c.name == "cargo-arc").unwrap();
        assert!(cargo_arc.path.exists(), "path should exist");
        // dependencies is empty because cargo-arc has no workspace-internal deps
        // (only external: clap, petgraph, etc.)
    }

    #[test]
    fn test_analyze_modules_self() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let crates = analyze_workspace(&manifest).expect("should analyze workspace");
        let cargo_arc = crates.iter().find(|c| c.name == "cargo-arc").unwrap();

        let tree = analyze_modules(cargo_arc).expect("should analyze modules");

        // cargo-arc root module should be named "cargo_arc"
        assert_eq!(tree.root.name, "cargo_arc");

        // cargo-arc has 4 modules: analyze, graph, layout, render
        let child_names: Vec<_> = tree.root.children.iter().map(|m| m.name.as_str()).collect();
        assert!(
            child_names.contains(&"analyze"),
            "should contain 'analyze' module, found: {:?}",
            child_names
        );
        assert!(
            child_names.contains(&"graph"),
            "should contain 'graph' module, found: {:?}",
            child_names
        );
        assert!(
            child_names.contains(&"layout"),
            "should contain 'layout' module, found: {:?}",
            child_names
        );
        assert!(
            child_names.contains(&"render"),
            "should contain 'render' module, found: {:?}",
            child_names
        );
    }
}
