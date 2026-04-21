use super::*;
use crate::ast::{
    Decl, Expr, FunctionDecl, ImportDecl, Literal, Program, Spanned, Stmt, Type, Visibility,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[test]
fn project_rewrite_fingerprint_ignores_body_only_dependency_change() {
    let temp_root = make_temp_project_root("rewrite-fp-body-only");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.arden");
    let helper_file = src_dir.join("helper.arden");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nimport lib.foo;\nfunction main(): None { value: Integer = foo(); return None; }\n",
        )
        .must("write main");
    fs::write(
        &helper_file,
        "package lib;\nfunction foo(): Integer { return 1; }\n",
    )
    .must("write helper");
    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main before"),
        parse_project_unit(&temp_root, &helper_file).must("parse helper before"),
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };
    let main_before = parsed_before
        .iter()
        .find(|u| u.file == main_file)
        .must("main");
    let rewrite_fp_before =
        compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

    thread::sleep(Duration::from_millis(5));
    fs::write(
        &helper_file,
        "package lib;\nfunction foo(): Integer { return 2; }\n",
    )
    .must("rewrite helper body");

    let parsed_files = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main after"),
        parse_project_unit(&temp_root, &helper_file).must("parse helper after"),
    ];
    let (
        namespace_files_map,
        _namespace_function_files,
        _namespace_class_files,
        _namespace_module_files,
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };
    let main_unit = parsed_files
        .iter()
        .find(|u| u.file == main_file)
        .must("main");
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
    let main_file = src_dir.join("main.arden");
    let helper_file = src_dir.join("helper.arden");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nimport lib.foo;\nfunction main(): None { value: Integer = foo(); return None; }\n",
        )
        .must("write main");
    fs::write(
        &helper_file,
        "package lib;\nfunction foo(): Integer { return 1; }\n",
    )
    .must("write helper");
    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main before"),
        parse_project_unit(&temp_root, &helper_file).must("parse helper before"),
    ];
    let (
        _namespace_files_map_before,
        _namespace_function_files_before,
        _namespace_class_files_before,
        _namespace_module_files_before,
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };
    let main_before = parsed_before
        .iter()
        .find(|u| u.file == main_file)
        .must("main");
    let rewrite_fp_before =
        compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

    thread::sleep(Duration::from_millis(5));
    fs::write(
        &helper_file,
        "package lib;\nfunction bar(): Integer { return 1; }\n",
    )
    .must("rewrite helper api");

    let parsed_files = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main"),
        parse_project_unit(&temp_root, &helper_file).must("parse helper"),
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };
    let main_unit = parsed_files
        .iter()
        .find(|u| u.file == main_file)
        .must("main");
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
    let main_file = src_dir.join("main.arden");
    let helper_file = src_dir.join("helper.arden");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nimport app as root;\nclass Book implements root.M.Api.Named { constructor() {} function name(): Integer { return 1; } }\nfunction main(): Integer { return 0; }\n",
        )
        .must("write main");
    fs::write(
        &helper_file,
        "package app;\nmodule M { module Api { interface Named { function name(): Integer; } } }\n",
    )
    .must("write helper before");

    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main before"),
        parse_project_unit(&temp_root, &helper_file).must("parse helper before"),
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };
    let main_before = parsed_before
        .iter()
        .find(|u| u.file == main_file)
        .must("main before");
    let rewrite_fp_before =
        compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

    thread::sleep(Duration::from_millis(5));
    fs::write(
            &helper_file,
            "package app;\nmodule M { module Api { interface Labelled { function name(): Integer; } } }\n",
        )
        .must("write helper after");

    let parsed_after = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main after"),
        parse_project_unit(&temp_root, &helper_file).must("parse helper after"),
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };
    let main_after = parsed_after
        .iter()
        .find(|u| u.file == main_file)
        .must("main after");
    let rewrite_fp_after =
        compute_rewrite_context_fingerprint_for_unit(main_after, "app", &rewrite_ctx_after);

    assert_ne!(rewrite_fp_before, rewrite_fp_after);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_rewrite_fingerprint_changes_on_keyword_import_alias_target_change() {
    let temp_root = make_temp_project_root("rewrite-fp-keyword-alias-change");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.arden");
    let helper_file = src_dir.join("helper.arden");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nimport lib.Maybe.Empty as Empty;\nfunction main(x: Maybe): None { match (x) { Empty => { return None; }, _ => { return None; } } }\n",
        )
        .must("write main");
    fs::write(
        &helper_file,
        "package lib;\nenum Maybe { Empty, Filled(value: Integer) }\n",
    )
    .must("write helper before");

    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main before"),
        parse_project_unit(&temp_root, &helper_file).must("parse helper before"),
    ];
    let (
        _namespace_files_map_before,
        _namespace_function_files_before,
        _namespace_class_files_before,
        _namespace_module_files_before,
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };
    let main_before = parsed_before
        .iter()
        .find(|u| u.file == main_file)
        .must("main before");
    let rewrite_fp_before =
        compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

    thread::sleep(Duration::from_millis(5));

    fs::write(&helper_file, "package lib;\nenum Maybe { Empty }\n").must("write helper after");

    let parsed_after = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main after"),
        parse_project_unit(&temp_root, &helper_file).must("parse helper after"),
    ];
    let (
        _namespace_files_map_after,
        _namespace_function_files_after,
        _namespace_class_files_after,
        _namespace_module_files_after,
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };
    let main_after = parsed_after
        .iter()
        .find(|u| u.file == main_file)
        .must("main after");
    let rewrite_fp_after =
        compute_rewrite_context_fingerprint_for_unit(main_after, "app", &rewrite_ctx_after);

    assert_ne!(rewrite_fp_before, rewrite_fp_after);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_rewrite_fingerprint_ignores_body_only_alias_target_change() {
    let temp_root = make_temp_project_root("rewrite-fp-alias-body-only");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.arden");
    let helper_file = src_dir.join("helper.arden");
    let helper_impl_file = src_dir.join("helper_impl.arden");
    write_test_project_config(
        &temp_root,
        &[
            "src/main.arden",
            "src/helper.arden",
            "src/helper_impl.arden",
        ],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nimport lib.Maybe.Empty as Empty;\nfunction main(x: Maybe): None { match (x) { Empty => { return None; }, _ => { return None; } } }\n",
        )
        .must("write main");
    fs::write(
            &helper_file,
            "package lib;\nenum Maybe { Empty, Filled(value: Integer) }\nfunction make(): Integer { return helper_value(); }\n",
        )
        .must("write helper before");
    fs::write(
        &helper_impl_file,
        "package lib;\nfunction helper_value(): Integer { return 1; }\n",
    )
    .must("write helper impl before");

    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main before"),
        parse_project_unit(&temp_root, &helper_file).must("parse helper before"),
        parse_project_unit(&temp_root, &helper_impl_file).must("parse helper impl before"),
    ];
    let rewrite_fp_before = rewrite_fingerprint_for_test_unit(&parsed_before, &main_file, "app");

    thread::sleep(Duration::from_millis(5));
    fs::write(
        &helper_impl_file,
        "package lib;\nfunction helper_value(): Integer { return 99; }\n",
    )
    .must("write helper impl after");

    let parsed_after = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main after"),
        parse_project_unit(&temp_root, &helper_file).must("parse helper after"),
        parse_project_unit(&temp_root, &helper_impl_file).must("parse helper impl after"),
    ];
    let rewrite_fp_after = rewrite_fingerprint_for_test_unit(&parsed_after, &main_file, "app");

    assert_eq!(rewrite_fp_before, rewrite_fp_after);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_rewrite_fingerprint_changes_on_same_namespace_enum_api_change_without_import() {
    let temp_root = make_temp_project_root("rewrite-fp-same-namespace-enum-api-change");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.arden");
    let enum_file = src_dir.join("enum.arden");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/enum.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nfunction main(): Integer { return match (State.Ok(1)) { Ok(value) => value, }; }\n",
        )
        .must("write main");
    fs::write(&enum_file, "package app;\nenum State { Ok(Integer) }\n").must("write enum before");

    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main before"),
        parse_project_unit(&temp_root, &enum_file).must("parse enum before"),
    ];
    let rewrite_fp_before = rewrite_fingerprint_for_test_unit(&parsed_before, &main_file, "app");

    thread::sleep(Duration::from_millis(5));
    fs::write(&enum_file, "package app;\nenum State { Ready(Integer) }\n").must("write enum after");

    let parsed_after = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main after"),
        parse_project_unit(&temp_root, &enum_file).must("parse enum after"),
    ];
    let rewrite_fp_after = rewrite_fingerprint_for_test_unit(&parsed_after, &main_file, "app");

    assert_ne!(rewrite_fp_before, rewrite_fp_after);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_rewrite_fingerprint_ignores_body_only_change_for_alias_heavy_builtin_consumer() {
    let temp_root = make_temp_project_root("rewrite-fp-alias-heavy-builtin-body-only");
    let src_dir = temp_root.join("src");
    let main_file = src_dir.join("main.arden");
    let helper_file = src_dir.join("helper.arden");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            &main_file,
            "package app;\nimport app.Option.Some as Present;\nimport app.Option.None as Empty;\nimport app.Result.Ok as Success;\nimport app.Result.Error as Failure;\nfunction unwrap_opt(value: Option<Integer>): Integer { return match (value) { Present(inner) => inner, Empty => 0, }; }\nfunction run(flag: Boolean): Integer { result: Result<Option<Integer>, String> = make(flag); value: Option<Integer> = match (result) { Success(inner) => inner, Failure(err) => Option<Integer>(), }; return unwrap_opt(value); }\n",
        )
        .must("write main");
    fs::write(
            &helper_file,
            "package app;\nfunction make(flag: Boolean): Result<Option<Integer>, String> { if (flag) { return Result<Option<Integer>, String>(); } return Result<Option<Integer>, String>(); }\n",
        )
        .must("write helper before");

    let parsed_before = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main before"),
        parse_project_unit(&temp_root, &helper_file).must("parse helper before"),
    ];
    let rewrite_fp_before = rewrite_fingerprint_for_test_unit(&parsed_before, &main_file, "app");

    thread::sleep(Duration::from_millis(5));
    fs::write(
            &helper_file,
            "package app;\nfunction make(flag: Boolean): Result<Option<Integer>, String> { if (flag) { return Result<Option<Integer>, String>(); } return Result<Option<Integer>, String>(); }\n// body-only comment perturbation\n",
        )
        .must("write helper after");

    let parsed_after = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main after"),
        parse_project_unit(&temp_root, &helper_file).must("parse helper after"),
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
        let main_file = src_dir.join("main.arden");
        let helper_file = src_dir.join("helper.arden");
        write_test_project_config(
            &temp_root,
            &["src/main.arden", "src/helper.arden"],
            "src/main.arden",
            "smoke",
        );
        fs::write(
                &main_file,
                "package app;\nimport lib.foo;\nfunction main(): None { value: Integer = foo(); return None; }\n",
            )
            .must("write main");
        fs::write(
            &helper_file,
            "package lib;\nfunction foo(): Integer { return 1; }\n",
        )
        .must("write helper");

        let parsed_before = vec![
            parse_project_unit(&temp_root, &main_file).must("parse main before"),
            parse_project_unit(&temp_root, &helper_file).must("parse helper before"),
        ];
        let (
            _namespace_files_map_before,
            _namespace_function_files_before,
            _namespace_class_files_before,
            _namespace_module_files_before,
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
            symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
            })),
        };
        let main_before = parsed_before
            .iter()
            .find(|u| u.file == main_file)
            .must("main");
        let rewrite_fp_before =
            compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

        fs::write(&helper_file, helper_after).must("rewrite helper body variant");
        let parsed_after = vec![
            parse_project_unit(&temp_root, &main_file).must("parse main after"),
            parse_project_unit(&temp_root, &helper_file).must("parse helper after"),
        ];
        let (
            _namespace_files_map_after,
            _namespace_function_files_after,
            _namespace_class_files_after,
            _namespace_module_files_after,
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
            symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
            })),
        };
        let main_after = parsed_after
            .iter()
            .find(|u| u.file == main_file)
            .must("main");
        let rewrite_fp_after =
            compute_rewrite_context_fingerprint_for_unit(main_after, "app", &rewrite_ctx_after);

        assert_eq!(rewrite_fp_before, rewrite_fp_after);
        let _ = fs::remove_dir_all(temp_root);
    }

    for helper_after in import_breaking_variants {
        let temp_root = make_temp_project_root("generated-rewrite-api");
        let src_dir = temp_root.join("src");
        let main_file = src_dir.join("main.arden");
        let helper_file = src_dir.join("helper.arden");
        write_test_project_config(
            &temp_root,
            &["src/main.arden", "src/helper.arden"],
            "src/main.arden",
            "smoke",
        );
        fs::write(
                &main_file,
                "package app;\nimport lib.foo;\nfunction main(): None { value: Integer = foo(); return None; }\n",
            )
            .must("write main");
        fs::write(
            &helper_file,
            "package lib;\nfunction foo(): Integer { return 1; }\n",
        )
        .must("write helper");

        let parsed_before = vec![
            parse_project_unit(&temp_root, &main_file).must("parse main before"),
            parse_project_unit(&temp_root, &helper_file).must("parse helper before"),
        ];
        let (
            _namespace_files_map_before,
            _namespace_function_files_before,
            _namespace_class_files_before,
            _namespace_module_files_before,
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
            symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
            })),
        };
        let main_before = parsed_before
            .iter()
            .find(|u| u.file == main_file)
            .must("main");
        let rewrite_fp_before =
            compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

        fs::write(&helper_file, helper_after).must("rewrite helper api variant");
        let parsed_after = vec![
            parse_project_unit(&temp_root, &main_file).must("parse main after"),
            parse_project_unit(&temp_root, &helper_file).must("parse helper after"),
        ];
        let (
            _namespace_files_map_after,
            _namespace_function_files_after,
            _namespace_class_files_after,
            _namespace_module_files_after,
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
            symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
            })),
        };
        let main_after = parsed_after
            .iter()
            .find(|u| u.file == main_file)
            .must("main");
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

