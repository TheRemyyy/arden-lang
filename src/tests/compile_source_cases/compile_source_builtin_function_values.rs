use super::*;
use std::fs;

#[test]
fn compile_source_runs_builtin_to_int_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-to-int-fn-value-runtime");
    let source_path = temp_root.join("builtin_to_int_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_to_int_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                conv: (Float) -> Integer = to_int;
                return if (conv(3.9) == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("to_int function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled to_int function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_to_string_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-to-string-fn-value-runtime");
    let source_path = temp_root.join("builtin_to_string_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_to_string_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                render: (Boolean) -> String = to_string;
                return if (render(true) == "true") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("to_string function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled to_string function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_mixed_numeric_assert_eq_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-mixed-assert-eq-fn-value-runtime");
    let source_path = temp_root.join("builtin_mixed_assert_eq_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_mixed_assert_eq_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                check: (Integer, Float) -> None = assert_eq;
                check(4, 4.0);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed numeric assert_eq function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mixed numeric assert_eq function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_mixed_numeric_assert_ne_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-mixed-assert-ne-fn-value-runtime");
    let source_path = temp_root.join("builtin_mixed_assert_ne_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_mixed_assert_ne_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                check: (Float, Integer) -> None = assert_ne;
                check(4.5, 4);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed numeric assert_ne function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mixed numeric assert_ne function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_assert_eq_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-assert-eq-fn-value-runtime");
    let source_path = temp_root.join("builtin_assert_eq_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_assert_eq_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                check: (Integer, Integer) -> None = assert_eq;
                check(4, 4);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("assert_eq function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled assert_eq function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_assert_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-assert-fn-value-runtime");
    let source_path = temp_root.join("builtin_assert_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_assert_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                ensure: (Boolean) -> None = assert;
                ensure(true);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("assert function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled assert function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_assert_ne_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-assert-ne-fn-value-runtime");
    let source_path = temp_root.join("builtin_assert_ne_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_assert_ne_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                check: (Integer, Integer) -> None = assert_ne;
                check(4, 5);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("assert_ne function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled assert_ne function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_assert_true_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-assert-true-fn-value-runtime");
    let source_path = temp_root.join("builtin_assert_true_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_assert_true_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                ensure_true: (Boolean) -> None = assert_true;
                ensure_true(true);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("assert_true function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled assert_true function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_assert_false_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-assert-false-fn-value-runtime");
    let source_path = temp_root.join("builtin_assert_false_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_assert_false_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                ensure_false: (Boolean) -> None = assert_false;
                ensure_false(false);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("assert_false function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled assert_false function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_fail_no_arg_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-fail-no-arg-fn-value-runtime");
    let source_path = temp_root.join("builtin_fail_no_arg_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_fail_no_arg_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                stop_now: () -> None = fail;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fail() no-arg function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled fail() no-arg function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_fail_string_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-fail-string-fn-value-runtime");
    let source_path = temp_root.join("builtin_fail_string_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_fail_string_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                stop_with: (String) -> None = fail;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fail(String) function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled fail(String) function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_exit_function_value_check_runtime() {
    let temp_root = make_temp_project_root("builtin-exit-fn-value-runtime");
    let source_path = temp_root.join("builtin_exit_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_exit_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                terminate: (Integer) -> None = exit;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("exit function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled exit function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_assert_function_value_with_string_parameter() {
    let temp_root = make_temp_project_root("reject-builtin-assert-fn-string-param");
    let source_path = temp_root.join("reject_builtin_assert_fn_string_param.arden");
    let output_path = temp_root.join("reject_builtin_assert_fn_string_param");
    let source = r#"
            function main(): Integer {
                ensure: (String) -> None = assert;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("assert(String) function value should fail");
    assert!(
        err.contains("Type mismatch") || err.contains("assert"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_assert_function_value_with_integer_parameter() {
    let temp_root = make_temp_project_root("reject-builtin-assert-fn-integer-param");
    let source_path = temp_root.join("reject_builtin_assert_fn_integer_param.arden");
    let output_path = temp_root.join("reject_builtin_assert_fn_integer_param");
    let source = r#"
            function main(): Integer {
                ensure: (Integer) -> None = assert;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("assert(Integer) function value should fail");
    assert!(
        err.contains("Type mismatch") || err.contains("assert"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_fail_function_value_with_integer_parameter() {
    let temp_root = make_temp_project_root("reject-builtin-fail-fn-integer-param");
    let source_path = temp_root.join("reject_builtin_fail_fn_integer_param.arden");
    let output_path = temp_root.join("reject_builtin_fail_fn_integer_param");
    let source = r#"
            function main(): Integer {
                stop_with: (Integer) -> None = fail;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("fail(Integer) function value should fail");
    assert!(
        err.contains("Type mismatch") || err.contains("fail"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_assert_true_function_value_with_integer_parameter() {
    let temp_root = make_temp_project_root("reject-builtin-assert-true-fn-integer-param");
    let source_path = temp_root.join("reject_builtin_assert_true_fn_integer_param.arden");
    let output_path = temp_root.join("reject_builtin_assert_true_fn_integer_param");
    let source = r#"
            function main(): Integer {
                ensure_true: (Integer) -> None = assert_true;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("assert_true(Integer) function value should fail");
    assert!(
        err.contains("Type mismatch") || err.contains("assert_true"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_assert_false_function_value_with_integer_parameter() {
    let temp_root = make_temp_project_root("reject-builtin-assert-false-fn-integer-param");
    let source_path = temp_root.join("reject_builtin_assert_false_fn_integer_param.arden");
    let output_path = temp_root.join("reject_builtin_assert_false_fn_integer_param");
    let source = r#"
            function main(): Integer {
                ensure_false: (Integer) -> None = assert_false;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("assert_false(Integer) function value should fail");
    assert!(
        err.contains("Type mismatch") || err.contains("assert_false"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_integer_range_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-int-range-fn-value-runtime");
    let source_path = temp_root.join("builtin_int_range_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_int_range_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                build: (Integer, Integer) -> Range<Integer> = range;
                mut values: Range<Integer> = build(1, 4);
                mut total: Integer = 0;
                while (values.has_next()) {
                    total = total + values.next();
                }
                return if (total == 6) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer range function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled integer range function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_float_range_step_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-float-range-step-fn-value-runtime");
    let source_path = temp_root.join("builtin_float_range_step_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_float_range_step_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                build: (Float, Float, Float) -> Range<Float> = range;
                mut values: Range<Float> = build(0.0, 1.0, 0.25);
                mut total: Float = 0.0;
                while (values.has_next()) {
                    total = total + values.next();
                }
                return if (total == 1.5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("float range function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled float range function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_read_line_alias_function_value_check_runtime() {
    let temp_root = make_temp_project_root("read-line-alias-fn-value-runtime");
    let source_path = temp_root.join("read_line_alias_fn_value_runtime.arden");
    let output_path = temp_root.join("read_line_alias_fn_value_runtime");
    let source = r#"
            import std.io.read_line as read_line;

            function main(): Integer {
                reader: () -> String = read_line;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("read_line alias function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled read_line alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_args_get_alias_value_runtime() {
    let temp_root = make_temp_project_root("args-get-alias-value-runtime");
    let source_path = temp_root.join("args_get_alias_value_runtime.arden");
    let output_path = temp_root.join("args_get_alias_value_runtime");
    let source = r#"
            import std.args.get as get;

            function main(): Integer {
                fetch: (Integer) -> String = get;
                value: String = fetch(0);
                return if (value != "") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Args.get alias function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled Args.get alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
