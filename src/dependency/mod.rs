use crate::ast::{ImportDecl, Program};
use crate::cache::*;
use crate::typeck::{ClassMethodEffectsSummary, FunctionEffectsSummary};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
pub(crate) fn namespace_prefixes(namespace: &str) -> Vec<String> {
    let mut prefixes = Vec::new();
    let mut current = namespace.trim();
    while !current.is_empty() {
        prefixes.push(current.to_string());
        if let Some((prefix, _)) = current.rsplit_once('.') {
            current = prefix;
        } else {
            break;
        }
    }
    prefixes
}

pub(crate) fn qualified_symbol_path(namespace: &str, symbol_name: &str) -> String {
    let separator_count = symbol_name.matches("__").count();
    let mut path = String::with_capacity(namespace.len() + symbol_name.len() + separator_count + 1);
    if !namespace.is_empty() {
        path.push_str(namespace);
        path.push('.');
    }
    if separator_count == 0 {
        path.push_str(symbol_name);
        return path;
    }

    let mut remaining = symbol_name;
    while let Some(index) = remaining.find("__") {
        path.push_str(&remaining[..index]);
        path.push('.');
        remaining = &remaining[index + 2..];
    }
    path.push_str(remaining);
    path
}

pub(crate) fn qualified_symbol_path_for_parts(
    namespace: &str,
    member_parts: &[String],
) -> Option<String> {
    if member_parts.is_empty() {
        return None;
    }

    Some(if namespace.is_empty() {
        member_parts.join(".")
    } else {
        format!("{}.{}", namespace, member_parts.join("."))
    })
}

pub(crate) fn wildcard_member_import_path(
    owner_namespace: &str,
    symbol_name: &str,
) -> (String, String) {
    let Some(last_separator) = symbol_name.rfind("__") else {
        return (owner_namespace.to_string(), symbol_name.to_string());
    };

    let member_name = symbol_name[last_separator + 2..].to_string();
    let prefix = &symbol_name[..last_separator];
    let separator_count = prefix.matches("__").count();
    let mut import_namespace =
        String::with_capacity(owner_namespace.len() + prefix.len() + separator_count + 1);
    import_namespace.push_str(owner_namespace);
    import_namespace.push('.');
    if separator_count == 0 {
        import_namespace.push_str(prefix);
        return (import_namespace, member_name);
    }

    let mut remaining = prefix;
    while let Some(index) = remaining.find("__") {
        import_namespace.push_str(&remaining[..index]);
        import_namespace.push('.');
        remaining = &remaining[index + 2..];
    }
    import_namespace.push_str(remaining);
    (import_namespace, member_name)
}

pub(crate) fn insert_lookup_resolution(
    target: &mut HashMap<String, Option<SharedSymbolLookupResolution>>,
    key: String,
    resolution: SharedSymbolLookupResolution,
) {
    match target.entry(key) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(Some(resolution));
        }
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            let unchanged = entry
                .get()
                .as_ref()
                .is_some_and(|current| current.as_ref() == resolution.as_ref());
            if !unchanged {
                entry.insert(None);
            }
        }
    }
}

pub(crate) fn insert_symbol_lookup_entry(
    exact_lookup: &mut ExactSymbolLookup,
    wildcard_lookup: &mut WildcardMemberLookup,
    owner_namespace: &str,
    symbol_name: &str,
    owner_file: &Path,
) {
    let resolution = Arc::new(SymbolLookupResolution {
        owner_namespace: owner_namespace.to_string(),
        symbol_name: symbol_name.to_string(),
        owner_file: owner_file.to_path_buf(),
    });
    insert_lookup_resolution(
        exact_lookup,
        qualified_symbol_path(owner_namespace, symbol_name),
        Arc::clone(&resolution),
    );

    let (import_namespace, member_name) = wildcard_member_import_path(owner_namespace, symbol_name);
    insert_lookup_resolution(
        wildcard_lookup.entry(import_namespace).or_default(),
        member_name,
        resolution,
    );
}

pub(crate) struct GlobalSymbolRegistrationContext<'a> {
    pub(crate) global_map: &'a mut HashMap<String, String>,
    pub(crate) global_file_map: &'a mut HashMap<String, PathBuf>,
    pub(crate) collisions: &'a mut Vec<(String, String, String)>,
    pub(crate) exact_lookup: &'a mut ExactSymbolLookup,
    pub(crate) wildcard_lookup: &'a mut WildcardMemberLookup,
    pub(crate) build_symbol_lookup: bool,
}

