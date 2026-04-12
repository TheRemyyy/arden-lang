use super::*;
use std::fs;

#[test]
fn compile_source_runs_if_expression_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("if-expression-builtin-function-value-runtime");
    let source_path = temp_root.join("if_expression_builtin_function_value_runtime.arden");
    let output_path = temp_root.join("if_expression_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            function choose(flag: Boolean): (Integer) -> Float {
                return if (flag) { to_float } else { to_float };
            }
            function main(): Integer {
                println("value={choose(true)(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("if-expression builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled if-expression builtin function value binary");
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
fn compile_source_runs_match_expression_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("match-expression-builtin-function-value-runtime");
    let source_path = temp_root.join("match_expression_builtin_function_value_runtime.arden");
    let output_path = temp_root.join("match_expression_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            enum Mode { A, B }
            function choose(mode: Mode): (Integer) -> Float {
                return match (mode) { Mode.A => { to_float } Mode.B => { to_float } };
            }
            function main(): Integer {
                println("value={choose(Mode.A)(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("match-expression builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled match-expression builtin function value binary");
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
fn compile_source_runs_if_expression_float_function_value_with_integer_argument_runtime() {
    let temp_root =
        make_temp_project_root("if-expression-float-function-value-integer-argument-runtime");
    let source_path =
        temp_root.join("if_expression_float_function_value_integer_argument_runtime.arden");
    let output_path = temp_root.join("if_expression_float_function_value_integer_argument_runtime");
    let source = r#"
            function scale(value: Float): Float {
                return value * 2.0;
            }

            function main(): Integer {
                result: Float = (if (true) { scale } else { scale })(3);
                return if (result == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None).must(
        "if-expression Float function value should lower Integer arguments through expected types",
    );

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled if-expression Float function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_option_some_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("option-some-builtin-function-value-runtime");
    let source_path = temp_root.join("option_some_builtin_function_value_runtime.arden");
    let output_path = temp_root.join("option_some_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            function choose(): Option<(Integer) -> Float> {
                return Option.some(to_float);
            }
            function main(): Integer {
                println("value={choose().unwrap()(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Option.some builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled Option.some builtin function value binary");
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
fn compile_source_runs_if_expression_option_some_builtin_function_value_runtime() {
    let temp_root =
        make_temp_project_root("if-expression-option-some-builtin-function-value-runtime");
    let source_path =
        temp_root.join("if_expression_option_some_builtin_function_value_runtime.arden");
    let output_path = temp_root.join("if_expression_option_some_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            function choose(flag: Boolean): Option<(Integer) -> Float> {
                return if (flag) { Option.some(to_float) } else { Option.some(to_float) };
            }
            function main(): Integer {
                println("value={choose(true).unwrap()(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("if-expression Option.some builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled if-expression Option.some builtin function value binary");
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
fn compile_source_runs_match_expression_option_some_builtin_function_value_runtime() {
    let temp_root =
        make_temp_project_root("match-expression-option-some-builtin-function-value-runtime");
    let source_path =
        temp_root.join("match_expression_option_some_builtin_function_value_runtime.arden");
    let output_path = temp_root.join("match_expression_option_some_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            enum Mode { A, B }
            function choose(mode: Mode): Option<(Integer) -> Float> {
                return match (mode) {
                    Mode.A => { Option.some(to_float) }
                    Mode.B => { Option.some(to_float) }
                };
            }
            function main(): Integer {
                println("value={choose(Mode.A).unwrap()(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("match-expression Option.some builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled match-expression Option.some builtin function value binary");
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
fn compile_source_runs_result_ok_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("result-ok-builtin-function-value-runtime");
    let source_path = temp_root.join("result_ok_builtin_function_value_runtime.arden");
    let output_path = temp_root.join("result_ok_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            function choose(): Result<(Integer) -> Float, String> {
                return Result.ok(to_float);
            }
            function main(): Integer {
                println("value={choose().unwrap()(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Result.ok builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled Result.ok builtin function value binary");
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
fn compile_source_runs_result_error_builtin_function_value_runtime() {
    let temp_root = make_temp_project_root("result-error-builtin-function-value-runtime");
    let source_path = temp_root.join("result_error_builtin_function_value_runtime.arden");
    let output_path = temp_root.join("result_error_builtin_function_value_runtime");
    let source = r#"
            import std.io.*;
            function choose(): Result<String, (Integer) -> Float> {
                return Result.error(to_float);
            }
            function main(): Integer {
                errf: (Integer) -> Float = match (choose()) {
                    Result.Error(f) => f,
                    _ => to_float,
                };
                println("value={errf(1)}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Result.error builtin function value should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled Result.error builtin function value binary");
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
fn compile_source_runs_direct_option_some_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-option-some-function-value-runtime");
    let source_path = temp_root.join("direct_option_some_function_value_runtime.arden");
    let output_path = temp_root.join("direct_option_some_function_value_runtime");
    let source = r#"
            function main(): Integer {
                wrap: (Integer) -> Option<Integer> = Option.some;
                value: Option<Integer> = wrap(7);
                return if (value == Option.some(7)) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Option.some function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Option.some function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_option_none_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-option-none-function-value-runtime");
    let source_path = temp_root.join("direct_option_none_function_value_runtime.arden");
    let output_path = temp_root.join("direct_option_none_function_value_runtime");
    let source = r#"
            function main(): Integer {
                empty: () -> Option<Integer> = Option.none;
                value: Option<Integer> = empty();
                return if (value == Option.none()) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Option.none function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Option.none function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_result_ok_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-result-ok-function-value-runtime");
    let source_path = temp_root.join("direct_result_ok_function_value_runtime.arden");
    let output_path = temp_root.join("direct_result_ok_function_value_runtime");
    let source = r#"
            function main(): Integer {
                wrap: (Integer) -> Result<Integer, String> = Result.ok;
                value: Result<Integer, String> = wrap(7);
                return if (value == Result.ok(7)) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Result.ok function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Result.ok function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_result_error_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-result-error-function-value-runtime");
    let source_path = temp_root.join("direct_result_error_function_value_runtime.arden");
    let output_path = temp_root.join("direct_result_error_function_value_runtime");
    let source = r#"
            function main(): Integer {
                wrap: (String) -> Result<Integer, String> = Result.error;
                value: Result<Integer, String> = wrap("boom");
                return if (value == Result.error("boom")) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Result.error function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Result.error function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_direct_option_some_function_value_type_mismatch() {
    let temp_root = make_temp_project_root("direct-option-some-function-value-type-mismatch");
    let source_path = temp_root.join("direct_option_some_function_value_type_mismatch.arden");
    let output_path = temp_root.join("direct_option_some_function_value_type_mismatch");
    let source = r#"
            function main(): Integer {
                wrap: (String) -> Option<Integer> = Option.some;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("direct Option.some mismatch should fail");
    assert!(
        err.contains(
            "Type mismatch: expected (String) -> Option<Integer>, got (unknown) -> Option<unknown>"
        ),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_enum_payload_variant_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-enum-payload-variant-function-value");
    let source_path = temp_root.join("direct_enum_payload_variant_function_value.arden");
    let output_path = temp_root.join("direct_enum_payload_variant_function_value");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            function main(): Integer {
                wrap: (Integer) -> Boxed = Boxed.Wrap;
                value: Boxed = wrap(7);
                return match (value) {
                    Boxed.Wrap(v) => { if (v == 7) { 0 } else { 1 } }
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct enum payload variant function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct enum payload variant function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_enum_unit_variant_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-enum-unit-variant-function-value");
    let source_path = temp_root.join("direct_enum_unit_variant_function_value.arden");
    let output_path = temp_root.join("direct_enum_unit_variant_function_value");
    let source = r#"
            enum Mode { A, B }
            function main(): Integer {
                pick: () -> Mode = Mode.A;
                return if (pick() == Mode.A) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct enum unit variant function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct enum unit variant function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_direct_enum_variant_function_value_type_mismatch() {
    let temp_root = make_temp_project_root("direct-enum-variant-function-value-type-mismatch");
    let source_path = temp_root.join("direct_enum_variant_function_value_type_mismatch.arden");
    let output_path = temp_root.join("direct_enum_variant_function_value_type_mismatch");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            function main(): Integer {
                wrap: (String) -> Boxed = Boxed.Wrap;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("direct enum variant mismatch should fail");
    assert!(
        err.contains("Type mismatch: expected (String) -> Boxed, got (Integer) -> Boxed"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}
