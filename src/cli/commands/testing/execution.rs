use crate::cli::output::{
    cli_accent, cli_elapsed, cli_soft, format_cli_path, print_test_runner_output,
};
use crate::compile_source;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitStatus};
use std::time::Instant;

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

pub(super) fn compile_and_run_test(
    source_path: &Path,
    exe_path: &Path,
    filtered_out: usize,
) -> Result<(), String> {
    let source = fs::read_to_string(source_path).map_err(|e| {
        format!(
            "Failed to read test runner '{}': {}",
            format_cli_path(source_path),
            e
        )
    })?;
    compile_source(&source, source_path, exe_path, false, true, None, None)?;
    run_test_executable(exe_path, filtered_out)
}

pub(super) fn run_test_executable(exe_path: &Path, filtered_out: usize) -> Result<(), String> {
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
            format!(
                "Failed to run test runner '{}': {}",
                format_cli_path(exe_path),
                e
            )
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
        return Err(format!(
            "test run failed for '{}': {}",
            format_cli_path(exe_path),
            format_exit_failure(output.status)
        ));
    }

    Ok(())
}