pub(crate) fn register_global_symbol(
    symbol_name: &str,
    owner_namespace: &str,
    owner_file: &Path,
    ctx: &mut GlobalSymbolRegistrationContext<'_>,
) {
    match ctx.global_map.entry(symbol_name.to_string()) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(owner_namespace.to_string());
            ctx.global_file_map
                .insert(symbol_name.to_string(), owner_file.to_path_buf());
            if ctx.build_symbol_lookup {
                insert_symbol_lookup_entry(
                    ctx.exact_lookup,
                    ctx.wildcard_lookup,
                    owner_namespace,
                    symbol_name,
                    owner_file,
                );
            }
        }
        std::collections::hash_map::Entry::Occupied(entry) => {
            if entry.get() != owner_namespace {
                ctx.collisions.push((
                    symbol_name.to_string(),
                    entry.get().clone(),
                    owner_namespace.to_string(),
                ));
            }
        }
    }
}

pub(crate) fn exact_symbol_resolution<'a>(
    lookup: &'a ProjectSymbolLookup,
    qualified_path: &str,
) -> Option<&'a SymbolLookupResolution> {
    lookup.exact.get(qualified_path).and_then(Option::as_deref)
}

pub(crate) fn wildcard_symbol_resolution<'a>(
    lookup: &'a ProjectSymbolLookup,
    import_namespace: &str,
    member_name: &str,
) -> Option<&'a SymbolLookupResolution> {
    lookup
        .wildcard_members
        .get(import_namespace)
        .and_then(|members| members.get(member_name))
        .and_then(Option::as_deref)
}

pub(crate) fn import_path_owner_file<'a>(
    path: &str,
    symbol_lookup: &'a ProjectSymbolLookup,
) -> Option<&'a PathBuf> {
    if let Some(resolution) = exact_symbol_resolution(symbol_lookup, path) {
        return Some(&resolution.owner_file);
    }

    if let Some((enum_path, _)) = path.rsplit_once('.') {
        if let Some(resolution) = exact_symbol_resolution(symbol_lookup, enum_path) {
            return Some(&resolution.owner_file);
        }
    }

    None
}

pub(crate) struct RewriteFingerprintContext<'a> {
    pub(crate) namespace_functions: &'a HashMap<String, HashSet<String>>,
    pub(crate) global_function_map: &'a HashMap<String, String>,
    pub(crate) global_function_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) namespace_classes: &'a HashMap<String, HashSet<String>>,
    pub(crate) global_class_map: &'a HashMap<String, String>,
    pub(crate) global_class_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) global_interface_map: &'a HashMap<String, String>,
    pub(crate) global_interface_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) global_enum_map: &'a HashMap<String, String>,
    pub(crate) global_enum_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) namespace_modules: &'a HashMap<String, HashSet<String>>,
    pub(crate) global_module_map: &'a HashMap<String, String>,
    pub(crate) global_module_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) namespace_api_fingerprints: &'a HashMap<String, String>,
    pub(crate) file_api_fingerprints: &'a HashMap<PathBuf, String>,
    pub(crate) symbol_lookup: Arc<ProjectSymbolLookup>,
}

pub(crate) struct DependencyResolutionContext<'a> {
    pub(crate) namespace_files_map: &'a HashMap<String, Vec<PathBuf>>,
    pub(crate) global_function_map: &'a HashMap<String, String>,
    pub(crate) global_function_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) global_class_map: &'a HashMap<String, String>,
    pub(crate) global_class_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) global_interface_map: &'a HashMap<String, String>,
    pub(crate) global_interface_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) global_enum_map: &'a HashMap<String, String>,
    pub(crate) global_enum_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) global_module_map: &'a HashMap<String, String>,
    pub(crate) global_module_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) symbol_lookup: Arc<ProjectSymbolLookup>,
}

pub(crate) fn import_lookup_key(import: &ImportDecl) -> String {
    import
        .alias
        .as_ref()
        .cloned()
        .unwrap_or_else(|| import.path.rsplit('.').next().unwrap_or("").to_string())
}

pub(crate) fn resolve_symbol_file_in_namespace_path(
    namespace_path: &str,
    member_parts: &[String],
    symbol_lookup: &ProjectSymbolLookup,
) -> Option<PathBuf> {
    if member_parts.len() == 1 {
        if let Some(resolution) =
            wildcard_symbol_resolution(symbol_lookup, namespace_path, &member_parts[0])
        {
            return Some(resolution.owner_file.clone());
        }
    }

    for prefix_len in (1..=member_parts.len()).rev() {
        let prefix = &member_parts[..prefix_len];
        let dotted_path = qualified_symbol_path_for_parts(namespace_path, prefix)?;
        if let Some(resolution) = exact_symbol_resolution(symbol_lookup, &dotted_path) {
            return Some(resolution.owner_file.clone());
        }
        if prefix_len > 1 {
            let mangled_prefix = prefix.join("__");
            let mangled_path = if namespace_path.is_empty() {
                mangled_prefix
            } else {
                format!("{}.{}", namespace_path, mangled_prefix)
            };
            if let Some(resolution) = exact_symbol_resolution(symbol_lookup, &mangled_path) {
                return Some(resolution.owner_file.clone());
            }
        }
    }

    None
}

