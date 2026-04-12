use super::*;
use std::fs;

#[test]
fn compile_source_no_check_rejects_invalid_list_constructor_arity_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-ctor-arity");
    let source_path = temp_root.join("no_check_invalid_list_ctor_arity.arden");
    let output_path = temp_root.join("no_check_invalid_list_ctor_arity");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>(1, 2);
                return xs.length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid list constructor arity should fail in codegen without checks");
    assert!(
        err.contains("Constructor List<Integer> expects 0 or 1 arguments, got 2"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_list_capacity_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-capacity-type");
    let source_path = temp_root.join("no_check_invalid_list_capacity_type.arden");
    let output_path = temp_root.join("no_check_invalid_list_capacity_type");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>("bad");
                return xs.length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-integer list capacity should fail in codegen without checks");
    assert!(
        err.contains("Constructor List<Integer> expects optional Integer capacity, got String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_map_constructor_arity_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-map-ctor-arity");
    let source_path = temp_root.join("no_check_invalid_map_ctor_arity.arden");
    let output_path = temp_root.join("no_check_invalid_map_ctor_arity");
    let source = r#"
            function main(): Integer {
                items: Map<String, Integer> = Map<String, Integer>(1);
                return items.length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("invalid map constructor arity should fail in codegen without checks");
    assert!(
        err.contains("Constructor Map<String, Integer> expects 0 arguments, got 1"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_module_local_constructor_in_single_file_mode() {
    let temp_root = make_temp_project_root("no-check-module-local-constructor-runtime");
    let source_path = temp_root.join("no_check_module_local_constructor_runtime.arden");
    let output_path = temp_root.join("no_check_module_local_constructor_runtime");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) {
                        this.value = value;
                    }
                }
            }

            function make(): M.Box {
                return M.Box(7);
            }

            function main(): Integer {
                value: M.Box = make();
                return value.value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("module-local constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled module-local constructor binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_current_package_namespace_alias_constructor() {
    let temp_root = make_temp_project_root("no-check-current-package-namespace-alias-ctor");
    let source_path = temp_root.join("no_check_current_package_namespace_alias_ctor.arden");
    let output_path = temp_root.join("no_check_current_package_namespace_alias_ctor");
    let source = r#"
            package app;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) {
                        this.value = value;
                    }
                }
            }

            import app as root;

            function main(): Integer {
                value: root.M.Box = root.M.Box(7);
                return value.value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("current-package namespace alias constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled current-package namespace alias constructor binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}
