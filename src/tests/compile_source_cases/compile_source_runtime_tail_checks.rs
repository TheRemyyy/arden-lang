use super::*;
use std::fs;

#[test]
fn compile_source_runs_borrowed_list_iteration_runtime() {
    let temp_root = make_temp_project_root("borrowed-list-iteration-runtime");
    let source_path = temp_root.join("borrowed_list_iteration_runtime.arden");
    let output_path = temp_root.join("borrowed_list_iteration_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                xs.push(2);
                view: &List<Integer> = &xs;
                mut total: Integer = 0;
                for (x in view) {
                    total = total + x;
                }
                return if (total == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed list iteration should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed list iteration binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_range_iteration_runtime() {
    let temp_root = make_temp_project_root("borrowed-range-iteration-runtime");
    let source_path = temp_root.join("borrowed_range_iteration_runtime.arden");
    let output_path = temp_root.join("borrowed_range_iteration_runtime");
    let source = r#"
            function main(): Integer {
                r: Range<Integer> = range(0, 3);
                rr: &Range<Integer> = &r;
                mut total: Integer = 0;
                for (x in rr) {
                    total = total + x;
                }
                return if (total == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed range iteration should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed range iteration binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_negative_list_index_assignment() {
    let temp_root = make_temp_project_root("list-index-assign-negative-runtime");
    let source_path = temp_root.join("list_index_assign_negative_runtime.arden");
    let output_path = temp_root.join("list_index_assign_negative_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                index: Integer = -1;
                xs[index] = 25;
                return 25;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("negative list assignment should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled negative list assignment binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_missing_map_index_object_results() {
    let temp_root = make_temp_project_root("map-index-missing-object-runtime");
    let source_path = temp_root.join("map_index_missing_object_runtime.arden");
    let output_path = temp_root.join("map_index_missing_object_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                return m[1].value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("missing map index object result should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled missing map index object binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_empty_list_get_object_results() {
    let temp_root = make_temp_project_root("list-get-empty-object-runtime");
    let source_path = temp_root.join("list_get_empty_object_runtime.arden");
    let output_path = temp_root.join("list_get_empty_object_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                xs: List<Boxed> = List<Boxed>();
                return xs.get(0).value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("empty list.get object result should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled empty list.get object binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_empty_list_pop_object_results() {
    let temp_root = make_temp_project_root("list-pop-empty-object-runtime");
    let source_path = temp_root.join("list_pop_empty_object_runtime.arden");
    let output_path = temp_root.join("list_pop_empty_object_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                xs: List<Boxed> = List<Boxed>();
                return xs.pop().value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("empty list.pop object result should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled empty list.pop object binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_negative_list_get_index() {
    let temp_root = make_temp_project_root("list-get-negative-index-runtime");
    let source_path = temp_root.join("list_get_negative_index_runtime.arden");
    let output_path = temp_root.join("list_get_negative_index_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(1);
                index: Integer = -1;
                return xs.get(index);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("negative list.get index should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled negative list.get binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_negative_list_index_operator() {
    let temp_root = make_temp_project_root("list-index-negative-runtime");
    let source_path = temp_root.join("list_index_negative_runtime.arden");
    let output_path = temp_root.join("list_index_negative_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(1);
                index: Integer = -1;
                return xs[index];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("negative list index operator should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled negative list index operator binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_index_operator() {
    let temp_root = make_temp_project_root("string-index-runtime");
    let source_path = temp_root.join("string_index_runtime.arden");
    let output_path = temp_root.join("string_index_runtime");
    let source = r#"
            function main(): Integer {
                c: Char = "abc"[1];
                if (c == 'b') { return 19; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string index operator should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string index binary");
    assert_eq!(status.code(), Some(19));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_main_with_string_return_type() {
    let temp_root = make_temp_project_root("main-string-return-type");
    let source_path = temp_root.join("main_string_return_type.arden");
    let output_path = temp_root.join("main_string_return_type");
    let source = r#"
            function main(): String {
                return "oops";
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("main string return type should fail before codegen");
    assert!(
        err.to_string()
            .contains("main() must return None or Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_file_no_check_rejects_main_with_string_return_type_cleanly() {
    let temp_root = make_temp_project_root("main-string-return-type-nocheck");
    let source_path = temp_root.join("main_string_return_type_nocheck.arden");
    let source = r#"
            function main(): String {
                return "oops";
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_file(&source_path, None, false, false, None, None)
        .must_err("unchecked main string return type should fail before codegen");
    assert!(err.contains("main() must return None or Integer"), "{err}");
    assert!(!err.contains("Clang failed"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_file_no_check_rejects_main_with_boolean_return_type_cleanly() {
    let temp_root = make_temp_project_root("main-boolean-return-type-nocheck");
    let source_path = temp_root.join("main_boolean_return_type_nocheck.arden");
    let source = r#"
            function main(): Integer {
                return true;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_file(&source_path, None, false, false, None, None)
        .must_err("unchecked main boolean return type should fail before codegen");
    assert!(
        err.contains("Type mismatch: expected Integer, got Boolean"),
        "{err}"
    );
    assert!(!err.contains("Clang failed"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_function_with_boolean_return_type_cleanly() {
    let temp_root = make_temp_project_root("function-boolean-return-type-nocheck");
    let source_path = temp_root.join("function_boolean_return_type_nocheck.arden");
    let output_path = temp_root.join("function_boolean_return_type_nocheck");
    let source = r#"
            function f(): Integer {
                return true;
            }

            function main(): Integer {
                return f();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unchecked function boolean return type should fail before codegen");
    assert!(
        err.contains("Type mismatch: expected Integer, got Boolean"),
        "{err}"
    );
    assert!(!err.contains("Clang failed"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_main_with_parameters() {
    let temp_root = make_temp_project_root("main-parameters");
    let source_path = temp_root.join("main_parameters.arden");
    let output_path = temp_root.join("main_parameters");
    let source = r#"
            function main(x: Integer): Integer {
                return x;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("main parameters should fail before codegen");
    assert!(
        err.to_string().contains("main() cannot declare parameters"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_try_outside_result_or_option_return_context() {
    let temp_root = make_temp_project_root("try-invalid-return-context");
    let source_path = temp_root.join("try_invalid_return_context.arden");
    let output_path = temp_root.join("try_invalid_return_context");
    let source = r#"
            function choose(): Result<Integer, String> {
                return Result.ok(1);
            }

            function helper(): Integer {
                value: Integer = choose()?;
                return value;
            }

            function main(): Integer {
                return helper();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("invalid try return context should fail before codegen");
    assert!(
        err.contains("'?' on Result requires the enclosing function to return Result"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_try_inside_lambda_even_with_outer_result_return() {
    let temp_root = make_temp_project_root("try-invalid-lambda-context");
    let source_path = temp_root.join("try_invalid_lambda_context.arden");
    let output_path = temp_root.join("try_invalid_lambda_context");
    let source = r#"
            function choose(): Result<Integer, String> {
                return Result.ok(1);
            }

            function wrap(): Result<Integer, String> {
                f: () -> Integer = () => choose()?;
                return Result.ok(f());
            }

            function main(): Integer {
                return wrap().unwrap();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("invalid try inside lambda should fail before codegen");
    assert!(
        err.contains("'?' on Result requires the enclosing function to return Result"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_invalid_opt_level() {
    let temp_root = make_temp_project_root("compile-invalid-opt");
    let source_path = temp_root.join("invalid_opt.arden");
    let output_path = temp_root.join("invalid_opt");
    let source = "function main(): None { return None; }\n";

    let err = compile_source(
        source,
        &source_path,
        &output_path,
        true,
        true,
        Some("turbo"),
        None,
    )
    .must_err("invalid opt level should be rejected");

    assert!(err.contains("Invalid optimization level"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_surfaces_lexer_error_at_boundary() {
    let temp_root = make_temp_project_root("compile-lexer-boundary-error");
    let source_path = temp_root.join("lexer_boundary_error.arden");
    let output_path = temp_root.join("lexer_boundary_error");
    let source = "function main(): None { $ return None; }\n";

    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("invalid token should fail at compile_source boundary");

    assert!(err.contains("Lexer error"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_out_of_bounds_string_index_operator() {
    let temp_root = make_temp_project_root("string-index-oob-runtime");
    let source_path = temp_root.join("string_index_oob_runtime.arden");
    let output_path = temp_root.join("string_index_oob_runtime");
    let source = r#"
            function main(): Integer {
                idx: Integer = 10;
                c: Char = "abc"[idx];
                return 20;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("out-of-bounds string index should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string index oob binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unicode_string_literal_index_operator() {
    let temp_root = make_temp_project_root("unicode-string-index-runtime");
    let source_path = temp_root.join("unicode_string_index_runtime.arden");
    let output_path = temp_root.join("unicode_string_index_runtime");
    let source = r#"
            function main(): Integer {
                c: Char = "🚀"[0];
                return if (c == '🚀') { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("unicode string literal index should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unicode string index binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
