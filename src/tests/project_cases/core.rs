use super::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn project_parse_cache_reuses_only_unchanged_files() {
    let temp_root = make_temp_project_root("parse-cache-selective");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.arden");
    let lib_file = src_dir.join("lib.arden");

    fs::write(
            &main_file,
            "package app;\nimport lib.math;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .must("write main file");
    fs::write(
        &lib_file,
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write lib file");

    let first_main = parse_project_unit(&temp_root, &main_file).must("first main parse");
    let first_lib = parse_project_unit(&temp_root, &lib_file).must("first lib parse");
    assert!(!first_main.from_parse_cache);
    assert!(!first_lib.from_parse_cache);

    thread::sleep(Duration::from_millis(5));
    fs::write(
        &lib_file,
        "package lib;\nfunction add(x: Integer): Integer { return x + 2; }\n",
    )
    .must("rewrite lib file");

    let second_main = parse_project_unit(&temp_root, &main_file).must("second main parse");
    let second_lib = parse_project_unit(&temp_root, &lib_file).must("second lib parse");

    assert!(second_main.from_parse_cache);
    assert!(!second_lib.from_parse_cache);
    assert_eq!(
        first_main.semantic_fingerprint,
        second_main.semantic_fingerprint
    );
    assert_ne!(
        first_lib.semantic_fingerprint,
        second_lib.semantic_fingerprint
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_multi_file_import_graph_tracks_real_parsed_owner_file() {
    let temp_root = make_temp_project_root("import-graph");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.arden");
    let math_file = src_dir.join("math.arden");

    fs::write(
            &main_file,
            "package app;\nimport lib.math;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .must("write main file");
    fs::write(
        &math_file,
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write math file");

    let parsed_files = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main"),
        parse_project_unit(&temp_root, &math_file).must("parse math"),
    ];

    let mut namespace_files_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut namespace_function_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let mut namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let mut namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let mut global_function_map: HashMap<String, String> = HashMap::new();
    let mut global_function_file_map: HashMap<String, PathBuf> = HashMap::new();
    let mut global_class_map: HashMap<String, String> = HashMap::new();
    let mut global_class_file_map: HashMap<String, PathBuf> = HashMap::new();
    let mut global_enum_map: HashMap<String, String> = HashMap::new();
    let mut global_enum_file_map: HashMap<String, PathBuf> = HashMap::new();
    let mut global_module_map: HashMap<String, String> = HashMap::new();
    let mut global_module_file_map: HashMap<String, PathBuf> = HashMap::new();

    for unit in &parsed_files {
        namespace_files_map
            .entry(unit.namespace.clone())
            .or_default()
            .push(unit.file.clone());
        for name in &unit.function_names {
            namespace_function_files
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_function_map.insert(name.clone(), unit.namespace.clone());
            global_function_file_map.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.class_names {
            namespace_class_files
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_class_map.insert(name.clone(), unit.namespace.clone());
            global_class_file_map.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.enum_names {
            global_enum_map.insert(name.clone(), unit.namespace.clone());
            global_enum_file_map.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.module_names {
            namespace_module_files
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_module_map.insert(name.clone(), unit.namespace.clone());
            global_module_file_map.insert(name.clone(), unit.file.clone());
        }
    }

    let symbol_lookup = Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
        function_map: &global_function_map,
        function_file_map: &global_function_file_map,
        class_map: &global_class_map,
        class_file_map: &global_class_file_map,
        interface_map: empty_global_interface_map(),
        interface_file_map: empty_global_interface_file_map(),
        enum_map: &global_enum_map,
        enum_file_map: &global_enum_file_map,
        module_map: &global_module_map,
        module_file_map: &global_module_file_map,
    }));
    let ctx = DependencyResolutionContext {
        namespace_files_map: &namespace_files_map,
        global_function_map: &global_function_map,
        global_function_file_map: &global_function_file_map,
        global_class_map: &global_class_map,
        global_class_file_map: &global_class_file_map,
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &global_enum_map,
        global_enum_file_map: &global_enum_file_map,
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
        symbol_lookup: Arc::clone(&symbol_lookup),
    };

    let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None, None);
    assert_eq!(
        graph.get(&main_file).cloned().unwrap_or_default(),
        HashSet::from([math_file.clone()])
    );
    assert!(graph
        .get(&math_file)
        .cloned()
        .unwrap_or_default()
        .is_empty());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_multi_file_dependency_graph_tracks_same_namespace_enum_reference_owner_file() {
    let temp_root = make_temp_project_root("enum-main-import-graph");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.arden");
    let enum_file = src_dir.join("enum.arden");

    fs::write(
            &main_file,
            "package core;\nfunction main(): Integer { return match (main.Ok(22)) { Ok(value) => value, }; }\n",
        )
        .must("write main file");
    fs::write(&enum_file, "package core;\nenum main { Ok(Integer) }\n").must("write enum file");

    let parsed_files = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main"),
        parse_project_unit(&temp_root, &enum_file).must("parse enum"),
    ];

    let mut namespace_files_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut namespace_function_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let mut namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let mut namespace_interface_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let mut namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let mut global_function_map: HashMap<String, String> = HashMap::new();
    let mut global_function_file_map: HashMap<String, PathBuf> = HashMap::new();
    let mut global_class_map: HashMap<String, String> = HashMap::new();
    let mut global_class_file_map: HashMap<String, PathBuf> = HashMap::new();
    let mut global_interface_map: HashMap<String, String> = HashMap::new();
    let mut global_interface_file_map: HashMap<String, PathBuf> = HashMap::new();
    let mut global_enum_map: HashMap<String, String> = HashMap::new();
    let mut global_enum_file_map: HashMap<String, PathBuf> = HashMap::new();
    let mut global_module_map: HashMap<String, String> = HashMap::new();
    let mut global_module_file_map: HashMap<String, PathBuf> = HashMap::new();

    for unit in &parsed_files {
        namespace_files_map
            .entry(unit.namespace.clone())
            .or_default()
            .push(unit.file.clone());
        for name in &unit.function_names {
            namespace_function_files
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_function_map.insert(name.clone(), unit.namespace.clone());
            global_function_file_map.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.class_names {
            namespace_class_files
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_class_map.insert(name.clone(), unit.namespace.clone());
            global_class_file_map.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.interface_names {
            namespace_interface_files
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_interface_map.insert(name.clone(), unit.namespace.clone());
            global_interface_file_map.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.enum_names {
            global_enum_map.insert(name.clone(), unit.namespace.clone());
            global_enum_file_map.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.module_names {
            namespace_module_files
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_module_map.insert(name.clone(), unit.namespace.clone());
            global_module_file_map.insert(name.clone(), unit.file.clone());
        }
    }

    let symbol_lookup = Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
        function_map: &global_function_map,
        function_file_map: &global_function_file_map,
        class_map: &global_class_map,
        class_file_map: &global_class_file_map,
        interface_map: &global_interface_map,
        interface_file_map: &global_interface_file_map,
        enum_map: &global_enum_map,
        enum_file_map: &global_enum_file_map,
        module_map: &global_module_map,
        module_file_map: &global_module_file_map,
    }));
    let ctx = DependencyResolutionContext {
        namespace_files_map: &namespace_files_map,
        global_function_map: &global_function_map,
        global_function_file_map: &global_function_file_map,
        global_class_map: &global_class_map,
        global_class_file_map: &global_class_file_map,
        global_interface_map: &global_interface_map,
        global_interface_file_map: &global_interface_file_map,
        global_enum_map: &global_enum_map,
        global_enum_file_map: &global_enum_file_map,
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
        symbol_lookup,
    };

    let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None, None);
    assert_eq!(
        graph.get(&main_file).cloned().unwrap_or_default(),
        HashSet::from([enum_file.clone()])
    );
    assert!(graph
        .get(&enum_file)
        .cloned()
        .unwrap_or_default()
        .is_empty());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_nested_generic_methods_on_nested_generic_classes() {
    let temp_root = make_temp_project_root("nested-generic-method-runtime-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n        function get(): T { return this.value; }\n    }\n}\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): Integer { b: M.Box<Integer> = M.Box<Integer>(2); return b.map<Integer>(inc).get(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support nested generic methods on nested generic classes");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled nested generic method binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_nested_generic_method_alias_paths() {
    let temp_root = make_temp_project_root("nested-generic-method-alias-runtime-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("util.arden"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n        function get(): T { return this.value; }\n    }\n}\nfunction inc(x: Integer): Integer { return x + 1; }\n",
        )
        .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app.M.Box as Boxed;\nimport app.inc as inc;\nfunction main(): Integer { b: Boxed<Integer> = Boxed<Integer>(2); return b.map<Integer>(inc).get(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support nested generic method alias paths");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled nested generic alias method binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_nested_generic_class_specializations() {
    let temp_root = make_temp_project_root("namespace-alias-nested-generic-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("util.arden"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function get(): T { return this.value; }\n        }\n        function mk(value: Integer): Box<Integer> { return Box<Integer>(value); }\n        async function mk_async(value: Integer): Task<Box<Integer>> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util as u;\nimport util.M.N.Box as B;\nfunction main(): Integer { return u.M.N.Box<Integer>(41).value + B<Integer>(1).get(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support namespace alias nested generic class specializations",
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_nested_generic_method_specializations() {
    let temp_root = make_temp_project_root("namespace-alias-nested-generic-method-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("util.arden"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n            function get(): T { return this.value; }\n        }\n        function mk(value: Integer): Box<Integer> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util as u;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): Integer { return u.M.N.mk(46).map<Integer>(inc).get(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support namespace alias nested generic method specializations",
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_emits_nested_generic_specialization_symbols_in_one_object_file() {
    let temp_root = make_temp_project_root("namespace-alias-nested-generic-object-ownership");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("util.arden"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n            function get(): T { return this.value; }\n        }\n        function mk(value: Integer): Box<Integer> { return Box<Integer>(value); }\n        async function mk_async(value: Integer): Task<Box<Integer>> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util as u;\nimport util.M.N.Box as B;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): Integer { return u.M.N.mk(46).map<Integer>(inc).get() + u.M.N.Box<Integer>(41).value + B<Integer>(1).get() + await(u.M.N.mk_async(43)).get(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should emit nested generic specialization bodies in a single object",
        );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled mixed nested generic specialization binary");
    assert_eq!(status.code(), Some(132));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_cross_package_nested_generic_function_returns_via_namespace_alias() {
    let temp_root =
        make_temp_project_root("cross-package-nested-generic-return-namespace-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("util.arden"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function get(): T { return this.value; }\n        }\n        function mk(value: Integer): Box<Integer> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util as u;\nfunction main(): Integer { return u.M.N.mk(42).get(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support cross-package nested generic returns via namespace alias",
        );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run cross-package nested generic return project binary");
    assert_eq!(status.code(), Some(42));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_cross_package_nested_generic_async_returns_via_namespace_alias() {
    let temp_root =
        make_temp_project_root("cross-package-nested-generic-async-return-namespace-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("util.arden"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function get(): T { return this.value; }\n        }\n        async function mk_async(value: Integer): Task<Box<Integer>> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util as u;\nfunction main(): Integer { return await(u.M.N.mk_async(43)).get(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
                "project build should support cross-package nested generic async returns via namespace alias",
            );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run cross-package nested generic async return project binary");
    assert_eq!(status.code(), Some(43));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_qualified_module_type_paths() {
    let temp_root = make_temp_project_root("qualified-module-type-path-runtime-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule util {\n    class Item {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        function get(): Integer { return this.value; }\n    }\n    function mk(): Item { return Item(7); }\n}\nfunction main(): Integer {\n    item: util.Item = util.mk();\n    return item.get();\n}\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support qualified module type paths end-to-end");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled qualified module type path binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_user_defined_generic_classes_named_like_builtins() {
    let temp_root = make_temp_project_root("user-defined-generic-class-named-like-builtin-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nclass Box<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n    function get(): T { return this.value; }\n}\nfunction mk(value: Integer): Box<Integer> {\n    return Box<Integer>(value);\n}\nfunction main(): Integer {\n    return mk(42).get();\n}\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
                "project build should prefer user-defined generic classes over built-in container names",
            );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled user-defined builtin-named generic class binary");
    assert_eq!(status.code(), Some(42));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_nested_generic_methods_on_expression_receivers() {
    let temp_root = make_temp_project_root("nested-generic-method-expr-runtime-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n        function get(): T { return this.value; }\n    }\n    function make<T>(value: T): Box<T> { return Box<T>(value); }\n}\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): Integer { return M.make<Integer>(2).map<Integer>(inc).get(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support nested generic methods on expression receivers");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled nested generic expression receiver binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_builtin_function_values_in_user_defined_builtin_named_generic_methods() {
    let temp_root =
        make_temp_project_root("builtin-fn-user-defined-builtin-named-generic-method-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nclass Box<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n    function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n    function get(): T { return this.value; }\n}\nfunction main(): Integer { mapped: Box<Float> = Box<Integer>(1).map<Float>(to_float); return if (mapped.get() == 1.0) { 0 } else { 1 }; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
                "project build should support builtin function values in user-defined builtin-named generic methods",
            );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled builtin function value generic method project binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_nested_generic_method_imported_expression_receivers() {
    let temp_root = make_temp_project_root("nested-generic-method-imported-expr-runtime-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/util.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("util.arden"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n        function get(): T { return this.value; }\n    }\n    function make<T>(value: T): Box<T> { return Box<T>(value); }\n}\nfunction inc(x: Integer): Integer { return x + 1; }\n",
        )
        .must("write util");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport app.M.make as make;\nimport app.inc as inc;\nfunction main(): Integer { return make<Integer>(2).map<Integer>(inc).get(); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must(
            "project build should support imported expression receivers for nested generic methods",
        );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled imported nested generic expression receiver binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_async_block_import_alias_calls() {
    let temp_root = make_temp_project_root("async-block-import-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util.add1 as inc;\nfunction main(): None { task: Task<Integer> = async { return inc(1); }; value: Integer = await(task); require(value == 2); return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support async-block import alias calls");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_async_block_namespace_alias_unit_enum_tail_runtime() {
    let temp_root = make_temp_project_root("async-block-ns-alias-unit-enum-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nenum E { A, B }\n",
    )
    .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util as u;\nfunction main(): Integer { task: Task<u.E> = async { u.E.A }; value: u.E = await(task); match (value) { u.E.A => { return 0; } u.E.B => { return 1; } } }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support async-block namespace-alias unit-enum tails");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled async-block namespace-alias unit-enum tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_async_block_import_alias_function_value_tail_runtime() {
    let temp_root = make_temp_project_root("async-block-import-alias-function-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util.add1 as inc;\nfunction main(): Integer { task: Task<(Integer) -> Integer> = async { inc }; f: (Integer) -> Integer = await(task); return f(1); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support async-block import-alias function-value tails");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled async-block import-alias function-value tail binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_async_block_import_alias_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-import-alias-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util.add1 as inc;\nfunction main(): Integer { task: Task<Integer> = async { inc(1) }; return await(task); }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support async-block import-alias tail expressions");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled async-block import-alias tail-expression binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_unit_enum_values() {
    let temp_root = make_temp_project_root("namespace-alias-unit-enum-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nenum E { A, B }\n",
    )
    .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util as u;\nfunction main(): None { e: u.E = u.E.A; match (e) { u.E.A => { } u.E.B => { } } return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support namespace alias unit enum values");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_try_expression_function_value_callees() {
    let temp_root = make_temp_project_root("try-function-callee-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction choose(): Result<(Integer) -> Integer, String> { return Result.ok(inc); }\nfunction compute(): Result<Integer, String> { value: Integer = (choose()?)(1); return Result.ok(value); }\nfunction main(): Integer { value: Integer = compute().unwrap(); require(value == 2); return 0; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support try-expression function-value callees");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_imported_explicit_generic_free_calls() {
    let temp_root = make_temp_project_root("imported-explicit-generic-free-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nfunction id<T>(x: T): T { return x; }\n",
    )
    .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util.id;\nfunction main(): None { value: Integer = id<Integer>(1); require(value == 1); return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support imported explicit generic free calls");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_imported_generic_class_instance_methods() {
    let temp_root = make_temp_project_root("imported-generic-class-method-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package util;\nclass Boxed<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n    function get(): T { return this.value; }\n}\n",
        )
        .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util.Boxed;\nfunction main(): None { value: Integer = Boxed<Integer>(7).get(); require(value == 7); return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support imported generic class instance methods");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_top_level_destructor_alias_rewrite() {
    let temp_root = make_temp_project_root("destructor-alias-rewrite-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util.add1 as inc;\nclass Boxed {\n    value: Integer;\n    constructor(value: Integer) { this.value = value; }\n    destructor() { require(inc(this.value) == 2); }\n}\nfunction main(): Integer { box: Boxed = Boxed(1); return 0; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should rewrite top-level destructor alias calls");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled destructor alias rewrite binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_module_local_destructor_function_rewrite() {
    let temp_root = make_temp_project_root("module-destructor-rewrite-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M {\n    function score(x: Integer): Integer { return x + 1; }\n    class Boxed {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        destructor() { require(score(this.value) == 2); }\n    }\n    function make(): Boxed { return Boxed(1); }\n}\nfunction main(): Integer { box: M.Boxed = M.make(); return 0; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should rewrite module-local destructor calls");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .must("run compiled module destructor rewrite binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_method_calls_on_function_returned_objects() {
    let temp_root = make_temp_project_root("function-return-method-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nclass Boxed<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n    function get(): T { return this.value; }\n}\nfunction make_box(): Boxed<Integer> { return Boxed<Integer>(9); }\nfunction main(): None { value: Integer = make_box().get(); require(value == 9); return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support method calls on function-returned objects");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_check_supports_namespace_alias_nested_module_generic_class_constructors() {
    let temp_root = make_temp_project_root("namespace-alias-nested-generic-class-check");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package util;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n    }\n}\n",
        )
        .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport util as u;\nfunction main(): None { b: u.M.Box<Integer> = u.M.Box<Integer>(1); return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        check_command(None, false).must(
            "project check should support namespace alias nested-module generic class constructors",
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_interface_implements() {
    let temp_root = make_temp_project_root("module-local-interface-implements-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M {\n    interface Named { function name(): Integer; }\n    class Book implements Named {\n        constructor() {}\n        function name(): Integer { return 1; }\n    }\n}\nfunction main(): None { return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support module-local interface implements");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_nested_interface_implements() {
    let temp_root = make_temp_project_root("module-local-nested-interface-implements-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M {\n    module Api { interface Named { function name(): Integer; } }\n    class Book implements Api.Named {\n        constructor() {}\n        function name(): Integer { return 1; }\n    }\n}\nfunction main(): None { return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support module-local nested interface implements");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_interface_extends() {
    let temp_root = make_temp_project_root("module-local-interface-extends-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M {\n    interface Named { function name(): Integer; }\n    interface Printable extends Named { function print_me(): Integer; }\n    class Report implements Printable {\n        constructor() {}\n        function name(): Integer { return 1; }\n        function print_me(): Integer { return 2; }\n    }\n}\nfunction main(): None { return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support module-local interface extends");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_nested_interface_extends() {
    let temp_root = make_temp_project_root("module-local-nested-interface-extends-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nmodule M {\n    module Api { interface Named { function name(): Integer; } }\n    interface Printable extends Api.Named { function print_me(): Integer; }\n    class Report implements Printable {\n        constructor() {}\n        function name(): Integer { return 1; }\n        function print_me(): Integer { return 2; }\n    }\n}\nfunction main(): None { return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support module-local nested interface extends");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_interface_extends_on_seeded_semantic_path() {
    let temp_root = make_temp_project_root("seeded-alias-interface-extends-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package lib;\ninterface Named { function name(): Integer; }\n",
    )
    .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport lib as u;\ninterface Printable extends u.Named { function print_me(): Integer; }\nclass Report implements Printable {\n    constructor() {}\n    function name(): Integer { return 1; }\n    function print_me(): Integer { return 2; }\n}\nfunction main(): None { return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support aliased interface extends on seeded path");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_nested_namespace_alias_interface_extends_on_seeded_semantic_path() {
    let temp_root = make_temp_project_root("seeded-nested-alias-interface-extends-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\nmodule Api {\n    interface Named { function name(): Integer; }\n    interface Printable { function print_me(): Integer; }\n}\n",
        )
        .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport lib as u;\ninterface Reportable extends u.Api.Named, u.Api.Printable {}\nclass Report implements Reportable {\n    constructor() {}\n    function name(): Integer { return 1; }\n    function print_me(): Integer { return 2; }\n}\nfunction main(): None { return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support nested aliased interface extends on seeded path");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_generic_bounds() {
    let temp_root = make_temp_project_root("namespace-alias-generic-bounds-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.arden"),
            "package lib;\ninterface Named { function name(): Integer; }\nclass Person implements Named {\n    constructor() {}\n    function name(): Integer { return 1; }\n}\n",
        )
        .must("write lib");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nimport lib as u;\nfunction read_name<T extends u.Named>(value: T): Integer { return value.name(); }\nfunction main(): None { person: u.Person = u.Person(); require(read_name(person) == 1); return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support namespace alias generic bounds");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_no_check_rejects_namespace_alias_generic_bound_method_signature_mismatch() {
    let temp_root =
        make_temp_project_root("namespace-alias-generic-bound-method-signature-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package lib;\ninterface Named { function name(): Integer; }\nclass Person implements Named {\n    constructor() {}\n    function name(): Integer { return 1; }\n}\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        "package app;\nimport lib as u;\nfunction read_name<T extends u.Named>(value: T): Integer { f: (Integer) -> Integer = value.name; return f(1); }\nfunction main(): None { person: u.Person = u.Person(); require(read_name(person) == 1); return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, false, false, false).must_err(
            "unchecked project build should reject generic bound method signature mismatch",
        );
        assert!(
            err.contains("Cannot use function value () -> Integer as (Integer) -> Integer"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_dereferenced_function_value_callees() {
    let temp_root = make_temp_project_root("deref-function-callee-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): None { f: &(Integer) -> Integer = &inc; value: Integer = (*f)(1); require(value == 2); return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support dereferenced function-value callees");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_check_rejects_async_borrowed_reference_results() {
    let temp_root = make_temp_project_root("async-borrowed-result-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): None { task: Task<&(Integer) -> Integer> = async { return &inc; }; return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        let err = check_command(None, false)
            .must_err("project check should reject async borrowed reference results");
        assert!(
            err.contains("Async block cannot return a value containing borrowed references"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_check_rejects_async_borrowed_reference_params_and_captures() {
    let temp_root = make_temp_project_root("async-borrowed-param-capture-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
            src_dir.join("main.arden"),
            "package app;\nasync function read_ref(r: &Integer): Task<Integer> { return *r; }\nfunction main(): None { x: Integer = 1; alias: &Integer = &x; task: Task<Integer> = async { return *alias; }; return None; }\n",
        )
        .must("write main");

    with_current_dir(&temp_root, || {
        let err = check_command(None, false).must_err(
            "project check should reject async borrowed reference parameters and captures",
        );
        assert!(
                err.contains("Async function 'app__read_ref' cannot accept a parameter containing borrowed references"),
                "{err}"
            );
        assert!(
            err.contains(
                "Async block cannot capture 'alias' because its type contains borrowed references"
            ),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn safe_rewrite_cache_reuse_requires_matching_entry_namespace() {
    let previous = DependencyGraphCache {
        schema: DEPENDENCY_GRAPH_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        entry_namespace: "app".to_string(),
        files: Vec::new(),
    };

    assert!(can_reuse_safe_rewrite_cache(Some(&previous), "app"));
    assert!(!can_reuse_safe_rewrite_cache(Some(&previous), "core"));
    assert!(!can_reuse_safe_rewrite_cache(None, "app"));
}

#[test]
fn project_commands_recover_after_repeated_output_path_toggles() {
    let temp_root = make_temp_project_root("project-commands-repeated-output-toggles");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke\"\n",
        )
        .must("write arden.toml");
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must("initial build should pass");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke-a\"\n",
        )
        .must("rewrite output path a");
    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("build should pass after first output toggle");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke-b\"\n",
        )
        .must("rewrite output path b");
    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("build should pass after second output toggle");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_recover_after_repeated_output_and_version_toggles() {
    let temp_root = make_temp_project_root("project-commands-output-version-toggles");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke\"\n",
        )
        .must("write arden.toml");
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).must("initial build should pass");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.1\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke-a\"\n",
        )
        .must("rewrite output/version a");
    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("build should pass after first metadata toggle");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.2\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke-b\"\n",
        )
        .must("rewrite output/version b");
    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("build should pass after second metadata toggle");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_ignore_files_order_only_toggles() {
    let temp_root = make_temp_project_root("project-commands-files-order-toggles");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke\"\n",
        )
        .must("write arden.toml");
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).must("initial check should pass");
        build_project(false, false, true, false, false).must("initial build should pass");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/helper.arden\", \"src/main.arden\"]\noutput = \"smoke\"\n",
        )
        .must("rewrite file order");

    with_current_dir(&temp_root, || {
        check_command(None, false).must("check should ignore files-order-only toggle");
        build_project(false, false, true, false, false)
            .must("build should ignore files-order-only toggle");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_recovers_cleanly_after_invalid_files_list_fix() {
    let temp_root = make_temp_project_root("project-build-invalid-files-list-fix");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/helper.txt\", \"src/main.arden\"]\noutput = \"smoke\"\n",
        )
        .must("write invalid arden.toml");
    fs::write(
        temp_root.join("src/main.arden"),
        "function main(): None { return None; }\n",
    )
    .must("write main");
    fs::write(temp_root.join("src/helper.txt"), "not arden\n").must("write helper");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should reject invalid files list entry");
        assert!(
            err.contains("src/helper.txt") || err.contains("is not an .arden file"),
            "{err}"
        );
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\noutput = \"smoke\"\n",
        )
        .must("rewrite valid arden.toml");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("build should recover cleanly after fixing files list");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_parse_error_reports_full_relative_path_when_basenames_collide() {
    let temp_root = make_temp_project_root("project-parse-error-colliding-basenames");
    fs::create_dir_all(temp_root.join("src/app")).must("create app dir");
    fs::create_dir_all(temp_root.join("src/lib")).must("create lib dir");
    fs::write(
        temp_root.join("arden.toml"),
        "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/app/main.arden\"\nfiles = [\"src/app/main.arden\", \"src/lib/main.arden\"]\noutput = \"smoke\"\n",
    )
    .must("write arden.toml");
    fs::write(
        temp_root.join("src/app/main.arden"),
        "package app;\nfunction main(): None { return None; }\n",
    )
    .must("write app main");
    fs::write(
        temp_root.join("src/lib/main.arden"),
        "package lib;\nfunction broken(: Integer { return 1; }\n",
    )
    .must("write malformed lib main");

    with_current_dir(&temp_root, || {
        let check_err =
            check_command(None, false).must_err("project check should fail on malformed lib main");
        assert!(
            check_err.contains("src/lib/main.arden") || check_err.contains("src\\lib\\main.arden"),
            "expected full relative file path in parse error, got: {check_err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_recover_cleanly_after_malformed_helper_fix() {
    let temp_root = make_temp_project_root("project-commands-recover-malformed-helper");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke\"\n",
        )
        .must("write arden.toml");
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(: Integer { return 1; }\n",
    )
    .must("write malformed helper");

    with_current_dir(&temp_root, || {
        let check_err =
            check_command(None, false).must_err("project check should fail on malformed helper");
        assert!(
            check_err.contains("Parse error") || check_err.contains("Expected an identifier"),
            "{check_err}"
        );
        let build_err = build_project(false, false, true, false, false)
            .must_err("build should fail on malformed helper");
        assert!(
            build_err.contains("Parse error") || build_err.contains("Expected an identifier"),
            "{build_err}"
        );
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("rewrite valid helper");

    with_current_dir(&temp_root, || {
        show_project_info().must("info should recover after helper fix");
        check_command(None, false).must("project check should recover after helper fix");
        build_project(false, false, true, false, false)
            .must("build should recover after helper fix");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_recover_after_malformed_source_then_output_toggle() {
    let temp_root = make_temp_project_root("project-commands-malformed-then-output-toggle");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke\"\n",
        )
        .must("write arden.toml");
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(: Integer { return 1; }\n",
    )
    .must("write malformed helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).must_err("project check should fail on malformed helper");
        build_project(false, false, true, false, false)
            .must_err("build should fail on malformed helper");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("rewrite valid helper");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke-renamed\"\n",
        )
        .must("rewrite output path after recovery");

    with_current_dir(&temp_root, || {
        check_command(None, false)
            .must("project check should recover after malformed helper fix and output toggle");
        build_project(false, false, true, false, false)
            .must("build should recover after malformed helper fix and output toggle");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_recovers_after_malformed_helper_fix_with_cache_history() {
    let temp_root = make_temp_project_root("project-build-recover-malformed-helper");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"smoke\"\n",
        )
        .must("write arden.toml");
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(: Integer { return 1; }\n",
    )
    .must("write malformed helper");

    with_current_dir(&temp_root, || {
        let check_err =
            check_command(None, false).must_err("project check should fail on malformed helper");
        assert!(
            check_err.contains("Parse error") || check_err.contains("Expected an identifier"),
            "{check_err}"
        );
        let build_err = build_project(false, false, true, false, false)
            .must_err("build should fail on malformed helper");
        assert!(
            build_err.contains("Parse error") || build_err.contains("Expected an identifier"),
            "{build_err}"
        );
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("rewrite valid helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).must("project check should recover after helper fix");
        build_project(false, false, true, false, false)
            .must("build should recover after helper fix");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn new_project_rejects_names_that_would_generate_invalid_scaffolding() {
    let temp_root = make_temp_project_root("new-project-invalid-name-parent");
    let project_path = temp_root.join("target");

    let err = new_project("bad\"name", Some(&project_path))
        .must_err("invalid project name should be rejected");
    assert!(err.contains("Invalid project name"), "{err}");
    assert!(
        !project_path.exists(),
        "invalid project name should not create scaffold directories"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn new_project_reports_existing_file_paths_without_claiming_they_are_directories() {
    let temp_root = make_temp_project_root("new-project-existing-file");
    let project_path = temp_root.join("existing-file");
    fs::write(&project_path, "occupied\n").must("write existing file");

    let err = new_project("demo", Some(&project_path))
        .must_err("existing file path should block scaffold creation");
    assert!(err.contains("Path '"), "{err}");
    assert!(!err.contains("Directory '"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rejects_output_paths_outside_project_root() {
    let temp_root = make_temp_project_root("project-output-escape");
    let outside_dir = temp_root
        .parent()
        .must("temp dir should have parent")
        .join("arden-output-escape-target");
    let rel_outside = format!(
        "../{}/smoke",
        outside_dir
            .file_name()
            .and_then(|name| name.to_str())
            .must("outside dir name")
    );
    fs::write(
            temp_root.join("arden.toml"),
            format!(
                "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\noutput = \"{}\"\n",
                rel_outside
            ),
        )
        .must("write arden.toml");
    fs::write(
        temp_root.join("src/main.arden"),
        "function main(): None { return None; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, true, true, false, false)
            .must_err("build should reject output paths outside the project root");
        assert!(err.contains("outside the project root"), "{err}");
    });

    assert!(
        !outside_dir.exists(),
        "rejected output path should not create directories outside the project root"
    );

    let _ = fs::remove_dir_all(temp_root);
    let _ = fs::remove_dir_all(outside_dir);
}

#[test]
fn project_build_rejects_output_path_matching_source_file() {
    let temp_root = make_temp_project_root("project-output-source-collision");
    fs::write(
            temp_root.join("arden.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\", \"src/helper.arden\"]\noutput = \"src/helper.arden\"\n",
        )
        .must("write arden.toml");
    fs::write(
        temp_root.join("src/main.arden"),
        "package app;\nimport lib.helper;\nfunction main(): Integer { return helper(); }\n",
    )
    .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction helper(): Integer { return 1; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should reject output path matching a source file");
        assert!(err.contains("overwrite source file"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_creates_missing_nested_output_parent_directory() {
    let temp_root = make_temp_project_root("project-output-create-parent");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden"],
        "src/main.arden",
        "build/bin/smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        "function main(): Integer { return 0; }\n",
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should create missing nested output directories");
    });

    #[cfg(windows)]
    let built_output = temp_root.join("build/bin/smoke.exe");
    #[cfg(not(windows))]
    let built_output = temp_root.join("build/bin/smoke");

    assert!(built_output.exists());

    let _ = fs::remove_dir_all(temp_root);
}
