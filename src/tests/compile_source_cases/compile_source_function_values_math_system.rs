use super::*;
use std::fs;

#[test]
fn compile_source_runs_stdlib_math_min_mixed_numeric_function_value_runtime() {
    let temp_root = make_temp_project_root("stdlib-math-min-mixed-fn-value-runtime");
    let source_path = temp_root.join("stdlib_math_min_mixed_fn_value_runtime.arden");
    let output_path = temp_root.join("stdlib_math_min_mixed_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                pick: (Integer, Float) -> Float = math.min;
                return if (pick(3, 1.5) == 1.5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed numeric Math.min function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mixed numeric Math.min function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_stdlib_math_max_mixed_numeric_function_value_runtime() {
    let temp_root = make_temp_project_root("stdlib-math-max-mixed-fn-value-runtime");
    let source_path = temp_root.join("stdlib_math_max_mixed_fn_value_runtime.arden");
    let output_path = temp_root.join("stdlib_math_max_mixed_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                pick: (Float, Integer) -> Float = math.max;
                return if (pick(1.5, 3) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed numeric Math.max function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mixed numeric Math.max function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_math_abs_widened_return_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-math-abs-widened-return-fn-value-runtime");
    let source_path = temp_root.join("direct_math_abs_widened_return_fn_value_runtime.arden");
    let output_path = temp_root.join("direct_math_abs_widened_return_fn_value_runtime");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                f: (Integer) -> Float = Math.abs;
                return if (f(-2) == 2.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Math.abs widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled Math.abs widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_wildcard_imported_math_abs_call_runtime() {
    let temp_root = make_temp_project_root("wildcard-imported-math-abs-call-runtime");
    let source_path = temp_root.join("wildcard_imported_math_abs_call_runtime.arden");
    let output_path = temp_root.join("wildcard_imported_math_abs_call_runtime");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                return if (abs(-2) == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("wildcard imported Math.abs call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled wildcard imported Math.abs call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_wildcard_imported_math_abs_widened_return_function_value_runtime() {
    let temp_root = make_temp_project_root("wildcard-imported-math-abs-widened-return-fn-value");
    let source_path = temp_root.join("wildcard_imported_math_abs_widened_return_fn_value.arden");
    let output_path = temp_root.join("wildcard_imported_math_abs_widened_return_fn_value");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                f: (Integer) -> Float = abs;
                return if (f(-2) == 2.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("wildcard imported Math.abs widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled wildcard imported Math.abs widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_alias_math_abs_widened_return_function_value_runtime() {
    let temp_root = make_temp_project_root("alias-math-abs-widened-return-fn-value-runtime");
    let source_path = temp_root.join("alias_math_abs_widened_return_fn_value_runtime.arden");
    let output_path = temp_root.join("alias_math_abs_widened_return_fn_value_runtime");
    let source = r#"
            import std.math.abs as abs;

            function main(): Integer {
                f: (Integer) -> Float = abs;
                return if (f(-2) == 2.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("alias Math.abs widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled alias Math.abs widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_namespace_alias_math_abs_widened_return_function_value_runtime() {
    let temp_root =
        make_temp_project_root("namespace-alias-math-abs-widened-return-fn-value-runtime");
    let source_path =
        temp_root.join("namespace_alias_math_abs_widened_return_fn_value_runtime.arden");
    let output_path = temp_root.join("namespace_alias_math_abs_widened_return_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                f: (Integer) -> Float = math.abs;
                return if (f(-2) == 2.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("namespace alias Math.abs widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled namespace alias Math.abs widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_math_min_widened_return_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-math-min-widened-return-fn-value-runtime");
    let source_path = temp_root.join("direct_math_min_widened_return_fn_value_runtime.arden");
    let output_path = temp_root.join("direct_math_min_widened_return_fn_value_runtime");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                f: (Integer, Integer) -> Float = Math.min;
                return if (f(3, 1) == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Math.min widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled Math.min widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_alias_math_min_widened_return_function_value_runtime() {
    let temp_root = make_temp_project_root("alias-math-min-widened-return-fn-value-runtime");
    let source_path = temp_root.join("alias_math_min_widened_return_fn_value_runtime.arden");
    let output_path = temp_root.join("alias_math_min_widened_return_fn_value_runtime");
    let source = r#"
            import std.math.min as min;

            function main(): Integer {
                f: (Integer, Integer) -> Float = min;
                return if (f(3, 1) == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("alias Math.min widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled alias Math.min widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_namespace_alias_math_min_widened_return_function_value_runtime() {
    let temp_root =
        make_temp_project_root("namespace-alias-math-min-widened-return-fn-value-runtime");
    let source_path =
        temp_root.join("namespace_alias_math_min_widened_return_fn_value_runtime.arden");
    let output_path = temp_root.join("namespace_alias_math_min_widened_return_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                f: (Integer, Integer) -> Float = math.min;
                return if (f(3, 1) == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("namespace alias Math.min widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled namespace alias Math.min widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_math_max_widened_return_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-math-max-widened-return-fn-value-runtime");
    let source_path = temp_root.join("direct_math_max_widened_return_fn_value_runtime.arden");
    let output_path = temp_root.join("direct_math_max_widened_return_fn_value_runtime");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                f: (Integer, Integer) -> Float = Math.max;
                return if (f(3, 1) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Math.max widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled Math.max widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_alias_math_max_widened_return_function_value_runtime() {
    let temp_root = make_temp_project_root("alias-math-max-widened-return-fn-value-runtime");
    let source_path = temp_root.join("alias_math_max_widened_return_fn_value_runtime.arden");
    let output_path = temp_root.join("alias_math_max_widened_return_fn_value_runtime");
    let source = r#"
            import std.math.max as max;

            function main(): Integer {
                f: (Integer, Integer) -> Float = max;
                return if (f(3, 1) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("alias Math.max widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled alias Math.max widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_namespace_alias_math_max_widened_return_function_value_runtime() {
    let temp_root =
        make_temp_project_root("namespace-alias-math-max-widened-return-fn-value-runtime");
    let source_path =
        temp_root.join("namespace_alias_math_max_widened_return_fn_value_runtime.arden");
    let output_path = temp_root.join("namespace_alias_math_max_widened_return_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                f: (Integer, Integer) -> Float = math.max;
                return if (f(3, 1) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("namespace alias Math.max widened return function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled namespace alias Math.max widened return function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_math_pow_integer_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-math-pow-int-fn-value-runtime");
    let source_path = temp_root.join("builtin_math_pow_int_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_math_pow_int_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                pow_ints: (Integer, Integer) -> Float = math.pow;
                return if (pow_ints(2, 3) == 8.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer Math.pow function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled integer Math.pow function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_math_pow_mixed_numeric_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-math-pow-mixed-fn-value-runtime");
    let source_path = temp_root.join("builtin_math_pow_mixed_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_math_pow_mixed_fn_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                pow_mixed: (Integer, Float) -> Float = math.pow;
                return if (pow_mixed(9, 0.5) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed numeric Math.pow function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mixed numeric Math.pow function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_math_random_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-math-random-fn-value-runtime");
    let source_path = temp_root.join("direct_math_random_fn_value_runtime.arden");
    let output_path = temp_root.join("direct_math_random_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> Float = Math.random;
                return if (f() >= 0.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Math.random function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Math.random function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_math_pi_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-math-pi-fn-value-runtime");
    let source_path = temp_root.join("direct_math_pi_fn_value_runtime.arden");
    let output_path = temp_root.join("direct_math_pi_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> Float = Math.pi;
                return if (f() > 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Math.pi function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Math.pi function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_math_sqrt_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-math-sqrt-fn-value-runtime");
    let source_path = temp_root.join("direct_math_sqrt_fn_value_runtime.arden");
    let output_path = temp_root.join("direct_math_sqrt_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: (Integer) -> Float = Math.sqrt;
                return if (f(9) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Math.sqrt function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Math.sqrt function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_system_cwd_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-system-cwd-fn-value-runtime");
    let source_path = temp_root.join("direct_system_cwd_fn_value_runtime.arden");
    let output_path = temp_root.join("direct_system_cwd_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> String = System.cwd;
                return if (f() != "") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct System.cwd function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct System.cwd function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_system_os_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-system-os-fn-value-runtime");
    let source_path = temp_root.join("direct_system_os_fn_value_runtime.arden");
    let output_path = temp_root.join("direct_system_os_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> String = System.os;
                return if (f() != "") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct System.os function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct System.os function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_time_unix_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-time-unix-fn-value-runtime");
    let source_path = temp_root.join("direct_time_unix_fn_value_runtime.arden");
    let output_path = temp_root.join("direct_time_unix_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> Integer = Time.unix;
                return if (f() >= 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Time.unix function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Time.unix function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_time_sleep_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-time-sleep-fn-value-runtime");
    let source_path = temp_root.join("direct_time_sleep_fn_value_runtime.arden");
    let output_path = temp_root.join("direct_time_sleep_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: (Integer) -> None = Time.sleep;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Time.sleep function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Time.sleep function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_args_count_function_value_runtime() {
    let temp_root = make_temp_project_root("direct-args-count-fn-value-runtime");
    let source_path = temp_root.join("direct_args_count_fn_value_runtime.arden");
    let output_path = temp_root.join("direct_args_count_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                f: () -> Integer = Args.count;
                return if (f() >= 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Args.count function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Args.count function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