pub(crate) fn resolve_symbol_in_namespace_path(
    namespace_path: &str,
    member_parts: &[String],
    symbol_lookup: &ProjectSymbolLookup,
) -> Option<(String, String)> {
    if member_parts.len() == 1 {
        if let Some(resolution) =
            wildcard_symbol_resolution(symbol_lookup, namespace_path, &member_parts[0])
        {
            return Some((
                resolution.owner_namespace.clone(),
                resolution.symbol_name.clone(),
            ));
        }
    }

    for prefix_len in (1..=member_parts.len()).rev() {
        let prefix = &member_parts[..prefix_len];
        let dotted_path = qualified_symbol_path_for_parts(namespace_path, prefix)?;
        if let Some(resolution) = exact_symbol_resolution(symbol_lookup, &dotted_path) {
            return Some((
                resolution.owner_namespace.clone(),
                resolution.symbol_name.clone(),
            ));
        }
        if prefix_len > 1 {
            let mangled_prefix = prefix.join("__");
            let mangled_path = if namespace_path.is_empty() {
                mangled_prefix
            } else {
                format!("{}.{}", namespace_path, mangled_prefix)
            };
            if let Some(resolution) = exact_symbol_resolution(symbol_lookup, &mangled_path) {
                return Some((
                    resolution.owner_namespace.clone(),
                    resolution.symbol_name.clone(),
                ));
            }
        }
    }

    None
}

pub(crate) fn resolve_owner_file_in_namespace_path(
    namespace_path: &str,
    member_parts: &[String],
    symbol_lookup: &ProjectSymbolLookup,
) -> Option<PathBuf> {
    resolve_symbol_file_in_namespace_path(namespace_path, member_parts, symbol_lookup)
}

pub(crate) fn resolve_symbol_owner_files_in_namespace(
    namespace: &str,
    referenced_symbols: &HashSet<String>,
    qualified_symbol_refs: &[Vec<String>],
    ctx: &DependencyResolutionContext<'_>,
    timings: Option<&DependencyGraphTimingTotals>,
) -> HashSet<PathBuf> {
    let mut deps = HashSet::new();

    for symbol in referenced_symbols {
        if let Some(timings) = timings {
            timings
                .direct_symbol_ref_count
                .fetch_add(1, Ordering::Relaxed);
        }
        let lookup_started_at = Instant::now();
        if let Some(file) = resolve_owner_file_in_namespace_path(
            namespace,
            std::slice::from_ref(symbol),
            ctx.symbol_lookup.as_ref(),
        ) {
            deps.insert(file);
        }
        if let Some(timings) = timings {
            timings
                .owner_lookup_ns
                .fetch_add(elapsed_nanos_u64(lookup_started_at), Ordering::Relaxed);
        }
    }

    for path in qualified_symbol_refs {
        if let Some(timings) = timings {
            timings.qualified_ref_count.fetch_add(1, Ordering::Relaxed);
        }
        let lookup_started_at = Instant::now();
        if let Some(file) =
            resolve_owner_file_in_namespace_path(namespace, path, ctx.symbol_lookup.as_ref())
        {
            deps.insert(file);
        }
        if let Some(timings) = timings {
            timings
                .owner_lookup_ns
                .fetch_add(elapsed_nanos_u64(lookup_started_at), Ordering::Relaxed);
        }
    }

    deps
}

pub(crate) fn namespace_dependency_files(
    namespace: &str,
    ctx: &DependencyResolutionContext<'_>,
    timings: Option<&DependencyGraphTimingTotals>,
) -> HashSet<PathBuf> {
    let started_at = Instant::now();
    let deps = ctx
        .namespace_files_map
        .get(namespace)
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    if let Some(timings) = timings {
        timings
            .namespace_files_ns
            .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
    }
    deps
}

