use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn find_test_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.exists() {
        return Err(format!("Path '{}' does not exist", dir.display()));
    }
    if !dir.is_dir() {
        return Err(format!("Path '{}' is not a directory", dir.display()));
    }

    let mut test_files = Vec::new();
    find_test_files_recursive(dir, &mut test_files)?;
    test_files.sort();
    Ok(test_files)
}

pub(crate) fn is_test_like_file(path: &Path) -> bool {
    if path.extension().and_then(|ext| ext.to_str()) != Some("arden") {
        return false;
    }

    let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let lowercase = file_name.to_ascii_lowercase();
    lowercase.contains("test") || lowercase.contains("spec")
}

fn find_test_files_recursive(dir: &Path, test_files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir.display(), e))?
    {
        let entry = entry.map_err(|e| {
            format!(
                "Failed to read directory entry in '{}': {}",
                dir.display(),
                e
            )
        })?;
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "Failed to inspect directory entry '{}' : {}",
                entry.path().display(),
                e
            )
        })?;
        let path = entry.path();

        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() {
            find_test_files_recursive(&path, test_files)?;
            continue;
        }

        if file_type.is_file() && is_test_like_file(&path) {
            test_files.push(path);
        }
    }
    Ok(())
}