pub(crate) fn empty_global_interface_map() -> &'static HashMap<String, String> {
    static EMPTY: OnceLock<HashMap<String, String>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

pub(crate) fn empty_global_interface_file_map() -> &'static HashMap<String, PathBuf> {
    static EMPTY: OnceLock<HashMap<String, PathBuf>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

#[test]
fn rewrite_context_for_specific_import_ignores_unrelated_namespace_api_changes() {
    let unit = make_unit("src/main.arden", "app", &["lib.foo"]);

    let namespace_functions = HashMap::from([(
        "lib".to_string(),
        HashSet::from(["foo".to_string(), "bar".to_string()]),
    )]);
    let global_function_map = HashMap::from([
        ("foo".to_string(), "lib".to_string()),
        ("bar".to_string(), "lib".to_string()),
    ]);
    let global_function_file_map = HashMap::from([
        ("foo".to_string(), PathBuf::from("src/lib_foo.arden")),
        ("bar".to_string(), PathBuf::from("src/lib_bar.arden")),
    ]);
    let _namespace_function_files = HashMap::from([(
        "lib".to_string(),
        HashMap::from([
            ("foo".to_string(), PathBuf::from("src/lib_foo.arden")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.arden")),
        ]),
    )]);
    let namespace_classes = HashMap::new();
    let _namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let global_class_map = HashMap::new();
    let global_class_file_map = HashMap::new();
    let global_enum_map = HashMap::new();
    let global_enum_file_map = HashMap::new();
    let namespace_modules = HashMap::new();
    let _namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let global_module_map = HashMap::new();
    let global_module_file_map = HashMap::new();
    let namespace_api_fingerprints = HashMap::from([("lib".to_string(), "ns-v1".to_string())]);
    let file_api_fingerprints = HashMap::from([
        (
            PathBuf::from("src/lib_foo.arden"),
            "file-foo-v1".to_string(),
        ),
        (
            PathBuf::from("src/lib_bar.arden"),
            "file-bar-v1".to_string(),
        ),
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };

    let fp_a = compute_rewrite_context_fingerprint_for_unit(&unit, "app", &ctx_a);
    let namespace_api_fingerprints_b = HashMap::from([("lib".to_string(), "ns-v2".to_string())]);
    let file_api_fingerprints_b = HashMap::from([
        (
            PathBuf::from("src/lib_foo.arden"),
            "file-foo-v1".to_string(),
        ),
        (
            PathBuf::from("src/lib_bar.arden"),
            "file-bar-v2".to_string(),
        ),
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };
    let fp_b = compute_rewrite_context_fingerprint_for_unit(&unit, "app", &ctx_b);

    assert_eq!(fp_a, fp_b);
}

#[test]
fn rewrite_context_for_wildcard_import_tracks_namespace_api_changes() {
    let unit = make_unit("src/main.arden", "app", &["lib.*"]);

    let namespace_functions = HashMap::from([(
        "lib".to_string(),
        HashSet::from(["foo".to_string(), "bar".to_string()]),
    )]);
    let global_function_map = HashMap::from([
        ("foo".to_string(), "lib".to_string()),
        ("bar".to_string(), "lib".to_string()),
    ]);
    let global_function_file_map = HashMap::from([
        ("foo".to_string(), PathBuf::from("src/lib_foo.arden")),
        ("bar".to_string(), PathBuf::from("src/lib_bar.arden")),
    ]);
    let _namespace_function_files = HashMap::from([(
        "lib".to_string(),
        HashMap::from([
            ("foo".to_string(), PathBuf::from("src/lib_foo.arden")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.arden")),
        ]),
    )]);
    let namespace_classes = HashMap::new();
    let _namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let global_class_map = HashMap::new();
    let global_class_file_map = HashMap::new();
    let global_enum_map = HashMap::new();
    let global_enum_file_map = HashMap::new();
    let namespace_modules = HashMap::new();
    let _namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };
    let fp_b = compute_rewrite_context_fingerprint_for_unit(&unit, "app", &ctx_b);

    assert_ne!(fp_a, fp_b);
}

#[test]
fn dependency_graph_tracks_specific_symbol_owner_file_only() {
    let app = make_unit("src/main.arden", "app", &["lib.foo"]);
    let foo = make_unit("src/lib_foo.arden", "lib", &[]);
    let bar = make_unit("src/lib_bar.arden", "lib", &[]);
    let parsed_files = vec![app.clone(), foo, bar];
    let namespace_files_map = HashMap::from([
        ("app".to_string(), vec![PathBuf::from("src/main.arden")]),
        (
            "lib".to_string(),
            vec![
                PathBuf::from("src/lib_bar.arden"),
                PathBuf::from("src/lib_foo.arden"),
            ],
        ),
    ]);

    let global_function_map = HashMap::from([
        ("foo".to_string(), "lib".to_string()),
        ("bar".to_string(), "lib".to_string()),
    ]);
    let global_function_file_map = HashMap::from([
        ("foo".to_string(), PathBuf::from("src/lib_foo.arden")),
        ("bar".to_string(), PathBuf::from("src/lib_bar.arden")),
    ]);
    let global_class_map = HashMap::new();
    let global_class_file_map = HashMap::new();
    let global_enum_map = HashMap::new();
    let global_enum_file_map = HashMap::new();
    let global_module_map = HashMap::new();
    let global_module_file_map = HashMap::new();
    let _namespace_function_files = HashMap::from([(
        "lib".to_string(),
        HashMap::from([
            ("foo".to_string(), PathBuf::from("src/lib_foo.arden")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.arden")),
        ]),
    )]);
    let _namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let _namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };
    let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None, None);

    assert_eq!(
        graph.get(&app.file).cloned().unwrap_or_default(),
        HashSet::from([PathBuf::from("src/lib_foo.arden")])
    );
}

#[test]
fn dependency_graph_tracks_same_namespace_symbol_references() {
    let mut app = make_unit("src/app.arden", "app", &[]);
    app.referenced_symbols = vec!["helper".to_string()];
    let mut helper = make_unit("src/helper.arden", "app", &[]);
    helper.function_names = vec!["helper".to_string()];
    let parsed_files = vec![app.clone(), helper.clone()];
    let namespace_files_map = HashMap::from([(
        "app".to_string(),
        vec![
            PathBuf::from("src/app.arden"),
            PathBuf::from("src/helper.arden"),
        ],
    )]);
    let _namespace_function_files = HashMap::from([(
        "app".to_string(),
        HashMap::from([("helper".to_string(), PathBuf::from("src/helper.arden"))]),
    )]);
    let _namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let _namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let global_function_map = HashMap::from([("helper".to_string(), "app".to_string())]);
    let global_function_file_map =
        HashMap::from([("helper".to_string(), PathBuf::from("src/helper.arden"))]);
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };

    let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None, None);

    assert_eq!(
        graph.get(&app.file).cloned().unwrap_or_default(),
        HashSet::from([PathBuf::from("src/helper.arden")])
    );
    assert!(graph
        .get(&helper.file)
        .cloned()
        .unwrap_or_default()
        .is_empty());
}

