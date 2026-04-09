use crate::ast::ImportDecl;
use crate::cache::*;
use crate::dependency::*;
use crate::specialization::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Instant;

/// A view of the five global symbol lookup tables (function, class, interface, enum, module).
///
/// Each kind has a *namespace* map (`symbol_name → owning_namespace`) and a *file* map
/// (`symbol_name → owner_file`).  Bundling them here avoids repeating ten identical
/// parameters in every closure/declaration-resolution helper.
pub(crate) struct GlobalSymbolMaps<'a> {
    pub(crate) function_map: &'a HashMap<String, String>,
    pub(crate) function_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) class_map: &'a HashMap<String, String>,
    pub(crate) class_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) interface_map: &'a HashMap<String, String>,
    pub(crate) interface_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) enum_map: &'a HashMap<String, String>,
    pub(crate) enum_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) module_map: &'a HashMap<String, String>,
    pub(crate) module_file_map: &'a HashMap<String, PathBuf>,
}

impl<'a> GlobalSymbolMaps<'a> {
    /// Returns the first file-map entry that owns `symbol`, scanning all five symbol kinds.
    pub(crate) fn any_owner_file(&self, symbol: &str) -> Option<&'a PathBuf> {
        self.function_file_map
            .get(symbol)
            .or_else(|| self.class_file_map.get(symbol))
            .or_else(|| self.interface_file_map.get(symbol))
            .or_else(|| self.enum_file_map.get(symbol))
            .or_else(|| self.module_file_map.get(symbol))
    }
}

