use super::*;
use std::fs;
#[cfg(not(windows))]
use std::os::unix::ffi::OsStringExt;

#[test]
fn compile_source_runs_unicode_string_literal_index_operator_with_dynamic_index() {
    let temp_root = make_temp_project_root("unicode-string-dynamic-index-runtime");
    let source_path = temp_root.join("unicode_string_dynamic_index_runtime.arden");
    let output_path = temp_root.join("unicode_string_dynamic_index_runtime");
    let source = r#"
            function main(): Integer {
                idx: Integer = 0;
                c: Char = "🚀"[idx];
                return if (c == '🚀') { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("unicode string literal dynamic index should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unicode dynamic string index binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_unicode_string_literal_index_operator_past_char_len() {
    let temp_root = make_temp_project_root("unicode-string-index-oob-runtime");
    let source_path = temp_root.join("unicode_string_index_oob_runtime.arden");
    let output_path = temp_root.join("unicode_string_index_oob_runtime");
    let source = r#"
            function main(): Integer {
                idx: Integer = 1;
                c: Char = "🚀"[idx];
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("unicode string literal oob dynamic index should still codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled unicode dynamic string index oob binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(stdout.contains("String index out of bounds\n"), "{stdout}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unicode_string_variable_index_operator() {
    let temp_root = make_temp_project_root("unicode-string-variable-index-runtime");
    let source_path = temp_root.join("unicode_string_variable_index_runtime.arden");
    let output_path = temp_root.join("unicode_string_variable_index_runtime");
    let source = r#"
            function main(): Integer {
                s: String = "🚀";
                idx: Integer = 0;
                c: Char = s[idx];
                return if (c == '🚀') { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("unicode string variable index should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unicode string variable index binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unicode_string_literal_length() {
    let temp_root = make_temp_project_root("unicode-string-literal-length-runtime");
    let source_path = temp_root.join("unicode_string_literal_length_runtime.arden");
    let output_path = temp_root.join("unicode_string_literal_length_runtime");
    let source = r#"
            function main(): Integer {
                return if ("🚀".length() == 1) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("unicode string literal length should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unicode string literal length binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unicode_string_variable_length() {
    let temp_root = make_temp_project_root("unicode-string-variable-length-runtime");
    let source_path = temp_root.join("unicode_string_variable_length_runtime.arden");
    let output_path = temp_root.join("unicode_string_variable_length_runtime");
    let source = r#"
            function main(): Integer {
                s: String = "🚀";
                return if (s.length() == 1) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("unicode string variable length should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unicode string variable length binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_system_exec_large_output() {
    let temp_root = make_temp_project_root("system-exec-large-output-runtime");
    let source_path = temp_root.join("system_exec_large_output_runtime.arden");
    let output_path = temp_root.join("system_exec_large_output_runtime");
    let command = if cfg!(windows) {
        r#"powershell -NoProfile -Command "$s='x'*5000; Write-Output $s""#
    } else {
        r#"python3 -c "print('x' * 5000)""#
    };
    let escaped_command = command.replace('\\', "\\\\").replace('"', "\\\"");
    let source = format!(
        r#"
                import std.system.*;

                function main(): Integer {{
                    out: String = System.exec("{escaped_command}");
                    return if (out.length() > 4500) {{ 0; }} else {{ 1; }};
                }}
            "#
    );

    fs::write(&source_path, source).must("write source");
    compile_source(
        &fs::read_to_string(&source_path).must("read source"),
        &source_path,
        &output_path,
        false,
        true,
        None,
        None,
    )
    .must("large System.exec output should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled large System.exec binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(not(windows))]
#[test]
fn compile_source_fails_fast_on_system_exec_nul_bytes() {
    let temp_root = make_temp_project_root("system-exec-nul-bytes-runtime");
    let source_path = temp_root.join("system_exec_nul_bytes_runtime.arden");
    let output_path = temp_root.join("system_exec_nul_bytes_runtime");
    let source = r#"
            import std.system.*;

            function main(): Integer {
                out: String = System.exec("python3 -c \"import sys; sys.stdout.buffer.write(b'A\\x00B')\"");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("System.exec NUL-byte failure path should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled System.exec NUL-byte binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("System.exec() cannot load NUL bytes\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(not(windows))]
#[test]
fn compile_source_fails_fast_on_system_exec_invalid_utf8() {
    let temp_root = make_temp_project_root("system-exec-invalid-utf8-runtime");
    let source_path = temp_root.join("system_exec_invalid_utf8_runtime.arden");
    let output_path = temp_root.join("system_exec_invalid_utf8_runtime");
    let source = r#"
            import std.system.*;

            function main(): Integer {
                out: String = System.exec("python3 -c \"import sys; sys.stdout.buffer.write(bytes([0xff]))\"");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("System.exec invalid UTF-8 failure path should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled System.exec invalid UTF-8 binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("Invalid UTF-8 sequence in String\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(not(windows))]
#[test]
fn compile_source_fails_fast_on_system_getenv_invalid_utf8() {
    let temp_root = make_temp_project_root("system-getenv-invalid-utf8-runtime");
    let source_path = temp_root.join("system_getenv_invalid_utf8_runtime.arden");
    let output_path = temp_root.join("system_getenv_invalid_utf8_runtime");
    let source = r#"
            import std.system.*;

            function main(): Integer {
                value: String = System.getenv("ARDEN_BAD_UTF8_ENV");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("System.getenv invalid UTF-8 failure path should codegen");

    let output = std::process::Command::new(&output_path)
        .env(
            "ARDEN_BAD_UTF8_ENV",
            std::ffi::OsString::from_vec(vec![0xff]),
        )
        .output()
        .must("run compiled System.getenv invalid UTF-8 binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("Invalid UTF-8 sequence in String\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(not(windows))]
#[test]
fn compile_source_runs_system_shell_with_decoded_exit_code() {
    let temp_root = make_temp_project_root("system-shell-decoded-exit-code-runtime");
    let source_path = temp_root.join("system_shell_decoded_exit_code_runtime.arden");
    let output_path = temp_root.join("system_shell_decoded_exit_code_runtime");
    let source = r#"
            import std.system.*;

            function main(): Integer {
                code: Integer = System.shell("sh -c 'exit 7'");
                return if (code == 7) { 0; } else { code; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("System.shell decoded exit code should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled System.shell decoded exit code binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_file_read_missing_path() {
    let temp_root = make_temp_project_root("file-read-missing-path-runtime");
    let source_path = temp_root.join("file_read_missing_path_runtime.arden");
    let output_path = temp_root.join("file_read_missing_path_runtime");
    let source = r#"
            import std.fs.*;

            function main(): Integer {
                data: String = File.read("missing.txt");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("File.read missing-path failure path should codegen");

    let output = std::process::Command::new(&output_path)
        .current_dir(&temp_root)
        .output()
        .must("run compiled File.read missing-path binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("File.read() failed to open file\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_time_now_with_long_format() {
    let temp_root = make_temp_project_root("time-now-long-format-runtime");
    let source_path = temp_root.join("time_now_long_format_runtime.arden");
    let output_path = temp_root.join("time_now_long_format_runtime");
    let source = r#"
            import std.time.*;

            function main(): Integer {
                out: String = Time.now("%Y-%m-%d %H:%M:%S %A %B %Y-%m-%d %H:%M:%S %A %B %Y-%m-%d %H:%M:%S %A %B");
                return if (out.length() > 40) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Time.now long format should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled Time.now long format binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(target_os = "linux")]
#[test]
fn compile_source_runs_system_cwd_with_long_working_directory() {
    let temp_root = make_temp_project_root("system-cwd-long-working-directory-runtime");
    let source_path = temp_root.join("system_cwd_long_working_directory_runtime.arden");
    let output_path = temp_root.join("system_cwd_long_working_directory_runtime");
    let source = r#"
            import std.string.*;
            import std.system.*;

            function main(): Integer {
                cwd: String = System.cwd();
                return if (Str.len(cwd) > 1100) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("System.cwd deep path runtime should codegen");

    let mut deep_dir = temp_root.join("cwd-depth-root");
    fs::create_dir_all(&deep_dir).must("create deep root");
    let segment = "a".repeat(60);
    for index in 0..18 {
        deep_dir = deep_dir.join(format!("{segment}{index}"));
        fs::create_dir_all(&deep_dir).must("create deep segment");
    }

    let status = std::process::Command::new(&output_path)
        .current_dir(&deep_dir)
        .status()
        .must("run compiled System.cwd binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_read_line_with_long_input() {
    let temp_root = make_temp_project_root("read-line-long-input-runtime");
    let source_path = temp_root.join("read_line_long_input_runtime.arden");
    let output_path = temp_root.join("read_line_long_input_runtime");
    let source = r#"
            import std.io.*;
            import std.string.*;

            function main(): Integer {
                line: String = read_line();
                return if (Str.len(line) > 1500) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("read_line long input should codegen");

    let mut child = std::process::Command::new(&output_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .must("spawn read_line binary");
    {
        use std::io::Write as _;
        let stdin = child.stdin.as_mut().must("child stdin");
        writeln!(stdin, "{}", "x".repeat(2000)).must("write long stdin");
    }
    let status = child.wait().must("wait for read_line binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unicode_str_len() {
    let temp_root = make_temp_project_root("unicode-str-len-runtime");
    let source_path = temp_root.join("unicode_str_len_runtime.arden");
    let output_path = temp_root.join("unicode_str_len_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                s: String = "🚀";
                return if (Str.len(s) == 1) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("unicode Str.len should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unicode Str.len binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_unicode_string_variable_index_operator_past_char_len() {
    let temp_root = make_temp_project_root("unicode-string-variable-index-oob-runtime");
    let source_path = temp_root.join("unicode_string_variable_index_oob_runtime.arden");
    let output_path = temp_root.join("unicode_string_variable_index_oob_runtime");
    let source = r#"
            function main(): Integer {
                s: String = "🚀";
                idx: Integer = 1;
                c: Char = s[idx];
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("unicode string variable oob index should still codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled unicode string variable index oob binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(stdout.contains("String index out of bounds\n"), "{stdout}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_equality_on_literals() {
    let temp_root = make_temp_project_root("string-eq-literal-runtime");
    let source_path = temp_root.join("string_eq_literal_runtime.arden");
    let output_path = temp_root.join("string_eq_literal_runtime");
    let source = r#"
            function main(): Integer {
                if ("b" == "b") { return 32; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string literal equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string equality literal binary");
    assert_eq!(status.code(), Some(32));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_equality_on_expression_results() {
    let temp_root = make_temp_project_root("string-eq-expr-runtime");
    let source_path = temp_root.join("string_eq_expr_runtime.arden");
    let output_path = temp_root.join("string_eq_expr_runtime");
    let source = r#"
            import std.string.*;
            function main(): Integer {
                if (Str.concat("a", "b") == "ab") { return 33; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string expression equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string equality expression binary");
    assert_eq!(status.code(), Some(33));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_list_identity_equality() {
    let temp_root = make_temp_project_root("list-eq-runtime");
    let source_path = temp_root.join("list_eq_runtime.arden");
    let output_path = temp_root.join("list_eq_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                if (xs == xs) { return 34; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("list identity equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled list equality binary");
    assert_eq!(status.code(), Some(34));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_list_constructor_with_preallocated_integer_capacity() {
    let temp_root = make_temp_project_root("list-capacity-runtime");
    let source_path = temp_root.join("list_capacity_runtime.arden");
    let output_path = temp_root.join("list_capacity_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>(3);
                xs.push(10);
                xs.push(20);
                xs.push(30);
                if (xs.length() == 3 && xs.get(0) == 10 && xs.get(2) == 30) {
                    return 80;
                }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("list constructor with integer capacity should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled list capacity binary");
    assert_eq!(status.code(), Some(80));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_list_constructor_with_preallocated_option_capacity() {
    let temp_root = make_temp_project_root("option-list-capacity-runtime");
    let source_path = temp_root.join("option_list_capacity_runtime.arden");
    let output_path = temp_root.join("option_list_capacity_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Option<Integer>> = List<Option<Integer>>(2);
                xs.push(Option<Integer>());
                xs.push(Option.some(9));
                if (xs.length() == 2 && xs.get(0).is_none() && xs.get(1).unwrap() == 9) {
                    return 81;
                }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("list constructor with option capacity should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled option list capacity binary");
    assert_eq!(status.code(), Some(81));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_runtime_on_negative_list_constructor_capacity_expression() {
    let temp_root = make_temp_project_root("negative-list-capacity-runtime");
    let source_path = temp_root.join("negative_list_capacity_runtime.arden");
    let output_path = temp_root.join("negative_list_capacity_runtime");
    let source = r#"
            function main(): Integer {
                cap: Integer = 1 - 2;
                xs: List<Integer> = List<Integer>(cap);
                return xs.length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("negative runtime list capacity source should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled negative list capacity binary");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("List constructor capacity cannot be negative"),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}