#[test]
fn dependency_graph_limits_wildcard_imports_to_used_owner_files() {
    let mut app = make_unit("src/main.arden", "app", &["lib.*"]);
    app.referenced_symbols = vec!["foo".to_string()];
    let mut foo = make_unit("src/lib_foo.arden", "lib", &[]);
    foo.function_names = vec!["foo".to_string()];
    let mut bar = make_unit("src/lib_bar.arden", "lib", &[]);
    bar.function_names = vec!["bar".to_string()];
    let parsed_files = vec![app.clone(), foo, bar];
    let namespace_files_map = HashMap::from([
        ("app".to_string(), vec![PathBuf::from("src/main.arden")]),
        (
            "lib".to_string(),
            vec![
                PathBuf::from("src/lib_bar.arden"),
                PathBuf::from("src/lib_foo.arden"),
            ],
        ),
    ]);
    let _namespace_function_files = HashMap::from([(
        "lib".to_string(),
        HashMap::from([
            ("foo".to_string(), PathBuf::from("src/lib_foo.arden")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.arden")),
        ]),
    )]);
    let ctx = DependencyResolutionContext {
        namespace_files_map: &namespace_files_map,
        global_function_map: &HashMap::from([
            ("foo".to_string(), "lib".to_string()),
            ("bar".to_string(), "lib".to_string()),
        ]),
        global_function_file_map: &HashMap::from([
            ("foo".to_string(), PathBuf::from("src/lib_foo.arden")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.arden")),
        ]),
        global_class_map: &HashMap::new(),
        global_class_file_map: &HashMap::new(),
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &HashMap::new(),
        global_enum_file_map: &HashMap::new(),
        global_module_map: &HashMap::new(),
        global_module_file_map: &HashMap::new(),
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
            function_map: &HashMap::from([
                ("foo".to_string(), "lib".to_string()),
                ("bar".to_string(), "lib".to_string()),
            ]),
            function_file_map: &HashMap::from([
                ("foo".to_string(), PathBuf::from("src/lib_foo.arden")),
                ("bar".to_string(), PathBuf::from("src/lib_bar.arden")),
            ]),
            class_map: &HashMap::new(),
            class_file_map: &HashMap::new(),
            interface_map: empty_global_interface_map(),
            interface_file_map: empty_global_interface_file_map(),
            enum_map: &HashMap::new(),
            enum_file_map: &HashMap::new(),
            module_map: &HashMap::new(),
            module_file_map: &HashMap::new(),
        })),
    };

    let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None, None);
    assert_eq!(
        graph.get(&app.file).cloned().unwrap_or_default(),
        HashSet::from([PathBuf::from("src/lib_foo.arden")])
    );
}

