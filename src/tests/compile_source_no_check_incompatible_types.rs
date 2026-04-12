use super::*;
use std::fs;

#[test]
fn compile_source_no_check_rejects_return_value_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-return-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_return_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_return_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            function make(): B {
                return A();
            }

            function main(): Integer {
                value: B = make();
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class return value should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_async_tail_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-async-tail-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_async_tail_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_async_tail_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            function main(): Integer {
                task: Task<B> = async { A() };
                value: B = await(task);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class async tail should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_match_arm_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-match-arm-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_match_arm_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_match_arm_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            enum Choice {
                Left
                Right
            }

            function main(): Integer {
                value: B = match (Choice.Left) {
                    Choice.Left => A(),
                    Choice.Right => B(),
                };
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class match arm should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_if_branch_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-if-branch-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_if_branch_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_if_branch_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            function main(): Integer {
                value: Option<B> = Option<B>();
                got: B = if (value.is_some()) { value.unwrap() } else { A() };
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class if branch should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_if_branch_with_incompatible_non_class_type() {
    let temp_root = make_temp_project_root("no-check-if-branch-incompatible-non-class-type");
    let source_path = temp_root.join("no_check_if_branch_incompatible_non_class_type.arden");
    let output_path = temp_root.join("no_check_if_branch_incompatible_non_class_type");
    let source = r#"
            class A {
                constructor() {}
            }

            function main(): Integer {
                flag: Boolean = true;
                value: Integer = if (flag) { 1 } else { A() };
                return value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible if branch should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected Integer, got A"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_let_with_incompatible_non_class_type() {
    let temp_root = make_temp_project_root("no-check-let-incompatible-non-class-type");
    let source_path = temp_root.join("no_check_let_incompatible_non_class_type.arden");
    let output_path = temp_root.join("no_check_let_incompatible_non_class_type");
    let source = r#"
            function main(): Integer {
                value: Integer = "oops";
                return value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible let binding should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected Integer, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_assign_with_incompatible_non_class_type() {
    let temp_root = make_temp_project_root("no-check-assign-incompatible-non-class-type");
    let source_path = temp_root.join("no_check_assign_incompatible_non_class_type.arden");
    let output_path = temp_root.join("no_check_assign_incompatible_non_class_type");
    let source = r#"
            function main(): Integer {
                mut value: Integer = 1;
                value = "oops";
                return value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible assignment should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected Integer, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_list_push_with_incompatible_non_class_type() {
    let temp_root = make_temp_project_root("no-check-list-push-incompatible-non-class-type");
    let source_path = temp_root.join("no_check_list_push_incompatible_non_class_type.arden");
    let output_path = temp_root.join("no_check_list_push_incompatible_non_class_type");
    let source = r#"
            function main(): Integer {
                values: List<Integer> = List<Integer>();
                values.push("oops");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible list push should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected Integer, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_option_some_with_incompatible_non_class_type() {
    let temp_root = make_temp_project_root("no-check-option-some-incompatible-non-class-type");
    let source_path = temp_root.join("no_check_option_some_incompatible_non_class_type.arden");
    let output_path = temp_root.join("no_check_option_some_incompatible_non_class_type");
    let source = r#"
            function main(): Integer {
                value: Option<Integer> = Option.Some("oops");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible Option.Some payload should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected Integer, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_result_ok_with_incompatible_non_class_type() {
    let temp_root = make_temp_project_root("no-check-result-ok-incompatible-non-class-type");
    let source_path = temp_root.join("no_check_result_ok_incompatible_non_class_type.arden");
    let output_path = temp_root.join("no_check_result_ok_incompatible_non_class_type");
    let source = r#"
            function main(): Integer {
                value: Result<Integer, String> = Result.Ok("oops");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible Result.Ok payload should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected Integer, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_builtin_box_with_incompatible_payload_type() {
    let temp_root = make_temp_project_root("no-check-builtin-box-incompatible-payload-type");
    let source_path = temp_root.join("no_check_builtin_box_incompatible_payload_type.arden");
    let output_path = temp_root.join("no_check_builtin_box_incompatible_payload_type");
    let source = r#"
            function main(): Integer {
                value: Box<Integer> = Box<String>("oops");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible Box payload specialization should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected Box<Integer>, got Box<String>"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_builtin_rc_with_incompatible_payload_type() {
    let temp_root = make_temp_project_root("no-check-builtin-rc-incompatible-payload-type");
    let source_path = temp_root.join("no_check_builtin_rc_incompatible_payload_type.arden");
    let output_path = temp_root.join("no_check_builtin_rc_incompatible_payload_type");
    let source = r#"
            function main(): Integer {
                value: Rc<Integer> = Rc<String>("oops");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible Rc payload specialization should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected Rc<Integer>, got Rc<String>"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_builtin_arc_with_incompatible_payload_type() {
    let temp_root = make_temp_project_root("no-check-builtin-arc-incompatible-payload-type");
    let source_path = temp_root.join("no_check_builtin_arc_incompatible_payload_type.arden");
    let output_path = temp_root.join("no_check_builtin_arc_incompatible_payload_type");
    let source = r#"
            function main(): Integer {
                value: Arc<Integer> = Arc<String>("oops");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible Arc payload specialization should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected Arc<Integer>, got Arc<String>"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_builtin_box_argument_with_incompatible_payload_type() {
    let temp_root = make_temp_project_root("no-check-builtin-box-arg-incompatible-payload-type");
    let source_path = temp_root.join("no_check_builtin_box_arg_incompatible_payload_type.arden");
    let output_path = temp_root.join("no_check_builtin_box_arg_incompatible_payload_type");
    let source = r#"
            function take(value: Box<Integer>): Integer {
                return 0;
            }

            function main(): Integer {
                return take(Box<String>("oops"));
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible Box argument specialization should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected Box<Integer>, got Box<String>"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_builtin_list_with_incompatible_specialization() {
    let temp_root = make_temp_project_root("no-check-builtin-list-incompatible-specialization");
    let source_path = temp_root.join("no_check_builtin_list_incompatible_specialization.arden");
    let output_path = temp_root.join("no_check_builtin_list_incompatible_specialization");
    let source = r#"
            function main(): Integer {
                value: List<Option<Integer>> = List<Option<String>>();
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible List specialization should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected List<Option<Integer>>, got List<Option<String>>"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_builtin_map_with_incompatible_specialization() {
    let temp_root = make_temp_project_root("no-check-builtin-map-incompatible-specialization");
    let source_path = temp_root.join("no_check_builtin_map_incompatible_specialization.arden");
    let output_path = temp_root.join("no_check_builtin_map_incompatible_specialization");
    let source = r#"
            function main(): Integer {
                value: Map<String, Integer> = Map<String, String>();
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible Map specialization should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected Map<String, Integer>, got Map<String, String>"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_builtin_option_with_incompatible_specialization() {
    let temp_root = make_temp_project_root("no-check-builtin-option-incompatible-specialization");
    let source_path = temp_root.join("no_check_builtin_option_incompatible_specialization.arden");
    let output_path = temp_root.join("no_check_builtin_option_incompatible_specialization");
    let source = r#"
            function main(): Integer {
                value: Option<List<Integer>> = Option<List<String>>();
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible Option specialization should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected Option<List<Integer>>, got Option<List<String>>"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_if_block_binding_outside_scope() {
    let temp_root = make_temp_project_root("no-check-if-scope-binding");
    let source_path = temp_root.join("no_check_if_scope_binding.arden");
    let output_path = temp_root.join("no_check_if_scope_binding");
    let source = r#"
            function main(): Integer {
                if (true) {
                    leaked: Integer = 7;
                }
                return leaked;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("if-block locals should not leak outside their scope");
    assert!(err.contains("Undefined variable: leaked"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_match_arm_binding_outside_scope() {
    let temp_root = make_temp_project_root("no-check-match-scope-binding");
    let source_path = temp_root.join("no_check_match_scope_binding.arden");
    let output_path = temp_root.join("no_check_match_scope_binding");
    let source = r#"
            function main(): Integer {
                match (true) {
                    true => { leaked: Integer = 7; }
                    false => { }
                }
                return leaked;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("match-arm locals should not leak outside their scope");
    assert!(err.contains("Undefined variable: leaked"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_match_arm_with_incompatible_non_class_type() {
    let temp_root = make_temp_project_root("no-check-match-arm-incompatible-non-class-type");
    let source_path = temp_root.join("no_check_match_arm_incompatible_non_class_type.arden");
    let output_path = temp_root.join("no_check_match_arm_incompatible_non_class_type");
    let source = r#"
            class A {
                constructor() {}
            }

            function main(): Integer {
                value: Integer = match (true) {
                    true => 1,
                    false => A(),
                };
                return value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("incompatible match arm should fail in codegen");
    assert!(
        err.contains("Type mismatch: expected Integer, got A"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_for_binding_outside_scope() {
    let temp_root = make_temp_project_root("no-check-integer-for-scope-binding");
    let source_path = temp_root.join("no_check_integer_for_scope_binding.arden");
    let output_path = temp_root.join("no_check_integer_for_scope_binding");
    let source = r#"
            function main(): Integer {
                for (i in 4) {
                }
                return i;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("integer for-loop binding should not leak outside its scope");
    assert!(err.contains("Undefined variable: i"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_list_for_binding_outside_scope() {
    let temp_root = make_temp_project_root("no-check-list-for-scope-binding");
    let source_path = temp_root.join("no_check_list_for_scope_binding.arden");
    let output_path = temp_root.join("no_check_list_for_scope_binding");
    let source = r#"
            function main(): Integer {
                values: List<Integer> = List<Integer>();
                values.push(1);
                for (item in values) {
                }
                return item;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("list for-loop binding should not leak outside its scope");
    assert!(err.contains("Undefined variable: item"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_builtin_to_float_function_value_runtime() {
    let temp_root = make_temp_project_root("builtin-to-float-fn-value-runtime");
    let source_path = temp_root.join("builtin_to_float_fn_value_runtime.arden");
    let output_path = temp_root.join("builtin_to_float_fn_value_runtime");
    let source = r#"
            function main(): Integer {
                conv: (Integer) -> Float = to_float;
                return if (conv(3) == 3.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("to_float function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled to_float function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
