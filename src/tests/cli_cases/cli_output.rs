use std::path::Path;

#[test]
fn cli_new_hint_uses_direct_run_command() {
    assert_eq!(crate::cli::output::cli_new_run_hint(), "arden run");
}

#[cfg(not(windows))]
#[test]
fn cli_path_preserves_regular_unix_paths() {
    let path = Path::new("/tmp/arden-demo");
    assert_eq!(crate::cli::output::format_cli_path(path), "/tmp/arden-demo");
}

#[cfg(windows)]
#[test]
fn cli_path_strips_verbatim_windows_prefix() {
    let path = Path::new(r"\\?\C:\Users\demo\project");
    assert_eq!(
        crate::cli::output::format_cli_path(path),
        r"C:\Users\demo\project"
    );
}
