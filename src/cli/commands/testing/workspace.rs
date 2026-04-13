use crate::cli::paths::current_dir_checked;
use crate::cli::test_discovery::find_test_files as discover_test_files;
use crate::project::{find_project_root, resolve_project_output_path, ProjectConfig};
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
        .map_err(|e| {
            format!(
                "Failed to create unique test runner path for '{}': {}",
                test_file.display(),
                e,
            )
        })?
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
    fs::create_dir_all(&temp_dir).map_err(|e| {
        format!(
            "Failed to create test runner workspace '{}': {}",
            temp_dir.display(),
            e
        )
    })?;

    let runner_path = temp_dir.join("runner.arden");
    #[cfg(windows)]
    let exe_path = temp_dir.join("runner.exe");
    #[cfg(not(windows))]
    let exe_path = temp_dir.join("runner");
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
        .map_err(|e| {
            format!(
                "Failed to create unique test runner project path for '{}': {}",
                test_file.display(),
                e
            )
        })?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "arden-project-test-runner-{}-{}",
        std::process::id(),
        unique
    ));
    fs::create_dir_all(&temp_dir).map_err(|e| {
        format!(
            "Failed to create test runner project workspace '{}': {}",
            temp_dir.display(),
            e
        )
    })?;

    let normalized_test_file = if test_file.is_absolute() {
        test_file.to_path_buf()
    } else {
        current_dir_checked()?.join(test_file)
    };
    let canonical_project_root = project_root.canonicalize().map_err(|e| {
        format!(
            "Failed to resolve project root '{}' for test workspace creation: {}",
            project_root.display(),
            e
        )
    })?;
    let canonical_test_file = normalized_test_file.canonicalize().map_err(|e| {
        format!(
            "Failed to resolve test file '{}' for test workspace creation: {}",
            normalized_test_file.display(),
            e
        )
    })?;
    let canonical_original_entry =
        project_root
            .join(&config.entry)
            .canonicalize()
            .map_err(|e| {
                format!(
                    "Failed to resolve project entry '{}' for test workspace creation: {}",
                    config.entry, e
                )
            })?;

    let test_rel = canonical_test_file
        .strip_prefix(&canonical_project_root)
        .map_err(|_| {
            format!(
                "Test file '{}' is outside project root '{}'",
                canonical_test_file.display(),
                canonical_project_root.display()
            )
        })?;
    let test_rel_string = test_rel.to_string_lossy().replace('\\', "/");
    let mut copied_files: Vec<String> = Vec::new();

    for source_file in config.get_source_files(project_root) {
        let canonical_source_file = source_file.canonicalize().map_err(|e| {
            format!(
                "Failed to resolve project source '{}': {}",
                source_file.display(),
                e
            )
        })?;
        if canonical_original_entry == canonical_source_file
            && canonical_source_file != canonical_test_file
        {
            continue;
        }
        let rel = canonical_source_file
            .strip_prefix(&canonical_project_root)
            .map_err(|_| {
                format!(
                    "Project source '{}' is outside project root '{}'",
                    canonical_source_file.display(),
                    canonical_project_root.display()
                )
            })?;
        let rel_string = rel.to_string_lossy().replace('\\', "/");
        copied_files.push(rel_string.clone());
        let rel_path = Path::new(&rel_string);
        let dest = temp_dir.join(rel_path);
        if !dest.starts_with(&temp_dir) {
            return Err(format!(
                "Refusing to write source '{}' outside test workspace '{}'",
                canonical_source_file.display(),
                temp_dir.display()
            ));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create runner source directory '{}': {}",
                    parent.display(),
                    e
                )
            })?;
        }
        if canonical_source_file == canonical_test_file {
            fs::write(&dest, runner_code).map_err(|e| {
                format!(
                    "Failed to write generated project test runner '{}': {}",
                    dest.display(),
                    e
                )
            })?;
        } else {
            fs::copy(&canonical_source_file, &dest).map_err(|e| {
                format!(
                    "Failed to copy project source '{}' into test workspace '{}': {}",
                    canonical_source_file.display(),
                    dest.display(),
                    e
                )
            })?;
        }
    }

    let runner_dest = temp_dir.join(test_rel);
    if !runner_dest.starts_with(&temp_dir) {
        return Err(format!(
            "Refusing to place generated runner outside test workspace '{}'",
            temp_dir.display()
        ));
    }
    if !runner_dest.exists() {
        if let Some(parent) = runner_dest.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create runner destination directory '{}': {}",
                    parent.display(),
                    e
                )
            })?;
        }
        fs::write(&runner_dest, runner_code).map_err(|e| {
            format!(
                "Failed to write generated runner source '{}': {}",
                runner_dest.display(),
                e
            )
        })?;
    }

    let mut temp_config = config.clone();
    temp_config.entry = test_rel_string.clone();
    temp_config.files = copied_files;
    if !temp_config
        .files
        .iter()
        .any(|file| file == &test_rel_string)
    {
        temp_config.files.push(test_rel_string);
    }
    temp_config.files.sort();
    temp_config.files.dedup();
    temp_config.output = "runner".to_string();
    temp_config
        .save(&temp_dir.join("arden.toml"))
        .map_err(|e| {
            format!(
                "Failed to write test runner project config '{}': {}",
                temp_dir.join("arden.toml").display(),
                e
            )
        })?;

    let runner_output_path = resolve_project_output_path(&temp_dir, &temp_config);

    Ok((temp_dir, runner_output_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "arden-test-workspace-{prefix}-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create temp workspace");
        dir
    }

    #[test]
    fn project_runner_workspace_returns_platform_resolved_output_path() {
        let project_root = make_temp_dir("output-path");
        let source_dir = project_root.join("src");
        fs::create_dir_all(&source_dir).expect("create source dir");
        let test_file = source_dir.join("main.arden");
        fs::write(
            &test_file,
            "@Test\nfunction smoke(): None { return None; }\nfunction main(): None { return None; }\n",
        )
        .expect("write source file");

        let config = ProjectConfig::new("smoke");
        let (runner_workspace, runner_output_path) = create_project_test_runner_workspace(
            &project_root,
            &config,
            &test_file,
            "function main(): None { return None; }\n",
        )
        .expect("create project test workspace");

        let runner_config = ProjectConfig::load(&runner_workspace.join("arden.toml"))
            .expect("load runner workspace config");
        let expected_output_path = resolve_project_output_path(&runner_workspace, &runner_config);
        assert_eq!(runner_output_path, expected_output_path);

        let _ = fs::remove_dir_all(&runner_workspace);
        let _ = fs::remove_dir_all(&project_root);
    }

    #[test]
    fn project_runner_workspace_accepts_dotdot_test_paths_inside_project() {
        let project_root = make_temp_dir("dotdot-path");
        let source_dir = project_root.join("src");
        fs::create_dir_all(&source_dir).expect("create source dir");
        let test_file = source_dir.join("main.arden");
        fs::write(
            &test_file,
            "@Test\nfunction smoke(): None { return None; }\nfunction main(): None { return None; }\n",
        )
        .expect("write source file");

        let dotted_path = project_root
            .join("src")
            .join("..")
            .join("src")
            .join("main.arden");
        let config = ProjectConfig::new("smoke");
        let (runner_workspace, runner_output_path) = create_project_test_runner_workspace(
            &project_root,
            &config,
            &dotted_path,
            "function main(): None { return None; }\n",
        )
        .expect("create project test workspace using dotdot test path");

        assert!(
            runner_output_path.starts_with(&runner_workspace),
            "runner output should stay in workspace: {}",
            runner_output_path.display()
        );

        let _ = fs::remove_dir_all(&runner_workspace);
        let _ = fs::remove_dir_all(&project_root);
    }

    #[test]
    fn single_file_runner_workspace_uses_platform_output_name() {
        let project_root = make_temp_dir("single-workspace-name");
        let test_file = project_root.join("case.arden");
        fs::write(
            &test_file,
            "@Test\nfunction smoke(): None { return None; }\n",
        )
        .expect("write source");

        let (runner_workspace, _runner_source, exe_path) =
            create_test_runner_workspace(&test_file).expect("create single-file workspace");

        #[cfg(windows)]
        assert_eq!(
            exe_path.file_name().and_then(|v| v.to_str()),
            Some("runner.exe")
        );
        #[cfg(not(windows))]
        assert_eq!(
            exe_path.file_name().and_then(|v| v.to_str()),
            Some("runner")
        );

        let _ = fs::remove_dir_all(&runner_workspace);
        let _ = fs::remove_dir_all(&project_root);
    }

    #[test]
    fn project_runner_workspace_normalizes_file_list_without_dotdot_segments() {
        let project_root = make_temp_dir("normalize-files");
        let source_dir = project_root.join("src");
        fs::create_dir_all(&source_dir).expect("create source dir");
        let test_file = source_dir.join("main.arden");
        fs::write(
            &test_file,
            "@Test\nfunction smoke(): None { return None; }\nfunction main(): None { return None; }\n",
        )
        .expect("write source file");

        let mut config = ProjectConfig::new("smoke");
        config.files = vec!["src/../src/main.arden".to_string()];
        config.entry = "src/../src/main.arden".to_string();

        let (runner_workspace, _runner_output_path) = create_project_test_runner_workspace(
            &project_root,
            &config,
            &test_file,
            "function main(): None { return None; }\n",
        )
        .expect("create project test workspace with dotdot file entries");

        let runner_config = ProjectConfig::load(&runner_workspace.join("arden.toml"))
            .expect("load generated runner config");
        assert!(
            runner_config
                .files
                .iter()
                .all(|file| !file.split('/').any(|segment| segment == "..")),
            "runner config must not contain dotdot segments: {:?}",
            runner_config.files
        );

        let _ = fs::remove_dir_all(&runner_workspace);
        let _ = fs::remove_dir_all(&project_root);
    }
}
