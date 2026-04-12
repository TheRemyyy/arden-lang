//! Arden Project Configuration
//!
//! Supports multi-file projects with arden.toml configuration

pub(crate) mod pipeline;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

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
struct ProjectTomlRoot {
    project: ProjectConfig,
}

fn default_output() -> String {
    "main".to_string()
}

fn default_opt_level() -> String {
    "3".to_string()
}

fn validate_project_path(
    canonical_root: &Path,
    _project_root: &Path,
    relative_path: &str,
    label: &str,
) -> Result<PathBuf, String> {
    let resolved_path = normalize_project_relative_path(canonical_root, Path::new(relative_path))?;

    if !resolved_path.exists() {
        return Err(format!(
            "{} '{}' not found at '{}'",
            label,
            relative_path,
            resolved_path.display()
        ));
    }

    let metadata = fs::symlink_metadata(&resolved_path).map_err(|e| {
        format!(
            "Failed to inspect {} '{}' at '{}': {}",
            label,
            relative_path,
            resolved_path.display(),
            e
        )
    })?;

    if !metadata.file_type().is_file() {
        return Err(format!(
            "{} '{}' must resolve to a file, found '{}'",
            label,
            relative_path,
            resolved_path.display()
        ));
    }

    if resolved_path.extension().and_then(|ext| ext.to_str()) != Some("arden") {
        return Err(format!(
            "{} '{}' must resolve to an .arden source file",
            label, relative_path
        ));
    }

    if metadata.file_type().is_symlink() {
        let canonical_path = resolved_path.canonicalize().map_err(|e| {
            format!(
                "Failed to resolve {} '{}' at '{}': {}",
                label,
                relative_path,
                resolved_path.display(),
                e
            )
        })?;
        if !canonical_path.starts_with(canonical_root) {
            return Err(format!(
                "{} '{}' resolves outside the project root '{}'",
                label,
                relative_path,
                canonical_root.display()
            ));
        }
        return Ok(canonical_path);
    }

    Ok(resolved_path)
}

fn validate_output_path(
    canonical_root: &Path,
    project_root: &Path,
    relative_path: &str,
) -> Result<(), String> {
    if relative_path.trim().is_empty() {
        return Err("Output path must not be empty".to_string());
    }

    let output_path = Path::new(relative_path);
    if output_path.is_absolute() {
        return Err(format!(
            "Output path '{}' must be relative to the project root",
            relative_path
        ));
    }

    let resolved_path = project_root.join(output_path);

    let existing_parent = resolved_path
        .ancestors()
        .find(|path| path.exists())
        .ok_or_else(|| {
            format!(
                "Failed to resolve output path '{}' relative to project root '{}'",
                relative_path,
                canonical_root.display()
            )
        })?;

    let canonical_parent = existing_parent.canonicalize().map_err(|e| {
        format!(
            "Failed to resolve output path '{}' at '{}': {}",
            relative_path,
            existing_parent.display(),
            e
        )
    })?;

    if !canonical_parent.starts_with(canonical_root) {
        return Err(format!(
            "Output path '{}' resolves outside the project root '{}'",
            relative_path,
            canonical_root.display()
        ));
    }

    if resolved_path.exists() && resolved_path.is_dir() {
        return Err(format!(
            "Output path '{}' must not point to a directory",
            relative_path
        ));
    }

    Ok(())
}

fn normalize_project_relative_path(
    canonical_root: &Path,
    relative_path: &Path,
) -> Result<PathBuf, String> {
    let mut normalized = canonical_root.to_path_buf();

    for component in relative_path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if !normalized.pop() || !normalized.starts_with(canonical_root) {
                    return Err(format!(
                        "Path '{}' resolves outside the project root '{}'",
                        relative_path.display(),
                        canonical_root.display()
                    ));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "Path '{}' must be relative to the project root",
                    relative_path.display()
                ));
            }
        }
    }

    Ok(normalized)
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
    /// Load project config from arden.toml
    pub fn load(path: &Path) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read project file: {}", e))?;

        if let Ok(config) = toml::from_str::<ProjectConfig>(&content) {
            return Ok(config);
        }
        if let Ok(wrapper) = toml::from_str::<ProjectTomlRoot>(&content) {
            return Ok(wrapper.project);
        }

        toml::from_str::<ProjectConfig>(&content)
            .map_err(|e| format!("Failed to parse project file: {}", e))
    }

    /// Save project config to arden.toml
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize project: {}", e))?;

        fs::write(path, content).map_err(|e| format!("Failed to write project file: {}", e))?;

        Ok(())
    }

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

    /// Validate project configuration
    pub fn validate(&self, project_root: &Path) -> Result<(), String> {
        let canonical_root = project_root.canonicalize().map_err(|e| {
            format!(
                "Failed to resolve project root '{}': {}",
                project_root.display(),
                e
            )
        })?;

        validate_project_path(&canonical_root, project_root, &self.entry, "Entry point")?;
        validate_output_path(&canonical_root, project_root, &self.output)?;

        let output_path =
            normalize_project_relative_path(&canonical_root, Path::new(&self.output))?;
        let config_path =
            normalize_project_relative_path(&canonical_root, Path::new("arden.toml"))?;
        if output_path == config_path {
            return Err(format!(
                "Output path '{}' must not overwrite the project config",
                self.output
            ));
        }

        let mut seen_files = HashSet::new();

        // Check all source files exist
        for file in &self.files {
            let source_path =
                validate_project_path(&canonical_root, project_root, file, "Source file")?;
            if !seen_files.insert(file.as_str()) {
                return Err(format!("Duplicate source file '{}' listed in files", file));
            }
            if output_path == source_path {
                return Err(format!(
                    "Output path '{}' must not overwrite source file '{}'",
                    self.output, file
                ));
            }
        }

        // Check entry is in files list
        if !self.files.contains(&self.entry) {
            return Err(format!(
                "Entry point '{}' must be listed in files",
                self.entry
            ));
        }

        Ok(())
    }
}

/// Find project root by looking for arden.toml
pub fn find_project_root(start_dir: &Path) -> Option<PathBuf> {
    let normalized = if start_dir.is_absolute() {
        start_dir.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(start_dir)
    };

    let mut current = if normalized.is_dir() {
        Some(normalized.as_path())
    } else if normalized.is_file() || normalized.extension().is_some() {
        normalized.parent()
    } else {
        Some(normalized.as_path())
    };

    while let Some(dir) = current {
        let config_path = dir.join("arden.toml");
        if config_path.is_file() {
            return Some(dir.to_path_buf());
        }

        current = dir.parent();
    }

    None
}
