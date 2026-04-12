use super::*;
use std::fs;

#[test]
fn compile_source_runs_stdlib_function_alias_call_runtime() {
    let temp_root = make_temp_project_root("stdlib-fn-alias-call-runtime");
    let source_path = temp_root.join("stdlib_fn_alias_call_runtime.arden");
    let output_path = temp_root.join("stdlib_fn_alias_call_runtime");
    let source = r#"
            import std.math.abs as abs;

            function main(): Integer {
                value: Integer = abs(-5);
                return if (value == 5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("stdlib function alias call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled stdlib function alias call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_args_count_alias_call_runtime() {
    let temp_root = make_temp_project_root("args-count-alias-call-runtime");
    let source_path = temp_root.join("args_count_alias_call_runtime.arden");
    let output_path = temp_root.join("args_count_alias_call_runtime");
    let source = r#"
            import std.args.count as count;

            function main(): Integer {
                value: Integer = count();
                return if (value >= 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Args.count alias call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled Args.count alias call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_capitalized_stdlib_function_alias_call_runtime() {
    let temp_root = make_temp_project_root("capitalized-stdlib-fn-alias-call-runtime");
    let source_path = temp_root.join("capitalized_stdlib_fn_alias_call_runtime.arden");
    let output_path = temp_root.join("capitalized_stdlib_fn_alias_call_runtime");
    let source = r#"
            import std.args.get as ArgGet;

            function main(): Integer {
                value: String = ArgGet(1);
                return if (value == "ok") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("capitalized stdlib function alias call should codegen");

    let status = std::process::Command::new(&output_path)
        .arg("ok")
        .status()
        .must("run compiled capitalized stdlib alias call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_capitalized_stdlib_numeric_alias_call_runtime() {
    let temp_root = make_temp_project_root("capitalized-stdlib-numeric-alias-call-runtime");
    let source_path = temp_root.join("capitalized_stdlib_numeric_alias_call_runtime.arden");
    let output_path = temp_root.join("capitalized_stdlib_numeric_alias_call_runtime");
    let source = r#"
            import std.math.abs as Abs;

            function main(): Integer {
                return if (Abs(-7) == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("capitalized stdlib numeric alias call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled capitalized stdlib numeric alias call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_capitalized_stdlib_function_alias_call_runtime() {
    let temp_root = make_temp_project_root("no-check-capitalized-stdlib-fn-alias-call-runtime");
    let source_path = temp_root.join("no_check_capitalized_stdlib_fn_alias_call_runtime.arden");
    let output_path = temp_root.join("no_check_capitalized_stdlib_fn_alias_call_runtime");
    let source = r#"
            import std.args.get as ArgGet;

            function main(): Integer {
                value: String = ArgGet(1);
                return if (value == "ok") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("capitalized stdlib function alias call should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .arg("ok")
        .status()
        .must("run compiled no-check capitalized stdlib alias call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_stdlib_math_min_alias_call_runtime() {
    let temp_root = make_temp_project_root("stdlib-math-min-alias-call-runtime");
    let source_path = temp_root.join("stdlib_math_min_alias_call_runtime.arden");
    let output_path = temp_root.join("stdlib_math_min_alias_call_runtime");
    let source = r#"
            import std.math.min as min;

            function main(): Integer {
                return if (min(3, 1) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Math.min alias call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled Math.min alias call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
