use crate::cli::output::{
    cli_error, cli_path, cli_soft, cli_success, cli_warning, format_cli_path,
};
use crate::cli::paths::{
    collect_arden_files, current_dir_checked, format_target_label, validate_source_file_path,
};
use crate::formatter;
use crate::lint;
use crate::project::{find_project_root, ProjectConfig};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn format_targets(path: Option<&Path>, check_only: bool) -> Result<(), String> {
    let current_dir = current_dir_checked()?;
    let target_label = format_target_label(path, &current_dir);
    let targets = if let Some(path) = path {
        collect_arden_files(path)?
    } else if let Some(project_root) = find_project_root(&current_dir) {
        let config = ProjectConfig::load(&project_root.join("arden.toml"))?;
        config.validate(&project_root)?;
        config.get_source_files(&project_root)
    } else {
        collect_arden_files(&current_dir)?
    };

    if targets.is_empty() {
        return Err("No .arden files found to format".to_string());
    }

    let mut changed = Vec::new();
    for file in targets {
        let source = fs::read_to_string(&file).map_err(|e| {
            format!(
                "{}: Failed to read file '{}': {}",
                "error".red().bold(),
                format_cli_path(&file),
                e
            )
        })?;
        let formatted = formatter::format_source(&source).map_err(|e| {
            format!(
                "{} in '{}': {}",
                "error".red().bold(),
                format_cli_path(&file),
                e
            )
        })?;

        if source != formatted {
            if check_only {
                changed.push(file);
            } else {
                fs::write(&file, formatted).map_err(|e| {
                    format!(
                        "{}: Failed to write '{}': {}",
                        "error".red().bold(),
                        format_cli_path(&file),
                        e
                    )
                })?;
                changed.push(file);
            }
        }
    }

    if check_only {
        if changed.is_empty() {
            println!(
                "{} {}",
                cli_success("Fmt check passed"),
                cli_soft(&target_label)
            );
            return Ok(());
        }

        eprintln!("{} format check failed for:", cli_error("error"));
        for file in changed {
            eprintln!("  - {}", cli_path(&file));
        }
        return Err("format check failed".to_string());
    }

    if changed.is_empty() {
        println!("{} {}", cli_success("Fmt clean"), cli_soft(&target_label));
    } else {
        println!("{} {} file(s)", cli_success("Formatted"), changed.len());
        for file in changed {
            println!("  - {}", cli_path(&file));
        }
    }

    Ok(())
}

pub(crate) fn lint_target(path: Option<&Path>) -> Result<(), String> {
    let file = resolve_default_file(path)?;
    let source = fs::read_to_string(&file).map_err(|e| {
        format!(
            "{}: Failed to read file '{}': {}",
            "error".red().bold(),
            format_cli_path(&file),
            e
        )
    })?;
    let result = lint::lint_source(&source, false).map_err(|e| {
        format!(
            "{} in '{}': {}",
            "error".red().bold(),
            format_cli_path(&file),
            e
        )
    })?;

    if result.findings.is_empty() {
        println!("{} {}", cli_success("Lint clean"), cli_path(&file));
        return Ok(());
    }

    eprintln!(
        "{} lint findings in {}:",
        cli_warning("warning"),
        cli_path(&file)
    );
    for finding in result.findings {
        eprintln!("  {}", finding.format());
    }
    Err("lint failed".to_string())
}

pub(crate) fn fix_target(path: Option<&Path>) -> Result<(), String> {
    let file = resolve_default_file(path)?;
    let source = fs::read_to_string(&file).map_err(|e| {
        format!(
            "{}: Failed to read file '{}': {}",
            "error".red().bold(),
            format_cli_path(&file),
            e
        )
    })?;
    let result = lint::lint_source(&source, true).map_err(|e| {
        format!(
            "{} in '{}': {}",
            "error".red().bold(),
            format_cli_path(&file),
            e
        )
    })?;
    let fixed_source = result.fixed_source.unwrap_or(source.clone());

    let formatted_source = formatter::format_source(&fixed_source).map_err(|e| {
        format!(
            "{} in '{}': {}",
            "error".red().bold(),
            format_cli_path(&file),
            e
        )
    })?;

    if source == formatted_source {
        println!("{} {}", cli_success("Fix clean"), cli_path(&file));
        return Ok(());
    }

    fs::write(&file, formatted_source).map_err(|e| {
        format!(
            "{}: Failed to write '{}': {}",
            "error".red().bold(),
            format_cli_path(&file),
            e
        )
    })?;
    println!("{} {}", cli_success("Fixed"), cli_path(&file));
    Ok(())
}

fn resolve_default_file(path: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(path) = path {
        validate_source_file_path(path)?;
        return Ok(path.to_path_buf());
    }

    let current_dir = current_dir_checked()?;
    if let Some(project_root) = find_project_root(&current_dir) {
        let config = ProjectConfig::load(&project_root.join("arden.toml"))?;
        config.validate(&project_root)?;
        for source_file in config.get_source_files(&project_root) {
            validate_source_file_path(&source_file)?;
        }
        return Ok(config.get_entry_path(&project_root));
    }

    Err("No file specified and no arden.toml found in the current directory".to_string())
}
