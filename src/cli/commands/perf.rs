use crate::cli::output::{cli_accent, cli_elapsed, cli_soft, cli_tertiary, cli_warning};
use crate::cli::paths::{current_dir_checked, unique_temp_binary_path};
use crate::project::{
    ensure_project_is_runnable, find_project_root, resolve_project_output_path, ProjectConfig,
};
use crate::{build_project, compile_file};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
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

fn run_binary(exe_path: &Path, args: &[String]) -> Result<(), String> {
    let status = Command::new(exe_path).args(args).status().map_err(|e| {
        format!(
            "{}: Failed to run '{}': {}",
            "error".red().bold(),
            exe_path.display(),
            e
        )
    })?;
    if !status.success() {
        return Err(format!(
            "{}: process '{}' {}",
            "error".red().bold(),
            exe_path.display(),
            format_exit_failure(status)
        ));
    }
    Ok(())
}

fn prepare_perf_binary(
    file: Option<&Path>,
    release: bool,
    temp_tag: &str,
) -> Result<(PathBuf, Option<PathBuf>, Vec<String>), String> {
    if let Some(file) = file {
        let output = unique_temp_binary_path(temp_tag, file)?;
        compile_file(
            file,
            Some(&output),
            false,
            true,
            release.then_some("3"),
            None,
        )?;
        return Ok((output.clone(), Some(output), Vec::new()));
    }

    let cwd = current_dir_checked()?;
    let project_root = find_project_root(&cwd).ok_or_else(|| {
        format!(
            "{}: No arden.toml found from current directory '{}'",
            "error".red().bold(),
            cwd.display()
        )
    })?;
    let config_path = project_root.join("arden.toml");
    let config = ProjectConfig::load(&config_path)?;
    config.validate(&project_root)?;
    ensure_project_is_runnable(&config.output_kind)?;
    build_project(release, false, true, false, false)?;
    Ok((
        resolve_project_output_path(&project_root, &config),
        None,
        Vec::new(),
    ))
}

pub(crate) fn bench_target(file: Option<&Path>, iterations: usize) -> Result<(), String> {
    if iterations == 0 {
        return Err("Iterations must be greater than zero.".to_string());
    }

    let (exe_path, cleanup_path, args) = prepare_perf_binary(file, false, "arden-bench")?;
    let run_result = (|| -> Result<Vec<f64>, String> {
        let mut samples_ms = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let start = Instant::now();
            run_binary(&exe_path, &args)?;
            samples_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        Ok(samples_ms)
    })();
    if let Some(cleanup_path) = cleanup_path {
        if let Err(err) = fs::remove_file(&cleanup_path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "{}: failed to remove temporary benchmark binary '{}': {}",
                    cli_warning("warning"),
                    cleanup_path.display(),
                    err
                );
            }
        }
    }
    let samples_ms = run_result?;

    let min = samples_ms
        .iter()
        .copied()
        .fold(f64::INFINITY, |acc, value| acc.min(value));
    let max = samples_ms
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |acc, value| acc.max(value));
    let mean = samples_ms.iter().sum::<f64>() / samples_ms.len() as f64;

    println!("{}", cli_accent("Benchmark"));
    println!(
        "  {} {}",
        cli_tertiary("runs"),
        cli_soft(samples_ms.len().to_string())
    );
    println!(
        "  {} {}",
        cli_tertiary("min"),
        cli_soft(format!("{:.6} s", min / 1000.0))
    );
    println!(
        "  {} {}",
        cli_tertiary("mean"),
        cli_soft(format!("{:.6} s", mean / 1000.0))
    );
    println!(
        "  {} {}",
        cli_tertiary("max"),
        cli_soft(format!("{:.6} s", max / 1000.0))
    );
    Ok(())
}

pub(crate) fn profile_target(file: Option<&Path>) -> Result<(), String> {
    let build_started = Instant::now();
    let (exe_path, cleanup_path, args) = prepare_perf_binary(file, false, "arden-profile")?;
    let build_elapsed = build_started.elapsed();
    let run_result = (|| -> Result<std::time::Duration, String> {
        let run_started = Instant::now();
        run_binary(&exe_path, &args)?;
        Ok(run_started.elapsed())
    })();
    if let Some(cleanup_path) = cleanup_path {
        if let Err(err) = fs::remove_file(&cleanup_path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "{}: failed to remove temporary profile binary '{}': {}",
                    cli_warning("warning"),
                    cleanup_path.display(),
                    err
                );
            }
        }
    }
    let run_elapsed = run_result?;

    println!("{}", cli_accent("Timing profile"));
    println!(
        "  {} {}",
        cli_tertiary("build"),
        cli_soft(cli_elapsed(build_elapsed))
    );
    println!(
        "  {} {}",
        cli_tertiary("run"),
        cli_soft(cli_elapsed(run_elapsed))
    );
    println!(
        "  {} {}",
        cli_tertiary("total"),
        cli_soft(cli_elapsed(build_elapsed + run_elapsed))
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(prefix: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let base = std::env::current_dir()
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("target")
            .join("test-temp");
        let path = base.join(format!("arden-{prefix}-{}-{suffix}", std::process::id()));
        fs::create_dir_all(&path).expect("failed to create temp dir");
        path
    }

    fn write_failing_program(path: &Path) {
        fs::write(path, "function main(): Integer { return 7; }\n")
            .expect("failed to write failing source");
    }

    fn temp_binary_prefix(tag: &str, source_path: &Path) -> String {
        let stem = source_path
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("input");
        format!("{tag}-{stem}-{}-", std::process::id())
    }

    fn temp_binary_set_with_prefix(prefix: &str) -> HashSet<PathBuf> {
        let mut paths = HashSet::new();
        let temp_dir = std::env::temp_dir();
        let Ok(entries) = fs::read_dir(&temp_dir) else {
            return paths;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if name.starts_with(prefix) {
                paths.insert(path);
            }
        }
        paths
    }

    #[test]
    fn bench_single_file_cleans_up_temp_binary_on_runtime_failure() {
        let temp_dir = make_temp_dir("bench-cleanup");
        let source_path = temp_dir.join("failing_bench.arden");
        write_failing_program(&source_path);
        let prefix = temp_binary_prefix("arden-bench", &source_path);
        let before = temp_binary_set_with_prefix(&prefix);

        let err = bench_target(Some(&source_path), 1)
            .expect_err("bench should return an error for non-zero exit");
        assert!(err.contains("exited with code 7"), "{err}");
        let after = temp_binary_set_with_prefix(&prefix);
        let leaked: Vec<PathBuf> = after.difference(&before).cloned().collect();
        assert!(
            leaked.is_empty(),
            "bench leaked temporary binaries: {leaked:?}"
        );

        let _ = fs::remove_file(source_path);
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn profile_single_file_cleans_up_temp_binary_on_runtime_failure() {
        let temp_dir = make_temp_dir("profile-cleanup");
        let source_path = temp_dir.join("failing_profile.arden");
        write_failing_program(&source_path);
        let prefix = temp_binary_prefix("arden-profile", &source_path);
        let before = temp_binary_set_with_prefix(&prefix);

        let err = profile_target(Some(&source_path))
            .expect_err("profile should return an error for non-zero exit");
        assert!(err.contains("exited with code 7"), "{err}");
        let after = temp_binary_set_with_prefix(&prefix);
        let leaked: Vec<PathBuf> = after.difference(&before).cloned().collect();
        assert!(
            leaked.is_empty(),
            "profile leaked temporary binaries: {leaked:?}"
        );

        let _ = fs::remove_file(source_path);
        let _ = fs::remove_dir_all(temp_dir);
    }
}
