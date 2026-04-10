use colored::control;
#[cfg(windows)]
use colored::*;
use std::path::Path;
use std::time::Duration;

const CLI_WHITE_RGB: (u8, u8, u8) = (255, 255, 255);
const CLI_SOFT_RGB: (u8, u8, u8) = (239, 232, 220);
const CLI_TERTIARY_RGB: (u8, u8, u8) = (217, 178, 158);

pub(crate) fn configure_cli_colors() {
    #[cfg(windows)]
    let _ = enable_ansi_support::enable_ansi_support();

    control::set_override(true);
}

pub(crate) fn cli_accent(text: impl AsRef<str>) -> String {
    #[cfg(windows)]
    {
        return text
            .as_ref()
            .truecolor(CLI_WHITE_RGB.0, CLI_WHITE_RGB.1, CLI_WHITE_RGB.2)
            .bold()
            .to_string();
    }

    #[cfg(not(windows))]
    {
        ansi_truecolor(text.as_ref(), CLI_WHITE_RGB, true)
    }
}

pub(crate) fn cli_soft(text: impl AsRef<str>) -> String {
    #[cfg(windows)]
    {
        return text
            .as_ref()
            .truecolor(CLI_SOFT_RGB.0, CLI_SOFT_RGB.1, CLI_SOFT_RGB.2)
            .to_string();
    }

    #[cfg(not(windows))]
    {
        ansi_truecolor(text.as_ref(), CLI_SOFT_RGB, false)
    }
}

pub(crate) fn cli_tertiary(text: impl AsRef<str>) -> String {
    #[cfg(windows)]
    {
        return text
            .as_ref()
            .truecolor(CLI_TERTIARY_RGB.0, CLI_TERTIARY_RGB.1, CLI_TERTIARY_RGB.2)
            .to_string();
    }

    #[cfg(not(windows))]
    {
        ansi_truecolor(text.as_ref(), CLI_TERTIARY_RGB, false)
    }
}

pub(crate) fn cli_success(text: impl AsRef<str>) -> String {
    cli_accent(text)
}

pub(crate) fn cli_warning(text: impl AsRef<str>) -> String {
    cli_soft(text)
}

pub(crate) fn cli_error(text: impl AsRef<str>) -> String {
    cli_accent(text)
}

pub(crate) fn cli_path(path: &Path) -> String {
    cli_soft(format_cli_path(path))
}

fn ansi_truecolor(text: &str, rgb: (u8, u8, u8), bold: bool) -> String {
    let bold_prefix = if bold { "\x1b[1m" } else { "" };
    format!(
        "{bold_prefix}\x1b[38;2;{};{};{}m{text}\x1b[0m",
        rgb.0, rgb.1, rgb.2
    )
}

pub(crate) fn cli_new_run_hint() -> &'static str {
    "arden run"
}

pub(crate) fn format_cli_path(path: &Path) -> String {
    #[cfg(windows)]
    {
        let raw = path.to_string_lossy().replace('/', "\\");
        if let Some(stripped) = raw.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{}", stripped);
        }
        if let Some(stripped) = raw.strip_prefix(r"\\?\") {
            return stripped.to_string();
        }
        raw
    }

    #[cfg(not(windows))]
    {
        path.to_string_lossy().into_owned()
    }
}

pub(crate) fn cli_elapsed(duration: Duration) -> String {
    format!("{:.6} s", duration.as_secs_f64())
}

pub(crate) struct TestRunReport {
    pub(crate) passed: usize,
    pub(crate) failed: usize,
    pub(crate) ignored: usize,
}

pub(crate) fn print_test_runner_output(stdout: &str, success: bool) -> TestRunReport {
    let mut report = TestRunReport {
        passed: 0,
        failed: 0,
        ignored: 0,
    };
    let mut active_test: Option<String> = None;

    for line in stdout.lines() {
        if let Some(name) = line.strip_prefix("__ARDEN_TEST_START__ ") {
            active_test = Some(name.to_string());
        } else if let Some(name) = line.strip_prefix("__ARDEN_TEST_PASS__ ") {
            report.passed += 1;
            active_test = None;
            println!(
                "{} {} {}",
                cli_accent("PASS"),
                cli_tertiary(">"),
                cli_soft(name)
            );
        } else if let Some(name) = line.strip_prefix("__ARDEN_TEST_SKIP__ ") {
            report.ignored += 1;
            println!(
                "{} {} {}",
                cli_tertiary("SKIP"),
                cli_tertiary(">"),
                cli_soft(name)
            );
        } else if let Some(reason) = line.strip_prefix("__ARDEN_TEST_SKIP_REASON__ ") {
            println!(" {}", cli_tertiary(reason));
        } else {
            println!("{line}");
        }
    }

    if !success {
        if let Some(name) = active_test {
            report.failed = 1;
            println!(
                "{} {} {}",
                cli_accent("FAILED"),
                cli_tertiary(">"),
                cli_soft(name)
            );
        } else {
            report.failed = 1;
        }
    }

    report
}

pub(crate) fn print_cli_step(message: impl AsRef<str>) {
    println!("{} {}", cli_accent("›"), cli_accent(message.as_ref()));
}

pub(crate) fn print_cli_cache(message: impl AsRef<str>) {
    println!("{} {}", cli_success("↺"), cli_success(message.as_ref()));
}

pub(crate) fn print_cli_artifact_result(
    action: &str,
    subject: &str,
    path: &Path,
    elapsed: Duration,
) {
    println!(
        "{} {} {} {} {}",
        cli_success(action),
        cli_accent(subject),
        cli_tertiary("->"),
        cli_path(path),
        cli_tertiary(format!("({})", cli_elapsed(elapsed)))
    );
}
