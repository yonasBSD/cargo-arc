//! Shared Data Structures
//!
//! Types used across analyze and graph modules, extracted to break circular dependencies.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CrateInfo {
    pub name: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DependencyRef {
    pub target_crate: String,
    pub target_module: String,
    pub target_item: Option<String>,
    pub source_file: PathBuf,
    pub line: usize,
}

impl DependencyRef {
    /// Returns full target path: "crate::module::item" or "crate::module" if no item.
    pub fn full_target(&self) -> String {
        match &self.target_item {
            Some(item) => format!("{}::{}::{}", self.target_crate, self.target_module, item),
            None => format!("{}::{}", self.target_crate, self.target_module),
        }
    }

    /// Returns module-level target: "crate::module" (ignores item).
    pub fn module_target(&self) -> String {
        format!("{}::{}", self.target_crate, self.target_module)
    }
}

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub full_path: String,
    pub children: Vec<ModuleInfo>,
    pub dependencies: Vec<DependencyRef>,
}

#[derive(Debug, Clone)]
pub struct ModuleTree {
    pub root: ModuleInfo,
}
