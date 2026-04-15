use crate::cli::output::{cli_accent, cli_path, cli_soft, cli_tertiary, format_cli_path};
use crate::cli::paths::{current_dir_checked, validate_source_file_path};
use crate::linker::validate_opt_level;
use crate::project::{find_project_root, ProjectConfig};
use colored::Colorize;
use std::fmt;

#[derive(Debug)]
enum InfoCommandError {
    ProjectDiscovery(String),
    ProjectConfigLoad(String),
    ProjectConfigValidate(String),
    OptLevelValidation(String),
    SourcePathValidation(String),
}

impl fmt::Display for InfoCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectDiscovery(message)
            | Self::ProjectConfigLoad(message)
            | Self::ProjectConfigValidate(message)
            | Self::OptLevelValidation(message)
            | Self::SourcePathValidation(message) => write!(f, "{message}"),
        }
    }
}

impl From<InfoCommandError> for String {
    fn from(value: InfoCommandError) -> Self {
        value.to_string()
    }
}

pub(crate) fn show_project_info() -> Result<(), String> {
    show_project_info_impl().map_err(Into::into)
}

fn show_project_info_impl() -> Result<(), InfoCommandError> {
    let cwd = current_dir_checked().map_err(InfoCommandError::ProjectDiscovery)?;
    let project_root = find_project_root(&cwd).ok_or_else(|| {
        InfoCommandError::ProjectDiscovery(format!(
            "{}: No arden.toml found in current directory '{}' or its parents.",
            "error".red().bold(),
            format_cli_path(&cwd)
        ))
    })?;

    let config_path = project_root.join("arden.toml");
    let config = ProjectConfig::load(&config_path).map_err(InfoCommandError::ProjectConfigLoad)?;
    config
        .validate(&project_root)
        .map_err(InfoCommandError::ProjectConfigValidate)?;
    validate_opt_level(Some(&config.opt_level)).map_err(InfoCommandError::OptLevelValidation)?;
    for file in config.get_source_files(&project_root) {
        validate_source_file_path(&file).map_err(InfoCommandError::SourcePathValidation)?;
    }

    println!("{}", cli_accent("Project"));
    println!("  {}: {}", cli_tertiary("name"), cli_soft(&config.name));
    println!(
        "  {}: {}",
        cli_tertiary("version"),
        cli_soft(&config.version)
    );
    println!("  {}: {}", cli_tertiary("entry"), cli_soft(&config.entry));
    println!("  {}: {}", cli_tertiary("output"), cli_soft(&config.output));
    println!(
        "  {}: {}",
        cli_tertiary("output kind"),
        cli_soft(format!("{:?}", config.output_kind))
    );
    println!(
        "  {}: {}",
        cli_tertiary("opt level"),
        cli_soft(&config.opt_level)
    );
    println!(
        "  {}: {}",
        cli_tertiary("target"),
        cli_soft(config.target.as_deref().unwrap_or("native/default"))
    );
    println!("  {}: {}", cli_tertiary("root"), cli_path(&project_root));

    println!("\n{}", cli_tertiary("source files"));
    for file in &config.files {
        println!("  - {}", cli_soft(file));
    }

    if !config.dependencies.is_empty() {
        println!("\n{}", cli_tertiary("dependencies"));
        for (name, version) in &config.dependencies {
            println!("  - {} = {}", cli_soft(name), cli_soft(version));
        }
    }

    if !config.link_search.is_empty() {
        println!("\n{}", cli_tertiary("link search"));
        for path in &config.link_search {
            println!("  - {}", cli_soft(path));
        }
    }

    if !config.link_libs.is_empty() {
        println!("\n{}", cli_tertiary("link libraries"));
        for lib in &config.link_libs {
            println!("  - {}", cli_soft(lib));
        }
    }

    Ok(())
}