pub(crate) fn resolve_import_dependency_files(
    unit: &ParsedProjectUnit,
    import: &ImportDecl,
    referenced_symbols: &HashSet<String>,
    qualified_symbol_refs: &[Vec<String>],
    ctx: &DependencyResolutionContext<'_>,
    timings: Option<&DependencyGraphTimingTotals>,
) -> HashSet<PathBuf> {
    let mut deps = HashSet::new();

    if import.path.ends_with(".*") {
        if let Some(timings) = timings {
            timings
                .import_wildcard_count
                .fetch_add(1, Ordering::Relaxed);
        }
        let started_at = Instant::now();
        let namespace = import.path.trim_end_matches(".*");
        let owner_files = resolve_symbol_owner_files_in_namespace(
            namespace,
            referenced_symbols,
            qualified_symbol_refs,
            ctx,
            timings,
        );
        if owner_files.is_empty() {
            let fallback_started_at = Instant::now();
            let deps = namespace_dependency_files(namespace, ctx, timings);
            if let Some(timings) = timings {
                timings
                    .namespace_fallback_count
                    .fetch_add(1, Ordering::Relaxed);
                timings
                    .namespace_fallback_ns
                    .fetch_add(elapsed_nanos_u64(fallback_started_at), Ordering::Relaxed);
                timings
                    .import_wildcard_ns
                    .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
            }
            return deps;
        }
        if let Some(timings) = timings {
            timings
                .import_wildcard_ns
                .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
        }
        return owner_files;
    }

    let exact_started_at = Instant::now();
    if let Some(owner_file) = import_path_owner_file(&import.path, ctx.symbol_lookup.as_ref()) {
        if let Some(timings) = timings {
            timings.import_exact_count.fetch_add(1, Ordering::Relaxed);
            timings
                .import_exact_ns
                .fetch_add(elapsed_nanos_u64(exact_started_at), Ordering::Relaxed);
        }
        deps.insert(owner_file.clone());
        return deps;
    }
    if let Some(timings) = timings {
        timings.import_exact_count.fetch_add(1, Ordering::Relaxed);
        timings
            .import_exact_ns
            .fetch_add(elapsed_nanos_u64(exact_started_at), Ordering::Relaxed);
    }

    let import_key = import_lookup_key(import);
    let namespace_like_import = ctx.namespace_files_map.contains_key(&import.path)
        || unit
            .imports
            .iter()
            .any(|candidate| candidate.path == import.path && candidate.alias.is_some());
    if namespace_like_import {
        if let Some(timings) = timings {
            timings
                .import_namespace_alias_count
                .fetch_add(1, Ordering::Relaxed);
        }
        let started_at = Instant::now();
        for path in qualified_symbol_refs {
            if path.first().is_some_and(|part| part == &import_key) {
                if let Some(timings) = timings {
                    timings.qualified_ref_count.fetch_add(1, Ordering::Relaxed);
                }
                let lookup_started_at = Instant::now();
                let rest = &path[1..];
                if let Some(file) = resolve_owner_file_in_namespace_path(
                    &import.path,
                    rest,
                    ctx.symbol_lookup.as_ref(),
                ) {
                    deps.insert(file);
                }
                if let Some(timings) = timings {
                    timings
                        .owner_lookup_ns
                        .fetch_add(elapsed_nanos_u64(lookup_started_at), Ordering::Relaxed);
                }
            }
        }
        if deps.is_empty() {
            let fallback_started_at = Instant::now();
            let exact_import_namespace_fallback =
                namespace_dependency_files(&import.path, ctx, timings);
            if !exact_import_namespace_fallback.is_empty() {
                if let Some(timings) = timings {
                    timings
                        .namespace_fallback_count
                        .fetch_add(1, Ordering::Relaxed);
                    timings
                        .namespace_fallback_ns
                        .fetch_add(elapsed_nanos_u64(fallback_started_at), Ordering::Relaxed);
                    timings
                        .import_namespace_alias_ns
                        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
                }
                return exact_import_namespace_fallback;
            }
            if let Some((namespace, _)) = import.path.rsplit_once('.') {
                let parent_started_at = Instant::now();
                let deps = namespace_dependency_files(namespace, ctx, timings);
                if let Some(timings) = timings {
                    timings
                        .import_parent_namespace_count
                        .fetch_add(1, Ordering::Relaxed);
                    timings
                        .import_parent_namespace_ns
                        .fetch_add(elapsed_nanos_u64(parent_started_at), Ordering::Relaxed);
                    timings
                        .import_namespace_alias_ns
                        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
                }
                return deps;
            }
        }
        if let Some(timings) = timings {
            timings
                .import_namespace_alias_ns
                .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
        }
        return deps;
    }

    if let Some((namespace, _)) = import.path.rsplit_once('.') {
        if let Some(timings) = timings {
            timings
                .import_parent_namespace_count
                .fetch_add(1, Ordering::Relaxed);
        }
        let started_at = Instant::now();
        let owner_files = resolve_symbol_owner_files_in_namespace(
            namespace,
            referenced_symbols,
            qualified_symbol_refs,
            ctx,
            timings,
        );
        if owner_files.is_empty() {
            let fallback_started_at = Instant::now();
            deps.extend(namespace_dependency_files(namespace, ctx, timings));
            if let Some(timings) = timings {
                timings
                    .namespace_fallback_count
                    .fetch_add(1, Ordering::Relaxed);
                timings
                    .namespace_fallback_ns
                    .fetch_add(elapsed_nanos_u64(fallback_started_at), Ordering::Relaxed);
            }
        } else {
            deps.extend(owner_files);
        }
        if let Some(timings) = timings {
            timings
                .import_parent_namespace_ns
                .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
        }
    }

    deps
}

