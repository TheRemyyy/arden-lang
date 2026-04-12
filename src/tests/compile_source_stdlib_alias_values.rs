use super::*;
use std::fs;

#[test]
fn compile_source_runs_stdlib_function_alias_value_runtime() {
    let temp_root = make_temp_project_root("stdlib-fn-alias-value-runtime");
    let source_path = temp_root.join("stdlib_fn_alias_value_runtime.arden");
    let output_path = temp_root.join("stdlib_fn_alias_value_runtime");
    let source = r#"
            import std.math.abs as abs;

            function main(): Integer {
                f: (Integer) -> Integer = abs;
                return if (f(-5) == 5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("stdlib alias function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled stdlib alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_stdlib_namespace_alias_function_value_runtime() {
    let temp_root = make_temp_project_root("stdlib-namespace-alias-value-runtime");
    let source_path = temp_root.join("stdlib_namespace_alias_value_runtime.arden");
    let output_path = temp_root.join("stdlib_namespace_alias_value_runtime");
    let source = r#"
            import std.math as math;

            function main(): Integer {
                f: (Integer) -> Integer = math.abs;
                return if (f(-9) == 9) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("stdlib namespace alias function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled stdlib namespace alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_stdlib_function_alias_callback_runtime() {
    let temp_root = make_temp_project_root("stdlib-fn-alias-callback-runtime");
    let source_path = temp_root.join("stdlib_fn_alias_callback_runtime.arden");
    let output_path = temp_root.join("stdlib_fn_alias_callback_runtime");
    let source = r#"
            import std.math.abs as abs;

            function apply_twice(f: (Integer) -> Integer, x: Integer): Integer {
                return f(f(x));
            }

            function main(): Integer {
                return if (apply_twice(abs, -2) == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("stdlib alias callback should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled stdlib alias callback binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_stdlib_math_min_alias_value_runtime() {
    let temp_root = make_temp_project_root("stdlib-math-min-alias-value-runtime");
    let source_path = temp_root.join("stdlib_math_min_alias_value_runtime.arden");
    let output_path = temp_root.join("stdlib_math_min_alias_value_runtime");
    let source = r#"
            import std.math.min as min;

            function main(): Integer {
                pick: (Integer, Integer) -> Integer = min;
                return if (pick(3, 1) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Math.min alias value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled Math.min alias value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
