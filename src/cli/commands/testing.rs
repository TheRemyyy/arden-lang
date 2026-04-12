mod execution;
mod workspace;

use crate::build_project;
use crate::cli::output::{cli_accent, cli_soft, cli_tertiary, cli_warning};
use crate::cli::paths::{current_dir_checked, validate_source_file_path, with_process_current_dir};
use crate::diagnostics::format_parse_error;
use crate::lexer;
use crate::parser::Parser;
use crate::project::{find_project_root, ProjectConfig};
use crate::test_runner::{
    discover_tests, generate_test_runner_with_source, print_discovery,
    validate_test_runner_attributes, TestDiscovery,
};
use execution::compile_and_run_test;
use std::fs;
use std::path::{Path, PathBuf};
use workspace::{
    create_project_test_runner_workspace, create_test_runner_workspace, default_test_files,
};

/// Run tests for a file or project.
pub(crate) fn run_tests(
    test_path: Option<&Path>,
    list_only: bool,
    filter: Option<&str>,
) -> Result<(), String> {
    let test_files = resolve_test_files(test_path)?;

    if test_files.is_empty() {
        println!("{}", cli_warning("No test files found"));
        println!(
            "{}",
            cli_soft("Create files with functions marked `@Test`.")
        );
        return Ok(());
    }

    let mut all_tests_found = false;

    for test_file in &test_files {
        let source = fs::read_to_string(test_file)
            .map_err(|e| format!("Failed to read test file: {}", e))?;

        let tokens = lexer::tokenize(&source).map_err(|e| format!("Lexer error: {}", e))?;
        let filename = test_file
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("input.arden");
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse_program()
            .map_err(|e| format_parse_error(&e, &source, filename))?;
        validate_test_runner_attributes(&program)?;

        let discovery = discover_tests(&program);
        if discovery.total_tests == 0 {
            continue;
        }
        all_tests_found = true;

        let filtered_suites: Vec<_> = if let Some(pattern) = filter {
            discovery
                .suites
                .into_iter()
                .map(|mut suite| {
                    suite.tests.retain(|test| test.name.contains(pattern));
                    suite
                })
                .filter(|suite| !suite.tests.is_empty())
                .collect()
        } else {
            discovery.suites
        };

        if filtered_suites.is_empty() {
            println!(
                "{}: no tests matched '{}'",
                test_file.display(),
                filter.unwrap_or("")
            );
            continue;
        }

        let filtered_total_tests: usize =
            filtered_suites.iter().map(|suite| suite.tests.len()).sum();
        let filtered_ignored_tests: usize = filtered_suites
            .iter()
            .map(|suite| suite.tests.iter().filter(|test| test.ignored).count())
            .sum();

        let filtered_discovery = TestDiscovery {
            suites: filtered_suites,
            total_tests: filtered_total_tests,
            ignored_tests: filtered_ignored_tests,
        };
        let filtered_out_tests = discovery
            .total_tests
            .saturating_sub(filtered_discovery.total_tests);

        if list_only {
            println!("\n{}", cli_accent(test_file.display().to_string()));
            print_discovery(&filtered_discovery);
        } else {
            let runner_code = generate_test_runner_with_source(&filtered_discovery, &source);
            if let Some(project_root) = test_file.parent().and_then(find_project_root) {
                let config_path = project_root.join("arden.toml");
                let config = ProjectConfig::load(&config_path)?;
                config.validate(&project_root)?;
                let (temp_dir, exe_path) = create_project_test_runner_workspace(
                    &project_root,
                    &config,
                    test_file,
                    &runner_code,
                )?;
                let build_result = with_process_current_dir(&temp_dir, || {
                    build_project(false, false, true, false, false)
                });
                let result = build_result
                    .and_then(|_| execution::run_test_executable(&exe_path, filtered_out_tests));
                if let Err(err) = fs::remove_dir_all(&temp_dir) {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        eprintln!(
                            "{}: failed to remove temporary test workspace '{}': {}",
                            cli_warning("warning"),
                            temp_dir.display(),
                            err
                        );
                    }
                }
                result?;
            } else {
                let (temp_dir, runner_path, exe_path) = create_test_runner_workspace(test_file)?;
                fs::write(&runner_path, &runner_code)
                    .map_err(|e| format!("Failed to write test runner: {}", e))?;

                let result = compile_and_run_test(&runner_path, &exe_path, filtered_out_tests);
                if let Err(err) = fs::remove_dir_all(&temp_dir) {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        eprintln!(
                            "{}: failed to remove temporary test workspace '{}': {}",
                            cli_warning("warning"),
                            temp_dir.display(),
                            err
                        );
                    }
                }
                result?;
            }
        }
    }

    if !all_tests_found {
        println!("{}", cli_warning("No tests discovered"));
        println!("{}", cli_soft("Mark functions with `@Test`:"));
        println!(
            "  {} function myTest(): None {{ ... }}",
            cli_tertiary("@Test")
        );
    }

    Ok(())
}

pub(super) fn resolve_test_files(test_path: Option<&Path>) -> Result<Vec<PathBuf>, String> {
    if let Some(path) = test_path {
        if path.is_file() {
            validate_source_file_path(path)?;
            Ok(vec![path.to_path_buf()])
        } else {
            workspace::find_test_files(path)
        }
    } else {
        let current_dir = current_dir_checked()?;
        default_test_files(&current_dir)
    }
}
