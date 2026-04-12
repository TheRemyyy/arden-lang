use super::*;
use std::fs;

#[test]
fn compile_source_runs_direct_range_method_calls() {
    let temp_root = make_temp_project_root("direct-range-method-runtime");
    let source_path = temp_root.join("direct_range_method_runtime.arden");
    let output_path = temp_root.join("direct_range_method_runtime");
    let source = r#"
            function main(): Integer {
                if (range(0, 10).has_next()) { return 40; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct range method call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct range method binary");
    assert_eq!(status.code(), Some(40));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_option_some_method_chains() {
    let temp_root = make_temp_project_root("direct-option-some-method-runtime");
    let source_path = temp_root.join("direct_option_some_method_runtime.arden");
    let output_path = temp_root.join("direct_option_some_method_runtime");
    let source = r#"
            function main(): Integer {
                if (Option.some(12).unwrap() == 12) { return 41; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Option.some method chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Option.some method binary");
    assert_eq!(status.code(), Some(41));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_option_some_object_method_chains() {
    let temp_root = make_temp_project_root("direct-option-some-object-method-runtime");
    let source_path = temp_root.join("direct_option_some_object_method_runtime.arden");
    let output_path = temp_root.join("direct_option_some_object_method_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                return Option.some(Boxed(14)).unwrap().value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Option.some object method chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Option.some object method binary");
    assert_eq!(status.code(), Some(14));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_result_ok_method_chains() {
    let temp_root = make_temp_project_root("direct-result-ok-method-runtime");
    let source_path = temp_root.join("direct_result_ok_method_runtime.arden");
    let output_path = temp_root.join("direct_result_ok_method_runtime");
    let source = r#"
            function main(): Integer {
                if (Result.ok(12).unwrap() == 12) { return 42; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Result.ok method chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Result.ok method binary");
    assert_eq!(status.code(), Some(42));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_result_ok_object_method_chains() {
    let temp_root = make_temp_project_root("direct-result-ok-object-method-runtime");
    let source_path = temp_root.join("direct_result_ok_object_method_runtime.arden");
    let output_path = temp_root.join("direct_result_ok_object_method_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                return Result.ok(Boxed(15)).unwrap().value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Result.ok object method chain should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Result.ok object method binary");
    assert_eq!(status.code(), Some(15));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_constructor_method_calls() {
    let temp_root = make_temp_project_root("direct-ctor-method-runtime");
    let source_path = temp_root.join("direct_ctor_method_runtime.arden");
    let output_path = temp_root.join("direct_ctor_method_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function get(): Integer { return this.value; }
            }

            function main(): Integer {
                return Boxed(23).get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct constructor method call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct constructor method binary");
    assert_eq!(status.code(), Some(23));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_result_error_integer_equality() {
    let temp_root = make_temp_project_root("direct-result-error-int-eq-runtime");
    let source_path = temp_root.join("direct_result_error_int_eq_runtime.arden");
    let output_path = temp_root.join("direct_result_error_int_eq_runtime");
    let source = r#"
            function main(): Integer {
                e: Integer = 7;
                if (Result.error(e) == Result.error(e)) { return 43; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Result.error integer equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Result.error integer equality binary");
    assert_eq!(status.code(), Some(43));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_result_error_object_identity_equality() {
    let temp_root = make_temp_project_root("direct-result-error-object-eq-runtime");
    let source_path = temp_root.join("direct_result_error_object_eq_runtime.arden");
    let output_path = temp_root.join("direct_result_error_object_eq_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                e: Boxed = Boxed(9);
                if (Result.error(e) == Result.error(e)) { return 44; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct Result.error object equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct Result.error object equality binary");
    assert_eq!(status.code(), Some(44));

    let _ = fs::remove_dir_all(temp_root);
}
