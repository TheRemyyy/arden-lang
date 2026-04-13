use crate::cli::output::format_cli_path;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn find_test_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.exists() {
        return Err(format!("Path '{}' does not exist", format_cli_path(dir)));
    }
    if !dir.is_dir() {
        return Err(format!(
            "Path '{}' is not a directory",
            format_cli_path(dir)
        ));
    }
    if crate::cli::paths::path_traverses_symlinked_directories(dir)? {
        return Err(format!(
            "Path '{}' must not traverse symlinked directories",
            format_cli_path(dir)
        ));
    }
    let metadata = fs::symlink_metadata(dir).map_err(|e| {
        format!(
            "Failed to inspect directory '{}': {}",
            format_cli_path(dir),
            e
        )
    })?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Path '{}' must not be a symlinked directory",
            format_cli_path(dir)
        ));
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
    has_test_suffix(file_name, "test") || has_test_suffix(file_name, "spec")
}

fn has_test_suffix(file_name: &str, suffix: &str) -> bool {
    let suffix_char_count = suffix.chars().count();
    let file_char_count = file_name.chars().count();
    if file_char_count < suffix_char_count {
        return false;
    }

    let suffix_start_char = file_char_count - suffix_char_count;
    let mut char_indices = file_name.char_indices();
    let suffix_start_byte = if suffix_start_char == 0 {
        0
    } else {
        let Some((idx, _)) = char_indices.nth(suffix_start_char) else {
            return false;
        };
        idx
    };

    let suffix_text = &file_name[suffix_start_byte..];
    if !suffix_text.eq_ignore_ascii_case(suffix) {
        return false;
    }
    if suffix_start_byte == 0 {
        return true;
    }

    let mut prefix_chars = file_name[..suffix_start_byte].chars();
    let Some(previous_char) = prefix_chars.next_back() else {
        return true;
    };
    if !previous_char.is_ascii_alphanumeric() {
        return true;
    }

    let Some(suffix_first) = suffix_text.chars().next() else {
        return false;
    };
    previous_char.is_ascii_lowercase() && suffix_first.is_ascii_uppercase()
}

fn find_test_files_recursive(dir: &Path, test_files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| {
        format!(
            "Failed to read directory '{}' while discovering tests: {}",
            format_cli_path(dir),
            e
        )
    })? {
        let entry = entry.map_err(|e| {
            format!(
                "Failed to read directory entry in '{}' while discovering tests: {}",
                format_cli_path(dir),
                e
            )
        })?;
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "Failed to inspect directory entry '{}' while discovering tests: {}",
                format_cli_path(&entry.path()),
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
