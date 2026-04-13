use crate::build_project;
use crate::cli::output::{cli_accent, cli_path, cli_warning, format_cli_path};
use crate::cli::paths::{current_dir_checked, unique_temp_binary_path};
use crate::compile_file;
use crate::linker::validate_opt_level;
use crate::project::{
    ensure_project_is_runnable, find_project_root, resolve_project_output_path, ProjectConfig,
};
use colored::Colorize;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitStatus};

fn format_exit_failure(status: ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            let reason = if signal == 11 {
                "segmentation fault"
            } else {
                "runtime signal"
            };
            return format!(
                "terminated by signal {signal} ({reason}). \
this indicates a runtime crash; rerun with `arden compile --emit-llvm ...` and report it."
            );
        }
    }

    if let Some(code) = status.code() {
        return format!("exited with code {code}");
    }

    "terminated without an exit code".to_string()
}

fn run_binary(exe_path: &Path, args: &[String]) -> Result<(), String> {
    let status = Command::new(exe_path).args(args).status().map_err(|e| {
        format!(
            "{}: Failed to run '{}': {}",
            "error".red().bold(),
            format_cli_path(exe_path),
            e
        )
    })?;
    if !status.success() {
        return Err(format!(
            "{}: process '{}' {}",
            "error".red().bold(),
            format_cli_path(exe_path),
            format_exit_failure(status)
        ));
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
    let cwd = current_dir_checked()?;
    let project_root = find_project_root(&cwd).ok_or_else(|| {
        format!(
            "{}: No arden.toml found from current directory '{}'",
            "error".red().bold(),
            format_cli_path(&cwd)
        )
    })?;

    let config_path = project_root.join("arden.toml");
    let config = ProjectConfig::load(&config_path)?;
    config.validate(&project_root)?;
    validate_opt_level(Some(&config.opt_level))?;
    ensure_project_is_runnable(&config.output_kind)?;

    build_project(release, false, do_check, false, show_timings)?;

    let output_path = resolve_project_output_path(&project_root, &config);
    println!("{} {}", cli_accent("Running"), cli_path(&output_path));
    println!();

    run_binary(&output_path, args)
}

/// Run a single file (legacy mode).
pub(crate) fn run_single_file(
    file: &Path,
    args: &[String],
    release: bool,
    do_check: bool,
) -> Result<(), String> {
    let output = unique_temp_binary_path("arden-run", file)?;

    compile_file(
        file,
        Some(&output),
        false,
        do_check,
        release.then_some("3"),
        None,
    )?;

    println!("{} {}", cli_accent("Running"), cli_path(&output));
    println!();

    let result = run_binary(&output, args);
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
