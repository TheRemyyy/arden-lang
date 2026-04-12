use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputKind {
    Bin,
    Shared,
    Static,
}

fn default_output_kind() -> OutputKind {
    OutputKind::Bin
}

fn default_output() -> String {
    "main".to_string()
}

fn default_opt_level() -> String {
    "3".to_string()
}

/// Project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name
    pub name: String,
    /// Project version
    pub version: String,
    /// Entry point file (contains main function)
    pub entry: String,
    /// Source files to compile (relative to project root)
    pub files: Vec<String>,
    /// Output binary name
    #[serde(default = "default_output")]
    pub output: String,
    /// Additional compiler flags
    #[serde(default)]
    pub flags: Vec<String>,
    /// Dependencies (future use)
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    /// Optimization level
    #[serde(default = "default_opt_level")]
    pub opt_level: String,
    /// Target triple (optional)
    #[serde(default)]
    pub target: Option<String>,
    /// Output kind (bin/shared/static)
    #[serde(default = "default_output_kind")]
    pub output_kind: OutputKind,
    /// Additional libraries to link (`-lfoo`)
    #[serde(default)]
    pub link_libs: Vec<String>,
    /// Additional library search paths (`-L/path`)
    #[serde(default)]
    pub link_search: Vec<String>,
    /// Additional raw linker arguments
    #[serde(default)]
    pub link_args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ProjectTomlRoot {
    pub(super) project: ProjectConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "untitled".to_string(),
            version: "0.1.0".to_string(),
            entry: "src/main.arden".to_string(),
            files: vec!["src/main.arden".to_string()],
            output: default_output(),
            flags: vec![],
            dependencies: HashMap::new(),
            opt_level: default_opt_level(),
            target: None,
            output_kind: default_output_kind(),
            link_libs: vec![],
            link_search: vec![],
            link_args: vec![],
        }
    }
}

impl ProjectConfig {
    /// Create default project config
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            entry: "src/main.arden".to_string(),
            files: vec!["src/main.arden".to_string()],
            output: name.to_string(),
            flags: vec![],
            dependencies: HashMap::new(),
            opt_level: "3".to_string(),
            target: None,
            output_kind: default_output_kind(),
            link_libs: vec![],
            link_search: vec![],
            link_args: vec![],
        }
    }

    /// Get all source files as absolute paths
    pub fn get_source_files(&self, project_root: &Path) -> Vec<PathBuf> {
        self.files.iter().map(|f| project_root.join(f)).collect()
    }

    /// Get entry point as absolute path
    pub fn get_entry_path(&self, project_root: &Path) -> PathBuf {
        project_root.join(&self.entry)
    }
}
