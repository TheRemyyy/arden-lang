#[cfg(unix)]
use crate::linker::find_tool_in_path_for_test;
#[cfg(target_os = "linux")]
use crate::linker::gcc_version_sort_keys_for_test;
#[cfg(target_os = "linux")]
use crate::linker::linux_target_descriptor_for_test;
use crate::linker::{escape_response_file_arg, windows_machine_flag};

#[test]
fn windows_machine_flag_prefers_x64_over_x86_substring() {
    assert_eq!(windows_machine_flag(Some("x86_64-pc-windows-msvc")), "x64");
    assert_eq!(windows_machine_flag(Some("amd64-pc-windows-msvc")), "x64");
}

#[test]
fn windows_machine_flag_keeps_other_windows_arches() {
    assert_eq!(windows_machine_flag(Some("i686-pc-windows-msvc")), "x86");
    assert_eq!(
        windows_machine_flag(Some("aarch64-pc-windows-msvc")),
        "arm64"
    );
}

#[test]
fn escape_response_file_arg_escapes_line_breaks() {
    assert_eq!(
        escape_response_file_arg("path\nwith\rcuts"),
        "\"path\\nwith\\rcuts\""
    );
}

#[test]
fn escape_response_file_arg_preserves_windows_backslashes() {
    assert_eq!(
        escape_response_file_arg(r#"\\?\C:\tmp\tagged "set".obj"#),
        r#""\\?\C:\tmp\tagged \"set\".obj""#
    );
}

#[test]
fn escape_response_file_arg_doubles_trailing_windows_backslashes() {
    assert_eq!(
        escape_response_file_arg(r#"C:\tmp\linker-dir\"#),
        r#""C:\tmp\linker-dir\\""#
    );
}

#[cfg(unix)]
#[test]
fn find_tool_in_path_ignores_non_executable_files() {
    use std::os::unix::fs::PermissionsExt;

    let root = std::env::temp_dir().join(format!(
        "arden-linker-path-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("create temp dir");
    let fake_tool = root.join("fake-linker");
    std::fs::write(&fake_tool, "#!/bin/sh\necho nope\n").expect("write fake tool");
    std::fs::set_permissions(&fake_tool, std::fs::Permissions::from_mode(0o644))
        .expect("set fake tool non-executable permissions");

    let detected = find_tool_in_path_for_test("fake-linker", root.as_os_str());

    let _ = std::fs::remove_file(&fake_tool);
    let _ = std::fs::remove_dir_all(&root);

    assert!(
        detected.is_none(),
        "non-executable file should not be treated as an available linker binary"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn linux_target_descriptor_rejects_musl_targets() {
    let error = linux_target_descriptor_for_test(Some("x86_64-unknown-linux-musl"))
        .expect_err("musl target should be rejected in direct linux linker mode");
    assert!(error.contains("GNU libc targets only"), "{error}");
}

#[cfg(target_os = "linux")]
#[test]
fn gcc_version_sort_prefers_numeric_newest_version() {
    let keys = gcc_version_sort_keys_for_test(&["9", "10", "11.2.0", "11.10.0", "trunk"]);
    let ordered = keys.into_iter().map(|entry| entry.2).collect::<Vec<_>>();
    assert_eq!(ordered, vec!["11.10.0", "11.2.0", "10", "9", "trunk"]);
}
