use crate::cli::paths::current_dir_checked;
use crate::cli::test_discovery::find_test_files as discover_test_files;
use crate::project::{find_project_root, ProjectConfig};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub(super) fn find_test_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    discover_test_files(path)
}

pub(super) fn default_test_files(current_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if let Some(project_root) = find_project_root(current_dir) {
        let config_path = project_root.join("arden.toml");
        let config = ProjectConfig::load(&config_path)?;
        config.validate(&project_root)?;

        let mut files = config.get_source_files(&project_root);
        files.sort();
        return Ok(files);
    }

    discover_test_files(current_dir)
}

pub(super) fn create_test_runner_workspace(
    test_file: &Path,
) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let unique = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Failed to create unique test runner path: {}", e))?
        .as_nanos();
    let stem = test_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("arden_test");
    let temp_dir = std::env::temp_dir().join(format!(
        "arden-test-runner-{}-{}-{}",
        stem,
        std::process::id(),
        unique
    ));
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create test runner workspace: {}", e))?;

    let runner_path = temp_dir.join("runner.arden");
    let exe_path = temp_dir.join("runner.exe");
    Ok((temp_dir, runner_path, exe_path))
}

pub(super) fn create_project_test_runner_workspace(
    project_root: &Path,
    config: &ProjectConfig,
    test_file: &Path,
    runner_code: &str,
) -> Result<(PathBuf, PathBuf), String> {
    let unique = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Failed to create unique test runner project path: {}", e))?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "arden-project-test-runner-{}-{}",
        std::process::id(),
        unique
    ));
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create test runner project workspace: {}", e))?;

    let normalized_test_file = if test_file.is_absolute() {
        test_file.to_path_buf()
    } else {
        current_dir_checked()?.join(test_file)
    };

    let test_rel = normalized_test_file
        .strip_prefix(project_root)
        .map_err(|_| {
            format!(
                "Test file '{}' is outside project root '{}'",
                normalized_test_file.display(),
                project_root.display()
            )
        })?;
    let test_rel_string = test_rel.to_string_lossy().replace('\\', "/");

    for source_file in config.get_source_files(project_root) {
        let rel = source_file.strip_prefix(project_root).map_err(|_| {
            format!(
                "Project source '{}' is outside project root '{}'",
                source_file.display(),
                project_root.display()
            )
        })?;
        let dest = temp_dir.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create runner source directory: {}", e))?;
        }
        if source_file == normalized_test_file {
            fs::write(&dest, runner_code)
                .map_err(|e| format!("Failed to write generated project test runner: {}", e))?;
        } else {
            fs::copy(&source_file, &dest)
                .map_err(|e| format!("Failed to copy project source into test workspace: {}", e))?;
        }
    }

    let runner_dest = temp_dir.join(test_rel);
    if !runner_dest.exists() {
        if let Some(parent) = runner_dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create runner destination directory: {}", e))?;
        }
        fs::write(&runner_dest, runner_code)
            .map_err(|e| format!("Failed to write generated runner source: {}", e))?;
    }

    let mut temp_config = config.clone();
    temp_config.entry = test_rel_string.clone();
    if config.entry != test_rel_string {
        temp_config.files.retain(|file| file != &config.entry);
    }
    if !temp_config
        .files
        .iter()
        .any(|file| file == &test_rel_string)
    {
        temp_config.files.push(test_rel_string);
        temp_config.files.sort();
        temp_config.files.dedup();
    }
    temp_config.output = "runner".to_string();
    temp_config
        .save(&temp_dir.join("arden.toml"))
        .map_err(|e| format!("Failed to write test runner project config: {}", e))?;

    Ok((temp_dir.clone(), temp_dir.join("runner")))
}