#[test]
fn dependency_graph_keeps_wildcard_namespace_dependencies_when_symbol_disappears() {
    let mut app = make_unit("src/main.arden", "app", &["lib.*"]);
    app.referenced_symbols = vec!["foo".to_string()];
    let mut foo = make_unit("src/lib_foo.arden", "lib", &[]);
    foo.function_names = vec!["other".to_string()];
    let mut bar = make_unit("src/lib_bar.arden", "lib", &[]);
    bar.function_names = vec!["bar".to_string()];
    let parsed_files = vec![app.clone(), foo, bar];
    let namespace_files_map = HashMap::from([
        ("app".to_string(), vec![PathBuf::from("src/main.arden")]),
        (
            "lib".to_string(),
            vec![
                PathBuf::from("src/lib_bar.arden"),
                PathBuf::from("src/lib_foo.arden"),
            ],
        ),
    ]);
    let _namespace_function_files = HashMap::from([(
        "lib".to_string(),
        HashMap::from([
            ("other".to_string(), PathBuf::from("src/lib_foo.arden")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.arden")),
        ]),
    )]);
    let ctx = DependencyResolutionContext {
        namespace_files_map: &namespace_files_map,
        global_function_map: &HashMap::from([
            ("other".to_string(), "lib".to_string()),
            ("bar".to_string(), "lib".to_string()),
        ]),
        global_function_file_map: &HashMap::from([
            ("other".to_string(), PathBuf::from("src/lib_foo.arden")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.arden")),
        ]),
        global_class_map: &HashMap::new(),
        global_class_file_map: &HashMap::new(),
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &HashMap::new(),
        global_enum_file_map: &HashMap::new(),
        global_module_map: &HashMap::new(),
        global_module_file_map: &HashMap::new(),
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
            function_map: &HashMap::from([
                ("other".to_string(), "lib".to_string()),
                ("bar".to_string(), "lib".to_string()),
            ]),
            function_file_map: &HashMap::from([
                ("other".to_string(), PathBuf::from("src/lib_foo.arden")),
                ("bar".to_string(), PathBuf::from("src/lib_bar.arden")),
            ]),
            class_map: &HashMap::new(),
            class_file_map: &HashMap::new(),
            interface_map: empty_global_interface_map(),
            interface_file_map: empty_global_interface_file_map(),
            enum_map: &HashMap::new(),
            enum_file_map: &HashMap::new(),
            module_map: &HashMap::new(),
            module_file_map: &HashMap::new(),
        })),
    };

    let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None, None);
    assert_eq!(
        graph.get(&app.file).cloned().unwrap_or_default(),
        HashSet::from([
            PathBuf::from("src/lib_bar.arden"),
            PathBuf::from("src/lib_foo.arden")
        ])
    );
}

