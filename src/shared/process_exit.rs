use std::process::ExitStatus;

const PROCESS_ERROR_OUTPUT_LIMIT: usize = 8192;

#[cfg(unix)]
fn unix_signal_name(signal: i32) -> &'static str {
    match signal {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        4 => "SIGILL (illegal instruction)",
        5 => "SIGTRAP",
        6 => "SIGABRT (abort)",
        7 => "SIGBUS (bus error)",
        8 => "SIGFPE (floating-point exception)",
        9 => "SIGKILL",
        11 => "SIGSEGV (segmentation fault)",
        13 => "SIGPIPE",
        14 => "SIGALRM",
        15 => "SIGTERM",
        24 => "SIGXCPU (cpu limit exceeded)",
        25 => "SIGXFSZ (file size limit exceeded)",
        31 => "SIGSYS (bad system call)",
        _ => "signal",
    }
}

#[cfg(unix)]
fn is_unix_crash_signal(signal: i32) -> bool {
    matches!(signal, 4 | 5 | 6 | 7 | 8 | 11 | 24 | 25 | 31)
}

#[cfg(windows)]
fn windows_status_name(status: u32) -> Option<&'static str> {
    match status {
        0xC0000005 => Some("STATUS_ACCESS_VIOLATION"),
        0xC000008C => Some("STATUS_ARRAY_BOUNDS_EXCEEDED"),
        0xC0000096 => Some("STATUS_PRIVILEGED_INSTRUCTION"),
        0xC000001D => Some("STATUS_ILLEGAL_INSTRUCTION"),
        0xC0000094 => Some("STATUS_INTEGER_DIVIDE_BY_ZERO"),
        0xC00000FD => Some("STATUS_STACK_OVERFLOW"),
        0xC000013A => Some("STATUS_CONTROL_C_EXIT"),
        0xC0000409 => Some("STATUS_STACK_BUFFER_OVERRUN"),
        0xC0000374 => Some("STATUS_HEAP_CORRUPTION"),
        _ => None,
    }
}

#[cfg(windows)]
fn is_windows_crash_status(status: u32) -> bool {
    matches!(
        status,
        0xC0000005
            | 0xC000008C
            | 0xC0000096
            | 0xC000001D
            | 0xC0000094
            | 0xC00000FD
            | 0xC0000409
            | 0xC0000374
    )
}

fn truncate_process_output(output: &str) -> String {
    let trimmed = output.trim();
    if trimmed.chars().count() <= PROCESS_ERROR_OUTPUT_LIMIT {
        return trimmed.to_string();
    }
    let head: String = trimmed.chars().take(PROCESS_ERROR_OUTPUT_LIMIT).collect();
    format!("{head}\n... [truncated, original output exceeded {PROCESS_ERROR_OUTPUT_LIMIT} chars]")
}

pub(crate) fn format_exit_failure(status: ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            let name = unix_signal_name(signal);
            if is_unix_crash_signal(signal) {
                return format!(
                    "terminated by signal {signal} ({name}). this indicates a runtime crash; rerun with `arden compile --emit-llvm ...` and report it."
                );
            }
            return format!("terminated by signal {signal} ({name})");
        }
    }

    if let Some(code) = status.code() {
        #[cfg(windows)]
        {
            let status_u32 = code as u32;
            if let Some(name) = windows_status_name(status_u32) {
                if is_windows_crash_status(status_u32) {
                    return format!(
                        "terminated by Windows exception 0x{status_u32:08X} ({name}). this indicates a runtime crash; rerun with `arden compile --emit-llvm ...` and report it."
                    );
                }
                return format!("terminated by Windows exception 0x{status_u32:08X} ({name})");
            }
            if status_u32 & 0x8000_0000 != 0 {
                return format!("terminated by Windows exception 0x{status_u32:08X}");
            }
        }
        return format!("exited with code {code}");
    }

    "terminated without an exit code".to_string()
}

pub(crate) fn command_failure_details(status: ExitStatus, stderr: &str, stdout: &str) -> String {
    if !stderr.trim().is_empty() {
        return truncate_process_output(stderr);
    }
    if !stdout.trim().is_empty() {
        return truncate_process_output(stdout);
    }
    format_exit_failure(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn unix_sigsegv_is_reported_as_runtime_crash() {
        use std::os::unix::process::ExitStatusExt;
        let status = ExitStatus::from_raw(11);
        let rendered = format_exit_failure(status);
        assert!(
            rendered.contains("SIGSEGV") && rendered.contains("runtime crash"),
            "{rendered}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_sigterm_is_reported_without_crash_hint() {
        use std::os::unix::process::ExitStatusExt;
        let status = ExitStatus::from_raw(15);
        let rendered = format_exit_failure(status);
        assert!(rendered.contains("SIGTERM"), "{rendered}");
        assert!(!rendered.contains("runtime crash"), "{rendered}");
    }

    #[cfg(unix)]
    #[test]
    fn command_failure_details_truncates_huge_output() {
        use std::os::unix::process::ExitStatusExt;
        let status = ExitStatus::from_raw(1 << 8);
        let long = "x".repeat(PROCESS_ERROR_OUTPUT_LIMIT + 200);
        let rendered = command_failure_details(status, &long, "");
        assert!(
            rendered.contains("[truncated, original output exceeded"),
            "{rendered}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_stack_buffer_overrun_reports_exception_name() {
        use std::os::windows::process::ExitStatusExt;
        let status = ExitStatus::from_raw(0xC0000409);
        let rendered = format_exit_failure(status);
        assert!(
            rendered.contains("STATUS_STACK_BUFFER_OVERRUN") && rendered.contains("runtime crash"),
            "{rendered}"
        );
    }
}
