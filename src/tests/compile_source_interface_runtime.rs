use super::*;
use std::fs;

#[test]
fn compile_source_runs_unique_interface_method_dispatch_runtime() {
    let temp_root = make_temp_project_root("interface-method-dispatch-runtime");
    let source_path = temp_root.join("interface_method_dispatch_runtime.arden");
    let output_path = temp_root.join("interface_method_dispatch_runtime");
    let source = r#"
            interface Named {
                function get(): String;
            }

            class Boxed implements Named {
                value: String;
                constructor(value: String) { this.value = value; }
                function get(): String { return this.value; }
            }

            function main(): Integer {
                n: Named = Boxed("abc");
                return if (n.get().length() == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("single-implementation interface method dispatch should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled interface method dispatch binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_module_local_import_alias_leaking_to_top_level() {
    let temp_root = make_temp_project_root("module-local-import-alias-leak-top-level");
    let source_path = temp_root.join("module_local_import_alias_leak_top_level.arden");
    let output_path = temp_root.join("module_local_import_alias_leak_top_level");
    let source = r#"
            import std.io.*;

            module Inner {
                import std.math as math;

                function keep(): None {
                    println(to_string(math.abs(-1.0)));
                    return None;
                }
            }

            function main(): None {
                value: Float = math.abs(-1.0);
                println(to_string(value));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("module-local alias should not resolve at top level");
    assert!(
        err.contains("Unknown type: math")
            || err.contains("Undefined variable: math")
            || err.contains("Unknown namespace alias usage 'math.abs'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_unique_interface_bound_method_value_runtime() {
    let temp_root = make_temp_project_root("interface-bound-method-value-runtime");
    let source_path = temp_root.join("interface_bound_method_value_runtime.arden");
    let output_path = temp_root.join("interface_bound_method_value_runtime");
    let source = r#"
            interface Named {
                function get(): String;
            }

            class Boxed implements Named {
                value: String;
                constructor(value: String) { this.value = value; }
                function get(): String { return this.value; }
            }

            function main(): Integer {
                n: Named = Boxed("abc");
                f: () -> String = n.get;
                return if (f().length() == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("single-implementation interface bound method value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled interface bound method value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_interface_method_wrong_arity_before_runtime() {
    let temp_root = make_temp_project_root("no-check-interface-method-wrong-arity");
    let source_path = temp_root.join("no_check_interface_method_wrong_arity.arden");
    let output_path = temp_root.join("no_check_interface_method_wrong_arity");
    let source = r#"
            interface Reader {
                function read(): Integer;
            }

            class Box implements Reader {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function read(): Integer { return this.value; }
            }

            function main(): Integer {
                reader: Reader = Box(7);
                return reader.read(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("interface method wrong arity should fail in codegen");
    assert!(
        err.contains("Reader.read() expects 0 argument(s), got 1"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_missing_interface_method_before_runtime() {
    let temp_root = make_temp_project_root("no-check-missing-interface-method");
    let source_path = temp_root.join("no_check_missing_interface_method.arden");
    let output_path = temp_root.join("no_check_missing_interface_method");
    let source = r#"
            interface Reader {
                function read(): Integer;
            }

            class Box implements Reader {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function read(): Integer { return this.value; }
            }

            class Other {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function missing(): Integer { return this.value; }
            }

            function main(): Integer {
                reader: Reader = Box(7);
                return reader.missing();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("missing interface method should fail in codegen");
    assert!(
        err.contains("Unknown method 'missing' for interface 'Reader'"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_missing_interface_bound_method_before_runtime() {
    let temp_root = make_temp_project_root("no-check-missing-interface-bound-method");
    let source_path = temp_root.join("no_check_missing_interface_bound_method.arden");
    let output_path = temp_root.join("no_check_missing_interface_bound_method");
    let source = r#"
            interface Reader {
                function read(): Integer;
            }

            class Box implements Reader {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function read(): Integer { return this.value; }
            }

            class Other {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function missing(): Integer { return this.value; }
            }

            function main(): Integer {
                reader: Reader = Box(7);
                f: () -> Integer = reader.missing;
                return f();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("missing interface bound method should fail in codegen");
    assert!(
        err.contains("Unknown method 'missing' for interface 'Reader'"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_interface_dispatch_to_non_implementor_before_runtime() {
    let temp_root = make_temp_project_root("no-check-interface-non-implementor-dispatch");
    let source_path = temp_root.join("no_check_interface_non_implementor_dispatch.arden");
    let output_path = temp_root.join("no_check_interface_non_implementor_dispatch");
    let source = r#"
            interface Reader {
                function read(): Integer;
            }

            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            class Other {
                function read(): Integer { return 9; }
            }

            function main(): Integer {
                reader: Reader = Box(7);
                return reader.read();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-implementor interface dispatch should fail in codegen");
    assert!(
        err.contains("Unknown interface method implementation: read"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_interface_bound_method_from_non_implementor_before_runtime() {
    let temp_root = make_temp_project_root("no-check-interface-non-implementor-bound-method");
    let source_path = temp_root.join("no_check_interface_non_implementor_bound_method.arden");
    let output_path = temp_root.join("no_check_interface_non_implementor_bound_method");
    let source = r#"
            interface Reader {
                function read(): Integer;
            }

            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            class Other {
                function read(): Integer { return 9; }
            }

            function main(): Integer {
                reader: Reader = Box(7);
                f: () -> Integer = reader.read;
                return f();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("non-implementor interface bound method should fail in codegen");
    assert!(
        err.contains("Unknown interface method implementation: read"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_interface_bound_method_function_value_signature_mismatch() {
    let temp_root = make_temp_project_root("no-check-interface-bound-method-signature-mismatch");
    let source_path = temp_root.join("no_check_interface_bound_method_signature_mismatch.arden");
    let output_path = temp_root.join("no_check_interface_bound_method_signature_mismatch");
    let source = r#"
            interface Named {
                function get(): Integer;
            }

            class Boxed implements Named {
                constructor() {}
                function get(): Integer { return 1; }
            }

            function main(): Integer {
                value: Named = Boxed();
                f: (Integer) -> Integer = value.get;
                return f(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("interface bound method signature mismatch should fail in codegen");
    assert!(
        err.contains("Cannot use function value () -> Integer as (Integer) -> Integer"),
        "{err}"
    );
    assert!(!err.contains("process exited with code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_bound_constructor_method_dispatch_runtime() {
    let temp_root = make_temp_project_root("generic-bound-constructor-dispatch-runtime");
    let source_path = temp_root.join("generic_bound_constructor_dispatch_runtime.arden");
    let output_path = temp_root.join("generic_bound_constructor_dispatch_runtime");
    let source = r#"
            interface Named {
                function name(): Integer;
            }

            class Person implements Named {
                constructor() {}
                function name(): Integer { return 1; }
            }

            class Holder<T extends Named> {
                value: T;

                constructor(value: T) {
                    require(value.name() == 1);
                    this.value = value;
                }
            }

            function main(): Integer {
                Holder<Person>(Person());
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("generic bound constructor dispatch should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run generic bound constructor dispatch binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_multi_bound_interface_method_dispatch_runtime() {
    let temp_root = make_temp_project_root("multi-bound-interface-method-dispatch-runtime");
    let source_path = temp_root.join("multi_bound_interface_method_dispatch_runtime.arden");
    let output_path = temp_root.join("multi_bound_interface_method_dispatch_runtime");
    let source = r#"
            interface A { function a(): Integer; }
            interface B { function b(): Integer; }

            class C implements A, B {
                constructor() {}
                function a(): Integer { return 1; }
                function b(): Integer { return 2; }
            }

            function read_b<T extends A, B>(value: T): Integer {
                return value.b();
            }

            function main(): Integer {
                return if (read_b(C()) == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("multi-bound interface method dispatch should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run multi-bound interface method dispatch binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_multi_bound_interface_bound_method_runtime() {
    let temp_root = make_temp_project_root("multi-bound-interface-bound-method-runtime");
    let source_path = temp_root.join("multi_bound_interface_bound_method_runtime.arden");
    let output_path = temp_root.join("multi_bound_interface_bound_method_runtime");
    let source = r#"
            interface A { function a(): Integer; }
            interface B { function b(): Integer; }

            class C implements A, B {
                constructor() {}
                function a(): Integer { return 1; }
                function b(): Integer { return 2; }
            }

            function read_b<T extends A, B>(value: T): Integer {
                f: () -> Integer = value.b;
                return f();
            }

            function main(): Integer {
                return if (read_b(C()) == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("multi-bound interface bound method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run multi-bound interface bound method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_interface_implements_runtime() {
    let temp_root = make_temp_project_root("generic-interface-implements-runtime");
    let source_path = temp_root.join("generic_interface_implements_runtime.arden");
    let output_path = temp_root.join("generic_interface_implements_runtime");
    let source = r#"
            interface I<T> {
                function get(): T;
            }

            class C implements I<String> {
                function get(): String { return "ok"; }
            }

            function main(): Integer {
                i: I<String> = C();
                return if (i.get().length() == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("generic interface implements clause should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled generic interface implements binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_specialized_parent_interface_method_runtime() {
    let temp_root = make_temp_project_root("specialized-parent-interface-runtime");
    let source_path = temp_root.join("specialized_parent_interface_runtime.arden");
    let output_path = temp_root.join("specialized_parent_interface_runtime");
    let source = r#"
            interface Reader<T> {
                function read(): T;
            }

            interface StringReader extends Reader<String> {}

            class FileReader implements StringReader {
                function read(): String { return "ok"; }
            }

            function main(): Integer {
                reader: StringReader = FileReader();
                f: () -> String = reader.read;
                return if (reader.read().length() == 2 && f().length() == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("specialized parent interface methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled specialized parent interface binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