#[test]
fn dependency_graph_keeps_nested_module_wildcard_namespace_dependencies_when_symbol_disappears() {
    let mut app = make_unit("src/main.arden", "app", &["app.U.*"]);
    app.referenced_symbols = vec!["id".to_string()];
    let mut helper = make_unit("src/helper.arden", "app", &[]);
    helper.module_names = vec!["U".to_string()];
    let parsed_files = vec![app.clone(), helper];
    let namespace_files_map = HashMap::from([
        (
            "app".to_string(),
            vec![
                PathBuf::from("src/helper.arden"),
                PathBuf::from("src/main.arden"),
            ],
        ),
        ("app.U".to_string(), vec![PathBuf::from("src/helper.arden")]),
    ]);
    let _namespace_module_files = HashMap::from([(
        "app".to_string(),
        HashMap::from([("U".to_string(), PathBuf::from("src/helper.arden"))]),
    )]);
    let global_module_map = HashMap::from([("U".to_string(), "app".to_string())]);
    let global_module_file_map =
        HashMap::from([("U".to_string(), PathBuf::from("src/helper.arden"))]);
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
    };

    let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None, None);
    assert_eq!(
        graph.get(&app.file).cloned().unwrap_or_default(),
        HashSet::from([PathBuf::from("src/helper.arden")])
    );
}

