use super::*;
use std::fs;

#[test]
fn compile_source_no_check_rejects_module_local_enum_variant_function_value_type_mismatch_with_user_facing_name(
) {
    let temp_root =
        make_temp_project_root("no-check-module-local-enum-variant-fn-value-type-mismatch");
    let source_path =
        temp_root.join("no_check_module_local_enum_variant_fn_value_type_mismatch.arden");
    let output_path = temp_root.join("no_check_module_local_enum_variant_fn_value_type_mismatch");
    let source = r#"
            module M {
                enum Token { Int(Integer) }
            }

            function main(): None {
                f: () -> M.Token = M.Token.Int;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local enum variant function value mismatch should fail");
    assert!(
        err.contains("Type mismatch: expected () -> M.Token, got (Integer) -> M.Token"),
        "{err}"
    );
    assert!(!err.contains("M__Token"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_specialized_constructor_wrong_arity_with_user_facing_diagnostic()
{
    let temp_root = make_temp_project_root("no-check-specialized-constructor-wrong-arity");
    let source_path = temp_root.join("no_check_specialized_constructor_wrong_arity.arden");
    let output_path = temp_root.join("no_check_specialized_constructor_wrong_arity");
    let source = r#"
            module M {
                class Pair_Box<T, U> {
                    first: T;
                    second: U;
                    constructor(first: T, second: U) {
                        this.first = first;
                        this.second = second;
                    }
                }
            }

            function main(): Integer {
                return M.Pair_Box<Integer, String>(7);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("specialized constructor wrong arity should fail in codegen");
    assert!(
        err.contains("Constructor M.Pair_Box<Integer, String> expects 2 argument(s), got 1"),
        "{err}"
    );
    assert!(!err.contains("Clang failed"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_specialized_method_wrong_arity_with_user_facing_diagnostic() {
    let temp_root = make_temp_project_root("no-check-specialized-method-wrong-arity");
    let source_path = temp_root.join("no_check_specialized_method_wrong_arity.arden");
    let output_path = temp_root.join("no_check_specialized_method_wrong_arity");
    let source = r#"
            module M {
                class Pair_Box<T, U> {
                    first: T;
                    second: U;
                    constructor(first: T, second: U) {
                        this.first = first;
                        this.second = second;
                    }

                    function first_value(): T {
                        return this.first;
                    }
                }
            }

            function main(): Integer {
                value: M.Pair_Box<Integer, String> = M.Pair_Box<Integer, String>(7, "x");
                return value.first_value(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("specialized method wrong arity should fail in codegen");
    assert!(
        err.contains("M.Pair_Box<Integer, String>.first_value() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_unknown_specialized_class_field_access_with_user_facing_class_diagnostic(
) {
    let temp_root = make_temp_project_root("no-check-unknown-specialized-class-field-access");
    let source_path = temp_root.join("no_check_unknown_specialized_class_field_access.arden");
    let output_path = temp_root.join("no_check_unknown_specialized_class_field_access");
    let source = r#"
            module M {
                class Pair_Box<T, U> {
                    first: T;
                    second: U;
                    constructor(first: T, second: U) {
                        this.first = first;
                        this.second = second;
                    }
                }
            }

            function main(): Integer {
                value: M.Pair_Box<Integer, String> = M.Pair_Box<Integer, String>(7, "x");
                return value.missing;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("missing specialized field access should fail in codegen");
    assert!(
        err.contains("Unknown field 'missing' on class 'M.Pair_Box<Integer, String>'"),
        "{err}"
    );
    assert!(!err.contains("M.Pair_Box.spec.I64_Str"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_missing_method_with_user_facing_class_name() {
    let temp_root = make_temp_project_root("no-check-module-local-missing-method-call");
    let source_path = temp_root.join("no_check_module_local_missing_method_call.arden");
    let output_path = temp_root.join("no_check_module_local_missing_method_call");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                return M.Box(1).missing();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local missing method should fail in codegen");
    assert!(
        err.contains("Unknown method 'missing' for class 'M.Box'"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_bound_method_function_value_wrong_arity() {
    let temp_root = make_temp_project_root("no-check-bound-method-function-value-wrong-arity");
    let source_path = temp_root.join("no_check_bound_method_function_value_wrong_arity.arden");
    let output_path = temp_root.join("no_check_bound_method_function_value_wrong_arity");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }

                function get(): Integer {
                    return this.value;
                }
            }

            function main(): Integer {
                b: Box = Box(7);
                f: () -> Integer = b.get;
                return f(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("bound method function value wrong arity should fail in codegen");
    assert!(
        err.contains("Function value () -> Integer expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_generic_bound_method_function_value_signature_mismatch() {
    let temp_root =
        make_temp_project_root("no-check-generic-bound-method-function-signature-mismatch");
    let source_path =
        temp_root.join("no_check_generic_bound_method_function_signature_mismatch.arden");
    let output_path = temp_root.join("no_check_generic_bound_method_function_signature_mismatch");
    let source = r#"
            interface Named {
                function name(): Integer;
            }

            class Person implements Named {
                constructor() {}
                function name(): Integer { return 1; }
            }

            function read_name<T extends Named>(value: T): Integer {
                f: (Integer) -> Integer = value.name;
                return f(1);
            }

            function main(): Integer {
                return read_name(Person());
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("generic bound method signature mismatch should fail in codegen");
    assert!(
        err.contains("Cannot use function value () -> Integer as (Integer) -> Integer"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_enum_missing_bound_method_value_with_user_facing_diagnostic() {
    let temp_root = make_temp_project_root("no-check-enum-missing-bound-method-value");
    let source_path = temp_root.join("no_check_enum_missing_bound_method_value.arden");
    let output_path = temp_root.join("no_check_enum_missing_bound_method_value");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }

                function missing(): Integer {
                    return this.value;
                }
            }

            enum Boxed { Wrap(Integer) }

            function main(): Integer {
                value: Boxed = Boxed.Wrap(1);
                f: () -> Integer = value.missing;
                return f();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("missing enum bound method value should fail in codegen");
    assert!(
        err.contains("Unknown field 'missing' on class 'Boxed'"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_enum_variant_function_value_field_access_without_panicking() {
    let temp_root = make_temp_project_root("no-check-enum-variant-function-value-field-access");
    let source_path = temp_root.join("no_check_enum_variant_function_value_field_access.arden");
    let output_path = temp_root.join("no_check_enum_variant_function_value_field_access");
    let source = r#"
            enum Boxed {
                Wrap(Integer)
            }

            function main(): Integer {
                f: (Integer) -> Boxed = Boxed.Wrap;
                return f(1).Wrap;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("enum variant function value field access should fail in codegen");
    assert!(
        err.contains("Unknown field 'Wrap' on class 'Boxed'"),
        "{err}"
    );
    assert!(!err.contains("panicked at"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_function_wrong_arity_instead_of_ignoring_extra_args() {
    let temp_root = make_temp_project_root("no-check-module-function-wrong-arity");
    let source_path = temp_root.join("no_check_module_function_wrong_arity.arden");
    let output_path = temp_root.join("no_check_module_function_wrong_arity");
    let source = r#"
            module M {
                function f(x: Integer): Integer {
                    return x;
                }
            }

            function main(): Integer {
                return M.f(7, 8);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module function wrong arity should fail in codegen");
    assert!(
        err.contains("Function value (Integer) -> Integer expects 1 argument(s), got 2"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_nested_integer_return_for_float_task_runtime() {
    let temp_root = make_temp_project_root("async-nested-int-return-float-task-runtime");
    let source_path = temp_root.join("async_nested_int_return_float_task_runtime.arden");
    let output_path = temp_root.join("async_nested_int_return_float_task_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Float> = async {
                    if (true) {
                        return 1;
                    }
                    return 2.5;
                };
                value: Float = await(task);
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("nested async Integer return for Task<Float> should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled nested async Integer return Float task binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_async_block_nested_mixed_numeric_returns_runtime() {
    let temp_root = make_temp_project_root("async-nested-mixed-numeric-returns-runtime");
    let source_path = temp_root.join("async_nested_mixed_numeric_returns_runtime.arden");
    let output_path = temp_root.join("async_nested_mixed_numeric_returns_runtime");
    let source = r#"
            function main(): Integer {
                task: Task<Float> = async {
                    if (false) {
                        return 1;
                    } else {
                        return 2.5;
                    }
                };
                value: Float = await(task);
                return if (value == 2.5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("nested async mixed numeric returns should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled nested async mixed numeric returns binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_float_loop_variable_over_integer_range_runtime() {
    let temp_root = make_temp_project_root("float-loop-var-integer-range-runtime");
    let source_path = temp_root.join("float_loop_var_integer_range_runtime.arden");
    let output_path = temp_root.join("float_loop_var_integer_range_runtime");
    let source = r#"
            function main(): Integer {
                mut total: Float = 0.0;
                for (x: Float in range(1, 4)) {
                    total = total + x;
                }
                return if (total == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Float loop variable over Integer range should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled Float loop variable Integer range binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_float_loop_variable_over_integer_list_runtime() {
    let temp_root = make_temp_project_root("float-loop-var-integer-list-runtime");
    let source_path = temp_root.join("float_loop_var_integer_list_runtime.arden");
    let output_path = temp_root.join("float_loop_var_integer_list_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                xs.push(2);
                xs.push(3);

                mut total: Float = 0.0;
                for (x: Float in xs) {
                    total = total + x;
                }

                return if (total == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("Float loop variable over Integer list should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled Float loop variable Integer list binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
