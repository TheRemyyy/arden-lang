use crate::build_project;
use crate::cli::output::{cli_accent, cli_path, cli_success};
use crate::cli::paths::{current_dir_checked, validate_source_file_path};
use crate::project::{find_project_root, ProjectConfig};
use crate::shared::frontend::{parse_program_from_source, run_single_file_semantic_checks};
use colored::Colorize;
use std::fs;
use std::path::Path;

pub(crate) fn check_command(file: Option<&Path>, show_timings: bool) -> Result<(), String> {
    if file.is_none() && find_project_root(&current_dir_checked()?).is_some() {
        return build_project(false, false, true, true, show_timings);
    }
    check_file(file)
}

pub(crate) fn check_file(file: Option<&Path>) -> Result<(), String> {
    let file_path = if let Some(file) = file {
        validate_source_file_path(file)?;
        file.to_path_buf()
    } else {
        let cwd = current_dir_checked()?;
        let project_root = find_project_root(&cwd).ok_or_else(|| {
            format!(
                "{}: No arden.toml found from current directory '{}'. Specify a file or run from a project directory.",
                "error".red().bold(),
                cwd.display()
            )
        })?;

        let config_path = project_root.join("arden.toml");
        let config = ProjectConfig::load(&config_path)?;
        config.validate(&project_root)?;
        for source_file in config.get_source_files(&project_root) {
            validate_source_file_path(&source_file)?;
        }
        config.get_entry_path(&project_root)
    };

    println!("{} {}", cli_accent("Checking"), cli_path(&file_path));

    let source = fs::read_to_string(&file_path).map_err(|e| {
        format!(
            "{}: Failed to read file '{}': {}",
            "error".red().bold(),
            file_path.display(),
            e
        )
    })?;

    let filename = file_path.to_string_lossy();

    let program = parse_program_from_source(&source, &filename)?;
    run_single_file_semantic_checks(&source, &filename, &program)?;

    println!("{} {}", cli_success("Check passed"), cli_path(&file_path));
    Ok(())
}
