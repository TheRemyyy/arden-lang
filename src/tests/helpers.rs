#![allow(unused_variables)]

use crate::ast::Program;
use crate::borrowck::BorrowChecker;
use crate::formatter::format_program_canonical;
use crate::parser::Parser;
use crate::typeck::TypeChecker;
use crate::{
    build_project_symbol_lookup, compute_namespace_api_fingerprints,
    compute_rewrite_context_fingerprint_for_unit, semantic_program_fingerprint, ParsedProjectUnit,
    RewriteFingerprintContext,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) struct ProjectSymbolMaps {
    pub(crate) namespace_files_map: HashMap<String, Vec<PathBuf>>,
    pub(crate) namespace_function_files: HashMap<String, HashMap<String, PathBuf>>,
    pub(crate) namespace_class_files: HashMap<String, HashMap<String, PathBuf>>,
    pub(crate) namespace_module_files: HashMap<String, HashMap<String, PathBuf>>,
    pub(crate) global_function_map: HashMap<String, String>,
    pub(crate) global_function_file_map: HashMap<String, PathBuf>,
    pub(crate) global_class_map: HashMap<String, String>,
    pub(crate) global_class_file_map: HashMap<String, PathBuf>,
    pub(crate) global_enum_map: HashMap<String, String>,
    pub(crate) global_enum_file_map: HashMap<String, PathBuf>,
    pub(crate) global_module_map: HashMap<String, String>,
    pub(crate) global_module_file_map: HashMap<String, PathBuf>,
}

impl ProjectSymbolMaps {
    #[allow(clippy::type_complexity)]
    pub(crate) fn into_parts(
        self,
    ) -> (
        HashMap<String, Vec<PathBuf>>,
        HashMap<String, HashMap<String, PathBuf>>,
        HashMap<String, HashMap<String, PathBuf>>,
        HashMap<String, HashMap<String, PathBuf>>,
        HashMap<String, String>,
        HashMap<String, PathBuf>,
        HashMap<String, String>,
        HashMap<String, PathBuf>,
        HashMap<String, String>,
        HashMap<String, PathBuf>,
        HashMap<String, String>,
        HashMap<String, PathBuf>,
    ) {
        (
            self.namespace_files_map,
            self.namespace_function_files,
            self.namespace_class_files,
            self.namespace_module_files,
            self.global_function_map,
            self.global_function_file_map,
            self.global_class_map,
            self.global_class_file_map,
            self.global_enum_map,
            self.global_enum_file_map,
            self.global_module_map,
            self.global_module_file_map,
        )
    }
}

pub(crate) fn parse_program(source: &str) -> Program {
    let tokens = crate::lexer::tokenize(source).expect("tokenize");
    let mut parser = Parser::new(tokens);
    parser.parse_program().expect("parse")
}

pub(crate) fn fingerprint_for(source: &str) -> String {
    let program = parse_program(source);
    semantic_program_fingerprint(&program)
}

pub(crate) fn rewrite_fingerprint_for_test_unit(
    parsed_files: &[ParsedProjectUnit],
    target_file: &Path,
    entry_namespace: &str,
) -> String {
    let symbol_maps = collect_project_symbol_maps(parsed_files);
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
    let namespace_api_fingerprints = compute_namespace_api_fingerprints(parsed_files);
    let file_api_fingerprints = parsed_files
        .iter()
        .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
        .collect::<HashMap<_, _>>();
    let namespace_interface_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let global_interface_map: HashMap<String, String> = HashMap::new();
    let global_interface_file_map: HashMap<String, PathBuf> = HashMap::new();
    let symbol_lookup = Arc::new(build_project_symbol_lookup(
        &crate::dependency::ProjectSymbolMaps {
            function_map: &symbol_maps.global_function_map,
            function_file_map: &symbol_maps.global_function_file_map,
            class_map: &symbol_maps.global_class_map,
            class_file_map: &symbol_maps.global_class_file_map,
            interface_map: &global_interface_map,
            interface_file_map: &global_interface_file_map,
            enum_map: &symbol_maps.global_enum_map,
            enum_file_map: &symbol_maps.global_enum_file_map,
            module_map: &symbol_maps.global_module_map,
            module_file_map: &symbol_maps.global_module_file_map,
        },
    ));
    let rewrite_ctx = RewriteFingerprintContext {
        namespace_functions: &namespace_functions,
        global_function_map: &symbol_maps.global_function_map,
        global_function_file_map: &symbol_maps.global_function_file_map,
        namespace_classes: &namespace_classes,
        global_class_map: &symbol_maps.global_class_map,
        global_class_file_map: &symbol_maps.global_class_file_map,
        global_interface_map: &global_interface_map,
        global_interface_file_map: &global_interface_file_map,
        global_enum_map: &symbol_maps.global_enum_map,
        global_enum_file_map: &symbol_maps.global_enum_file_map,
        namespace_modules: &namespace_modules,
        global_module_map: &symbol_maps.global_module_map,
        global_module_file_map: &symbol_maps.global_module_file_map,
        namespace_api_fingerprints: &namespace_api_fingerprints,
        file_api_fingerprints: &file_api_fingerprints,
        symbol_lookup: Arc::clone(&symbol_lookup),
    };
    let target_unit = parsed_files
        .iter()
        .find(|u| u.file == target_file)
        .expect("target unit");
    compute_rewrite_context_fingerprint_for_unit(target_unit, entry_namespace, &rewrite_ctx)
}

