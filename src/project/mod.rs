//! Arden Project Configuration
//!
//! Supports multi-file projects with arden.toml configuration

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

/// Check if path is inside a project
#[allow(dead_code)]
pub fn is_in_project(path: &Path) -> bool {
    find_project_root(path).is_some()
}

#[cfg(test)]
mod tests {
    use super::{OutputKind, ProjectConfig};
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let base_temp = std::env::temp_dir()
            .canonicalize()
            .unwrap_or_else(|_| std::env::temp_dir());
        base_temp.join(format!("{prefix}_{unique}"))
    }

    #[test]
    fn defaults_include_linker_configuration_fields() {
        let config = ProjectConfig::new("demo");
        assert_eq!(config.output_kind, OutputKind::Bin);
        assert!(config.link_libs.is_empty());
        assert!(config.link_search.is_empty());
        assert!(config.link_args.is_empty());
    }

    #[test]
    fn parses_linker_configuration_from_toml() {
        let config: ProjectConfig = toml::from_str(
            r#"
name = "demo"
version = "0.1.0"
entry = "src/main.arden"
files = ["src/main.arden"]
output = "demo"
output_kind = "shared"
link_libs = ["ssl", "crypto"]
link_search = ["native/lib", "/usr/local/lib"]
link_args = ["-Wl,--as-needed"]
"#,
        )
        .expect("project config parses");

        assert_eq!(config.output_kind, OutputKind::Shared);
        assert_eq!(config.link_libs, vec!["ssl", "crypto"]);
        assert_eq!(config.link_search, vec!["native/lib", "/usr/local/lib"]);
        assert_eq!(config.link_args, vec!["-Wl,--as-needed"]);
    }

    #[test]
    fn loads_project_table_toml_shape() {
        let dir = std::env::temp_dir();
        let path = dir.join("arden_project_table_shape_test.toml");
        let content = r#"
[project]
name = "demo"
version = "0.1.0"
entry = "src/main.arden"
files = ["src/main.arden"]
output = "demo"
"#;
        std::fs::write(&path, content).expect("write temporary toml");
        let config = ProjectConfig::load(&path).expect("project table shape should load");
        let _ = std::fs::remove_file(&path);
        assert_eq!(config.name, "demo");
        assert_eq!(config.entry, "src/main.arden");
    }

    #[test]
    fn validate_rejects_entry_outside_project_root() {
        let project_root = unique_temp_dir("arden_project_validate_entry_escape");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(
            src_dir.join("main.arden"),
            "function main(): None { return None; }\n",
        )
        .expect("entry file should be written");

        let escaped_file = project_root
            .parent()
            .expect("temp dir should have parent")
            .join("escaped_entry.arden");
        std::fs::write(&escaped_file, "function main(): None { return None; }\n")
            .expect("escaped file should be written");

        let mut config = ProjectConfig::new("demo");
        config.entry = "../escaped_entry.arden".to_string();
        config.files = vec!["../escaped_entry.arden".to_string()];

        let error = config
            .validate(&project_root)
            .expect_err("entry outside project root should be rejected");

        let _ = std::fs::remove_file(&escaped_file);
        let _ = std::fs::remove_dir_all(&project_root);

        assert!(error.contains("outside the project root"), "{error}");
    }

    #[test]
    fn validate_rejects_source_file_outside_project_root() {
        let project_root = unique_temp_dir("arden_project_validate_file_escape");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(
            src_dir.join("main.arden"),
            "function main(): None { return None; }\n",
        )
        .expect("entry file should be written");

        let escaped_file = project_root
            .parent()
            .expect("temp dir should have parent")
            .join("escaped_module.arden");
        std::fs::write(&escaped_file, "function helper(): None { return None; }\n")
            .expect("escaped module should be written");

        let mut config = ProjectConfig::new("demo");
        config.files.push("../escaped_module.arden".to_string());

        let error = config
            .validate(&project_root)
            .expect_err("source file outside project root should be rejected");

        let _ = std::fs::remove_file(&escaped_file);
        let _ = std::fs::remove_dir_all(&project_root);

        assert!(error.contains("outside the project root"), "{error}");
    }

    #[test]
    fn validate_rejects_directory_entry_path() {
        let project_root = unique_temp_dir("arden_project_validate_entry_dir");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");

        let mut config = ProjectConfig::new("demo");
        config.entry = "src".to_string();
        config.files = vec!["src".to_string()];

        let error = config
            .validate(&project_root)
            .expect_err("directory entry path should be rejected");

        let _ = std::fs::remove_dir_all(&project_root);

        assert!(error.contains("must resolve to a file"), "{error}");
    }

    #[test]
    fn validate_rejects_directory_source_path() {
        let project_root = unique_temp_dir("arden_project_validate_file_dir");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(
            src_dir.join("main.arden"),
            "function main(): None { return None; }\n",
        )
        .expect("entry file should be written");
        std::fs::create_dir_all(src_dir.join("nested")).expect("nested source dir should exist");

        let mut config = ProjectConfig::new("demo");
        config.files.push("src/nested".to_string());

        let error = config
            .validate(&project_root)
            .expect_err("directory source path should be rejected");

        let _ = std::fs::remove_dir_all(&project_root);

        assert!(error.contains("must resolve to a file"), "{error}");
    }

    #[test]
    fn validate_rejects_non_arden_entry_path() {
        let project_root = unique_temp_dir("arden_project_validate_entry_non_arden");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(src_dir.join("main.txt"), "not arden\n")
            .expect("entry file should be written");

        let mut config = ProjectConfig::new("demo");
        config.entry = "src/main.txt".to_string();
        config.files = vec!["src/main.txt".to_string()];

        let error = config
            .validate(&project_root)
            .expect_err("non-arden entry path should be rejected");

        let _ = std::fs::remove_dir_all(&project_root);

        assert!(
            error.contains("must resolve to an .arden source file"),
            "{error}"
        );
    }

    #[test]
    fn validate_rejects_non_arden_source_path() {
        let project_root = unique_temp_dir("arden_project_validate_source_non_arden");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(
            src_dir.join("main.arden"),
            "function main(): None { return None; }\n",
        )
        .expect("entry file should be written");
        std::fs::write(src_dir.join("helper.txt"), "not arden\n")
            .expect("helper file should be written");

        let mut config = ProjectConfig::new("demo");
        config.files.push("src/helper.txt".to_string());

        let error = config
            .validate(&project_root)
            .expect_err("non-arden source path should be rejected");

        let _ = std::fs::remove_dir_all(&project_root);

        assert!(
            error.contains("must resolve to an .arden source file"),
            "{error}"
        );
    }

    #[test]
    fn validate_rejects_output_path_outside_project_root() {
        let project_root = unique_temp_dir("arden_project_validate_output_escape");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(
            src_dir.join("main.arden"),
            "function main(): None { return None; }\n",
        )
        .expect("entry file should be written");

        let mut config = ProjectConfig::new("demo");
        config.output = "../escaped-output/demo".to_string();

        let error = config
            .validate(&project_root)
            .expect_err("output outside project root should be rejected");

        let _ = std::fs::remove_dir_all(&project_root);

        assert!(error.contains("outside the project root"), "{error}");
    }

    #[test]
    fn validate_rejects_output_path_matching_project_config() {
        let project_root = unique_temp_dir("arden_project_validate_output_config_collision");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(
            src_dir.join("main.arden"),
            "function main(): None { return None; }\n",
        )
        .expect("entry file should be written");

        let mut config = ProjectConfig::new("demo");
        config.output = "arden.toml".to_string();

        let error = config
            .validate(&project_root)
            .expect_err("output matching arden.toml should be rejected");

        let _ = std::fs::remove_dir_all(&project_root);

        assert!(error.contains("project config"), "{error}");
    }

    #[test]
    fn validate_rejects_output_path_matching_entry_file() {
        let project_root = unique_temp_dir("arden_project_validate_output_entry_collision");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(
            src_dir.join("main.arden"),
            "function main(): None { return None; }\n",
        )
        .expect("entry file should be written");

        let mut config = ProjectConfig::new("demo");
        config.output = "src/main.arden".to_string();

        let error = config
            .validate(&project_root)
            .expect_err("output matching entry should be rejected");

        let _ = std::fs::remove_dir_all(&project_root);

        assert!(error.contains("overwrite source file"), "{error}");
    }

    #[test]
    fn validate_rejects_output_path_matching_secondary_source_file() {
        let project_root = unique_temp_dir("arden_project_validate_output_source_collision");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(
            src_dir.join("main.arden"),
            "function main(): None { return None; }\n",
        )
        .expect("entry file should be written");
        std::fs::write(
            src_dir.join("helper.arden"),
            "function helper(): None { return None; }\n",
        )
        .expect("helper file should be written");

        let mut config = ProjectConfig::new("demo");
        config.files.push("src/helper.arden".to_string());
        config.output = "src/helper.arden".to_string();

        let error = config
            .validate(&project_root)
            .expect_err("output matching secondary source should be rejected");

        let _ = std::fs::remove_dir_all(&project_root);

        assert!(error.contains("overwrite source file"), "{error}");
    }

    #[test]
    fn validate_rejects_duplicate_source_files() {
        let project_root = unique_temp_dir("arden_project_validate_duplicate_files");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(
            src_dir.join("main.arden"),
            "function main(): None { return None; }\n",
        )
        .expect("entry file should be written");

        let mut config = ProjectConfig::new("demo");
        config.files = vec!["src/main.arden".to_string(), "src/main.arden".to_string()];

        let error = config
            .validate(&project_root)
            .expect_err("duplicate source file should be rejected");

        let _ = std::fs::remove_dir_all(&project_root);

        assert!(error.contains("Duplicate source file"), "{error}");
    }

    #[test]
    fn find_project_root_accepts_source_file_path() {
        let project_root = unique_temp_dir("arden_project_find_root_file");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(project_root.join("arden.toml"), "name = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\n")
            .expect("project config should be written");
        let source_file = src_dir.join("main.arden");
        std::fs::write(&source_file, "function main(): None { return None; }\n")
            .expect("source file should be written");

        let discovered = super::find_project_root(&source_file);

        let _ = std::fs::remove_dir_all(&project_root);

        assert_eq!(discovered.as_deref(), Some(project_root.as_path()));
    }

    #[test]
    fn is_in_project_accepts_source_file_path() {
        let project_root = unique_temp_dir("arden_project_is_in_project_file");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(project_root.join("arden.toml"), "name = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\n")
            .expect("project config should be written");
        let source_file = src_dir.join("main.arden");
        std::fs::write(&source_file, "function main(): None { return None; }\n")
            .expect("source file should be written");

        let result = super::is_in_project(&source_file);

        let _ = std::fs::remove_dir_all(&project_root);

        assert!(result);
    }

    #[test]
    fn find_project_root_accepts_nonexistent_source_file_path() {
        let project_root = unique_temp_dir("arden_project_find_root_missing_file");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(project_root.join("arden.toml"), "name = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\n")
            .expect("project config should be written");
        let future_source_file = src_dir.join("new_file.arden");

        let discovered = super::find_project_root(&future_source_file);

        let _ = std::fs::remove_dir_all(&project_root);

        assert_eq!(discovered.as_deref(), Some(project_root.as_path()));
    }

    #[test]
    fn find_project_root_accepts_existing_directory_with_dot_in_name() {
        let parent_root = unique_temp_dir("arden_project_find_root_dotted_dir_parent");
        let project_root = parent_root.join("demo.v1");
        std::fs::create_dir_all(project_root.join("src")).expect("project src dir should exist");
        std::fs::write(
            project_root.join("arden.toml"),
            "name = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\n",
        )
        .expect("project config should be written");

        let discovered = super::find_project_root(&project_root);

        let _ = std::fs::remove_dir_all(&parent_root);

        assert_eq!(discovered.as_deref(), Some(project_root.as_path()));
    }

    #[test]
    fn find_project_root_accepts_relative_existing_directory_inside_project() {
        let project_root = unique_temp_dir("arden_project_find_root_relative_dir");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");
        std::fs::write(
            project_root.join("arden.toml"),
            "name = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\n",
        )
        .expect("project config should be written");

        let previous_dir = std::env::current_dir().expect("current dir");
        std::env::set_current_dir(&project_root).expect("enter project root");
        let discovered = super::find_project_root(Path::new("src"));
        let _ = std::env::set_current_dir(previous_dir);

        let _ = std::fs::remove_dir_all(&project_root);

        assert_eq!(discovered.as_deref(), Some(project_root.as_path()));
    }

    #[test]
    fn find_project_root_rejects_directory_named_arden_toml() {
        let project_root = unique_temp_dir("arden_project_find_root_fake_config_dir");
        let fake_config_dir = project_root.join("arden.toml");
        let src_dir = project_root.join("src");
        std::fs::create_dir_all(&fake_config_dir).expect("fake arden.toml directory should exist");
        std::fs::create_dir_all(&src_dir).expect("project src dir should be created");

        let discovered = super::find_project_root(&src_dir);

        let _ = std::fs::remove_dir_all(&project_root);

        assert!(discovered.is_none());
    }
}