pub(crate) fn resolve_direct_dependencies_for_unit(
    unit: &ParsedProjectUnit,
    ctx: &DependencyResolutionContext<'_>,
    timings: Option<&DependencyGraphTimingTotals>,
) -> HashSet<PathBuf> {
    let mut deps = HashSet::new();
    let referenced_symbols: HashSet<String> = unit.referenced_symbols.iter().cloned().collect();

    let direct_started_at = Instant::now();
    for symbol in &unit.referenced_symbols {
        if let Some(timings) = timings {
            timings
                .direct_symbol_ref_count
                .fetch_add(1, Ordering::Relaxed);
        }
        if ctx
            .global_function_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_function_file_map.get(symbol) {
                if owner_file != &unit.file {
                    deps.insert(owner_file.clone());
                }
            }
        }
        if ctx
            .global_class_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_class_file_map.get(symbol) {
                if owner_file != &unit.file {
                    deps.insert(owner_file.clone());
                }
            }
        }
        if ctx
            .global_interface_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_interface_file_map.get(symbol) {
                if owner_file != &unit.file {
                    deps.insert(owner_file.clone());
                }
            }
        }
        if ctx
            .global_enum_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_enum_file_map.get(symbol) {
                if owner_file != &unit.file {
                    deps.insert(owner_file.clone());
                }
            }
        }
        if ctx
            .global_module_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_module_file_map.get(symbol) {
                if owner_file != &unit.file {
                    deps.insert(owner_file.clone());
                }
            }
        }
    }
    if let Some(timings) = timings {
        timings
            .direct_symbol_refs_ns
            .fetch_add(elapsed_nanos_u64(direct_started_at), Ordering::Relaxed);
    }

    for import in &unit.imports {
        deps.extend(resolve_import_dependency_files(
            unit,
            import,
            &referenced_symbols,
            &unit.qualified_symbol_refs,
            ctx,
            timings,
        ));
    }

    deps.remove(&unit.file);
    deps
}

