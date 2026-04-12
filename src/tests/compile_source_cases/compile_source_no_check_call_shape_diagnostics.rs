use super::*;
use std::fs;

#[test]
fn compile_source_no_check_rejects_option_static_call_type_args_cleanly() {
    let temp_root = make_temp_project_root("no-check-option-static-call-type-args");
    let source_path = temp_root.join("no_check_option_static_call_type_args.arden");
    let output_path = temp_root.join("no_check_option_static_call_type_args");
    let source = r#"
            function main(): Option<Integer> {
                return Option.some<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Option.some explicit type args should fail in codegen");
    assert!(
        err.contains("Option static methods do not accept explicit type arguments"),
        "{err}"
    );
    assert!(
        !err.contains("Explicit generic call code generation is not supported yet"),
        "{err}"
    );
    assert!(!err.contains("Clang failed"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_imported_option_some_alias_type_args_cleanly() {
    let temp_root = make_temp_project_root("no-check-imported-option-some-alias-type-args");
    let source_path = temp_root.join("no_check_imported_option_some_alias_type_args.arden");
    let output_path = temp_root.join("no_check_imported_option_some_alias_type_args");
    let source = r#"
            import Option.Some as Present;

            function main(): Option<Integer> {
                return Present<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("imported Option.Some alias type args should fail in codegen");
    assert!(
        err.contains("Built-in function 'Option.some' does not accept type arguments"),
        "{err}"
    );
    assert!(!err.contains("Unknown variant 'Some'"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_result_static_call_type_args_cleanly() {
    let temp_root = make_temp_project_root("no-check-result-static-call-type-args");
    let source_path = temp_root.join("no_check_result_static_call_type_args.arden");
    let output_path = temp_root.join("no_check_result_static_call_type_args");
    let source = r#"
            function main(): Result<Integer, String> {
                return Result.ok<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Result.ok explicit type args should fail in codegen");
    assert!(
        err.contains("Result static methods do not accept explicit type arguments"),
        "{err}"
    );
    assert!(
        !err.contains("Explicit generic call code generation is not supported yet"),
        "{err}"
    );
    assert!(!err.contains("Clang failed"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_explicit_type_args_on_non_function_field_call_cleanly() {
    let temp_root = make_temp_project_root("no-check-non-function-field-generic-call");
    let source_path = temp_root.join("no_check_non_function_field_generic_call.arden");
    let output_path = temp_root.join("no_check_non_function_field_generic_call");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                return Box(1).value<Integer>();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("generic call on non-function field should fail in codegen");
    assert!(
        err.contains("Unknown method 'value' for class 'Box'"),
        "{err}"
    );
    assert!(
        !err.contains("Cannot call non-function type Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_explicit_type_args_on_non_function_field_value_cleanly() {
    let temp_root = make_temp_project_root("no-check-non-function-field-generic-value");
    let source_path = temp_root.join("no_check_non_function_field_generic_value.arden");
    let output_path = temp_root.join("no_check_non_function_field_generic_value");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                f: Integer = Box(1).value<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("generic function value on non-function field should fail in codegen");
    assert!(
        err.contains("Unknown field 'value' on class 'Box'"),
        "{err}"
    );
    assert!(
        !err.contains(
            "Explicit generic function value should be specialized before code generation"
        ),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_namespaced_non_function_field_call_with_demangled_class_name() {
    let temp_root = make_temp_project_root("no-check-namespaced-non-function-field-call");
    let source_path = temp_root.join("no_check_namespaced_non_function_field_call.arden");
    let output_path = temp_root.join("no_check_namespaced_non_function_field_call");
    let source = r#"
            module U {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                return U.Box(1).value<Integer>();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("generic call on namespaced non-function field should fail in codegen");
    assert!(
        err.contains("Unknown method 'value' for class 'U.Box'"),
        "{err}"
    );
    assert!(!err.contains("U__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_namespaced_non_function_field_value_with_demangled_class_name() {
    let temp_root = make_temp_project_root("no-check-namespaced-non-function-field-value");
    let source_path = temp_root.join("no_check_namespaced_non_function_field_value.arden");
    let output_path = temp_root.join("no_check_namespaced_non_function_field_value");
    let source = r#"
            module U {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                f: Integer = U.Box(1).value<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("generic value on namespaced non-function field should fail in codegen");
    assert!(
        err.contains("Unknown field 'value' on class 'U.Box'"),
        "{err}"
    );
    assert!(!err.contains("U__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_unknown_string_method_with_string_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-string-method-name");
    let source_path = temp_root.join("no_check_invalid_string_method_name.arden");
    let output_path = temp_root.join("no_check_invalid_string_method_name");
    let source = r#"
            function main(): Integer {
                s: String = "abc";
                return s.missing();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unknown String method should fail in codegen");
    assert!(err.contains("Unknown String method: missing"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_list_method_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-method-arity");
    let source_path = temp_root.join("no_check_invalid_list_method_arity.arden");
    let output_path = temp_root.join("no_check_invalid_list_method_arity");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                return xs.length(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("list method arity mismatch should fail in codegen");
    assert!(
        err.contains("List.length() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_map_method_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-map-method-arity");
    let source_path = temp_root.join("no_check_invalid_map_method_arity.arden");
    let output_path = temp_root.join("no_check_invalid_map_method_arity");
    let source = r#"
            function main(): Integer {
                values: Map<Integer, Integer> = Map<Integer, Integer>();
                return values.get(1, 2);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("map method arity mismatch should fail in codegen");
    assert!(
        err.contains("Map.get() expects 1 argument(s), got 2"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_set_method_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-set-method-arity");
    let source_path = temp_root.join("no_check_invalid_set_method_arity.arden");
    let output_path = temp_root.join("no_check_invalid_set_method_arity");
    let source = r#"
            function main(): Integer {
                values: Set<Integer> = Set<Integer>();
                return if (values.contains(1, 2)) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("set method arity mismatch should fail in codegen");
    assert!(
        err.contains("Set.contains() expects 1 argument(s), got 2"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_option_method_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-option-method-arity");
    let source_path = temp_root.join("no_check_invalid_option_method_arity.arden");
    let output_path = temp_root.join("no_check_invalid_option_method_arity");
    let source = r#"
            function main(): Integer {
                value: Option<Integer> = Option.some(1);
                return value.unwrap(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("option method arity mismatch should fail in codegen");
    assert!(
        err.contains("Option.unwrap() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_result_method_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-result-method-arity");
    let source_path = temp_root.join("no_check_invalid_result_method_arity.arden");
    let output_path = temp_root.join("no_check_invalid_result_method_arity");
    let source = r#"
            function main(): Integer {
                value: Result<Integer, String> = Result.ok(1);
                return value.unwrap(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("result method arity mismatch should fail in codegen");
    assert!(
        err.contains("Result.unwrap() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_range_method_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-range-method-arity");
    let source_path = temp_root.join("no_check_invalid_range_method_arity.arden");
    let output_path = temp_root.join("no_check_invalid_range_method_arity");
    let source = r#"
            function main(): Integer {
                values: Range<Integer> = range(0, 3);
                return values.next(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("range method arity mismatch should fail in codegen");
    assert!(
        err.contains("Range.next() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_option_none_constructor_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-option-none-arity");
    let source_path = temp_root.join("no_check_invalid_option_none_arity.arden");
    let output_path = temp_root.join("no_check_invalid_option_none_arity");
    let source = r#"
            function main(): Integer {
                value: Option<Integer> = Option.none(1);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Option.none arity mismatch should fail in codegen");
    assert!(
        err.contains("Option.none() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_stdlib_math_abs_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-math-abs-arity");
    let source_path = temp_root.join("no_check_invalid_math_abs_arity.arden");
    let output_path = temp_root.join("no_check_invalid_math_abs_arity");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                return Math.abs();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Math.abs arity mismatch should fail in codegen");
    assert!(
        err.contains("Math__abs() expects 1 argument(s), got 0"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_stdlib_math_pi_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-math-pi-arity");
    let source_path = temp_root.join("no_check_invalid_math_pi_arity.arden");
    let output_path = temp_root.join("no_check_invalid_math_pi_arity");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                return if (Math.pi(1) > 0.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Math.pi arity mismatch should fail in codegen");
    assert!(
        err.contains("Math__pi() expects 0 argument(s), got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_exit_arity_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-exit-arity");
    let source_path = temp_root.join("no_check_invalid_exit_arity.arden");
    let output_path = temp_root.join("no_check_invalid_exit_arity");
    let source = r#"
            function main(): Integer {
                exit();
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("exit arity mismatch should fail in codegen");
    assert!(err.contains("exit() expects 1 argument(s), got 0"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_math_abs_boolean_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-math-abs-boolean");
    let source_path = temp_root.join("no_check_invalid_math_abs_boolean.arden");
    let output_path = temp_root.join("no_check_invalid_math_abs_boolean");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                value: Boolean = true;
                return Math.abs(value);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Math.abs(Boolean) should fail in codegen");
    assert!(
        err.contains("Math.abs() requires numeric type, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_math_min_boolean_arguments_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-math-min-boolean");
    let source_path = temp_root.join("no_check_invalid_math_min_boolean.arden");
    let output_path = temp_root.join("no_check_invalid_math_min_boolean");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                value: Boolean = Math.min(true, false);
                return if (value) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Math.min(Boolean, Boolean) should fail in codegen");
    assert!(
        err.contains("Math.min() arguments must be numeric types, got Boolean and Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_math_min_on_module_local_non_numeric_type_with_user_facing_name()
{
    let temp_root = make_temp_project_root("no-check-invalid-math-min-module-local-type");
    let source_path = temp_root.join("no_check_invalid_math_min_module_local_type.arden");
    let output_path = temp_root.join("no_check_invalid_math_min_module_local_type");
    let source = r#"
            import std.math.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Float {
                return Math.min(M.Box(7), 1.0);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Math.min(module-local Box, Float) should fail in codegen");
    assert!(
        err.contains("Math.min() arguments must be numeric types, got M.Box and Float"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_math_max_on_module_local_non_numeric_type_with_user_facing_name()
{
    let temp_root = make_temp_project_root("no-check-invalid-math-max-module-local-type");
    let source_path = temp_root.join("no_check_invalid_math_max_module_local_type.arden");
    let output_path = temp_root.join("no_check_invalid_math_max_module_local_type");
    let source = r#"
            import std.math.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Float {
                return Math.max(M.Box(7), 1.0);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Math.max(module-local Box, Float) should fail in codegen");
    assert!(
        err.contains("Math.max() arguments must be numeric types, got M.Box and Float"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_logical_operator_on_module_local_non_boolean_type_with_user_facing_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-logical-module-local-type");
    let source_path = temp_root.join("no_check_invalid_logical_module_local_type.arden");
    let output_path = temp_root.join("no_check_invalid_logical_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Boolean {
                return M.Box(7) && true;
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("logical operator on module-local Box should fail in codegen");
    assert!(
        err.contains("Logical operator requires Boolean types, got M.Box and Boolean"),
        "{err}"
    );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_to_float_function_value_signature() {
    let temp_root = make_temp_project_root("no-check-invalid-to-float-fn-value-signature");
    let source_path = temp_root.join("no_check_invalid_to_float_fn_value_signature.arden");
    let output_path = temp_root.join("no_check_invalid_to_float_fn_value_signature");
    let source = r#"
            function main(): Integer {
                f: (Boolean) -> Float = to_float;
                value: Float = f(true);
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid to_float function value signature should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected (Boolean) -> Float, got (unknown) -> Float"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_assert_true_function_value_signature() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-true-fn-value-signature");
    let source_path = temp_root.join("no_check_invalid_assert_true_fn_value_signature.arden");
    let output_path = temp_root.join("no_check_invalid_assert_true_fn_value_signature");
    let source = r#"
            function main(): Integer {
                ensure_true: (Integer) -> None = assert_true;
                ensure_true(1);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid assert_true function value signature should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected (Integer) -> None, got (unknown) -> None"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_match_literal_type_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-match-literal-type");
    let source_path = temp_root.join("no_check_invalid_match_literal_type.arden");
    let output_path = temp_root.join("no_check_invalid_match_literal_type");
    let source = r#"
            function main(): Integer {
                return match (true) {
                    1 => 0,
                    _ => 1,
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("match literal type mismatch should fail in codegen");
    assert!(
        err.contains("Pattern type mismatch: expected Boolean, found Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_match_literal_type_mismatch_with_user_facing_name()
{
    let temp_root = make_temp_project_root("no-check-invalid-match-literal-module-local-type");
    let source_path = temp_root.join("no_check_invalid_match_literal_module_local_type.arden");
    let output_path = temp_root.join("no_check_invalid_match_literal_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                return match (M.Box(1)) {
                    1 => 0,
                    _ => 1,
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local match literal type mismatch should fail in codegen");
    assert!(
        err.contains("Pattern type mismatch: expected M.Box, found Integer"),
        "{err}"
    );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_match_expr_variant_type_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-match-expr-variant-type");
    let source_path = temp_root.join("no_check_invalid_match_expr_variant_type.arden");
    let output_path = temp_root.join("no_check_invalid_match_expr_variant_type");
    let source = r#"
            function main(): Integer {
                return match (true) {
                    Some(v) => 0,
                    _ => 1,
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("match expression variant mismatch should fail in codegen");
    assert!(
        err.contains("Cannot match variant Some on type Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_match_expr_variant_type_mismatch_with_user_facing_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-match-expr-module-local-variant-type");
    let source_path = temp_root.join("no_check_invalid_match_expr_module_local_variant_type.arden");
    let output_path = temp_root.join("no_check_invalid_match_expr_module_local_variant_type");
    let source = r#"
            module M {
                enum Token { Int(Integer) }
            }

            function main(): Integer {
                value: M.Token = M.Token.Int(1);
                return match (value) {
                    Some(v) => 0,
                    _ => 1,
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local match expression variant mismatch should fail");
    assert!(
        err.contains("Cannot match variant Some on type M.Token"),
        "{err}"
    );
    assert!(!err.contains("M__Token"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_match_stmt_variant_type_mismatch_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-match-stmt-variant-type");
    let source_path = temp_root.join("no_check_invalid_match_stmt_variant_type.arden");
    let output_path = temp_root.join("no_check_invalid_match_stmt_variant_type");
    let source = r#"
            function main(): Integer {
                match (true) {
                    Some(v) => { return 0; }
                    _ => { return 1; }
                }
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("match statement variant mismatch should fail in codegen");
    assert!(
        err.contains("Cannot match variant Some on type Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_match_stmt_variant_type_mismatch_with_user_facing_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-match-stmt-module-local-variant-type");
    let source_path = temp_root.join("no_check_invalid_match_stmt_module_local_variant_type.arden");
    let output_path = temp_root.join("no_check_invalid_match_stmt_module_local_variant_type");
    let source = r#"
            module M {
                enum Token { Int(Integer) }
            }

            function main(): Integer {
                value: M.Token = M.Token.Int(1);
                match (value) {
                    Some(v) => { return 0; }
                    _ => { return 1; }
                }
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local match statement variant mismatch should fail");
    assert!(
        err.contains("Cannot match variant Some on type M.Token"),
        "{err}"
    );
    assert!(!err.contains("M__Token"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}
