use crate::linker::windows_machine_flag;

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
