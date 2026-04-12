use super::*;
use std::fs;

#[test]
fn compile_source_runs_match_expr_with_user_enum_some_string_payload_runtime() {
    let temp_root = make_temp_project_root("match-expr-user-enum-some-string-runtime");
    let source_path = temp_root.join("match_expr_user_enum_some_string_runtime.arden");
    let output_path = temp_root.join("match_expr_user_enum_some_string_runtime");
    let source = r#"
            enum E {
                Some(String),
                Missing
            }

            function main(): Integer {
                value: E = E.Some("hello");
                return match (value) {
                    E.Some(v) => v.length(),
                    E.Missing => 0,
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("match expression with user enum Some(String) should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled user enum Some(String) match expression binary");
    assert_eq!(status.code(), Some(5));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_boolean_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-bool-runtime");
    let source_path = temp_root.join("string_interpolation_bool_runtime.arden");
    let output_path = temp_root.join("string_interpolation_bool_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = "{true}";
                return if (Str.compare(value, "true") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string interpolation on Boolean should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string interpolation Boolean binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_char_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-char-runtime");
    let source_path = temp_root.join("string_interpolation_char_runtime.arden");
    let output_path = temp_root.join("string_interpolation_char_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = "{'b'}";
                return if (Str.compare(value, "b") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string interpolation on Char should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string interpolation Char binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_none_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-none-runtime");
    let source_path = temp_root.join("string_interpolation_none_runtime.arden");
    let output_path = temp_root.join("string_interpolation_none_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = "{None}";
                return if (Str.compare(value, "None") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string interpolation on None should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string interpolation None binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_math_abs_min_integer_runtime() {
    let temp_root = make_temp_project_root("math-abs-min-integer-runtime");
    let source_path = temp_root.join("math_abs_min_integer_runtime.arden");
    let output_path = temp_root.join("math_abs_min_integer_runtime");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                value: Integer = 0 - 9223372036854775807 - 1;
                result: Integer = Math.abs(value);
                return if (result < 0) { 1 } else { 0 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Math.abs minimum integer should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled Math.abs minimum integer binary");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("Math.abs() overflow on minimum Integer\n"),
        "{stdout}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_boolean_with_user_facing_representation() {
    let temp_root = make_temp_project_root("print-bool-runtime");
    let source_path = temp_root.join("print_bool_runtime.arden");
    let output_path = temp_root.join("print_bool_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("print(Boolean) should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled print Boolean binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "true");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_unicode_char_with_user_facing_representation() {
    let temp_root = make_temp_project_root("print-char-runtime");
    let source_path = temp_root.join("print_char_runtime.arden");
    let output_path = temp_root.join("print_char_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print('🚀');
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("print(Char) should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled print Char binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "🚀");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_prints_none_with_user_facing_representation() {
    let temp_root = make_temp_project_root("print-none-runtime");
    let source_path = temp_root.join("print_none_runtime.arden");
    let output_path = temp_root.join("print_none_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print(None);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("print(None) should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled print None binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "None");

    let _ = fs::remove_dir_all(temp_root);
}
