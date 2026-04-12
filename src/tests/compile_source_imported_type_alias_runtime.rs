use super::*;
use std::fs;

#[test]
fn compile_source_runs_imported_top_level_type_alias_runtime() {
    let temp_root = make_temp_project_root("imported-top-level-type-alias-runtime");
    let source_path = temp_root.join("imported_top_level_type_alias_runtime.arden");
    let output_path = temp_root.join("imported_top_level_type_alias_runtime");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function get(): Integer { return this.value; }
            }
            import Box as B;
            function main(): Integer {
                return B(2).get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported top-level type alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported top-level type alias binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_nested_type_alias_runtime() {
    let temp_root = make_temp_project_root("imported-nested-type-alias-runtime");
    let source_path = temp_root.join("imported_nested_type_alias_runtime.arden");
    let output_path = temp_root.join("imported_nested_type_alias_runtime");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                    function get(): Integer { return this.value; }
                }
            }
            import M.Box as B;
            function main(): Integer {
                return B(2).get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported nested type alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported nested type alias binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_generic_top_level_type_alias_runtime() {
    let temp_root = make_temp_project_root("imported-generic-top-level-type-alias-runtime");
    let source_path = temp_root.join("imported_generic_top_level_type_alias_runtime.arden");
    let output_path = temp_root.join("imported_generic_top_level_type_alias_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }
            import Box as B;
            function main(): Integer {
                return B<Integer>(2).get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported generic top-level type alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported generic top-level type alias binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_generic_function_alias_returning_generic_class_runtime() {
    let temp_root =
        make_temp_project_root("imported-generic-function-alias-returning-generic-class-runtime");
    let source_path =
        temp_root.join("imported_generic_function_alias_returning_generic_class_runtime.arden");
    let output_path =
        temp_root.join("imported_generic_function_alias_returning_generic_class_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                    function get(): T { return this.value; }
                }
                function mk<T>(value: T): Box<T> { return Box<T>(value); }
            }
            import M.mk as mk;
            function main(): Integer {
                return mk<Integer>(2).get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported generic function alias returning generic class should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported generic function alias returning generic class binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_generic_function_alias_runtime() {
    let temp_root = make_temp_project_root("imported-generic-function-alias-runtime");
    let source_path = temp_root.join("imported_generic_function_alias_runtime.arden");
    let output_path = temp_root.join("imported_generic_function_alias_runtime");
    let source = r#"
            module M {
                function id<T>(value: T): T { return value; }
            }
            import M.id as id;
            function main(): Integer {
                return id<Integer>(2);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported generic function alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported generic function alias binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_exact_imported_module_named_main_runtime() {
    let temp_root = make_temp_project_root("exact-imported-module-main-runtime");
    let source_path = temp_root.join("exact_imported_module_main_runtime.arden");
    let output_path = temp_root.join("exact_imported_module_main_runtime");
    let source = r#"
            module M {
                module main {
                    function ping(): Integer { return 22; }
                }
            }

            import M.main as Main;

            function main(): Integer {
                return Main.ping();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("exact-imported module named main should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled exact-imported module named main binary");
    assert_eq!(status.code(), Some(22));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_wildcard_imported_module_named_main_runtime() {
    let temp_root = make_temp_project_root("wildcard-imported-module-main-runtime");
    let source_path = temp_root.join("wildcard_imported_module_main_runtime.arden");
    let output_path = temp_root.join("wildcard_imported_module_main_runtime");
    let source = r#"
            module M {
                module main {
                    function ping(): Integer { return 22; }
                }
            }

            import M.*;

            function main(): Integer {
                return main.ping();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("wildcard-imported module named main should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled wildcard-imported module named main binary");
    assert_eq!(status.code(), Some(22));

    let _ = fs::remove_dir_all(temp_root);
}
