use super::*;
use std::fs;

#[test]
fn compile_source_runs_string_interpolation_with_string_literal_index_key_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-map-string-key-runtime");
    let source_path = temp_root.join("string_interpolation_map_string_key_runtime.arden");
    let output_path = temp_root.join("string_interpolation_map_string_key_runtime");
    let source = r#"
            function main(): Integer {
                mut m: Map<String, Integer> = Map<String, Integer>();
                m["x"] = 7;
                s: String = "{m["x"]}";
                return if (s == "7") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string interpolation with string literal key should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string interpolation with string key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_long_string_interpolation_runtime() {
    let temp_root = make_temp_project_root("long-string-interpolation-runtime");
    let source_path = temp_root.join("long_string_interpolation_runtime.arden");
    let output_path = temp_root.join("long_string_interpolation_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                mut s: String = "";
                mut i: Integer = 0;
                while (i < 60000) {
                    s = Str.concat(s, "a");
                    i = i + 1;
                }
                out: String = "x{s}y";
                return if (out.length() == 60002) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("long string interpolation should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled long string interpolation binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_with_nested_braces_string_literal_runtime() {
    let temp_root = make_temp_project_root("string-interp-nested-braces-string-runtime");
    let source_path = temp_root.join("string_interp_nested_braces_string_runtime.arden");
    let output_path = temp_root.join("string_interp_nested_braces_string_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                s: String = "{Str.contains("\{x\}", "{")}";
                return if (s == "true") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string interpolation with nested braces in string literal should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled nested braces string interpolation binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_with_char_brace_literal_runtime() {
    let temp_root = make_temp_project_root("string-interp-char-brace-runtime");
    let source_path = temp_root.join("string_interp_char_brace_runtime.arden");
    let output_path = temp_root.join("string_interp_char_brace_runtime");
    let source = r#"
            function main(): Integer {
                s: String = "{'}'}";
                return if (s == "}") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string interpolation with char brace literal should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled char brace interpolation binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_float_interpolation_from_nested_module_runtime() {
    let temp_root = make_temp_project_root("float-interpolation-nested-module-runtime");
    let source_path = temp_root.join("float_interpolation_nested_module_runtime.arden");
    let output_path = temp_root.join("float_interpolation_nested_module_runtime");
    let source = r#"
            import std.io.*;

            module Metrics {
                module Api {
                    function ratio(value: Integer): Float {
                        return to_float(value) / 2.0;
                    }
                }
            }

            function main(): Integer {
                println("ratio={Metrics.Api.ratio(3)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("nested module float interpolation should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled nested module float interpolation binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("ratio=1.500000"),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_inline_mixed_numeric_if_interpolation_runtime() {
    let temp_root = make_temp_project_root("inline-mixed-if-interpolation-runtime");
    let source_path = temp_root.join("inline_mixed_if_interpolation_runtime.arden");
    let output_path = temp_root.join("inline_mixed_if_interpolation_runtime");
    let source = r#"
            import std.io.*;
            function main(): Integer {
                println("value={if (true) { 1 } else { 2.5 }}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("inline mixed numeric if interpolation should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled inline mixed numeric if interpolation binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=1.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_inline_mixed_numeric_match_interpolation_runtime() {
    let temp_root = make_temp_project_root("inline-mixed-match-interpolation-runtime");
    let source_path = temp_root.join("inline_mixed_match_interpolation_runtime.arden");
    let output_path = temp_root.join("inline_mixed_match_interpolation_runtime");
    let source = r#"
            import std.io.*;
            enum Kind { A, B }
            function main(): Integer {
                println("value={match (Kind.A) { Kind.A => { 1 } Kind.B => { 2.5 } }}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("inline mixed numeric match interpolation should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled inline mixed numeric match interpolation binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=1.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_option_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-option-runtime");
    let source_path = temp_root.join("string_interpolation_option_runtime.arden");
    let output_path = temp_root.join("string_interpolation_option_runtime");
    let source = r#"
            function main(): Integer {
                value: String = "{Option.some(1)}";
                return if (value == "Some(1)") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string interpolation on Option should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string interpolation Option binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_direct_option_none_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-direct-option-none-runtime");
    let source_path = temp_root.join("string_interpolation_direct_option_none_runtime.arden");
    let output_path = temp_root.join("string_interpolation_direct_option_none_runtime");
    let source = r#"
            function main(): Integer {
                value: String = "{Option.none()}";
                return if (value == "None") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string interpolation on direct Option.none should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string interpolation direct Option.none binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_direct_result_error_with_option_none_runtime() {
    let temp_root =
        make_temp_project_root("string-interpolation-direct-result-error-option-none-runtime");
    let source_path =
        temp_root.join("string_interpolation_direct_result_error_option_none_runtime.arden");
    let output_path =
        temp_root.join("string_interpolation_direct_result_error_option_none_runtime");
    let source = r#"
            function main(): Integer {
                value: String = "{Result.error(Option.none())}";
                return if (value == "Error(None)") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string interpolation on direct Result.error(Option.none()) should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string interpolation direct Result.error(Option.none()) binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_result_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-result-runtime");
    let source_path = temp_root.join("string_interpolation_result_runtime.arden");
    let output_path = temp_root.join("string_interpolation_result_runtime");
    let source = r#"
            function main(): Integer {
                result: Result<Integer, String> = Result.error("boom");
                value: String = "{result}";
                return if (value == "Error(boom)") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string interpolation on Result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string interpolation Result binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_interpolation_on_direct_result_runtime() {
    let temp_root = make_temp_project_root("string-interpolation-direct-result-runtime");
    let source_path = temp_root.join("string_interpolation_direct_result_runtime.arden");
    let output_path = temp_root.join("string_interpolation_direct_result_runtime");
    let source = r#"
            function main(): Integer {
                value: String = "{Result.ok(1)}";
                return if (value == "Ok(1)") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string interpolation on direct Result.ok should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string interpolation direct Result binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
