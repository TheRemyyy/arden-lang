use super::parse_index_metrics::{record_parse_index_metrics, ParseIndexMetrics};
use super::parse_index_types::ParseIndexOutputs;
use crate::cache::{
    elapsed_nanos_u64, BuildTimings, ExactSymbolLookup, ParsedProjectUnit, ProjectSymbolLookup,
    WildcardMemberLookup,
};
use crate::dependency::{register_global_symbol, GlobalSymbolRegistrationContext};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug)]
enum ParseIndexPhaseError {
    ParseAndScan(String),
}

impl fmt::Display for ParseIndexPhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseAndScan(message) => write!(f, "{message}"),
        }
    }
}

impl From<ParseIndexPhaseError> for String {
    fn from(value: ParseIndexPhaseError) -> Self {
        value.to_string()
    }
}

impl From<String> for ParseIndexPhaseError {
    fn from(value: String) -> Self {
        Self::ParseAndScan(value)
    }
}

pub(crate) fn run_parse_index_phase(
    build_timings: &mut BuildTimings,
    project_root: &Path,
    files: &[PathBuf],
) -> Result<ParseIndexOutputs, String> {
    run_parse_index_phase_impl(build_timings, project_root, files).map_err(Into::into)
}

fn run_parse_index_phase_impl(
    build_timings: &mut BuildTimings,
    project_root: &Path,
    files: &[PathBuf],
) -> Result<ParseIndexOutputs, ParseIndexPhaseError> {
    let mut parsed_files: Vec<ParsedProjectUnit> = Vec::new();
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
    let mut project_symbol_lookup_exact: ExactSymbolLookup = HashMap::new();
    let mut project_symbol_lookup_wildcard_members: WildcardMemberLookup = HashMap::new();
    let mut namespace_class_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut namespace_interface_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut namespace_enum_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut namespace_module_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut function_collisions: Vec<(String, String, String)> = Vec::new();
    let mut class_collisions: Vec<(String, String, String)> = Vec::new();
    let mut interface_collisions: Vec<(String, String, String)> = Vec::new();
    let mut enum_collisions: Vec<(String, String, String)> = Vec::new();
    let mut module_collisions: Vec<(String, String, String)> = Vec::new();
    let mut parse_cache_hits = 0_usize;

    let mut parsed_units: Vec<ParsedProjectUnit> =
        build_timings.measure("parse + symbol scan", || {
            files
                .par_iter()
                .map(|file| crate::parse_project_unit(project_root, file).map_err(Into::into))
                .collect::<Result<Vec<_>, ParseIndexPhaseError>>()
        })?;
    parsed_units.sort_by(|a, b| a.file.cmp(&b.file));

    let total_function_names: usize = parsed_units
        .iter()
        .map(|unit| unit.function_names.len())
        .sum();
    let total_class_names: usize = parsed_units.iter().map(|unit| unit.class_names.len()).sum();
    let total_interface_names: usize = parsed_units
        .iter()
        .map(|unit| unit.interface_names.len())
        .sum();
    let total_enum_names: usize = parsed_units.iter().map(|unit| unit.enum_names.len()).sum();
    let total_module_names: usize = parsed_units
        .iter()
        .map(|unit| unit.module_names.len())
        .sum();
    let needs_project_symbol_lookup = parsed_units.iter().any(|unit| {
        unit.imports
            .iter()
            .any(|import| !import.path.starts_with("std."))
    });

    global_function_map.reserve(total_function_names);
    global_function_file_map.reserve(total_function_names);
    global_class_map.reserve(total_class_names);
    global_class_file_map.reserve(total_class_names);
    global_interface_map.reserve(total_interface_names);
    global_interface_file_map.reserve(total_interface_names);
    global_enum_map.reserve(total_enum_names);
    global_enum_file_map.reserve(total_enum_names);
    global_module_map.reserve(total_module_names);
    global_module_file_map.reserve(total_module_names);
    if needs_project_symbol_lookup {
        let total_named_symbols = total_function_names
            + total_class_names
            + total_interface_names
            + total_enum_names
            + total_module_names;
        project_symbol_lookup_exact.reserve(total_named_symbols);
        project_symbol_lookup_wildcard_members.reserve(total_named_symbols);
    }
    namespace_class_map.reserve(parsed_units.len());
    namespace_interface_map.reserve(parsed_units.len());
    namespace_enum_map.reserve(parsed_units.len());
    namespace_module_map.reserve(parsed_units.len());

    let mut parse_index_namespace_sets_ns = 0_u64;
    let mut parse_index_function_register_ns = 0_u64;
    let mut parse_index_class_register_ns = 0_u64;
    let mut parse_index_interface_register_ns = 0_u64;
    let mut parse_index_enum_register_ns = 0_u64;
    let mut parse_index_module_register_ns = 0_u64;
    let mut parse_index_parsed_file_push_ns = 0_u64;

    build_timings.measure_step("parse index assembly", || {
        for unit in parsed_units {
            if unit.from_parse_cache {
                parse_cache_hits += 1;
            }

            let namespace_sets_started_at = Instant::now();
            parse_index_function_register_ns += register_symbol_batch(
                &unit.function_names,
                &unit.namespace,
                &unit.file,
                SymbolBatchContext {
                    namespace_map: None,
                    global_map: &mut global_function_map,
                    global_file_map: &mut global_function_file_map,
                    collisions: &mut function_collisions,
                    exact_lookup: &mut project_symbol_lookup_exact,
                    wildcard_lookup: &mut project_symbol_lookup_wildcard_members,
                    build_symbol_lookup: needs_project_symbol_lookup,
                },
            );
            parse_index_class_register_ns += register_symbol_batch(
                &unit.class_names,
                &unit.namespace,
                &unit.file,
                SymbolBatchContext {
                    namespace_map: Some(&mut namespace_class_map),
                    global_map: &mut global_class_map,
                    global_file_map: &mut global_class_file_map,
                    collisions: &mut class_collisions,
                    exact_lookup: &mut project_symbol_lookup_exact,
                    wildcard_lookup: &mut project_symbol_lookup_wildcard_members,
                    build_symbol_lookup: needs_project_symbol_lookup,
                },
            );
            parse_index_interface_register_ns += register_symbol_batch(
                &unit.interface_names,
                &unit.namespace,
                &unit.file,
                SymbolBatchContext {
                    namespace_map: Some(&mut namespace_interface_map),
                    global_map: &mut global_interface_map,
                    global_file_map: &mut global_interface_file_map,
                    collisions: &mut interface_collisions,
                    exact_lookup: &mut project_symbol_lookup_exact,
                    wildcard_lookup: &mut project_symbol_lookup_wildcard_members,
                    build_symbol_lookup: needs_project_symbol_lookup,
                },
            );
            parse_index_enum_register_ns += register_symbol_batch(
                &unit.enum_names,
                &unit.namespace,
                &unit.file,
                SymbolBatchContext {
                    namespace_map: Some(&mut namespace_enum_map),
                    global_map: &mut global_enum_map,
                    global_file_map: &mut global_enum_file_map,
                    collisions: &mut enum_collisions,
                    exact_lookup: &mut project_symbol_lookup_exact,
                    wildcard_lookup: &mut project_symbol_lookup_wildcard_members,
                    build_symbol_lookup: needs_project_symbol_lookup,
                },
            );
            parse_index_module_register_ns += register_symbol_batch(
                &unit.module_names,
                &unit.namespace,
                &unit.file,
                SymbolBatchContext {
                    namespace_map: Some(&mut namespace_module_map),
                    global_map: &mut global_module_map,
                    global_file_map: &mut global_module_file_map,
                    collisions: &mut module_collisions,
                    exact_lookup: &mut project_symbol_lookup_exact,
                    wildcard_lookup: &mut project_symbol_lookup_wildcard_members,
                    build_symbol_lookup: needs_project_symbol_lookup,
                },
            );
            parse_index_namespace_sets_ns += elapsed_nanos_u64(namespace_sets_started_at);

            let push_started_at = Instant::now();
            parsed_files.push(unit);
            parse_index_parsed_file_push_ns += elapsed_nanos_u64(push_started_at);
        }
    });

    record_parse_index_metrics(
        build_timings,
        ParseIndexMetrics {
            files_len: files.len(),
            parse_cache_hits,
            total_function_names,
            total_class_names,
            total_interface_names,
            total_enum_names,
            total_module_names,
            needs_project_symbol_lookup,
            parse_index_namespace_sets_ns,
            parse_index_function_register_ns,
            parse_index_class_register_ns,
            parse_index_interface_register_ns,
            parse_index_enum_register_ns,
            parse_index_module_register_ns,
            parse_index_parsed_file_push_ns,
        },
    );

    Ok(ParseIndexOutputs {
        parsed_files,
        global_function_map,
        global_function_file_map,
        global_class_map,
        global_class_file_map,
        global_interface_map,
        global_interface_file_map,
        global_enum_map,
        global_enum_file_map,
        global_module_map,
        global_module_file_map,
        namespace_class_map,
        namespace_interface_map,
        namespace_enum_map,
        namespace_module_map,
        function_collisions,
        class_collisions,
        interface_collisions,
        enum_collisions,
        module_collisions,
        project_symbol_lookup: ProjectSymbolLookup {
            exact: project_symbol_lookup_exact,
            wildcard_members: project_symbol_lookup_wildcard_members,
        },
        total_module_names,
    })
}