pub(crate) fn build_file_dependency_graph_incremental(
    parsed_files: &[ParsedProjectUnit],
    ctx: &DependencyResolutionContext<'_>,
    previous: Option<&DependencyGraphCache>,
    timings: Option<&DependencyGraphTimingTotals>,
) -> (HashMap<PathBuf, HashSet<PathBuf>>, usize) {
    let current_api_fingerprints: HashMap<&PathBuf, &str> = parsed_files
        .iter()
        .map(|unit| (&unit.file, unit.api_fingerprint.as_str()))
        .collect();
    let previous_entries = previous
        .map(|cache| {
            cache
                .files
                .iter()
                .map(|entry| (&entry.file, entry))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let previous_reverse_graph = previous
        .map(|cache| {
            let mut reverse: HashMap<&PathBuf, HashSet<&PathBuf>> = HashMap::new();
            for entry in &cache.files {
                reverse.entry(&entry.file).or_default();
                for dep in &entry.direct_dependencies {
                    reverse.entry(dep).or_default().insert(&entry.file);
                }
            }
            reverse
        })
        .unwrap_or_default();

    let mut graph = HashMap::new();
    let mut reused = 0usize;

    for unit in parsed_files {
        let deps = if let Some(previous_entry) = previous_entries.get(&unit.file) {
            let cache_started_at = Instant::now();
            let direct_dependency_api_changed =
                previous_entry.direct_dependencies.iter().any(|dep| {
                    previous_entries
                        .get(dep)
                        .and_then(|previous_dep| {
                            current_api_fingerprints
                                .get(dep)
                                .map(|current| previous_dep.api_fingerprint != *current)
                        })
                        .unwrap_or(true)
                });
            let direct_dependent_api_changed = previous_reverse_graph
                .get(&unit.file)
                .into_iter()
                .flatten()
                .any(|dependent| {
                    previous_entries
                        .get(dependent)
                        .and_then(|previous_dependent| {
                            current_api_fingerprints
                                .get(dependent)
                                .map(|current| previous_dependent.api_fingerprint != *current)
                        })
                        .unwrap_or(true)
                });
            if let Some(timings) = timings {
                timings
                    .cache_validation_ns
                    .fetch_add(elapsed_nanos_u64(cache_started_at), Ordering::Relaxed);
            }

            if previous_entry.semantic_fingerprint == unit.semantic_fingerprint
                && previous_entry.api_fingerprint == unit.api_fingerprint
                && !direct_dependency_api_changed
                && !direct_dependent_api_changed
            {
                reused += 1;
                if let Some(timings) = timings {
                    timings.files_reused.fetch_add(1, Ordering::Relaxed);
                }
                previous_entry
                    .direct_dependencies
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>()
            } else {
                if let Some(timings) = timings {
                    timings.files_rebuilt.fetch_add(1, Ordering::Relaxed);
                }
                resolve_direct_dependencies_for_unit(unit, ctx, timings)
            }
        } else {
            if let Some(timings) = timings {
                timings.files_rebuilt.fetch_add(1, Ordering::Relaxed);
            }
            resolve_direct_dependencies_for_unit(unit, ctx, timings)
        };
        graph.insert(unit.file.clone(), deps);
    }

    (graph, reused)
}

pub(crate) fn semantic_check_components(
    parsed_files: &[ParsedProjectUnit],
    forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> Vec<Vec<PathBuf>> {
    let reverse_graph = build_reverse_dependency_graph(forward_graph);
    let mut remaining: HashSet<PathBuf> =
        parsed_files.iter().map(|unit| unit.file.clone()).collect();
    let mut components = Vec::new();

    while let Some(start) = remaining.iter().next().cloned() {
        let mut component = Vec::new();
        let mut stack = vec![start.clone()];
        remaining.remove(&start);

        while let Some(file) = stack.pop() {
            component.push(file.clone());

            if let Some(next) = forward_graph.get(&file) {
                for dep in next {
                    if remaining.remove(dep) {
                        stack.push(dep.clone());
                    }
                }
            }
            if let Some(next) = reverse_graph.get(&file) {
                for dep in next {
                    if remaining.remove(dep) {
                        stack.push(dep.clone());
                    }
                }
            }
        }

        component.sort();
        components.push(component);
    }

    components.sort_by(|a, b| a.first().cmp(&b.first()));
    components
}

pub(crate) fn build_reverse_dependency_graph(
    forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> HashMap<PathBuf, HashSet<PathBuf>> {
    let mut reverse = HashMap::new();
    for (file, deps) in forward_graph {
        reverse.entry(file.clone()).or_insert_with(HashSet::new);
        for dep in deps {
            reverse
                .entry(dep.clone())
                .or_insert_with(HashSet::new)
                .insert(file.clone());
        }
    }
    reverse
}

pub(crate) fn transitive_dependents(
    reverse_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
    roots: &HashSet<PathBuf>,
) -> HashSet<PathBuf> {
    let mut out = HashSet::new();
    let mut stack: Vec<PathBuf> = roots.iter().cloned().collect();
    while let Some(file) = stack.pop() {
        if !out.insert(file.clone()) {
            continue;
        }
        if let Some(next) = reverse_graph.get(&file) {
            stack.extend(next.iter().cloned());
        }
    }
    out
}

pub(crate) struct PrecomputedDependencyClosures {
    pub(crate) files: Vec<PathBuf>,
    pub(crate) file_indices: HashMap<PathBuf, usize>,
    pub(crate) closures: Vec<Vec<u64>>,
}

pub(crate) fn precompute_all_transitive_dependencies(
    forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> PrecomputedDependencyClosures {
    fn empty_words(word_count: usize) -> Vec<u64> {
        vec![0; word_count]
    }

    fn set_bit(words: &mut [u64], index: usize) {
        let word = index / 64;
        let bit = index % 64;
        words[word] |= 1u64 << bit;
    }

    fn union_words(dst: &mut [u64], src: &[u64]) {
        for (dst_word, src_word) in dst.iter_mut().zip(src.iter()) {
            *dst_word |= *src_word;
        }
    }

    fn visit(
        file: &PathBuf,
        forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
        file_indices: &HashMap<PathBuf, usize>,
        word_count: usize,
        memo: &mut HashMap<PathBuf, Vec<u64>>,
        visiting: &mut HashSet<PathBuf>,
    ) -> Vec<u64> {
        if let Some(cached) = memo.get(file) {
            return cached.clone();
        }
        if !visiting.insert(file.clone()) {
            return empty_words(word_count);
        }

        let mut closure = empty_words(word_count);
        if let Some(deps) = forward_graph.get(file) {
            for dep in deps {
                if let Some(dep_index) = file_indices.get(dep) {
                    set_bit(&mut closure, *dep_index);
                }
                let dep_closure =
                    visit(dep, forward_graph, file_indices, word_count, memo, visiting);
                union_words(&mut closure, &dep_closure);
            }
        }

        visiting.remove(file);
        memo.insert(file.clone(), closure.clone());
        closure
    }

    let mut files: Vec<PathBuf> = forward_graph.keys().cloned().collect();
    files.sort();
    let file_indices = files
        .iter()
        .enumerate()
        .map(|(index, file)| (file.clone(), index))
        .collect::<HashMap<_, _>>();
    let word_count = files.len().div_ceil(64);
    let mut memo = HashMap::new();
    let mut visiting = HashSet::new();

    for file in &files {
        visit(
            file,
            forward_graph,
            &file_indices,
            word_count,
            &mut memo,
            &mut visiting,
        );
    }

    let closures = files
        .iter()
        .map(|file| memo.remove(file).unwrap_or_else(|| empty_words(word_count)))
        .collect();

    PrecomputedDependencyClosures {
        files,
        file_indices,
        closures,
    }
}

pub(crate) fn transitive_dependencies_from_precomputed(
    precomputed: &PrecomputedDependencyClosures,
    root: &Path,
) -> HashSet<PathBuf> {
    let Some(root_index) = precomputed.file_indices.get(root).copied() else {
        return HashSet::new();
    };
    let Some(words) = precomputed.closures.get(root_index) else {
        return HashSet::new();
    };
    let mut out = HashSet::new();
    for (word_index, word) in words.iter().copied().enumerate() {
        if word == 0 {
            continue;
        }
        let base = word_index * 64;
        for bit in 0..64 {
            if (word & (1u64 << bit)) == 0 {
                continue;
            }
            let file_index = base + bit;
            if let Some(file) = precomputed.files.get(file_index) {
                out.insert(file.clone());
            }
        }
    }
    out
}

pub(crate) fn dependency_graph_cache_from_state(
    entry_namespace: &str,
    parsed_files: &[ParsedProjectUnit],
    forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> DependencyGraphCache {
    let mut files: Vec<DependencyGraphFileEntry> = parsed_files
        .iter()
        .map(|unit| {
            let mut direct_dependencies = forward_graph
                .get(&unit.file)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect::<Vec<_>>();
            direct_dependencies.sort();
            DependencyGraphFileEntry {
                file: unit.file.clone(),
                semantic_fingerprint: unit.semantic_fingerprint.clone(),
                api_fingerprint: unit.api_fingerprint.clone(),
                direct_dependencies,
            }
        })
        .collect();
    files.sort_by(|a, b| a.file.cmp(&b.file));

    DependencyGraphCache {
        schema: DEPENDENCY_GRAPH_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        entry_namespace: entry_namespace.to_string(),
        files,
    }
}

pub(crate) fn can_reuse_safe_rewrite_cache(
    previous_dependency_graph: Option<&DependencyGraphCache>,
    entry_namespace: &str,
) -> bool {
    previous_dependency_graph.is_some_and(|cache| cache.entry_namespace == entry_namespace)
}

pub(crate) fn semantic_summary_cache_from_state(
    parsed_files: &[ParsedProjectUnit],
    components: &[Vec<PathBuf>],
    function_effects: HashMap<String, Vec<String>>,
    class_method_effects: HashMap<String, HashMap<String, Vec<String>>>,
    class_mutating_methods: HashMap<String, HashSet<String>>,
) -> SemanticSummaryCache {
    let mut files: Vec<SemanticSummaryFileEntry> = parsed_files
        .iter()
        .map(|unit| SemanticSummaryFileEntry {
            file: unit.file.clone(),
            semantic_fingerprint: unit.semantic_fingerprint.clone(),
            function_names: unit.function_names.clone(),
            class_names: unit.class_names.clone(),
        })
        .collect();
    files.sort_by(|a, b| a.file.cmp(&b.file));

    let class_mutating_methods = class_mutating_methods
        .into_iter()
        .map(|(class_name, methods)| {
            let mut methods = methods.into_iter().collect::<Vec<_>>();
            methods.sort();
            (class_name, methods)
        })
        .collect();

    let file_entries: HashMap<&PathBuf, &SemanticSummaryFileEntry> =
        files.iter().map(|entry| (&entry.file, entry)).collect();
    let current_fingerprints: HashMap<PathBuf, String> = parsed_files
        .iter()
        .map(|unit| (unit.file.clone(), unit.semantic_fingerprint.clone()))
        .collect();
    let mut components = components
        .iter()
        .map(|component| {
            let mut function_names = Vec::new();
            let mut class_names = Vec::new();
            for file in component {
                if let Some(entry) = file_entries.get(file) {
                    function_names.extend(entry.function_names.iter().cloned());
                    class_names.extend(entry.class_names.iter().cloned());
                }
            }
            function_names.sort();
            function_names.dedup();
            class_names.sort();
            class_names.dedup();
            SemanticSummaryComponentEntry {
                component_fingerprint: component_fingerprint(component, &current_fingerprints),
                files: component.clone(),
                function_names,
                class_names,
            }
        })
        .collect::<Vec<_>>();
    components.sort_by(|a, b| a.component_fingerprint.cmp(&b.component_fingerprint));

    SemanticSummaryCache {
        schema: SEMANTIC_SUMMARY_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        files,
        components,
        function_effects,
        class_method_effects,
        class_mutating_methods,
    }
}

pub(crate) fn component_fingerprint(
    component_files: &[PathBuf],
    current_fingerprints: &HashMap<PathBuf, String>,
) -> String {
    let mut hasher = stable_hasher();
    for file in component_files {
        file.hash(&mut hasher);
        if let Some(fingerprint) = current_fingerprints.get(file) {
            fingerprint.hash(&mut hasher);
        }
    }
    format!("{:016x}", hasher.finish())
}

pub(crate) fn typecheck_summary_cache_from_state(
    current_fingerprints: &HashMap<PathBuf, String>,
    components: &[Vec<PathBuf>],
) -> TypecheckSummaryCache {
    let mut files = Vec::new();
    for component in components {
        let component_fingerprint = component_fingerprint(component, current_fingerprints);
        for file in component {
            if let Some(semantic_fingerprint) = current_fingerprints.get(file) {
                files.push(TypecheckSummaryFileEntry {
                    file: file.clone(),
                    semantic_fingerprint: semantic_fingerprint.clone(),
                    component_fingerprint: component_fingerprint.clone(),
                });
            }
        }
    }
    files.sort_by(|a, b| a.file.cmp(&b.file));

    TypecheckSummaryCache {
        schema: TYPECHECK_SUMMARY_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        files,
    }
}

pub(crate) fn typecheck_summary_cache_matches(
    cache: &TypecheckSummaryCache,
    current_fingerprints: &HashMap<PathBuf, String>,
    components: &[Vec<PathBuf>],
) -> bool {
    let cached_entries: HashMap<&PathBuf, &TypecheckSummaryFileEntry> = cache
        .files
        .iter()
        .map(|entry| (&entry.file, entry))
        .collect();

    if cached_entries.len() != current_fingerprints.len() {
        return false;
    }

    for component in components {
        let current_component_fingerprint = component_fingerprint(component, current_fingerprints);
        for file in component {
            let Some(entry) = cached_entries.get(file) else {
                return false;
            };
            let Some(current_semantic_fingerprint) = current_fingerprints.get(file) else {
                return false;
            };
            if entry.semantic_fingerprint != *current_semantic_fingerprint
                || entry.component_fingerprint != current_component_fingerprint
            {
                return false;
            }
        }
    }

    true
}

pub(crate) fn reusable_component_fingerprints(
    cache: &TypecheckSummaryCache,
    current_fingerprints: &HashMap<PathBuf, String>,
    components: &[Vec<PathBuf>],
) -> HashSet<String> {
    let cached_entries: HashMap<&PathBuf, &TypecheckSummaryFileEntry> = cache
        .files
        .iter()
        .map(|entry| (&entry.file, entry))
        .collect();

    let mut reusable = HashSet::new();
    for component in components {
        let current_component_fingerprint = component_fingerprint(component, current_fingerprints);
        let matches = component.iter().all(|file| {
            cached_entries.get(file).is_some_and(|entry| {
                current_fingerprints.get(file).is_some_and(|current_fp| {
                    entry.semantic_fingerprint == *current_fp
                        && entry.component_fingerprint == current_component_fingerprint
                })
            })
        });
        if matches {
            reusable.insert(current_component_fingerprint);
        }
    }
    reusable
}

pub(crate) fn merge_reusable_component_semantic_data(
    cache: &SemanticSummaryCache,
    reusable_component_fingerprints: &HashSet<String>,
) -> (
    FunctionEffectsSummary,
    ClassMethodEffectsSummary,
    HashMap<String, HashSet<String>>,
) {
    let reusable_components = cache
        .components
        .iter()
        .filter(|component| {
            reusable_component_fingerprints.contains(&component.component_fingerprint)
        })
        .collect::<Vec<_>>();

    let mut function_effects = HashMap::new();
    let mut class_method_effects = HashMap::new();
    let mut class_mutating_methods = HashMap::new();

    for component in reusable_components {
        for function_name in &component.function_names {
            if let Some(effects) = cache.function_effects.get(function_name) {
                function_effects.insert(function_name.clone(), effects.clone());
            }
        }
        for class_name in &component.class_names {
            if let Some(methods) = cache.class_method_effects.get(class_name) {
                class_method_effects.insert(class_name.clone(), methods.clone());
            }
            if let Some(methods) = cache.class_mutating_methods.get(class_name) {
                class_mutating_methods
                    .insert(class_name.clone(), methods.iter().cloned().collect());
            }
        }
    }

    (
        function_effects,
        class_method_effects,
        class_mutating_methods,
    )
}

pub(crate) fn semantic_program_for_component(
    rewritten_files: &[RewrittenProjectUnit],
    component_files: &HashSet<PathBuf>,
    full_files: &HashSet<PathBuf>,
) -> Program {
    let mut program = Program {
        package: None,
        declarations: Vec::new(),
    };

    for unit in rewritten_files {
        if !component_files.contains(&unit.file) {
            continue;
        }
        let source_program = if full_files.contains(&unit.file) {
            unit.program.clone()
        } else {
            unit.api_program.clone()
        };
        program.declarations.extend(source_program.declarations);
    }

    program
}
