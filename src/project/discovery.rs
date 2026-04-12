use std::path::{Path, PathBuf};

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