pub(crate) fn assert_frontend_pipeline_ok(source: &str) {
    let program = parse_program(source);

    let mut type_checker = TypeChecker::new();
    assert_diagnostics_ok(type_checker.check(&program), "type check failed", |error| {
        error.message
    });

    let mut borrow_checker = BorrowChecker::new();
    assert_diagnostics_ok(
        borrow_checker.check(&program),
        "borrow check failed",
        |error| error.message,
    );

    let formatted = format_program_canonical(&program);
    let reparsed = parse_program(&formatted);

    let mut type_checker = TypeChecker::new();
    assert_diagnostics_ok(
        type_checker.check(&reparsed),
        "type check after format failed",
        |error| error.message,
    );

    let mut borrow_checker = BorrowChecker::new();
    assert_diagnostics_ok(
        borrow_checker.check(&reparsed),
        "borrow check after format failed",
        |error| error.message,
    );
}

fn assert_diagnostics_ok<T, E>(
    result: std::result::Result<T, Vec<E>>,
    context: &str,
    mut message: impl FnMut(E) -> String,
) {
    if let Err(errors) = result {
        panic!(
            "{}: {}",
            context,
            errors
                .into_iter()
                .map(&mut message)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

pub(crate) fn make_temp_project_root(tag: &str) -> PathBuf {
    let base_temp = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|_| std::env::temp_dir());
    let temp_root = base_temp.join(format!(
        "arden-project-smoke-{}-{}-{}",
        tag,
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    fs::create_dir_all(temp_root.join("src")).expect("create temp project src dir");
    temp_root
}

pub(crate) fn normalize_output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

pub(crate) fn cli_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub(crate) struct CwdRestore {
    previous: PathBuf,
}

fn fallback_working_dir() -> PathBuf {
    std::env::temp_dir()
}

fn capture_working_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| fallback_working_dir())
}

impl Drop for CwdRestore {
    fn drop(&mut self) {
        if std::env::set_current_dir(&self.previous).is_err() {
            let _ = std::env::set_current_dir(fallback_working_dir());
        }
    }
}

pub(crate) fn with_current_dir<T>(dir: &Path, f: impl FnOnce() -> T) -> T {
    let _lock = cli_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let previous = capture_working_dir();
    std::env::set_current_dir(dir).expect("set current dir");
    let _restore = CwdRestore { previous };
    f()
}

pub(crate) fn normalize_nested_cargo_linker_env(command: &mut Command) {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (key, value) in std::env::vars_os() {
        let key_text = key.to_string_lossy();
        if !key_text.starts_with("CARGO_TARGET_") || !key_text.ends_with("_LINKER") {
            continue;
        }

        let linker = PathBuf::from(&value);
        let linker_text = linker.to_string_lossy();
        let looks_like_relative_path = linker.is_relative()
            && (linker_text.starts_with('.')
                || linker_text.contains('/')
                || linker_text.contains('\\'));
        if looks_like_relative_path {
            command.env(&key, repo_root.join(linker));
        }
    }
}

pub(crate) fn write_test_project_config(root: &Path, files: &[&str], entry: &str, output: &str) {
    let files_toml = files
        .iter()
        .map(|file| format!("\"{}\"", file))
        .collect::<Vec<_>>()
        .join(", ");
    let config = format!(
        "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"{}\"\nfiles = [{}]\noutput = \"{}\"\n",
        entry, files_toml, output
    );
    fs::write(root.join("arden.toml"), config).expect("write arden.toml");
}

pub(crate) fn collect_project_symbol_maps(parsed_files: &[ParsedProjectUnit]) -> ProjectSymbolMaps {
    let mut namespace_files_map = HashMap::new();
    let mut namespace_function_files = HashMap::new();
    let mut namespace_class_files = HashMap::new();
    let mut namespace_module_files = HashMap::new();
    let mut global_function_map = HashMap::new();
    let mut global_function_file_map = HashMap::new();
    let mut global_class_map = HashMap::new();
    let mut global_class_file_map = HashMap::new();
    let mut global_enum_map = HashMap::new();
    let mut global_enum_file_map = HashMap::new();
    let mut global_module_map = HashMap::new();
    let mut global_module_file_map = HashMap::new();

    for unit in parsed_files {
        namespace_files_map
            .entry(unit.namespace.clone())
            .or_insert_with(Vec::new)
            .push(unit.file.clone());
        for module_name in &unit.module_names {
            namespace_files_map
                .entry(format!(
                    "{}.{}",
                    unit.namespace,
                    module_name.replace("__", ".")
                ))
                .or_insert_with(Vec::new)
                .push(unit.file.clone());
        }
        for name in &unit.function_names {
            namespace_function_files
                .entry(unit.namespace.clone())
                .or_insert_with(HashMap::new)
                .insert(name.clone(), unit.file.clone());
            global_function_map.insert(name.clone(), unit.namespace.clone());
            global_function_file_map.insert(name.clone(), unit.file.clone());
        }
        for name in &unit.class_names {
            namespace_class_files
                .entry(unit.namespace.clone())
                .or_insert_with(HashMap::new)
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
                .or_insert_with(HashMap::new)
                .insert(name.clone(), unit.file.clone());
            global_module_map.insert(name.clone(), unit.namespace.clone());
            global_module_file_map.insert(name.clone(), unit.file.clone());
        }
    }

    for files in namespace_files_map.values_mut() {
        files.sort();
        files.dedup();
    }

    ProjectSymbolMaps {
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
    }
}
