use crate::project::ProjectConfig;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

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

impl ProjectConfig {
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

        // Check all source files exist.
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

        // Check entry is in files list.
        if !self.files.contains(&self.entry) {
            return Err(format!(
                "Entry point '{}' must be listed in files",
                self.entry
            ));
        }

        Ok(())
    }
}
