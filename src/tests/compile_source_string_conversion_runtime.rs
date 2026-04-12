use super::*;
use std::fs;

#[test]
fn compile_source_runs_string_to_int_conversion_runtime() {
    let temp_root = make_temp_project_root("string-to-int-runtime");
    let source_path = temp_root.join("string_to_int_runtime.arden");
    let output_path = temp_root.join("string_to_int_runtime");
    let source = r#"
            function main(): Integer {
                input: String = "100";
                value: Integer = to_int(input);
                return if (value == 100) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string to int conversion should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string to int binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_option_runtime() {
    let temp_root = make_temp_project_root("to-string-option-runtime");
    let source_path = temp_root.join("to_string_option_runtime.arden");
    let output_path = temp_root.join("to_string_option_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = to_string(Option.some(1));
                return if (Str.compare(value, "Some(1)") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("to_string on Option should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled to_string Option binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_direct_option_none_runtime() {
    let temp_root = make_temp_project_root("to-string-direct-option-none-runtime");
    let source_path = temp_root.join("to_string_direct_option_none_runtime.arden");
    let output_path = temp_root.join("to_string_direct_option_none_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = to_string(Option.none());
                return if (Str.compare(value, "None") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("to_string on direct Option.none should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled to_string direct Option.none binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_nested_direct_option_runtime() {
    let temp_root = make_temp_project_root("to-string-nested-direct-option-runtime");
    let source_path = temp_root.join("to_string_nested_direct_option_runtime.arden");
    let output_path = temp_root.join("to_string_nested_direct_option_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = to_string(Option.some(Option.none()));
                return if (Str.compare(value, "Some(None)") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("to_string on nested direct Option should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled to_string nested direct Option binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_result_runtime() {
    let temp_root = make_temp_project_root("to-string-result-runtime");
    let source_path = temp_root.join("to_string_result_runtime.arden");
    let output_path = temp_root.join("to_string_result_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                result: Result<Integer, String> = Result.ok(1);
                value: String = to_string(result);
                return if (Str.compare(value, "Ok(1)") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("to_string on Result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled to_string Result binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_direct_result_ok_runtime() {
    let temp_root = make_temp_project_root("to-string-direct-result-ok-runtime");
    let source_path = temp_root.join("to_string_direct_result_ok_runtime.arden");
    let output_path = temp_root.join("to_string_direct_result_ok_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = to_string(Result.ok(1));
                return if (Str.compare(value, "Ok(1)") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("to_string on direct Result.ok should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled to_string direct Result.ok binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_direct_result_error_with_option_none_runtime() {
    let temp_root = make_temp_project_root("to-string-direct-result-error-option-none-runtime");
    let source_path = temp_root.join("to_string_direct_result_error_option_none_runtime.arden");
    let output_path = temp_root.join("to_string_direct_result_error_option_none_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                value: String = to_string(Result.error(Option.none()));
                return if (Str.compare(value, "Error(None)") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("to_string on direct Result.error(Option.none()) should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled to_string direct Result.error(Option.none()) binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_char_runtime() {
    let temp_root = make_temp_project_root("to-string-char-runtime");
    let source_path = temp_root.join("to_string_char_runtime.arden");
    let output_path = temp_root.join("to_string_char_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                c: Char = 'b';
                value: String = to_string(c);
                return if (Str.compare(value, "b") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("to_string on Char should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled to_string Char binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_to_string_on_unicode_char_runtime() {
    let temp_root = make_temp_project_root("to-string-unicode-char-runtime");
    let source_path = temp_root.join("to_string_unicode_char_runtime.arden");
    let output_path = temp_root.join("to_string_unicode_char_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                c: Char = '🚀';
                value: String = to_string(c);
                return if (Str.compare(value, "🚀") == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("to_string on Unicode Char should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled to_string Unicode Char binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_str_ends_with_false_for_longer_suffix_runtime() {
    let temp_root = make_temp_project_root("str-ends-with-longer-suffix-runtime");
    let source_path = temp_root.join("str_ends_with_longer_suffix_runtime.arden");
    let output_path = temp_root.join("str_ends_with_longer_suffix_runtime");
    let source = r#"
            import std.string.*;

            function main(): Integer {
                if (Str.endsWith("a", "abc")) { return 1; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Str.endsWith longer suffix should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled Str.endsWith longer suffix binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
