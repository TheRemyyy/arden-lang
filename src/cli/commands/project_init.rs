use crate::cli::output::{
    cli_accent, cli_new_run_hint, cli_path, cli_soft, cli_success, cli_tertiary, format_cli_path,
};
use crate::project::ProjectConfig;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn create_new_project(name: &str, path: Option<&Path>) -> Result<(), String> {
    validate_new_project_name(name)?;

    let project_path = path
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(name));

    if project_path.exists() {
        let kind = if project_path.is_dir() {
            "Directory"
        } else {
            "Path"
        };
        return Err(format!(
            "{}: {} '{}' already exists",
            "error".red().bold(),
            kind,
            format_cli_path(&project_path)
        ));
    }

    fs::create_dir_all(&project_path).map_err(|e| {
        format!(
            "{}: Failed to create project directory '{}': {}",
            "error".red().bold(),
            format_cli_path(&project_path),
            e
        )
    })?;
    let src_dir = project_path.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| {
        format!(
            "{}: Failed to create src directory '{}': {}",
            "error".red().bold(),
            format_cli_path(&src_dir),
            e
        )
    })?;

    let config = ProjectConfig::new(name);
    let config_path = project_path.join("arden.toml");
    config.save(&config_path)?;

    let main_content = format!(
        r#"import std.io.*;

function main(): None {{
    println("hello from {}");
    return None;
}}
"#,
        name
    );

    let main_path = project_path.join("src").join("main.arden");
    fs::write(&main_path, main_content).map_err(|e| {
        format!(
            "{}: Failed to create main.arden '{}': {}",
            "error".red().bold(),
            format_cli_path(&main_path),
            e
        )
    })?;

    let readme_content = format!(
        r#"# {}

Project scaffold created with `arden new`.

This directory is intentionally small, but it already contains the pieces Arden uses for project-mode development.

## Project Structure

```
.
├── arden.toml      # Project configuration
├── src/
│   └── main.arden  # Entry point
└── README.md       # Project notes and workflow guide
```

## What Each File Does

- `arden.toml` declares the project name, entry file, output path, optimization level, and explicit source list.
- `src/main.arden` is the default entrypoint used by `arden run` and `arden build`.
- `README.md` is where you should document the local workflow, architecture notes, and useful commands as the project grows.

## Common Workflow

- `arden build` - Build the project
- `arden run` - Build and run the project
- `arden check` - Parse and type-check the project
- `arden test` - Run test files in the project
- `arden fmt` - Format project sources
- `arden info` - Print the resolved project configuration

A good first pass after generating the project is:

```bash
arden info
arden run
arden check
```

## How Project Mode Works

When you run Arden inside this directory without passing a file path, the compiler reads `arden.toml` and uses it as the source of truth for:

- project name and version
- entrypoint
- native output name
- output kind
- which source files belong to the build

That makes project builds explicit and reproducible. As the project grows, add new files to `files = [...]` in `arden.toml`.

## Configuration

Edit `arden.toml` to customize your project:

```toml
name = "{}"
version = "0.1.0"
entry = "src/main.arden"
files = ["src/main.arden"]
output = "{}"
opt_level = "3"
output_kind = "bin"
```

## Next Steps

- add more `.arden` files under `src/`
- list them in `arden.toml`
- use `arden test` for test files and `arden fmt` before commits
- read the upstream documentation in the main Arden repository for language and stdlib details
"#,
        name, name, name
    );

    let readme_path = project_path.join("README.md");
    fs::write(&readme_path, readme_content).map_err(|e| {
        format!(
            "{}: Failed to create README.md '{}': {}",
            "error".red().bold(),
            format_cli_path(&readme_path),
            e
        )
    })?;

    println!("{} {}", cli_success("Created project"), cli_accent(name));
    let root_display = project_path
        .canonicalize()
        .unwrap_or_else(|_| project_path.clone());
    println!("  {} {}", cli_tertiary("Root"), cli_path(&root_display));
    println!("\n{}", cli_tertiary("Next"));
    println!(
        "  {} {}",
        cli_tertiary("cd"),
        cli_soft(format_cli_path(path.unwrap_or(Path::new(name))))
    );
    println!("  {}", cli_soft(cli_new_run_hint()));

    Ok(())
}

fn validate_new_project_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err(format!(
            "{}: Project name cannot be empty",
            "error".red().bold()
        ));
    }

    let is_valid = name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');
    if is_valid {
        return Ok(());
    }

    Err(format!(
        "{}: Invalid project name '{}'. Use only ASCII letters, digits, '_' or '-'.",
        "error".red().bold(),
        name
    ))
}
