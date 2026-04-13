use crate::cli::output::format_cli_path;
use crate::project::ProjectConfig;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

fn validate_project_path(
    canonical_root: &Path,
    project_root: &Path,
    relative_path: &str,
    label: &str,
) -> Result<PathBuf, String> {
    let resolved_path = normalize_project_relative_path(canonical_root, Path::new(relative_path))?;

    if !resolved_path.exists() {
        return Err(format!(
            "{} '{}' not found at '{}' (project root '{}')",
            label,
            relative_path,
            format_cli_path(&resolved_path),
            format_cli_path(project_root)
        ));
    }

    let canonical_path = resolved_path.canonicalize().map_err(|e| {
        format!(
            "Failed to resolve {} '{}' at '{}' while validating project root '{}': {}",
            label,
            relative_path,
            format_cli_path(&resolved_path),
            format_cli_path(project_root),
            e
        )
    })?;
    if !canonical_path.starts_with(canonical_root) {
        return Err(format!(
            "{} '{}' resolves outside the project root '{}'",
            label,
            relative_path,
            format_cli_path(canonical_root)
        ));
    }

    let metadata = fs::metadata(&canonical_path).map_err(|e| {
        format!(
            "Failed to inspect {} '{}' at canonical path '{}' while validating project root '{}': {}",
            label,
            relative_path,
            format_cli_path(&canonical_path),
            format_cli_path(project_root),
            e
        )
    })?;
    if !metadata.file_type().is_file() {
        return Err(format!(
            "{} '{}' must resolve to a file, found canonical path '{}' (project root '{}')",
            label,
            relative_path,
            format_cli_path(&canonical_path),
            format_cli_path(project_root)
        ));
    }
    if canonical_path.extension().and_then(|ext| ext.to_str()) != Some("arden") {
        return Err(format!(
            "{} '{}' must resolve to an .arden source file (canonical path '{}')",
            label,
            relative_path,
            format_cli_path(&canonical_path)
        ));
    }

    Ok(canonical_path)
}

fn validate_output_path(
    canonical_root: &Path,
    project_root: &Path,
    relative_path: &str,
) -> Result<(), String> {
    if relative_path.trim().is_empty() {
        return Err("Output path must not be empty (field 'output' in arden.toml)".to_string());
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
                "Failed to resolve output path '{}' relative to project root '{}': no existing ancestor path found",
                relative_path,
                format_cli_path(canonical_root)
            )
        })?;

    let canonical_parent = existing_parent.canonicalize().map_err(|e| {
        format!(
            "Failed to resolve output path '{}' at existing ancestor '{}': {}",
            relative_path,
            format_cli_path(existing_parent),
            e
        )
    })?;

    if !canonical_parent.starts_with(canonical_root) {
        return Err(format!(
            "Output path '{}' resolves outside the project root '{}'",
            relative_path,
            format_cli_path(canonical_root)
        ));
    }

    if resolved_path.exists() && resolved_path.is_dir() {
        return Err(format!(
            "Output path '{}' must not point to a directory (resolved '{}')",
            relative_path,
            format_cli_path(&resolved_path)
        ));
    }

    if resolved_path.exists() {
        let metadata = fs::symlink_metadata(&resolved_path).map_err(|e| {
            format!(
                "Failed to inspect output path '{}' at '{}': {}",
                relative_path,
                format_cli_path(&resolved_path),
                e
            )
        })?;
        if metadata.file_type().is_symlink() {
            let canonical_output = resolved_path.canonicalize().map_err(|e| {
                format!(
                    "Failed to resolve output path '{}' at '{}': {}",
                    relative_path,
                    format_cli_path(&resolved_path),
                    e
                )
            })?;
            if !canonical_output.starts_with(canonical_root) {
                return Err(format!(
                    "Output path '{}' resolves outside the project root '{}'",
                    relative_path,
                    format_cli_path(canonical_root)
                ));
            }
        }
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
                        format_cli_path(relative_path),
                        format_cli_path(canonical_root)
                    ));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "Path '{}' must be relative to the project root",
                    format_cli_path(relative_path)
                ));
            }
        }
    }

    Ok(normalized)
}

impl ProjectConfig {
    /// Validate project configuration
    pub fn validate(&self, project_root: &Path) -> Result<(), String> {
        let canonical_root = project_root.canonicalize().map_err(|e| {
            format!(
                "Failed to resolve project root '{}': {}",
                format_cli_path(project_root),
                e
            )
        })?;

        let entry_path =
            validate_project_path(&canonical_root, project_root, &self.entry, "Entry point")?;
        validate_output_path(&canonical_root, project_root, &self.output)?;

        let output_path =
            normalize_project_relative_path(&canonical_root, Path::new(&self.output))?;
        let output_path_canonical_if_exists = if output_path.exists() {
            Some(output_path.canonicalize().map_err(|e| {
                format!(
                    "Failed to resolve output path '{}' at '{}': {}",
                    self.output,
                    format_cli_path(&output_path),
                    e
                )
            })?)
        } else {
            None
        };
        let config_path =
            normalize_project_relative_path(&canonical_root, Path::new("arden.toml"))?;
        if output_path == config_path {
            return Err(format!(
                "Output path '{}' must not overwrite the project config",
                self.output
            ));
        }

        let mut seen_files = HashSet::new();
        let mut seen_resolved_files = HashSet::new();
        let mut entry_in_files = false;

        // Check all source files exist.
        for file in &self.files {
            let source_path =
                validate_project_path(&canonical_root, project_root, file, "Source file")?;
            if !seen_files.insert(file.as_str()) {
                return Err(format!("Duplicate source file '{}' listed in files", file));
            }
            if !seen_resolved_files.insert(source_path.clone()) {
                return Err(format!(
                    "Duplicate source file path '{}' resolves to the same file as another entry",
                    file
                ));
            }
            if source_path == entry_path {
                entry_in_files = true;
            }
            if output_path == source_path {
                return Err(format!(
                    "Output path '{}' must not overwrite source file '{}'",
                    self.output, file
                ));
            }
            if output_path_canonical_if_exists
                .as_ref()
                .is_some_and(|output_canonical| output_canonical == &source_path)
            {
                return Err(format!(
                    "Output path '{}' must not overwrite source file '{}'",
                    self.output, file
                ));
            }
        }

        // Check entry is in files list (resolved-path aware).
        if !entry_in_files {
            return Err(format!(
                "Entry point '{}' must be listed in files",
                self.entry
            ));
        }

        Ok(())
    }
}