#[test]
fn parsed_dependency_graph_tracks_nested_module_wildcard_import_owner_files() {
    let temp_root = make_temp_project_root("nested-module-wildcard-dependency-graph");
    let main_file = temp_root.join("src/main.arden");
    let helper_file = temp_root.join("src/helper.arden");
    fs::write(
        &main_file,
        "package app;\nimport app.U.*;\nfunction main(): Integer { return id(1); }\n",
    )
    .must("write main");
    fs::write(
        &helper_file,
        "package app;\nmodule U { function id(x: Integer): Integer { return x + 1; } }\n",
    )
    .must("write helper");

    let parsed_files = vec![
        parse_project_unit(&temp_root, &main_file).must("parse main"),
        parse_project_unit(&temp_root, &helper_file).must("parse helper"),
    ];
    let (
        namespace_files_map,
        _namespace_function_files,
        _namespace_class_files,
        _namespace_module_files,
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
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
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
        })),
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.arden"),
        "package app;\nimport lib.*;\nfunction main(): Integer { return add(1); }\n",
    )
    .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial wildcard project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction other(x: Integer): Integer { return x + 1; }\n",
    )
    .must("rewrite helper without imported symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after wildcard-imported symbol removal");
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.arden"),
        "package app;\nimport app.U.*;\nfunction main(): Integer { return id(1); }\n",
    )
    .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function id(x: Integer): Integer { return x + 1; } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial nested module wildcard project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function plus(x: Integer): Integer { return x + 1; } }\n",
    )
    .must("rewrite helper without nested-module imported symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after nested-module wildcard-imported symbol removal");
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport app.U.*;\nfunction main(): Integer { f: (Integer) -> Integer = id<Integer>; return f(1); }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function id<T>(x: T): T { return x; } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial nested module wildcard generic function value build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function plus<T>(x: T): T { return x; } }\n",
    )
    .must("rewrite helper without nested-module wildcard generic symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after nested-module wildcard generic symbol removal");
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.arden"),
        "package app;\nimport app.U.id as ident;\nfunction main(): Integer { return ident(1); }\n",
    )
    .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function id(x: Integer): Integer { return x + 1; } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial nested exact-import alias project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function plus(x: Integer): Integer { return x + 1; } }\n",
    )
    .must("rewrite helper without nested exact-import alias symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after nested exact-import alias symbol removal");
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport app.U.id as ident;\nfunction main(): Integer {\n    f: (Integer) -> Integer = ident;\n    return f(1);\n}\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function id(x: Integer): Integer { return x + 1; } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial stale nested exact-import alias function-value build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function plus(x: Integer): Integer { return x + 1; } }\n",
    )
    .must("rewrite helper without nested exact-import alias function-value symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).must_err(
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.arden"),
        "package app;\nimport lib as l;\nfunction main(): Integer { return l.add(1); }\n",
    )
    .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial namespace-alias project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction other(x: Integer): Integer { return x + 1; }\n",
    )
    .must("rewrite helper without namespace-alias symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after namespace-alias symbol removal");
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.arden"),
        "package app;\nimport app as root;\nfunction main(): Integer { return root.U.id(1); }\n",
    )
    .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function id(x: Integer): Integer { return x + 1; } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial stale root namespace alias call build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function plus(x: Integer): Integer { return x + 1; } }\n",
    )
    .must("rewrite helper without stale root namespace alias call symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after stale root namespace alias call symbol removal");
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport app as root;\nfunction main(): Integer { return match (root.M.E.A(5)) { root.M.E.A(v) => v, root.M.E.B(v) => v, }; }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule M { enum E { A(Integer), B(Integer) } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial root namespace alias nested enum build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule M { enum E { C(Integer), D(Integer) } }\n",
    )
    .must("rewrite helper without nested enum alias variant");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).must_err(
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport app.U.id as ident;\nfunction main(): Integer { f: (Integer) -> Integer = ident<Integer>; return f(7); }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function id<T>(value: T): T { return value; } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial exact-import alias generic function value build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function other<T>(value: T): T { return value; } }\n",
    )
    .must("rewrite helper without exact-import alias generic symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).must_err(
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport app.M.E.B as Variant;\nfunction main(): None { e: M.E = Variant(2); match (e) { Variant(v) => { require(v == 2); } M.E.A(v) => { require(false); } } return None; }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule M { enum E { A(Integer) B(Integer) } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial exact imported nested enum variant alias build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule M { enum E { C(Integer) D(Integer) } }\n",
    )
    .must("rewrite helper without exact imported nested enum variant alias");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).must_err(
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport app.M.E as Enum;\nfunction main(): Integer { value: Enum = Enum.B(2); return 0; }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule M { enum E { A(Integer), B(Integer) } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial exact imported nested enum alias type build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule M { enum F { A(Integer), B(Integer) } }\n",
    )
    .must("rewrite helper without exact imported nested enum alias type");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).must_err(
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport app.M.*;\nfunction main(): Integer { value: E = E.B(2); return 0; }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule M { enum E { A(Integer), B(Integer) } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial wildcard imported nested enum type build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule M { enum F { A(Integer), B(Integer) } }\n",
    )
    .must("rewrite helper without wildcard imported nested enum type");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after stale wildcard imported nested enum type removal");
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport app as root;\nfunction main(): Integer { f: (Integer) -> Integer = root.U.id<Integer>; return f(7); }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function id<T>(value: T): T { return value; } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial root namespace alias generic function value build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function other<T>(value: T): T { return value; } }\n",
    )
    .must("rewrite helper without root namespace alias generic symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false).must_err(
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
            temp_root.join("src/main.arden"),
            "package app;\nimport app as root;\nfunction main(): Integer { f: (Integer) -> Integer = root.U.id; return f(1); }\n",
        )
        .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function id(x: Integer): Integer { return x + 1; } }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial root namespace alias function value build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package app;\nmodule U { function plus(x: Integer): Integer { return x + 1; } }\n",
    )
    .must("rewrite helper without root namespace alias function value symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after stale root namespace alias function value removal");
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.arden"),
        "package app;\nimport lib.add as inc;\nfunction main(): Integer { return inc(1); }\n",
    )
    .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial exact-import-alias project build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction other(x: Integer): Integer { return x + 1; }\n",
    )
    .must("rewrite helper without exact-import alias symbol");

    with_current_dir(&temp_root, || {
        let err = build_project(false, false, true, false, false)
            .must_err("build should fail after exact-import alias symbol removal");
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
        &["src/main.arden", "src/helper.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        temp_root.join("src/main.arden"),
        r#"
package app;
import lib.add as inc;

function main(): Integer {
    inc: (Integer) -> Integer = (x: Integer) => x + 10;
    return inc(1);
}
"#,
    )
    .must("write main");
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("write helper");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("initial exact-import alias shadowing build should succeed");
    });

    std::thread::sleep(std::time::Duration::from_millis(5));
    fs::write(
        temp_root.join("src/helper.arden"),
        "package lib;\nfunction other(x: Integer): Integer { return x + 1; }\n",
    )
    .must("rewrite helper without shadowed exact-import alias symbol");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("shadowed exact-import alias local call should stay valid after symbol removal");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled shadowed exact-import alias binary");
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
    write_test_project_config(&temp_root, &["src/main.arden"], "src/main.arden", "smoke");
    fs::write(
        temp_root.join("src/main.arden"),
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
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should succeed for module-local nested constructor and fn value");
    });

    let output = std::process::Command::new(temp_root.join("smoke"))
        .output()
        .must("run compiled nested module project binary");
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
    let mut app = make_unit("src/app.arden", "app", &["lib.foo"]);
    app.api_fingerprint = "app-v1".to_string();
    app.semantic_fingerprint = "app-v1".to_string();
    let mut foo = make_unit("src/lib_foo.arden", "lib", &[]);
    foo.function_names = vec!["foo".to_string()];
    foo.api_fingerprint = "foo-v2".to_string();
    foo.semantic_fingerprint = "foo-v2".to_string();

    let previous = DependencyGraphCache {
        schema: DEPENDENCY_GRAPH_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        entry_namespace: "app".to_string(),
        files: vec![
            DependencyGraphFileEntry {
                file: PathBuf::from("src/app.arden"),
                semantic_fingerprint: "app-v1".to_string(),
                api_fingerprint: "app-v1".to_string(),
                direct_dependencies: vec![PathBuf::from("src/lib_foo.arden")],
            },
            DependencyGraphFileEntry {
                file: PathBuf::from("src/lib_foo.arden"),
                semantic_fingerprint: "foo-v1".to_string(),
                api_fingerprint: "foo-v1".to_string(),
                direct_dependencies: vec![],
            },
        ],
    };

    let parsed_files = vec![app.clone(), foo.clone()];
    let namespace_files_map = HashMap::from([
        ("app".to_string(), vec![PathBuf::from("src/app.arden")]),
        ("lib".to_string(), vec![PathBuf::from("src/lib_foo.arden")]),
    ]);
    let _namespace_function_files = HashMap::from([(
        "lib".to_string(),
        HashMap::from([("foo".to_string(), PathBuf::from("src/lib_foo.arden"))]),
    )]);
    let ctx = DependencyResolutionContext {
        namespace_files_map: &namespace_files_map,
        global_function_map: &HashMap::from([("foo".to_string(), "lib".to_string())]),
        global_function_file_map: &HashMap::from([(
            "foo".to_string(),
            PathBuf::from("src/lib_foo.arden"),
        )]),
        global_class_map: &HashMap::new(),
        global_class_file_map: &HashMap::new(),
        global_interface_map: empty_global_interface_map(),
        global_interface_file_map: empty_global_interface_file_map(),
        global_enum_map: &HashMap::new(),
        global_enum_file_map: &HashMap::new(),
        global_module_map: &HashMap::new(),
        global_module_file_map: &HashMap::new(),
        symbol_lookup: Arc::new(build_project_symbol_lookup(&ProjectSymbolLookupMaps {
            function_map: &HashMap::from([("foo".to_string(), "lib".to_string())]),
            function_file_map: &HashMap::from([(
                "foo".to_string(),
                PathBuf::from("src/lib_foo.arden"),
            )]),
            class_map: &HashMap::new(),
            class_file_map: &HashMap::new(),
            interface_map: empty_global_interface_map(),
            interface_file_map: empty_global_interface_file_map(),
            enum_map: &HashMap::new(),
            enum_file_map: &HashMap::new(),
            module_map: &HashMap::new(),
            module_file_map: &HashMap::new(),
        })),
    };

    let (_, reused) =
        build_file_dependency_graph_incremental(&parsed_files, &ctx, Some(&previous), None);
    assert_eq!(reused, 0);
}

