use crate::cli::output::format_cli_path;
use crate::cli::paths::current_dir_checked;
use crate::cli::test_discovery::find_test_files as discover_test_files;
use crate::project::{find_project_root, resolve_project_output_path, ProjectConfig};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug)]
enum TestWorkspaceError {
    Discovery(String),
    ProjectConfigLoad(String),
    ProjectConfigValidate(String),
    InvalidRelativePath(String),
    UniquePathGeneration(String),
    WorkspaceCreate(String),
    CurrentDir(String),
    CanonicalProjectRoot(String),
    CanonicalTestFile(String),
    CanonicalProjectEntry(String),
    TestOutsideProject(String),
    SourceCanonicalization(String),
    SourceOutsideProject(String),
    WorkspaceEscape(String),
    SourceDirCreate(String),
    RunnerWrite(String),
    SourceCopy(String),
    RunnerDestinationCreate(String),
    ConfigSave(String),
}

impl fmt::Display for TestWorkspaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(message)
            | Self::ProjectConfigLoad(message)
            | Self::ProjectConfigValidate(message)
            | Self::InvalidRelativePath(message)
            | Self::UniquePathGeneration(message)
            | Self::WorkspaceCreate(message)
            | Self::CurrentDir(message)
            | Self::CanonicalProjectRoot(message)
            | Self::CanonicalTestFile(message)
            | Self::CanonicalProjectEntry(message)
            | Self::TestOutsideProject(message)
            | Self::SourceCanonicalization(message)
            | Self::SourceOutsideProject(message)
            | Self::WorkspaceEscape(message)
            | Self::SourceDirCreate(message)
            | Self::RunnerWrite(message)
            | Self::SourceCopy(message)
            | Self::RunnerDestinationCreate(message)
            | Self::ConfigSave(message) => write!(f, "{message}"),
        }
    }
}

impl From<TestWorkspaceError> for String {
    fn from(value: TestWorkspaceError) -> Self {
        value.to_string()
    }
}

fn normalized_relative_path_string(
    path: &Path,
    context: &str,
) -> Result<String, TestWorkspaceError> {
    let as_str = path.to_str().ok_or_else(|| {
        TestWorkspaceError::InvalidRelativePath(format!(
            "Path '{}' for {} is not valid UTF-8",
            format_cli_path(path),
            context
        ))
    })?;
    Ok(as_str.replace('\\', "/"))
}

pub(super) fn find_test_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    find_test_files_impl(path).map_err(Into::into)
}

fn find_test_files_impl(path: &Path) -> Result<Vec<PathBuf>, TestWorkspaceError> {
    discover_test_files(path).map_err(TestWorkspaceError::Discovery)
}

pub(super) fn default_test_files(current_dir: &Path) -> Result<Vec<PathBuf>, String> {
    default_test_files_impl(current_dir).map_err(Into::into)
}

fn default_test_files_impl(current_dir: &Path) -> Result<Vec<PathBuf>, TestWorkspaceError> {
    if let Some(project_root) = find_project_root(current_dir) {
        let config_path = project_root.join("arden.toml");
        let config =
            ProjectConfig::load(&config_path).map_err(TestWorkspaceError::ProjectConfigLoad)?;
        config
            .validate(&project_root)
            .map_err(TestWorkspaceError::ProjectConfigValidate)?;

        let mut files = config.get_source_files(&project_root);
        files.sort();
        return Ok(files);
    }

    discover_test_files(current_dir).map_err(TestWorkspaceError::Discovery)
}

pub(super) fn create_test_runner_workspace(
    test_file: &Path,
) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    create_test_runner_workspace_impl(test_file).map_err(Into::into)
}

