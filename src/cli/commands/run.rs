use crate::build_project;
use crate::cli::output::{cli_accent, cli_path, cli_warning, format_cli_path};
use crate::cli::paths::{current_dir_checked, unique_temp_binary_path};
use crate::compile_file;
use crate::linker::validate_opt_level;
use crate::project::{
    ensure_project_is_runnable, find_project_root, resolve_project_output_path, ProjectConfig,
};
use crate::shared::process_exit::format_exit_failure;
use colored::Colorize;
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug)]
enum RunCommandError {
    BinaryValidation(String),
    ProcessLaunch(String),
    ProcessExit(String),
    ProjectDiscovery(String),
    ProjectConfigLoad(String),
    ProjectConfigValidate(String),
    OptLevelValidation(String),
    RunnableValidation(String),
    Build(String),
    SingleFileCompile(String),
}

impl fmt::Display for RunCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BinaryValidation(message)
            | Self::ProcessLaunch(message)
            | Self::ProcessExit(message)
            | Self::ProjectDiscovery(message)
            | Self::ProjectConfigLoad(message)
            | Self::ProjectConfigValidate(message)
            | Self::OptLevelValidation(message)
            | Self::RunnableValidation(message)
            | Self::Build(message)
            | Self::SingleFileCompile(message) => write!(f, "{message}"),
        }
    }
}

impl From<RunCommandError> for String {
    fn from(value: RunCommandError) -> Self {
        value.to_string()
    }
}

fn run_binary_impl(exe_path: &Path, args: &[String]) -> Result<(), RunCommandError> {
    if !exe_path.exists() {
        return Err(RunCommandError::BinaryValidation(format!(
            "{}: Executable '{}' does not exist",
            "error".red().bold(),
            format_cli_path(exe_path)
        )));
    }
    if !exe_path.is_file() {
        return Err(RunCommandError::BinaryValidation(format!(
            "{}: Executable path '{}' is not a file",
            "error".red().bold(),
            format_cli_path(exe_path)
        )));
    }
    let status = Command::new(exe_path).args(args).status().map_err(|e| {
        RunCommandError::ProcessLaunch(format!(
            "{}: Failed to run '{}': {}",
            "error".red().bold(),
            format_cli_path(exe_path),
            e
        ))
    })?;
    if !status.success() {
        return Err(RunCommandError::ProcessExit(format!(
            "{}: process '{}' {}",
            "error".red().bold(),
            format_cli_path(exe_path),
            format_exit_failure(status)
        )));
    }
    Ok(())
}

/// Build and run the current project.
pub(crate) fn run_project(
    args: &[String],
    release: bool,
    do_check: bool,
    show_timings: bool,
) -> Result<(), String> {
    run_project_impl(args, release, do_check, show_timings).map_err(Into::into)
}

fn run_project_impl(
    args: &[String],
    release: bool,
    do_check: bool,
    show_timings: bool,
) -> Result<(), RunCommandError> {
    let cwd = current_dir_checked().map_err(RunCommandError::ProjectDiscovery)?;
    let project_root = find_project_root(&cwd).ok_or_else(|| {
        RunCommandError::ProjectDiscovery(format!(
            "{}: No arden.toml found from current directory '{}'",
            "error".red().bold(),
            format_cli_path(&cwd)
        ))
    })?;

    let config_path = project_root.join("arden.toml");
    let config = ProjectConfig::load(&config_path).map_err(RunCommandError::ProjectConfigLoad)?;
    config
        .validate(&project_root)
        .map_err(RunCommandError::ProjectConfigValidate)?;
    validate_opt_level(Some(&config.opt_level)).map_err(RunCommandError::OptLevelValidation)?;
    ensure_project_is_runnable(&config.output_kind).map_err(RunCommandError::RunnableValidation)?;

    build_project(release, false, do_check, false, show_timings).map_err(RunCommandError::Build)?;

    let output_path = resolve_project_output_path(&project_root, &config);
    println!("{} {}", cli_accent("Running"), cli_path(&output_path));
    println!();

    run_binary_impl(&output_path, args)
}

/// Run a single file (legacy mode).
pub(crate) fn run_single_file(
    file: &Path,
    args: &[String],
    release: bool,
    do_check: bool,
) -> Result<(), String> {
    run_single_file_impl(file, args, release, do_check).map_err(Into::into)
}

fn run_single_file_impl(
    file: &Path,
    args: &[String],
    release: bool,
    do_check: bool,
) -> Result<(), RunCommandError> {
    let output = unique_temp_binary_path("arden-run", file).map_err(RunCommandError::Build)?;

    compile_file(
        file,
        Some(&output),
        false,
        do_check,
        release.then_some("3"),
        None,
    )
    .map_err(RunCommandError::SingleFileCompile)?;

    println!("{} {}", cli_accent("Running"), cli_path(&output));
    println!();

    let result = run_binary_impl(&output, args);
    if let Err(err) = fs::remove_file(&output) {
        if err.kind() != std::io::ErrorKind::NotFound {
            eprintln!(
                "{}: failed to remove temporary run binary '{}': {}",
                cli_warning("warning"),
                format_cli_path(&output),
                err
            );
        }
    }
    result
}
