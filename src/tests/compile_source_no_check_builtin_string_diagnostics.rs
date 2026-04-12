use super::*;
use std::fs;

#[test]
fn compile_source_no_check_rejects_non_string_str_len_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-len-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_len_argument_type.arden");
    let output_path = temp_root.join("no_check_invalid_str_len_argument_type");
    let source = r#"
            import std.str.*;

            function main(): Integer {
                return Str.len(true);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Str.len(Boolean) should fail in codegen");
    assert!(
        err.contains("Str.len() requires String, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_string_str_len_argument_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-str-len-module-local-type");
    let source_path = temp_root.join("no_check_invalid_str_len_module_local_type.arden");
    let output_path = temp_root.join("no_check_invalid_str_len_module_local_type");
    let source = r#"
            import std.str.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                return Str.len(M.Box(7));
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Str.len(module-local Box) should fail in codegen");
    assert!(
        err.contains("Str.len() requires String, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_compare_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-compare-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_compare_argument_type.arden");
    let output_path = temp_root.join("no_check_invalid_str_compare_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: Integer = Str.compare(true, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Str.compare(Boolean, String) should fail in codegen");
    assert!(
        err.contains("Str.compare() requires String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_concat_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-concat-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_concat_argument_type.arden");
    let output_path = temp_root.join("no_check_invalid_str_concat_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: String = Str.concat(true, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Str.concat(Boolean, String) should fail in codegen");
    assert!(
        err.contains("Str.concat() requires String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_upper_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-upper-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_upper_argument_type.arden");
    let output_path = temp_root.join("no_check_invalid_str_upper_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: String = Str.upper(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Str.upper(Boolean) should fail in codegen");
    assert!(err.contains("Str.upper() requires String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_lower_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-lower-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_lower_argument_type.arden");
    let output_path = temp_root.join("no_check_invalid_str_lower_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: String = Str.lower(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Str.lower(Boolean) should fail in codegen");
    assert!(err.contains("Str.lower() requires String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_trim_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-trim-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_trim_argument_type.arden");
    let output_path = temp_root.join("no_check_invalid_str_trim_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: String = Str.trim(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Str.trim(Boolean) should fail in codegen");
    assert!(err.contains("Str.trim() requires String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_contains_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-contains-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_contains_argument_type.arden");
    let output_path = temp_root.join("no_check_invalid_str_contains_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: Boolean = Str.contains(true, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Str.contains(Boolean, String) should fail in codegen");
    assert!(
        err.contains("Str.contains() requires two String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_starts_with_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-starts-with-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_starts_with_argument_type.arden");
    let output_path = temp_root.join("no_check_invalid_str_starts_with_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: Boolean = Str.startsWith(true, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Str.startsWith(Boolean, String) should fail in codegen");
    assert!(
        err.contains("Str.startsWith() requires two String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_str_ends_with_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-str-ends-with-argument-type");
    let source_path = temp_root.join("no_check_invalid_str_ends_with_argument_type.arden");
    let output_path = temp_root.join("no_check_invalid_str_ends_with_argument_type");
    let source = r#"
            import std.str.*;

            function main(): None {
                value: Boolean = Str.endsWith(true, "a");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Str.endsWith(Boolean, String) should fail in codegen");
    assert!(
        err.contains("Str.endsWith() requires two String arguments"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_numeric_to_float_string_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-to-float-string-argument-type");
    let source_path = temp_root.join("no_check_invalid_to_float_string_argument_type.arden");
    let output_path = temp_root.join("no_check_invalid_to_float_string_argument_type");
    let source = r#"
            function main(): Float {
                return to_float("8");
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("to_float(String) should fail in codegen");
    assert!(
        err.contains("to_float() requires Integer or Float, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_numeric_to_float_boolean_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-to-float-boolean-argument-type");
    let source_path = temp_root.join("no_check_invalid_to_float_boolean_argument_type.arden");
    let output_path = temp_root.join("no_check_invalid_to_float_boolean_argument_type");
    let source = r#"
            function main(): Float {
                return to_float(true);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("to_float(Boolean) should fail in codegen");
    assert!(
        err.contains("to_float() requires Integer or Float, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_numeric_to_float_argument_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-to-float-module-local-type");
    let source_path = temp_root.join("no_check_invalid_to_float_module_local_type.arden");
    let output_path = temp_root.join("no_check_invalid_to_float_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Float {
                return to_float(M.Box(7));
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("to_float(module-local Box) should fail in codegen");
    assert!(
        err.contains("to_float() requires Integer or Float, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_supported_to_int_boolean_argument_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-to-int-boolean-argument-type");
    let source_path = temp_root.join("no_check_invalid_to_int_boolean_argument_type.arden");
    let output_path = temp_root.join("no_check_invalid_to_int_boolean_argument_type");
    let source = r#"
            function main(): Integer {
                return to_int(true);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("to_int(Boolean) should fail in codegen");
    assert!(
        err.contains("to_int() requires Integer, Float, or String, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_supported_to_int_argument_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-to-int-module-local-type");
    let source_path = temp_root.join("no_check_invalid_to_int_module_local_type.arden");
    let output_path = temp_root.join("no_check_invalid_to_int_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                return to_int(M.Box(7));
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("to_int(module-local Box) should fail in codegen");
    assert!(
        err.contains("to_int() requires Integer, Float, or String, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_zero_range_step_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-range-zero-step");
    let source_path = temp_root.join("no_check_invalid_range_zero_step.arden");
    let output_path = temp_root.join("no_check_invalid_range_zero_step");
    let source = r#"
            function main(): Integer {
                value: Range<Integer> = range(0, 10, 0);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("range(..., 0) should fail in codegen");
    assert!(err.contains("range() step cannot be 0"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_time_sleep_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-time-sleep-negative-constant");
    let source_path = temp_root.join("no_check_invalid_time_sleep_negative_constant.arden");
    let output_path = temp_root.join("no_check_invalid_time_sleep_negative_constant");
    let source = r#"
            import std.time.*;

            function main(): None {
                Time.sleep(-1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Time.sleep(-1) should fail in codegen");
    assert!(
        err.contains("Time.sleep() milliseconds must be non-negative"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_args_get_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-args-get-negative-constant");
    let source_path = temp_root.join("no_check_invalid_args_get_negative_constant.arden");
    let output_path = temp_root.join("no_check_invalid_args_get_negative_constant");
    let source = r#"
            import std.args.*;

            function main(): None {
                value: String = Args.get(-1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Args.get(-1) should fail in codegen");
    assert!(err.contains("Args.get() index cannot be negative"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_await_timeout_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-await-timeout-negative-constant");
    let source_path = temp_root.join("no_check_invalid_await_timeout_negative_constant.arden");
    let output_path = temp_root.join("no_check_invalid_await_timeout_negative_constant");
    let source = r#"
            async function work(): Task<Integer> {
                return 1;
            }

            function main(): None {
                value: Option<Integer> = work().await_timeout(-1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("await_timeout(-1) should fail in codegen");
    assert!(
        err.contains("Task.await_timeout() timeout must be non-negative"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_await_on_string_literal_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-await-string-literal");
    let source_path = temp_root.join("no_check_invalid_await_string_literal.arden");
    let output_path = temp_root.join("no_check_invalid_await_string_literal");
    let source = r#"
            function main(): String {
                return await "hi";
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("await on String literal should fail in codegen");
    assert!(
        err.contains("'await' can only be used on Task types, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_await_on_string_local_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-await-string-local");
    let source_path = temp_root.join("no_check_invalid_await_string_local.arden");
    let output_path = temp_root.join("no_check_invalid_await_string_local");
    let source = r#"
            function main(): String {
                value: String = "hi";
                return await value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("await on String local should fail in codegen");
    assert!(
        err.contains("'await' can only be used on Task types, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_await_on_box_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-await-box");
    let source_path = temp_root.join("no_check_invalid_await_box.arden");
    let output_path = temp_root.join("no_check_invalid_await_box");
    let source = r#"
            function main(): Integer {
                value: Box<Integer> = Box<Integer>(7);
                return await value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("await on Box<Integer> should fail in codegen");
    assert!(
        err.contains("'await' can only be used on Task types, got Box<Integer>"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_await_on_module_local_box_with_user_facing_type_name() {
    let temp_root = make_temp_project_root("no-check-invalid-await-module-local-box");
    let source_path = temp_root.join("no_check_invalid_await_module_local_box.arden");
    let output_path = temp_root.join("no_check_invalid_await_module_local_box");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Integer {
                return await M.Box(7);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("await on module-local Box should fail in codegen");
    assert!(
        err.contains("'await' can only be used on Task types, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_println_on_unsupported_display_type_with_type_name() {
    let temp_root = make_temp_project_root("no-check-invalid-println-unsupported-display");
    let source_path = temp_root.join("no_check_invalid_println_unsupported_display.arden");
    let output_path = temp_root.join("no_check_invalid_println_unsupported_display");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): None {
                b: Box = Box(7);
                println(b);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("println(Box) should fail in codegen");
    assert!(
            err.contains(
                "println() currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got Box"
            ),
            "{err}"
        );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_interpolation_on_unsupported_display_type_with_type_name() {
    let temp_root = make_temp_project_root("no-check-invalid-interpolation-unsupported-display");
    let source_path = temp_root.join("no_check_invalid_interpolation_unsupported_display.arden");
    let output_path = temp_root.join("no_check_invalid_interpolation_unsupported_display");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function render(): String {
                b: Box = Box(7);
                return "box={b}";
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("string interpolation on Box should fail in codegen");
    assert!(
            err.contains(
                "display formatting currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got Box"
            ),
            "{err}"
        );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_interpolation_on_module_local_unsupported_display_type_with_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-interpolation-module-local-display");
    let source_path = temp_root.join("no_check_invalid_interpolation_module_local_display.arden");
    let output_path = temp_root.join("no_check_invalid_interpolation_module_local_display");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): String {
                return "box={M.Box(7)}";
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("string interpolation on module-local Box should fail in codegen");
    assert!(
            err.contains(
                "display formatting currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got M.Box"
            ),
            "{err}"
        );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_println_on_module_local_unsupported_display_type_with_type_name()
{
    let temp_root = make_temp_project_root("no-check-invalid-println-module-local-display");
    let source_path = temp_root.join("no_check_invalid_println_module_local_display.arden");
    let output_path = temp_root.join("no_check_invalid_println_module_local_display");
    let source = r#"
            import std.io.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): None {
                println(M.Box(7));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("println on module-local Box should fail in codegen");
    assert!(
            err.contains(
                "println() currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got M.Box"
            ),
            "{err}"
        );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_to_string_on_module_local_unsupported_display_type_with_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-to-string-module-local-display");
    let source_path = temp_root.join("no_check_invalid_to_string_module_local_display.arden");
    let output_path = temp_root.join("no_check_invalid_to_string_module_local_display");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): String {
                return to_string(M.Box(7));
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("to_string on module-local Box should fail in codegen");
    assert!(
            err.contains(
                "to_string() currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got M.Box"
            ),
            "{err}"
        );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}
