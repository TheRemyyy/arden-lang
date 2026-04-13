use super::*;
use std::fs;

#[test]
fn compile_source_no_check_rejects_assignment_to_immutable_variable_in_codegen() {
    let temp_root = make_temp_project_root("no-check-immutable-local-assign");
    let source_path = temp_root.join("no_check_immutable_local_assign.arden");
    let output_path = temp_root.join("no_check_immutable_local_assign");
    let source = r#"
            function main(): Integer {
                value: Integer = 1;
                value = 9;
                return value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("immutable local assignment should fail in codegen without checks");
    assert!(
        err.contains("Cannot assign to immutable variable 'value'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_assignment_through_immutable_reference_in_codegen() {
    let temp_root = make_temp_project_root("no-check-immutable-ref-assign");
    let source_path = temp_root.join("no_check_immutable_ref_assign.arden");
    let output_path = temp_root.join("no_check_immutable_ref_assign");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                view: &List<Integer> = &xs;
                view[0] = 7;
                return xs[0];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("immutable reference assignment should fail in codegen without checks");
    assert!(
        err.contains("Cannot assign through immutable reference 'view'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_deref_assignment_through_immutable_reference_in_codegen() {
    let temp_root = make_temp_project_root("no-check-immutable-deref-assign");
    let source_path = temp_root.join("no_check_immutable_deref_assign.arden");
    let output_path = temp_root.join("no_check_immutable_deref_assign");
    let source = r#"
            function main(): Integer {
                mut value: Integer = 1;
                r: &Integer = &value;
                *r = 9;
                return value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("immutable deref assignment should fail in codegen without checks");
    assert!(
        err.contains("Cannot assign through immutable reference 'r'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_function_parameter_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-function-parameter-assignment-runtime");
    let source_path = temp_root.join("mutable_function_parameter_assignment_runtime.arden");
    let output_path = temp_root.join("mutable_function_parameter_assignment_runtime");
    let source = r#"
            function bump(mut value: Integer): Integer {
                value = value + 3;
                return value;
            }

            function main(): Integer {
                return if (bump(4) == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable function parameter assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable function parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_method_parameter_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-method-parameter-assignment-runtime");
    let source_path = temp_root.join("mutable_method_parameter_assignment_runtime.arden");
    let output_path = temp_root.join("mutable_method_parameter_assignment_runtime");
    let source = r#"
            class Counter {
                function bump(mut value: Integer): Integer {
                    value += 5;
                    return value;
                }
            }

            function main(): Integer {
                counter: Counter = Counter();
                return if (counter.bump(2) == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable method parameter assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable method parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_constructor_parameter_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-constructor-parameter-assignment-runtime");
    let source_path = temp_root.join("mutable_constructor_parameter_assignment_runtime.arden");
    let output_path = temp_root.join("mutable_constructor_parameter_assignment_runtime");
    let source = r#"
            class Boxed {
                value: Integer;

                constructor(mut value: Integer) {
                    value += 2;
                    this.value = value;
                }
            }

            function main(): Integer {
                boxed: Boxed = Boxed(5);
                return if (boxed.value == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable constructor parameter assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable constructor parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrow_mut_parameter_read_runtime() {
    let temp_root = make_temp_project_root("borrow-mut-parameter-read-runtime");
    let source_path = temp_root.join("borrow_mut_parameter_read_runtime.arden");
    let output_path = temp_root.join("borrow_mut_parameter_read_runtime");
    let source = r#"
            function len_plus_one(borrow mut value: String): Integer {
                return value.length() + 1;
            }

            function main(): Integer {
                mut text: String = "abc";
                return if (len_plus_one(text) == 4) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrow mut parameter reads should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrow mut read binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrow_mut_parameter_assignment_runtime() {
    let temp_root = make_temp_project_root("borrow-mut-parameter-assignment-runtime");
    let source_path = temp_root.join("borrow_mut_parameter_assignment_runtime.arden");
    let output_path = temp_root.join("borrow_mut_parameter_assignment_runtime");
    let source = r#"
            function edit(borrow mut value: Integer): Integer {
                value += 1;
                return value;
            }

            function main(): Integer {
                mut n: Integer = 1;
                return if (edit(n) == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrow mut parameter assignments should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrow mut assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_async_parameter_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-async-parameter-assignment-runtime");
    let source_path = temp_root.join("mutable_async_parameter_assignment_runtime.arden");
    let output_path = temp_root.join("mutable_async_parameter_assignment_runtime");
    let source = r#"
            async function bump(mut value: Integer): Integer {
                value = value * 2;
                return value;
            }

            function main(): Integer {
                return if (await(bump(6)) == 12) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable async parameter assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable async parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
