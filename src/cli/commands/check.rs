use crate::build_project;
use crate::cli::output::{cli_accent, cli_path, cli_success, format_cli_path};
use crate::cli::paths::{current_dir_checked, validate_source_file_path};
use crate::project::{find_project_root, ProjectConfig};
use crate::shared::frontend::{parse_program_from_source, run_single_file_semantic_checks};
use colored::Colorize;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug)]
enum CheckCommandError {
    ProjectBuild(String),
    SourcePathValidation(String),
    CurrentDirRead(String),
    ProjectRootMissing(String),
    ProjectConfigLoad(String),
    ProjectConfigValidate(String),
    SourceRead(String),
    Parse(String),
    Semantic(String),
}

impl fmt::Display for CheckCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectBuild(message)
            | Self::SourcePathValidation(message)
            | Self::CurrentDirRead(message)
            | Self::ProjectRootMissing(message)
            | Self::ProjectConfigLoad(message)
            | Self::ProjectConfigValidate(message)
            | Self::SourceRead(message)
            | Self::Parse(message)
            | Self::Semantic(message) => write!(f, "{message}"),
        }
    }
}

impl From<CheckCommandError> for String {
    fn from(value: CheckCommandError) -> Self {
        value.to_string()
    }
}

impl From<String> for CheckCommandError {
    fn from(value: String) -> Self {
        Self::ProjectBuild(value)
    }
}

pub(crate) fn check_command(file: Option<&Path>, show_timings: bool) -> Result<(), String> {
    check_command_impl(file, show_timings).map_err(Into::into)
}

fn check_command_impl(file: Option<&Path>, show_timings: bool) -> Result<(), CheckCommandError> {
    if file.is_none()
        && find_project_root(&current_dir_checked().map_err(CheckCommandError::CurrentDirRead)?)
            .is_some()
    {
        return build_project(false, false, true, true, show_timings)
            .map_err(CheckCommandError::ProjectBuild);
    }
    check_file_impl(file)
}

#[cfg(test)]
pub(crate) fn check_file(file: Option<&Path>) -> Result<(), String> {
    check_file_impl(file).map_err(Into::into)
}

fn check_file_impl(file: Option<&Path>) -> Result<(), CheckCommandError> {
    let file_path = if let Some(file) = file {
        validate_source_file_path(file).map_err(CheckCommandError::SourcePathValidation)?;
        file.to_path_buf()
    } else {
        let cwd = current_dir_checked().map_err(CheckCommandError::CurrentDirRead)?;
        let project_root = find_project_root(&cwd).ok_or_else(|| {
            CheckCommandError::ProjectRootMissing(format!(
                "{}: No arden.toml found from current directory '{}'. Specify a file or run from a project directory.",
                "error".red().bold(),
                format_cli_path(&cwd)
            ))
        })?;

        let config_path = project_root.join("arden.toml");
        let config =
            ProjectConfig::load(&config_path).map_err(CheckCommandError::ProjectConfigLoad)?;
        config
            .validate(&project_root)
            .map_err(CheckCommandError::ProjectConfigValidate)?;
        for source_file in config.get_source_files(&project_root) {
            validate_source_file_path(&source_file)
                .map_err(CheckCommandError::SourcePathValidation)?;
        }
        config.get_entry_path(&project_root)
    };

    println!("{} {}", cli_accent("Checking"), cli_path(&file_path));

    let source = fs::read_to_string(&file_path).map_err(|e| {
        CheckCommandError::SourceRead(format!(
            "{}: Failed to read file '{}': {}",
            "error".red().bold(),
            format_cli_path(&file_path),
            e
        ))
    })?;

    let filename = format_cli_path(&file_path);

    let program =
        parse_program_from_source(&source, &filename).map_err(CheckCommandError::Parse)?;
    run_single_file_semantic_checks(&source, &filename, &program)
        .map_err(CheckCommandError::Semantic)?;

    println!("{} {}", cli_success("Check passed"), cli_path(&file_path));
    Ok(())
}