pub(crate) fn insert_declaration_symbol_for_owner(
    symbol: &str,
    owner_ns: &str,
    owner_file: &Path,
    entry_namespace: &str,
    declaration_symbols: &mut HashSet<String>,
    maps: &GlobalSymbolMaps<'_>,
) {
    let is_function_owner = maps
        .function_map
        .get(symbol)
        .is_some_and(|ns| ns == owner_ns)
        && maps
            .function_file_map
            .get(symbol)
            .is_some_and(|path| path == owner_file);
    if is_function_owner {
        declaration_symbols.insert(mangle_project_symbol_for_codegen(
            owner_ns,
            entry_namespace,
            symbol,
        ));
    }

    let is_nominal_owner = [
        maps.class_map.get(symbol).is_some_and(|ns| ns == owner_ns)
            && maps
                .class_file_map
                .get(symbol)
                .is_some_and(|path| path == owner_file),
        maps.interface_map
            .get(symbol)
            .is_some_and(|ns| ns == owner_ns)
            && maps
                .interface_file_map
                .get(symbol)
                .is_some_and(|path| path == owner_file),
        maps.enum_map.get(symbol).is_some_and(|ns| ns == owner_ns)
            && maps
                .enum_file_map
                .get(symbol)
                .is_some_and(|path| path == owner_file),
        maps.module_map.get(symbol).is_some_and(|ns| ns == owner_ns)
            && maps
                .module_file_map
                .get(symbol)
                .is_some_and(|path| path == owner_file),
    ]
    .into_iter()
    .any(|matched| matched);

    if is_nominal_owner || !is_function_owner {
        declaration_symbols.insert(mangle_project_nominal_symbol_for_codegen(owner_ns, symbol));
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CodegenReferenceMetadata {
    pub(crate) imports: Vec<ImportDecl>,
    pub(crate) referenced_symbols: Vec<String>,
    pub(crate) qualified_symbol_refs: Vec<Vec<String>>,
    pub(crate) api_referenced_symbols: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn extend_declaration_symbols_for_reference(
    current_file: &Path,
    prefer_local_owner: bool,
    symbol: &str,
    entry_namespace: &str,
    declaration_symbols: &mut HashSet<String>,
    stack: &mut Vec<PathBuf>,
    closure_files: &HashSet<PathBuf>,
    maps: &GlobalSymbolMaps<'_>,
) {
    let mut push_owner = |owner_ns: &str, owner_file: &Path| {
        if closure_files.contains(owner_file) {
            insert_declaration_symbol_for_owner(
                symbol,
                owner_ns,
                owner_file,
                entry_namespace,
                declaration_symbols,
                maps,
            );
            stack.push(owner_file.to_path_buf());
        }
    };

    if prefer_local_owner
        && maps
            .function_file_map
            .get(symbol)
            .is_none_or(|owner_file| owner_file != current_file)
    {
        if let (Some(owner_ns), Some(owner_file)) =
            (maps.class_map.get(symbol), maps.class_file_map.get(symbol))
        {
            if owner_file == current_file {
                push_owner(owner_ns, owner_file);
                return;
            }
        }
        if let (Some(owner_ns), Some(owner_file)) = (
            maps.interface_map.get(symbol),
            maps.interface_file_map.get(symbol),
        ) {
            if owner_file == current_file {
                push_owner(owner_ns, owner_file);
                return;
            }
        }
        if let (Some(owner_ns), Some(owner_file)) =
            (maps.enum_map.get(symbol), maps.enum_file_map.get(symbol))
        {
            if owner_file == current_file {
                push_owner(owner_ns, owner_file);
                return;
            }
        }
        if let (Some(owner_ns), Some(owner_file)) = (
            maps.module_map.get(symbol),
            maps.module_file_map.get(symbol),
        ) {
            if owner_file == current_file {
                push_owner(owner_ns, owner_file);
                return;
            }
        }
    }

    if let Some((owner_symbol, _member)) = symbol.rsplit_once("__") {
        if let (Some(owner_ns), Some(owner_file)) = (
            maps.class_map.get(owner_symbol),
            maps.class_file_map.get(owner_symbol),
        ) {
            push_owner(owner_ns, owner_file);
        }
        // Try deeper split for nested modules
        let mut parts = symbol.split("__").collect::<Vec<_>>();
        while parts.len() > 1 {
            parts.pop();
            let parent = parts.join("__");
            if let (Some(owner_ns), Some(owner_file)) = (
                maps.class_map.get(&parent),
                maps.class_file_map.get(&parent),
            ) {
                push_owner(owner_ns, owner_file);
            }
            if let (Some(owner_ns), Some(owner_file)) = (
                maps.module_map.get(&parent),
                maps.module_file_map.get(&parent),
            ) {
                push_owner(owner_ns, owner_file);
            }
        }
    }

    if let (Some(owner_ns), Some(owner_file)) = (
        maps.function_map.get(symbol),
        maps.function_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) =
        (maps.class_map.get(symbol), maps.class_file_map.get(symbol))
    {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) =
        (maps.enum_map.get(symbol), maps.enum_file_map.get(symbol))
    {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) = (
        maps.interface_map.get(symbol),
        maps.interface_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) = (
        maps.module_map.get(symbol),
        maps.module_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
}

pub(crate) fn resolve_exact_imported_symbol_file<'a>(
    namespace_path: &str,
    symbol_name: &str,
    symbol_lookup: &'a ProjectSymbolLookup,
) -> Option<(String, String, &'a PathBuf)> {
    let full_path = qualified_symbol_path(namespace_path, symbol_name);
    exact_symbol_resolution(symbol_lookup, &full_path).map(|resolution| {
        (
            resolution.owner_namespace.clone(),
            resolution.symbol_name.clone(),
            &resolution.owner_file,
        )
    })
}

pub(crate) fn resolve_exact_imported_symbol_owner<'a>(
    namespace_path: &str,
    symbol_name: &str,
    symbol_lookup: &'a ProjectSymbolLookup,
) -> Option<(String, String, &'a PathBuf)> {
    resolve_exact_imported_symbol_file(namespace_path, symbol_name, symbol_lookup)
}

pub(crate) fn extend_declaration_symbols_for_exact_import(
    import: &ImportDecl,
    entry_namespace: &str,
    declaration_symbols: &mut HashSet<String>,
    stack: &mut Vec<PathBuf>,
    closure_files: &HashSet<PathBuf>,
    symbol_lookup: &ProjectSymbolLookup,
    maps: &GlobalSymbolMaps<'_>,
) {
    let Some((namespace, symbol)) = import.path.rsplit_once('.') else {
        return;
    };

    if let Some((owner_ns, symbol_name, owner_file)) =
        resolve_exact_imported_symbol_owner(namespace, symbol, symbol_lookup)
    {
        if closure_files.contains(owner_file) {
            insert_declaration_symbol_for_owner(
                &symbol_name,
                &owner_ns,
                owner_file,
                entry_namespace,
                declaration_symbols,
                maps,
            );
            stack.push(owner_file.clone());
        }
        return;
    }

    if let Some((enum_namespace, enum_name)) = namespace.rsplit_once('.') {
        if let Some((owner_ns, resolved_enum_name, owner_file)) =
            resolve_exact_imported_symbol_file(enum_namespace, enum_name, symbol_lookup)
        {
            if closure_files.contains(owner_file) {
                declaration_symbols.insert(mangle_project_nominal_symbol_for_codegen(
                    &owner_ns,
                    &resolved_enum_name,
                ));
                stack.push(owner_file.clone());
            }
            return;
        }
    }

    let mut push_owner = |owner_ns: &str, owner_file: &Path| {
        if owner_ns == namespace && closure_files.contains(owner_file) {
            insert_declaration_symbol_for_owner(
                symbol,
                owner_ns,
                owner_file,
                entry_namespace,
                declaration_symbols,
                maps,
            );
            stack.push(owner_file.to_path_buf());
        }
    };

    if let (Some(owner_ns), Some(owner_file)) = (
        maps.function_map.get(symbol),
        maps.function_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) =
        (maps.class_map.get(symbol), maps.class_file_map.get(symbol))
    {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) = (
        maps.interface_map.get(symbol),
        maps.interface_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) =
        (maps.enum_map.get(symbol), maps.enum_file_map.get(symbol))
    {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) = (
        maps.module_map.get(symbol),
        maps.module_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DeclarationClosure {
    pub(crate) symbols: HashSet<String>,
    pub(crate) files: HashSet<PathBuf>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn declaration_symbols_for_unit(
    root_file: &Path,
    root_active_symbols: &HashSet<String>,
    precomputed_dependency_closures: &PrecomputedDependencyClosures,
    reference_metadata: &HashMap<PathBuf, CodegenReferenceMetadata>,
    entry_namespace: &str,
    symbol_lookup: &ProjectSymbolLookup,
    maps: &GlobalSymbolMaps<'_>,
    timings: Option<&DeclarationClosureTimingTotals>,
) -> DeclarationClosure {
    let closure_seed_started_at = Instant::now();
    let mut closure_files =
        transitive_dependencies_from_precomputed(precomputed_dependency_closures, root_file);
    closure_files.insert(root_file.to_path_buf());
    if let Some(timings) = timings {
        timings.closure_seed_ns.fetch_add(
            elapsed_nanos_u64(closure_seed_started_at),
            Ordering::Relaxed,
        );
    }

    let mut declaration_symbols = root_active_symbols.clone();
    let mut visited_files = HashSet::new();
    let mut stack = vec![root_file.to_path_buf()];

    while let Some(file) = stack.pop() {
        if !visited_files.insert(file.clone()) {
            continue;
        }
        if let Some(timings) = timings {
            timings.visited_file_count.fetch_add(1, Ordering::Relaxed);
        }

        let metadata_lookup_started_at = Instant::now();
        let Some(metadata) = reference_metadata.get(&file) else {
            if let Some(timings) = timings {
                timings.metadata_lookup_ns.fetch_add(
                    elapsed_nanos_u64(metadata_lookup_started_at),
                    Ordering::Relaxed,
                );
            }
            continue;
        };
        if let Some(timings) = timings {
            timings.metadata_lookup_ns.fetch_add(
                elapsed_nanos_u64(metadata_lookup_started_at),
                Ordering::Relaxed,
            );
        }

        for import in &metadata.imports {
            if import.path.ends_with(".*") {
                if let Some(timings) = timings {
                    timings
                        .wildcard_import_count
                        .fetch_add(1, Ordering::Relaxed);
                }
                let wildcard_started_at = Instant::now();
                let namespace = import.path.trim_end_matches(".*");
                for symbol in &metadata.referenced_symbols {
                    if let Some(timings) = timings {
                        timings
                            .reference_symbol_count
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    if let Some((owner_ns, candidate)) = resolve_symbol_in_namespace_path(
                        namespace,
                        std::slice::from_ref(symbol),
                        symbol_lookup,
                    ) {
                        let owner_file = maps.any_owner_file(&candidate);
                        if let Some(owner_file) = owner_file {
                            if closure_files.contains(owner_file) {
                                insert_declaration_symbol_for_owner(
                                    &candidate,
                                    &owner_ns,
                                    owner_file,
                                    entry_namespace,
                                    &mut declaration_symbols,
                                    maps,
                                );
                                stack.push(owner_file.to_path_buf());
                            }
                        }
                    }
                }
                if let Some(timings) = timings {
                    timings
                        .wildcard_imports_ns
                        .fetch_add(elapsed_nanos_u64(wildcard_started_at), Ordering::Relaxed);
                }
                continue;
            }

            if let Some(timings) = timings {
                timings.exact_import_count.fetch_add(1, Ordering::Relaxed);
            }
            let exact_started_at = Instant::now();
            extend_declaration_symbols_for_exact_import(
                import,
                entry_namespace,
                &mut declaration_symbols,
                &mut stack,
                &closure_files,
                symbol_lookup,
                maps,
            );
            if let Some(timings) = timings {
                timings
                    .exact_imports_ns
                    .fetch_add(elapsed_nanos_u64(exact_started_at), Ordering::Relaxed);
            }

            let import_key = import_lookup_key(import);
            let qualified_started_at = Instant::now();
            for path in &metadata.qualified_symbol_refs {
                if path.first().is_some_and(|part| part == &import_key) {
                    if let Some(timings) = timings {
                        timings.qualified_ref_count.fetch_add(1, Ordering::Relaxed);
                    }
                    let rest = &path[1..];
                    if let Some((owner_ns, candidate)) =
                        resolve_symbol_in_namespace_path(&import.path, rest, symbol_lookup)
                    {
                        let owner_file = maps.any_owner_file(&candidate);
                        if let Some(owner_file) = owner_file {
                            if closure_files.contains(owner_file) {
                                insert_declaration_symbol_for_owner(
                                    &candidate,
                                    &owner_ns,
                                    owner_file,
                                    entry_namespace,
                                    &mut declaration_symbols,
                                    maps,
                                );
                                stack.push(owner_file.to_path_buf());
                            }
                        }
                    }
                }
            }
            if let Some(timings) = timings {
                timings
                    .qualified_refs_ns
                    .fetch_add(elapsed_nanos_u64(qualified_started_at), Ordering::Relaxed);
            }
        }

        let symbols = if file == root_file {
            &metadata.referenced_symbols
        } else {
            &metadata.api_referenced_symbols
        };
        let reference_symbols_started_at = Instant::now();
        for symbol in symbols {
            if let Some(timings) = timings {
                timings
                    .reference_symbol_count
                    .fetch_add(1, Ordering::Relaxed);
            }
            extend_declaration_symbols_for_reference(
                &file,
                file != root_file,
                symbol,
                entry_namespace,
                &mut declaration_symbols,
                &mut stack,
                &closure_files,
                maps,
            );
        }
        if let Some(timings) = timings {
            timings.reference_symbols_ns.fetch_add(
                elapsed_nanos_u64(reference_symbols_started_at),
                Ordering::Relaxed,
            );
        }
    }

    DeclarationClosure {
        symbols: declaration_symbols,
        files: visited_files,
    }
}

pub(crate) fn closure_body_symbols_for_files(
    root_files: &HashSet<PathBuf>,
    declaration_symbols: &HashSet<String>,
    global_function_file_map: &HashMap<String, PathBuf>,
    global_class_file_map: &HashMap<String, PathBuf>,
    global_module_file_map: &HashMap<String, PathBuf>,
) -> HashSet<String> {
    declaration_symbols
        .iter()
        .filter(|symbol| {
            if symbol.as_str() == "main" {
                return global_function_file_map
                    .get("main")
                    .is_some_and(|owner_file| root_files.contains(owner_file));
            }

            if global_function_file_map
                .get(symbol.as_str())
                .is_some_and(|owner_file| root_files.contains(owner_file))
            {
                return true;
            }

            if global_class_file_map
                .get(symbol.as_str())
                .is_some_and(|owner_file| root_files.contains(owner_file))
            {
                return true;
            }

            if global_module_file_map
                .get(symbol.as_str())
                .is_some_and(|owner_file| root_files.contains(owner_file))
            {
                return true;
            }

            if let Some(owner) = symbol.strip_suffix("__new") {
                return global_class_file_map
                    .get(owner)
                    .is_some_and(|owner_file| root_files.contains(owner_file));
            }

            if let Some((owner, _)) = symbol.rsplit_once("__") {
                if global_class_file_map
                    .get(owner)
                    .is_some_and(|owner_file| root_files.contains(owner_file))
                {
                    return true;
                }

                if global_module_file_map
                    .get(owner)
                    .is_some_and(|owner_file| root_files.contains(owner_file))
                {
                    return true;
                }
            }

            false
        })
        .cloned()
        .collect()
}
