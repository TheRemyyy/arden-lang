use super::*;
use std::fs;

#[test]
fn project_build_supports_exact_imported_nested_function_aliases_returning_classes() {
    let temp_root = make_temp_project_root("exact-nested-function-alias-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule M {\n    class Box {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        function get(): Integer { return this.value; }\n    }\n    function mk(value: Integer): Box { return Box(value); }\n}\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.M.mk as mk;\nfunction main(): None { value: Integer = mk(2).get(); require(value == 2); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support exact imported nested function aliases returning classes",
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_nested_class_aliases() {
    let temp_root = make_temp_project_root("exact-nested-class-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule M {\n    class Box {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        function get(): Integer { return this.value; }\n    }\n}\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.M.Box as Boxed;\nfunction main(): None { value: Integer = Boxed(2).get(); require(value == 2); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support exact imported nested class aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_local_qualified_nested_class_paths() {
    let temp_root = make_temp_project_root("local-qualified-nested-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule M {\n    class Box {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        function get(): Integer { return this.value; }\n    }\n}\nfunction main(): None { b: M.Box = M.Box(2); require(b.get() == 2); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support local qualified nested class paths");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_local_qualified_nested_generic_class_paths() {
    let temp_root = make_temp_project_root("local-qualified-nested-generic-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n}\nfunction main(): None { b: M.Box<Integer> = M.Box<Integer>(2); require(b.get() == 2); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support local qualified nested generic class paths");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_nested_generic_class_aliases() {
    let temp_root = make_temp_project_root("exact-nested-generic-class-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n}\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.M.Box as Boxed;\nfunction main(): None { b: Boxed<Integer> = Boxed<Integer>(2); require(b.get() == 2); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support exact imported nested generic class aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_nested_generic_function_aliases_returning_classes() {
    let temp_root = make_temp_project_root("exact-nested-generic-function-alias-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n    function mk<T>(value: T): Box<T> { return Box<T>(value); }\n}\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.M.mk as mk;\nfunction main(): None { value: Integer = mk<Integer>(2).get(); require(value == 2); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support exact imported nested generic function aliases returning classes",
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_local_nested_generic_functions_returning_classes() {
    let temp_root = make_temp_project_root("local-nested-generic-function-runtime-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n    function mk<T>(value: T): Box<T> { return Box<T>(value); }\n}\nfunction main(): Integer { return M.mk<Integer>(2).get(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support local nested generic function returns");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled local nested generic function binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_exact_imported_nested_generic_function_aliases_returning_classes() {
    let temp_root =
        make_temp_project_root("exact-nested-generic-function-alias-class-runtime-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n    function mk<T>(value: T): Box<T> { return Box<T>(value); }\n}\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.M.mk as mk;\nfunction main(): Integer { return mk<Integer>(2).get(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support exact imported nested generic function aliases at runtime",
        );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled imported nested generic function binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_specialized_parent_interface_methods() {
    let temp_root = make_temp_project_root("specialized-parent-interface-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package app;\ninterface Reader<T> { function read(): T; }\ninterface StringReader extends Reader<String> {}\nclass FileReader implements StringReader { function read(): String { return \"ok\"; } }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.StringReader;\nimport app.FileReader;\nfunction main(): Integer { reader: StringReader = FileReader(); f: () -> String = reader.read; return if (reader.read().length() == 2 && f().length() == 2) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support specialized parent interface methods");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled specialized parent interface project binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_generic_namespace_alias_parent_interfaces() {
    let temp_root = make_temp_project_root("generic-alias-parent-interface-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.Api as api;\ninterface StringReader extends api.Reader<String> {}\nclass FileReader implements StringReader { function read(): String { return \"ok\"; } }\nfunction main(): Integer { reader: StringReader = FileReader(); return if (reader.read().length() == 2) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support generic namespace-alias parent interfaces");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled generic namespace-alias parent interface binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_generic_exact_import_alias_parent_interfaces() {
    let temp_root = make_temp_project_root("generic-exact-alias-parent-interface-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.Api.Reader as ReaderAlias;\ninterface StringReader extends ReaderAlias<String> {}\nclass FileReader implements StringReader { function read(): String { return \"ok\"; } }\nfunction main(): None { reader: StringReader = FileReader(); require(reader.read().length() == 2); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support generic exact-import alias parent interfaces");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_generic_namespace_alias_interface_annotations() {
    let temp_root = make_temp_project_root("generic-alias-interface-annotation-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.Api as api;\nclass FileReader implements api.Reader<String> { function read(): String { return \"ok\"; } }\nfunction main(): Integer { reader: api.Reader<String> = FileReader(); return if (reader.read().length() == 2) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support generic namespace-alias interface annotations");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled generic namespace-alias interface annotation binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_generic_exact_import_alias_interface_annotations() {
    let temp_root = make_temp_project_root("generic-exact-alias-interface-annotation-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.Api.Reader as ReaderAlias;\nclass FileReader implements ReaderAlias<String> { function read(): String { return \"ok\"; } }\nfunction main(): Integer { reader: ReaderAlias<String> = FileReader(); return if (reader.read().length() == 2) { 0 } else { 1 }; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support generic exact-import alias interface annotations");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled generic exact-import alias interface annotation binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_generic_namespace_alias_interface_parameters() {
    let temp_root = make_temp_project_root("generic-alias-interface-parameter-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.Api as api;\nclass FileReader implements api.Reader<String> { function read(): String { return \"ok\"; } }\nfunction use_reader(reader: api.Reader<String>): Integer { return reader.read().length(); }\nfunction main(): Integer { return use_reader(FileReader()); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support generic namespace-alias interface parameters");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled generic namespace-alias interface parameter binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_generic_exact_import_alias_interface_returns() {
    let temp_root = make_temp_project_root("generic-exact-alias-interface-return-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.Api.Reader as ReaderAlias;\nclass FileReader implements ReaderAlias<String> { function read(): String { return \"ok\"; } }\nfunction make_reader(): ReaderAlias<String> { return FileReader(); }\nfunction main(): Integer { return make_reader().read().length(); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support generic exact-import alias interface returns");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled generic exact-import alias interface return binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_module_local_generic_namespace_alias_interface_parameters() {
    let temp_root = make_temp_project_root("module-generic-alias-interface-parameter-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.Api as api;\nclass FileReader implements api.Reader<String> { function read(): String { return \"ok\"; } }\nmodule Helpers {\n    function use_reader(reader: api.Reader<String>): Integer { return reader.read().length(); }\n}\nfunction main(): Integer { return Helpers.use_reader(FileReader()); }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support module-local generic namespace-alias interface parameters",
        );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled module-local generic namespace-alias interface parameter binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_generic_namespace_alias_interface_lambda_parameters() {
    let temp_root = make_temp_project_root("lambda-generic-alias-interface-parameter-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("util.arden"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .must("write util");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport app.Api as api;\nclass FileReader implements api.Reader<String> { function read(): String { return \"ok\"; } }\nfunction main(): Integer {\n    use_reader: (api.Reader<String>) -> Integer = (reader: api.Reader<String>) => reader.read().length();\n    return use_reader(FileReader());\n}\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support generic namespace-alias interface lambda parameters",
        );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled generic namespace-alias interface lambda parameter binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}
