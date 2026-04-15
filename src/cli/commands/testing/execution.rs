use crate::cli::output::{
    cli_accent, cli_elapsed, cli_soft, format_cli_path, print_test_runner_output,
};
use crate::compile_source;
use crate::shared::process_exit::format_exit_failure;
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

#[derive(Debug)]
enum TestExecutionError {
    SourceRead(String),
    Compile(String),
    BinaryValidation(String),
    Launch(String),
    Failed(String),
}

impl fmt::Display for TestExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceRead(message)
            | Self::Compile(message)
            | Self::BinaryValidation(message)
            | Self::Launch(message)
            | Self::Failed(message) => write!(f, "{message}"),
        }
    }
}

impl From<TestExecutionError> for String {
    fn from(value: TestExecutionError) -> Self {
        value.to_string()
    }
}

impl From<String> for TestExecutionError {
    fn from(value: String) -> Self {
        Self::Compile(value)
    }
}

pub(super) fn compile_and_run_test(
    source_path: &Path,
    exe_path: &Path,
    filtered_out: usize,
) -> Result<(), String> {
    compile_and_run_test_impl(source_path, exe_path, filtered_out).map_err(Into::into)
}

fn compile_and_run_test_impl(
    source_path: &Path,
    exe_path: &Path,
    filtered_out: usize,
) -> Result<(), TestExecutionError> {
    let source = fs::read_to_string(source_path).map_err(|e| {
        TestExecutionError::SourceRead(format!(
            "Failed to read test runner '{}': {}",
            format_cli_path(source_path),
            e
        ))
    })?;
    compile_source(&source, source_path, exe_path, false, true, None, None)
        .map_err(TestExecutionError::Compile)?;
    run_test_executable_impl(exe_path, filtered_out)
}

pub(super) fn run_test_executable(exe_path: &Path, filtered_out: usize) -> Result<(), String> {
    run_test_executable_impl(exe_path, filtered_out).map_err(Into::into)
}

fn run_test_executable_impl(
    exe_path: &Path,
    filtered_out: usize,
) -> Result<(), TestExecutionError> {
    if !exe_path.exists() {
        return Err(TestExecutionError::BinaryValidation(format!(
            "Failed to run test runner '{}': executable does not exist",
            format_cli_path(exe_path)
        )));
    }
    if !exe_path.is_file() {
        return Err(TestExecutionError::BinaryValidation(format!(
            "Failed to run test runner '{}': executable path is not a file",
            format_cli_path(exe_path)
        )));
    }
    let started_at = Instant::now();
    println!();

    let working_dir = exe_path
        .parent()
        .filter(|dir| dir.is_dir())
        .map(Path::to_path_buf)
        .unwrap_or_else(std::env::temp_dir);
    let output = Command::new(exe_path)
        .current_dir(working_dir)
        .output()
        .map_err(|e| {
            TestExecutionError::Launch(format!(
                "Failed to run test runner '{}': {}",
                format_cli_path(exe_path),
                e
            ))
        })?;

    let report = print_test_runner_output(
        &String::from_utf8_lossy(&output.stdout),
        output.status.success(),
    );
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    let elapsed = started_at.elapsed();

    println!();
    println!("{}", cli_accent("test result:"));
    println!(" {}", cli_soft(format!("{} passed;", report.passed)));
    println!(" {}", cli_soft(format!("{} failed;", report.failed)));
    println!(" {}", cli_soft(format!("{} ignored;", report.ignored)));
    println!(" {}", cli_soft("0 measured;"));
    println!(" {}", cli_soft(format!("{} filtered out;", filtered_out)));
    println!(
        " {}",
        cli_soft(format!("finished in {}", cli_elapsed(elapsed)))
    );

    if !output.status.success() {
        return Err(TestExecutionError::Failed(format!(
            "test run failed for '{}': {}",
            format_cli_path(exe_path),
            format_exit_failure(output.status)
        )));
    }

    Ok(())
}