#[test]
fn typecheck_summary_cache_matches_identical_component_fingerprints() {
    let current = HashMap::from([
        (PathBuf::from("a.arden"), "sem-a".to_string()),
        (PathBuf::from("b.arden"), "sem-b".to_string()),
    ]);
    let components = vec![
        vec![PathBuf::from("a.arden")],
        vec![PathBuf::from("b.arden")],
    ];
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
            (PathBuf::from("a.arden"), "sem-a".to_string()),
            (PathBuf::from("b.arden"), "sem-b".to_string()),
            (PathBuf::from("c.arden"), "sem-c-old".to_string()),
        ]),
        &[
            vec![PathBuf::from("a.arden"), PathBuf::from("b.arden")],
            vec![PathBuf::from("c.arden")],
        ],
    );
    let current = HashMap::from([
        (PathBuf::from("a.arden"), "sem-a".to_string()),
        (PathBuf::from("b.arden"), "sem-b".to_string()),
        (PathBuf::from("c.arden"), "sem-c-new".to_string()),
    ]);
    let components = vec![
        vec![PathBuf::from("a.arden"), PathBuf::from("b.arden")],
        vec![PathBuf::from("c.arden")],
    ];

    let reusable = reusable_component_fingerprints(&previous, &current, &components);

    assert_eq!(reusable.len(), 1);
    assert!(reusable.contains(&component_fingerprint(&components[0], &current)));
}

