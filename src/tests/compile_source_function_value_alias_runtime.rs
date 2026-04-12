use super::*;
use std::fs;

#[test]
fn compile_source_runs_imported_alias_explicit_generic_function_value_runtime() {
    let temp_root = make_temp_project_root("imported-alias-explicit-generic-fn-value-runtime");
    let source_path = temp_root.join("imported_alias_explicit_generic_fn_value_runtime.arden");
    let output_path = temp_root.join("imported_alias_explicit_generic_fn_value_runtime");
    let source = r#"
            function id<T>(x: T): T {
                return x;
            }

            import id as ident;

            function main(): Integer {
                f: (Integer) -> Integer = ident<Integer>;
                return if (f(7) == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported alias explicit generic function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported alias explicit generic function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_option_some_alias_runtime() {
    let temp_root = make_temp_project_root("imported-option-some-alias-runtime");
    let source_path = temp_root.join("imported_option_some_alias_runtime.arden");
    let output_path = temp_root.join("imported_option_some_alias_runtime");
    let source = r#"
            import Option.Some as Present;

            function main(): Integer {
                value: Option<Integer> = Present(7);
                return if (value.unwrap() == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported Option.Some alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported Option.Some alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_option_some_alias_function_value_runtime() {
    let temp_root = make_temp_project_root("imported-option-some-alias-fn-value-runtime");
    let source_path = temp_root.join("imported_option_some_alias_fn_value_runtime.arden");
    let output_path = temp_root.join("imported_option_some_alias_fn_value_runtime");
    let source = r#"
            import Option.Some as Present;

            function main(): Integer {
                wrap: (Integer) -> Option<Integer> = Present;
                value: Option<Integer> = wrap(9);
                return if (value.unwrap() == 9) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported Option.Some alias function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported Option.Some alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_option_alias_match_runtime() {
    let temp_root = make_temp_project_root("imported-option-alias-match-runtime");
    let source_path = temp_root.join("imported_option_alias_match_runtime.arden");
    let output_path = temp_root.join("imported_option_alias_match_runtime");
    let source = r#"
            import Option.Some as Present;
            import Option.None as Empty;

            function main(): Integer {
                value: Option<Integer> = Present(7);
                return match (value) {
                    Present(inner) => if (inner == 7) { 0 } else { 1 },
                    Empty => 2,
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported Option alias match should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported Option alias match binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_result_ok_alias_runtime() {
    let temp_root = make_temp_project_root("imported-result-ok-alias-runtime");
    let source_path = temp_root.join("imported_result_ok_alias_runtime.arden");
    let output_path = temp_root.join("imported_result_ok_alias_runtime");
    let source = r#"
            import Result.Ok as Success;

            function main(): Integer {
                value: Result<Integer, String> = Success(5);
                return if (value.unwrap() == 5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported Result.Ok alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported Result.Ok alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_exact_imported_nested_enum_variant_aliases_runtime() {
    let temp_root = make_temp_project_root("exact-imported-nested-enum-variant-aliases-runtime");
    let source_path = temp_root.join("exact_imported_nested_enum_variant_aliases_runtime.arden");
    let output_path = temp_root.join("exact_imported_nested_enum_variant_aliases_runtime");
    let source = r#"
            module util { enum Result { Ok(Integer), Error(String) } }
            import util.Result.Ok as Success;
            import util.Result.Error as Failure;

            function main(): Integer {
                value: util.Result = Success(2);
                return match (value) {
                    Success(v) => if (v == 2) { 0 } else { 1 },
                    Failure(err) => 2,
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("exact imported nested enum variant aliases should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled exact imported nested enum variant alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_function_variable_retyped_to_float_return_runtime() {
    let temp_root = make_temp_project_root("fn-var-retype-float-runtime");
    let source_path = temp_root.join("fn_var_retype_float_runtime.arden");
    let output_path = temp_root.join("fn_var_retype_float_runtime");
    let source = r#"
            function one(): Integer {
                return 1;
            }

            function main(): Integer {
                g: () -> Integer = one;
                f: () -> Float = g;
                value: Float = f();
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("function variable retyped to Float return should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled function variable retyped Float return binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_named_function_value_retyped_to_integer_parameter_runtime() {
    let temp_root = make_temp_project_root("named-fn-retype-int-param-runtime");
    let source_path = temp_root.join("named_fn_retype_int_param_runtime.arden");
    let output_path = temp_root.join("named_fn_retype_int_param_runtime");
    let source = r#"
            function scale(value: Float): Float {
                return value * 2.0;
            }

            function main(): Integer {
                f: (Integer) -> Float = scale;
                result: Float = f(3);
                return if (result == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("named function value retyped Integer parameter should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled named function value retyped Integer parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_named_function_value_with_interface_return_runtime() {
    let temp_root = make_temp_project_root("named-fn-interface-return-runtime");
    let source_path = temp_root.join("named_fn_interface_return_runtime.arden");
    let output_path = temp_root.join("named_fn_interface_return_runtime");
    let source = r#"
            interface Named {
                function name(): Integer;
            }

            class Book implements Named {
                constructor() {}
                function name(): Integer { return 7; }
            }

            function build_book(): Book {
                return Book();
            }

            function main(): Integer {
                f: () -> Named = build_book;
                return if (f().name() == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("named function value with interface return should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled named function value interface return binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_named_function_value_with_interface_parameter_runtime() {
    let temp_root = make_temp_project_root("named-fn-interface-param-runtime");
    let source_path = temp_root.join("named_fn_interface_param_runtime.arden");
    let output_path = temp_root.join("named_fn_interface_param_runtime");
    let source = r#"
            interface Named {
                function name(): Integer;
            }

            class Book implements Named {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function name(): Integer { return this.value; }
            }

            function read_name(value: Named): Integer {
                return value.name();
            }

            function main(): Integer {
                f: (Book) -> Integer = read_name;
                return if (f(Book(9)) == 9) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("named function value with interface parameter should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled named function value interface parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_function_variable_retyped_to_integer_parameter_runtime() {
    let temp_root = make_temp_project_root("fn-var-retype-int-param-runtime");
    let source_path = temp_root.join("fn_var_retype_int_param_runtime.arden");
    let output_path = temp_root.join("fn_var_retype_int_param_runtime");
    let source = r#"
            function scale(value: Float): Float {
                return value * 2.0;
            }

            function main(): Integer {
                g: (Float) -> Float = scale;
                f: (Integer) -> Float = g;
                result: Float = f(3);
                return if (result == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("function variable retyped Integer parameter should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled function variable retyped Integer parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_function_value_retyped_to_narrower_integer_parameter() {
    let temp_root = make_temp_project_root("fn-retype-narrower-int-param");
    let source_path = temp_root.join("fn_retype_narrower_int_param.arden");
    let output_path = temp_root.join("fn_retype_narrower_int_param");
    let source = r#"
            function truncate(value: Integer): Integer {
                return value;
            }

            function main(): Integer {
                f: (Float) -> Integer = truncate;
                return f(1.5);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("retyping function value to narrower Integer parameter should fail");
    assert!(
        err.contains("Type mismatch") || err.contains("cannot assign"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}