fn create_test_runner_workspace_impl(
    test_file: &Path,
) -> Result<(PathBuf, PathBuf, PathBuf), TestWorkspaceError> {
    let unique = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| {
            TestWorkspaceError::UniquePathGeneration(format!(
                "Failed to create unique test runner path for '{}': {}",
                format_cli_path(test_file),
                e,
            ))
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
        TestWorkspaceError::WorkspaceCreate(format!(
            "Failed to create test runner workspace '{}': {}",
            format_cli_path(&temp_dir),
            e
        ))
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
    create_project_test_runner_workspace_impl(project_root, config, test_file, runner_code)
        .map_err(Into::into)
}

fn create_project_test_runner_workspace_impl(
    project_root: &Path,
    config: &ProjectConfig,
    test_file: &Path,
    runner_code: &str,
) -> Result<(PathBuf, PathBuf), TestWorkspaceError> {
    let unique = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| {
            TestWorkspaceError::UniquePathGeneration(format!(
                "Failed to create unique test runner project path for '{}': {}",
                format_cli_path(test_file),
                e
            ))
        })?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "arden-project-test-runner-{}-{}",
        std::process::id(),
        unique
    ));
    fs::create_dir_all(&temp_dir).map_err(|e| {
        TestWorkspaceError::WorkspaceCreate(format!(
            "Failed to create test runner project workspace '{}': {}",
            format_cli_path(&temp_dir),
            e
        ))
    })?;

    let normalized_test_file = if test_file.is_absolute() {
        test_file.to_path_buf()
    } else {
        current_dir_checked()
            .map_err(TestWorkspaceError::CurrentDir)?
            .join(test_file)
    };
    let canonical_project_root = project_root.canonicalize().map_err(|e| {
        TestWorkspaceError::CanonicalProjectRoot(format!(
            "Failed to resolve project root '{}' for test workspace creation: {}",
            format_cli_path(project_root),
            e
        ))
    })?;
    let canonical_test_file = normalized_test_file.canonicalize().map_err(|e| {
        TestWorkspaceError::CanonicalTestFile(format!(
            "Failed to resolve test file '{}' for test workspace creation: {}",
            format_cli_path(&normalized_test_file),
            e
        ))
    })?;
    let canonical_original_entry =
        project_root
            .join(&config.entry)
            .canonicalize()
            .map_err(|e| {
                TestWorkspaceError::CanonicalProjectEntry(format!(
                    "Failed to resolve project entry '{}' for test workspace creation: {}",
                    config.entry, e
                ))
            })?;

    let test_rel = canonical_test_file
        .strip_prefix(&canonical_project_root)
        .map_err(|_| {
            TestWorkspaceError::TestOutsideProject(format!(
                "Test file '{}' is outside project root '{}'",
                format_cli_path(&canonical_test_file),
                format_cli_path(&canonical_project_root)
            ))
        })?;
    let test_rel_string = normalized_relative_path_string(test_rel, "test runner entry path")?;
    let mut copied_files: Vec<String> = Vec::new();

    for source_file in config.get_source_files(project_root) {
        let canonical_source_file = source_file.canonicalize().map_err(|e| {
            TestWorkspaceError::SourceCanonicalization(format!(
                "Failed to resolve project source '{}': {}",
                format_cli_path(&source_file),
                e
            ))
        })?;
        if canonical_original_entry == canonical_source_file
            && canonical_source_file != canonical_test_file
        {
            continue;
        }
        let rel = canonical_source_file
            .strip_prefix(&canonical_project_root)
            .map_err(|_| {
                TestWorkspaceError::SourceOutsideProject(format!(
                    "Project source '{}' is outside project root '{}'",
                    format_cli_path(&canonical_source_file),
                    format_cli_path(&canonical_project_root)
                ))
            })?;
        let rel_string = normalized_relative_path_string(rel, "project source path")?;
        copied_files.push(rel_string.clone());
        let rel_path = Path::new(&rel_string);
        let dest = temp_dir.join(rel_path);
        if !dest.starts_with(&temp_dir) {
            return Err(TestWorkspaceError::WorkspaceEscape(format!(
                "Refusing to write source '{}' outside test workspace '{}'",
                format_cli_path(&canonical_source_file),
                format_cli_path(&temp_dir)
            )));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                TestWorkspaceError::SourceDirCreate(format!(
                    "Failed to create runner source directory '{}': {}",
                    format_cli_path(parent),
                    e
                ))
            })?;
        }
        if canonical_source_file == canonical_test_file {
            fs::write(&dest, runner_code).map_err(|e| {
                TestWorkspaceError::RunnerWrite(format!(
                    "Failed to write generated project test runner '{}': {}",
                    format_cli_path(&dest),
                    e
                ))
            })?;
        } else {
            fs::copy(&canonical_source_file, &dest).map_err(|e| {
                TestWorkspaceError::SourceCopy(format!(
                    "Failed to copy project source '{}' into test workspace '{}': {}",
                    format_cli_path(&canonical_source_file),
                    format_cli_path(&dest),
                    e
                ))
            })?;
        }
    }

    let runner_dest = temp_dir.join(test_rel);
    if !runner_dest.starts_with(&temp_dir) {
        return Err(TestWorkspaceError::WorkspaceEscape(format!(
            "Refusing to place generated runner outside test workspace '{}'",
            format_cli_path(&temp_dir)
        )));
    }
    if !runner_dest.exists() {
        if let Some(parent) = runner_dest.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                TestWorkspaceError::RunnerDestinationCreate(format!(
                    "Failed to create runner destination directory '{}': {}",
                    format_cli_path(parent),
                    e
                ))
            })?;
        }
        fs::write(&runner_dest, runner_code).map_err(|e| {
            TestWorkspaceError::RunnerWrite(format!(
                "Failed to write generated runner source '{}': {}",
                format_cli_path(&runner_dest),
                e
            ))
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
            TestWorkspaceError::ConfigSave(format!(
                "Failed to write test runner project config '{}': {}",
                format_cli_path(&temp_dir.join("arden.toml")),
                e
            ))
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