#[test]
fn reverse_dependency_graph_returns_only_transitive_dependents() {
    let reverse = build_reverse_dependency_graph(&HashMap::from([
        (
            PathBuf::from("a.arden"),
            HashSet::from([PathBuf::from("b.arden")]),
        ),
        (
            PathBuf::from("c.arden"),
            HashSet::from([PathBuf::from("a.arden")]),
        ),
        (PathBuf::from("d.arden"), HashSet::new()),
    ]));

    let impacted = transitive_dependents(&reverse, &HashSet::from([PathBuf::from("b.arden")]));

    assert_eq!(
        impacted,
        HashSet::from([
            PathBuf::from("b.arden"),
            PathBuf::from("a.arden"),
            PathBuf::from("c.arden"),
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
    let temp =
        std::env::temp_dir().join(format!("arden-link-manifest-test-{}", std::process::id()));
    fs::write(&temp, b"bin").must("write output placeholder");
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
    let a = vec![PathBuf::from("src/a.arden"), PathBuf::from("src/b.arden")];
    let b = vec![PathBuf::from("src/b.arden"), PathBuf::from("src/a.arden")];

    assert_eq!(object_shard_cache_key(&a), object_shard_cache_key(&b));
}

#[test]
fn object_shard_cache_hit_ignores_member_order() {
    let temp_root = make_temp_project_root("object-shard-cache-member-order");
    let files_ab = vec![PathBuf::from("src/a.arden"), PathBuf::from("src/b.arden")];
    let files_ba = vec![PathBuf::from("src/b.arden"), PathBuf::from("src/a.arden")];
    let cache_paths = object_shard_cache_paths(&temp_root, &files_ab);
    let fingerprint = "obj-fp";
    let members_ab = vec![
        ObjectShardMemberFingerprint {
            file: PathBuf::from("src/a.arden"),
            semantic_fingerprint: "sem-a".to_string(),
            rewrite_context_fingerprint: "rw-a".to_string(),
        },
        ObjectShardMemberFingerprint {
            file: PathBuf::from("src/b.arden"),
            semantic_fingerprint: "sem-b".to_string(),
            rewrite_context_fingerprint: "rw-b".to_string(),
        },
    ];
    let members_ba = vec![
        ObjectShardMemberFingerprint {
            file: PathBuf::from("src/b.arden"),
            semantic_fingerprint: "sem-b".to_string(),
            rewrite_context_fingerprint: "rw-b".to_string(),
        },
        ObjectShardMemberFingerprint {
            file: PathBuf::from("src/a.arden"),
            semantic_fingerprint: "sem-a".to_string(),
            rewrite_context_fingerprint: "rw-a".to_string(),
        },
    ];

    fs::create_dir_all(
        cache_paths
            .object_path
            .parent()
            .must("object shard cache path should have parent"),
    )
    .must("create object shard cache directory");
    fs::write(&cache_paths.object_path, b"obj").must("write cached object shard");
    save_object_shard_cache_meta(&cache_paths, &members_ab, fingerprint)
        .must("save object shard meta");

    let reordered_cache_paths = object_shard_cache_paths(&temp_root, &files_ba);
    let hit = load_object_shard_cache_hit(&reordered_cache_paths, &members_ba, fingerprint)
        .must("load object shard cache hit");
    assert_eq!(hit, Some(reordered_cache_paths.object_path.clone()));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn load_link_manifest_cache_reports_io_errors_instead_of_silent_cache_miss() {
    let temp_root = make_temp_project_root("link-manifest-io-error");
    let manifest_path = temp_root
        .join(".ardencache")
        .join("link")
        .join("latest.json");
    fs::create_dir_all(&manifest_path).must("create manifest path as directory");

    let err = load_link_manifest_cache(&temp_root)
        .must_err("directory-shaped manifest path should surface an io error");
    assert!(err.contains("link manifest cache"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn read_cache_blob_treats_invalid_payload_as_cache_miss() {
    let temp_root = make_temp_project_root("cache-decode-error");
    let cache_path = temp_root
        .join(".ardencache")
        .join("parsed")
        .join("broken.bin");
    fs::create_dir_all(
        cache_path
            .parent()
            .must("cache path should have parent directory"),
    )
    .must("create cache dir");
    fs::write(&cache_path, b"not valid bincode").must("write invalid cache payload");

    let cache = read_cache_blob::<ParsedFileCacheEntry>(&cache_path, "parse cache")
        .must("invalid cache payload should be treated as a cache miss");
    assert!(
        cache.is_none(),
        "invalid cache payload should be ignored as a cache miss"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn load_cached_fingerprint_reports_io_errors_instead_of_silent_cache_miss() {
    let temp_root = make_temp_project_root("build-fingerprint-io-error");
    let cache_path = temp_root.join(".ardencache").join("build_fingerprint");
    fs::create_dir_all(&cache_path).must("create directory-shaped build cache path");

    let err = load_cached_fingerprint(&temp_root)
        .must_err("directory-shaped build fingerprint path should surface an io error");
    assert!(err.contains("Failed to read build cache"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn load_semantic_cached_fingerprint_reports_io_errors_instead_of_silent_cache_miss() {
    let temp_root = make_temp_project_root("semantic-fingerprint-io-error");
    let cache_path = temp_root
        .join(".ardencache")
        .join("semantic_build_fingerprint");
    fs::create_dir_all(&cache_path).must("create directory-shaped semantic cache path");

    let err = load_semantic_cached_fingerprint(&temp_root)
        .must_err("directory-shaped semantic fingerprint path should surface an io error");
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
    let main_file = src_dir.join("main.arden");
    let helper_file = src_dir.join("helper.arden");

    fs::write(
            &main_file,
            "package app;\nimport lib.math;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .must("write main file");
    fs::write(
        &helper_file,
        "package lib;\nfunction add(: Integer { return 1; }\n",
    )
    .must("write invalid helper file");

    let first_main = parse_project_unit(&temp_root, &main_file).must("first main parse");
    let first_helper_err =
        parse_project_unit(&temp_root, &helper_file).must_err("invalid helper should fail parsing");
    assert!(
        first_helper_err.contains("Parse error")
            || first_helper_err.contains("Expected an identifier"),
        "{first_helper_err}"
    );
    assert!(!first_main.from_parse_cache);

    thread::sleep(Duration::from_millis(5));
    fs::write(
        &helper_file,
        "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
    )
    .must("rewrite helper file");

    let second_main = parse_project_unit(&temp_root, &main_file).must("second main parse");
    let second_helper = parse_project_unit(&temp_root, &helper_file).must("second helper parse");
    assert!(second_main.from_parse_cache);
    assert!(!second_helper.from_parse_cache);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn parse_cache_reuses_same_content_even_after_metadata_change() {
    let temp_root = std::env::temp_dir().join(format!(
        "arden-parse-cache-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .must("time")
            .as_nanos()
    ));
    let src_dir = temp_root.join("src");
    fs::create_dir_all(&src_dir).must("create temp src dir");
    let file = src_dir.join("main.arden");
    let source = "function main(): None { return None; }\n";
    fs::write(&file, source).must("write source");

    let first = parse_project_unit(&temp_root, &file).must("first parse");
    assert!(!first.from_parse_cache);

    thread::sleep(Duration::from_millis(5));
    fs::write(&file, source).must("rewrite identical source");

    let second = parse_project_unit(&temp_root, &file).must("second parse");
    assert!(second.from_parse_cache);
    assert_eq!(first.semantic_fingerprint, second.semantic_fingerprint);

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn response_file_args_escape_quotes_and_backslashes() {
    assert_eq!(
        escape_response_file_arg("C:\\tmp\\a \"b\".o"),
        "\"C:\\tmp\\a \\\"b\\\".o\""
    );
}

#[test]
fn precompute_transitive_dependencies_matches_expected_closure() {
    let graph = HashMap::from([
        (
            PathBuf::from("a.arden"),
            HashSet::from([PathBuf::from("b.arden"), PathBuf::from("c.arden")]),
        ),
        (
            PathBuf::from("b.arden"),
            HashSet::from([PathBuf::from("d.arden")]),
        ),
        (
            PathBuf::from("c.arden"),
            HashSet::from([PathBuf::from("d.arden")]),
        ),
        (PathBuf::from("d.arden"), HashSet::new()),
    ]);

    let all = precompute_all_transitive_dependencies(&graph);
    assert_eq!(
        transitive_dependencies_from_precomputed(&all, Path::new("a.arden")),
        HashSet::from([
            PathBuf::from("b.arden"),
            PathBuf::from("c.arden"),
            PathBuf::from("d.arden"),
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
        make_unit("a.arden", "fa", 3, 0, 1, false),
        make_unit("b.arden", "fb", 4, 0, 1, false),
        make_unit("c.arden", "fc", 5, 0, 1, true),
    ];
    let rewritten_file_indices = HashMap::from([
        (PathBuf::from("a.arden"), 0usize),
        (PathBuf::from("b.arden"), 1usize),
        (PathBuf::from("c.arden"), 2usize),
    ]);
    let program = codegen_program_for_unit(
        &rewritten_files,
        &rewritten_file_indices,
        Path::new("a.arden"),
        Some(&HashSet::from([PathBuf::from("b.arden")])),
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
