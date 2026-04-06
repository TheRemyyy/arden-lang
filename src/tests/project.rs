#![allow(unused_variables)]

#[allow(unused_imports)]
use super::*;
use crate::ast::{
    Decl, Expr, FunctionDecl, ImportDecl, Literal, Program, Spanned, Stmt, Type, Visibility,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[test]
fn project_build_supports_shadowed_alias_in_helper_return_path_survives_runtime() {
    let temp_root = make_temp_project_root("shadowed-alias-helper-return-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nmodule M { function add1(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        r#"
package app;
import util as u;

class Holder {
    value: Integer;
    constructor(v: Integer) { this.value = v; }
    function get(): Integer { return this.value + 10; }
}

function fetch(v: Integer): Holder {
    u: Holder = Holder(v);
    return u;
}

function main(): Integer {
    h: Holder = fetch(2);
    return h.get();
}
"#,
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support shadowed alias in helper return path");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled shadowed-alias-helper-return binary");
    assert_eq!(
        output.status.code(),
        Some(12),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_incorrectly_filters_class_dependency_under_shadowed_alias_in_dependency_closure() {
    let temp_root = make_temp_project_root("shadowed-alias-dependency-filtering");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package util;\nclass Box { value: Integer; constructor(v: Integer) { this.value = v; } }\n",
        )
        .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        r#"
package app;
import util as u;

class Local {
    value: Integer;
    constructor(v: Integer) { this.value = v; }
    function get(): Integer { return this.value + 10; }
}

function main(): Integer {
    u: Local = Local(2);
    return u.get();
}
"#,
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should handle shadowed alias in dependency closure");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled shadowed-alias-dependency-filtering binary");
    assert_eq!(
        output.status.code(),
        Some(12),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_runs_split_file_module_named_main_in_entry_namespace_runtime() {
    let temp_root = make_temp_project_root("project-module-main-entry-runtime");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/module.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package core;\nfunction main(): Integer { return main.ping(); }\n",
    )
    .expect("write main");
    fs::write(
        src_dir.join("module.apex"),
        "package core;\nmodule main { function ping(): Integer { return 22; } }\n",
    )
    .expect("write module");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support split-file module named main");
    });

    let output = std::process::Command::new(temp_root.join("smoke"))
        .output()
        .expect("run compiled split-file module named main binary");
    assert_eq!(
        output.status.code(),
        Some(22),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_runs_split_file_class_named_main_in_entry_namespace_runtime() {
    let temp_root = make_temp_project_root("project-class-main-entry-runtime");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/model.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        r#"
package core;
function main(): Integer {
    value: main = main(22);
    return value.get();
}
"#,
    )
    .expect("write main");
    fs::write(
        src_dir.join("model.apex"),
        r#"
package core;
class main {
    value: Integer;
    constructor(v: Integer) { this.value = v; }
    function get(): Integer { return this.value; }
}
"#,
    )
    .expect("write model");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support split-file class named main");
    });

    let output = std::process::Command::new(temp_root.join("smoke"))
        .output()
        .expect("run compiled split-file class named main binary");
    assert_eq!(
        output.status.code(),
        Some(22),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_runs_split_file_enum_named_main_in_entry_namespace_runtime() {
    let temp_root = make_temp_project_root("project-enum-main-entry-runtime");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/enum.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        r#"
package core;
function main(): Integer {
    return match (main.Ok(22)) {
        Ok(value) => value,
    };
}
"#,
    )
    .expect("write main");
    fs::write(
        src_dir.join("enum.apex"),
        r#"
package core;
enum main {
    Ok(Integer)
}
"#,
    )
    .expect("write enum");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support split-file enum named main");
    });

    let output = std::process::Command::new(temp_root.join("smoke"))
        .output()
        .expect("run compiled split-file enum named main binary");
    assert_eq!(
        output.status.code(),
        Some(22),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_parse_cache_reuses_only_unchanged_files() {
    let temp_root = make_temp_project_root("parse-cache-selective");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.apex");
    let lib_file = src_dir.join("lib.apex");

    fs::write(
            &main_file,
            "package app;\nimport lib.math;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main file");
    fs::write(
        &lib_file,
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write lib file");

    let first_main = parse_project_unit(&temp_root, &main_file).expect("first main parse");
    let first_lib = parse_project_unit(&temp_root, &lib_file).expect("first lib parse");
    assert!(!first_main.from_parse_cache);
    assert!(!first_lib.from_parse_cache);

    thread::sleep(Duration::from_millis(5));
    fs::write(
        &lib_file,
        "package lib;\nfunction add(x: Integer): Integer { return x + 2; }\n",
    )
    .expect("rewrite lib file");

    let second_main = parse_project_unit(&temp_root, &main_file).expect("second main parse");
    let second_lib = parse_project_unit(&temp_root, &lib_file).expect("second lib parse");

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
    let main_file = src_dir.join("main.apex");
    let math_file = src_dir.join("math.apex");

    fs::write(
            &main_file,
            "package app;\nimport lib.math;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main file");
    fs::write(
        &math_file,
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write math file");

    let parsed_files = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main"),
        parse_project_unit(&temp_root, &math_file).expect("parse math"),
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

    let symbol_lookup = Arc::new(build_project_symbol_lookup(
        &crate::dependency::ProjectSymbolMaps {
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
        },
    ));
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
    let main_file = src_dir.join("main.apex");
    let enum_file = src_dir.join("enum.apex");

    fs::write(
            &main_file,
            "package core;\nfunction main(): Integer { return match (main.Ok(22)) { Ok(value) => value, }; }\n",
        )
        .expect("write main file");
    fs::write(&enum_file, "package core;\nenum main { Ok(Integer) }\n").expect("write enum file");

    let parsed_files = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main"),
        parse_project_unit(&temp_root, &enum_file).expect("parse enum"),
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

    let symbol_lookup = Arc::new(build_project_symbol_lookup(
        &crate::dependency::ProjectSymbolMaps {
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
        },
    ));
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
fn project_check_supports_cross_file_function_value_references() {
    let temp_root = make_temp_project_root("function-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package app;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nfunction main(): None { o: Option<(Integer) -> Integer> = Option.some(add1); r: Result<(Integer) -> Integer, String> = Result.ok(add1); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        check_command(None, false).expect("project check should support function value refs");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_imported_function_value_alias_references() {
    let temp_root = make_temp_project_root("function-value-import-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.add1 as inc;\nfunction main(): None { f: (Integer) -> Integer = inc; o: Option<(Integer) -> Integer> = Option.some(inc); x: Integer = f(2); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support imported function value aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_function_values() {
    let temp_root = make_temp_project_root("function-value-namespace-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\nfunction twice(f: (Integer) -> Integer, x: Integer): Integer { return f(f(x)); }\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): None { f: (Integer) -> Integer = u.add1; x: Integer = u.twice(f, 1); y: Integer = u.add1(2); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support namespace alias function values");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_nested_namespace_alias_function_values() {
    let temp_root = make_temp_project_root("function-value-nested-namespace-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nmodule M { function add1(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): None { f: (Integer) -> Integer = u.M.add1; x: Integer = u.M.add1(1); y: Integer = f(2); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support nested namespace alias function values");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_import_alias_explicit_generic_function_values() {
    let temp_root = make_temp_project_root("generic-fn-value-exact-import-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule U { function id<T>(value: T): T { return value; } }\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.U.id as ident;\nfunction main(): Integer { f: (Integer) -> Integer = ident<Integer>; return if (f(7) == 7) { 0 } else { 1 }; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support exact-import alias explicit generic function values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled exact-import alias explicit generic function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_root_namespace_alias_explicit_generic_function_values() {
    let temp_root = make_temp_project_root("generic-fn-value-root-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule U { function id<T>(value: T): T { return value; } }\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\nfunction main(): Integer { f: (Integer) -> Integer = root.U.id<Integer>; return if (f(7) == 7) { 0 } else { 1 }; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support root namespace alias explicit generic function values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled root namespace alias explicit generic function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_wildcard_import_calls() {
    let temp_root = make_temp_project_root("module-wildcard-import-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule U { function id<T>(value: T): T { return value; } }\n",
    )
    .expect("write util");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport app.U.*;\nfunction main(): Integer { return id(7); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support module wildcard import calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled module wildcard import call binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_wildcard_import_explicit_generic_function_values() {
    let temp_root = make_temp_project_root("module-wildcard-import-generic-fn-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule U { function id<T>(value: T): T { return value; } }\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.U.*;\nfunction main(): Integer { f: (Integer) -> Integer = id<Integer>; return if (f(7) == 7) { 0 } else { 1 }; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support module wildcard import explicit generic function values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled module wildcard import explicit generic function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_wildcard_import_integer_to_float_calls() {
    let temp_root = make_temp_project_root("module-wildcard-import-int-to-float-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule U { function scale(value: Float): Float { return value * 2.0; } }\n",
    )
    .expect("write util");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport app.U.*;\nfunction main(): Integer { value: Float = scale(3); return if (value == 6.0) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support wildcard imported int-to-float calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled wildcard-import int-to-float call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_wildcard_imported_nested_module_integer_to_float_calls() {
    let temp_root =
        make_temp_project_root("wildcard-import-nested-module-int-to-float-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule U { module Math { function scale(value: Float): Float { return value * 2.0; } } }\n",
    )
    .expect("write util");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport app.U.*;\nfunction main(): Integer { value: Float = Math.scale(3); return if (value == 6.0) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support wildcard imported nested-module int-to-float calls",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled wildcard-import nested-module int-to-float call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_wildcard_import_calls() {
    let temp_root = make_temp_project_root("stdlib-wildcard-import-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.math.*;\nfunction main(): Integer { return if (abs(-7) == 7) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support stdlib wildcard import calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled stdlib wildcard import call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_wildcard_import_function_values() {
    let temp_root = make_temp_project_root("stdlib-wildcard-import-fn-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.math.*;\nfunction main(): Integer { f: (Integer) -> Float = abs; return if (f(-7) == 7.0) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support stdlib wildcard import function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled stdlib wildcard import function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_exact_import_calls() {
    let temp_root = make_temp_project_root("stdlib-exact-import-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.math.abs as absolute;\nfunction main(): Integer { return if (absolute(-7) == 7) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support stdlib exact import calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled stdlib exact import call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_exact_import_function_values() {
    let temp_root = make_temp_project_root("stdlib-exact-import-fn-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.math.abs as absolute;\nfunction main(): Integer { f: (Integer) -> Float = absolute; return if (f(-7) == 7.0) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support stdlib exact import function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled stdlib exact import function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_exact_import_values() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-exact-import-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { value: Float = Pi; return if (value > 3.14 && value < 3.15) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support zero-arg stdlib exact import values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg stdlib exact import value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_string_exact_import_values() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-string-exact-import-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction main(): Integer { value: String = CurrentDir; return if (value.length() >= 1) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support zero-arg stdlib string exact import values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg stdlib string exact import value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_integer_exact_import_values() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-integer-exact-import-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { value: Integer = ArgCount; return if (value >= 1) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support zero-arg stdlib integer exact import values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg stdlib integer exact import value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_exact_import_if_expressions() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-exact-import-if-expr-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { value: Float = if (true) { Pi } else { 0.0 }; return if (value > 3.14 && value < 3.15) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support zero-arg stdlib exact import if expressions");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg stdlib exact import if expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_exact_import_match_expressions() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-exact-import-match-expr-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { value: Float = match (true) { true => Pi, false => 0.0, }; return if (value > 3.14 && value < 3.15) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support zero-arg stdlib exact import match expressions");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg stdlib exact import match expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_stdlib_zero_arg_exact_import_values() {
    let temp_root =
        make_temp_project_root("module-local-stdlib-zero-arg-exact-import-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nmodule Inner {\n    import std.system.cwd as CurrentDir;\n    function read(): String { value: String = CurrentDir; return value; }\n}\nfunction main(): Integer { value: String = Inner.read(); return if (value.length() >= 1) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support module-local zero-arg stdlib exact import values",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled module-local zero-arg stdlib exact import value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_exact_import_return_values() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-exact-import-return-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.system.cwd as CurrentDir;\nfunction read(): String { return CurrentDir; }\nfunction main(): Integer { value: String = read(); return if (value.length() >= 1) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support zero-arg stdlib exact import return values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg stdlib exact import return value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_wildcard_values() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-wildcard-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.math.*;\nfunction main(): Integer { value: Float = pi; return if (value > 3.14 && value < 3.15) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support zero-arg stdlib wildcard values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg stdlib wildcard value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_namespace_zero_arg_values() {
    let temp_root = make_temp_project_root("stdlib-namespace-zero-arg-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.math as math;\nfunction main(): Integer { value: Float = math.pi; return if (value > 3.14 && value < 3.15) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support stdlib namespace zero-arg values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled stdlib namespace zero-arg value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_stdlib_zero_arg_wildcard_string_values() {
    let temp_root = make_temp_project_root("stdlib-zero-arg-wildcard-string-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.system.*;\nfunction main(): Integer { value: String = cwd; return if (value.length() >= 1) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support stdlib zero-arg wildcard string values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled stdlib zero-arg wildcard string value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_if_expression_builtin_function_values() {
    let temp_root = make_temp_project_root("if-expression-builtin-function-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nfunction choose(flag: Boolean): (Integer) -> Float { return if (flag) { to_float } else { to_float }; }\nfunction main(): Integer { return if (choose(true)(1) == 1.0) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support if-expression builtin function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled if-expression builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_match_expression_builtin_function_values() {
    let temp_root = make_temp_project_root("match-expression-builtin-function-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nenum Mode { A, B }\nfunction choose(mode: Mode): (Integer) -> Float { return match (mode) { Mode.A => { to_float } Mode.B => { to_float } }; }\nfunction main(): Integer { return if (choose(Mode.A)(1) == 1.0) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support match-expression builtin function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled match-expression builtin function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_typed_lists() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-typed-list-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { values: List<Float> = List<Float>(); values.push(Pi); return if (values[0] > 3.14 && values[0] < 3.15) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support zero-arg exact import values in typed lists");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg exact import typed list binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_builtin_calls() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-builtin-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.math.pi as Pi;\nfunction main(): Integer { text: String = to_string(Pi); return if (text.length() >= 1) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support zero-arg exact import values in builtin calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg exact import builtin call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_string_builtins() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-string-builtin-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.system.cwd as CurrentDir;\nimport std.string.*;\nfunction main(): Integer { return if (Str.len(CurrentDir) >= 1) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support zero-arg exact import values in string builtins");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg exact import string builtin binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_time_builtin_calls() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-time-builtin-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.system.cwd as CurrentDir;\nimport std.time.*;\nfunction main(): Integer { formatted: String = Time.now(CurrentDir); return if (formatted.length() >= 0) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support zero-arg exact import values in time builtins");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg exact import time builtin binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_list_index_methods() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-list-index-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { values: List<Integer> = List<Integer>(); values.push(10); values.push(20); return values.get(ArgCount) - 20; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support zero-arg exact import values in list index methods");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg exact import list index binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_list_constructor_capacity() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-list-capacity-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { values: List<Integer> = List<Integer>(ArgCount); values.push(7); return values.get(0) - 7; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support zero-arg exact import values in list constructor capacity",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg exact import list capacity binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_index_expressions() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-index-expression-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { values: List<Integer> = List<Integer>(); values.push(10); values.push(20); return values[ArgCount] - 20; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support zero-arg exact import values in index expressions");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg exact import index expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_string_index_expressions() {
    let temp_root =
        make_temp_project_root("zero-arg-exact-import-value-string-index-expression-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.args.count as ArgCount;\nfunction main(): Integer { text: String = \"ab\"; letter: Char = text[ArgCount]; return if (letter == 'b') { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support zero-arg exact import values in string index expressions",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg exact import string index binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_zero_arg_exact_import_values_in_task_await_timeout() {
    let temp_root = make_temp_project_root("zero-arg-exact-import-value-await-timeout-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport std.args.count as ArgCount;\nimport std.time.*;\nfunction work(): Task<Integer> { return async { Time.sleep(50); return 7; }; }\nfunction main(): Integer { value: Option<Integer> = work().await_timeout(ArgCount); return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support zero-arg exact import values in Task.await_timeout",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled zero-arg exact import await_timeout binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_some_alias_calls() {
    let temp_root = make_temp_project_root("builtin-option-some-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport Option.Some as Present;\nfunction main(): Integer { value: Option<Integer> = Present(7); return if (value.unwrap() == 7) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support builtin Option.Some aliases");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled builtin Option.Some alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_result_ok_alias_function_values() {
    let temp_root = make_temp_project_root("builtin-result-ok-alias-fn-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport Result.Ok as Success;\nfunction main(): Integer { f: (Integer) -> Result<Integer, String> = Success; value: Result<Integer, String> = f(7); return if (value.unwrap() == 7) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support builtin Result.Ok alias function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled builtin Result.Ok alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_result_error_alias_function_values() {
    let temp_root = make_temp_project_root("builtin-result-error-alias-fn-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport Result.Error as Failure;\nfunction main(): Integer { f: (String) -> Result<Integer, String> = Failure; value: Result<Integer, String> = f(\"boom\"); return if (value.is_error()) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support builtin Result.Error alias function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled builtin Result.Error alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_alias_patterns() {
    let temp_root = make_temp_project_root("builtin-option-alias-pattern-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport Option.Some as Present;\nimport Option.None as Empty;\nfunction main(): Integer { value: Option<Integer> = Present(7); return match (value) { Present(inner) => if (inner == 7) { 0 } else { 1 }, Empty => 2, }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support builtin Option alias patterns");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled builtin Option alias pattern binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_result_alias_patterns() {
    let temp_root = make_temp_project_root("builtin-result-alias-pattern-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport Result.Ok as Success;\nimport Result.Error as Failure;\nfunction main(): Integer { value: Result<Integer, String> = Success(7); return match (value) { Success(inner) => if (inner == 7) { 0 } else { 1 }, Failure(err) => 2, }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support builtin Result alias patterns");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled builtin Result alias pattern binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_none_alias_values() {
    let temp_root = make_temp_project_root("builtin-option-none-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport Option.None as Empty;\nfunction main(): Integer { value: Option<Integer> = Empty; return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support builtin Option.None aliases");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled builtin Option.None alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_builtin_option_none_alias_values() {
    let temp_root = make_temp_project_root("module-local-builtin-option-none-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nmodule Inner { import Option.None as Empty; function keep(): Integer { value: Option<Integer> = Empty; return if (value.is_none()) { 0 } else { 1 }; } }\nfunction main(): Integer { return Inner.keep(); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support module-local builtin Option.None aliases");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled module-local builtin Option.None alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_none_alias_function_values() {
    let temp_root = make_temp_project_root("builtin-option-none-alias-fn-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport Option.None as Empty;\nfunction main(): Integer { f: () -> Option<Integer> = Empty; value: Option<Integer> = f(); return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support builtin Option.None alias function values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled builtin Option.None alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_none_alias_return_values() {
    let temp_root = make_temp_project_root("builtin-option-none-alias-return-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport Option.None as Empty;\nfunction make(): Option<Integer> { return Empty; }\nfunction main(): Integer { value: Option<Integer> = make(); return if (value.is_none()) { 0 } else { 1 }; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support builtin Option.None alias return values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled builtin Option.None alias return value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_builtin_option_none_alias_argument_values() {
    let temp_root = make_temp_project_root("builtin-option-none-alias-arg-value-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport Option.None as Empty;\nfunction take(value: Option<Integer>): Integer { return if (value.is_none()) { 0 } else { 1 }; }\nfunction main(): Integer { return take(Empty); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support builtin Option.None alias argument values");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled builtin Option.None alias argument value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_preserves_default_extern_link_names() {
    let temp_root = make_temp_project_root("project-extern-default-link-name");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nextern(c) function abs(value: Integer): Integer;\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport util.abs;\nfunction main(): Integer { return abs(-7); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should preserve default extern link names");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled project extern default-link-name binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_preserves_default_extern_link_names_through_exact_import_aliases() {
    let temp_root = make_temp_project_root("project-extern-default-link-name-alias");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nextern(c) function abs(value: Integer): Integer;\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport util.abs as absolute;\nfunction main(): Integer { return absolute(-7); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should preserve extern link names through exact import aliases");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled project extern default-link-name alias binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_preserves_module_extern_link_names_through_exact_import_aliases() {
    let temp_root = make_temp_project_root("project-module-extern-default-link-name-alias");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nmodule C { extern(c) function abs(value: Integer): Integer; }\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport util.C.abs as absolute;\nfunction main(): Integer { return absolute(-7); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should preserve module extern link names through exact import aliases",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled project module extern default-link-name alias binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_prefers_shadowed_local_over_namespace_alias_for_nested_field_chain_calls() {
    let temp_root = make_temp_project_root("shadowed-local-over-namespace-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nmodule M { function add1(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nclass Holder { function add1(x: Integer): Integer { return x + 5; } }\nfunction main(): Integer { u: Holder = Holder(); return u.add1(1); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should prefer shadowed local over namespace alias");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled shadowed-local-over-namespace-alias binary");
    assert_eq!(
        output.status.code(),
        Some(6),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_shadowed_local_nested_method_calls_without_alias_leakage() {
    let temp_root = make_temp_project_root("shadowed-local-nested-method-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package util;\nmodule M { class Box { value: Integer; constructor(v: Integer) { this.value = v; } function get(): Integer { return this.value; } } }\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nclass Holder { inner: u.M.Box; constructor(v: Integer) { this.inner = u.M.Box(v); } function get(): Integer { return this.inner.get() + 10; } }\nfunction main(): Integer { u: Holder = Holder(2); return u.get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should keep shadowed local nested method calls local");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled shadowed-local-nested-method binary");
    assert_eq!(
        output.status.code(),
        Some(12),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_class_constructors() {
    let temp_root = make_temp_project_root("class-constructor-namespace-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package util;\nclass Box { value: Integer; constructor(v: Integer) { this.value = v; } }\n",
        )
        .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport util as u;\nfunction main(): None { u.Box(2); return None; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support namespace alias class constructors");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_nested_module_class_constructors() {
    let temp_root =
        make_temp_project_root("nested-class-constructor-namespace-alias-build-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package util;\nmodule Api {\n    class Box {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n    }\n}\n",
        )
        .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport util as u;\nfunction main(): None { u.Api.Box(2); return None; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support namespace alias nested-module class constructors",
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_nested_module_namespace_aliases_without_functions() {
    let temp_root = make_temp_project_root("nested-module-namespace-alias-build-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nmodule Api {\n    class Box {\n        constructor() {}\n    }\n}\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport util.Api as u;\nfunction main(): None { u.Box(); return None; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support nested module namespace aliases without functions",
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_deep_nested_module_namespace_aliases_without_functions() {
    let temp_root = make_temp_project_root("deep-nested-module-namespace-alias-build-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package util;\nmodule Api {\n    module V1 {\n        class Box {\n            constructor() {}\n        }\n    }\n}\n",
        )
        .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport util.Api.V1 as u;\nfunction main(): None { u.Box(); return None; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support deep nested module namespace aliases without functions",
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_deep_nested_module_interface_aliases() {
    let temp_root = make_temp_project_root("deep-nested-module-interface-alias-build-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package util;\nmodule Api {\n    module V1 {\n        interface Named { function name(): Integer; }\n    }\n}\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.Api.V1 as u;\ninterface Printable extends u.Named { function print_me(): Integer; }\nclass Report implements Printable {\n    constructor() {}\n    function name(): Integer { return 1; }\n    function print_me(): Integer { return 2; }\n}\nfunction main(): None { return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support deep nested module interface aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_deep_nested_module_enum_alias_patterns() {
    let temp_root = make_temp_project_root("deep-nested-module-enum-alias-build-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package util;\nmodule Api {\n    module V1 {\n        enum Value { Ok(Integer) Error(Integer) }\n    }\n}\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.Api.V1 as u;\nfunction main(): None { value: u.Value = u.Value.Ok(2); match (value) { u.Value.Ok(v) => { require(v == 2); } u.Value.Error(err) => { require(false); } } return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support deep nested module enum alias patterns");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_if_expression_function_value_callees() {
    let temp_root = make_temp_project_root("ifexpr-function-callee-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction dec(x: Integer): Integer { return x - 1; }\nfunction main(): None { x: Integer = (if (true) { inc; } else { dec; })(1); require(x == 2); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support if-expression function-value callees");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_unit_enum_variant_values() {
    let temp_root = make_temp_project_root("unit-enum-variant-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nenum E { A, B }\nfunction main(): None { e: E = E.A; match (e) { E.A => { } E.B => { } } return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support unit enum variant values");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_enum_variant_aliases() {
    let temp_root = make_temp_project_root("exact-enum-variant-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nenum E { A(Integer) B(Integer) }\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.E.B as Variant;\nfunction main(): None { e: E = Variant(2); match (e) { E.A(v) => { require(false); } E.B(v) => { require(v == 2); } } return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support exact imported enum variant aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_imported_payload_enum_variant_function_value_aliases() {
    let temp_root = make_temp_project_root("imported-payload-enum-variant-fn-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nenum E { Wrap(Integer) }\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.E.Wrap as WrapCtor;\nfunction main(): Integer { f: (Integer) -> E = WrapCtor; value: E = f(7); return match (value) { E.Wrap(v) => { if (v == 7) { 0 } else { 1 } } }; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support imported payload enum variant function value aliases",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run imported payload enum variant function value alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_imported_unit_enum_variant_function_value_aliases() {
    let temp_root = make_temp_project_root("imported-unit-enum-variant-fn-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nenum Mode { A, B }\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.Mode.A as Pick;\nfunction main(): Integer { f: () -> Mode = Pick; return if (f() == Mode.A) { 0 } else { 1 }; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support imported unit enum variant function value aliases",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run imported unit enum variant function value alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rejects_colliding_top_level_enum_names_across_namespaces() {
    let temp_root = make_temp_project_root("colliding-enum-project");
    let src_dir = temp_root.join("src");
    let left_dir = src_dir.join("left");
    let right_dir = src_dir.join("right");
    fs::create_dir_all(&left_dir).expect("create left namespace dir");
    fs::create_dir_all(&right_dir).expect("create right namespace dir");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/left/util.apex", "src/right/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        left_dir.join("util.apex"),
        "package left;\nenum Shared { A }\n",
    )
    .expect("write left enum");
    fs::write(
        right_dir.join("util.apex"),
        "package right;\nenum Shared { B }\n",
    )
    .expect("write right enum");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nfunction main(): None { return None; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build should reject colliding enum names");
        assert!(
            err.contains("colliding top-level enum names"),
            "unexpected error: {err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_enum_variant_alias_patterns() {
    let temp_root = make_temp_project_root("exact-enum-variant-alias-pattern-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nenum E { A(Integer) B(Integer) }\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.E.B as Variant;\nfunction main(): None { e: E = Variant(2); match (e) { Variant(v) => { require(v == 2); } E.A(v) => { require(false); } } return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support exact imported enum variant alias patterns");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_nested_enum_variant_patterns() {
    let temp_root = make_temp_project_root("namespace-alias-nested-enum-pattern-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package util;\nmodule Result {\n    enum Value { Ok(Integer) Error(Integer) }\n}\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): None { value: u.Result.Value = u.Result.Value.Ok(2); match (value) { u.Result.Value.Ok(v) => { require(v == 2); } u.Result.Value.Error(err) => { require(false); } } return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support namespace alias nested enum variant patterns");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_nested_enum_aliases() {
    let temp_root = make_temp_project_root("exact-nested-enum-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule M { enum E { A(Integer) B(Integer) } }\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.E as Enum;\nfunction main(): None { e: Enum = Enum.B(2); match (e) { Enum.B(v) => { require(v == 2); } Enum.A(v) => { require(false); } } return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support exact imported nested enum aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_nested_enum_variant_aliases() {
    let temp_root = make_temp_project_root("exact-nested-enum-variant-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule M { enum E { A(Integer) B(Integer) } }\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.E.B as Variant;\nfunction main(): None { e: M.E = Variant(2); match (e) { Variant(v) => { require(v == 2); } M.E.A(v) => { require(false); } } return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support exact imported nested enum variant aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_nested_enums() {
    let temp_root = make_temp_project_root("namespace-alias-nested-enum-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule M { enum E { A(Integer) B(Integer) } }\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as u;\nfunction main(): None { e: u.M.E = u.M.E.B(2); match (e) { u.M.E.B(v) => { require(v == 2); } u.M.E.A(v) => { require(false); } } return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support namespace alias nested enums");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_nested_function_aliases_returning_classes() {
    let temp_root = make_temp_project_root("exact-nested-function-alias-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        function get(): Integer { return this.value; }\n    }\n    function mk(value: Integer): Box { return Box(value); }\n}\n",
        )
        .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.mk as mk;\nfunction main(): None { value: Integer = mk(2).get(); require(value == 2); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
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
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        function get(): Integer { return this.value; }\n    }\n}\n",
        )
        .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.Box as Boxed;\nfunction main(): None { value: Integer = Boxed(2).get(); require(value == 2); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support exact imported nested class aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_local_qualified_nested_class_paths() {
    let temp_root = make_temp_project_root("local-qualified-nested-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    class Box {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        function get(): Integer { return this.value; }\n    }\n}\nfunction main(): None { b: M.Box = M.Box(2); require(b.get() == 2); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support local qualified nested class paths");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_local_qualified_nested_generic_class_paths() {
    let temp_root = make_temp_project_root("local-qualified-nested-generic-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n}\nfunction main(): None { b: M.Box<Integer> = M.Box<Integer>(2); require(b.get() == 2); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support local qualified nested generic class paths");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_nested_generic_class_aliases() {
    let temp_root = make_temp_project_root("exact-nested-generic-class-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n}\n",
        )
        .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.Box as Boxed;\nfunction main(): None { b: Boxed<Integer> = Boxed<Integer>(2); require(b.get() == 2); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support exact imported nested generic class aliases");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_exact_imported_nested_generic_function_aliases_returning_classes() {
    let temp_root = make_temp_project_root("exact-nested-generic-function-alias-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n    function mk<T>(value: T): Box<T> { return Box<T>(value); }\n}\n",
        )
        .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.mk as mk;\nfunction main(): None { value: Integer = mk<Integer>(2).get(); require(value == 2); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
                "project build should support exact imported nested generic function aliases returning classes",
            );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_local_nested_generic_functions_returning_classes() {
    let temp_root = make_temp_project_root("local-nested-generic-function-runtime-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n    function mk<T>(value: T): Box<T> { return Box<T>(value); }\n}\nfunction main(): Integer { return M.mk<Integer>(2).get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support local nested generic function returns");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled local nested generic function binary");
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
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n    function mk<T>(value: T): Box<T> { return Box<T>(value); }\n}\n",
        )
        .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.mk as mk;\nfunction main(): Integer { return mk<Integer>(2).get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
                "project build should support exact imported nested generic function aliases at runtime",
            );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled imported nested generic function binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_specialized_parent_interface_methods() {
    let temp_root = make_temp_project_root("specialized-parent-interface-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package app;\ninterface Reader<T> { function read(): T; }\ninterface StringReader extends Reader<String> {}\nclass FileReader implements StringReader { function read(): String { return \"ok\"; } }\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.StringReader;\nimport app.FileReader;\nfunction main(): Integer { reader: StringReader = FileReader(); f: () -> String = reader.read; return if (reader.read().length() == 2 && f().length() == 2) { 0 } else { 1 }; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support specialized parent interface methods");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled specialized parent interface project binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_generic_namespace_alias_parent_interfaces() {
    let temp_root = make_temp_project_root("generic-alias-parent-interface-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.Api as api;\ninterface StringReader extends api.Reader<String> {}\nclass FileReader implements StringReader { function read(): String { return \"ok\"; } }\nfunction main(): Integer { reader: StringReader = FileReader(); return if (reader.read().length() == 2) { 0 } else { 1 }; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support generic namespace-alias parent interfaces");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled generic namespace-alias parent interface binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_generic_exact_import_alias_parent_interfaces() {
    let temp_root = make_temp_project_root("generic-exact-alias-parent-interface-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.Api.Reader as ReaderAlias;\ninterface StringReader extends ReaderAlias<String> {}\nclass FileReader implements StringReader { function read(): String { return \"ok\"; } }\nfunction main(): None { reader: StringReader = FileReader(); require(reader.read().length() == 2); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support generic exact-import alias parent interfaces");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_generic_namespace_alias_interface_annotations() {
    let temp_root = make_temp_project_root("generic-alias-interface-annotation-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.Api as api;\nclass FileReader implements api.Reader<String> { function read(): String { return \"ok\"; } }\nfunction main(): Integer { reader: api.Reader<String> = FileReader(); return if (reader.read().length() == 2) { 0 } else { 1 }; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support generic namespace-alias interface annotations");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled generic namespace-alias interface annotation binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_generic_exact_import_alias_interface_annotations() {
    let temp_root = make_temp_project_root("generic-exact-alias-interface-annotation-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.Api.Reader as ReaderAlias;\nclass FileReader implements ReaderAlias<String> { function read(): String { return \"ok\"; } }\nfunction main(): Integer { reader: ReaderAlias<String> = FileReader(); return if (reader.read().length() == 2) { 0 } else { 1 }; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support generic exact-import alias interface annotations",
        );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled generic exact-import alias interface annotation binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_generic_namespace_alias_interface_parameters() {
    let temp_root = make_temp_project_root("generic-alias-interface-parameter-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.Api as api;\nclass FileReader implements api.Reader<String> { function read(): String { return \"ok\"; } }\nfunction use_reader(reader: api.Reader<String>): Integer { return reader.read().length(); }\nfunction main(): Integer { return use_reader(FileReader()); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support generic namespace-alias interface parameters");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled generic namespace-alias interface parameter binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_generic_exact_import_alias_interface_returns() {
    let temp_root = make_temp_project_root("generic-exact-alias-interface-return-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.Api.Reader as ReaderAlias;\nclass FileReader implements ReaderAlias<String> { function read(): String { return \"ok\"; } }\nfunction make_reader(): ReaderAlias<String> { return FileReader(); }\nfunction main(): Integer { return make_reader().read().length(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support generic exact-import alias interface returns");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled generic exact-import alias interface return binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_module_local_generic_namespace_alias_interface_parameters() {
    let temp_root = make_temp_project_root("module-generic-alias-interface-parameter-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.Api as api;\nclass FileReader implements api.Reader<String> { function read(): String { return \"ok\"; } }\nmodule Helpers {\n    function use_reader(reader: api.Reader<String>): Integer { return reader.read().length(); }\n}\nfunction main(): Integer { return Helpers.use_reader(FileReader()); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
                "project build should support module-local generic namespace-alias interface parameters",
            );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled module-local generic namespace-alias interface parameter binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_generic_namespace_alias_interface_lambda_parameters() {
    let temp_root = make_temp_project_root("lambda-generic-alias-interface-parameter-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("util.apex"),
        "package app;\nmodule Api {\n    interface Reader<T> { function read(): T; }\n}\n",
    )
    .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.Api as api;\nclass FileReader implements api.Reader<String> { function read(): String { return \"ok\"; } }\nfunction main(): Integer {\n    use_reader: (api.Reader<String>) -> Integer = |reader: api.Reader<String>| reader.read().length();\n    return use_reader(FileReader());\n}\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support generic namespace-alias interface lambda parameters",
        );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled generic namespace-alias interface lambda parameter binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_nested_generic_methods_on_nested_generic_classes() {
    let temp_root = make_temp_project_root("nested-generic-method-runtime-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n        function get(): T { return this.value; }\n    }\n}\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): Integer { b: M.Box<Integer> = M.Box<Integer>(2); return b.map<Integer>(inc).get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support nested generic methods on nested generic classes",
        );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled nested generic method binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_nested_generic_method_alias_paths() {
    let temp_root = make_temp_project_root("nested-generic-method-alias-runtime-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n        function get(): T { return this.value; }\n    }\n}\nfunction inc(x: Integer): Integer { return x + 1; }\n",
        )
        .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.Box as Boxed;\nimport app.inc as inc;\nfunction main(): Integer { b: Boxed<Integer> = Boxed<Integer>(2); return b.map<Integer>(inc).get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support nested generic method alias paths");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled nested generic alias method binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_nested_generic_class_specializations() {
    let temp_root = make_temp_project_root("namespace-alias-nested-generic-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("util.apex"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function get(): T { return this.value; }\n        }\n        function mk(value: Integer): Box<Integer> { return Box<Integer>(value); }\n        async function mk_async(value: Integer): Task<Box<Integer>> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nimport util.M.N.Box as B;\nfunction main(): Integer { return u.M.N.Box<Integer>(41).value + B<Integer>(1).get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
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
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("util.apex"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n            function get(): T { return this.value; }\n        }\n        function mk(value: Integer): Box<Integer> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): Integer { return u.M.N.mk(46).map<Integer>(inc).get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
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
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("util.apex"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n            function get(): T { return this.value; }\n        }\n        function mk(value: Integer): Box<Integer> { return Box<Integer>(value); }\n        async function mk_async(value: Integer): Task<Box<Integer>> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nimport util.M.N.Box as B;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): Integer { return u.M.N.mk(46).map<Integer>(inc).get() + u.M.N.Box<Integer>(41).value + B<Integer>(1).get() + await(u.M.N.mk_async(43)).get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should emit nested generic specialization bodies in a single object",
        );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled mixed nested generic specialization binary");
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
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("util.apex"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function get(): T { return this.value; }\n        }\n        function mk(value: Integer): Box<Integer> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): Integer { return u.M.N.mk(42).get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support cross-package nested generic returns via namespace alias",
        );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run cross-package nested generic return project binary");
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
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("util.apex"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function get(): T { return this.value; }\n        }\n        async function mk_async(value: Integer): Task<Box<Integer>> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): Integer { return await(u.M.N.mk_async(43)).get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
                "project build should support cross-package nested generic async returns via namespace alias",
            );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run cross-package nested generic async return project binary");
    assert_eq!(status.code(), Some(43));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_qualified_module_type_paths() {
    let temp_root = make_temp_project_root("qualified-module-type-path-runtime-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule util {\n    class Item {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        function get(): Integer { return this.value; }\n    }\n    function mk(): Item { return Item(7); }\n}\nfunction main(): Integer {\n    item: util.Item = util.mk();\n    return item.get();\n}\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support qualified module type paths end-to-end");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled qualified module type path binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_user_defined_generic_classes_named_like_builtins() {
    let temp_root = make_temp_project_root("user-defined-generic-class-named-like-builtin-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nclass Box<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n    function get(): T { return this.value; }\n}\nfunction mk(value: Integer): Box<Integer> {\n    return Box<Integer>(value);\n}\nfunction main(): Integer {\n    return mk(42).get();\n}\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
                "project build should prefer user-defined generic classes over built-in container names",
            );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled user-defined builtin-named generic class binary");
    assert_eq!(status.code(), Some(42));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_nested_generic_methods_on_expression_receivers() {
    let temp_root = make_temp_project_root("nested-generic-method-expr-runtime-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n        function get(): T { return this.value; }\n    }\n    function make<T>(value: T): Box<T> { return Box<T>(value); }\n}\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): Integer { return M.make<Integer>(2).map<Integer>(inc).get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support nested generic methods on expression receivers");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled nested generic expression receiver binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_builtin_function_values_in_user_defined_builtin_named_generic_methods() {
    let temp_root =
        make_temp_project_root("builtin-fn-user-defined-builtin-named-generic-method-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nclass Box<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n    function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n    function get(): T { return this.value; }\n}\nfunction main(): Integer { mapped: Box<Float> = Box<Integer>(1).map<Float>(to_float); return if (mapped.get() == 1.0) { 0 } else { 1 }; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
                "project build should support builtin function values in user-defined builtin-named generic methods",
            );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled builtin function value generic method project binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_nested_generic_method_imported_expression_receivers() {
    let temp_root = make_temp_project_root("nested-generic-method-imported-expr-runtime-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/util.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n        function get(): T { return this.value; }\n    }\n    function make<T>(value: T): Box<T> { return Box<T>(value); }\n}\nfunction inc(x: Integer): Integer { return x + 1; }\n",
        )
        .expect("write util");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.make as make;\nimport app.inc as inc;\nfunction main(): Integer { return make<Integer>(2).map<Integer>(inc).get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support imported expression receivers for nested generic methods",
        );
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled imported nested generic expression receiver binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_async_block_import_alias_calls() {
    let temp_root = make_temp_project_root("async-block-import-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.add1 as inc;\nfunction main(): None { task: Task<Integer> = async { return inc(1); }; value: Integer = await(task); require(value == 2); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support async-block import alias calls");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_generic_bound_errors() {
    let temp_root = make_temp_project_root("project-demangled-generic-bound-errors");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\ninterface Named { function name(): Integer; }\nclass Plain { constructor() {} }\nclass Box<T extends Named> {\n    value: Integer;\n    constructor() { this.value = 1; }\n}\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport lib as u;\nfunction main(): Integer {\n    bad: u.Box<u.Plain> = u.Box<u.Plain>();\n    return bad.value;\n}\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with invalid bound should fail");
        assert!(err.contains("lib.Plain"), "{err}");
        assert!(err.contains("lib.Named"), "{err}");
        assert!(!err.contains("lib__Plain"), "{err}");
        assert!(!err.contains("lib__Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_if_branch_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-if-branch-mismatch");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\nclass A { constructor() {} }\nclass B { constructor() {} }\nfunction pick(flag: Boolean): Integer {\n    value: Integer = if (flag) { A() } else { B() };\n    return value;\n}\n",
        )
        .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport lib.pick as pick;\nfunction main(): Integer { return pick(true); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with if branch mismatch should fail");
        assert!(err.contains("then is lib.A, else is lib.B"), "{err}");
        assert!(!err.contains("lib__A"), "{err}");
        assert!(!err.contains("lib__B"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_assignment_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-assignment-mismatch");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package lib;\nclass Named { constructor() {} }\nclass Plain { constructor() {} }\n",
    )
    .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport lib as u;\nfunction main(): Integer {\n    value: u.Named = u.Plain();\n    return 0;\n}\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with assignment mismatch should fail");
        assert!(
            err.contains("cannot assign lib.Plain to variable of type lib.Named"),
            "{err}"
        );
        assert!(!err.contains("lib__Plain"), "{err}");
        assert!(!err.contains("lib__Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_unknown_field_class_name() {
    let temp_root = make_temp_project_root("project-demangled-unknown-field-class");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\nclass Named { constructor() {} }\nclass Box<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n}\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport lib as u;\nfunction main(): Integer {\n    return u.Box<u.Named>(u.Named()).missing;\n}\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with unknown field should fail");
        assert!(
            err.contains("Unknown field 'missing' on class 'lib.Box<lib.Named>'"),
            "{err}"
        );
        assert!(!err.contains("lib__Box"), "{err}");
        assert!(!err.contains("lib__Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_non_function_call_type() {
    let temp_root = make_temp_project_root("project-demangled-non-function-call-type");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\nclass Box<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n}\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport lib as u;\nfunction main(): Integer {\n    value: u.Box<Integer> = u.Box<Integer>(1);\n    return value(2);\n}\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with non-function call should fail");
        assert!(
            err.contains("Cannot call non-function type lib.Box<Integer>"),
            "{err}"
        );
        assert!(!err.contains("lib__Box"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_if_condition_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-if-condition-type");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\nclass Flag { constructor() {} }\nfunction bad(): Integer {\n    if (Flag()) { return 1; }\n    return 0;\n}\n",
        )
        .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport lib.bad as bad;\nfunction main(): Integer { return bad(); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with non-boolean if condition should fail");
        assert!(
            err.contains("Condition must be Boolean, found lib.Flag"),
            "{err}"
        );
        assert!(!err.contains("lib__Flag"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_index_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-index-type-mismatch");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\nclass Key { constructor() {} }\nfunction bad(): Integer {\n    xs: List<Integer> = List<Integer>();\n    xs.push(1);\n    return xs[Key()];\n}\n",
        )
        .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport lib.bad as bad;\nfunction main(): Integer { return bad(); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with bad index type should fail");
        assert!(
            err.contains("Index must be Integer, found lib.Key"),
            "{err}"
        );
        assert!(!err.contains("lib__Key"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_await_operand_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-await-operand-type");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\nclass Job { constructor() {} }\nfunction bad(): Integer {\n    return await(Job());\n}\n",
        )
        .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport lib.bad as bad;\nfunction main(): Integer { return bad(); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with non-task await operand should fail");
        assert!(
            err.contains("'await' can only be used on Task types, got lib.Job"),
            "{err}"
        );
        assert!(!err.contains("lib__Job"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_match_arm_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-match-arm-type-mismatch");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\nclass Left { constructor() {} }\nclass Right { constructor() {} }\nfunction bad(flag: Boolean): Integer {\n    value: Integer = match (flag) {\n        true => Left(),\n        false => Right(),\n    };\n    return value;\n}\n",
        )
        .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport lib.bad as bad;\nfunction main(): Integer { return bad(true); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with match arm mismatch should fail");
        assert!(
            err.contains("Match expression arm type mismatch: expected lib.Left, got lib.Right"),
            "{err}"
        );
        assert!(!err.contains("lib__Left"), "{err}");
        assert!(!err.contains("lib__Right"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_pattern_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-pattern-type-mismatch");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\nclass Token { constructor() {} }\nfunction bad(value: Token): Integer {\n    return match (value) {\n        1 => 0,\n        _ => 1,\n    };\n}\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport lib.bad as bad;\nimport lib.Token as Token;\nfunction main(): Integer { return bad(Token()); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with pattern type mismatch should fail");
        assert!(
            err.contains("Pattern type mismatch: expected lib.Token, found Integer"),
            "{err}"
        );
        assert!(!err.contains("lib__Token"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_option_some_argument_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-option-some-arg-mismatch");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\nclass Token { constructor() {} }\nfunction wrap(flag: Boolean): Option<Token> {\n    if (flag) {\n        return Option.some(1);\n    }\n    return Option.none();\n}\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport lib.wrap as wrap;\nfunction main(): Integer { return if (wrap(true).is_some()) { 1 } else { 0 }; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with Option.some argument mismatch should fail");
        assert!(
            err.contains("Return type mismatch: expected Option<lib.Token>, found Option<Integer>"),
            "{err}"
        );
        assert!(!err.contains("lib__Token"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_unknown_type_name() {
    let temp_root = make_temp_project_root("project-demangled-unknown-type-name");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package lib;\nmodule Api {\n    class Token { constructor() {} }\n}\n",
    )
    .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport lib as u;\nfunction read(value: u.Api.Missing): Integer {\n    return 0;\n}\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with unknown type should fail");
        assert!(err.contains("Unknown type: u.Api.Missing"), "{err}");
        assert!(!err.contains("lib__Api__Missing"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_invalid_nested_namespace_aliased_extern_signature() {
    let temp_root =
        make_temp_project_root("project-invalid-nested-namespace-aliased-extern-signature");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\nextern(c) function host(value: root.M.Api.Named): root.M.Api.Named;\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main");
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("write helper");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail for invalid namespace aliased extern signature");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
        assert!(
            !err.contains("Extern function 'app__host' has non-FFI-safe parameter"),
            "{err}"
        );
        assert!(
            !err.contains("Extern function 'app__host' has non-FFI-safe return type"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_lambda_signature() {
    let temp_root =
        make_temp_project_root("project-stale-nested-namespace-aliased-lambda-signature");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\nfunction main(): Integer {\n    f: (root.M.Api.Named) -> Integer = (value: root.M.Api.Named) => 0;\n    return 0;\n}\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.apex"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial namespace aliased lambda signature build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("rewrite helper without namespace aliased lambda signature interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).expect_err(
            "build should fail after namespace aliased lambda signature interface removal",
        );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_invalid_nested_namespace_aliased_constructor_type_arg() {
    let temp_root =
        make_temp_project_root("project-invalid-nested-namespace-aliased-constructor-type-arg");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\nfunction main(): Integer {\n    root.M.Box<root.M.Api.Named>();\n    return 0;\n}\n",
        )
        .expect("write main");
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M {\n    module Api { interface Labelled { function name(): Integer; } }\n    class Box<T> { constructor() {} }\n}\n",
        )
        .expect("write helper");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail for invalid namespace aliased constructor type arg");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Built smoke"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_function_type_let_annotation(
) {
    let temp_root = make_temp_project_root(
        "project-stale-nested-namespace-aliased-function-type-let-annotation",
    );
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\nfunction main(): Integer {\n    f: (root.M.Api.Named) -> Integer = (value: root.M.Api.Named) => 0;\n    return 0;\n}\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.apex"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial namespace aliased function-type let annotation build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("rewrite helper without namespace aliased function-type let annotation interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).expect_err(
                "build should fail after namespace aliased function-type let annotation interface removal",
            );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_nested_namespace_aliased_function_type_inside_generic_constructor() {
    let temp_root = make_temp_project_root(
        "project-nested-namespace-aliased-function-type-inside-generic-constructor",
    );
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\nfunction main(): Integer {\n    values: List<(root.M.Api.Named) -> Integer> = List<(root.M.Api.Named) -> Integer>();\n    return 0;\n}\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.apex"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
                "project build should accept nested namespace aliased function types in generic constructors",
            );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_nested_module_local_function_type_inside_generic_constructor() {
    let temp_root = make_temp_project_root(
        "project-nested-module-local-function-type-inside-generic-constructor",
    );
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    interface Named { function name(): Integer; }\n    function make(): Integer {\n        values: List<(Named) -> Integer> = List<(Named) -> Integer>();\n        return 0;\n    }\n}\nfunction main(): Integer { return M.make(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
                "project build should accept nested module-local function types in generic constructors",
            );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_local_module_function_call_type_args() {
    let temp_root = make_temp_project_root("project-local-module-function-call-type-args");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.*;\nmodule M {\n    class Box { constructor() {} }\n    function make<T>(): None { }\n}\nfunction main(): None {\n    M.make<M.Box>();\n}\n",
        )
        .expect("write main");
    fs::write(
            src_dir.join("helper.apex"),
            "package util;\nmodule N {\n    module M {\n        class Box { constructor() {} }\n    }\n}\n",
        )
        .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should accept local module function call type args");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_module_local_generic_interface_references() {
    let temp_root = make_temp_project_root("project-module-local-generic-interface-references");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    class Payload { constructor() {} }\n    interface Named<T> { }\n    interface Child extends Named<Payload> { }\n    class Book implements Named<Payload> { constructor() {} }\n}\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should accept module-local generic interface references");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_module_local_generic_function_values_with_local_type_args() {
    let temp_root =
        make_temp_project_root("project-module-local-generic-function-values-local-type-args");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    class Box {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n    }\n    function id<T>(value: T): T { return value; }\n    function run(): Integer {\n        f: (Box) -> Box = id<Box>;\n        return f(Box(7)).value;\n    }\n}\nfunction main(): Integer { return M.run(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should accept module-local generic function values with local type args",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled module-local generic function value binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_module_local_lambda_parameter_types() {
    let temp_root = make_temp_project_root("project-module-local-lambda-parameter-types");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    interface Named { function value(): Integer; }\n    class Box implements Named {\n        inner: Integer;\n        constructor(inner: Integer) { this.inner = inner; }\n        function value(): Integer { return this.inner; }\n    }\n    function run(): Integer {\n        f: (Named) -> Integer = (value: Named) => value.value();\n        return f(Box(21));\n    }\n}\nfunction main(): Integer { return M.run(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should accept module-local lambda parameter types");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled module-local lambda parameter binary");
    assert_eq!(status.code(), Some(21));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_accepts_module_local_nested_enum_variant_patterns() {
    let temp_root = make_temp_project_root("project-module-local-nested-enum-variant-patterns");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    module N {\n        enum E { A(Integer), B(Integer) }\n    }\n    function run(): Integer {\n        value: N.E = N.E.A(44);\n        return match (value) {\n            N.E.A(v) => v,\n            N.E.B(v) => v,\n        };\n    }\n}\nfunction main(): Integer { return M.run(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should accept module-local nested enum variant patterns");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run compiled module-local nested enum variant pattern binary");
    assert_eq!(status.code(), Some(44));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_avoids_cascading_errors_for_stale_nested_namespace_aliased_interface_type() {
    let temp_root = make_temp_project_root("project-stale-nested-namespace-aliased-interface-type");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\nclass Book implements root.M.Api.Named { constructor() {} function name(): Integer { return 1; } }\nfunction main(): Integer { value: root.M.Api.Named = Book(); return value.name(); }\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.apex"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial nested namespace aliased interface build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("rewrite helper without nested namespace aliased interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after nested namespace aliased interface removal");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(
            !err.contains(
                "Type mismatch: cannot assign app.Book to variable of type root.M.Api.Named"
            ),
            "{err}"
        );
        assert!(!err.contains("Unknown class: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_exact_imported_interface_alias_in_implements() {
    let temp_root =
        make_temp_project_root("project-stale-exact-imported-interface-alias-implements");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.Api.Named as Named;\nclass Book implements Named { constructor() {} function name(): Integer { return 1; } }\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.apex"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial exact imported interface implements build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("rewrite helper without exact imported implemented interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after exact imported implemented interface removal");
        assert!(
            err.contains("Imported alias 'Named' no longer resolves"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(
            !err.contains("Class 'app.Book' implements unknown interface 'Named'"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_in_implements() {
    let temp_root =
        make_temp_project_root("project-stale-nested-namespace-aliased-interface-implements");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\nclass Book implements root.M.Api.Named { constructor() {} function name(): Integer { return 1; } }\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.apex"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial namespace aliased implements build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("rewrite helper without namespace aliased implemented interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after namespace aliased implemented interface removal");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(
            !err.contains("Class 'app.Book' implements unknown interface 'root.M.Api.Named'"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_avoids_cascading_errors_for_stale_for_loop_namespace_aliased_interface_type() {
    let temp_root =
        make_temp_project_root("project-stale-for-loop-namespace-aliased-interface-type");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\nclass Book implements root.M.Api.Named { constructor() {} function name(): Integer { return 1; } }\nfunction books(): List<Book> { values: List<Book> = List<Book>(); values.push(Book()); return values; }\nfunction main(): Integer { for (value: root.M.Api.Named in books()) { return value.name(); } return 0; }\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.apex"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial for-loop namespace aliased interface build should succeed");
    });
    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled for-loop namespace aliased interface binary");
    assert_eq!(status.code(), Some(1));

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("rewrite helper without for-loop namespace aliased interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after for-loop namespace aliased interface removal");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(
                !err.contains("Loop variable type mismatch: declared root.M.Api.Named, but iterating over List<app.Book>"),
                "{err}"
            );
        assert!(!err.contains("Unknown class: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_class_field() {
    let temp_root =
        make_temp_project_root("project-stale-nested-namespace-aliased-interface-class-field");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\nclass Book { value: root.M.Api.Named; constructor() {} }\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.apex"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial namespace aliased class field build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("rewrite helper without namespace aliased class field interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after namespace aliased class field interface removal");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
        assert!(!err.contains("Unknown class: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_enum_payload() {
    let temp_root =
        make_temp_project_root("project-stale-nested-namespace-aliased-interface-enum-payload");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\nenum Wrap { Named(root.M.Api.Named) }\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.apex"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial namespace aliased enum payload build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("rewrite helper without namespace aliased enum payload interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after namespace aliased enum payload interface removal");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
        assert!(!err.contains("Unknown class: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_method_signature(
) {
    let temp_root =
        make_temp_project_root("project-stale-nested-namespace-aliased-interface-method-signature");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\nclass Book { constructor() {} function take(value: root.M.Api.Named): Integer { return 0; } }\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.apex"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial namespace aliased method signature build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("rewrite helper without namespace aliased method signature interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).expect_err(
            "build should fail after namespace aliased method signature interface removal",
        );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_constructor_signature(
) {
    let temp_root = make_temp_project_root(
        "project-stale-nested-namespace-aliased-interface-constructor-signature",
    );
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\nclass Book { constructor(value: root.M.Api.Named) {} }\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.apex"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial namespace aliased constructor signature build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("rewrite helper without namespace aliased constructor signature interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).expect_err(
            "build should fail after namespace aliased constructor signature interface removal",
        );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_interface_signature(
) {
    let temp_root = make_temp_project_root(
        "project-stale-nested-namespace-aliased-interface-interface-signature",
    );
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\ninterface NamedConsumer { function take(value: root.M.Api.Named): Integer; }\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.apex"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial namespace aliased interface signature build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("rewrite helper without namespace aliased interface signature interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).expect_err(
            "build should fail after namespace aliased interface signature interface removal",
        );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_import_error_for_stale_nested_namespace_aliased_interface_interface_return_signature(
) {
    let temp_root = make_temp_project_root(
        "project-stale-nested-namespace-aliased-interface-interface-return-signature",
    );
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as root;\ninterface NamedFactory { function make(): root.M.Api.Named; }\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main");
    fs::write(
        src_dir.join("helper.apex"),
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial namespace aliased interface return signature build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            src_dir.join("helper.apex"),
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("rewrite helper without namespace aliased interface return signature interface");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).expect_err(
                "build should fail after namespace aliased interface return signature interface removal",
            );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.Api.Named'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Unknown type: root.M.Api.Named"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_list_constructor_capacity_type_mismatch() {
    let temp_root = make_temp_project_root("project-demangled-list-constructor-capacity");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package lib;\nclass Token { constructor() {} }\n",
    )
    .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport lib.Token as Token;\nfunction main(): Integer {\n    xs: List<Integer> = List<Integer>(Token());\n    return xs.length();\n}\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with non-integer list capacity should fail");
        assert!(
            err.contains(
                "Constructor List<Integer> expects optional Integer capacity, got lib.Token"
            ),
            "{err}"
        );
        assert!(!err.contains("lib__Token"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rejects_unknown_interface_signature_types() {
    let temp_root = make_temp_project_root("project-unknown-interface-signature-types");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package lib;\ninterface Api {\n    function decode(value: Missing): Missing;\n}\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nfunction main(): Integer { return 0; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with unknown interface signature types should fail");
        assert!(err.contains("Unknown type: Missing"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rejects_unknown_extern_signature_types() {
    let temp_root = make_temp_project_root("project-unknown-extern-signature-types");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nextern(c) function host(value: Missing): Missing;\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with unknown extern signature types should fail");
        assert!(err.contains("Unknown type: Missing"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rejects_unknown_enum_payload_types() {
    let temp_root = make_temp_project_root("project-unknown-enum-payload-types");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package lib;\nenum Message { Value(Missing) }\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nfunction main(): Integer { return 0; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with unknown enum payload type should fail");
        assert!(err.contains("Unknown type: Missing"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_unknown_implemented_interface() {
    let temp_root = make_temp_project_root("project-demangled-unknown-implemented-interface");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\nmodule Api {\n    class Report implements Missing {\n        constructor() {}\n    }\n}\n",
        )
        .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nfunction main(): Integer { return 0; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with unknown implemented interface should fail");
        assert!(
            err.contains("Class 'lib.Api.Report' implements unknown interface 'Missing'"),
            "{err}"
        );
        assert!(!err.contains("lib__Api__Report"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_demangled_unknown_variant_enum_name() {
    let temp_root = make_temp_project_root("project-demangled-unknown-variant-enum-name");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\nenum Choice { Left, Right }\nfunction read(): Integer {\n    return match (Choice.Left) {\n        Choice.Missing => 0,\n        _ => 1,\n    };\n}\n",
        )
        .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport lib.read as read;\nfunction main(): Integer { return read(); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build with unknown variant should fail");
        assert!(
            err.contains("Unknown variant 'Choice.Missing' for enum 'lib.Choice'"),
            "{err}"
        );
        assert!(!err.contains("lib__Choice"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_async_block_namespace_alias_unit_enum_tail_runtime() {
    let temp_root = make_temp_project_root("async-block-ns-alias-unit-enum-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(src_dir.join("lib.apex"), "package util;\nenum E { A, B }\n").expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): Integer { task: Task<u.E> = async { u.E.A }; value: u.E = await(task); match (value) { u.E.A => { return 0; } u.E.B => { return 1; } } }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support async-block namespace-alias unit-enum tails");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled async-block namespace-alias unit-enum tail binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_async_block_import_alias_function_value_tail_runtime() {
    let temp_root = make_temp_project_root("async-block-import-alias-function-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.add1 as inc;\nfunction main(): Integer { task: Task<(Integer) -> Integer> = async { inc }; f: (Integer) -> Integer = await(task); return f(1); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support async-block import-alias function-value tails");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled async-block import-alias function-value tail binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_async_block_import_alias_tail_expression_runtime() {
    let temp_root = make_temp_project_root("async-block-import-alias-tail-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.add1 as inc;\nfunction main(): Integer { task: Task<Integer> = async { inc(1) }; return await(task); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support async-block import-alias tail expressions");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled async-block import-alias tail-expression binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_unit_enum_values() {
    let temp_root = make_temp_project_root("namespace-alias-unit-enum-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(src_dir.join("lib.apex"), "package util;\nenum E { A, B }\n").expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): None { e: u.E = u.E.A; match (e) { u.E.A => { } u.E.B => { } } return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support namespace alias unit enum values");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_try_expression_function_value_callees() {
    let temp_root = make_temp_project_root("try-function-callee-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction choose(): Result<(Integer) -> Integer, String> { return Result.ok(inc); }\nfunction compute(): Result<Integer, String> { value: Integer = (choose()?)(1); return Result.ok(value); }\nfunction main(): Integer { value: Integer = compute().unwrap(); require(value == 2); return 0; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support try-expression function-value callees");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_imported_explicit_generic_free_calls() {
    let temp_root = make_temp_project_root("imported-explicit-generic-free-call-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nfunction id<T>(x: T): T { return x; }\n",
    )
    .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.id;\nfunction main(): None { value: Integer = id<Integer>(1); require(value == 1); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support imported explicit generic free calls");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_imported_generic_class_instance_methods() {
    let temp_root = make_temp_project_root("imported-generic-class-method-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package util;\nclass Boxed<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n    function get(): T { return this.value; }\n}\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.Boxed;\nfunction main(): None { value: Integer = Boxed<Integer>(7).get(); require(value == 7); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support imported generic class instance methods");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_top_level_destructor_alias_rewrite() {
    let temp_root = make_temp_project_root("destructor-alias-rewrite-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.add1 as inc;\nclass Boxed {\n    value: Integer;\n    constructor(value: Integer) { this.value = value; }\n    destructor() { require(inc(this.value) == 2); }\n}\nfunction main(): Integer { box: Boxed = Boxed(1); return 0; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should rewrite top-level destructor alias calls");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled destructor alias rewrite binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_module_local_destructor_function_rewrite() {
    let temp_root = make_temp_project_root("module-destructor-rewrite-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    function score(x: Integer): Integer { return x + 1; }\n    class Boxed {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        destructor() { require(score(this.value) == 2); }\n    }\n    function make(): Boxed { return Boxed(1); }\n}\nfunction main(): Integer { box: M.Boxed = M.make(); return 0; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should rewrite module-local destructor calls");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled module destructor rewrite binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_method_calls_on_function_returned_objects() {
    let temp_root = make_temp_project_root("function-return-method-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nclass Boxed<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n    function get(): T { return this.value; }\n}\nfunction make_box(): Boxed<Integer> { return Boxed<Integer>(9); }\nfunction main(): None { value: Integer = make_box().get(); require(value == 9); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support method calls on function-returned objects");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_check_supports_namespace_alias_nested_module_generic_class_constructors() {
    let temp_root = make_temp_project_root("namespace-alias-nested-generic-class-check");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package util;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n    }\n}\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): None { b: u.M.Box<Integer> = u.M.Box<Integer>(1); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        check_command(None, false).expect(
            "project check should support namespace alias nested-module generic class constructors",
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_interface_implements() {
    let temp_root = make_temp_project_root("module-local-interface-implements-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    interface Named { function name(): Integer; }\n    class Book implements Named {\n        constructor() {}\n        function name(): Integer { return 1; }\n    }\n}\nfunction main(): None { return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support module-local interface implements");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_nested_interface_implements() {
    let temp_root = make_temp_project_root("module-local-nested-interface-implements-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    module Api { interface Named { function name(): Integer; } }\n    class Book implements Api.Named {\n        constructor() {}\n        function name(): Integer { return 1; }\n    }\n}\nfunction main(): None { return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support module-local nested interface implements");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_interface_extends() {
    let temp_root = make_temp_project_root("module-local-interface-extends-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    interface Named { function name(): Integer; }\n    interface Printable extends Named { function print_me(): Integer; }\n    class Report implements Printable {\n        constructor() {}\n        function name(): Integer { return 1; }\n        function print_me(): Integer { return 2; }\n    }\n}\nfunction main(): None { return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support module-local interface extends");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_nested_interface_extends() {
    let temp_root = make_temp_project_root("module-local-nested-interface-extends-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    module Api { interface Named { function name(): Integer; } }\n    interface Printable extends Api.Named { function print_me(): Integer; }\n    class Report implements Printable {\n        constructor() {}\n        function name(): Integer { return 1; }\n        function print_me(): Integer { return 2; }\n    }\n}\nfunction main(): None { return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support module-local nested interface extends");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_interface_extends_on_seeded_semantic_path() {
    let temp_root = make_temp_project_root("seeded-alias-interface-extends-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package lib;\ninterface Named { function name(): Integer; }\n",
    )
    .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport lib as u;\ninterface Printable extends u.Named { function print_me(): Integer; }\nclass Report implements Printable {\n    constructor() {}\n    function name(): Integer { return 1; }\n    function print_me(): Integer { return 2; }\n}\nfunction main(): None { return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support aliased interface extends on seeded path");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_nested_namespace_alias_interface_extends_on_seeded_semantic_path() {
    let temp_root = make_temp_project_root("seeded-nested-alias-interface-extends-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\nmodule Api {\n    interface Named { function name(): Integer; }\n    interface Printable { function print_me(): Integer; }\n}\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport lib as u;\ninterface Reportable extends u.Api.Named, u.Api.Printable {}\nclass Report implements Reportable {\n    constructor() {}\n    function name(): Integer { return 1; }\n    function print_me(): Integer { return 2; }\n}\nfunction main(): None { return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support nested aliased interface extends on seeded path");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_namespace_alias_generic_bounds() {
    let temp_root = make_temp_project_root("namespace-alias-generic-bounds-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            src_dir.join("lib.apex"),
            "package lib;\ninterface Named { function name(): Integer; }\nclass Person implements Named {\n    constructor() {}\n    function name(): Integer { return 1; }\n}\n",
        )
        .expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport lib as u;\nfunction read_name<T extends u.Named>(value: T): Integer { return value.name(); }\nfunction main(): None { person: u.Person = u.Person(); require(read_name(person) == 1); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support namespace alias generic bounds");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_no_check_rejects_namespace_alias_generic_bound_method_signature_mismatch() {
    let temp_root = make_temp_project_root("namespace-alias-generic-bound-method-signature-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package lib;\ninterface Named { function name(): Integer; }\nclass Person implements Named {\n    constructor() {}\n    function name(): Integer { return 1; }\n}\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport lib as u;\nfunction read_name<T extends u.Named>(value: T): Integer { f: (Integer) -> Integer = value.name; return f(1); }\nfunction main(): None { person: u.Person = u.Person(); require(read_name(person) == 1); return None; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, false, false, false)
            .expect_err("unchecked project build should reject generic bound method signature mismatch");
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
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): None { f: &(Integer) -> Integer = &inc; value: Integer = (*f)(1); require(value == 2); return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support dereferenced function-value callees");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_check_rejects_async_borrowed_reference_results() {
    let temp_root = make_temp_project_root("async-borrowed-result-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): None { task: Task<&(Integer) -> Integer> = async { return &inc; }; return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        let err = check_command(None, false)
            .expect_err("project check should reject async borrowed reference results");
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
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nasync function read_ref(r: &Integer): Task<Integer> { return *r; }\nfunction main(): None { x: Integer = 1; alias: &Integer = &x; task: Task<Integer> = async { return *alias; }; return None; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        let err = check_command(None, false).expect_err(
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
fn project_build_runs_float_interpolation_from_aliased_async_module_runtime() {
    let temp_root = make_temp_project_root("float-interpolation-aliased-async-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        r#"
package util;
module Api {
    module V1 {
        function promote(value: Integer): Float {
            return to_float(value) / 2.0;
        }

        async function measure(value: Integer): Task<Float> {
            return promote(value) + 0.25;
        }
    }
}
"#,
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        r#"
package app;
import std.io.*;
import util as u;

function main(): Integer {
    println("async_value={await(u.Api.V1.measure(3))}");
    println("direct_value={u.Api.V1.promote(3)}");
    return 0;
}
"#,
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support aliased async float interpolation");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled aliased async float interpolation binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("async_value=1.750000"), "stdout={stdout}");
    assert!(stdout.contains("direct_value=1.500000"), "stdout={stdout}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_runs_dotted_package_async_float_await_runtime() {
    let temp_root = make_temp_project_root("dotted-package-async-float-await-runtime");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/lib.apex", "src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        r#"
package demo.analytics;
module Api {
    module V2 {
        async function score(v: Integer): Task<Float> {
            return to_float(v) + 10.0;
        }
    }
}
"#,
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        r#"
package app;
import std.io.*;
import demo.analytics as analytics;

function main(): Integer {
    score: Float = await(analytics.Api.V2.score(10));
    println("score={score}");
    return if (score == 20.0) { 0 } else { 1 };
}
"#,
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should preserve dotted-package async Float await values");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled dotted-package async float await binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("score=20.000000"),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_runs_dotted_package_async_function_value_runtime() {
    let temp_root = make_temp_project_root("dotted-package-async-function-value-runtime");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/lib.apex", "src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        r#"
package demo.analytics;
module Api {
    module V2 {
        async function score(v: Integer): Task<Float> {
            return to_float(v) + 10.0;
        }
    }
}
"#,
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        r#"
package app;
import std.io.*;
import demo.analytics as analytics;

function main(): Integer {
    f: (Integer) -> Task<Float> = analytics.Api.V2.score;
    score: Float = await(f(10));
    println("score={score}");
    return if (score == 20.0) { 0 } else { 1 };
}
"#,
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support dotted-package async function values");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled dotted-package async function value binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "score=20.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_namespace_alias_unit_enum_match_expressions() {
    let temp_root = make_temp_project_root("namespace-alias-unit-enum-match-expression-runtime");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(src_dir.join("lib.apex"), "package util;\nenum E { A, B }\n").expect("write lib");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): Integer { value: Integer = match (u.E.A) { u.E.A => { 1 } u.E.B => { 2 } }; require(value == 1); return 0; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("namespace alias unit enum match expression should build");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled namespace alias unit enum match expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_direct_constructor_method_calls() {
    let temp_root = make_temp_project_root("direct-ctor-method-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nclass Boxed { value: Integer; constructor(value: Integer) { this.value = value; } function get(): Integer { return this.value; } }\nfunction main(): Integer { return Boxed(23).get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, false, false, false)
            .expect("project build should support direct constructor method calls");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run direct constructor method project binary");
    assert_eq!(status.code(), Some(23));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_no_check_rejects_main_with_string_return_type_cleanly() {
    let temp_root = make_temp_project_root("project-main-string-return-type-nocheck");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nfunction main(): String { return \"oops\"; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, false, false, false)
            .expect_err("unchecked project build should reject invalid main signature");
        assert!(err.contains("main() must return None or Integer"), "{err}");
        assert!(!err.contains("Clang failed"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_rejects_non_binary_output_kind() {
    let temp_root = make_temp_project_root("run-non-binary-project");
    let src_dir = temp_root.join("src");
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\"]\noutput = \"smoke\"\noutput_kind = \"static\"\n",
        )
        .expect("write apex.toml");
    fs::write(
        src_dir.join("main.apex"),
        "function main(): None { return None; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err =
            run_project(&[], false, true, false).expect_err("run should reject library output");
        assert!(err.contains("requires `output_kind = \"bin\"`"), "{err}");
    });
    assert!(
        !temp_root.join("smoke").exists(),
        "run should fail before creating a library artifact"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_local_qualified_nested_enum_match_expressions() {
    let temp_root = make_temp_project_root("local-nested-enum-match-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M { enum E { A(Integer), B(Integer) } class Box { value: Integer; constructor(value: Integer) { this.value = value; } } }\nfunction main(): Integer { return (match (M.E.A(42)) { M.E.A(v) => M.Box(v), M.E.B(v) => M.Box(v) }).value; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, false, false, false)
            .expect("project build should support local qualified nested enum match expressions");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run local nested enum match project binary");
    assert_eq!(status.code(), Some(42));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_module_local_qualified_async_function_paths() {
    let temp_root = make_temp_project_root("module-local-qualified-async-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M { class Box { value: Integer; constructor(value: Integer) { this.value = value; } } async function mk(): M.Box { return M.Box(43); } }\nfunction main(): Integer { return await(M.mk()).value; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, false, false, false)
            .expect("project build should support module-local qualified async function paths");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run module-local qualified async project binary");
    assert_eq!(status.code(), Some(43));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_deeper_local_nested_module_function_paths() {
    let temp_root = make_temp_project_root("deeper-local-nested-module-function-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M { module N { class Box { value: Integer; constructor(value: Integer) { this.value = value; } function get(): Integer { return this.value; } } function mk(): Box { return Box(51); } } }\nfunction main(): Integer { return M.N.mk().get(); }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, false, false, false)
            .expect("project build should support deeper local nested module function paths");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run deeper local nested module function project binary");
    assert_eq!(status.code(), Some(51));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_deeper_local_nested_module_async_paths() {
    let temp_root = make_temp_project_root("deeper-local-nested-module-async-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M { module N { class Box { value: Integer; constructor(value: Integer) { this.value = value; } } async function mk(): Box { return Box(53); } } }\nfunction main(): Integer { return await(M.N.mk()).value; }\n",
        )
        .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, false, false, false)
            .expect("project build should support deeper local nested module async paths");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run deeper local nested module async project binary");
    assert_eq!(status.code(), Some(53));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_run_supports_nested_module_destructors_with_import_alias_calls() {
    let temp_root = make_temp_project_root("nested-module-destructor-import-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport util.add1 as inc;\nmodule M { module N { class Box { constructor() {} destructor() { require(inc(1) == 2); } } } }\nfunction main(): Integer { b: M.N.Box = M.N.Box(); return 0; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support nested-module destructors with import alias calls",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run nested-module destructor import-alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_nested_module_generic_bounds_through_file_scope_aliases() {
    let temp_root = make_temp_project_root("nested-module-generic-bound-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package app;\ninterface Named { function name(): Integer; }\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport app.Named as NamedAlias;\nmodule M { module N { class Box<T extends NamedAlias> { value: T; constructor(value: T) { this.value = value; } function get(): Integer { return this.value.name(); } } } }\nclass Item implements NamedAlias { function name(): Integer { return 7; } }\nfunction main(): Integer { return M.N.Box<Item>(Item()).get(); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support nested-module generic bounds through file-scope aliases",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run nested-module generic bound alias binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_nested_module_interface_generic_bounds_through_file_scope_aliases() {
    let temp_root = make_temp_project_root("nested-module-interface-generic-bound-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package app;\ninterface Named { function name(): Integer; }\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport app.Named as NamedAlias;\nmodule M { module N { interface Reader<T extends NamedAlias> { function read(value: T): Integer; } class Box implements Reader<Item> { constructor() {} function read(value: Item): Integer { return value.name(); } } } }\nclass Item implements NamedAlias { function name(): Integer { return 7; } }\nfunction main(): Integer { reader: M.N.Reader<Item> = M.N.Box(); return reader.read(Item()); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should support nested-module interface generic bounds through file-scope aliases",
        );
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run nested-module interface generic bound alias binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_nested_module_generic_base_classes() {
    let temp_root = make_temp_project_root("nested-module-generic-base-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nmodule M { module N { class Payload { constructor() {} } class Base<T> { constructor() {} } class Child extends Base<Payload> { constructor() {} } } }\nfunction main(): Integer { value: M.N.Child = M.N.Child(); return 0; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support nested-module generic base classes");
    });

    let output_path = temp_root.join("smoke");
    let status = std::process::Command::new(&output_path)
        .status()
        .expect("run nested-module generic base class binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_generic_exact_import_alias_base_classes() {
    let temp_root = make_temp_project_root("generic-exact-alias-base-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package lib;\nclass Payload { constructor() {} }\nclass Base<T> { constructor() {} }\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport lib.Base as BaseAlias;\nimport lib.Payload as PayloadAlias;\nclass Child extends BaseAlias<PayloadAlias> { constructor() {} }\nfunction main(): Integer { value: Child = Child(); return 0; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support generic exact-import alias base classes");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled generic exact-import alias base class binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_generic_namespace_alias_base_classes() {
    let temp_root = make_temp_project_root("generic-namespace-alias-base-class-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package lib;\nclass Payload { constructor() {} }\nclass Base<T> { constructor() {} }\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nimport lib as u;\nclass Child extends u.Base<u.Payload> { constructor() {} }\nfunction main(): Integer { value: Child = Child(); return 0; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support generic namespace alias base classes");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled generic namespace alias base class binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_namespace_alias_imports() {
    let temp_root = make_temp_project_root("module-local-namespace-alias-import-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package lib;\nclass Box<T> { value: T; constructor(value: T) { this.value = value; } function get(): T { return this.value; } }\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nmodule M { import lib as u; function make(): Integer { f: (Integer) -> u.Box<Integer> = u.Box<Integer>; value: u.Box<Integer> = f(7); return value.get(); } }\nfunction main(): Integer { return M.make(); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support module-local namespace alias imports");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled module-local namespace alias binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_supports_module_local_exact_import_aliases() {
    let temp_root = make_temp_project_root("module-local-exact-import-alias-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/lib.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.apex"),
        "package lib;\nclass Box<T> { value: T; constructor(value: T) { this.value = value; } function get(): T { return this.value; } }\n",
    )
    .expect("write lib");
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nmodule M { import lib.Box as Boxed; function make(): Integer { f: (Integer) -> Boxed<Integer> = Boxed<Integer>; value: Boxed<Integer> = f(7); return value.get(); } }\nfunction main(): Integer { return M.make(); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should support module-local exact import aliases");
    });

    let status = std::process::Command::new(temp_root.join("smoke"))
        .status()
        .expect("run compiled module-local exact import alias binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_no_check_rejects_module_local_wildcard_import_leaking_to_top_level() {
    let temp_root = make_temp_project_root("module-local-wildcard-import-leak-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "package app;\nmodule Inner { import std.math.*; function keep(): Float { return abs(-1.0); } }\nfunction main(): Float { return abs(-1.0); }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("project build should reject top-level use of module-local wildcard import");
        assert!(
            err.contains("Function 'abs' is defined in 'std.math' but not imported in 'app'")
                || err.contains("Import check failed"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rechecks_module_local_wildcard_import_dependents_after_symbol_removal() {
    let temp_root = make_temp_project_root("project-build-module-local-wildcard-symbol-removal");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.apex"),
        "package app;\nmodule Inner { import lib.*; function run(): Integer { return add(1); } }\nfunction main(): Integer { return Inner.run(); }\n",
    )
    .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial module-local wildcard project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction plus(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("rewrite helper without module-local wildcard imported symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after module-local wildcard-imported symbol removal");
        assert!(
            err.contains("Wildcard import 'lib.*' no longer provides 'add'")
                || err.contains("Function 'add' is defined in 'lib' but not imported in 'app'")
                || err.contains("Import check failed"),
            "{err}"
        );
        assert!(!err.contains("Undefined variable: add"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rechecks_module_local_exact_import_alias_dependents_after_symbol_removal() {
    let temp_root =
        make_temp_project_root("project-build-module-local-exact-import-alias-symbol-removal");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.apex"),
        "package app;\nmodule Inner { import lib.add as plus_one; function run(): Integer { return plus_one(1); } }\nfunction main(): Integer { return Inner.run(); }\n",
    )
    .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial module-local exact-import alias project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction plus(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("rewrite helper without module-local exact-import alias symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after module-local exact-import alias symbol removal");
        assert!(
            err.contains("Imported alias 'plus_one' no longer resolves")
                || err.contains("Function 'plus_one' is defined")
                || err.contains("Import check failed"),
            "{err}"
        );
        assert!(!err.contains("Undefined variable: plus_one"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rechecks_module_local_namespace_alias_dependents_after_symbol_removal() {
    let temp_root =
        make_temp_project_root("project-build-module-local-namespace-alias-symbol-removal");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.apex"),
        "package app;\nmodule Inner { import lib as l; function run(): Integer { return l.add(1); } }\nfunction main(): Integer { return Inner.run(); }\n",
    )
    .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial module-local namespace-alias project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction plus(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("rewrite helper without module-local namespace-alias symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after module-local namespace-alias symbol removal");
        assert!(
            err.contains("Imported namespace alias 'l' has no member 'add'")
                || err.contains("Import check failed"),
            "{err}"
        );
        assert!(!err.contains("Undefined variable: l"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_check_recovers_cleanly_after_invalid_files_list_fix() {
    let temp_root = make_temp_project_root("project-check-invalid-files-list-fix");
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/helper.txt\", \"src/main.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("write invalid apex.toml");
    fs::write(
        temp_root.join("src/main.apex"),
        "function main(): None { return None; }\n",
    )
    .expect("write main");
    fs::write(temp_root.join("src/helper.txt"), "not apex\n").expect("write helper");

    with_current_dir(&temp_root, || {
        let err = check_file(None).expect_err("check should reject invalid files list entry");
        assert!(
            err.contains("src/helper.txt") || err.contains("is not an .apex file"),
            "{err}"
        );
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("rewrite valid apex.toml");

    with_current_dir(&temp_root, || {
        check_file(None).expect("check should recover cleanly after fixing files list");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_recover_after_repeated_helper_validity_toggles() {
    let temp_root = make_temp_project_root("project-commands-helper-validity-toggles");
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("write apex.toml");
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main");

    let invalid_helper = "package lib;\nfunction add(: Integer { return 1; }\n";
    let valid_helper = "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n";

    fs::write(temp_root.join("src/helper.apex"), invalid_helper).expect("write invalid helper");
    with_current_dir(&temp_root, || {
        check_command(None, false).expect_err("check should fail on first invalid helper");
        build_project(false, false, true, false, false)
            .expect_err("build should fail on first invalid helper");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(temp_root.join("src/helper.apex"), valid_helper).expect("write valid helper");
    with_current_dir(&temp_root, || {
        check_command(None, false).expect("check should pass on first valid helper");
        build_project(false, false, true, false, false)
            .expect("build should pass on first valid helper");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(temp_root.join("src/helper.apex"), invalid_helper).expect("rewrite invalid helper");
    with_current_dir(&temp_root, || {
        check_command(None, false).expect_err("check should fail on second invalid helper");
        build_project(false, false, true, false, false)
            .expect_err("build should fail on second invalid helper");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(temp_root.join("src/helper.apex"), valid_helper).expect("rewrite valid helper again");
    with_current_dir(&temp_root, || {
        check_command(None, false).expect("check should pass after repeated validity toggles");
        build_project(false, false, true, false, false)
            .expect("build should pass after repeated validity toggles");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_ignore_metadata_only_touch_after_recovery() {
    let temp_root = make_temp_project_root("project-commands-metadata-touch-after-recovery");
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("write apex.toml");
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(: Integer { return 1; }\n",
    )
    .expect("write malformed helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).expect_err("project check should fail on malformed helper");
        build_project(false, false, true, false, false)
            .expect_err("build should fail on malformed helper");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    let fixed_helper = "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n";
    fs::write(temp_root.join("src/helper.apex"), fixed_helper).expect("rewrite valid helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).expect("project check should recover after helper fix");
        build_project(false, false, true, false, false)
            .expect("build should recover after helper fix");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(temp_root.join("src/helper.apex"), fixed_helper)
        .expect("rewrite identical helper for metadata touch");

    with_current_dir(&temp_root, || {
        check_command(None, false)
            .expect("project check should ignore metadata-only touch after recovery");
        build_project(false, false, true, false, false)
            .expect("build should ignore metadata-only touch after recovery");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_recover_cleanly_after_metadata_only_config_edit() {
    let temp_root = make_temp_project_root("project-commands-metadata-config-edit");
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("write apex.toml");
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).expect("project check should pass initially");
        build_project(false, false, true, false, false).expect("build should pass initially");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.1\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke2\"\n",
        )
        .expect("rewrite metadata-only apex.toml");

    with_current_dir(&temp_root, || {
        check_command(None, false)
            .expect("project check should recover after metadata-only config edit");
        build_project(false, false, true, false, false)
            .expect("build should recover after metadata-only config edit");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_recover_cleanly_after_output_only_config_edit() {
    let temp_root = make_temp_project_root("project-commands-output-only-config-edit");
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("write apex.toml");
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).expect("project check should pass initially");
        build_project(false, false, true, false, false).expect("build should pass initially");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke-renamed\"\n",
        )
        .expect("rewrite output-only apex.toml");

    with_current_dir(&temp_root, || {
        check_command(None, false).expect("project check should ignore output-only config edit");
        build_project(false, false, true, false, false)
            .expect("build should rebuild cleanly after output-only config edit");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(not(windows))]
#[test]
fn project_build_rebuilds_after_same_length_source_edit_with_preserved_mtime() {
    let temp_root = make_temp_project_root("project-build-same-length-preserved-mtime");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "build/out");
    let source_path = temp_root.join("src/main.apex");
    let output_path = temp_root.join("build/out");
    let mtime_reference = temp_root.join("src/main.mtime_ref.apex");

    fs::write(
        &source_path,
        "package app;\nfunction main(): Integer { return 11; }\n",
    )
    .expect("write initial main");
    fs::copy(&source_path, &mtime_reference).expect("write mtime reference");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect("initial build should pass");
    });
    let first_status = std::process::Command::new(&output_path)
        .status()
        .expect("run first built binary");
    assert_eq!(first_status.code(), Some(11));

    fs::write(
        &source_path,
        "package app;\nfunction main(): Integer { return 22; }\n",
    )
    .expect("rewrite main with same-length content");
    let touch_status = std::process::Command::new("touch")
        .arg("-r")
        .arg(&mtime_reference)
        .arg(&source_path)
        .status()
        .expect("run touch to preserve main mtime");
    assert!(touch_status.success(), "touch should preserve source mtime");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("build should rebuild after same-length content change");
    });
    let second_status = std::process::Command::new(&output_path)
        .status()
        .expect("run rebuilt binary after same-length content change");
    assert_eq!(second_status.code(), Some(22));

    let _ = fs::remove_file(mtime_reference);
    let _ = fs::remove_dir_all(temp_root);
}

#[cfg(not(windows))]
#[test]
fn parse_project_unit_reparses_after_same_length_source_edit_with_preserved_mtime() {
    let temp_root = make_temp_project_root("parse-cache-same-length-preserved-mtime");
    let source_path = temp_root.join("src/main.apex");
    let mtime_reference = temp_root.join("src/main.mtime_ref.apex");

    fs::write(
        &source_path,
        "package app;\nfunction main(): Integer { return 11; }\n",
    )
    .expect("write initial source");
    fs::copy(&source_path, &mtime_reference).expect("write mtime reference");

    let first = parse_project_unit(&temp_root, &source_path).expect("first parse");
    assert!(!first.from_parse_cache);

    fs::write(
        &source_path,
        "package app;\nfunction main(): Integer { return 22; }\n",
    )
    .expect("rewrite source with same-length content");
    let touch_status = std::process::Command::new("touch")
        .arg("-r")
        .arg(&mtime_reference)
        .arg(&source_path)
        .status()
        .expect("run touch to preserve source mtime");
    assert!(touch_status.success(), "touch should preserve source mtime");

    let second = parse_project_unit(&temp_root, &source_path)
        .expect("second parse after same-length content change");
    assert!(!second.from_parse_cache);
    assert_ne!(first.semantic_fingerprint, second.semantic_fingerprint);

    let _ = fs::remove_file(mtime_reference);
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
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("write apex.toml");
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect("initial build should pass");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke-a\"\n",
        )
        .expect("rewrite output path a");
    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("build should pass after first output toggle");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke-b\"\n",
        )
        .expect("rewrite output path b");
    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("build should pass after second output toggle");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_recover_after_repeated_output_and_version_toggles() {
    let temp_root = make_temp_project_root("project-commands-output-version-toggles");
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("write apex.toml");
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect("initial build should pass");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.1\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke-a\"\n",
        )
        .expect("rewrite output/version a");
    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("build should pass after first metadata toggle");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.2\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke-b\"\n",
        )
        .expect("rewrite output/version b");
    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("build should pass after second metadata toggle");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_ignore_files_order_only_toggles() {
    let temp_root = make_temp_project_root("project-commands-files-order-toggles");
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("write apex.toml");
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).expect("initial check should pass");
        build_project(false, false, true, false, false).expect("initial build should pass");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/helper.apex\", \"src/main.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("rewrite file order");

    with_current_dir(&temp_root, || {
        check_command(None, false).expect("check should ignore files-order-only toggle");
        build_project(false, false, true, false, false)
            .expect("build should ignore files-order-only toggle");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_recovers_cleanly_after_invalid_files_list_fix() {
    let temp_root = make_temp_project_root("project-build-invalid-files-list-fix");
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/helper.txt\", \"src/main.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("write invalid apex.toml");
    fs::write(
        temp_root.join("src/main.apex"),
        "function main(): None { return None; }\n",
    )
    .expect("write main");
    fs::write(temp_root.join("src/helper.txt"), "not apex\n").expect("write helper");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should reject invalid files list entry");
        assert!(
            err.contains("src/helper.txt") || err.contains("is not an .apex file"),
            "{err}"
        );
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("rewrite valid apex.toml");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("build should recover cleanly after fixing files list");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_recover_cleanly_after_malformed_helper_fix() {
    let temp_root = make_temp_project_root("project-commands-recover-malformed-helper");
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("write apex.toml");
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(: Integer { return 1; }\n",
    )
    .expect("write malformed helper");

    with_current_dir(&temp_root, || {
        let check_err =
            check_command(None, false).expect_err("project check should fail on malformed helper");
        assert!(check_err.contains("Parse error"), "{check_err}");
        let build_err = build_project(false, false, true, false, false)
            .expect_err("build should fail on malformed helper");
        assert!(build_err.contains("Parse error"), "{build_err}");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("rewrite valid helper");

    with_current_dir(&temp_root, || {
        show_project_info().expect("info should recover after helper fix");
        check_command(None, false).expect("project check should recover after helper fix");
        build_project(false, false, true, false, false)
            .expect("build should recover after helper fix");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_commands_recover_after_malformed_source_then_output_toggle() {
    let temp_root = make_temp_project_root("project-commands-malformed-then-output-toggle");
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("write apex.toml");
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(: Integer { return 1; }\n",
    )
    .expect("write malformed helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).expect_err("project check should fail on malformed helper");
        build_project(false, false, true, false, false)
            .expect_err("build should fail on malformed helper");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("rewrite valid helper");
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke-renamed\"\n",
        )
        .expect("rewrite output path after recovery");

    with_current_dir(&temp_root, || {
        check_command(None, false)
            .expect("project check should recover after malformed helper fix and output toggle");
        build_project(false, false, true, false, false)
            .expect("build should recover after malformed helper fix and output toggle");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_recovers_after_malformed_helper_fix_with_cache_history() {
    let temp_root = make_temp_project_root("project-build-recover-malformed-helper");
    fs::write(
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"smoke\"\n",
        )
        .expect("write apex.toml");
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport lib.add;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(: Integer { return 1; }\n",
    )
    .expect("write malformed helper");

    with_current_dir(&temp_root, || {
        let check_err =
            check_command(None, false).expect_err("project check should fail on malformed helper");
        assert!(check_err.contains("Parse error"), "{check_err}");
        let build_err = build_project(false, false, true, false, false)
            .expect_err("build should fail on malformed helper");
        assert!(build_err.contains("Parse error"), "{build_err}");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("rewrite valid helper");

    with_current_dir(&temp_root, || {
        check_command(None, false).expect("project check should recover after helper fix");
        build_project(false, false, true, false, false)
            .expect("build should recover after helper fix");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn new_project_rejects_names_that_would_generate_invalid_scaffolding() {
    let temp_root = make_temp_project_root("new-project-invalid-name-parent");
    let project_path = temp_root.join("target");

    let err = new_project("bad\"name", Some(&project_path))
        .expect_err("invalid project name should be rejected");
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
    fs::write(&project_path, "occupied\n").expect("write existing file");

    let err = new_project("demo", Some(&project_path))
        .expect_err("existing file path should block scaffold creation");
    assert!(err.contains("Path '"), "{err}");
    assert!(!err.contains("Directory '"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rejects_output_paths_outside_project_root() {
    let temp_root = make_temp_project_root("project-output-escape");
    let outside_dir = temp_root
        .parent()
        .expect("temp dir should have parent")
        .join("apex-output-escape-target");
    let rel_outside = format!(
        "../{}/smoke",
        outside_dir
            .file_name()
            .and_then(|name| name.to_str())
            .expect("outside dir name")
    );
    fs::write(
            temp_root.join("apex.toml"),
            format!(
                "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\"]\noutput = \"{}\"\n",
                rel_outside
            ),
        )
        .expect("write apex.toml");
    fs::write(
        temp_root.join("src/main.apex"),
        "function main(): None { return None; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        let err = build_project(false, true, true, false, false)
            .expect_err("build should reject output paths outside the project root");
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
            temp_root.join("apex.toml"),
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"src/main.apex\"\nfiles = [\"src/main.apex\", \"src/helper.apex\"]\noutput = \"src/helper.apex\"\n",
        )
        .expect("write apex.toml");
    fs::write(
        temp_root.join("src/main.apex"),
        "package app;\nimport lib.helper;\nfunction main(): Integer { return helper(); }\n",
    )
    .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction helper(): Integer { return 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should reject output path matching a source file");
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
        &["src/main.apex"],
        "src/main.apex",
        "build/bin/smoke",
    );
    fs::write(
        src_dir.join("main.apex"),
        "function main(): Integer { return 0; }\n",
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("project build should create missing nested output directories");
    });

    assert!(temp_root.join("build/bin/smoke").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_rewrite_fingerprint_ignores_body_only_dependency_change() {
    let temp_root = make_temp_project_root("rewrite-fp-body-only");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.apex");
    let helper_file = src_dir.join("helper.apex");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nimport lib.foo;\nfunction main(): None { value: Integer = foo(); return None; }\n",
        )
        .expect("write main");
    fs::write(
        &helper_file,
        "package lib;\nfunction foo(): Integer { return 1; }\n",
    )
    .expect("write helper");
    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main before"),
        parse_project_unit(&temp_root, &helper_file).expect("parse helper before"),
    ];
    let symbol_maps_before = collect_project_symbol_maps(&parsed_before);
    let namespace_functions_before = parsed_before.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.function_names.iter().cloned());
            acc
        },
    );
    let namespace_classes_before = parsed_before.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.class_names.iter().cloned());
            acc
        },
    );
    let namespace_modules_before = parsed_before.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.module_names.iter().cloned());
            acc
        },
    );
    let namespace_api_fingerprints_before = compute_namespace_api_fingerprints(&parsed_before);
    let file_api_fingerprints_before = parsed_before
        .iter()
        .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
        .collect::<HashMap<_, _>>();
    let rewrite_ctx_before = RewriteFingerprintContext {
        namespace_functions: &namespace_functions_before,
        global_function_map: &symbol_maps_before.global_function_map,
        global_function_file_map: &symbol_maps_before.global_function_file_map,
        namespace_classes: &namespace_classes_before,
        global_class_map: &symbol_maps_before.global_class_map,
        global_class_file_map: &symbol_maps_before.global_class_file_map,
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &symbol_maps_before.global_enum_map,
        global_enum_file_map: &symbol_maps_before.global_enum_file_map,
        namespace_modules: &namespace_modules_before,
        global_module_map: &symbol_maps_before.global_module_map,
        global_module_file_map: &symbol_maps_before.global_module_file_map,
        namespace_api_fingerprints: &namespace_api_fingerprints_before,
        file_api_fingerprints: &file_api_fingerprints_before,
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
                function_map: &symbol_maps_before.global_function_map,
                function_file_map: &symbol_maps_before.global_function_file_map,
                class_map: &symbol_maps_before.global_class_map,
                class_file_map: &symbol_maps_before.global_class_file_map,
                interface_map: empty_global_interface_map(),
                interface_file_map: empty_global_interface_file_map(),
                enum_map: &symbol_maps_before.global_enum_map,
                enum_file_map: &symbol_maps_before.global_enum_file_map,
                module_map: &symbol_maps_before.global_module_map,
                module_file_map: &symbol_maps_before.global_module_file_map,
            },
        )),
    };
    let main_before = parsed_before
        .iter()
        .find(|u| u.file == main_file)
        .expect("main");
    let rewrite_fp_before =
        compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

    thread::sleep(Duration::from_millis(5));
    fs::write(
        &helper_file,
        "package lib;\nfunction foo(): Integer { return 2; }\n",
    )
    .expect("rewrite helper body");

    let parsed_files = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main after"),
        parse_project_unit(&temp_root, &helper_file).expect("parse helper after"),
    ];
    let (
        namespace_files_map,
        namespace_function_files,
        namespace_class_files,
        namespace_module_files,
        global_function_map,
        global_function_file_map,
        global_class_map,
        global_class_file_map,
        global_enum_map,
        global_enum_file_map,
        global_module_map,
        global_module_file_map,
    ) = collect_project_symbol_maps(&parsed_files).into_parts();
    let namespace_functions = parsed_files.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.function_names.iter().cloned());
            acc
        },
    );
    let namespace_classes = parsed_files.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.class_names.iter().cloned());
            acc
        },
    );
    let namespace_modules = parsed_files.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.module_names.iter().cloned());
            acc
        },
    );
    let namespace_api_fingerprints = compute_namespace_api_fingerprints(&parsed_files);
    let file_api_fingerprints = parsed_files
        .iter()
        .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
        .collect::<HashMap<_, _>>();
    let rewrite_ctx = RewriteFingerprintContext {
        namespace_functions: &namespace_functions,
        global_function_map: &global_function_map,
        global_function_file_map: &global_function_file_map,
        namespace_classes: &namespace_classes,
        global_class_map: &global_class_map,
        global_class_file_map: &global_class_file_map,
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &global_enum_map,
        global_enum_file_map: &global_enum_file_map,
        namespace_modules: &namespace_modules,
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
        namespace_api_fingerprints: &namespace_api_fingerprints,
        file_api_fingerprints: &file_api_fingerprints,
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
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
            },
        )),
    };
    let main_unit = parsed_files
        .iter()
        .find(|u| u.file == main_file)
        .expect("main");
    let rewrite_fp_after =
        compute_rewrite_context_fingerprint_for_unit(main_unit, "app", &rewrite_ctx);
    let _ = namespace_files_map;

    assert_eq!(rewrite_fp_before, rewrite_fp_after);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_rewrite_fingerprint_changes_on_import_breaking_api_change() {
    let temp_root = make_temp_project_root("rewrite-fp-api-change");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.apex");
    let helper_file = src_dir.join("helper.apex");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nimport lib.foo;\nfunction main(): None { value: Integer = foo(); return None; }\n",
        )
        .expect("write main");
    fs::write(
        &helper_file,
        "package lib;\nfunction foo(): Integer { return 1; }\n",
    )
    .expect("write helper");
    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main before"),
        parse_project_unit(&temp_root, &helper_file).expect("parse helper before"),
    ];
    let (
        _namespace_files_map_before,
        namespace_function_files_before,
        namespace_class_files_before,
        namespace_module_files_before,
        global_function_map_before,
        global_function_file_map_before,
        global_class_map_before,
        global_class_file_map_before,
        global_enum_map_before,
        global_enum_file_map_before,
        global_module_map_before,
        global_module_file_map_before,
    ) = collect_project_symbol_maps(&parsed_before).into_parts();
    let namespace_functions_before = parsed_before.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.function_names.iter().cloned());
            acc
        },
    );
    let namespace_classes_before = parsed_before.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.class_names.iter().cloned());
            acc
        },
    );
    let namespace_modules_before = parsed_before.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.module_names.iter().cloned());
            acc
        },
    );
    let namespace_api_fingerprints_before = compute_namespace_api_fingerprints(&parsed_before);
    let file_api_fingerprints_before = parsed_before
        .iter()
        .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
        .collect::<HashMap<_, _>>();
    let rewrite_ctx_before = RewriteFingerprintContext {
        namespace_functions: &namespace_functions_before,
        global_function_map: &global_function_map_before,
        global_function_file_map: &global_function_file_map_before,
        namespace_classes: &namespace_classes_before,
        global_class_map: &global_class_map_before,
        global_class_file_map: &global_class_file_map_before,
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &global_enum_map_before,
        global_enum_file_map: &global_enum_file_map_before,
        namespace_modules: &namespace_modules_before,
        global_module_map: &global_module_map_before,
        global_module_file_map: &global_module_file_map_before,
        namespace_api_fingerprints: &namespace_api_fingerprints_before,
        file_api_fingerprints: &file_api_fingerprints_before,
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
                function_map: &global_function_map_before,
                function_file_map: &global_function_file_map_before,
                class_map: &global_class_map_before,
                class_file_map: &global_class_file_map_before,
                interface_map: empty_global_interface_map(),
                interface_file_map: empty_global_interface_file_map(),
                enum_map: &global_enum_map_before,
                enum_file_map: &global_enum_file_map_before,
                module_map: &global_module_map_before,
                module_file_map: &global_module_file_map_before,
            },
        )),
    };
    let main_before = parsed_before
        .iter()
        .find(|u| u.file == main_file)
        .expect("main");
    let rewrite_fp_before =
        compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

    thread::sleep(Duration::from_millis(5));
    fs::write(
        &helper_file,
        "package lib;\nfunction bar(): Integer { return 1; }\n",
    )
    .expect("rewrite helper api");

    let parsed_files = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main"),
        parse_project_unit(&temp_root, &helper_file).expect("parse helper"),
    ];
    let symbol_maps = collect_project_symbol_maps(&parsed_files);
    let namespace_functions = parsed_files.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.function_names.iter().cloned());
            acc
        },
    );
    let namespace_classes = parsed_files.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.class_names.iter().cloned());
            acc
        },
    );
    let namespace_modules = parsed_files.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.module_names.iter().cloned());
            acc
        },
    );
    let namespace_api_fingerprints = compute_namespace_api_fingerprints(&parsed_files);
    let file_api_fingerprints = parsed_files
        .iter()
        .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
        .collect::<HashMap<_, _>>();
    let rewrite_ctx = RewriteFingerprintContext {
        namespace_functions: &namespace_functions,
        global_function_map: &symbol_maps.global_function_map,
        global_function_file_map: &symbol_maps.global_function_file_map,
        namespace_classes: &namespace_classes,
        global_class_map: &symbol_maps.global_class_map,
        global_class_file_map: &symbol_maps.global_class_file_map,
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &symbol_maps.global_enum_map,
        global_enum_file_map: &symbol_maps.global_enum_file_map,
        namespace_modules: &namespace_modules,
        global_module_map: &symbol_maps.global_module_map,
        global_module_file_map: &symbol_maps.global_module_file_map,
        namespace_api_fingerprints: &namespace_api_fingerprints,
        file_api_fingerprints: &file_api_fingerprints,
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
                function_map: &symbol_maps.global_function_map,
                function_file_map: &symbol_maps.global_function_file_map,
                class_map: &symbol_maps.global_class_map,
                class_file_map: &symbol_maps.global_class_file_map,
                interface_map: empty_global_interface_map(),
                interface_file_map: empty_global_interface_file_map(),
                enum_map: &symbol_maps.global_enum_map,
                enum_file_map: &symbol_maps.global_enum_file_map,
                module_map: &symbol_maps.global_module_map,
                module_file_map: &symbol_maps.global_module_file_map,
            },
        )),
    };
    let main_unit = parsed_files
        .iter()
        .find(|u| u.file == main_file)
        .expect("main");
    let rewrite_fp_after =
        compute_rewrite_context_fingerprint_for_unit(main_unit, "app", &rewrite_ctx);

    assert_ne!(rewrite_fp_before, rewrite_fp_after);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_rewrite_fingerprint_changes_on_nested_namespace_aliased_interface_implements_api_change()
{
    let temp_root = make_temp_project_root("rewrite-fp-nested-alias-interface-implements");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.apex");
    let helper_file = src_dir.join("helper.apex");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nimport app as root;\nclass Book implements root.M.Api.Named { constructor() {} function name(): Integer { return 1; } }\nfunction main(): Integer { return 0; }\n",
        )
        .expect("write main");
    fs::write(
        &helper_file,
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .expect("write helper before");

    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main before"),
        parse_project_unit(&temp_root, &helper_file).expect("parse helper before"),
    ];

    let mut namespace_function_files_before: HashMap<String, HashMap<String, PathBuf>> =
        HashMap::new();
    let mut namespace_class_files_before: HashMap<String, HashMap<String, PathBuf>> =
        HashMap::new();
    let mut namespace_interface_files_before: HashMap<String, HashMap<String, PathBuf>> =
        HashMap::new();
    let mut namespace_module_files_before: HashMap<String, HashMap<String, PathBuf>> =
        HashMap::new();
    let mut global_function_map_before: HashMap<String, String> = HashMap::new();
    let mut global_function_file_map_before: HashMap<String, PathBuf> = HashMap::new();
    let mut global_class_map_before: HashMap<String, String> = HashMap::new();
    let mut global_class_file_map_before: HashMap<String, PathBuf> = HashMap::new();
    let mut global_interface_map_before: HashMap<String, String> = HashMap::new();
    let mut global_interface_file_map_before: HashMap<String, PathBuf> = HashMap::new();
    let mut global_enum_map_before: HashMap<String, String> = HashMap::new();
    let mut global_enum_file_map_before: HashMap<String, PathBuf> = HashMap::new();
    let mut global_module_map_before: HashMap<String, String> = HashMap::new();
    let mut global_module_file_map_before: HashMap<String, PathBuf> = HashMap::new();

    for unit in &parsed_before {
        for name in &unit.function_names {
            namespace_function_files_before
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_function_map_before.insert(name.clone(), unit.namespace.clone());
            global_function_file_map_before.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.class_names {
            namespace_class_files_before
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_class_map_before.insert(name.clone(), unit.namespace.clone());
            global_class_file_map_before.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.interface_names {
            namespace_interface_files_before
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_interface_map_before.insert(name.clone(), unit.namespace.clone());
            global_interface_file_map_before.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.enum_names {
            global_enum_map_before.insert(name.clone(), unit.namespace.clone());
            global_enum_file_map_before.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.module_names {
            namespace_module_files_before
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_module_map_before.insert(name.clone(), unit.namespace.clone());
            global_module_file_map_before.insert(name.clone(), unit.file.clone());
        }
    }

    let namespace_functions_before = parsed_before.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.function_names.iter().cloned());
            acc
        },
    );
    let namespace_classes_before = parsed_before.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.class_names.iter().cloned());
            acc
        },
    );
    let namespace_modules_before = parsed_before.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.module_names.iter().cloned());
            acc
        },
    );
    let namespace_api_fingerprints_before = compute_namespace_api_fingerprints(&parsed_before);
    let file_api_fingerprints_before = parsed_before
        .iter()
        .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
        .collect::<HashMap<_, _>>();
    let rewrite_ctx_before = RewriteFingerprintContext {
        namespace_functions: &namespace_functions_before,
        global_function_map: &global_function_map_before,
        global_function_file_map: &global_function_file_map_before,
        namespace_classes: &namespace_classes_before,
        global_class_map: &global_class_map_before,
        global_class_file_map: &global_class_file_map_before,
        global_interface_map: &global_interface_map_before,
        global_interface_file_map: &global_interface_file_map_before,
        global_enum_map: &global_enum_map_before,
        global_enum_file_map: &global_enum_file_map_before,
        namespace_modules: &namespace_modules_before,
        global_module_map: &global_module_map_before,
        global_module_file_map: &global_module_file_map_before,
        namespace_api_fingerprints: &namespace_api_fingerprints_before,
        file_api_fingerprints: &file_api_fingerprints_before,
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
                function_map: &global_function_map_before,
                function_file_map: &global_function_file_map_before,
                class_map: &global_class_map_before,
                class_file_map: &global_class_file_map_before,
                interface_map: &global_interface_map_before,
                interface_file_map: &global_interface_file_map_before,
                enum_map: &global_enum_map_before,
                enum_file_map: &global_enum_file_map_before,
                module_map: &global_module_map_before,
                module_file_map: &global_module_file_map_before,
            },
        )),
    };
    let main_before = parsed_before
        .iter()
        .find(|u| u.file == main_file)
        .expect("main before");
    let rewrite_fp_before =
        compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

    thread::sleep(Duration::from_millis(5));
    fs::write(
            &helper_file,
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .expect("write helper after");

    let parsed_after = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main after"),
        parse_project_unit(&temp_root, &helper_file).expect("parse helper after"),
    ];

    let mut namespace_function_files_after: HashMap<String, HashMap<String, PathBuf>> =
        HashMap::new();
    let mut namespace_class_files_after: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let mut namespace_interface_files_after: HashMap<String, HashMap<String, PathBuf>> =
        HashMap::new();
    let mut namespace_module_files_after: HashMap<String, HashMap<String, PathBuf>> =
        HashMap::new();
    let mut global_function_map_after: HashMap<String, String> = HashMap::new();
    let mut global_function_file_map_after: HashMap<String, PathBuf> = HashMap::new();
    let mut global_class_map_after: HashMap<String, String> = HashMap::new();
    let mut global_class_file_map_after: HashMap<String, PathBuf> = HashMap::new();
    let mut global_interface_map_after: HashMap<String, String> = HashMap::new();
    let mut global_interface_file_map_after: HashMap<String, PathBuf> = HashMap::new();
    let mut global_enum_map_after: HashMap<String, String> = HashMap::new();
    let mut global_enum_file_map_after: HashMap<String, PathBuf> = HashMap::new();
    let mut global_module_map_after: HashMap<String, String> = HashMap::new();
    let mut global_module_file_map_after: HashMap<String, PathBuf> = HashMap::new();

    for unit in &parsed_after {
        for name in &unit.function_names {
            namespace_function_files_after
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_function_map_after.insert(name.clone(), unit.namespace.clone());
            global_function_file_map_after.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.class_names {
            namespace_class_files_after
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_class_map_after.insert(name.clone(), unit.namespace.clone());
            global_class_file_map_after.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.interface_names {
            namespace_interface_files_after
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_interface_map_after.insert(name.clone(), unit.namespace.clone());
            global_interface_file_map_after.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.enum_names {
            global_enum_map_after.insert(name.clone(), unit.namespace.clone());
            global_enum_file_map_after.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.module_names {
            namespace_module_files_after
                .entry(unit.namespace.clone())
                .or_default()
                .insert(name.clone(), unit.file.clone());
            global_module_map_after.insert(name.clone(), unit.namespace.clone());
            global_module_file_map_after.insert(name.clone(), unit.file.clone());
        }
    }

    let namespace_functions_after = parsed_after.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.function_names.iter().cloned());
            acc
        },
    );
    let namespace_classes_after = parsed_after.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.class_names.iter().cloned());
            acc
        },
    );
    let namespace_modules_after = parsed_after.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.module_names.iter().cloned());
            acc
        },
    );
    let namespace_api_fingerprints_after = compute_namespace_api_fingerprints(&parsed_after);
    let file_api_fingerprints_after = parsed_after
        .iter()
        .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
        .collect::<HashMap<_, _>>();
    let rewrite_ctx_after = RewriteFingerprintContext {
        namespace_functions: &namespace_functions_after,
        global_function_map: &global_function_map_after,
        global_function_file_map: &global_function_file_map_after,
        namespace_classes: &namespace_classes_after,
        global_class_map: &global_class_map_after,
        global_class_file_map: &global_class_file_map_after,
        global_interface_map: &global_interface_map_after,
        global_interface_file_map: &global_interface_file_map_after,
        global_enum_map: &global_enum_map_after,
        global_enum_file_map: &global_enum_file_map_after,
        namespace_modules: &namespace_modules_after,
        global_module_map: &global_module_map_after,
        global_module_file_map: &global_module_file_map_after,
        namespace_api_fingerprints: &namespace_api_fingerprints_after,
        file_api_fingerprints: &file_api_fingerprints_after,
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
                function_map: &global_function_map_after,
                function_file_map: &global_function_file_map_after,
                class_map: &global_class_map_after,
                class_file_map: &global_class_file_map_after,
                interface_map: &global_interface_map_after,
                interface_file_map: &global_interface_file_map_after,
                enum_map: &global_enum_map_after,
                enum_file_map: &global_enum_file_map_after,
                module_map: &global_module_map_after,
                module_file_map: &global_module_file_map_after,
            },
        )),
    };
    let main_after = parsed_after
        .iter()
        .find(|u| u.file == main_file)
        .expect("main after");
    let rewrite_fp_after =
        compute_rewrite_context_fingerprint_for_unit(main_after, "app", &rewrite_ctx_after);

    assert_ne!(rewrite_fp_before, rewrite_fp_after);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_rewrite_fingerprint_changes_on_keyword_import_alias_target_change() {
    let temp_root = make_temp_project_root("rewrite-fp-keyword-alias-change");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.apex");
    let helper_file = src_dir.join("helper.apex");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nimport lib.Maybe.Empty as Empty;\nfunction main(x: Maybe): None { match (x) { Empty => { return None; }, _ => { return None; } } }\n",
        )
        .expect("write main");
    fs::write(
        &helper_file,
        "package lib;\nenum Maybe { Empty, Filled(value: Integer) }\n",
    )
    .expect("write helper before");

    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main before"),
        parse_project_unit(&temp_root, &helper_file).expect("parse helper before"),
    ];
    let (
        _namespace_files_map_before,
        namespace_function_files_before,
        namespace_class_files_before,
        namespace_module_files_before,
        global_function_map_before,
        global_function_file_map_before,
        global_class_map_before,
        global_class_file_map_before,
        global_enum_map_before,
        global_enum_file_map_before,
        global_module_map_before,
        global_module_file_map_before,
    ) = collect_project_symbol_maps(&parsed_before).into_parts();
    let namespace_functions_before = parsed_before.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.function_names.iter().cloned());
            acc
        },
    );
    let namespace_classes_before = parsed_before.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.class_names.iter().cloned());
            acc
        },
    );
    let namespace_modules_before = parsed_before.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.module_names.iter().cloned());
            acc
        },
    );
    let namespace_api_fingerprints_before = compute_namespace_api_fingerprints(&parsed_before);
    let file_api_fingerprints_before = parsed_before
        .iter()
        .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
        .collect::<HashMap<_, _>>();
    let rewrite_ctx_before = RewriteFingerprintContext {
        namespace_functions: &namespace_functions_before,
        global_function_map: &global_function_map_before,
        global_function_file_map: &global_function_file_map_before,
        namespace_classes: &namespace_classes_before,
        global_class_map: &global_class_map_before,
        global_class_file_map: &global_class_file_map_before,
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &global_enum_map_before,
        global_enum_file_map: &global_enum_file_map_before,
        namespace_modules: &namespace_modules_before,
        global_module_map: &global_module_map_before,
        global_module_file_map: &global_module_file_map_before,
        namespace_api_fingerprints: &namespace_api_fingerprints_before,
        file_api_fingerprints: &file_api_fingerprints_before,
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
                function_map: &global_function_map_before,
                function_file_map: &global_function_file_map_before,
                class_map: &global_class_map_before,
                class_file_map: &global_class_file_map_before,
                interface_map: empty_global_interface_map(),
                interface_file_map: empty_global_interface_file_map(),
                enum_map: &global_enum_map_before,
                enum_file_map: &global_enum_file_map_before,
                module_map: &global_module_map_before,
                module_file_map: &global_module_file_map_before,
            },
        )),
    };
    let main_before = parsed_before
        .iter()
        .find(|u| u.file == main_file)
        .expect("main before");
    let rewrite_fp_before =
        compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

    thread::sleep(Duration::from_millis(5));

    fs::write(&helper_file, "package lib;\nenum Maybe { Empty }\n").expect("write helper after");

    let parsed_after = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main after"),
        parse_project_unit(&temp_root, &helper_file).expect("parse helper after"),
    ];
    let (
        _namespace_files_map_after,
        namespace_function_files_after,
        namespace_class_files_after,
        namespace_module_files_after,
        global_function_map_after,
        global_function_file_map_after,
        global_class_map_after,
        global_class_file_map_after,
        global_enum_map_after,
        global_enum_file_map_after,
        global_module_map_after,
        global_module_file_map_after,
    ) = collect_project_symbol_maps(&parsed_after).into_parts();
    let namespace_functions_after = parsed_after.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.function_names.iter().cloned());
            acc
        },
    );
    let namespace_classes_after = parsed_after.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.class_names.iter().cloned());
            acc
        },
    );
    let namespace_modules_after = parsed_after.iter().fold(
        HashMap::<String, HashSet<String>>::new(),
        |mut acc, unit| {
            acc.entry(unit.namespace.clone())
                .or_default()
                .extend(unit.module_names.iter().cloned());
            acc
        },
    );
    let namespace_api_fingerprints_after = compute_namespace_api_fingerprints(&parsed_after);
    let file_api_fingerprints_after = parsed_after
        .iter()
        .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
        .collect::<HashMap<_, _>>();
    let rewrite_ctx_after = RewriteFingerprintContext {
        namespace_functions: &namespace_functions_after,
        global_function_map: &global_function_map_after,
        global_function_file_map: &global_function_file_map_after,
        namespace_classes: &namespace_classes_after,
        global_class_map: &global_class_map_after,
        global_class_file_map: &global_class_file_map_after,
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &global_enum_map_after,
        global_enum_file_map: &global_enum_file_map_after,
        namespace_modules: &namespace_modules_after,
        global_module_map: &global_module_map_after,
        global_module_file_map: &global_module_file_map_after,
        namespace_api_fingerprints: &namespace_api_fingerprints_after,
        file_api_fingerprints: &file_api_fingerprints_after,
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
                function_map: &global_function_map_after,
                function_file_map: &global_function_file_map_after,
                class_map: &global_class_map_after,
                class_file_map: &global_class_file_map_after,
                interface_map: empty_global_interface_map(),
                interface_file_map: empty_global_interface_file_map(),
                enum_map: &global_enum_map_after,
                enum_file_map: &global_enum_file_map_after,
                module_map: &global_module_map_after,
                module_file_map: &global_module_file_map_after,
            },
        )),
    };
    let main_after = parsed_after
        .iter()
        .find(|u| u.file == main_file)
        .expect("main after");
    let rewrite_fp_after =
        compute_rewrite_context_fingerprint_for_unit(main_after, "app", &rewrite_ctx_after);

    assert_ne!(rewrite_fp_before, rewrite_fp_after);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_rewrite_fingerprint_ignores_body_only_alias_target_change() {
    let temp_root = make_temp_project_root("rewrite-fp-alias-body-only");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.apex");
    let helper_file = src_dir.join("helper.apex");
    let helper_impl_file = src_dir.join("helper_impl.apex");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex", "src/helper_impl.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nimport lib.Maybe.Empty as Empty;\nfunction main(x: Maybe): None { match (x) { Empty => { return None; }, _ => { return None; } } }\n",
        )
        .expect("write main");
    fs::write(
            &helper_file,
            "package lib;\nenum Maybe { Empty, Filled(value: Integer) }\nfunction make(): Integer { return helper_value(); }\n",
        )
        .expect("write helper before");
    fs::write(
        &helper_impl_file,
        "package lib;\nfunction helper_value(): Integer { return 1; }\n",
    )
    .expect("write helper impl before");

    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main before"),
        parse_project_unit(&temp_root, &helper_file).expect("parse helper before"),
        parse_project_unit(&temp_root, &helper_impl_file).expect("parse helper impl before"),
    ];
    let rewrite_fp_before = rewrite_fingerprint_for_test_unit(&parsed_before, &main_file, "app");

    thread::sleep(Duration::from_millis(5));
    fs::write(
        &helper_impl_file,
        "package lib;\nfunction helper_value(): Integer { return 99; }\n",
    )
    .expect("write helper impl after");

    let parsed_after = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main after"),
        parse_project_unit(&temp_root, &helper_file).expect("parse helper after"),
        parse_project_unit(&temp_root, &helper_impl_file).expect("parse helper impl after"),
    ];
    let rewrite_fp_after = rewrite_fingerprint_for_test_unit(&parsed_after, &main_file, "app");

    assert_eq!(rewrite_fp_before, rewrite_fp_after);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_rewrite_fingerprint_changes_on_same_namespace_enum_api_change_without_import() {
    let temp_root = make_temp_project_root("rewrite-fp-same-namespace-enum-api-change");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.apex");
    let enum_file = src_dir.join("enum.apex");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/enum.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nfunction main(): Integer { return match (State.Ok(1)) { Ok(value) => value, }; }\n",
        )
        .expect("write main");
    fs::write(&enum_file, "package app;\nenum State { Ok(Integer) }\n").expect("write enum before");

    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main before"),
        parse_project_unit(&temp_root, &enum_file).expect("parse enum before"),
    ];
    let rewrite_fp_before = rewrite_fingerprint_for_test_unit(&parsed_before, &main_file, "app");

    thread::sleep(Duration::from_millis(5));
    fs::write(&enum_file, "package app;\nenum State { Ready(Integer) }\n")
        .expect("write enum after");

    let parsed_after = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main after"),
        parse_project_unit(&temp_root, &enum_file).expect("parse enum after"),
    ];
    let rewrite_fp_after = rewrite_fingerprint_for_test_unit(&parsed_after, &main_file, "app");

    assert_ne!(rewrite_fp_before, rewrite_fp_after);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_rewrite_fingerprint_ignores_body_only_change_for_alias_heavy_builtin_consumer() {
    let temp_root = make_temp_project_root("rewrite-fp-alias-heavy-builtin-body-only");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.apex");
    let helper_file = src_dir.join("helper.apex");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nimport app.Option.Some as Present;\nimport app.Option.None as Empty;\nimport app.Result.Ok as Success;\nimport app.Result.Error as Failure;\nfunction unwrap_opt(value: Option<Integer>): Integer { return match (value) { Present(inner) => inner, Empty => 0, }; }\nfunction run(flag: Boolean): Integer { result: Result<Option<Integer>, String> = make(flag); value: Option<Integer> = match (result) { Success(inner) => inner, Failure(err) => Option<Integer>(), }; return unwrap_opt(value); }\n",
        )
        .expect("write main");
    fs::write(
            &helper_file,
            "package app;\nfunction make(flag: Boolean): Result<Option<Integer>, String> { if (flag) { return Result<Option<Integer>, String>(); } return Result<Option<Integer>, String>(); }\n",
        )
        .expect("write helper before");

    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main before"),
        parse_project_unit(&temp_root, &helper_file).expect("parse helper before"),
    ];
    let rewrite_fp_before = rewrite_fingerprint_for_test_unit(&parsed_before, &main_file, "app");

    thread::sleep(Duration::from_millis(5));
    fs::write(
            &helper_file,
            "package app;\nfunction make(flag: Boolean): Result<Option<Integer>, String> { if (flag) { return Result<Option<Integer>, String>(); } return Result<Option<Integer>, String>(); }\n// body-only comment perturbation\n",
        )
        .expect("write helper after");

    let parsed_after = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main after"),
        parse_project_unit(&temp_root, &helper_file).expect("parse helper after"),
    ];
    let rewrite_fp_after = rewrite_fingerprint_for_test_unit(&parsed_after, &main_file, "app");

    assert_eq!(rewrite_fp_before, rewrite_fp_after);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn generated_project_rewrite_fingerprint_matrix_matches_expected_invalidation() {
    let body_only_variants = [
        "package lib;\nfunction foo(): Integer { return 2; }\n",
        "package lib;\nfunction foo(): Integer { return 99; }\n",
    ];
    let import_breaking_variants = [
        "package lib;\nfunction bar(): Integer { return 1; }\n",
        "package lib;\nfunction foo(x: Integer): Integer { return x; }\n",
    ];

    for helper_after in body_only_variants {
        let temp_root = make_temp_project_root("generated-rewrite-body");
        let src_dir = temp_root.join("src");
        let main_file = src_dir.join("main.apex");
        let helper_file = src_dir.join("helper.apex");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/helper.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
                &main_file,
                "package app;\nimport lib.foo;\nfunction main(): None { value: Integer = foo(); return None; }\n",
            )
            .expect("write main");
        fs::write(
            &helper_file,
            "package lib;\nfunction foo(): Integer { return 1; }\n",
        )
        .expect("write helper");

        let parsed_before = vec![
            parse_project_unit(&temp_root, &main_file).expect("parse main before"),
            parse_project_unit(&temp_root, &helper_file).expect("parse helper before"),
        ];
        let (
            _namespace_files_map_before,
            namespace_function_files_before,
            namespace_class_files_before,
            namespace_module_files_before,
            global_function_map_before,
            global_function_file_map_before,
            global_class_map_before,
            global_class_file_map_before,
            global_enum_map_before,
            global_enum_file_map_before,
            global_module_map_before,
            global_module_file_map_before,
        ) = collect_project_symbol_maps(&parsed_before).into_parts();
        let namespace_functions_before = parsed_before.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.function_names.iter().cloned());
                acc
            },
        );
        let namespace_classes_before = parsed_before.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.class_names.iter().cloned());
                acc
            },
        );
        let namespace_modules_before = parsed_before.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.module_names.iter().cloned());
                acc
            },
        );
        let namespace_api_fingerprints_before = compute_namespace_api_fingerprints(&parsed_before);
        let file_api_fingerprints_before = parsed_before
            .iter()
            .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
            .collect::<HashMap<_, _>>();
        let rewrite_ctx_before = RewriteFingerprintContext {
            namespace_functions: &namespace_functions_before,
            global_function_map: &global_function_map_before,
            global_function_file_map: &global_function_file_map_before,
            namespace_classes: &namespace_classes_before,
            global_class_map: &global_class_map_before,
            global_class_file_map: &global_class_file_map_before,
            global_interface_map: empty_global_interface_map(),
            global_interface_file_map: empty_global_interface_file_map(),
            global_enum_map: &global_enum_map_before,
            global_enum_file_map: &global_enum_file_map_before,
            namespace_modules: &namespace_modules_before,
            global_module_map: &global_module_map_before,
            global_module_file_map: &global_module_file_map_before,
            namespace_api_fingerprints: &namespace_api_fingerprints_before,
            file_api_fingerprints: &file_api_fingerprints_before,
            symbol_lookup: Arc::new(build_project_symbol_lookup(
                &crate::dependency::ProjectSymbolMaps {
                    function_map: &global_function_map_before,
                    function_file_map: &global_function_file_map_before,
                    class_map: &global_class_map_before,
                    class_file_map: &global_class_file_map_before,
                    interface_map: empty_global_interface_map(),
                    interface_file_map: empty_global_interface_file_map(),
                    enum_map: &global_enum_map_before,
                    enum_file_map: &global_enum_file_map_before,
                    module_map: &global_module_map_before,
                    module_file_map: &global_module_file_map_before,
                },
            )),
        };
        let main_before = parsed_before
            .iter()
            .find(|u| u.file == main_file)
            .expect("main");
        let rewrite_fp_before =
            compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

        fs::write(&helper_file, helper_after).expect("rewrite helper body variant");
        let parsed_after = vec![
            parse_project_unit(&temp_root, &main_file).expect("parse main after"),
            parse_project_unit(&temp_root, &helper_file).expect("parse helper after"),
        ];
        let (
            _namespace_files_map_after,
            namespace_function_files_after,
            namespace_class_files_after,
            namespace_module_files_after,
            global_function_map_after,
            global_function_file_map_after,
            global_class_map_after,
            global_class_file_map_after,
            global_enum_map_after,
            global_enum_file_map_after,
            global_module_map_after,
            global_module_file_map_after,
        ) = collect_project_symbol_maps(&parsed_after).into_parts();
        let namespace_functions_after = parsed_after.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.function_names.iter().cloned());
                acc
            },
        );
        let namespace_classes_after = parsed_after.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.class_names.iter().cloned());
                acc
            },
        );
        let namespace_modules_after = parsed_after.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.module_names.iter().cloned());
                acc
            },
        );
        let namespace_api_fingerprints_after = compute_namespace_api_fingerprints(&parsed_after);
        let file_api_fingerprints_after = parsed_after
            .iter()
            .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
            .collect::<HashMap<_, _>>();
        let rewrite_ctx_after = RewriteFingerprintContext {
            namespace_functions: &namespace_functions_after,
            global_function_map: &global_function_map_after,
            global_function_file_map: &global_function_file_map_after,
            namespace_classes: &namespace_classes_after,
            global_class_map: &global_class_map_after,
            global_class_file_map: &global_class_file_map_after,
            global_interface_map: empty_global_interface_map(),
            global_interface_file_map: empty_global_interface_file_map(),
            global_enum_map: &global_enum_map_after,
            global_enum_file_map: &global_enum_file_map_after,
            namespace_modules: &namespace_modules_after,
            global_module_map: &global_module_map_after,
            global_module_file_map: &global_module_file_map_after,
            namespace_api_fingerprints: &namespace_api_fingerprints_after,
            file_api_fingerprints: &file_api_fingerprints_after,
            symbol_lookup: Arc::new(build_project_symbol_lookup(
                &crate::dependency::ProjectSymbolMaps {
                    function_map: &global_function_map_after,
                    function_file_map: &global_function_file_map_after,
                    class_map: &global_class_map_after,
                    class_file_map: &global_class_file_map_after,
                    interface_map: empty_global_interface_map(),
                    interface_file_map: empty_global_interface_file_map(),
                    enum_map: &global_enum_map_after,
                    enum_file_map: &global_enum_file_map_after,
                    module_map: &global_module_map_after,
                    module_file_map: &global_module_file_map_after,
                },
            )),
        };
        let main_after = parsed_after
            .iter()
            .find(|u| u.file == main_file)
            .expect("main");
        let rewrite_fp_after =
            compute_rewrite_context_fingerprint_for_unit(main_after, "app", &rewrite_ctx_after);

        assert_eq!(rewrite_fp_before, rewrite_fp_after);
        let _ = fs::remove_dir_all(temp_root);
    }

    for helper_after in import_breaking_variants {
        let temp_root = make_temp_project_root("generated-rewrite-api");
        let src_dir = temp_root.join("src");
        let main_file = src_dir.join("main.apex");
        let helper_file = src_dir.join("helper.apex");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/helper.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
                &main_file,
                "package app;\nimport lib.foo;\nfunction main(): None { value: Integer = foo(); return None; }\n",
            )
            .expect("write main");
        fs::write(
            &helper_file,
            "package lib;\nfunction foo(): Integer { return 1; }\n",
        )
        .expect("write helper");

        let parsed_before = vec![
            parse_project_unit(&temp_root, &main_file).expect("parse main before"),
            parse_project_unit(&temp_root, &helper_file).expect("parse helper before"),
        ];
        let (
            _namespace_files_map_before,
            namespace_function_files_before,
            namespace_class_files_before,
            namespace_module_files_before,
            global_function_map_before,
            global_function_file_map_before,
            global_class_map_before,
            global_class_file_map_before,
            global_enum_map_before,
            global_enum_file_map_before,
            global_module_map_before,
            global_module_file_map_before,
        ) = collect_project_symbol_maps(&parsed_before).into_parts();
        let namespace_functions_before = parsed_before.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.function_names.iter().cloned());
                acc
            },
        );
        let namespace_classes_before = parsed_before.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.class_names.iter().cloned());
                acc
            },
        );
        let namespace_modules_before = parsed_before.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.module_names.iter().cloned());
                acc
            },
        );
        let namespace_api_fingerprints_before = compute_namespace_api_fingerprints(&parsed_before);
        let file_api_fingerprints_before = parsed_before
            .iter()
            .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
            .collect::<HashMap<_, _>>();
        let rewrite_ctx_before = RewriteFingerprintContext {
            namespace_functions: &namespace_functions_before,
            global_function_map: &global_function_map_before,
            global_function_file_map: &global_function_file_map_before,
            namespace_classes: &namespace_classes_before,
            global_class_map: &global_class_map_before,
            global_class_file_map: &global_class_file_map_before,
            global_interface_map: empty_global_interface_map(),
            global_interface_file_map: empty_global_interface_file_map(),
            global_enum_map: &global_enum_map_before,
            global_enum_file_map: &global_enum_file_map_before,
            namespace_modules: &namespace_modules_before,
            global_module_map: &global_module_map_before,
            global_module_file_map: &global_module_file_map_before,
            namespace_api_fingerprints: &namespace_api_fingerprints_before,
            file_api_fingerprints: &file_api_fingerprints_before,
            symbol_lookup: Arc::new(build_project_symbol_lookup(
                &crate::dependency::ProjectSymbolMaps {
                    function_map: &global_function_map_before,
                    function_file_map: &global_function_file_map_before,
                    class_map: &global_class_map_before,
                    class_file_map: &global_class_file_map_before,
                    interface_map: empty_global_interface_map(),
                    interface_file_map: empty_global_interface_file_map(),
                    enum_map: &global_enum_map_before,
                    enum_file_map: &global_enum_file_map_before,
                    module_map: &global_module_map_before,
                    module_file_map: &global_module_file_map_before,
                },
            )),
        };
        let main_before = parsed_before
            .iter()
            .find(|u| u.file == main_file)
            .expect("main");
        let rewrite_fp_before =
            compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

        fs::write(&helper_file, helper_after).expect("rewrite helper api variant");
        let parsed_after = vec![
            parse_project_unit(&temp_root, &main_file).expect("parse main after"),
            parse_project_unit(&temp_root, &helper_file).expect("parse helper after"),
        ];
        let (
            _namespace_files_map_after,
            namespace_function_files_after,
            namespace_class_files_after,
            namespace_module_files_after,
            global_function_map_after,
            global_function_file_map_after,
            global_class_map_after,
            global_class_file_map_after,
            global_enum_map_after,
            global_enum_file_map_after,
            global_module_map_after,
            global_module_file_map_after,
        ) = collect_project_symbol_maps(&parsed_after).into_parts();
        let namespace_functions_after = parsed_after.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.function_names.iter().cloned());
                acc
            },
        );
        let namespace_classes_after = parsed_after.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.class_names.iter().cloned());
                acc
            },
        );
        let namespace_modules_after = parsed_after.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.module_names.iter().cloned());
                acc
            },
        );
        let namespace_api_fingerprints_after = compute_namespace_api_fingerprints(&parsed_after);
        let file_api_fingerprints_after = parsed_after
            .iter()
            .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
            .collect::<HashMap<_, _>>();
        let rewrite_ctx_after = RewriteFingerprintContext {
            namespace_functions: &namespace_functions_after,
            global_function_map: &global_function_map_after,
            global_function_file_map: &global_function_file_map_after,
            namespace_classes: &namespace_classes_after,
            global_class_map: &global_class_map_after,
            global_class_file_map: &global_class_file_map_after,
            global_interface_map: empty_global_interface_map(),
            global_interface_file_map: empty_global_interface_file_map(),
            global_enum_map: &global_enum_map_after,
            global_enum_file_map: &global_enum_file_map_after,
            namespace_modules: &namespace_modules_after,
            global_module_map: &global_module_map_after,
            global_module_file_map: &global_module_file_map_after,
            namespace_api_fingerprints: &namespace_api_fingerprints_after,
            file_api_fingerprints: &file_api_fingerprints_after,
            symbol_lookup: Arc::new(build_project_symbol_lookup(
                &crate::dependency::ProjectSymbolMaps {
                    function_map: &global_function_map_after,
                    function_file_map: &global_function_file_map_after,
                    class_map: &global_class_map_after,
                    class_file_map: &global_class_file_map_after,
                    interface_map: empty_global_interface_map(),
                    interface_file_map: empty_global_interface_file_map(),
                    enum_map: &global_enum_map_after,
                    enum_file_map: &global_enum_file_map_after,
                    module_map: &global_module_map_after,
                    module_file_map: &global_module_file_map_after,
                },
            )),
        };
        let main_after = parsed_after
            .iter()
            .find(|u| u.file == main_file)
            .expect("main");
        let rewrite_fp_after =
            compute_rewrite_context_fingerprint_for_unit(main_after, "app", &rewrite_ctx_after);

        assert_ne!(rewrite_fp_before, rewrite_fp_after);
        let _ = fs::remove_dir_all(temp_root);
    }
}

fn make_unit(file: &str, namespace: &str, imports: &[&str]) -> ParsedProjectUnit {
    ParsedProjectUnit {
        file: PathBuf::from(file),
        namespace: namespace.to_string(),
        program: Program {
            package: Some(namespace.to_string()),
            declarations: Vec::new(),
        },
        imports: imports
            .iter()
            .map(|path| ImportDecl {
                path: (*path).to_string(),
                alias: None,
            })
            .collect(),
        api_fingerprint: "api".to_string(),
        semantic_fingerprint: "sem".to_string(),
        import_check_fingerprint: "import".to_string(),
        function_names: Vec::new(),
        class_names: Vec::new(),
        interface_names: Vec::new(),
        enum_names: Vec::new(),
        module_names: Vec::new(),
        referenced_symbols: Vec::new(),
        qualified_symbol_refs: Vec::new(),
        api_referenced_symbols: Vec::new(),
        from_parse_cache: false,
    }
}

fn empty_global_interface_map() -> &'static HashMap<String, String> {
    static EMPTY: OnceLock<HashMap<String, String>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

fn empty_global_interface_file_map() -> &'static HashMap<String, PathBuf> {
    static EMPTY: OnceLock<HashMap<String, PathBuf>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

#[test]
fn rewrite_context_for_specific_import_ignores_unrelated_namespace_api_changes() {
    let unit = make_unit("src/main.apex", "app", &["lib.foo"]);

    let namespace_functions = HashMap::from([(
        "lib".to_string(),
        HashSet::from(["foo".to_string(), "bar".to_string()]),
    )]);
    let global_function_map = HashMap::from([
        ("foo".to_string(), "lib".to_string()),
        ("bar".to_string(), "lib".to_string()),
    ]);
    let global_function_file_map = HashMap::from([
        ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
        ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
    ]);
    let namespace_function_files = HashMap::from([(
        "lib".to_string(),
        HashMap::from([
            ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
        ]),
    )]);
    let namespace_classes = HashMap::new();
    let namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let global_class_map = HashMap::new();
    let global_class_file_map = HashMap::new();
    let global_enum_map = HashMap::new();
    let global_enum_file_map = HashMap::new();
    let namespace_modules = HashMap::new();
    let namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let global_module_map = HashMap::new();
    let global_module_file_map = HashMap::new();
    let namespace_api_fingerprints = HashMap::from([("lib".to_string(), "ns-v1".to_string())]);
    let file_api_fingerprints = HashMap::from([
        (PathBuf::from("src/lib_foo.apex"), "file-foo-v1".to_string()),
        (PathBuf::from("src/lib_bar.apex"), "file-bar-v1".to_string()),
    ]);
    let ctx_a = RewriteFingerprintContext {
        namespace_functions: &namespace_functions,
        global_function_map: &global_function_map,
        global_function_file_map: &global_function_file_map,
        namespace_classes: &namespace_classes,
        global_class_map: &global_class_map,
        global_class_file_map: &global_class_file_map,
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &global_enum_map,
        global_enum_file_map: &global_enum_file_map,
        namespace_modules: &namespace_modules,
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
        namespace_api_fingerprints: &namespace_api_fingerprints,
        file_api_fingerprints: &file_api_fingerprints,
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
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
            },
        )),
    };

    let fp_a = compute_rewrite_context_fingerprint_for_unit(&unit, "app", &ctx_a);
    let namespace_api_fingerprints_b = HashMap::from([("lib".to_string(), "ns-v2".to_string())]);
    let file_api_fingerprints_b = HashMap::from([
        (PathBuf::from("src/lib_foo.apex"), "file-foo-v1".to_string()),
        (PathBuf::from("src/lib_bar.apex"), "file-bar-v2".to_string()),
    ]);
    let ctx_b = RewriteFingerprintContext {
        namespace_functions: &namespace_functions,
        global_function_map: &global_function_map,
        global_function_file_map: &global_function_file_map,
        namespace_classes: &namespace_classes,
        global_class_map: &global_class_map,
        global_class_file_map: &global_class_file_map,
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &global_enum_map,
        global_enum_file_map: &global_enum_file_map,
        namespace_modules: &namespace_modules,
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
        namespace_api_fingerprints: &namespace_api_fingerprints_b,
        file_api_fingerprints: &file_api_fingerprints_b,
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
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
            },
        )),
    };
    let fp_b = compute_rewrite_context_fingerprint_for_unit(&unit, "app", &ctx_b);

    assert_eq!(fp_a, fp_b);
}

#[test]
fn rewrite_context_for_wildcard_import_tracks_namespace_api_changes() {
    let unit = make_unit("src/main.apex", "app", &["lib.*"]);

    let namespace_functions = HashMap::from([(
        "lib".to_string(),
        HashSet::from(["foo".to_string(), "bar".to_string()]),
    )]);
    let global_function_map = HashMap::from([
        ("foo".to_string(), "lib".to_string()),
        ("bar".to_string(), "lib".to_string()),
    ]);
    let global_function_file_map = HashMap::from([
        ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
        ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
    ]);
    let namespace_function_files = HashMap::from([(
        "lib".to_string(),
        HashMap::from([
            ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
        ]),
    )]);
    let namespace_classes = HashMap::new();
    let namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let global_class_map = HashMap::new();
    let global_class_file_map = HashMap::new();
    let global_enum_map = HashMap::new();
    let global_enum_file_map = HashMap::new();
    let namespace_modules = HashMap::new();
    let namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let global_module_map = HashMap::new();
    let global_module_file_map = HashMap::new();
    let namespace_api_fingerprints_a = HashMap::from([("lib".to_string(), "ns-v1".to_string())]);
    let ctx_a = RewriteFingerprintContext {
        namespace_functions: &namespace_functions,
        global_function_map: &global_function_map,
        global_function_file_map: &global_function_file_map,
        namespace_classes: &namespace_classes,
        global_class_map: &global_class_map,
        global_class_file_map: &global_class_file_map,
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &global_enum_map,
        global_enum_file_map: &global_enum_file_map,
        namespace_modules: &namespace_modules,
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
        namespace_api_fingerprints: &namespace_api_fingerprints_a,
        file_api_fingerprints: &HashMap::new(),
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
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
            },
        )),
    };
    let fp_a = compute_rewrite_context_fingerprint_for_unit(&unit, "app", &ctx_a);
    let namespace_api_fingerprints_b = HashMap::from([("lib".to_string(), "ns-v2".to_string())]);
    let ctx_b = RewriteFingerprintContext {
        namespace_functions: &namespace_functions,
        global_function_map: &global_function_map,
        global_function_file_map: &global_function_file_map,
        namespace_classes: &namespace_classes,
        global_class_map: &global_class_map,
        global_class_file_map: &global_class_file_map,
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &global_enum_map,
        global_enum_file_map: &global_enum_file_map,
        namespace_modules: &namespace_modules,
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
        namespace_api_fingerprints: &namespace_api_fingerprints_b,
        file_api_fingerprints: &HashMap::new(),
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
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
            },
        )),
    };
    let fp_b = compute_rewrite_context_fingerprint_for_unit(&unit, "app", &ctx_b);

    assert_ne!(fp_a, fp_b);
}

#[test]
fn dependency_graph_tracks_specific_symbol_owner_file_only() {
    let app = make_unit("src/main.apex", "app", &["lib.foo"]);
    let foo = make_unit("src/lib_foo.apex", "lib", &[]);
    let bar = make_unit("src/lib_bar.apex", "lib", &[]);
    let parsed_files = vec![app.clone(), foo, bar];
    let namespace_files_map = HashMap::from([
        ("app".to_string(), vec![PathBuf::from("src/main.apex")]),
        (
            "lib".to_string(),
            vec![
                PathBuf::from("src/lib_bar.apex"),
                PathBuf::from("src/lib_foo.apex"),
            ],
        ),
    ]);

    let global_function_map = HashMap::from([
        ("foo".to_string(), "lib".to_string()),
        ("bar".to_string(), "lib".to_string()),
    ]);
    let global_function_file_map = HashMap::from([
        ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
        ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
    ]);
    let global_class_map = HashMap::new();
    let global_class_file_map = HashMap::new();
    let global_enum_map = HashMap::new();
    let global_enum_file_map = HashMap::new();
    let global_module_map = HashMap::new();
    let global_module_file_map = HashMap::new();
    let namespace_function_files = HashMap::from([(
        "lib".to_string(),
        HashMap::from([
            ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
        ]),
    )]);
    let namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
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
            },
        )),
    };
    let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None, None);

    assert_eq!(
        graph.get(&app.file).cloned().unwrap_or_default(),
        HashSet::from([PathBuf::from("src/lib_foo.apex")])
    );
}

#[test]
fn dependency_graph_tracks_same_namespace_symbol_references() {
    let mut app = make_unit("src/app.apex", "app", &[]);
    app.referenced_symbols = vec!["helper".to_string()];
    let mut helper = make_unit("src/helper.apex", "app", &[]);
    helper.function_names = vec!["helper".to_string()];
    let parsed_files = vec![app.clone(), helper.clone()];
    let namespace_files_map = HashMap::from([(
        "app".to_string(),
        vec![
            PathBuf::from("src/app.apex"),
            PathBuf::from("src/helper.apex"),
        ],
    )]);
    let namespace_function_files = HashMap::from([(
        "app".to_string(),
        HashMap::from([("helper".to_string(), PathBuf::from("src/helper.apex"))]),
    )]);
    let namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let global_function_map = HashMap::from([("helper".to_string(), "app".to_string())]);
    let global_function_file_map =
        HashMap::from([("helper".to_string(), PathBuf::from("src/helper.apex"))]);
    let global_class_map = HashMap::new();
    let global_class_file_map = HashMap::new();
    let global_enum_map = HashMap::new();
    let global_enum_file_map = HashMap::new();
    let global_module_map = HashMap::new();
    let global_module_file_map = HashMap::new();
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
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
            },
        )),
    };

    let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None, None);

    assert_eq!(
        graph.get(&app.file).cloned().unwrap_or_default(),
        HashSet::from([PathBuf::from("src/helper.apex")])
    );
    assert!(graph
        .get(&helper.file)
        .cloned()
        .unwrap_or_default()
        .is_empty());
}

#[test]
fn dependency_graph_limits_wildcard_imports_to_used_owner_files() {
    let mut app = make_unit("src/main.apex", "app", &["lib.*"]);
    app.referenced_symbols = vec!["foo".to_string()];
    let mut foo = make_unit("src/lib_foo.apex", "lib", &[]);
    foo.function_names = vec!["foo".to_string()];
    let mut bar = make_unit("src/lib_bar.apex", "lib", &[]);
    bar.function_names = vec!["bar".to_string()];
    let parsed_files = vec![app.clone(), foo, bar];
    let namespace_files_map = HashMap::from([
        ("app".to_string(), vec![PathBuf::from("src/main.apex")]),
        (
            "lib".to_string(),
            vec![
                PathBuf::from("src/lib_bar.apex"),
                PathBuf::from("src/lib_foo.apex"),
            ],
        ),
    ]);
    let namespace_function_files = HashMap::from([(
        "lib".to_string(),
        HashMap::from([
            ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
        ]),
    )]);
    let ctx = DependencyResolutionContext {
        namespace_files_map: &namespace_files_map,
        global_function_map: &HashMap::from([
            ("foo".to_string(), "lib".to_string()),
            ("bar".to_string(), "lib".to_string()),
        ]),
        global_function_file_map: &HashMap::from([
            ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
        ]),
        global_class_map: &HashMap::new(),
        global_class_file_map: &HashMap::new(),
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &HashMap::new(),
        global_enum_file_map: &HashMap::new(),
        global_module_map: &HashMap::new(),
        global_module_file_map: &HashMap::new(),
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
                function_map: &HashMap::from([
                    ("foo".to_string(), "lib".to_string()),
                    ("bar".to_string(), "lib".to_string()),
                ]),
                function_file_map: &HashMap::from([
                    ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
                    ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
                ]),
                class_map: &HashMap::new(),
                class_file_map: &HashMap::new(),
                interface_map: empty_global_interface_map(),
                interface_file_map: empty_global_interface_file_map(),
                enum_map: &HashMap::new(),
                enum_file_map: &HashMap::new(),
                module_map: &HashMap::new(),
                module_file_map: &HashMap::new(),
            },
        )),
    };

    let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None, None);
    assert_eq!(
        graph.get(&app.file).cloned().unwrap_or_default(),
        HashSet::from([PathBuf::from("src/lib_foo.apex")])
    );
}

#[test]
fn dependency_graph_keeps_wildcard_namespace_dependencies_when_symbol_disappears() {
    let mut app = make_unit("src/main.apex", "app", &["lib.*"]);
    app.referenced_symbols = vec!["foo".to_string()];
    let mut foo = make_unit("src/lib_foo.apex", "lib", &[]);
    foo.function_names = vec!["other".to_string()];
    let mut bar = make_unit("src/lib_bar.apex", "lib", &[]);
    bar.function_names = vec!["bar".to_string()];
    let parsed_files = vec![app.clone(), foo, bar];
    let namespace_files_map = HashMap::from([
        ("app".to_string(), vec![PathBuf::from("src/main.apex")]),
        (
            "lib".to_string(),
            vec![
                PathBuf::from("src/lib_bar.apex"),
                PathBuf::from("src/lib_foo.apex"),
            ],
        ),
    ]);
    let namespace_function_files = HashMap::from([(
        "lib".to_string(),
        HashMap::from([
            ("other".to_string(), PathBuf::from("src/lib_foo.apex")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
        ]),
    )]);
    let ctx = DependencyResolutionContext {
        namespace_files_map: &namespace_files_map,
        global_function_map: &HashMap::from([
            ("other".to_string(), "lib".to_string()),
            ("bar".to_string(), "lib".to_string()),
        ]),
        global_function_file_map: &HashMap::from([
            ("other".to_string(), PathBuf::from("src/lib_foo.apex")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
        ]),
        global_class_map: &HashMap::new(),
        global_class_file_map: &HashMap::new(),
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &HashMap::new(),
        global_enum_file_map: &HashMap::new(),
        global_module_map: &HashMap::new(),
        global_module_file_map: &HashMap::new(),
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
                function_map: &HashMap::from([
                    ("other".to_string(), "lib".to_string()),
                    ("bar".to_string(), "lib".to_string()),
                ]),
                function_file_map: &HashMap::from([
                    ("other".to_string(), PathBuf::from("src/lib_foo.apex")),
                    ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
                ]),
                class_map: &HashMap::new(),
                class_file_map: &HashMap::new(),
                interface_map: empty_global_interface_map(),
                interface_file_map: empty_global_interface_file_map(),
                enum_map: &HashMap::new(),
                enum_file_map: &HashMap::new(),
                module_map: &HashMap::new(),
                module_file_map: &HashMap::new(),
            },
        )),
    };

    let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None, None);
    assert_eq!(
        graph.get(&app.file).cloned().unwrap_or_default(),
        HashSet::from([
            PathBuf::from("src/lib_bar.apex"),
            PathBuf::from("src/lib_foo.apex")
        ])
    );
}

#[test]
fn dependency_graph_keeps_nested_module_wildcard_namespace_dependencies_when_symbol_disappears() {
    let mut app = make_unit("src/main.apex", "app", &["app.U.*"]);
    app.referenced_symbols = vec!["id".to_string()];
    let mut helper = make_unit("src/helper.apex", "app", &[]);
    helper.module_names = vec!["U".to_string()];
    let parsed_files = vec![app.clone(), helper];
    let namespace_files_map = HashMap::from([
        (
            "app".to_string(),
            vec![
                PathBuf::from("src/helper.apex"),
                PathBuf::from("src/main.apex"),
            ],
        ),
        ("app.U".to_string(), vec![PathBuf::from("src/helper.apex")]),
    ]);
    let namespace_module_files = HashMap::from([(
        "app".to_string(),
        HashMap::from([("U".to_string(), PathBuf::from("src/helper.apex"))]),
    )]);
    let global_module_map = HashMap::from([("U".to_string(), "app".to_string())]);
    let global_module_file_map =
        HashMap::from([("U".to_string(), PathBuf::from("src/helper.apex"))]);
    let ctx = DependencyResolutionContext {
        namespace_files_map: &namespace_files_map,
        global_function_map: &HashMap::new(),
        global_function_file_map: &HashMap::new(),
        global_class_map: &HashMap::new(),
        global_class_file_map: &HashMap::new(),
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &HashMap::new(),
        global_enum_file_map: &HashMap::new(),
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
                function_map: &HashMap::new(),
                function_file_map: &HashMap::new(),
                class_map: &HashMap::new(),
                class_file_map: &HashMap::new(),
                interface_map: empty_global_interface_map(),
                interface_file_map: empty_global_interface_file_map(),
                enum_map: &HashMap::new(),
                enum_file_map: &HashMap::new(),
                module_map: &global_module_map,
                module_file_map: &global_module_file_map,
            },
        )),
    };

    let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None, None);
    assert_eq!(
        graph.get(&app.file).cloned().unwrap_or_default(),
        HashSet::from([PathBuf::from("src/helper.apex")])
    );
}

#[test]
fn parsed_dependency_graph_tracks_nested_module_wildcard_import_owner_files() {
    let temp_root = make_temp_project_root("nested-module-wildcard-dependency-graph");
    let main_file = temp_root.join("src/main.apex");
    let helper_file = temp_root.join("src/helper.apex");
    fs::write(
        &main_file,
        "package app;\nimport app.U.*;\nfunction main(): Integer { return id(1); }\n",
    )
    .expect("write main");
    fs::write(
        &helper_file,
        "package app;\nmodule U { function id(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("write helper");

    let parsed_files = vec![
        parse_project_unit(&temp_root, &main_file).expect("parse main"),
        parse_project_unit(&temp_root, &helper_file).expect("parse helper"),
    ];
    let (
        namespace_files_map,
        namespace_function_files,
        namespace_class_files,
        namespace_module_files,
        global_function_map,
        global_function_file_map,
        global_class_map,
        global_class_file_map,
        global_enum_map,
        global_enum_file_map,
        global_module_map,
        global_module_file_map,
    ) = collect_project_symbol_maps(&parsed_files).into_parts();
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
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
            },
        )),
    };

    let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None, None);
    assert_eq!(
        graph.get(&main_file).cloned().unwrap_or_default(),
        HashSet::from([helper_file.clone()])
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rechecks_wildcard_import_dependents_after_symbol_removal() {
    let temp_root = make_temp_project_root("project-build-wildcard-symbol-removal");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.apex"),
        "package app;\nimport lib.*;\nfunction main(): Integer { return add(1); }\n",
    )
    .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial wildcard project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction other(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("rewrite helper without imported symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after wildcard-imported symbol removal");
        assert!(
            err.contains("Wildcard import 'lib.*' no longer provides 'add'")
                || err.contains("Import check failed"),
            "{err}"
        );
        assert!(!err.contains("Undefined variable: add"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rechecks_nested_module_wildcard_import_dependents_after_symbol_removal() {
    let temp_root = make_temp_project_root("project-build-nested-module-wildcard-symbol-removal");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.apex"),
        "package app;\nimport app.U.*;\nfunction main(): Integer { return id(1); }\n",
    )
    .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function id(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial nested module wildcard project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function plus(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("rewrite helper without nested-module imported symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after nested-module wildcard-imported symbol removal");
        assert!(
            err.contains("Wildcard import 'app.U.*' no longer provides 'id'")
                || err.contains("Import check failed"),
            "{err}"
        );
        assert!(!err.contains("Undefined variable: id"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_user_facing_error_for_stale_nested_module_wildcard_generic_function_value()
{
    let temp_root = make_temp_project_root("project-build-nested-module-wildcard-generic-fn-value");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport app.U.*;\nfunction main(): Integer { f: (Integer) -> Integer = id<Integer>; return f(1); }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function id<T>(x: T): T { return x; } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial nested module wildcard generic function value build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function plus<T>(x: T): T { return x; } }\n",
    )
    .expect("rewrite helper without nested-module wildcard generic symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after nested-module wildcard generic symbol removal");
        assert!(
            err.contains("Wildcard import 'app.U.*' no longer provides 'id'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Undefined variable: id"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rechecks_nested_exact_import_alias_dependents_after_symbol_removal() {
    let temp_root =
        make_temp_project_root("project-build-nested-exact-import-alias-symbol-removal");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.apex"),
        "package app;\nimport app.U.id as ident;\nfunction main(): Integer { return ident(1); }\n",
    )
    .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function id(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial nested exact-import alias project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function plus(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("rewrite helper without nested exact-import alias symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after nested exact-import alias symbol removal");
        assert!(
            err.contains("Imported alias 'ident' no longer resolves")
                || err.contains("Import check failed"),
            "{err}"
        );
        assert!(!err.contains("Undefined variable: ident"), "{err}");
        assert!(!err.contains("app__U__id"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_user_facing_error_for_stale_nested_exact_import_alias_function_value() {
    let temp_root =
        make_temp_project_root("project-build-stale-nested-exact-import-alias-function-value");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport app.U.id as ident;\nfunction main(): Integer {\n    f: (Integer) -> Integer = ident;\n    return f(1);\n}\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function id(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial stale nested exact-import alias function-value build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function plus(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("rewrite helper without nested exact-import alias function-value symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).expect_err(
            "build should fail after stale nested exact-import alias function-value symbol removal",
        );
        assert!(
            err.contains("Imported alias 'ident' no longer resolves"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Undefined variable: ident"), "{err}");
        assert!(!err.contains("app__U__id"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rechecks_namespace_alias_dependents_after_symbol_removal() {
    let temp_root = make_temp_project_root("project-build-namespace-alias-symbol-removal");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.apex"),
        "package app;\nimport lib as l;\nfunction main(): Integer { return l.add(1); }\n",
    )
    .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial namespace-alias project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction other(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("rewrite helper without namespace-alias symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after namespace-alias symbol removal");
        assert!(
            err.contains("Imported namespace alias 'l' has no member 'add'")
                || err.contains("Import check failed"),
            "{err}"
        );
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_user_facing_error_for_stale_root_namespace_alias_call() {
    let temp_root = make_temp_project_root("project-build-stale-root-namespace-alias-call");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.apex"),
        "package app;\nimport app as root;\nfunction main(): Integer { return root.U.id(1); }\n",
    )
    .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function id(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial stale root namespace alias call build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function plus(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("rewrite helper without stale root namespace alias call symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after stale root namespace alias call symbol removal");
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'U.id'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Undefined variable: root"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_user_facing_error_for_stale_root_namespace_alias_nested_enum_variant() {
    let temp_root =
        make_temp_project_root("project-build-stale-root-namespace-alias-nested-enum-variant");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport app as root;\nfunction main(): Integer { return match (root.M.E.A(5)) { root.M.E.A(v) => v, root.M.E.B(v) => v, }; }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule M { enum E { A(Integer), B(Integer) } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial root namespace alias nested enum build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule M { enum E { C(Integer), D(Integer) } }\n",
    )
    .expect("rewrite helper without nested enum alias variant");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).expect_err(
            "build should fail after stale root namespace alias nested enum variant removal",
        );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'M.E.A'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Undefined variable: app__M__E"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_user_facing_error_for_stale_exact_import_alias_generic_function_value() {
    let temp_root =
        make_temp_project_root("project-build-stale-exact-import-alias-generic-fn-value");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport app.U.id as ident;\nfunction main(): Integer { f: (Integer) -> Integer = ident<Integer>; return f(7); }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function id<T>(value: T): T { return value; } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial exact-import alias generic function value build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function other<T>(value: T): T { return value; } }\n",
    )
    .expect("rewrite helper without exact-import alias generic symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).expect_err(
            "build should fail after stale exact-import alias generic function value removal",
        );
        assert!(
            err.contains("Imported alias 'ident' no longer resolves"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Undefined variable: ident"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_user_facing_error_for_stale_exact_imported_nested_enum_variant_alias() {
    let temp_root =
        make_temp_project_root("project-build-stale-exact-imported-nested-enum-variant-alias");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport app.M.E.B as Variant;\nfunction main(): None { e: M.E = Variant(2); match (e) { Variant(v) => { require(v == 2); } M.E.A(v) => { require(false); } } return None; }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule M { enum E { A(Integer) B(Integer) } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial exact imported nested enum variant alias build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule M { enum E { C(Integer) D(Integer) } }\n",
    )
    .expect("rewrite helper without exact imported nested enum variant alias");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).expect_err(
            "build should fail after stale exact imported nested enum variant alias removal",
        );
        assert!(
            err.contains("Imported alias 'Variant' no longer resolves"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Undefined variable: app__M__E"), "{err}");
        assert!(!err.contains("Unknown variant 'app__M__E.B'"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_user_facing_error_for_stale_exact_imported_nested_enum_alias_type() {
    let temp_root =
        make_temp_project_root("project-build-stale-exact-imported-nested-enum-alias-type");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport app.M.E as Enum;\nfunction main(): Integer { value: Enum = Enum.B(2); return 0; }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule M { enum E { A(Integer), B(Integer) } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial exact imported nested enum alias type build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule M { enum F { A(Integer), B(Integer) } }\n",
    )
    .expect("rewrite helper without exact imported nested enum alias type");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).expect_err(
            "build should fail after stale exact imported nested enum alias type removal",
        );
        assert!(
            err.contains("Imported alias 'Enum' no longer resolves"),
            "{err}"
        );
        assert_eq!(
            err.matches("Imported alias 'Enum' no longer resolves")
                .count(),
            1,
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Undefined variable: app__M__E"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_user_facing_error_for_stale_wildcard_imported_nested_enum_type() {
    let temp_root =
        make_temp_project_root("project-build-stale-wildcard-imported-nested-enum-type");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport app.M.*;\nfunction main(): Integer { value: E = E.B(2); return 0; }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule M { enum E { A(Integer), B(Integer) } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial wildcard imported nested enum type build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule M { enum F { A(Integer), B(Integer) } }\n",
    )
    .expect("rewrite helper without wildcard imported nested enum type");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after stale wildcard imported nested enum type removal");
        assert!(
            err.contains("Wildcard import 'app.M.*' no longer provides 'E'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Undefined variable: E"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_user_facing_error_for_stale_root_namespace_alias_generic_function_value() {
    let temp_root =
        make_temp_project_root("project-build-stale-root-namespace-alias-generic-fn-value");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport app as root;\nfunction main(): Integer { f: (Integer) -> Integer = root.U.id<Integer>; return f(7); }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function id<T>(value: T): T { return value; } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial root namespace alias generic function value build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function other<T>(value: T): T { return value; } }\n",
    )
    .expect("rewrite helper without root namespace alias generic symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).expect_err(
            "build should fail after stale root namespace alias generic function value removal",
        );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'U.id'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Undefined variable: root"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_reports_user_facing_error_for_stale_root_namespace_alias_function_value() {
    let temp_root =
        make_temp_project_root("project-build-stale-root-namespace-alias-function-value");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.apex"),
            "package app;\nimport app as root;\nfunction main(): Integer { f: (Integer) -> Integer = root.U.id; return f(1); }\n",
        )
        .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function id(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial root namespace alias function value build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package app;\nmodule U { function plus(x: Integer): Integer { return x + 1; } }\n",
    )
    .expect("rewrite helper without root namespace alias function value symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).expect_err(
            "build should fail after stale root namespace alias function value removal",
        );
        assert!(
            err.contains("Imported namespace alias 'root' has no member 'U.id'"),
            "{err}"
        );
        assert!(err.contains("Import check failed"), "{err}");
        assert!(!err.contains("Undefined variable: root"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_rechecks_exact_import_alias_dependents_after_symbol_removal() {
    let temp_root = make_temp_project_root("project-build-exact-import-alias-symbol-removal");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.apex"),
        "package app;\nimport lib.add as inc;\nfunction main(): Integer { return inc(1); }\n",
    )
    .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial exact-import-alias project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction other(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("rewrite helper without exact-import alias symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .expect_err("build should fail after exact-import alias symbol removal");
        assert!(
            err.contains("Imported alias 'inc' no longer resolves")
                || err.contains("Import check failed"),
            "{err}"
        );
        assert!(!err.contains("Undefined variable: inc"), "{err}");
        assert!(!err.contains("lib__add"), "{err}");
    });

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_keeps_shadowed_exact_import_alias_calls_local_after_symbol_removal() {
    let temp_root = make_temp_project_root("project-build-shadowed-exact-import-alias-call");
    write_test_project_config(
        &temp_root,
        &["src/main.apex", "src/helper.apex"],
        "src/main.apex",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.apex"),
        r#"
package app;
import lib.add as inc;

function main(): Integer {
    inc: (Integer) -> Integer = (x: Integer) => x + 10;
    return inc(1);
}
"#,
    )
    .expect("write main");
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .expect("initial exact-import alias shadowing build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.apex"),
        "package lib;\nfunction other(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("rewrite helper without shadowed exact-import alias symbol");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "shadowed exact-import alias local call should stay valid after symbol removal",
        );
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .expect("run compiled shadowed exact-import alias binary");
    assert_eq!(
        output.status.code(),
        Some(11),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_runs_module_local_nested_module_constructor_and_generic_function_value() {
    let temp_root = make_temp_project_root("project-build-module-local-nested-members");
    write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
    fs::write(
        temp_root.join("src/main.apex"),
        r#"
package app;

module M {
    module N {
        class Box {
            value: Integer;
            constructor(value: Integer) { this.value = value; }
        }

        function id<T>(value: T): T { return value; }
    }

    function run_box(): Integer {
        value: N.Box = N.Box(55);
        return value.value;
    }

    function run_id(): Integer {
        f: (N.Box) -> N.Box = N.id<N.Box>;
        return f(N.Box(55)).value;
    }
}

function main(): Integer {
    return M.run_box() + M.run_id();
}
"#,
    )
    .expect("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false).expect(
            "project build should succeed for module-local nested constructor and fn value",
        );
    });

    let output = std::process::Command::new(temp_root.join("smoke"))
        .output()
        .expect("run compiled nested module project binary");
    assert_eq!(
        output.status.code(),
        Some(110),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn dependency_graph_recomputes_direct_neighbors_after_api_change() {
    let mut app = make_unit("src/app.apex", "app", &["lib.foo"]);
    app.api_fingerprint = "app-v1".to_string();
    app.semantic_fingerprint = "app-v1".to_string();
    let mut foo = make_unit("src/lib_foo.apex", "lib", &[]);
    foo.function_names = vec!["foo".to_string()];
    foo.api_fingerprint = "foo-v2".to_string();
    foo.semantic_fingerprint = "foo-v2".to_string();

    let previous = DependencyGraphCache {
        schema: DEPENDENCY_GRAPH_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        entry_namespace: "app".to_string(),
        files: vec![
            DependencyGraphFileEntry {
                file: PathBuf::from("src/app.apex"),
                semantic_fingerprint: "app-v1".to_string(),
                api_fingerprint: "app-v1".to_string(),
                direct_dependencies: vec![PathBuf::from("src/lib_foo.apex")],
            },
            DependencyGraphFileEntry {
                file: PathBuf::from("src/lib_foo.apex"),
                semantic_fingerprint: "foo-v1".to_string(),
                api_fingerprint: "foo-v1".to_string(),
                direct_dependencies: vec![],
            },
        ],
    };

    let parsed_files = vec![app.clone(), foo.clone()];
    let namespace_files_map = HashMap::from([
        ("app".to_string(), vec![PathBuf::from("src/app.apex")]),
        ("lib".to_string(), vec![PathBuf::from("src/lib_foo.apex")]),
    ]);
    let namespace_function_files = HashMap::from([(
        "lib".to_string(),
        HashMap::from([("foo".to_string(), PathBuf::from("src/lib_foo.apex"))]),
    )]);
    let ctx = DependencyResolutionContext {
        namespace_files_map: &namespace_files_map,
        global_function_map: &HashMap::from([("foo".to_string(), "lib".to_string())]),
        global_function_file_map: &HashMap::from([(
            "foo".to_string(),
            PathBuf::from("src/lib_foo.apex"),
        )]),
        global_class_map: &HashMap::new(),
        global_class_file_map: &HashMap::new(),
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &HashMap::new(),
        global_enum_file_map: &HashMap::new(),
        global_module_map: &HashMap::new(),
        global_module_file_map: &HashMap::new(),
        symbol_lookup: Arc::new(build_project_symbol_lookup(
            &crate::dependency::ProjectSymbolMaps {
                function_map: &HashMap::from([("foo".to_string(), "lib".to_string())]),
                function_file_map: &HashMap::from([(
                    "foo".to_string(),
                    PathBuf::from("src/lib_foo.apex"),
                )]),
                class_map: &HashMap::new(),
                class_file_map: &HashMap::new(),
                interface_map: empty_global_interface_map(),
                interface_file_map: empty_global_interface_file_map(),
                enum_map: &HashMap::new(),
                enum_file_map: &HashMap::new(),
                module_map: &HashMap::new(),
                module_file_map: &HashMap::new(),
            },
        )),
    };

    let (_, reused) =
        build_file_dependency_graph_incremental(&parsed_files, &ctx, Some(&previous), None);
    assert_eq!(reused, 0);
}

#[test]
fn typecheck_summary_cache_matches_identical_component_fingerprints() {
    let current = HashMap::from([
        (PathBuf::from("a.apex"), "sem-a".to_string()),
        (PathBuf::from("b.apex"), "sem-b".to_string()),
    ]);
    let components = vec![vec![PathBuf::from("a.apex")], vec![PathBuf::from("b.apex")]];
    let cache = typecheck_summary_cache_from_state(&current, &components);

    assert!(typecheck_summary_cache_matches(
        &cache,
        &current,
        &components
    ));
}

#[test]
fn reusable_component_fingerprints_allows_partial_semantic_reuse() {
    let previous = typecheck_summary_cache_from_state(
        &HashMap::from([
            (PathBuf::from("a.apex"), "sem-a".to_string()),
            (PathBuf::from("b.apex"), "sem-b".to_string()),
            (PathBuf::from("c.apex"), "sem-c-old".to_string()),
        ]),
        &[
            vec![PathBuf::from("a.apex"), PathBuf::from("b.apex")],
            vec![PathBuf::from("c.apex")],
        ],
    );
    let current = HashMap::from([
        (PathBuf::from("a.apex"), "sem-a".to_string()),
        (PathBuf::from("b.apex"), "sem-b".to_string()),
        (PathBuf::from("c.apex"), "sem-c-new".to_string()),
    ]);
    let components = vec![
        vec![PathBuf::from("a.apex"), PathBuf::from("b.apex")],
        vec![PathBuf::from("c.apex")],
    ];

    let reusable = reusable_component_fingerprints(&previous, &current, &components);

    assert_eq!(reusable.len(), 1);
    assert!(reusable.contains(&component_fingerprint(&components[0], &current)));
}

#[test]
fn reverse_dependency_graph_returns_only_transitive_dependents() {
    let reverse = build_reverse_dependency_graph(&HashMap::from([
        (
            PathBuf::from("a.apex"),
            HashSet::from([PathBuf::from("b.apex")]),
        ),
        (
            PathBuf::from("c.apex"),
            HashSet::from([PathBuf::from("a.apex")]),
        ),
        (PathBuf::from("d.apex"), HashSet::new()),
    ]));

    let impacted = transitive_dependents(&reverse, &HashSet::from([PathBuf::from("b.apex")]));

    assert_eq!(
        impacted,
        HashSet::from([
            PathBuf::from("b.apex"),
            PathBuf::from("a.apex"),
            PathBuf::from("c.apex"),
        ])
    );
}

#[test]
fn link_manifest_skip_requires_exact_manifest_match_and_no_object_misses() {
    let output_path = PathBuf::from("build/app");
    let link_inputs = vec![PathBuf::from("a.o"), PathBuf::from("b.o")];
    let link = LinkConfig {
        opt_level: Some("3"),
        target: None,
        output_kind: OutputKind::Bin,
        link_search: &[],
        link_libs: &[],
        link_args: &[],
    };
    let current = LinkManifestCache {
        schema: LINK_MANIFEST_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        link_fingerprint: compute_link_fingerprint(&output_path, &link_inputs, &link),
        link_inputs: link_inputs.clone(),
    };

    assert!(!should_skip_final_link(None, &current, &output_path, 0));
    assert!(!should_skip_final_link(
        Some(&current),
        &current,
        &output_path,
        1
    ));
}

#[test]
fn link_manifest_skip_allows_relink_elision_for_identical_cached_inputs() {
    let temp = std::env::temp_dir().join(format!("apex-link-manifest-test-{}", std::process::id()));
    fs::write(&temp, b"bin").expect("write output placeholder");
    let link_inputs = vec![PathBuf::from("a.o"), PathBuf::from("b.o")];
    let link = LinkConfig {
        opt_level: Some("3"),
        target: None,
        output_kind: OutputKind::Bin,
        link_search: &[],
        link_libs: &[],
        link_args: &[],
    };
    let current = LinkManifestCache {
        schema: LINK_MANIFEST_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        link_fingerprint: compute_link_fingerprint(&temp, &link_inputs, &link),
        link_inputs,
    };

    assert!(should_skip_final_link(Some(&current), &current, &temp, 0));

    let _ = fs::remove_file(temp);
}

#[test]
fn object_shard_cache_key_ignores_member_order() {
    let a = vec![PathBuf::from("src/a.apex"), PathBuf::from("src/b.apex")];
    let b = vec![PathBuf::from("src/b.apex"), PathBuf::from("src/a.apex")];

    assert_eq!(object_shard_cache_key(&a), object_shard_cache_key(&b));
}

#[test]
fn object_shard_cache_hit_ignores_member_order() {
    let temp_root = make_temp_project_root("object-shard-cache-member-order");
    let files_ab = vec![PathBuf::from("src/a.apex"), PathBuf::from("src/b.apex")];
    let files_ba = vec![PathBuf::from("src/b.apex"), PathBuf::from("src/a.apex")];
    let cache_paths = object_shard_cache_paths(&temp_root, &files_ab);
    let fingerprint = "obj-fp";
    let members_ab = vec![
        ObjectShardMemberFingerprint {
            file: PathBuf::from("src/a.apex"),
            semantic_fingerprint: "sem-a".to_string(),
            rewrite_context_fingerprint: "rw-a".to_string(),
        },
        ObjectShardMemberFingerprint {
            file: PathBuf::from("src/b.apex"),
            semantic_fingerprint: "sem-b".to_string(),
            rewrite_context_fingerprint: "rw-b".to_string(),
        },
    ];
    let members_ba = vec![
        ObjectShardMemberFingerprint {
            file: PathBuf::from("src/b.apex"),
            semantic_fingerprint: "sem-b".to_string(),
            rewrite_context_fingerprint: "rw-b".to_string(),
        },
        ObjectShardMemberFingerprint {
            file: PathBuf::from("src/a.apex"),
            semantic_fingerprint: "sem-a".to_string(),
            rewrite_context_fingerprint: "rw-a".to_string(),
        },
    ];

    fs::create_dir_all(
        cache_paths
            .object_path
            .parent()
            .expect("object shard cache path should have parent"),
    )
    .expect("create object shard cache directory");
    fs::write(&cache_paths.object_path, b"obj").expect("write cached object shard");
    save_object_shard_cache_meta(&cache_paths, &members_ab, fingerprint)
        .expect("save object shard meta");

    let reordered_cache_paths = object_shard_cache_paths(&temp_root, &files_ba);
    let hit = load_object_shard_cache_hit(&reordered_cache_paths, &members_ba, fingerprint)
        .expect("load object shard cache hit");
    assert_eq!(hit, Some(reordered_cache_paths.object_path.clone()));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn load_link_manifest_cache_reports_io_errors_instead_of_silent_cache_miss() {
    let temp_root = make_temp_project_root("link-manifest-io-error");
    let manifest_path = temp_root
        .join(".apexcache")
        .join("link")
        .join("latest.json");
    fs::create_dir_all(&manifest_path).expect("create manifest path as directory");

    let err = load_link_manifest_cache(&temp_root)
        .expect_err("directory-shaped manifest path should surface an io error");
    assert!(err.contains("link manifest cache"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn read_cache_blob_reports_decode_errors_instead_of_silent_cache_miss() {
    let temp_root = make_temp_project_root("cache-decode-error");
    let cache_path = temp_root
        .join(".apexcache")
        .join("parsed")
        .join("broken.bin");
    fs::create_dir_all(
        cache_path
            .parent()
            .expect("cache path should have parent directory"),
    )
    .expect("create cache dir");
    fs::write(&cache_path, b"not valid bincode").expect("write invalid cache payload");

    let err = read_cache_blob::<ParsedFileCacheEntry>(&cache_path, "parse cache")
        .expect_err("invalid cache payload should surface a decode error");
    assert!(err.contains("Failed to decode parse cache"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn load_cached_fingerprint_reports_io_errors_instead_of_silent_cache_miss() {
    let temp_root = make_temp_project_root("build-fingerprint-io-error");
    let cache_path = temp_root.join(".apexcache").join("build_fingerprint");
    fs::create_dir_all(&cache_path).expect("create directory-shaped build cache path");

    let err = load_cached_fingerprint(&temp_root)
        .expect_err("directory-shaped build fingerprint path should surface an io error");
    assert!(err.contains("Failed to read build cache"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn load_semantic_cached_fingerprint_reports_io_errors_instead_of_silent_cache_miss() {
    let temp_root = make_temp_project_root("semantic-fingerprint-io-error");
    let cache_path = temp_root
        .join(".apexcache")
        .join("semantic_build_fingerprint");
    fs::create_dir_all(&cache_path).expect("create directory-shaped semantic cache path");

    let err = load_semantic_cached_fingerprint(&temp_root)
        .expect_err("directory-shaped semantic fingerprint path should surface an io error");
    assert!(err.contains("Failed to read semantic build cache"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn dedupe_link_inputs_removes_duplicate_object_paths_stably() {
    let deduped = dedupe_link_inputs(vec![
        PathBuf::from("a.obj"),
        PathBuf::from("b.obj"),
        PathBuf::from("a.obj"),
        PathBuf::from("c.obj"),
        PathBuf::from("b.obj"),
    ]);

    assert_eq!(
        deduped,
        vec![
            PathBuf::from("a.obj"),
            PathBuf::from("b.obj"),
            PathBuf::from("c.obj")
        ]
    );
}

#[test]
fn project_parse_cache_recovers_cleanly_after_invalid_sibling_fix() {
    let temp_root = make_temp_project_root("parse-cache-invalid-sibling-fix");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.apex");
    let helper_file = src_dir.join("helper.apex");

    fs::write(
            &main_file,
            "package app;\nimport lib.math;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main file");
    fs::write(
        &helper_file,
        "package lib;\nfunction add(: Integer { return 1; }\n",
    )
    .expect("write invalid helper file");

    let first_main = parse_project_unit(&temp_root, &main_file).expect("first main parse");
    let first_helper_err = parse_project_unit(&temp_root, &helper_file)
        .expect_err("invalid helper should fail parsing");
    assert!(
        first_helper_err.contains("Parse error"),
        "{first_helper_err}"
    );
    assert!(!first_main.from_parse_cache);

    thread::sleep(Duration::from_millis(5));
    fs::write(
        &helper_file,
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .expect("rewrite helper file");

    let second_main = parse_project_unit(&temp_root, &main_file).expect("second main parse");
    let second_helper = parse_project_unit(&temp_root, &helper_file).expect("second helper parse");
    assert!(second_main.from_parse_cache);
    assert!(!second_helper.from_parse_cache);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn parse_cache_reuses_same_content_even_after_metadata_change() {
    let temp_root = std::env::temp_dir().join(format!(
        "apex-parse-cache-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    let src_dir = temp_root.join("src");
    fs::create_dir_all(&src_dir).expect("create temp src dir");
    let file = src_dir.join("main.apex");
    let source = "function main(): None { return None; }\n";
    fs::write(&file, source).expect("write source");

    let first = parse_project_unit(&temp_root, &file).expect("first parse");
    assert!(!first.from_parse_cache);

    thread::sleep(Duration::from_millis(5));
    fs::write(&file, source).expect("rewrite identical source");

    let second = parse_project_unit(&temp_root, &file).expect("second parse");
    assert!(second.from_parse_cache);
    assert_eq!(first.semantic_fingerprint, second.semantic_fingerprint);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn response_file_args_escape_quotes_and_backslashes() {
    assert_eq!(
        escape_response_file_arg("C:\\tmp\\a \"b\".o"),
        "\"C:\\\\tmp\\\\a \\\"b\\\".o\""
    );
}

#[test]
fn precompute_transitive_dependencies_matches_expected_closure() {
    let graph = HashMap::from([
        (
            PathBuf::from("a.apex"),
            HashSet::from([PathBuf::from("b.apex"), PathBuf::from("c.apex")]),
        ),
        (
            PathBuf::from("b.apex"),
            HashSet::from([PathBuf::from("d.apex")]),
        ),
        (
            PathBuf::from("c.apex"),
            HashSet::from([PathBuf::from("d.apex")]),
        ),
        (PathBuf::from("d.apex"), HashSet::new()),
    ]);

    let all = precompute_all_transitive_dependencies(&graph);
    assert_eq!(
        transitive_dependencies_from_precomputed(&all, Path::new("a.apex")),
        HashSet::from([
            PathBuf::from("b.apex"),
            PathBuf::from("c.apex"),
            PathBuf::from("d.apex"),
        ])
    );
}

#[test]
fn codegen_program_for_unit_uses_api_for_dependencies_and_projection_for_specialization_files() {
    let make_stmt = |value: i64| {
        Spanned::new(
            Stmt::Return(Some(Spanned::new(
                Expr::Literal(Literal::Integer(value)),
                0..0,
            ))),
            0..0,
        )
    };
    let make_function = |name: &str, body_len: usize| {
        Spanned::new(
            Decl::Function(FunctionDecl {
                name: name.to_string(),
                params: Vec::new(),
                return_type: Type::None,
                body: (0..body_len).map(|idx| make_stmt(idx as i64)).collect(),
                generic_params: Vec::new(),
                visibility: Visibility::Public,
                is_async: false,
                is_extern: false,
                extern_abi: None,
                extern_link_name: None,
                attributes: Vec::new(),
                is_variadic: false,
            }),
            0..0,
        )
    };
    let make_unit = |file: &str,
                     body_name: &str,
                     body_len: usize,
                     api_len: usize,
                     projection_len: usize,
                     has_specialization_demand: bool| RewrittenProjectUnit {
        file: PathBuf::from(file),
        program: Program {
            package: None,
            declarations: vec![make_function(body_name, body_len)],
        },
        api_program: Program {
            package: None,
            declarations: vec![make_function(body_name, api_len)],
        },
        specialization_projection: Program {
            package: None,
            declarations: vec![make_function(body_name, projection_len)],
        },
        semantic_fingerprint: "sem".to_string(),
        rewrite_context_fingerprint: "rw".to_string(),
        active_symbols: HashSet::from([body_name.to_string()]),
        has_specialization_demand,
        from_rewrite_cache: false,
    };

    let rewritten_files = vec![
        make_unit("a.apex", "fa", 3, 0, 1, false),
        make_unit("b.apex", "fb", 4, 0, 1, false),
        make_unit("c.apex", "fc", 5, 0, 1, true),
    ];
    let rewritten_file_indices = HashMap::from([
        (PathBuf::from("a.apex"), 0usize),
        (PathBuf::from("b.apex"), 1usize),
        (PathBuf::from("c.apex"), 2usize),
    ]);
    let program = codegen_program_for_unit(
        &rewritten_files,
        &rewritten_file_indices,
        Path::new("a.apex"),
        Some(&HashSet::from([PathBuf::from("b.apex")])),
        Some(&HashSet::from(["fb".to_string()])),
    );

    let bodies = program
        .declarations
        .iter()
        .filter_map(|decl| match &decl.node {
            Decl::Function(func) => Some((func.name.clone(), func.body.len())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        bodies,
        vec![
            ("fa".to_string(), 3usize),
            ("fb".to_string(), 0usize),
            ("fc".to_string(), 1usize),
        ]
    );
}