struct SymbolBatchContext<'a> {
    namespace_map: Option<&'a mut HashMap<String, HashSet<String>>>,
    global_map: &'a mut HashMap<String, String>,
    global_file_map: &'a mut HashMap<String, PathBuf>,
    collisions: &'a mut Vec<(String, String, String)>,
    exact_lookup: &'a mut ExactSymbolLookup,
    wildcard_lookup: &'a mut WildcardMemberLookup,
    build_symbol_lookup: bool,
}

fn register_symbol_batch(
    names: &[String],
    namespace: &str,
    file: &Path,
    ctx: SymbolBatchContext<'_>,
) -> u64 {
    if names.is_empty() {
        return 0;
    }

    if let Some(namespace_map) = ctx.namespace_map {
        let entry = namespace_map
            .entry(namespace.to_string())
            .or_insert_with(|| HashSet::with_capacity(names.len()));
        for name in names {
            entry.insert(name.clone());
        }
    }

    let mut register_ns = 0_u64;
    for name in names {
        let started_at = Instant::now();
        register_global_symbol(
            name,
            namespace,
            file,
            &mut GlobalSymbolRegistrationContext {
                global_map: ctx.global_map,
                global_file_map: ctx.global_file_map,
                collisions: ctx.collisions,
                exact_lookup: ctx.exact_lookup,
                wildcard_lookup: ctx.wildcard_lookup,
                build_symbol_lookup: ctx.build_symbol_lookup,
            },
        );
        register_ns += elapsed_nanos_u64(started_at);
    }

    register_ns
}
