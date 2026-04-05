use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use crate::ast::{self, Decl, Expr, ImportDecl, Program, Stmt};
use crate::cache::elapsed_nanos_u64;
use crate::parser::parse_type_source;
use crate::stdlib::stdlib_registry;

type ImportedMap = HashMap<String, (String, String)>;

#[derive(Debug, Clone, Default)]
pub struct RewriteTimingSnapshot {
    pub import_map_build_ns: u64,
    pub wildcard_match_ns: u64,
    pub wildcard_match_calls: usize,
    pub exact_import_resolve_ns: u64,
    pub exact_import_resolve_calls: usize,
    pub block_rewrite_ns: u64,
    pub block_rewrite_calls: usize,
    pub stmt_rewrite_ns: u64,
    pub stmt_rewrite_calls: usize,
    pub expr_rewrite_ns: u64,
    pub expr_rewrite_calls: usize,
    pub type_rewrite_ns: u64,
    pub type_rewrite_calls: usize,
    pub pattern_rewrite_ns: u64,
    pub pattern_rewrite_calls: usize,
}

#[derive(Default)]
struct RewriteInternalTimingTotals {
    import_map_build_ns: AtomicU64,
    wildcard_match_ns: AtomicU64,
    wildcard_match_calls: AtomicUsize,
    exact_import_resolve_ns: AtomicU64,
    exact_import_resolve_calls: AtomicUsize,
    block_rewrite_ns: AtomicU64,
    block_rewrite_calls: AtomicUsize,
    stmt_rewrite_ns: AtomicU64,
    stmt_rewrite_calls: AtomicUsize,
    expr_rewrite_ns: AtomicU64,
    expr_rewrite_calls: AtomicUsize,
    type_rewrite_ns: AtomicU64,
    type_rewrite_calls: AtomicUsize,
    pattern_rewrite_ns: AtomicU64,
    pattern_rewrite_calls: AtomicUsize,
}

static REWRITE_INTERNAL_TIMING_TOTALS: RewriteInternalTimingTotals = RewriteInternalTimingTotals {
    import_map_build_ns: AtomicU64::new(0),
    wildcard_match_ns: AtomicU64::new(0),
    wildcard_match_calls: AtomicUsize::new(0),
    exact_import_resolve_ns: AtomicU64::new(0),
    exact_import_resolve_calls: AtomicUsize::new(0),
    block_rewrite_ns: AtomicU64::new(0),
    block_rewrite_calls: AtomicUsize::new(0),
    stmt_rewrite_ns: AtomicU64::new(0),
    stmt_rewrite_calls: AtomicUsize::new(0),
    expr_rewrite_ns: AtomicU64::new(0),
    expr_rewrite_calls: AtomicUsize::new(0),
    type_rewrite_ns: AtomicU64::new(0),
    type_rewrite_calls: AtomicUsize::new(0),
    pattern_rewrite_ns: AtomicU64::new(0),
    pattern_rewrite_calls: AtomicUsize::new(0),
};

pub fn reset_rewrite_timings() {
    REWRITE_INTERNAL_TIMING_TOTALS
        .import_map_build_ns
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .wildcard_match_ns
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .wildcard_match_calls
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .exact_import_resolve_ns
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .exact_import_resolve_calls
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .block_rewrite_ns
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .block_rewrite_calls
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .stmt_rewrite_ns
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .stmt_rewrite_calls
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .expr_rewrite_ns
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .expr_rewrite_calls
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .type_rewrite_ns
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .type_rewrite_calls
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .pattern_rewrite_ns
        .store(0, Ordering::Relaxed);
    REWRITE_INTERNAL_TIMING_TOTALS
        .pattern_rewrite_calls
        .store(0, Ordering::Relaxed);
}

pub fn snapshot_rewrite_timings() -> RewriteTimingSnapshot {
    RewriteTimingSnapshot {
        import_map_build_ns: REWRITE_INTERNAL_TIMING_TOTALS
            .import_map_build_ns
            .load(Ordering::Relaxed),
        wildcard_match_ns: REWRITE_INTERNAL_TIMING_TOTALS
            .wildcard_match_ns
            .load(Ordering::Relaxed),
        wildcard_match_calls: REWRITE_INTERNAL_TIMING_TOTALS
            .wildcard_match_calls
            .load(Ordering::Relaxed),
        exact_import_resolve_ns: REWRITE_INTERNAL_TIMING_TOTALS
            .exact_import_resolve_ns
            .load(Ordering::Relaxed),
        exact_import_resolve_calls: REWRITE_INTERNAL_TIMING_TOTALS
            .exact_import_resolve_calls
            .load(Ordering::Relaxed),
        block_rewrite_ns: REWRITE_INTERNAL_TIMING_TOTALS
            .block_rewrite_ns
            .load(Ordering::Relaxed),
        block_rewrite_calls: REWRITE_INTERNAL_TIMING_TOTALS
            .block_rewrite_calls
            .load(Ordering::Relaxed),
        stmt_rewrite_ns: REWRITE_INTERNAL_TIMING_TOTALS
            .stmt_rewrite_ns
            .load(Ordering::Relaxed),
        stmt_rewrite_calls: REWRITE_INTERNAL_TIMING_TOTALS
            .stmt_rewrite_calls
            .load(Ordering::Relaxed),
        expr_rewrite_ns: REWRITE_INTERNAL_TIMING_TOTALS
            .expr_rewrite_ns
            .load(Ordering::Relaxed),
        expr_rewrite_calls: REWRITE_INTERNAL_TIMING_TOTALS
            .expr_rewrite_calls
            .load(Ordering::Relaxed),
        type_rewrite_ns: REWRITE_INTERNAL_TIMING_TOTALS
            .type_rewrite_ns
            .load(Ordering::Relaxed),
        type_rewrite_calls: REWRITE_INTERNAL_TIMING_TOTALS
            .type_rewrite_calls
            .load(Ordering::Relaxed),
        pattern_rewrite_ns: REWRITE_INTERNAL_TIMING_TOTALS
            .pattern_rewrite_ns
            .load(Ordering::Relaxed),
        pattern_rewrite_calls: REWRITE_INTERNAL_TIMING_TOTALS
            .pattern_rewrite_calls
            .load(Ordering::Relaxed),
    }
}

struct RewriteTypeContext<'a> {
    current_namespace: &'a str,
    local_classes: &'a HashSet<String>,
    imported_classes: &'a ImportedMap,
    global_class_map: &'a HashMap<String, String>,
    local_interfaces: &'a HashSet<String>,
    imported_interfaces: &'a ImportedMap,
    global_interface_map: &'a HashMap<String, String>,
    local_enums: &'a HashSet<String>,
    imported_enums: &'a ImportedMap,
    global_enum_map: &'a HashMap<String, String>,
    imported_modules: &'a ImportedMap,
    entry_namespace: &'a str,
}

fn alias_qualified_symbol_name(alias: &str, symbol_name: &str) -> String {
    format!("{}.{}", alias, symbol_name.replace("__", "."))
}

fn resolve_exact_imported_symbol_path(
    namespace_path: &str,
    symbol_name: &str,
    global_symbol_map: &HashMap<String, String>,
) -> Option<(String, String)> {
    let started_at = Instant::now();
    REWRITE_INTERNAL_TIMING_TOTALS
        .exact_import_resolve_calls
        .fetch_add(1, Ordering::Relaxed);
    if global_symbol_map
        .get(symbol_name)
        .is_some_and(|owner_ns| owner_ns == namespace_path)
    {
        let result = Some((namespace_path.to_string(), symbol_name.to_string()));
        REWRITE_INTERNAL_TIMING_TOTALS
            .exact_import_resolve_ns
            .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
        return result;
    }

    let full_path = format!("{}.{}", namespace_path, symbol_name);
    let mut matches = global_symbol_map
        .iter()
        .filter_map(|(candidate, owner_ns)| {
            let candidate_path = format!("{}.{}", owner_ns, candidate.replace("__", "."));
            (candidate_path == full_path).then(|| (owner_ns.clone(), candidate.clone()))
        })
        .collect::<Vec<_>>();
    matches.sort_unstable();
    matches.dedup();
    let result = (matches.len() == 1).then(|| matches.swap_remove(0));
    REWRITE_INTERNAL_TIMING_TOTALS
        .exact_import_resolve_ns
        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
    result
}

fn resolve_exact_imported_variant_alias(
    import_ns: &str,
    symbol_name: &str,
    global_enum_map: &HashMap<String, String>,
) -> Option<(String, String, String)> {
    if symbol_name.contains('.') || symbol_name.contains("__") {
        return None;
    }
    let (namespace_path, enum_leaf) = import_ns
        .rsplit_once('.')
        .map_or((String::new(), import_ns.to_string()), |(ns, name)| {
            (ns.to_string(), name.to_string())
        });
    resolve_exact_imported_symbol_path(&namespace_path, &enum_leaf, global_enum_map)
        .map(|(owner_ns, enum_name)| (owner_ns, enum_name, symbol_name.to_string()))
}

fn direct_wildcard_member_name(
    import_path: &str,
    owner_ns: &str,
    symbol_name: &str,
) -> Option<String> {
    let started_at = Instant::now();
    REWRITE_INTERNAL_TIMING_TOTALS
        .wildcard_match_calls
        .fetch_add(1, Ordering::Relaxed);
    let result = if owner_ns == import_path {
        (!symbol_name.contains("__")).then(|| symbol_name.to_string())
    } else {
        let module_path = import_path.strip_prefix(owner_ns)?.strip_prefix('.')?;
        if module_path.is_empty() {
            None
        } else {
            let module_prefix = module_path.replace('.', "__");
            let remainder = symbol_name.strip_prefix(&format!("{}__", module_prefix))?;
            (!remainder.is_empty() && !remainder.contains("__")).then(|| remainder.to_string())
        }
    };
    REWRITE_INTERNAL_TIMING_TOTALS
        .wildcard_match_ns
        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
    result
}

fn resolve_exact_imported_symbol_from_namespaces(
    namespace_path: &str,
    symbol_name: &str,
    namespace_symbols: &HashMap<String, HashSet<String>>,
) -> Option<(String, String)> {
    let started_at = Instant::now();
    REWRITE_INTERNAL_TIMING_TOTALS
        .exact_import_resolve_calls
        .fetch_add(1, Ordering::Relaxed);

    let mut matches = Vec::new();
    let mut current = Some(namespace_path);
    while let Some(owner_ns) = current {
        if let Some(symbols) = namespace_symbols.get(owner_ns) {
            let candidate = if owner_ns == namespace_path {
                symbol_name.to_string()
            } else if let Some(module_path) = namespace_path
                .strip_prefix(owner_ns)
                .and_then(|rest| rest.strip_prefix('.'))
            {
                format!("{}__{}", module_path.replace('.', "__"), symbol_name)
            } else {
                current = owner_ns.rsplit_once('.').map(|(parent, _)| parent);
                continue;
            };
            if symbols.contains(&candidate) {
                matches.push((owner_ns.to_string(), candidate));
            }
        }
        current = owner_ns.rsplit_once('.').map(|(parent, _)| parent);
    }

    matches.sort_unstable();
    matches.dedup();
    let result = (matches.len() == 1).then(|| matches.swap_remove(0));
    REWRITE_INTERNAL_TIMING_TOTALS
        .exact_import_resolve_ns
        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
    result
}

fn extend_wildcard_import_map(
    import_path: &str,
    namespace_symbols: &HashMap<String, HashSet<String>>,
    imported: &mut ImportedMap,
) {
    let started_at = Instant::now();
    let mut current = Some(import_path);
    while let Some(owner_ns) = current {
        let Some(symbols) = namespace_symbols.get(owner_ns) else {
            current = owner_ns.rsplit_once('.').map(|(parent, _)| parent);
            continue;
        };
        let module_prefix = if owner_ns == import_path {
            None
        } else {
            import_path
                .strip_prefix(owner_ns)
                .and_then(|rest| rest.strip_prefix('.'))
                .map(|rest| format!("{}__", rest.replace('.', "__")))
        };
        for symbol_name in symbols {
            REWRITE_INTERNAL_TIMING_TOTALS
                .wildcard_match_calls
                .fetch_add(1, Ordering::Relaxed);
            let imported_name = match &module_prefix {
                None => (!symbol_name.contains("__")).then(|| symbol_name.clone()),
                Some(prefix) => symbol_name
                    .strip_prefix(prefix)
                    .filter(|remainder| !remainder.is_empty() && !remainder.contains("__"))
                    .map(str::to_string),
            };
            if let Some(imported_name) = imported_name {
                imported.insert(imported_name, (owner_ns.to_string(), symbol_name.clone()));
            }
        }
        current = owner_ns.rsplit_once('.').map(|(parent, _)| parent);
    }
    REWRITE_INTERNAL_TIMING_TOTALS
        .wildcard_match_ns
        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
}

fn format_type_string(ty: &ast::Type) -> String {
    match ty {
        ast::Type::Integer => "Integer".to_string(),
        ast::Type::Float => "Float".to_string(),
        ast::Type::Boolean => "Boolean".to_string(),
        ast::Type::String => "String".to_string(),
        ast::Type::Char => "Char".to_string(),
        ast::Type::None => "None".to_string(),
        ast::Type::Named(name) => name.clone(),
        ast::Type::Option(inner) => format!("Option<{}>", format_type_string(inner)),
        ast::Type::Result(ok, err) => {
            format!(
                "Result<{}, {}>",
                format_type_string(ok),
                format_type_string(err)
            )
        }
        ast::Type::List(inner) => format!("List<{}>", format_type_string(inner)),
        ast::Type::Map(k, v) => {
            format!("Map<{}, {}>", format_type_string(k), format_type_string(v))
        }
        ast::Type::Set(inner) => format!("Set<{}>", format_type_string(inner)),
        ast::Type::Ref(inner) => format!("&{}", format_type_string(inner)),
        ast::Type::MutRef(inner) => format!("&mut {}", format_type_string(inner)),
        ast::Type::Box(inner) => format!("Box<{}>", format_type_string(inner)),
        ast::Type::Rc(inner) => format!("Rc<{}>", format_type_string(inner)),
        ast::Type::Arc(inner) => format!("Arc<{}>", format_type_string(inner)),
        ast::Type::Ptr(inner) => format!("Ptr<{}>", format_type_string(inner)),
        ast::Type::Task(inner) => format!("Task<{}>", format_type_string(inner)),
        ast::Type::Range(inner) => format!("Range<{}>", format_type_string(inner)),
        ast::Type::Function(params, ret) => {
            let params_str = params
                .iter()
                .map(format_type_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({}) -> {}", params_str, format_type_string(ret))
        }
        ast::Type::Generic(name, args) => {
            let args_str = args
                .iter()
                .map(format_type_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{}>", name, args_str)
        }
    }
}

fn format_construct_type_name(base: &str, type_args: &[ast::Type]) -> String {
    if type_args.is_empty() {
        base.to_string()
    } else {
        let args = type_args
            .iter()
            .map(format_type_string)
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}<{}>", base, args)
    }
}

fn rewrite_generic_params_for_project(
    generic_params: &[ast::GenericParam],
    rewrite_bound: impl Fn(&str) -> String,
) -> Vec<ast::GenericParam> {
    generic_params
        .iter()
        .map(|param| ast::GenericParam {
            name: param.name.clone(),
            bounds: param
                .bounds
                .iter()
                .map(|bound| rewrite_bound(bound))
                .collect(),
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn rewrite_construct_type_name_for_project(
    ty: &str,
    current_namespace: &str,
    local_classes: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    local_interfaces: &HashSet<String>,
    imported_interfaces: &ImportedMap,
    global_interface_map: &HashMap<String, String>,
    local_enums: &HashSet<String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &ImportedMap,
    entry_namespace: &str,
) -> String {
    let ctx = RewriteTypeContext {
        current_namespace,
        local_classes,
        imported_classes,
        global_class_map,
        local_interfaces,
        imported_interfaces,
        global_interface_map,
        local_enums,
        imported_enums,
        global_enum_map,
        imported_modules,
        entry_namespace,
    };
    match parse_type_source(ty) {
        Ok(parsed) => format_type_string(&rewrite_type_for_project_with_ctx(&parsed, &ctx)),
        Err(_) => ty.to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::needless_borrow)]
pub fn rewrite_program_for_project(
    program: &Program,
    current_namespace: &str,
    entry_namespace: &str,
    namespace_functions: &HashMap<String, HashSet<String>>,
    global_function_map: &HashMap<String, String>,
    namespace_classes: &HashMap<String, HashSet<String>>,
    global_class_map: &HashMap<String, String>,
    namespace_interfaces: &HashMap<String, HashSet<String>>,
    global_interface_map: &HashMap<String, String>,
    namespace_enums: &HashMap<String, HashSet<String>>,
    global_enum_map: &HashMap<String, String>,
    namespace_modules: &HashMap<String, HashSet<String>>,
    global_module_map: &HashMap<String, String>,
    imports: &[ImportDecl],
) -> Program {
    let import_map_started_at = Instant::now();
    let empty_functions = HashSet::new();
    let empty_classes = HashSet::new();
    let empty_interfaces = HashSet::new();
    let empty_enums = HashSet::new();
    let empty_modules = HashSet::new();
    let local_functions = namespace_functions
        .get(current_namespace)
        .unwrap_or(&empty_functions);
    let local_classes = namespace_classes
        .get(current_namespace)
        .unwrap_or(&empty_classes);
    let local_interfaces = namespace_interfaces
        .get(current_namespace)
        .unwrap_or(&empty_interfaces);
    let local_enums = namespace_enums
        .get(current_namespace)
        .unwrap_or(&empty_enums);
    let local_modules = namespace_modules
        .get(current_namespace)
        .unwrap_or(&empty_modules);

    let mut imported_map: ImportedMap = HashMap::new();
    let mut imported_classes: ImportedMap = HashMap::new();
    let mut imported_interfaces: ImportedMap = HashMap::new();
    let mut imported_enums: ImportedMap = HashMap::new();
    let mut imported_modules: ImportedMap = HashMap::new();
    for import in imports {
        let import_key = import
            .alias
            .as_ref()
            .cloned()
            .unwrap_or_else(|| import.path.rsplit('.').next().unwrap_or("").to_string());
        if import.path.ends_with(".*") {
            let import_path = import.path.trim_end_matches(".*");
            let imported_map_before = imported_map.len();
            extend_wildcard_import_map(import_path, namespace_functions, &mut imported_map);
            if imported_map.len() == imported_map_before {
                for (symbol_name, owner_ns) in global_function_map {
                    if let Some(imported_name) =
                        direct_wildcard_member_name(import_path, owner_ns, symbol_name)
                    {
                        imported_map.insert(imported_name, (owner_ns.clone(), symbol_name.clone()));
                    }
                }
            }
            let imported_classes_before = imported_classes.len();
            extend_wildcard_import_map(import_path, namespace_classes, &mut imported_classes);
            if imported_classes.len() == imported_classes_before {
                for (symbol_name, owner_ns) in global_class_map {
                    if let Some(imported_name) =
                        direct_wildcard_member_name(import_path, owner_ns, symbol_name)
                    {
                        imported_classes
                            .insert(imported_name, (owner_ns.clone(), symbol_name.clone()));
                    }
                }
            }
            let imported_interfaces_before = imported_interfaces.len();
            extend_wildcard_import_map(import_path, namespace_interfaces, &mut imported_interfaces);
            if imported_interfaces.len() == imported_interfaces_before {
                for (symbol_name, owner_ns) in global_interface_map {
                    if let Some(imported_name) =
                        direct_wildcard_member_name(import_path, owner_ns, symbol_name)
                    {
                        imported_interfaces
                            .insert(imported_name, (owner_ns.clone(), symbol_name.clone()));
                    }
                }
            }
            let imported_enums_before = imported_enums.len();
            extend_wildcard_import_map(import_path, namespace_enums, &mut imported_enums);
            if imported_enums.len() == imported_enums_before {
                for (symbol_name, owner_ns) in global_enum_map {
                    if let Some(imported_name) =
                        direct_wildcard_member_name(import_path, owner_ns, symbol_name)
                    {
                        imported_enums
                            .insert(imported_name, (owner_ns.clone(), symbol_name.clone()));
                    }
                }
            }
            let imported_modules_before = imported_modules.len();
            extend_wildcard_import_map(import_path, namespace_modules, &mut imported_modules);
            if imported_modules.len() == imported_modules_before {
                for (symbol_name, owner_ns) in global_module_map {
                    if let Some(imported_name) =
                        direct_wildcard_member_name(import_path, owner_ns, symbol_name)
                    {
                        imported_modules
                            .insert(imported_name, (owner_ns.clone(), symbol_name.clone()));
                    }
                }
            }
        } else if import.path.contains('.') {
            let mut parts = import.path.split('.').collect::<Vec<_>>();
            if let Some(source_name) = parts.pop() {
                let ns = parts.join(".");
                if let Some((owner_ns, function_name)) =
                    resolve_exact_imported_symbol_from_namespaces(
                        &ns,
                        source_name,
                        namespace_functions,
                    )
                    .or_else(|| {
                        resolve_exact_imported_symbol_path(&ns, source_name, global_function_map)
                    })
                {
                    imported_map.insert(import_key.clone(), (owner_ns, function_name));
                }
                if let Some((owner_ns, class_name)) = resolve_exact_imported_symbol_from_namespaces(
                    &ns,
                    source_name,
                    namespace_classes,
                )
                .or_else(|| resolve_exact_imported_symbol_path(&ns, source_name, global_class_map))
                {
                    imported_classes.insert(import_key.clone(), (owner_ns, class_name));
                }
                if let Some((owner_ns, interface_name)) =
                    resolve_exact_imported_symbol_from_namespaces(
                        &ns,
                        source_name,
                        namespace_interfaces,
                    )
                    .or_else(|| {
                        resolve_exact_imported_symbol_path(&ns, source_name, global_interface_map)
                    })
                {
                    imported_interfaces.insert(import_key.clone(), (owner_ns, interface_name));
                }
                if let Some((owner_ns, enum_name)) =
                    resolve_exact_imported_symbol_from_namespaces(&ns, source_name, namespace_enums)
                        .or_else(|| {
                            resolve_exact_imported_symbol_path(&ns, source_name, global_enum_map)
                        })
                {
                    imported_enums.insert(import_key.clone(), (owner_ns, enum_name));
                }
                imported_modules.insert(import_key, (ns, source_name.to_string()));
            }
        } else if namespace_functions.contains_key(&import.path)
            || namespace_classes.contains_key(&import.path)
            || namespace_interfaces.contains_key(&import.path)
            || namespace_enums.contains_key(&import.path)
            || namespace_modules.contains_key(&import.path)
        {
            // Namespace import without explicit symbol (e.g. `import math_utils as mu`)
            // should allow `mu.someFunction()` rewrite resolution.
            imported_modules.insert(import_key.clone(), (import.path.clone(), String::new()));
            if let Some(symbols) = namespace_classes.get(&import.path) {
                for symbol_name in symbols {
                    imported_classes.insert(
                        alias_qualified_symbol_name(&import_key, symbol_name),
                        (import.path.clone(), symbol_name.clone()),
                    );
                }
            }
            if let Some(symbols) = namespace_interfaces.get(&import.path) {
                for symbol_name in symbols {
                    imported_interfaces.insert(
                        alias_qualified_symbol_name(&import_key, symbol_name),
                        (import.path.clone(), symbol_name.clone()),
                    );
                }
            }
            if let Some(symbols) = namespace_enums.get(&import.path) {
                for symbol_name in symbols {
                    imported_enums.insert(
                        alias_qualified_symbol_name(&import_key, symbol_name),
                        (import.path.clone(), symbol_name.clone()),
                    );
                }
            }
        }
    }
    REWRITE_INTERNAL_TIMING_TOTALS
        .import_map_build_ns
        .fetch_add(elapsed_nanos_u64(import_map_started_at), Ordering::Relaxed);

    Program {
        package: None,
        declarations: program
            .declarations
            .iter()
            .filter(|d| !matches!(d.node, Decl::Import(_)))
            .map(|d| {
                let node = match &d.node {
                    Decl::Function(func) => {
                        let mut f = func.clone();
                        f.generic_params =
                            rewrite_generic_params_for_project(&f.generic_params, |bound| {
                                rewrite_interface_reference_for_project(
                                    bound,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    &local_interfaces,
                                    &imported_interfaces,
                                    &local_enums,
                                    &imported_enums,
                                    global_enum_map,
                                    &imported_modules,
                                    global_interface_map,
                                    entry_namespace,
                                )
                            });
                        let mut scopes = vec![f.params.iter().map(|p| p.name.clone()).collect()];
                        f.params = f
                            .params
                            .iter()
                            .map(|p| ast::Parameter {
                                name: p.name.clone(),
                                ty: rewrite_type_for_project_with_interfaces(
                                    &p.ty,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    &local_interfaces,
                                    &imported_interfaces,
                                    global_interface_map,
                                    &local_enums,
                                    &imported_enums,
                                    global_enum_map,
                                    &imported_modules,
                                    entry_namespace,
                                ),
                                mutable: p.mutable,
                                mode: p.mode,
                            })
                            .collect();
                        f.return_type = rewrite_type_for_project_with_interfaces(
                            &f.return_type,
                            current_namespace,
                            &local_classes,
                            &imported_classes,
                            global_class_map,
                            &local_interfaces,
                            &imported_interfaces,
                            global_interface_map,
                            &local_enums,
                            &imported_enums,
                            global_enum_map,
                            &imported_modules,
                            entry_namespace,
                        );
                        f.body = rewrite_block_calls_for_project(
                            &f.body,
                            current_namespace,
                            entry_namespace,
                            &local_functions,
                            &imported_map,
                            global_function_map,
                            &local_classes,
                            &imported_classes,
                            global_class_map,
                            &local_interfaces,
                            &imported_interfaces,
                            global_interface_map,
                            &imported_enums,
                            global_enum_map,
                            &local_modules,
                            &imported_modules,
                            global_module_map,
                            &mut scopes,
                        );
                        f.name = mangle_project_function_symbol(
                            current_namespace,
                            entry_namespace,
                            &f.name,
                        );
                        Decl::Function(f)
                    }
                    Decl::Class(class) => {
                        let mut c = class.clone();
                        c.name = mangle_project_symbol(current_namespace, entry_namespace, &c.name);
                        c.generic_params =
                            rewrite_generic_params_for_project(&c.generic_params, |bound| {
                                rewrite_interface_reference_for_project(
                                    bound,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    &local_interfaces,
                                    &imported_interfaces,
                                    &local_enums,
                                    &imported_enums,
                                    global_enum_map,
                                    &imported_modules,
                                    global_interface_map,
                                    entry_namespace,
                                )
                            });
                        c.extends = class.extends.as_ref().map(|extends| {
                            rewrite_named_reference_for_project(
                                extends,
                                current_namespace,
                                &local_classes,
                                &imported_classes,
                                global_class_map,
                                &local_enums,
                                &imported_enums,
                                global_enum_map,
                                &imported_modules,
                                entry_namespace,
                            )
                        });
                        c.implements = class
                            .implements
                            .iter()
                            .map(|implemented| {
                                rewrite_interface_reference_for_project(
                                    implemented,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    &local_interfaces,
                                    &imported_interfaces,
                                    &local_enums,
                                    &imported_enums,
                                    global_enum_map,
                                    &imported_modules,
                                    global_interface_map,
                                    entry_namespace,
                                )
                            })
                            .collect();
                        c.fields = c
                            .fields
                            .iter()
                            .map(|field| ast::Field {
                                name: field.name.clone(),
                                ty: rewrite_type_for_project_with_interfaces(
                                    &field.ty,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    &local_interfaces,
                                    &imported_interfaces,
                                    global_interface_map,
                                    &local_enums,
                                    &imported_enums,
                                    global_enum_map,
                                    &imported_modules,
                                    entry_namespace,
                                ),
                                mutable: field.mutable,
                                visibility: field.visibility,
                            })
                            .collect();
                        if let Some(ctor) = &class.constructor {
                            let mut new_ctor = ctor.clone();
                            let mut scopes: Vec<HashSet<String>> =
                                vec![new_ctor.params.iter().map(|p| p.name.clone()).collect()];
                            if let Some(scope) = scopes.last_mut() {
                                scope.insert("this".to_string());
                            }
                            new_ctor.params = new_ctor
                                .params
                                .iter()
                                .map(|p| ast::Parameter {
                                    name: p.name.clone(),
                                    ty: rewrite_type_for_project_with_interfaces(
                                        &p.ty,
                                        current_namespace,
                                        &local_classes,
                                        &imported_classes,
                                        global_class_map,
                                        &local_interfaces,
                                        &imported_interfaces,
                                        global_interface_map,
                                        &local_enums,
                                        &imported_enums,
                                        global_enum_map,
                                        &imported_modules,
                                        entry_namespace,
                                    ),
                                    mutable: p.mutable,
                                    mode: p.mode,
                                })
                                .collect();
                            new_ctor.body = rewrite_block_calls_for_project(
                                &new_ctor.body,
                                current_namespace,
                                entry_namespace,
                                &local_functions,
                                &imported_map,
                                global_function_map,
                                &local_classes,
                                &imported_classes,
                                global_class_map,
                                &local_interfaces,
                                &imported_interfaces,
                                global_interface_map,
                                &imported_enums,
                                global_enum_map,
                                &local_modules,
                                &imported_modules,
                                global_module_map,
                                &mut scopes,
                            );
                            c.constructor = Some(new_ctor);
                        }
                        if let Some(dtor) = &class.destructor {
                            let mut new_dtor = dtor.clone();
                            let mut scopes: Vec<HashSet<String>> =
                                vec![HashSet::from(["this".to_string()])];
                            new_dtor.body = rewrite_block_calls_for_project(
                                &new_dtor.body,
                                current_namespace,
                                entry_namespace,
                                &local_functions,
                                &imported_map,
                                global_function_map,
                                &local_classes,
                                &imported_classes,
                                global_class_map,
                                &local_interfaces,
                                &imported_interfaces,
                                global_interface_map,
                                &imported_enums,
                                global_enum_map,
                                &local_modules,
                                &imported_modules,
                                global_module_map,
                                &mut scopes,
                            );
                            c.destructor = Some(new_dtor);
                        }
                        c.methods = class
                            .methods
                            .iter()
                            .map(|m| {
                                let mut nm = m.clone();
                                nm.generic_params = rewrite_generic_params_for_project(
                                    &nm.generic_params,
                                    |bound| {
                                        rewrite_interface_reference_for_project(
                                            bound,
                                            current_namespace,
                                            &local_classes,
                                            &imported_classes,
                                            global_class_map,
                                            &local_interfaces,
                                            &imported_interfaces,
                                            &local_enums,
                                            &imported_enums,
                                            global_enum_map,
                                            &imported_modules,
                                            global_interface_map,
                                            entry_namespace,
                                        )
                                    },
                                );
                                let mut scopes: Vec<HashSet<String>> =
                                    vec![nm.params.iter().map(|p| p.name.clone()).collect()];
                                if let Some(scope) = scopes.last_mut() {
                                    scope.insert("this".to_string());
                                }
                                nm.params = nm
                                    .params
                                    .iter()
                                    .map(|p| ast::Parameter {
                                        name: p.name.clone(),
                                        ty: rewrite_type_for_project_with_interfaces(
                                            &p.ty,
                                            current_namespace,
                                            &local_classes,
                                            &imported_classes,
                                            global_class_map,
                                            &local_interfaces,
                                            &imported_interfaces,
                                            global_interface_map,
                                            &local_enums,
                                            &imported_enums,
                                            global_enum_map,
                                            &imported_modules,
                                            entry_namespace,
                                        ),
                                        mutable: p.mutable,
                                        mode: p.mode,
                                    })
                                    .collect();
                                nm.return_type = rewrite_type_for_project_with_interfaces(
                                    &nm.return_type,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    &local_interfaces,
                                    &imported_interfaces,
                                    global_interface_map,
                                    &local_enums,
                                    &imported_enums,
                                    global_enum_map,
                                    &imported_modules,
                                    entry_namespace,
                                );
                                nm.body = rewrite_block_calls_for_project(
                                    &nm.body,
                                    current_namespace,
                                    entry_namespace,
                                    &local_functions,
                                    &imported_map,
                                    global_function_map,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    &local_interfaces,
                                    &imported_interfaces,
                                    global_interface_map,
                                    &imported_enums,
                                    global_enum_map,
                                    &local_modules,
                                    &imported_modules,
                                    global_module_map,
                                    &mut scopes,
                                );
                                nm
                            })
                            .collect();
                        Decl::Class(c)
                    }
                    Decl::Module(module) => {
                        let mut m = module.clone();
                        m.name = mangle_project_symbol(current_namespace, entry_namespace, &m.name);
                        let module_prefix = module.name.clone();
                        let (
                            module_local_functions,
                            module_local_classes,
                            module_local_interfaces,
                            module_local_enums,
                            module_local_modules,
                        ) = collect_direct_module_symbol_names(&module.declarations);
                        m.declarations = module
                            .declarations
                            .iter()
                            .map(|inner| {
                                let node = match &inner.node {
                                    Decl::Function(func) => {
                                        let mut f = func.clone();
                                        f.generic_params = rewrite_generic_params_for_project(
                                            &f.generic_params,
                                            |bound| {
                                                rewrite_interface_reference_for_module(
                                                    bound,
                                                    &module_prefix,
                                                    current_namespace,
                                                    entry_namespace,
                                                    &module_local_classes,
                                                    &module_local_interfaces,
                                                    &module_local_enums,
                                                    &module_local_modules,
                                                    &imported_classes,
                                                    &imported_interfaces,
                                                    &imported_enums,
                                                    &imported_modules,
                                                    global_class_map,
                                                    global_interface_map,
                                                    global_enum_map,
                                                )
                                            },
                                        );
                                        let mut scopes =
                                            vec![f.params.iter().map(|p| p.name.clone()).collect()];
                                        f.params = f
                                            .params
                                            .iter()
                                            .map(|p| ast::Parameter {
                                                name: p.name.clone(),
                                                ty: rewrite_module_local_type(
                                                    &p.ty,
                                                    &module_prefix,
                                                    current_namespace,
                                                    entry_namespace,
                                                    &module_local_classes,
                                                    &module_local_interfaces,
                                                    &module_local_enums,
                                                    &module_local_modules,
                                                    &imported_classes,
                                                    global_class_map,
                                                    &imported_enums,
                                                    global_enum_map,
                                                    &imported_modules,
                                                    global_interface_map,
                                                ),
                                                mutable: p.mutable,
                                                mode: p.mode,
                                            })
                                            .collect();
                                        f.return_type = rewrite_module_local_type(
                                            &f.return_type,
                                            &module_prefix,
                                            current_namespace,
                                            entry_namespace,
                                            &module_local_classes,
                                            &module_local_interfaces,
                                            &module_local_enums,
                                            &module_local_modules,
                                            &imported_classes,
                                            global_class_map,
                                            &imported_enums,
                                            global_enum_map,
                                            &imported_modules,
                                            global_interface_map,
                                        );
                                        f.body = fix_module_local_block(
                                            &rewrite_block_calls_for_project(
                                                &f.body,
                                                current_namespace,
                                                entry_namespace,
                                                &module_local_functions,
                                                &imported_map,
                                                global_function_map,
                                                &module_local_classes,
                                                &imported_classes,
                                                global_class_map,
                                                &module_local_interfaces,
                                                &imported_interfaces,
                                                global_interface_map,
                                                &imported_enums,
                                                global_enum_map,
                                                &module_local_modules,
                                                &imported_modules,
                                                global_module_map,
                                                &mut scopes,
                                            ),
                                            current_namespace,
                                            entry_namespace,
                                            &module_prefix,
                                            &module_local_functions,
                                            &module_local_classes,
                                            &module_local_interfaces,
                                            &module_local_enums,
                                            &module_local_modules,
                                            &imported_classes,
                                            global_class_map,
                                            &imported_enums,
                                            global_enum_map,
                                            &imported_modules,
                                            global_interface_map,
                                        );
                                        Decl::Function(f)
                                    }
                                    Decl::Class(class) => {
                                        let mut c = class.clone();
                                        c.generic_params = rewrite_generic_params_for_project(
                                            &c.generic_params,
                                            |bound| {
                                                rewrite_interface_reference_for_module(
                                                    bound,
                                                    &module_prefix,
                                                    current_namespace,
                                                    entry_namespace,
                                                    &module_local_classes,
                                                    &module_local_interfaces,
                                                    &module_local_enums,
                                                    &module_local_modules,
                                                    &imported_classes,
                                                    &imported_interfaces,
                                                    &imported_enums,
                                                    &imported_modules,
                                                    global_class_map,
                                                    global_interface_map,
                                                    global_enum_map,
                                                )
                                            },
                                        );
                                        c.extends = class.extends.as_ref().map(|extends| {
                                            match rewrite_module_local_type(
                                                &ast::Type::Named(extends.clone()),
                                                &module_prefix,
                                                current_namespace,
                                                entry_namespace,
                                                &module_local_classes,
                                                &module_local_interfaces,
                                                &module_local_enums,
                                                &module_local_modules,
                                                &imported_classes,
                                                global_class_map,
                                                &imported_enums,
                                                global_enum_map,
                                                &imported_modules,
                                                global_interface_map,
                                            ) {
                                                ast::Type::Named(rewritten) => rewritten,
                                                _ => extends.clone(),
                                            }
                                        });
                                        c.implements = class
                                            .implements
                                            .iter()
                                            .map(|implemented| {
                                                rewrite_interface_reference_for_module(
                                                    implemented,
                                                    &module_prefix,
                                                    current_namespace,
                                                    entry_namespace,
                                                    &module_local_classes,
                                                    &module_local_interfaces,
                                                    &module_local_enums,
                                                    &module_local_modules,
                                                    &imported_classes,
                                                    &imported_interfaces,
                                                    &imported_enums,
                                                    &imported_modules,
                                                    global_class_map,
                                                    global_interface_map,
                                                    global_enum_map,
                                                )
                                            })
                                            .collect();
                                        c.fields = c
                                            .fields
                                            .iter()
                                            .map(|field| ast::Field {
                                                name: field.name.clone(),
                                                ty: rewrite_module_local_type(
                                                    &field.ty,
                                                    &module_prefix,
                                                    current_namespace,
                                                    entry_namespace,
                                                    &module_local_classes,
                                                    &module_local_interfaces,
                                                    &module_local_enums,
                                                    &module_local_modules,
                                                    &imported_classes,
                                                    global_class_map,
                                                    &imported_enums,
                                                    global_enum_map,
                                                    &imported_modules,
                                                    global_interface_map,
                                                ),
                                                mutable: field.mutable,
                                                visibility: field.visibility,
                                            })
                                            .collect();
                                        if let Some(ctor) = &class.constructor {
                                            let mut new_ctor = ctor.clone();
                                            let mut scopes: Vec<HashSet<String>> = vec![new_ctor
                                                .params
                                                .iter()
                                                .map(|p| p.name.clone())
                                                .collect()];
                                            if let Some(scope) = scopes.last_mut() {
                                                scope.insert("this".to_string());
                                            }
                                            new_ctor.params = new_ctor
                                                .params
                                                .iter()
                                                .map(|p| ast::Parameter {
                                                    name: p.name.clone(),
                                                    ty: rewrite_module_local_type(
                                                        &p.ty,
                                                        &module_prefix,
                                                        current_namespace,
                                                        entry_namespace,
                                                        &module_local_classes,
                                                        &module_local_interfaces,
                                                        &module_local_enums,
                                                        &module_local_modules,
                                                        &imported_classes,
                                                        global_class_map,
                                                        &imported_enums,
                                                        global_enum_map,
                                                        &imported_modules,
                                                        global_interface_map,
                                                    ),
                                                    mutable: p.mutable,
                                                    mode: p.mode,
                                                })
                                                .collect();
                                            new_ctor.body = fix_module_local_block(
                                                &rewrite_block_calls_for_project(
                                                    &new_ctor.body,
                                                    current_namespace,
                                                    entry_namespace,
                                                    &module_local_functions,
                                                    &imported_map,
                                                    global_function_map,
                                                    &module_local_classes,
                                                    &imported_classes,
                                                    global_class_map,
                                                    &module_local_interfaces,
                                                    &imported_interfaces,
                                                    global_interface_map,
                                                    &imported_enums,
                                                    global_enum_map,
                                                    &module_local_modules,
                                                    &imported_modules,
                                                    global_module_map,
                                                    &mut scopes,
                                                ),
                                                current_namespace,
                                                entry_namespace,
                                                &module_prefix,
                                                &module_local_functions,
                                                &module_local_classes,
                                                &module_local_interfaces,
                                                &module_local_enums,
                                                &module_local_modules,
                                                &imported_classes,
                                                global_class_map,
                                                &imported_enums,
                                                global_enum_map,
                                                &imported_modules,
                                                global_interface_map,
                                            );
                                            c.constructor = Some(new_ctor);
                                        }
                                        if let Some(dtor) = &class.destructor {
                                            let mut new_dtor = dtor.clone();
                                            let mut scopes: Vec<HashSet<String>> =
                                                vec![HashSet::from(["this".to_string()])];
                                            new_dtor.body = fix_module_local_block(
                                                &rewrite_block_calls_for_project(
                                                    &new_dtor.body,
                                                    current_namespace,
                                                    entry_namespace,
                                                    &module_local_functions,
                                                    &imported_map,
                                                    global_function_map,
                                                    &module_local_classes,
                                                    &imported_classes,
                                                    global_class_map,
                                                    &module_local_interfaces,
                                                    &imported_interfaces,
                                                    global_interface_map,
                                                    &imported_enums,
                                                    global_enum_map,
                                                    &module_local_modules,
                                                    &imported_modules,
                                                    global_module_map,
                                                    &mut scopes,
                                                ),
                                                current_namespace,
                                                entry_namespace,
                                                &module_prefix,
                                                &module_local_functions,
                                                &module_local_classes,
                                                &module_local_interfaces,
                                                &module_local_enums,
                                                &module_local_modules,
                                                &imported_classes,
                                                global_class_map,
                                                &imported_enums,
                                                global_enum_map,
                                                &imported_modules,
                                                global_interface_map,
                                            );
                                            c.destructor = Some(new_dtor);
                                        }
                                        c.methods = class
                                            .methods
                                            .iter()
                                            .map(|method| {
                                                let mut nm = method.clone();
                                                nm.generic_params =
                                                    rewrite_generic_params_for_project(
                                                        &nm.generic_params,
                                                        |bound| {
                                                            rewrite_interface_reference_for_module(
                                                                bound,
                                                                &module_prefix,
                                                                current_namespace,
                                                                entry_namespace,
                                                                &module_local_classes,
                                                                &module_local_interfaces,
                                                                &module_local_enums,
                                                                &module_local_modules,
                                                                &imported_classes,
                                                                &imported_interfaces,
                                                                &imported_enums,
                                                                &imported_modules,
                                                                global_class_map,
                                                                global_interface_map,
                                                                global_enum_map,
                                                            )
                                                        },
                                                    );
                                                let mut scopes: Vec<HashSet<String>> = vec![nm
                                                    .params
                                                    .iter()
                                                    .map(|p| p.name.clone())
                                                    .collect()];
                                                if let Some(scope) = scopes.last_mut() {
                                                    scope.insert("this".to_string());
                                                }
                                                nm.params = nm
                                                    .params
                                                    .iter()
                                                    .map(|p| ast::Parameter {
                                                        name: p.name.clone(),
                                                        ty: rewrite_module_local_type(
                                                            &p.ty,
                                                            &module_prefix,
                                                            current_namespace,
                                                            entry_namespace,
                                                            &module_local_classes,
                                                            &module_local_interfaces,
                                                            &module_local_enums,
                                                            &module_local_modules,
                                                            &imported_classes,
                                                            global_class_map,
                                                            &imported_enums,
                                                            global_enum_map,
                                                            &imported_modules,
                                                            global_interface_map,
                                                        ),
                                                        mutable: p.mutable,
                                                        mode: p.mode,
                                                    })
                                                    .collect();
                                                nm.return_type = rewrite_module_local_type(
                                                    &nm.return_type,
                                                    &module_prefix,
                                                    current_namespace,
                                                    entry_namespace,
                                                    &module_local_classes,
                                                    &module_local_interfaces,
                                                    &module_local_enums,
                                                    &module_local_modules,
                                                    &imported_classes,
                                                    global_class_map,
                                                    &imported_enums,
                                                    global_enum_map,
                                                    &imported_modules,
                                                    global_interface_map,
                                                );
                                                nm.body = fix_module_local_block(
                                                    &rewrite_block_calls_for_project(
                                                        &nm.body,
                                                        current_namespace,
                                                        entry_namespace,
                                                        &module_local_functions,
                                                        &imported_map,
                                                        global_function_map,
                                                        &module_local_classes,
                                                        &imported_classes,
                                                        global_class_map,
                                                        &module_local_interfaces,
                                                        &imported_interfaces,
                                                        global_interface_map,
                                                        &imported_enums,
                                                        global_enum_map,
                                                        &module_local_modules,
                                                        &imported_modules,
                                                        global_module_map,
                                                        &mut scopes,
                                                    ),
                                                    current_namespace,
                                                    entry_namespace,
                                                    &module_prefix,
                                                    &module_local_functions,
                                                    &module_local_classes,
                                                    &module_local_interfaces,
                                                    &module_local_enums,
                                                    &module_local_modules,
                                                    &imported_classes,
                                                    global_class_map,
                                                    &imported_enums,
                                                    global_enum_map,
                                                    &imported_modules,
                                                    global_interface_map,
                                                );
                                                nm
                                            })
                                            .collect();
                                        Decl::Class(c)
                                    }
                                    Decl::Enum(en) => {
                                        let mut e = en.clone();
                                        e.generic_params = rewrite_generic_params_for_project(
                                            &e.generic_params,
                                            |bound| {
                                                rewrite_interface_reference_for_module(
                                                    bound,
                                                    &module_prefix,
                                                    current_namespace,
                                                    entry_namespace,
                                                    &module_local_classes,
                                                    &module_local_interfaces,
                                                    &module_local_enums,
                                                    &module_local_modules,
                                                    &imported_classes,
                                                    &imported_interfaces,
                                                    &imported_enums,
                                                    &imported_modules,
                                                    global_class_map,
                                                    global_interface_map,
                                                    global_enum_map,
                                                )
                                            },
                                        );
                                        e.variants = e
                                            .variants
                                            .iter()
                                            .map(|variant| ast::EnumVariant {
                                                name: variant.name.clone(),
                                                fields: variant
                                                    .fields
                                                    .iter()
                                                    .map(|field| ast::EnumField {
                                                        name: field.name.clone(),
                                                        ty: rewrite_module_local_type(
                                                            &field.ty,
                                                            &module_prefix,
                                                            current_namespace,
                                                            entry_namespace,
                                                            &module_local_classes,
                                                            &module_local_interfaces,
                                                            &module_local_enums,
                                                            &module_local_modules,
                                                            &imported_classes,
                                                            global_class_map,
                                                            &imported_enums,
                                                            global_enum_map,
                                                            &imported_modules,
                                                            global_interface_map,
                                                        ),
                                                    })
                                                    .collect(),
                                            })
                                            .collect();
                                        Decl::Enum(e)
                                    }
                                    Decl::Module(module) => {
                                        let nested_prefix =
                                            module_prefixed_symbol(&module_prefix, &module.name);
                                        let (
                                            nested_local_functions,
                                            nested_local_classes,
                                            nested_local_interfaces,
                                            nested_local_enums,
                                            nested_local_modules,
                                        ) = collect_direct_module_symbol_names(
                                            &module.declarations,
                                        );
                                        rewrite_nested_module_decl_for_project(
                                            inner,
                                            &nested_prefix,
                                            current_namespace,
                                            entry_namespace,
                                            &nested_local_functions,
                                            &nested_local_classes,
                                            &nested_local_interfaces,
                                            &nested_local_enums,
                                            &nested_local_modules,
                                            &imported_map,
                                            global_function_map,
                                            &imported_classes,
                                            global_class_map,
                                            &imported_interfaces,
                                            &imported_enums,
                                            global_enum_map,
                                            &imported_modules,
                                            global_interface_map,
                                            global_module_map,
                                        )
                                        .node
                                    }
                                    Decl::Interface(interface) => {
                                        let mut rewritten = interface.clone();
                                        rewritten.generic_params =
                                            rewrite_generic_params_for_project(
                                                &rewritten.generic_params,
                                                |bound| {
                                                    rewrite_interface_reference_for_module(
                                                        bound,
                                                        &module_prefix,
                                                        current_namespace,
                                                        entry_namespace,
                                                        &module_local_classes,
                                                        &module_local_interfaces,
                                                        &module_local_enums,
                                                        &module_local_modules,
                                                        &imported_classes,
                                                        &imported_interfaces,
                                                        &imported_enums,
                                                        &imported_modules,
                                                        global_class_map,
                                                        global_interface_map,
                                                        global_enum_map,
                                                    )
                                                },
                                            );
                                        rewritten.extends = interface
                                            .extends
                                            .iter()
                                            .map(|extended| {
                                                rewrite_interface_reference_for_module(
                                                    extended,
                                                    &module_prefix,
                                                    current_namespace,
                                                    entry_namespace,
                                                    &module_local_classes,
                                                    &module_local_interfaces,
                                                    &module_local_enums,
                                                    &module_local_modules,
                                                    &imported_classes,
                                                    &imported_interfaces,
                                                    &imported_enums,
                                                    &imported_modules,
                                                    global_class_map,
                                                    global_interface_map,
                                                    global_enum_map,
                                                )
                                            })
                                            .collect();
                                        rewritten.methods = interface
                                            .methods
                                            .iter()
                                            .map(|method| {
                                                let mut new_method = method.clone();
                                                let mut scopes: Vec<HashSet<String>> =
                                                    vec![new_method
                                                        .params
                                                        .iter()
                                                        .map(|p| p.name.clone())
                                                        .collect()];
                                                if let Some(scope) = scopes.last_mut() {
                                                    scope.insert("this".to_string());
                                                }
                                                new_method.params = new_method
                                                    .params
                                                    .iter()
                                                    .map(|param| ast::Parameter {
                                                        name: param.name.clone(),
                                                        ty: rewrite_module_local_type(
                                                            &param.ty,
                                                            &module_prefix,
                                                            current_namespace,
                                                            entry_namespace,
                                                            &module_local_classes,
                                                            &module_local_interfaces,
                                                            &module_local_enums,
                                                            &module_local_modules,
                                                            &imported_classes,
                                                            global_class_map,
                                                            &imported_enums,
                                                            global_enum_map,
                                                            &imported_modules,
                                                            global_interface_map,
                                                        ),
                                                        mutable: param.mutable,
                                                        mode: param.mode,
                                                    })
                                                    .collect();
                                                new_method.return_type = rewrite_module_local_type(
                                                    &new_method.return_type,
                                                    &module_prefix,
                                                    current_namespace,
                                                    entry_namespace,
                                                    &module_local_classes,
                                                    &module_local_interfaces,
                                                    &module_local_enums,
                                                    &module_local_modules,
                                                    &imported_classes,
                                                    global_class_map,
                                                    &imported_enums,
                                                    global_enum_map,
                                                    &imported_modules,
                                                    global_interface_map,
                                                );
                                                new_method.default_impl =
                                                    method.default_impl.as_ref().map(|block| {
                                                        fix_module_local_block(
                                                            &rewrite_block_calls_for_project(
                                                                block,
                                                                current_namespace,
                                                                entry_namespace,
                                                                &module_local_functions,
                                                                &imported_map,
                                                                global_function_map,
                                                                &module_local_classes,
                                                                &imported_classes,
                                                                global_class_map,
                                                                &module_local_interfaces,
                                                                &imported_interfaces,
                                                                global_interface_map,
                                                                &imported_enums,
                                                                global_enum_map,
                                                                &module_local_modules,
                                                                &imported_modules,
                                                                global_module_map,
                                                                &mut scopes,
                                                            ),
                                                            current_namespace,
                                                            entry_namespace,
                                                            &module_prefix,
                                                            &module_local_functions,
                                                            &module_local_classes,
                                                            &module_local_interfaces,
                                                            &module_local_enums,
                                                            &module_local_modules,
                                                            &imported_classes,
                                                            global_class_map,
                                                            &imported_enums,
                                                            global_enum_map,
                                                            &imported_modules,
                                                            global_interface_map,
                                                        )
                                                    });
                                                new_method
                                            })
                                            .collect();
                                        Decl::Interface(rewritten)
                                    }
                                    _ => inner.node.clone(),
                                };
                                ast::Spanned::new(node, inner.span.clone())
                            })
                            .collect();
                        Decl::Module(m)
                    }
                    Decl::Enum(en) => {
                        let mut e = en.clone();
                        e.name = mangle_project_symbol(current_namespace, entry_namespace, &e.name);
                        e.generic_params =
                            rewrite_generic_params_for_project(&e.generic_params, |bound| {
                                rewrite_interface_reference_for_project(
                                    bound,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    &local_interfaces,
                                    &imported_interfaces,
                                    &local_enums,
                                    &imported_enums,
                                    global_enum_map,
                                    &imported_modules,
                                    global_interface_map,
                                    entry_namespace,
                                )
                            });
                        e.variants = e
                            .variants
                            .iter()
                            .map(|v| ast::EnumVariant {
                                name: v.name.clone(),
                                fields: v
                                    .fields
                                    .iter()
                                    .map(|f| ast::EnumField {
                                        name: f.name.clone(),
                                        ty: rewrite_type_for_project_with_interfaces(
                                            &f.ty,
                                            current_namespace,
                                            &local_classes,
                                            &imported_classes,
                                            global_class_map,
                                            &local_interfaces,
                                            &imported_interfaces,
                                            global_interface_map,
                                            &local_enums,
                                            &imported_enums,
                                            global_enum_map,
                                            &imported_modules,
                                            entry_namespace,
                                        ),
                                    })
                                    .collect(),
                            })
                            .collect();
                        Decl::Enum(e)
                    }
                    Decl::Interface(interface) => {
                        let mut rewritten = interface.clone();
                        rewritten.name = mangle_project_symbol(
                            current_namespace,
                            entry_namespace,
                            &rewritten.name,
                        );
                        rewritten.generic_params = rewrite_generic_params_for_project(
                            &rewritten.generic_params,
                            |bound| {
                                rewrite_interface_reference_for_project(
                                    bound,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    &local_interfaces,
                                    &imported_interfaces,
                                    &local_enums,
                                    &imported_enums,
                                    global_enum_map,
                                    &imported_modules,
                                    global_interface_map,
                                    entry_namespace,
                                )
                            },
                        );
                        rewritten.extends = interface
                            .extends
                            .iter()
                            .map(|extended| {
                                rewrite_interface_reference_for_project(
                                    extended,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    &local_interfaces,
                                    &imported_interfaces,
                                    &local_enums,
                                    &imported_enums,
                                    global_enum_map,
                                    &imported_modules,
                                    global_interface_map,
                                    entry_namespace,
                                )
                            })
                            .collect();
                        rewritten.methods = interface
                            .methods
                            .iter()
                            .map(|method| {
                                let mut new_method = method.clone();
                                let mut scopes: Vec<HashSet<String>> = vec![new_method
                                    .params
                                    .iter()
                                    .map(|p| p.name.clone())
                                    .collect()];
                                if let Some(scope) = scopes.last_mut() {
                                    scope.insert("this".to_string());
                                }
                                new_method.params = new_method
                                    .params
                                    .iter()
                                    .map(|param| ast::Parameter {
                                        name: param.name.clone(),
                                        ty: rewrite_type_for_project_with_interfaces(
                                            &param.ty,
                                            current_namespace,
                                            &local_classes,
                                            &imported_classes,
                                            global_class_map,
                                            &local_interfaces,
                                            &imported_interfaces,
                                            global_interface_map,
                                            &local_enums,
                                            &imported_enums,
                                            global_enum_map,
                                            &imported_modules,
                                            entry_namespace,
                                        ),
                                        mutable: param.mutable,
                                        mode: param.mode,
                                    })
                                    .collect();
                                new_method.return_type = rewrite_type_for_project_with_interfaces(
                                    &new_method.return_type,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    &local_interfaces,
                                    &imported_interfaces,
                                    global_interface_map,
                                    &local_enums,
                                    &imported_enums,
                                    global_enum_map,
                                    &imported_modules,
                                    entry_namespace,
                                );
                                new_method.default_impl =
                                    method.default_impl.as_ref().map(|block| {
                                        rewrite_block_calls_for_project(
                                            block,
                                            current_namespace,
                                            entry_namespace,
                                            &local_functions,
                                            &imported_map,
                                            global_function_map,
                                            &local_classes,
                                            &imported_classes,
                                            global_class_map,
                                            &local_interfaces,
                                            &imported_interfaces,
                                            global_interface_map,
                                            &imported_enums,
                                            global_enum_map,
                                            &local_modules,
                                            &imported_modules,
                                            global_module_map,
                                            &mut scopes,
                                        )
                                    });
                                new_method
                            })
                            .collect();
                        Decl::Interface(rewritten)
                    }
                    _ => d.node.clone(),
                };
                ast::Spanned::new(node, d.span.clone())
            })
            .collect(),
    }
}

fn mangle_project_symbol(namespace: &str, entry_namespace: &str, name: &str) -> String {
    let _ = entry_namespace;
    format!("{}__{}", namespace.replace('.', "__"), name)
}

fn mangle_project_function_symbol(namespace: &str, entry_namespace: &str, name: &str) -> String {
    if name == "main" && namespace == entry_namespace {
        "main".to_string()
    } else {
        mangle_project_symbol(namespace, entry_namespace, name)
    }
}

fn rewrite_type_for_project_with_ctx(ty: &ast::Type, ctx: &RewriteTypeContext<'_>) -> ast::Type {
    let started_at = Instant::now();
    REWRITE_INTERNAL_TIMING_TOTALS
        .type_rewrite_calls
        .fetch_add(1, Ordering::Relaxed);
    fn rewrite_named_type_name_for_project(name: &str, ctx: &RewriteTypeContext<'_>) -> String {
        if ctx.local_classes.contains(name) {
            return mangle_project_symbol(ctx.current_namespace, ctx.entry_namespace, name);
        }
        if let Some((ns, symbol_name)) = ctx.imported_classes.get(name) {
            return mangle_project_symbol(ns, ctx.entry_namespace, symbol_name);
        }
        if let Some(ns) = ctx.global_class_map.get(name) {
            return mangle_project_symbol(ns, ctx.entry_namespace, name);
        }
        if ctx.local_interfaces.contains(name) {
            return mangle_project_symbol(ctx.current_namespace, ctx.entry_namespace, name);
        }
        if let Some((ns, symbol_name)) = ctx.imported_interfaces.get(name) {
            return mangle_project_symbol(ns, ctx.entry_namespace, symbol_name);
        }
        if let Some(ns) = ctx.global_interface_map.get(name) {
            return mangle_project_symbol(ns, ctx.entry_namespace, name);
        }
        if let Some((ns, symbol_name)) = ctx.imported_modules.get(name) {
            if let Some((owner_ns, interface_name)) =
                resolve_exact_imported_symbol_path(ns, symbol_name, ctx.global_interface_map)
            {
                return mangle_project_symbol(&owner_ns, ctx.entry_namespace, &interface_name);
            }
        }
        if ctx.local_enums.contains(name) {
            return mangle_project_symbol(ctx.current_namespace, ctx.entry_namespace, name);
        }
        if let Some((ns, symbol_name)) = ctx.imported_enums.get(name) {
            return mangle_project_symbol(ns, ctx.entry_namespace, symbol_name);
        }
        if let Some(ns) = ctx.global_enum_map.get(name) {
            return mangle_project_symbol(ns, ctx.entry_namespace, name);
        }

        let Some((alias, rest)) = name.split_once('.') else {
            return name.to_string();
        };
        let member_parts = rest
            .split('.')
            .map(|part| part.to_string())
            .collect::<Vec<_>>();

        if let Some((owner_ns, class_name)) = resolve_module_alias_class_candidate(
            ctx.current_namespace,
            alias,
            &member_parts,
            ctx.global_class_map,
        ) {
            return mangle_project_symbol(&owner_ns, ctx.entry_namespace, &class_name);
        }

        if let Some((owner_ns, interface_name)) = resolve_module_alias_class_candidate(
            ctx.current_namespace,
            alias,
            &member_parts,
            ctx.global_interface_map,
        ) {
            return mangle_project_symbol(&owner_ns, ctx.entry_namespace, &interface_name);
        }
        if let Some((ns, symbol_name)) = ctx.imported_modules.get(alias) {
            if let Some((owner_ns, interface_name)) = resolve_module_alias_class_candidate(
                ns,
                symbol_name,
                &member_parts,
                ctx.global_interface_map,
            ) {
                return mangle_project_symbol(&owner_ns, ctx.entry_namespace, &interface_name);
            }
        }

        if let Some((owner_ns, enum_name)) = resolve_module_alias_enum_type_candidate(
            ctx.current_namespace,
            alias,
            &member_parts,
            ctx.global_enum_map,
        ) {
            return mangle_project_symbol(&owner_ns, ctx.entry_namespace, &enum_name);
        }

        let Some((ns, symbol_name)) = ctx.imported_modules.get(alias) else {
            return name.to_string();
        };

        if let Some((owner_ns, class_name)) = resolve_module_alias_class_candidate(
            ns,
            symbol_name,
            &member_parts,
            ctx.global_class_map,
        ) {
            return mangle_project_symbol(&owner_ns, ctx.entry_namespace, &class_name);
        }

        if let Some((owner_ns, interface_name)) = resolve_module_alias_class_candidate(
            ns,
            symbol_name,
            &member_parts,
            ctx.global_interface_map,
        ) {
            return mangle_project_symbol(&owner_ns, ctx.entry_namespace, &interface_name);
        }

        if let Some((owner_ns, enum_name)) = resolve_module_alias_enum_type_candidate(
            ns,
            symbol_name,
            &member_parts,
            ctx.global_enum_map,
        ) {
            return mangle_project_symbol(&owner_ns, ctx.entry_namespace, &enum_name);
        }

        name.to_string()
    }

    let rewritten = match ty {
        ast::Type::Named(name) => ast::Type::Named(rewrite_named_type_name_for_project(name, ctx)),
        ast::Type::Generic(name, args) => ast::Type::Generic(
            rewrite_named_type_name_for_project(name, ctx),
            args.iter()
                .map(|a| rewrite_type_for_project_with_ctx(a, ctx))
                .collect(),
        ),
        ast::Type::Function(params, ret) => ast::Type::Function(
            params
                .iter()
                .map(|p| rewrite_type_for_project_with_ctx(p, ctx))
                .collect(),
            Box::new(rewrite_type_for_project_with_ctx(ret, ctx)),
        ),
        ast::Type::Option(inner) => {
            let rewritten_inner = rewrite_type_for_project_with_ctx(inner, ctx);
            let rewritten_name = rewrite_named_type_name_for_project("Option", ctx);
            if rewritten_name != "Option" {
                ast::Type::Generic(rewritten_name, vec![rewritten_inner])
            } else {
                ast::Type::Option(Box::new(rewritten_inner))
            }
        }
        ast::Type::Result(ok, err) => {
            let rewritten_ok = rewrite_type_for_project_with_ctx(ok, ctx);
            let rewritten_err = rewrite_type_for_project_with_ctx(err, ctx);
            let rewritten_name = rewrite_named_type_name_for_project("Result", ctx);
            if rewritten_name != "Result" {
                ast::Type::Generic(rewritten_name, vec![rewritten_ok, rewritten_err])
            } else {
                ast::Type::Result(Box::new(rewritten_ok), Box::new(rewritten_err))
            }
        }
        ast::Type::List(inner) => {
            let rewritten_inner = rewrite_type_for_project_with_ctx(inner, ctx);
            let rewritten_name = rewrite_named_type_name_for_project("List", ctx);
            if rewritten_name != "List" {
                ast::Type::Generic(rewritten_name, vec![rewritten_inner])
            } else {
                ast::Type::List(Box::new(rewritten_inner))
            }
        }
        ast::Type::Map(k, v) => {
            let rewritten_k = rewrite_type_for_project_with_ctx(k, ctx);
            let rewritten_v = rewrite_type_for_project_with_ctx(v, ctx);
            let rewritten_name = rewrite_named_type_name_for_project("Map", ctx);
            if rewritten_name != "Map" {
                ast::Type::Generic(rewritten_name, vec![rewritten_k, rewritten_v])
            } else {
                ast::Type::Map(Box::new(rewritten_k), Box::new(rewritten_v))
            }
        }
        ast::Type::Set(inner) => {
            let rewritten_inner = rewrite_type_for_project_with_ctx(inner, ctx);
            let rewritten_name = rewrite_named_type_name_for_project("Set", ctx);
            if rewritten_name != "Set" {
                ast::Type::Generic(rewritten_name, vec![rewritten_inner])
            } else {
                ast::Type::Set(Box::new(rewritten_inner))
            }
        }
        ast::Type::Ref(inner) => {
            ast::Type::Ref(Box::new(rewrite_type_for_project_with_ctx(inner, ctx)))
        }
        ast::Type::MutRef(inner) => {
            ast::Type::MutRef(Box::new(rewrite_type_for_project_with_ctx(inner, ctx)))
        }
        ast::Type::Box(inner) => {
            let rewritten_inner = rewrite_type_for_project_with_ctx(inner, ctx);
            let rewritten_name = rewrite_named_type_name_for_project("Box", ctx);
            if rewritten_name != "Box" {
                ast::Type::Generic(rewritten_name, vec![rewritten_inner])
            } else {
                ast::Type::Box(Box::new(rewritten_inner))
            }
        }
        ast::Type::Rc(inner) => {
            let rewritten_inner = rewrite_type_for_project_with_ctx(inner, ctx);
            let rewritten_name = rewrite_named_type_name_for_project("Rc", ctx);
            if rewritten_name != "Rc" {
                ast::Type::Generic(rewritten_name, vec![rewritten_inner])
            } else {
                ast::Type::Rc(Box::new(rewritten_inner))
            }
        }
        ast::Type::Arc(inner) => {
            let rewritten_inner = rewrite_type_for_project_with_ctx(inner, ctx);
            let rewritten_name = rewrite_named_type_name_for_project("Arc", ctx);
            if rewritten_name != "Arc" {
                ast::Type::Generic(rewritten_name, vec![rewritten_inner])
            } else {
                ast::Type::Arc(Box::new(rewritten_inner))
            }
        }
        ast::Type::Ptr(inner) => {
            let rewritten_inner = rewrite_type_for_project_with_ctx(inner, ctx);
            let rewritten_name = rewrite_named_type_name_for_project("Ptr", ctx);
            if rewritten_name != "Ptr" {
                ast::Type::Generic(rewritten_name, vec![rewritten_inner])
            } else {
                ast::Type::Ptr(Box::new(rewritten_inner))
            }
        }
        ast::Type::Task(inner) => {
            let rewritten_inner = rewrite_type_for_project_with_ctx(inner, ctx);
            let rewritten_name = rewrite_named_type_name_for_project("Task", ctx);
            if rewritten_name != "Task" {
                ast::Type::Generic(rewritten_name, vec![rewritten_inner])
            } else {
                ast::Type::Task(Box::new(rewritten_inner))
            }
        }
        ast::Type::Range(inner) => {
            let rewritten_inner = rewrite_type_for_project_with_ctx(inner, ctx);
            let rewritten_name = rewrite_named_type_name_for_project("Range", ctx);
            if rewritten_name != "Range" {
                ast::Type::Generic(rewritten_name, vec![rewritten_inner])
            } else {
                ast::Type::Range(Box::new(rewritten_inner))
            }
        }
        _ => ty.clone(),
    };
    REWRITE_INTERNAL_TIMING_TOTALS
        .type_rewrite_ns
        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
    rewritten
}

#[allow(clippy::too_many_arguments)]
fn rewrite_type_for_project_with_interfaces(
    ty: &ast::Type,
    current_namespace: &str,
    local_classes: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    local_interfaces: &HashSet<String>,
    imported_interfaces: &ImportedMap,
    global_interface_map: &HashMap<String, String>,
    local_enums: &HashSet<String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &ImportedMap,
    entry_namespace: &str,
) -> ast::Type {
    let ctx = RewriteTypeContext {
        current_namespace,
        local_classes,
        imported_classes,
        global_class_map,
        local_interfaces,
        imported_interfaces,
        global_interface_map,
        local_enums,
        imported_enums,
        global_enum_map,
        imported_modules,
        entry_namespace,
    };
    rewrite_type_for_project_with_ctx(ty, &ctx)
}

#[allow(clippy::too_many_arguments)]
fn rewrite_named_reference_for_project_with_interfaces(
    name: &str,
    current_namespace: &str,
    local_classes: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    local_interfaces: &HashSet<String>,
    imported_interfaces: &ImportedMap,
    global_interface_map: &HashMap<String, String>,
    local_enums: &HashSet<String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &ImportedMap,
    entry_namespace: &str,
) -> String {
    match rewrite_type_for_project_with_interfaces(
        &ast::Type::Named(name.to_string()),
        current_namespace,
        local_classes,
        imported_classes,
        global_class_map,
        local_interfaces,
        imported_interfaces,
        global_interface_map,
        local_enums,
        imported_enums,
        global_enum_map,
        imported_modules,
        entry_namespace,
    ) {
        ast::Type::Named(rewritten) => rewritten,
        _ => name.to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn rewrite_named_reference_for_project(
    name: &str,
    current_namespace: &str,
    local_classes: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    local_enums: &HashSet<String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &ImportedMap,
    entry_namespace: &str,
) -> String {
    rewrite_named_reference_for_project_with_interfaces(
        name,
        current_namespace,
        local_classes,
        imported_classes,
        global_class_map,
        &HashSet::new(),
        &HashMap::new(),
        &HashMap::new(),
        local_enums,
        imported_enums,
        global_enum_map,
        imported_modules,
        entry_namespace,
    )
}

#[allow(clippy::too_many_arguments)]
fn rewrite_interface_reference_for_project(
    name: &str,
    current_namespace: &str,
    local_classes: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    local_interfaces: &HashSet<String>,
    imported_interfaces: &ImportedMap,
    local_enums: &HashSet<String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &ImportedMap,
    global_interface_map: &HashMap<String, String>,
    entry_namespace: &str,
) -> String {
    if let Ok(ast::Type::Generic(base, args)) = parse_type_source(name) {
        let rewritten_base = rewrite_interface_reference_for_project(
            &base,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            local_interfaces,
            imported_interfaces,
            local_enums,
            imported_enums,
            global_enum_map,
            imported_modules,
            global_interface_map,
            entry_namespace,
        );
        let rewritten_args = args
            .iter()
            .map(|arg| {
                format_type_string(&rewrite_type_for_project_with_interfaces(
                    arg,
                    current_namespace,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    local_enums,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    entry_namespace,
                ))
            })
            .collect::<Vec<_>>()
            .join(", ");
        return format!("{}<{}>", rewritten_base, rewritten_args);
    }

    if local_interfaces.contains(name) {
        return mangle_project_symbol(current_namespace, entry_namespace, name);
    }
    if let Some((ns, symbol_name)) = imported_interfaces.get(name) {
        return mangle_project_symbol(ns, entry_namespace, symbol_name);
    }
    if let Some(ns) = global_interface_map.get(name) {
        return mangle_project_symbol(ns, entry_namespace, name);
    }

    let Some((alias, rest)) = name.split_once('.') else {
        return name.to_string();
    };
    let member_parts = rest
        .split('.')
        .map(|part| part.to_string())
        .collect::<Vec<_>>();

    if let Some((owner_ns, interface_name)) = resolve_module_alias_class_candidate(
        current_namespace,
        alias,
        &member_parts,
        global_interface_map,
    ) {
        return mangle_project_symbol(&owner_ns, entry_namespace, &interface_name);
    }

    if let Some((ns, symbol_name)) = imported_interfaces.get(alias) {
        if let Some((owner_ns, interface_name)) = resolve_module_alias_class_candidate(
            ns,
            symbol_name,
            &member_parts,
            global_interface_map,
        ) {
            return mangle_project_symbol(&owner_ns, entry_namespace, &interface_name);
        }
    }

    if let Some((ns, symbol_name)) = imported_modules.get(alias) {
        if let Some((owner_ns, interface_name)) = resolve_module_alias_class_candidate(
            ns,
            symbol_name,
            &member_parts,
            global_interface_map,
        ) {
            return mangle_project_symbol(&owner_ns, entry_namespace, &interface_name);
        }
    }

    name.to_string()
}

#[allow(clippy::too_many_arguments)]
fn rewrite_interface_reference_for_module(
    name: &str,
    module_prefix: &str,
    current_namespace: &str,
    entry_namespace: &str,
    local_classes: &HashSet<String>,
    local_interfaces: &HashSet<String>,
    local_enums: &HashSet<String>,
    local_modules: &HashSet<String>,
    imported_classes: &ImportedMap,
    imported_interfaces: &ImportedMap,
    imported_enums: &ImportedMap,
    imported_modules: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    global_interface_map: &HashMap<String, String>,
    global_enum_map: &HashMap<String, String>,
) -> String {
    if let Ok(ast::Type::Generic(base, args)) = parse_type_source(name) {
        let rewritten_base = rewrite_interface_reference_for_module(
            &base,
            module_prefix,
            current_namespace,
            entry_namespace,
            local_classes,
            local_interfaces,
            local_enums,
            local_modules,
            imported_classes,
            imported_interfaces,
            imported_enums,
            imported_modules,
            global_class_map,
            global_interface_map,
            global_enum_map,
        );
        let rewritten_args = args
            .iter()
            .map(|arg| {
                format_type_string(&rewrite_module_local_type(
                    arg,
                    module_prefix,
                    current_namespace,
                    entry_namespace,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ))
            })
            .collect::<Vec<_>>()
            .join(", ");
        return format!("{}<{}>", rewritten_base, rewritten_args);
    }

    if local_interfaces.contains(name) {
        return mangle_project_symbol(
            current_namespace,
            entry_namespace,
            &module_prefixed_symbol(module_prefix, name),
        );
    }

    if let Some((head, _tail)) = name.split_once('.') {
        if local_modules.contains(head) {
            return mangle_project_symbol(
                current_namespace,
                entry_namespace,
                &module_prefixed_symbol(module_prefix, &name.replace('.', "__")),
            );
        }
    }

    rewrite_interface_reference_for_project(
        name,
        current_namespace,
        local_classes,
        imported_classes,
        global_class_map,
        local_interfaces,
        imported_interfaces,
        local_enums,
        imported_enums,
        global_enum_map,
        imported_modules,
        global_interface_map,
        entry_namespace,
    )
}

fn is_shadowed(name: &str, scopes: &[HashSet<String>]) -> bool {
    scopes.iter().rev().any(|scope| scope.contains(name))
}

fn flatten_field_chain(expr: &Expr) -> Option<Vec<String>> {
    match expr {
        Expr::Ident(name) => Some(vec![name.clone()]),
        Expr::Field { object, field } => {
            let mut parts = flatten_field_chain(&object.node)?;
            parts.push(field.clone());
            Some(parts)
        }
        _ => None,
    }
}

fn resolve_module_alias_function_candidate(
    import_ns: &str,
    symbol_name: &str,
    member_parts: &[String],
    global_function_map: &HashMap<String, String>,
) -> Option<(String, String)> {
    if member_parts.is_empty() {
        return None;
    }

    let mut owner_namespaces: HashSet<&str> = HashSet::new();
    for owner in global_function_map.values() {
        owner_namespaces.insert(owner.as_str());
    }

    for owner_ns in owner_namespaces {
        let module_path = if import_ns == owner_ns {
            String::new()
        } else if let Some(suffix) = import_ns.strip_prefix(owner_ns) {
            if let Some(rest) = suffix.strip_prefix('.') {
                rest.replace('.', "__")
            } else {
                continue;
            }
        } else {
            continue;
        };

        let full_parts = if symbol_name.is_empty() {
            member_parts.to_vec()
        } else {
            let mut parts = vec![symbol_name.to_string()];
            parts.extend_from_slice(member_parts);
            parts
        };
        let joined = full_parts.join("__");
        let candidate = if module_path.is_empty() {
            joined
        } else {
            format!("{}__{}", module_path, joined)
        };
        if global_function_map
            .get(&candidate)
            .is_some_and(|owner| owner == owner_ns)
        {
            return Some((owner_ns.to_string(), candidate));
        }
    }
    None
}

fn resolve_module_alias_class_candidate(
    import_ns: &str,
    symbol_name: &str,
    member_parts: &[String],
    global_class_map: &HashMap<String, String>,
) -> Option<(String, String)> {
    if member_parts.is_empty() {
        return None;
    }

    let mut owner_namespaces: HashSet<&str> = HashSet::new();
    owner_namespaces.extend(global_class_map.values().map(String::as_str));

    for owner_ns in owner_namespaces {
        let module_path = if import_ns == owner_ns {
            String::new()
        } else if let Some(suffix) = import_ns.strip_prefix(owner_ns) {
            if let Some(rest) = suffix.strip_prefix('.') {
                rest.replace('.', "__")
            } else {
                continue;
            }
        } else {
            continue;
        };

        let full_parts = if symbol_name.is_empty() {
            member_parts.to_vec()
        } else {
            let mut parts = vec![symbol_name.to_string()];
            parts.extend_from_slice(member_parts);
            parts
        };
        let joined = full_parts.join("__");
        let candidate = if module_path.is_empty() {
            joined
        } else {
            format!("{}__{}", module_path, joined)
        };
        if global_class_map
            .get(&candidate)
            .is_some_and(|owner| owner == owner_ns)
        {
            return Some((owner_ns.to_string(), candidate));
        }
    }

    None
}

fn resolve_module_alias_enum_candidate(
    import_ns: &str,
    symbol_name: &str,
    member_parts: &[String],
    global_enum_map: &HashMap<String, String>,
) -> Option<(String, String, String)> {
    if member_parts.len() < 2 {
        return None;
    }

    let (enum_parts, variant_name) = member_parts.split_at(member_parts.len() - 1);
    resolve_module_alias_enum_type_candidate(import_ns, symbol_name, enum_parts, global_enum_map)
        .map(|(owner_ns, enum_name)| (owner_ns, enum_name, variant_name[0].clone()))
}

fn resolve_module_alias_enum_type_candidate(
    import_ns: &str,
    symbol_name: &str,
    member_parts: &[String],
    global_enum_map: &HashMap<String, String>,
) -> Option<(String, String)> {
    if member_parts.is_empty() {
        return None;
    }

    let mut owner_namespaces: HashSet<&str> = HashSet::new();
    owner_namespaces.extend(global_enum_map.values().map(String::as_str));

    for owner_ns in owner_namespaces {
        let module_path = if import_ns == owner_ns {
            String::new()
        } else if let Some(suffix) = import_ns.strip_prefix(owner_ns) {
            if let Some(rest) = suffix.strip_prefix('.') {
                rest.replace('.', "__")
            } else {
                continue;
            }
        } else {
            continue;
        };

        let joined = if symbol_name.is_empty() {
            member_parts.join("__")
        } else {
            format!("{}__{}", symbol_name, member_parts.join("__"))
        };
        let candidate = if module_path.is_empty() {
            joined
        } else {
            format!("{}__{}", module_path, joined)
        };
        if global_enum_map
            .get(&candidate)
            .is_some_and(|owner| owner == owner_ns)
        {
            return Some((owner_ns.to_string(), candidate));
        }
    }

    None
}

fn push_scope(scopes: &mut Vec<HashSet<String>>) {
    scopes.push(HashSet::new());
}

fn pop_scope(scopes: &mut Vec<HashSet<String>>) {
    if scopes.len() > 1 {
        scopes.pop();
    }
}

fn bind_pattern_locals(pattern: &ast::Pattern, scope: &mut HashSet<String>) {
    match pattern {
        ast::Pattern::Ident(name) => {
            scope.insert(name.clone());
        }
        ast::Pattern::Variant(_, bindings) => {
            for b in bindings {
                scope.insert(b.clone());
            }
        }
        _ => {}
    }
}

fn rewrite_pattern_for_project(
    pattern: &ast::Pattern,
    current_namespace: &str,
    entry_namespace: &str,
    local_modules: &HashSet<String>,
    imported_modules: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
) -> ast::Pattern {
    let started_at = Instant::now();
    REWRITE_INTERNAL_TIMING_TOTALS
        .pattern_rewrite_calls
        .fetch_add(1, Ordering::Relaxed);
    let rewritten = match pattern {
        ast::Pattern::Variant(name, bindings) => {
            if !name.contains('.') {
                if let Some((import_ns, symbol_name)) = imported_modules.get(name) {
                    if let Some((owner_ns, enum_name, variant_name)) =
                        resolve_exact_imported_variant_alias(
                            import_ns,
                            symbol_name,
                            global_enum_map,
                        )
                    {
                        return ast::Pattern::Variant(
                            format!(
                                "{}.{}",
                                mangle_project_symbol(&owner_ns, entry_namespace, &enum_name),
                                variant_name
                            ),
                            bindings.clone(),
                        );
                    }
                }
            } else if let Some((module_alias, rest)) = name.split_once('.') {
                let member_parts = rest
                    .split('.')
                    .map(|part| part.to_string())
                    .collect::<Vec<_>>();
                if local_modules.contains(module_alias) {
                    if let Some((owner_ns, enum_name, variant_name)) =
                        resolve_module_alias_enum_candidate(
                            current_namespace,
                            module_alias,
                            &member_parts,
                            global_enum_map,
                        )
                    {
                        return ast::Pattern::Variant(
                            format!(
                                "{}.{}",
                                mangle_project_symbol(&owner_ns, entry_namespace, &enum_name),
                                variant_name
                            ),
                            bindings.clone(),
                        );
                    }
                }
                if let Some((ns, symbol_name)) = imported_modules.get(module_alias) {
                    if let Some((owner_ns, enum_name, variant_name)) =
                        resolve_module_alias_enum_candidate(
                            ns,
                            symbol_name,
                            &member_parts,
                            global_enum_map,
                        )
                    {
                        return ast::Pattern::Variant(
                            format!(
                                "{}.{}",
                                mangle_project_symbol(&owner_ns, entry_namespace, &enum_name),
                                variant_name
                            ),
                            bindings.clone(),
                        );
                    }
                }
            }
            ast::Pattern::Variant(name.clone(), bindings.clone())
        }
        _ => pattern.clone(),
    };
    REWRITE_INTERNAL_TIMING_TOTALS
        .pattern_rewrite_ns
        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
    rewritten
}

fn rewrite_pattern_for_module(
    pattern: &ast::Pattern,
    module_prefix: &str,
    current_namespace: &str,
    entry_namespace: &str,
    local_modules: &HashSet<String>,
    imported_modules: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
) -> ast::Pattern {
    match pattern {
        ast::Pattern::Variant(name, bindings) => {
            if !name.contains('.') {
                if let Some((import_ns, symbol_name)) = imported_modules.get(name) {
                    if let Some((owner_ns, enum_name, variant_name)) =
                        resolve_exact_imported_variant_alias(
                            import_ns,
                            symbol_name,
                            global_enum_map,
                        )
                    {
                        return ast::Pattern::Variant(
                            format!(
                                "{}.{}",
                                mangle_project_symbol(&owner_ns, entry_namespace, &enum_name),
                                variant_name
                            ),
                            bindings.clone(),
                        );
                    }
                }
            } else if let Some((module_alias, rest)) = name.split_once('.') {
                let member_parts = rest
                    .split('.')
                    .map(|part| part.to_string())
                    .collect::<Vec<_>>();
                if local_modules.contains(module_alias) {
                    if let Some((owner_ns, enum_name, variant_name)) =
                        resolve_module_alias_enum_candidate(
                            current_namespace,
                            &module_prefixed_symbol(module_prefix, module_alias),
                            &member_parts,
                            global_enum_map,
                        )
                    {
                        return ast::Pattern::Variant(
                            format!(
                                "{}.{}",
                                mangle_project_symbol(&owner_ns, entry_namespace, &enum_name),
                                variant_name
                            ),
                            bindings.clone(),
                        );
                    }
                }
                if let Some((ns, symbol_name)) = imported_modules.get(module_alias) {
                    if let Some((owner_ns, enum_name, variant_name)) =
                        resolve_module_alias_enum_candidate(
                            ns,
                            symbol_name,
                            &member_parts,
                            global_enum_map,
                        )
                    {
                        return ast::Pattern::Variant(
                            format!(
                                "{}.{}",
                                mangle_project_symbol(&owner_ns, entry_namespace, &enum_name),
                                variant_name
                            ),
                            bindings.clone(),
                        );
                    }
                }
            }
            ast::Pattern::Variant(name.clone(), bindings.clone())
        }
        _ => pattern.clone(),
    }
}

fn collect_local_enum_names(
    global_enum_map: &HashMap<String, String>,
    current_namespace: &str,
) -> HashSet<String> {
    global_enum_map
        .iter()
        .filter_map(|(name, owner_ns)| (owner_ns == current_namespace).then_some(name.clone()))
        .collect()
}

type DirectModuleSymbolSets = (
    HashSet<String>,
    HashSet<String>,
    HashSet<String>,
    HashSet<String>,
    HashSet<String>,
);

fn collect_direct_module_symbol_names(
    declarations: &[ast::Spanned<Decl>],
) -> DirectModuleSymbolSets {
    let mut functions = HashSet::new();
    let mut classes = HashSet::new();
    let mut interfaces = HashSet::new();
    let mut enums = HashSet::new();
    let mut modules = HashSet::new();

    for decl in declarations {
        match &decl.node {
            Decl::Function(func) => {
                functions.insert(func.name.clone());
            }
            Decl::Class(class) => {
                classes.insert(class.name.clone());
            }
            Decl::Interface(interface) => {
                interfaces.insert(interface.name.clone());
            }
            Decl::Enum(en) => {
                enums.insert(en.name.clone());
            }
            Decl::Module(module) => {
                modules.insert(module.name.clone());
            }
            Decl::Import(_) => {}
        }
    }

    (functions, classes, interfaces, enums, modules)
}

fn module_prefixed_symbol(module_prefix: &str, name: &str) -> String {
    format!("{}__{}", module_prefix, name)
}

#[allow(clippy::too_many_arguments)]
fn rewrite_nested_module_decl_for_project(
    decl: &ast::Spanned<Decl>,
    module_prefix: &str,
    current_namespace: &str,
    entry_namespace: &str,
    module_local_functions: &HashSet<String>,
    module_local_classes: &HashSet<String>,
    module_local_interfaces: &HashSet<String>,
    module_local_enums: &HashSet<String>,
    module_local_modules: &HashSet<String>,
    imported_map: &ImportedMap,
    global_function_map: &HashMap<String, String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    imported_interfaces: &ImportedMap,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &ImportedMap,
    global_interface_map: &HashMap<String, String>,
    global_module_map: &HashMap<String, String>,
) -> ast::Spanned<Decl> {
    let node = match &decl.node {
        Decl::Function(func) => {
            let mut f = func.clone();
            let mut scopes = vec![f.params.iter().map(|p| p.name.clone()).collect()];
            f.params = f
                .params
                .iter()
                .map(|p| ast::Parameter {
                    name: p.name.clone(),
                    ty: rewrite_module_local_type(
                        &p.ty,
                        module_prefix,
                        current_namespace,
                        entry_namespace,
                        module_local_classes,
                        module_local_interfaces,
                        module_local_enums,
                        module_local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    ),
                    mutable: p.mutable,
                    mode: p.mode,
                })
                .collect();
            f.return_type = rewrite_module_local_type(
                &f.return_type,
                module_prefix,
                current_namespace,
                entry_namespace,
                module_local_classes,
                module_local_interfaces,
                module_local_enums,
                module_local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            f.body = fix_module_local_block(
                &rewrite_block_calls_for_project(
                    &f.body,
                    current_namespace,
                    entry_namespace,
                    module_local_functions,
                    imported_map,
                    global_function_map,
                    module_local_classes,
                    imported_classes,
                    global_class_map,
                    module_local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    module_local_modules,
                    imported_modules,
                    global_module_map,
                    &mut scopes,
                ),
                current_namespace,
                entry_namespace,
                module_prefix,
                module_local_functions,
                module_local_classes,
                module_local_interfaces,
                module_local_enums,
                module_local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            Decl::Function(f)
        }
        Decl::Class(class) => {
            let mut c = class.clone();
            c.extends = class.extends.as_ref().map(|extends| {
                match rewrite_module_local_type(
                    &ast::Type::Named(extends.clone()),
                    module_prefix,
                    current_namespace,
                    entry_namespace,
                    module_local_classes,
                    module_local_interfaces,
                    module_local_enums,
                    module_local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ) {
                    ast::Type::Named(rewritten) => rewritten,
                    _ => extends.clone(),
                }
            });
            c.implements = class
                .implements
                .iter()
                .map(|implemented| {
                    rewrite_interface_reference_for_module(
                        implemented,
                        module_prefix,
                        current_namespace,
                        entry_namespace,
                        module_local_classes,
                        module_local_interfaces,
                        module_local_enums,
                        module_local_modules,
                        imported_classes,
                        imported_interfaces,
                        imported_enums,
                        imported_modules,
                        global_class_map,
                        global_interface_map,
                        global_enum_map,
                    )
                })
                .collect();
            c.fields = c
                .fields
                .iter()
                .map(|field| ast::Field {
                    name: field.name.clone(),
                    ty: rewrite_module_local_type(
                        &field.ty,
                        module_prefix,
                        current_namespace,
                        entry_namespace,
                        module_local_classes,
                        module_local_interfaces,
                        module_local_enums,
                        module_local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    ),
                    mutable: field.mutable,
                    visibility: field.visibility,
                })
                .collect();
            if let Some(ctor) = &class.constructor {
                let mut new_ctor = ctor.clone();
                let mut scopes: Vec<HashSet<String>> =
                    vec![new_ctor.params.iter().map(|p| p.name.clone()).collect()];
                if let Some(scope) = scopes.last_mut() {
                    scope.insert("this".to_string());
                }
                new_ctor.params = new_ctor
                    .params
                    .iter()
                    .map(|p| ast::Parameter {
                        name: p.name.clone(),
                        ty: rewrite_module_local_type(
                            &p.ty,
                            module_prefix,
                            current_namespace,
                            entry_namespace,
                            module_local_classes,
                            module_local_interfaces,
                            module_local_enums,
                            module_local_modules,
                            imported_classes,
                            global_class_map,
                            imported_enums,
                            global_enum_map,
                            imported_modules,
                            global_interface_map,
                        ),
                        mutable: p.mutable,
                        mode: p.mode,
                    })
                    .collect();
                new_ctor.body = fix_module_local_block(
                    &rewrite_block_calls_for_project(
                        &new_ctor.body,
                        current_namespace,
                        entry_namespace,
                        module_local_functions,
                        imported_map,
                        global_function_map,
                        module_local_classes,
                        imported_classes,
                        global_class_map,
                        module_local_interfaces,
                        imported_interfaces,
                        global_interface_map,
                        imported_enums,
                        global_enum_map,
                        module_local_modules,
                        imported_modules,
                        global_module_map,
                        &mut scopes,
                    ),
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    module_local_functions,
                    module_local_classes,
                    module_local_interfaces,
                    module_local_enums,
                    module_local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                );
                c.constructor = Some(new_ctor);
            }
            c.methods = class
                .methods
                .iter()
                .map(|method| {
                    let mut nm = method.clone();
                    let mut scopes: Vec<HashSet<String>> =
                        vec![nm.params.iter().map(|p| p.name.clone()).collect()];
                    if let Some(scope) = scopes.last_mut() {
                        scope.insert("this".to_string());
                    }
                    nm.params = nm
                        .params
                        .iter()
                        .map(|p| ast::Parameter {
                            name: p.name.clone(),
                            ty: rewrite_module_local_type(
                                &p.ty,
                                module_prefix,
                                current_namespace,
                                entry_namespace,
                                module_local_classes,
                                module_local_interfaces,
                                module_local_enums,
                                module_local_modules,
                                imported_classes,
                                global_class_map,
                                imported_enums,
                                global_enum_map,
                                imported_modules,
                                global_interface_map,
                            ),
                            mutable: p.mutable,
                            mode: p.mode,
                        })
                        .collect();
                    nm.return_type = rewrite_module_local_type(
                        &nm.return_type,
                        module_prefix,
                        current_namespace,
                        entry_namespace,
                        module_local_classes,
                        module_local_interfaces,
                        module_local_enums,
                        module_local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    );
                    nm.body = fix_module_local_block(
                        &rewrite_block_calls_for_project(
                            &nm.body,
                            current_namespace,
                            entry_namespace,
                            module_local_functions,
                            imported_map,
                            global_function_map,
                            module_local_classes,
                            imported_classes,
                            global_class_map,
                            module_local_interfaces,
                            imported_interfaces,
                            global_interface_map,
                            imported_enums,
                            global_enum_map,
                            module_local_modules,
                            imported_modules,
                            global_module_map,
                            &mut scopes,
                        ),
                        current_namespace,
                        entry_namespace,
                        module_prefix,
                        module_local_functions,
                        module_local_classes,
                        module_local_interfaces,
                        module_local_enums,
                        module_local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    );
                    nm
                })
                .collect();
            Decl::Class(c)
        }
        Decl::Enum(en) => {
            let mut e = en.clone();
            e.variants = e
                .variants
                .iter()
                .map(|v| ast::EnumVariant {
                    name: v.name.clone(),
                    fields: v
                        .fields
                        .iter()
                        .map(|f| ast::EnumField {
                            name: f.name.clone(),
                            ty: rewrite_module_local_type(
                                &f.ty,
                                module_prefix,
                                current_namespace,
                                entry_namespace,
                                module_local_classes,
                                module_local_interfaces,
                                module_local_enums,
                                module_local_modules,
                                imported_classes,
                                global_class_map,
                                imported_enums,
                                global_enum_map,
                                imported_modules,
                                global_interface_map,
                            ),
                        })
                        .collect(),
                })
                .collect();
            Decl::Enum(e)
        }
        Decl::Module(module) => {
            let (
                nested_local_functions,
                nested_local_classes,
                nested_local_interfaces,
                nested_local_enums,
                nested_local_modules,
            ) = collect_direct_module_symbol_names(&module.declarations);
            let mut nested = module.clone();
            nested.declarations = module
                .declarations
                .iter()
                .map(|inner| {
                    let inner_prefix = match &inner.node {
                        Decl::Module(nested_module) => {
                            module_prefixed_symbol(module_prefix, &nested_module.name)
                        }
                        _ => module_prefix.to_string(),
                    };
                    rewrite_nested_module_decl_for_project(
                        inner,
                        &inner_prefix,
                        current_namespace,
                        entry_namespace,
                        &nested_local_functions,
                        &nested_local_classes,
                        &nested_local_interfaces,
                        &nested_local_enums,
                        &nested_local_modules,
                        imported_map,
                        global_function_map,
                        imported_classes,
                        global_class_map,
                        imported_interfaces,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                        global_module_map,
                    )
                })
                .collect();
            Decl::Module(nested)
        }
        Decl::Interface(interface) => {
            let mut rewritten = interface.clone();
            rewritten.extends = interface
                .extends
                .iter()
                .map(|extended| {
                    rewrite_interface_reference_for_module(
                        extended,
                        module_prefix,
                        current_namespace,
                        entry_namespace,
                        module_local_classes,
                        module_local_interfaces,
                        module_local_enums,
                        module_local_modules,
                        imported_classes,
                        imported_interfaces,
                        imported_enums,
                        imported_modules,
                        global_class_map,
                        global_interface_map,
                        global_enum_map,
                    )
                })
                .collect();
            rewritten.methods = interface
                .methods
                .iter()
                .map(|method| {
                    let mut new_method = method.clone();
                    let mut scopes: Vec<HashSet<String>> =
                        vec![new_method.params.iter().map(|p| p.name.clone()).collect()];
                    if let Some(scope) = scopes.last_mut() {
                        scope.insert("this".to_string());
                    }
                    new_method.params = new_method
                        .params
                        .iter()
                        .map(|param| ast::Parameter {
                            name: param.name.clone(),
                            ty: rewrite_module_local_type(
                                &param.ty,
                                module_prefix,
                                current_namespace,
                                entry_namespace,
                                module_local_classes,
                                module_local_interfaces,
                                module_local_enums,
                                module_local_modules,
                                imported_classes,
                                global_class_map,
                                imported_enums,
                                global_enum_map,
                                imported_modules,
                                global_interface_map,
                            ),
                            mutable: param.mutable,
                            mode: param.mode,
                        })
                        .collect();
                    new_method.return_type = rewrite_module_local_type(
                        &new_method.return_type,
                        module_prefix,
                        current_namespace,
                        entry_namespace,
                        module_local_classes,
                        module_local_interfaces,
                        module_local_enums,
                        module_local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    );
                    new_method.default_impl = method.default_impl.as_ref().map(|block| {
                        fix_module_local_block(
                            &rewrite_block_calls_for_project(
                                block,
                                current_namespace,
                                entry_namespace,
                                module_local_functions,
                                imported_map,
                                global_function_map,
                                module_local_classes,
                                imported_classes,
                                global_class_map,
                                module_local_interfaces,
                                imported_interfaces,
                                global_interface_map,
                                imported_enums,
                                global_enum_map,
                                module_local_modules,
                                imported_modules,
                                global_module_map,
                                &mut scopes,
                            ),
                            current_namespace,
                            entry_namespace,
                            module_prefix,
                            module_local_functions,
                            module_local_classes,
                            module_local_interfaces,
                            module_local_enums,
                            module_local_modules,
                            imported_classes,
                            global_class_map,
                            imported_enums,
                            global_enum_map,
                            imported_modules,
                            global_interface_map,
                        )
                    });
                    new_method
                })
                .collect();
            Decl::Interface(rewritten)
        }
        Decl::Import(_) => decl.node.clone(),
    };

    ast::Spanned::new(node, decl.span.clone())
}

fn remap_module_local_mangled_name(
    name: &str,
    current_namespace: &str,
    entry_namespace: &str,
    module_prefix: &str,
    local_symbols: &HashSet<String>,
) -> Option<String> {
    let expected_prefix = mangle_project_symbol(current_namespace, entry_namespace, "");
    let _ = expected_prefix;
    local_symbols.iter().find_map(|symbol| {
        let unscoped = mangle_project_symbol(current_namespace, entry_namespace, symbol);
        if name == unscoped {
            return Some(mangle_project_symbol(
                current_namespace,
                entry_namespace,
                &module_prefixed_symbol(module_prefix, symbol),
            ));
        }
        name.strip_prefix(&format!("{}<", unscoped)).map(|suffix| {
            mangle_project_symbol(
                current_namespace,
                entry_namespace,
                &module_prefixed_symbol(module_prefix, symbol),
            ) + "<"
                + suffix
        })
    })
}

fn resolve_module_local_member_prefix(
    head: &str,
    current_namespace: &str,
    entry_namespace: &str,
    module_prefix: &str,
    local_modules: &HashSet<String>,
) -> Option<String> {
    if local_modules.contains(head) {
        return Some(module_prefixed_symbol(module_prefix, head));
    }

    local_modules.iter().find_map(|local_module| {
        let prefixed_module = module_prefixed_symbol(module_prefix, local_module);
        let unscoped_module =
            mangle_project_symbol(current_namespace, entry_namespace, local_module);
        if head == unscoped_module {
            return Some(prefixed_module);
        }
        let mangled_module =
            mangle_project_symbol(current_namespace, entry_namespace, &prefixed_module);
        (head == mangled_module).then_some(prefixed_module)
    })
}

#[allow(clippy::too_many_arguments)]
fn rewrite_module_local_construct_type_name(
    ty: &str,
    module_prefix: &str,
    current_namespace: &str,
    entry_namespace: &str,
    local_classes: &HashSet<String>,
    local_interfaces: &HashSet<String>,
    local_enums: &HashSet<String>,
    local_modules: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &ImportedMap,
    global_interface_map: &HashMap<String, String>,
) -> String {
    parse_type_source(ty)
        .ok()
        .map(|parsed| {
            format_type_string(&rewrite_module_local_type(
                &parsed,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            ))
        })
        .unwrap_or_else(|| {
            remap_module_local_mangled_name(
                ty,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_classes,
            )
            .unwrap_or_else(|| ty.to_string())
        })
}

#[allow(clippy::too_many_arguments)]
fn fix_module_local_expr(
    expr: &Expr,
    current_namespace: &str,
    entry_namespace: &str,
    module_prefix: &str,
    local_functions: &HashSet<String>,
    local_classes: &HashSet<String>,
    local_interfaces: &HashSet<String>,
    local_enums: &HashSet<String>,
    local_modules: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &ImportedMap,
    global_interface_map: &HashMap<String, String>,
) -> Expr {
    match expr {
        Expr::Ident(name) => remap_module_local_mangled_name(
            name,
            current_namespace,
            entry_namespace,
            module_prefix,
            local_functions,
        )
        .or_else(|| {
            remap_module_local_mangled_name(
                name,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_modules,
            )
        })
        .map_or_else(|| Expr::Ident(name.clone()), Expr::Ident),
        Expr::Call {
            callee,
            args,
            type_args,
        } => {
            let module_name =
                mangle_project_symbol(current_namespace, entry_namespace, module_prefix);
            if let Some(parts) = flatten_field_chain(&callee.node) {
                if let Some(module_alias) = parts.first() {
                    if parts.len() >= 2 {
                        if let Some(prefixed_alias) = resolve_module_local_member_prefix(
                            module_alias,
                            current_namespace,
                            entry_namespace,
                            module_prefix,
                            local_modules,
                        ) {
                            let member_parts = parts[1..].to_vec();
                            let direct_member_candidate =
                                format!("{}__{}", prefixed_alias, member_parts.join("__"));
                            if global_class_map
                                .get(&direct_member_candidate)
                                .is_some_and(|owner_ns| owner_ns == current_namespace)
                            {
                                let rewritten_type_args = type_args
                                    .iter()
                                    .map(|arg| {
                                        rewrite_module_local_type(
                                            arg,
                                            module_prefix,
                                            current_namespace,
                                            entry_namespace,
                                            local_classes,
                                            local_interfaces,
                                            local_enums,
                                            local_modules,
                                            imported_classes,
                                            global_class_map,
                                            imported_enums,
                                            global_enum_map,
                                            imported_modules,
                                            global_interface_map,
                                        )
                                    })
                                    .collect::<Vec<_>>();
                                return Expr::Construct {
                                    ty: format_construct_type_name(
                                        &mangle_project_symbol(
                                            current_namespace,
                                            entry_namespace,
                                            &direct_member_candidate,
                                        ),
                                        &rewritten_type_args,
                                    ),
                                    args: args
                                        .iter()
                                        .map(|arg| {
                                            ast::Spanned::new(
                                                fix_module_local_expr(
                                                    &arg.node,
                                                    current_namespace,
                                                    entry_namespace,
                                                    module_prefix,
                                                    local_functions,
                                                    local_classes,
                                                    local_interfaces,
                                                    local_enums,
                                                    local_modules,
                                                    imported_classes,
                                                    global_class_map,
                                                    imported_enums,
                                                    global_enum_map,
                                                    imported_modules,
                                                    global_interface_map,
                                                ),
                                                arg.span.clone(),
                                            )
                                        })
                                        .collect(),
                                };
                            }
                            if let Some((owner_ns, enum_name, variant_name)) =
                                resolve_module_alias_enum_candidate(
                                    current_namespace,
                                    &prefixed_alias,
                                    &member_parts,
                                    global_enum_map,
                                )
                            {
                                return Expr::Call {
                                    callee: Box::new(ast::Spanned::new(
                                        Expr::Field {
                                            object: Box::new(ast::Spanned::new(
                                                Expr::Ident(mangle_project_symbol(
                                                    &owner_ns,
                                                    entry_namespace,
                                                    &enum_name,
                                                )),
                                                callee.span.clone(),
                                            )),
                                            field: variant_name,
                                        },
                                        callee.span.clone(),
                                    )),
                                    args: args
                                        .iter()
                                        .map(|arg| {
                                            ast::Spanned::new(
                                                fix_module_local_expr(
                                                    &arg.node,
                                                    current_namespace,
                                                    entry_namespace,
                                                    module_prefix,
                                                    local_functions,
                                                    local_classes,
                                                    local_interfaces,
                                                    local_enums,
                                                    local_modules,
                                                    imported_classes,
                                                    global_class_map,
                                                    imported_enums,
                                                    global_enum_map,
                                                    imported_modules,
                                                    global_interface_map,
                                                ),
                                                arg.span.clone(),
                                            )
                                        })
                                        .collect(),
                                    type_args: vec![],
                                };
                            }
                            if let Some((owner_ns, class_name)) =
                                resolve_module_alias_class_candidate(
                                    current_namespace,
                                    &prefixed_alias,
                                    &member_parts,
                                    global_class_map,
                                )
                            {
                                let rewritten_type_args = type_args
                                    .iter()
                                    .map(|arg| {
                                        rewrite_module_local_type(
                                            arg,
                                            module_prefix,
                                            current_namespace,
                                            entry_namespace,
                                            local_classes,
                                            local_interfaces,
                                            local_enums,
                                            local_modules,
                                            imported_classes,
                                            global_class_map,
                                            imported_enums,
                                            global_enum_map,
                                            imported_modules,
                                            global_interface_map,
                                        )
                                    })
                                    .collect::<Vec<_>>();
                                return Expr::Construct {
                                    ty: format_construct_type_name(
                                        &mangle_project_symbol(
                                            &owner_ns,
                                            entry_namespace,
                                            &class_name,
                                        ),
                                        &rewritten_type_args,
                                    ),
                                    args: args
                                        .iter()
                                        .map(|arg| {
                                            ast::Spanned::new(
                                                fix_module_local_expr(
                                                    &arg.node,
                                                    current_namespace,
                                                    entry_namespace,
                                                    module_prefix,
                                                    local_functions,
                                                    local_classes,
                                                    local_interfaces,
                                                    local_enums,
                                                    local_modules,
                                                    imported_classes,
                                                    global_class_map,
                                                    imported_enums,
                                                    global_enum_map,
                                                    imported_modules,
                                                    global_interface_map,
                                                ),
                                                arg.span.clone(),
                                            )
                                        })
                                        .collect(),
                                };
                            }
                        }
                    }
                }
                if parts.first().is_some_and(|part| part == &module_name) && parts.len() == 2 {
                    let member = &parts[1];
                    if local_functions.contains(member) {
                        return Expr::Call {
                            callee: Box::new(ast::Spanned::new(
                                Expr::Ident(mangle_project_symbol(
                                    current_namespace,
                                    entry_namespace,
                                    &module_prefixed_symbol(module_prefix, member),
                                )),
                                callee.span.clone(),
                            )),
                            args: args
                                .iter()
                                .map(|arg| {
                                    ast::Spanned::new(
                                        fix_module_local_expr(
                                            &arg.node,
                                            current_namespace,
                                            entry_namespace,
                                            module_prefix,
                                            local_functions,
                                            local_classes,
                                            local_interfaces,
                                            local_enums,
                                            local_modules,
                                            imported_classes,
                                            global_class_map,
                                            imported_enums,
                                            global_enum_map,
                                            imported_modules,
                                            global_interface_map,
                                        ),
                                        arg.span.clone(),
                                    )
                                })
                                .collect(),
                            type_args: type_args
                                .iter()
                                .map(|arg| {
                                    rewrite_module_local_type(
                                        arg,
                                        module_prefix,
                                        current_namespace,
                                        entry_namespace,
                                        local_classes,
                                        local_interfaces,
                                        local_enums,
                                        local_modules,
                                        imported_classes,
                                        global_class_map,
                                        imported_enums,
                                        global_enum_map,
                                        imported_modules,
                                        global_interface_map,
                                    )
                                })
                                .collect(),
                        };
                    }
                    if local_classes.contains(member) {
                        let rewritten_type_args = type_args
                            .iter()
                            .map(|arg| {
                                rewrite_module_local_type(
                                    arg,
                                    module_prefix,
                                    current_namespace,
                                    entry_namespace,
                                    local_classes,
                                    local_interfaces,
                                    local_enums,
                                    local_modules,
                                    imported_classes,
                                    global_class_map,
                                    imported_enums,
                                    global_enum_map,
                                    imported_modules,
                                    global_interface_map,
                                )
                            })
                            .collect::<Vec<_>>();
                        return Expr::Construct {
                            ty: format_construct_type_name(
                                &mangle_project_symbol(
                                    current_namespace,
                                    entry_namespace,
                                    &module_prefixed_symbol(module_prefix, member),
                                ),
                                &rewritten_type_args,
                            ),
                            args: args
                                .iter()
                                .map(|arg| {
                                    ast::Spanned::new(
                                        fix_module_local_expr(
                                            &arg.node,
                                            current_namespace,
                                            entry_namespace,
                                            module_prefix,
                                            local_functions,
                                            local_classes,
                                            local_interfaces,
                                            local_enums,
                                            local_modules,
                                            imported_classes,
                                            global_class_map,
                                            imported_enums,
                                            global_enum_map,
                                            imported_modules,
                                            global_interface_map,
                                        ),
                                        arg.span.clone(),
                                    )
                                })
                                .collect(),
                        };
                    }
                }
            }
            Expr::Call {
                callee: Box::new(ast::Spanned::new(
                    fix_module_local_expr(
                        &callee.node,
                        current_namespace,
                        entry_namespace,
                        module_prefix,
                        local_functions,
                        local_classes,
                        local_interfaces,
                        local_enums,
                        local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    ),
                    callee.span.clone(),
                )),
                args: args
                    .iter()
                    .map(|arg| {
                        ast::Spanned::new(
                            fix_module_local_expr(
                                &arg.node,
                                current_namespace,
                                entry_namespace,
                                module_prefix,
                                local_functions,
                                local_classes,
                                local_interfaces,
                                local_enums,
                                local_modules,
                                imported_classes,
                                global_class_map,
                                imported_enums,
                                global_enum_map,
                                imported_modules,
                                global_interface_map,
                            ),
                            arg.span.clone(),
                        )
                    })
                    .collect(),
                type_args: type_args
                    .iter()
                    .map(|arg| {
                        rewrite_module_local_type(
                            arg,
                            module_prefix,
                            current_namespace,
                            entry_namespace,
                            local_classes,
                            local_interfaces,
                            local_enums,
                            local_modules,
                            imported_classes,
                            global_class_map,
                            imported_enums,
                            global_enum_map,
                            imported_modules,
                            global_interface_map,
                        )
                    })
                    .collect(),
            }
        }
        Expr::Construct { ty, args } => {
            let ty = rewrite_module_local_construct_type_name(
                ty,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            Expr::Construct {
                ty,
                args: args
                    .iter()
                    .map(|arg| {
                        ast::Spanned::new(
                            fix_module_local_expr(
                                &arg.node,
                                current_namespace,
                                entry_namespace,
                                module_prefix,
                                local_functions,
                                local_classes,
                                local_interfaces,
                                local_enums,
                                local_modules,
                                imported_classes,
                                global_class_map,
                                imported_enums,
                                global_enum_map,
                                imported_modules,
                                global_interface_map,
                            ),
                            arg.span.clone(),
                        )
                    })
                    .collect(),
            }
        }
        Expr::GenericFunctionValue { callee, type_args } => {
            if let Some(path_parts) = flatten_field_chain(&callee.node) {
                if let Some(module_alias) = path_parts.first() {
                    if !path_parts[1..].is_empty() {
                        if let Some(prefixed_alias) = resolve_module_local_member_prefix(
                            module_alias,
                            current_namespace,
                            entry_namespace,
                            module_prefix,
                            local_modules,
                        ) {
                            let candidate =
                                format!("{}__{}", prefixed_alias, path_parts[1..].join("__"));
                            return Expr::GenericFunctionValue {
                                callee: Box::new(ast::Spanned::new(
                                    Expr::Ident(mangle_project_function_symbol(
                                        current_namespace,
                                        entry_namespace,
                                        &candidate,
                                    )),
                                    callee.span.clone(),
                                )),
                                type_args: type_args
                                    .iter()
                                    .map(|arg| {
                                        rewrite_module_local_type(
                                            arg,
                                            module_prefix,
                                            current_namespace,
                                            entry_namespace,
                                            local_classes,
                                            local_interfaces,
                                            local_enums,
                                            local_modules,
                                            imported_classes,
                                            global_class_map,
                                            imported_enums,
                                            global_enum_map,
                                            imported_modules,
                                            global_interface_map,
                                        )
                                    })
                                    .collect(),
                            };
                        }
                    }
                }
            }
            Expr::GenericFunctionValue {
                callee: Box::new(ast::Spanned::new(
                    fix_module_local_expr(
                        &callee.node,
                        current_namespace,
                        entry_namespace,
                        module_prefix,
                        local_functions,
                        local_classes,
                        local_interfaces,
                        local_enums,
                        local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    ),
                    callee.span.clone(),
                )),
                type_args: type_args
                    .iter()
                    .map(|arg| {
                        rewrite_module_local_type(
                            arg,
                            module_prefix,
                            current_namespace,
                            entry_namespace,
                            local_classes,
                            local_interfaces,
                            local_enums,
                            local_modules,
                            imported_classes,
                            global_class_map,
                            imported_enums,
                            global_enum_map,
                            imported_modules,
                            global_interface_map,
                        )
                    })
                    .collect(),
            }
        }
        Expr::Field { object, field } => {
            if let Some(path_parts) = flatten_field_chain(expr) {
                let module_alias = &path_parts[0];
                let member_parts = &path_parts[1..];
                if !member_parts.is_empty() {
                    if let Some(prefixed_alias) = resolve_module_local_member_prefix(
                        module_alias,
                        current_namespace,
                        entry_namespace,
                        module_prefix,
                        local_modules,
                    ) {
                        if let Some((owner_ns, enum_name, variant_name)) =
                            resolve_module_alias_enum_candidate(
                                current_namespace,
                                &prefixed_alias,
                                member_parts,
                                global_enum_map,
                            )
                        {
                            return Expr::Field {
                                object: Box::new(ast::Spanned::new(
                                    Expr::Ident(mangle_project_symbol(
                                        &owner_ns,
                                        entry_namespace,
                                        &enum_name,
                                    )),
                                    object.span.clone(),
                                )),
                                field: variant_name,
                            };
                        }
                        if let Some((owner_ns, class_name)) = resolve_module_alias_class_candidate(
                            current_namespace,
                            &prefixed_alias,
                            member_parts,
                            global_class_map,
                        ) {
                            return Expr::Ident(mangle_project_symbol(
                                &owner_ns,
                                entry_namespace,
                                &class_name,
                            ));
                        }
                    }
                }
            }
            Expr::Field {
                object: Box::new(ast::Spanned::new(
                    fix_module_local_expr(
                        &object.node,
                        current_namespace,
                        entry_namespace,
                        module_prefix,
                        local_functions,
                        local_classes,
                        local_interfaces,
                        local_enums,
                        local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    ),
                    object.span.clone(),
                )),
                field: field.clone(),
            }
        }
        Expr::Binary { op, left, right } => Expr::Binary {
            op: *op,
            left: Box::new(ast::Spanned::new(
                fix_module_local_expr(
                    &left.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                left.span.clone(),
            )),
            right: Box::new(ast::Spanned::new(
                fix_module_local_expr(
                    &right.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                right.span.clone(),
            )),
        },
        Expr::Unary { op, expr } => Expr::Unary {
            op: *op,
            expr: Box::new(ast::Spanned::new(
                fix_module_local_expr(
                    &expr.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                expr.span.clone(),
            )),
        },
        Expr::IfExpr {
            condition,
            then_branch,
            else_branch,
        } => Expr::IfExpr {
            condition: Box::new(ast::Spanned::new(
                fix_module_local_expr(
                    &condition.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                condition.span.clone(),
            )),
            then_branch: fix_module_local_block(
                then_branch,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_functions,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            ),
            else_branch: else_branch.as_ref().map(|block| {
                fix_module_local_block(
                    block,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                )
            }),
        },
        Expr::Block(block) => Expr::Block(fix_module_local_block(
            block,
            current_namespace,
            entry_namespace,
            module_prefix,
            local_functions,
            local_classes,
            local_interfaces,
            local_enums,
            local_modules,
            imported_classes,
            global_class_map,
            imported_enums,
            global_enum_map,
            imported_modules,
            global_interface_map,
        )),
        Expr::AsyncBlock(body) => Expr::AsyncBlock(fix_module_local_block(
            body,
            current_namespace,
            entry_namespace,
            module_prefix,
            local_functions,
            local_classes,
            local_interfaces,
            local_enums,
            local_modules,
            imported_classes,
            global_class_map,
            imported_enums,
            global_enum_map,
            imported_modules,
            global_interface_map,
        )),
        Expr::Match { expr, arms } => Expr::Match {
            expr: Box::new(ast::Spanned::new(
                fix_module_local_expr(
                    &expr.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                expr.span.clone(),
            )),
            arms: arms
                .iter()
                .map(|arm| ast::MatchArm {
                    pattern: rewrite_pattern_for_module(
                        &arm.pattern,
                        module_prefix,
                        current_namespace,
                        entry_namespace,
                        local_modules,
                        imported_modules,
                        global_enum_map,
                    ),
                    body: fix_module_local_block(
                        &arm.body,
                        current_namespace,
                        entry_namespace,
                        module_prefix,
                        local_functions,
                        local_classes,
                        local_interfaces,
                        local_enums,
                        local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    ),
                })
                .collect(),
        },
        Expr::Index { object, index } => Expr::Index {
            object: Box::new(ast::Spanned::new(
                fix_module_local_expr(
                    &object.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                object.span.clone(),
            )),
            index: Box::new(ast::Spanned::new(
                fix_module_local_expr(
                    &index.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                index.span.clone(),
            )),
        },
        Expr::Try(inner) => Expr::Try(Box::new(ast::Spanned::new(
            fix_module_local_expr(
                &inner.node,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_functions,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            ),
            inner.span.clone(),
        ))),
        Expr::Borrow(inner) => Expr::Borrow(Box::new(ast::Spanned::new(
            fix_module_local_expr(
                &inner.node,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_functions,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            ),
            inner.span.clone(),
        ))),
        Expr::MutBorrow(inner) => Expr::MutBorrow(Box::new(ast::Spanned::new(
            fix_module_local_expr(
                &inner.node,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_functions,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            ),
            inner.span.clone(),
        ))),
        Expr::Deref(inner) => Expr::Deref(Box::new(ast::Spanned::new(
            fix_module_local_expr(
                &inner.node,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_functions,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            ),
            inner.span.clone(),
        ))),
        Expr::Await(inner) => Expr::Await(Box::new(ast::Spanned::new(
            fix_module_local_expr(
                &inner.node,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_functions,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            ),
            inner.span.clone(),
        ))),
        Expr::Lambda { params, body } => Expr::Lambda {
            params: params
                .iter()
                .map(|param| ast::Parameter {
                    name: param.name.clone(),
                    ty: rewrite_module_local_type(
                        &param.ty,
                        module_prefix,
                        current_namespace,
                        entry_namespace,
                        local_classes,
                        local_interfaces,
                        local_enums,
                        local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    ),
                    mutable: param.mutable,
                    mode: param.mode,
                })
                .collect(),
            body: Box::new(ast::Spanned::new(
                fix_module_local_expr(
                    &body.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                body.span.clone(),
            )),
        },
        Expr::Require { condition, message } => Expr::Require {
            condition: Box::new(ast::Spanned::new(
                fix_module_local_expr(
                    &condition.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                condition.span.clone(),
            )),
            message: message.as_ref().map(|msg| {
                Box::new(ast::Spanned::new(
                    fix_module_local_expr(
                        &msg.node,
                        current_namespace,
                        entry_namespace,
                        module_prefix,
                        local_functions,
                        local_classes,
                        local_interfaces,
                        local_enums,
                        local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    ),
                    msg.span.clone(),
                ))
            }),
        },
        Expr::Range {
            start,
            end,
            inclusive,
        } => Expr::Range {
            start: start.as_ref().map(|expr| {
                Box::new(ast::Spanned::new(
                    fix_module_local_expr(
                        &expr.node,
                        current_namespace,
                        entry_namespace,
                        module_prefix,
                        local_functions,
                        local_classes,
                        local_interfaces,
                        local_enums,
                        local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    ),
                    expr.span.clone(),
                ))
            }),
            end: end.as_ref().map(|expr| {
                Box::new(ast::Spanned::new(
                    fix_module_local_expr(
                        &expr.node,
                        current_namespace,
                        entry_namespace,
                        module_prefix,
                        local_functions,
                        local_classes,
                        local_interfaces,
                        local_enums,
                        local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    ),
                    expr.span.clone(),
                ))
            }),
            inclusive: *inclusive,
        },
        Expr::StringInterp(parts) => Expr::StringInterp(
            parts
                .iter()
                .map(|part| match part {
                    ast::StringPart::Literal(text) => ast::StringPart::Literal(text.clone()),
                    ast::StringPart::Expr(expr) => ast::StringPart::Expr(ast::Spanned::new(
                        fix_module_local_expr(
                            &expr.node,
                            current_namespace,
                            entry_namespace,
                            module_prefix,
                            local_functions,
                            local_classes,
                            local_interfaces,
                            local_enums,
                            local_modules,
                            imported_classes,
                            global_class_map,
                            imported_enums,
                            global_enum_map,
                            imported_modules,
                            global_interface_map,
                        ),
                        expr.span.clone(),
                    )),
                })
                .collect(),
        ),
        _ => expr.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn fix_module_local_stmt(
    stmt: &Stmt,
    current_namespace: &str,
    entry_namespace: &str,
    module_prefix: &str,
    local_functions: &HashSet<String>,
    local_classes: &HashSet<String>,
    local_interfaces: &HashSet<String>,
    local_enums: &HashSet<String>,
    local_modules: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &ImportedMap,
    global_interface_map: &HashMap<String, String>,
) -> Stmt {
    match stmt {
        Stmt::Let {
            name,
            ty,
            value,
            mutable,
        } => Stmt::Let {
            name: name.clone(),
            ty: rewrite_module_local_type(
                ty,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            ),
            value: ast::Spanned::new(
                fix_module_local_expr(
                    &value.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                value.span.clone(),
            ),
            mutable: *mutable,
        },
        Stmt::Assign { target, value } => Stmt::Assign {
            target: ast::Spanned::new(
                fix_module_local_expr(
                    &target.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                target.span.clone(),
            ),
            value: ast::Spanned::new(
                fix_module_local_expr(
                    &value.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                value.span.clone(),
            ),
        },
        Stmt::Expr(expr) => Stmt::Expr(ast::Spanned::new(
            fix_module_local_expr(
                &expr.node,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_functions,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            ),
            expr.span.clone(),
        )),
        Stmt::Return(expr) => Stmt::Return(expr.as_ref().map(|expr| {
            ast::Spanned::new(
                fix_module_local_expr(
                    &expr.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                expr.span.clone(),
            )
        })),
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => Stmt::If {
            condition: ast::Spanned::new(
                fix_module_local_expr(
                    &condition.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                condition.span.clone(),
            ),
            then_block: fix_module_local_block(
                then_block,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_functions,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            ),
            else_block: else_block.as_ref().map(|block| {
                fix_module_local_block(
                    block,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                )
            }),
        },
        Stmt::While { condition, body } => Stmt::While {
            condition: ast::Spanned::new(
                fix_module_local_expr(
                    &condition.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                condition.span.clone(),
            ),
            body: fix_module_local_block(
                body,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_functions,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            ),
        },
        Stmt::For {
            var,
            var_type,
            iterable,
            body,
        } => Stmt::For {
            var: var.clone(),
            var_type: var_type.as_ref().map(|ty| {
                rewrite_module_local_type(
                    ty,
                    module_prefix,
                    current_namespace,
                    entry_namespace,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                )
            }),
            iterable: ast::Spanned::new(
                fix_module_local_expr(
                    &iterable.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                iterable.span.clone(),
            ),
            body: fix_module_local_block(
                body,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_functions,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            ),
        },
        Stmt::Match { expr, arms } => Stmt::Match {
            expr: ast::Spanned::new(
                fix_module_local_expr(
                    &expr.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                expr.span.clone(),
            ),
            arms: arms
                .iter()
                .map(|arm| ast::MatchArm {
                    pattern: rewrite_pattern_for_module(
                        &arm.pattern,
                        module_prefix,
                        current_namespace,
                        entry_namespace,
                        local_modules,
                        imported_modules,
                        global_enum_map,
                    ),
                    body: fix_module_local_block(
                        &arm.body,
                        current_namespace,
                        entry_namespace,
                        module_prefix,
                        local_functions,
                        local_classes,
                        local_interfaces,
                        local_enums,
                        local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    ),
                })
                .collect(),
        },
        _ => stmt.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn fix_module_local_block(
    block: &ast::Block,
    current_namespace: &str,
    entry_namespace: &str,
    module_prefix: &str,
    local_functions: &HashSet<String>,
    local_classes: &HashSet<String>,
    local_interfaces: &HashSet<String>,
    local_enums: &HashSet<String>,
    local_modules: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &ImportedMap,
    global_interface_map: &HashMap<String, String>,
) -> ast::Block {
    block
        .iter()
        .map(|stmt| {
            ast::Spanned::new(
                fix_module_local_stmt(
                    &stmt.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                    local_interfaces,
                    local_enums,
                    local_modules,
                    imported_classes,
                    global_class_map,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    global_interface_map,
                ),
                stmt.span.clone(),
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn rewrite_module_local_type(
    ty: &ast::Type,
    module_prefix: &str,
    current_namespace: &str,
    entry_namespace: &str,
    local_classes: &HashSet<String>,
    local_interfaces: &HashSet<String>,
    local_enums: &HashSet<String>,
    local_modules: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &ImportedMap,
    global_interface_map: &HashMap<String, String>,
) -> ast::Type {
    let rewrite_shadowed_builtin =
        |builtin_name: &str, args: Vec<ast::Type>| -> Option<ast::Type> {
            local_classes.contains(builtin_name).then(|| {
                ast::Type::Generic(
                    mangle_project_symbol(
                        current_namespace,
                        entry_namespace,
                        &module_prefixed_symbol(module_prefix, builtin_name),
                    ),
                    args,
                )
            })
        };

    match ty {
        ast::Type::Named(name) => {
            if local_classes.contains(name)
                || local_interfaces.contains(name)
                || local_enums.contains(name)
            {
                ast::Type::Named(mangle_project_symbol(
                    current_namespace,
                    entry_namespace,
                    &module_prefixed_symbol(module_prefix, name),
                ))
            } else if let Some(remapped) = remap_module_local_mangled_name(
                name,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_classes,
            )
            .or_else(|| {
                remap_module_local_mangled_name(
                    name,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_interfaces,
                )
            })
            .or_else(|| {
                remap_module_local_mangled_name(
                    name,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_enums,
                )
            }) {
                ast::Type::Named(remapped)
            } else if let Some((head, _tail)) = name.split_once('.') {
                if local_modules.contains(head) {
                    ast::Type::Named(mangle_project_symbol(
                        current_namespace,
                        entry_namespace,
                        &module_prefixed_symbol(module_prefix, &name.replace('.', "__")),
                    ))
                } else {
                    rewrite_type_for_project_with_interfaces(
                        ty,
                        current_namespace,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        local_interfaces,
                        &HashMap::new(),
                        global_interface_map,
                        local_enums,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        entry_namespace,
                    )
                }
            } else {
                rewrite_type_for_project_with_interfaces(
                    ty,
                    current_namespace,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    &HashMap::new(),
                    global_interface_map,
                    local_enums,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    entry_namespace,
                )
            }
        }
        ast::Type::Generic(name, args) => ast::Type::Generic(
            match rewrite_module_local_type(
                &ast::Type::Named(name.clone()),
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            ) {
                ast::Type::Named(name) => name,
                other => format_type_string(&other),
            },
            args.iter()
                .map(|arg| {
                    rewrite_module_local_type(
                        arg,
                        module_prefix,
                        current_namespace,
                        entry_namespace,
                        local_classes,
                        local_interfaces,
                        local_enums,
                        local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    )
                })
                .collect(),
        ),
        ast::Type::Function(params, ret) => ast::Type::Function(
            params
                .iter()
                .map(|param| {
                    rewrite_module_local_type(
                        param,
                        module_prefix,
                        current_namespace,
                        entry_namespace,
                        local_classes,
                        local_interfaces,
                        local_enums,
                        local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_interface_map,
                    )
                })
                .collect(),
            Box::new(rewrite_module_local_type(
                ret,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            )),
        ),
        ast::Type::Option(inner) => {
            let rewritten_inner = rewrite_module_local_type(
                inner,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            rewrite_shadowed_builtin("Option", vec![rewritten_inner.clone()])
                .unwrap_or_else(|| ast::Type::Option(Box::new(rewritten_inner)))
        }
        ast::Type::Result(ok, err) => {
            let rewritten_ok = rewrite_module_local_type(
                ok,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            let rewritten_err = rewrite_module_local_type(
                err,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            rewrite_shadowed_builtin("Result", vec![rewritten_ok.clone(), rewritten_err.clone()])
                .unwrap_or_else(|| {
                    ast::Type::Result(Box::new(rewritten_ok), Box::new(rewritten_err))
                })
        }
        ast::Type::List(inner) => {
            let rewritten_inner = rewrite_module_local_type(
                inner,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            rewrite_shadowed_builtin("List", vec![rewritten_inner.clone()])
                .unwrap_or_else(|| ast::Type::List(Box::new(rewritten_inner)))
        }
        ast::Type::Map(k, v) => {
            let rewritten_key = rewrite_module_local_type(
                k,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            let rewritten_value = rewrite_module_local_type(
                v,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            rewrite_shadowed_builtin("Map", vec![rewritten_key.clone(), rewritten_value.clone()])
                .unwrap_or_else(|| {
                    ast::Type::Map(Box::new(rewritten_key), Box::new(rewritten_value))
                })
        }
        ast::Type::Set(inner) => {
            let rewritten_inner = rewrite_module_local_type(
                inner,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            rewrite_shadowed_builtin("Set", vec![rewritten_inner.clone()])
                .unwrap_or_else(|| ast::Type::Set(Box::new(rewritten_inner)))
        }
        ast::Type::Ref(inner) => ast::Type::Ref(Box::new(rewrite_module_local_type(
            inner,
            module_prefix,
            current_namespace,
            entry_namespace,
            local_classes,
            local_interfaces,
            local_enums,
            local_modules,
            imported_classes,
            global_class_map,
            imported_enums,
            global_enum_map,
            imported_modules,
            global_interface_map,
        ))),
        ast::Type::MutRef(inner) => ast::Type::MutRef(Box::new(rewrite_module_local_type(
            inner,
            module_prefix,
            current_namespace,
            entry_namespace,
            local_classes,
            local_interfaces,
            local_enums,
            local_modules,
            imported_classes,
            global_class_map,
            imported_enums,
            global_enum_map,
            imported_modules,
            global_interface_map,
        ))),
        ast::Type::Box(inner) => {
            let rewritten_inner = rewrite_module_local_type(
                inner,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            rewrite_shadowed_builtin("Box", vec![rewritten_inner.clone()])
                .unwrap_or_else(|| ast::Type::Box(Box::new(rewritten_inner)))
        }
        ast::Type::Rc(inner) => {
            let rewritten_inner = rewrite_module_local_type(
                inner,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            rewrite_shadowed_builtin("Rc", vec![rewritten_inner.clone()])
                .unwrap_or_else(|| ast::Type::Rc(Box::new(rewritten_inner)))
        }
        ast::Type::Arc(inner) => {
            let rewritten_inner = rewrite_module_local_type(
                inner,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            rewrite_shadowed_builtin("Arc", vec![rewritten_inner.clone()])
                .unwrap_or_else(|| ast::Type::Arc(Box::new(rewritten_inner)))
        }
        ast::Type::Ptr(inner) => {
            let rewritten_inner = rewrite_module_local_type(
                inner,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            rewrite_shadowed_builtin("Ptr", vec![rewritten_inner.clone()])
                .unwrap_or_else(|| ast::Type::Ptr(Box::new(rewritten_inner)))
        }
        ast::Type::Task(inner) => {
            let rewritten_inner = rewrite_module_local_type(
                inner,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            rewrite_shadowed_builtin("Task", vec![rewritten_inner.clone()])
                .unwrap_or_else(|| ast::Type::Task(Box::new(rewritten_inner)))
        }
        ast::Type::Range(inner) => {
            let rewritten_inner = rewrite_module_local_type(
                inner,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_interfaces,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
                global_interface_map,
            );
            rewrite_shadowed_builtin("Range", vec![rewritten_inner.clone()])
                .unwrap_or_else(|| ast::Type::Range(Box::new(rewritten_inner)))
        }
        _ => ty.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn rewrite_block_calls_for_project(
    block: &ast::Block,
    current_namespace: &str,
    entry_namespace: &str,
    local_functions: &HashSet<String>,
    imported_map: &ImportedMap,
    global_function_map: &HashMap<String, String>,
    local_classes: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    local_interfaces: &HashSet<String>,
    imported_interfaces: &ImportedMap,
    global_interface_map: &HashMap<String, String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    local_modules: &HashSet<String>,
    imported_modules: &ImportedMap,
    global_module_map: &HashMap<String, String>,
    scopes: &mut Vec<HashSet<String>>,
) -> ast::Block {
    let started_at = Instant::now();
    REWRITE_INTERNAL_TIMING_TOTALS
        .block_rewrite_calls
        .fetch_add(1, Ordering::Relaxed);
    let rewritten = block
        .iter()
        .map(|stmt| {
            ast::Spanned::new(
                rewrite_stmt_calls_for_project(
                    &stmt.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                stmt.span.clone(),
            )
        })
        .collect();
    REWRITE_INTERNAL_TIMING_TOTALS
        .block_rewrite_ns
        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
    rewritten
}

#[allow(clippy::too_many_arguments)]
fn rewrite_stmt_calls_for_project(
    stmt: &Stmt,
    current_namespace: &str,
    entry_namespace: &str,
    local_functions: &HashSet<String>,
    imported_map: &ImportedMap,
    global_function_map: &HashMap<String, String>,
    local_classes: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    local_interfaces: &HashSet<String>,
    imported_interfaces: &ImportedMap,
    global_interface_map: &HashMap<String, String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    local_modules: &HashSet<String>,
    imported_modules: &ImportedMap,
    global_module_map: &HashMap<String, String>,
    scopes: &mut Vec<HashSet<String>>,
) -> Stmt {
    let started_at = Instant::now();
    REWRITE_INTERNAL_TIMING_TOTALS
        .stmt_rewrite_calls
        .fetch_add(1, Ordering::Relaxed);
    let rewritten = match stmt {
        Stmt::Let {
            name,
            ty,
            value,
            mutable,
        } => {
            let rewritten = Stmt::Let {
                name: name.clone(),
                ty: rewrite_type_for_project_with_interfaces(
                    ty,
                    current_namespace,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    &collect_local_enum_names(global_enum_map, current_namespace),
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    entry_namespace,
                ),
                value: ast::Spanned::new(
                    rewrite_expr_calls_for_project(
                        &value.node,
                        current_namespace,
                        entry_namespace,
                        local_functions,
                        imported_map,
                        global_function_map,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        local_interfaces,
                        imported_interfaces,
                        global_interface_map,
                        imported_enums,
                        global_enum_map,
                        local_modules,
                        imported_modules,
                        global_module_map,
                        scopes,
                    ),
                    value.span.clone(),
                ),
                mutable: *mutable,
            };
            if let Some(scope) = scopes.last_mut() {
                scope.insert(name.clone());
            }
            rewritten
        }
        Stmt::Assign { target, value } => Stmt::Assign {
            target: ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &target.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                target.span.clone(),
            ),
            value: ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &value.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                value.span.clone(),
            ),
        },
        Stmt::Expr(expr) => Stmt::Expr(ast::Spanned::new(
            rewrite_expr_calls_for_project(
                &expr.node,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_interfaces,
                imported_interfaces,
                global_interface_map,
                imported_enums,
                global_enum_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            ),
            expr.span.clone(),
        )),
        Stmt::Return(Some(expr)) => Stmt::Return(Some(ast::Spanned::new(
            rewrite_expr_calls_for_project(
                &expr.node,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_interfaces,
                imported_interfaces,
                global_interface_map,
                imported_enums,
                global_enum_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            ),
            expr.span.clone(),
        ))),
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            let condition = ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &condition.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                condition.span.clone(),
            );
            push_scope(scopes);
            let then_block = rewrite_block_calls_for_project(
                then_block,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_interfaces,
                imported_interfaces,
                global_interface_map,
                imported_enums,
                global_enum_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            );
            pop_scope(scopes);
            let else_block = else_block.as_ref().map(|b| {
                push_scope(scopes);
                let rewritten = rewrite_block_calls_for_project(
                    b,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                );
                pop_scope(scopes);
                rewritten
            });
            Stmt::If {
                condition,
                then_block,
                else_block,
            }
        }
        Stmt::While { condition, body } => {
            let condition = ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &condition.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                condition.span.clone(),
            );
            push_scope(scopes);
            let body = rewrite_block_calls_for_project(
                body,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_interfaces,
                imported_interfaces,
                global_interface_map,
                imported_enums,
                global_enum_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            );
            pop_scope(scopes);
            Stmt::While { condition, body }
        }
        Stmt::For {
            var,
            var_type,
            iterable,
            body,
        } => {
            let iterable = ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &iterable.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                iterable.span.clone(),
            );
            push_scope(scopes);
            if let Some(scope) = scopes.last_mut() {
                scope.insert(var.clone());
            }
            let body = rewrite_block_calls_for_project(
                body,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_interfaces,
                imported_interfaces,
                global_interface_map,
                imported_enums,
                global_enum_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            );
            pop_scope(scopes);
            Stmt::For {
                var: var.clone(),
                var_type: var_type.as_ref().map(|t| {
                    rewrite_type_for_project_with_interfaces(
                        t,
                        current_namespace,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        local_interfaces,
                        imported_interfaces,
                        global_interface_map,
                        &collect_local_enum_names(global_enum_map, current_namespace),
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        entry_namespace,
                    )
                }),
                iterable,
                body,
            }
        }
        Stmt::Match { expr, arms } => Stmt::Match {
            expr: ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &expr.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                expr.span.clone(),
            ),
            arms: arms
                .iter()
                .map(|arm| {
                    let rewritten_pattern = rewrite_pattern_for_project(
                        &arm.pattern,
                        current_namespace,
                        entry_namespace,
                        local_modules,
                        imported_modules,
                        global_enum_map,
                    );
                    push_scope(scopes);
                    if let Some(scope) = scopes.last_mut() {
                        bind_pattern_locals(&rewritten_pattern, scope);
                    }
                    let body = rewrite_block_calls_for_project(
                        &arm.body,
                        current_namespace,
                        entry_namespace,
                        local_functions,
                        imported_map,
                        global_function_map,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        local_interfaces,
                        imported_interfaces,
                        global_interface_map,
                        imported_enums,
                        global_enum_map,
                        local_modules,
                        imported_modules,
                        global_module_map,
                        scopes,
                    );
                    pop_scope(scopes);
                    ast::MatchArm {
                        pattern: rewritten_pattern,
                        body,
                    }
                })
                .collect(),
        },
        _ => stmt.clone(),
    };
    REWRITE_INTERNAL_TIMING_TOTALS
        .stmt_rewrite_ns
        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
    rewritten
}

#[allow(clippy::too_many_arguments)]
fn rewrite_expr_calls_for_project(
    expr: &Expr,
    current_namespace: &str,
    entry_namespace: &str,
    local_functions: &HashSet<String>,
    imported_map: &ImportedMap,
    global_function_map: &HashMap<String, String>,
    local_classes: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    local_interfaces: &HashSet<String>,
    imported_interfaces: &ImportedMap,
    global_interface_map: &HashMap<String, String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    local_modules: &HashSet<String>,
    imported_modules: &ImportedMap,
    global_module_map: &HashMap<String, String>,
    scopes: &mut Vec<HashSet<String>>,
) -> Expr {
    let started_at = Instant::now();
    REWRITE_INTERNAL_TIMING_TOTALS
        .expr_rewrite_calls
        .fetch_add(1, Ordering::Relaxed);
    let rewritten = match expr {
        Expr::Call {
            callee,
            args,
            type_args,
        } => {
            if let Some(path_parts) = flatten_field_chain(&callee.node) {
                let module_alias = &path_parts[0];
                let member_parts = &path_parts[1..];
                if !member_parts.is_empty() && !is_shadowed(module_alias, scopes) {
                    if local_modules.contains(module_alias) {
                        if let Some((owner_ns, candidate)) = resolve_module_alias_function_candidate(
                            current_namespace,
                            module_alias,
                            member_parts,
                            global_function_map,
                        ) {
                            return Expr::Call {
                                callee: Box::new(ast::Spanned::new(
                                    Expr::Ident(mangle_project_function_symbol(
                                        &owner_ns,
                                        entry_namespace,
                                        &candidate,
                                    )),
                                    callee.span.clone(),
                                )),
                                args: args
                                    .iter()
                                    .map(|arg| {
                                        ast::Spanned::new(
                                            rewrite_expr_calls_for_project(
                                                &arg.node,
                                                current_namespace,
                                                entry_namespace,
                                                local_functions,
                                                imported_map,
                                                global_function_map,
                                                local_classes,
                                                imported_classes,
                                                global_class_map,
                                                local_interfaces,
                                                imported_interfaces,
                                                global_interface_map,
                                                imported_enums,
                                                global_enum_map,
                                                local_modules,
                                                imported_modules,
                                                global_module_map,
                                                scopes,
                                            ),
                                            arg.span.clone(),
                                        )
                                    })
                                    .collect(),
                                type_args: type_args
                                    .iter()
                                    .map(|ty| {
                                        rewrite_type_for_project_with_interfaces(
                                            ty,
                                            current_namespace,
                                            local_classes,
                                            imported_classes,
                                            global_class_map,
                                            local_interfaces,
                                            imported_interfaces,
                                            global_interface_map,
                                            &collect_local_enum_names(
                                                global_enum_map,
                                                current_namespace,
                                            ),
                                            imported_enums,
                                            global_enum_map,
                                            imported_modules,
                                            entry_namespace,
                                        )
                                    })
                                    .collect(),
                            };
                        }
                        if let Some((owner_ns, enum_name, variant_name)) =
                            resolve_module_alias_enum_candidate(
                                current_namespace,
                                module_alias,
                                member_parts,
                                global_enum_map,
                            )
                        {
                            return Expr::Call {
                                callee: Box::new(ast::Spanned::new(
                                    Expr::Field {
                                        object: Box::new(ast::Spanned::new(
                                            Expr::Ident(mangle_project_symbol(
                                                &owner_ns,
                                                entry_namespace,
                                                &enum_name,
                                            )),
                                            callee.span.clone(),
                                        )),
                                        field: variant_name,
                                    },
                                    callee.span.clone(),
                                )),
                                args: args
                                    .iter()
                                    .map(|arg| {
                                        ast::Spanned::new(
                                            rewrite_expr_calls_for_project(
                                                &arg.node,
                                                current_namespace,
                                                entry_namespace,
                                                local_functions,
                                                imported_map,
                                                global_function_map,
                                                local_classes,
                                                imported_classes,
                                                global_class_map,
                                                local_interfaces,
                                                imported_interfaces,
                                                global_interface_map,
                                                imported_enums,
                                                global_enum_map,
                                                local_modules,
                                                imported_modules,
                                                global_module_map,
                                                scopes,
                                            ),
                                            arg.span.clone(),
                                        )
                                    })
                                    .collect(),
                                type_args: vec![],
                            };
                        }
                        if let Some((owner_ns, class_name)) = resolve_module_alias_class_candidate(
                            current_namespace,
                            module_alias,
                            member_parts,
                            global_class_map,
                        ) {
                            let rewritten_type_args = type_args
                                .iter()
                                .map(|ty| {
                                    rewrite_type_for_project_with_interfaces(
                                        ty,
                                        current_namespace,
                                        local_classes,
                                        imported_classes,
                                        global_class_map,
                                        local_interfaces,
                                        imported_interfaces,
                                        global_interface_map,
                                        &collect_local_enum_names(
                                            global_enum_map,
                                            current_namespace,
                                        ),
                                        imported_enums,
                                        global_enum_map,
                                        imported_modules,
                                        entry_namespace,
                                    )
                                })
                                .collect::<Vec<_>>();
                            return Expr::Construct {
                                ty: format_construct_type_name(
                                    &mangle_project_symbol(&owner_ns, entry_namespace, &class_name),
                                    &rewritten_type_args,
                                ),
                                args: args
                                    .iter()
                                    .map(|arg| {
                                        ast::Spanned::new(
                                            rewrite_expr_calls_for_project(
                                                &arg.node,
                                                current_namespace,
                                                entry_namespace,
                                                local_functions,
                                                imported_map,
                                                global_function_map,
                                                local_classes,
                                                imported_classes,
                                                global_class_map,
                                                local_interfaces,
                                                imported_interfaces,
                                                global_interface_map,
                                                imported_enums,
                                                global_enum_map,
                                                local_modules,
                                                imported_modules,
                                                global_module_map,
                                                scopes,
                                            ),
                                            arg.span.clone(),
                                        )
                                    })
                                    .collect(),
                            };
                        }
                    }
                    if let Some((ns, symbol_name)) = imported_modules.get(module_alias) {
                        if let Some((owner_ns, class_name)) = resolve_module_alias_class_candidate(
                            ns,
                            symbol_name,
                            member_parts,
                            global_class_map,
                        ) {
                            let rewritten_type_args = type_args
                                .iter()
                                .map(|ty| {
                                    rewrite_type_for_project_with_interfaces(
                                        ty,
                                        current_namespace,
                                        local_classes,
                                        imported_classes,
                                        global_class_map,
                                        local_interfaces,
                                        imported_interfaces,
                                        global_interface_map,
                                        &collect_local_enum_names(
                                            global_enum_map,
                                            current_namespace,
                                        ),
                                        imported_enums,
                                        global_enum_map,
                                        imported_modules,
                                        entry_namespace,
                                    )
                                })
                                .collect::<Vec<_>>();
                            return Expr::Construct {
                                ty: format_construct_type_name(
                                    &mangle_project_symbol(&owner_ns, entry_namespace, &class_name),
                                    &rewritten_type_args,
                                ),
                                args: args
                                    .iter()
                                    .map(|arg| {
                                        ast::Spanned::new(
                                            rewrite_expr_calls_for_project(
                                                &arg.node,
                                                current_namespace,
                                                entry_namespace,
                                                local_functions,
                                                imported_map,
                                                global_function_map,
                                                local_classes,
                                                imported_classes,
                                                global_class_map,
                                                local_interfaces,
                                                imported_interfaces,
                                                global_interface_map,
                                                imported_enums,
                                                global_enum_map,
                                                local_modules,
                                                imported_modules,
                                                global_module_map,
                                                scopes,
                                            ),
                                            arg.span.clone(),
                                        )
                                    })
                                    .collect(),
                            };
                        }
                    }
                }
            }
            if let Expr::Ident(name) = &callee.node {
                if !is_shadowed(name, scopes) {
                    if let Some((ns, symbol_name)) = imported_classes.get(name) {
                        let rewritten_type_args = type_args
                            .iter()
                            .map(|ty| {
                                rewrite_type_for_project_with_interfaces(
                                    ty,
                                    current_namespace,
                                    local_classes,
                                    imported_classes,
                                    global_class_map,
                                    local_interfaces,
                                    imported_interfaces,
                                    global_interface_map,
                                    &collect_local_enum_names(global_enum_map, current_namespace),
                                    imported_enums,
                                    global_enum_map,
                                    imported_modules,
                                    entry_namespace,
                                )
                            })
                            .collect::<Vec<_>>();
                        return Expr::Construct {
                            ty: format_construct_type_name(
                                &mangle_project_symbol(ns, entry_namespace, symbol_name),
                                &rewritten_type_args,
                            ),
                            args: args
                                .iter()
                                .map(|arg| {
                                    ast::Spanned::new(
                                        rewrite_expr_calls_for_project(
                                            &arg.node,
                                            current_namespace,
                                            entry_namespace,
                                            local_functions,
                                            imported_map,
                                            global_function_map,
                                            local_classes,
                                            imported_classes,
                                            global_class_map,
                                            local_interfaces,
                                            imported_interfaces,
                                            global_interface_map,
                                            imported_enums,
                                            global_enum_map,
                                            local_modules,
                                            imported_modules,
                                            global_module_map,
                                            scopes,
                                        ),
                                        arg.span.clone(),
                                    )
                                })
                                .collect(),
                        };
                    }
                    if let Some((import_ns, symbol_name)) = imported_modules.get(name) {
                        if let Some((owner_ns, enum_name, variant_name)) =
                            resolve_exact_imported_variant_alias(
                                import_ns,
                                symbol_name,
                                global_enum_map,
                            )
                        {
                            return Expr::Call {
                                callee: Box::new(ast::Spanned::new(
                                    Expr::Field {
                                        object: Box::new(ast::Spanned::new(
                                            Expr::Ident(mangle_project_symbol(
                                                &owner_ns,
                                                entry_namespace,
                                                &enum_name,
                                            )),
                                            callee.span.clone(),
                                        )),
                                        field: variant_name,
                                    },
                                    callee.span.clone(),
                                )),
                                args: args
                                    .iter()
                                    .map(|arg| {
                                        ast::Spanned::new(
                                            rewrite_expr_calls_for_project(
                                                &arg.node,
                                                current_namespace,
                                                entry_namespace,
                                                local_functions,
                                                imported_map,
                                                global_function_map,
                                                local_classes,
                                                imported_classes,
                                                global_class_map,
                                                local_interfaces,
                                                imported_interfaces,
                                                global_interface_map,
                                                imported_enums,
                                                global_enum_map,
                                                local_modules,
                                                imported_modules,
                                                global_module_map,
                                                scopes,
                                            ),
                                            arg.span.clone(),
                                        )
                                    })
                                    .collect(),
                                type_args: vec![],
                            };
                        }
                    }
                }
            }
            let rewritten_callee = match &callee.node {
                // Namespace alias + module dot syntax:
                // import lib as l; l.Tools.ping() -> lib__Tools__ping()
                Expr::Field { object, field }
                    if matches!(&object.node, Expr::Field { .. })
                        && flatten_field_chain(&object.node)
                            .is_some_and(|parts| !parts.is_empty()) =>
                {
                    let chain_parts =
                        flatten_field_chain(&object.node).expect("guarded by is_some");
                    let alias_ident = &chain_parts[0];

                    if is_shadowed(alias_ident, scopes) {
                        rewrite_expr_calls_for_project(
                            &callee.node,
                            current_namespace,
                            entry_namespace,
                            local_functions,
                            imported_map,
                            global_function_map,
                            local_classes,
                            imported_classes,
                            global_class_map,
                            local_interfaces,
                            imported_interfaces,
                            global_interface_map,
                            imported_enums,
                            global_enum_map,
                            local_modules,
                            imported_modules,
                            global_module_map,
                            scopes,
                        )
                    } else if let Some((ns, symbol_name)) = imported_modules.get(alias_ident) {
                        if symbol_name.is_empty() {
                            let mut member_parts = chain_parts[1..].to_vec();
                            member_parts.push(field.to_string());
                            if let Some((owner_ns, class_name)) =
                                resolve_module_alias_class_candidate(
                                    ns,
                                    symbol_name,
                                    &member_parts,
                                    global_class_map,
                                )
                            {
                                let rewritten_type_args = type_args
                                    .iter()
                                    .map(|ty| {
                                        rewrite_type_for_project_with_interfaces(
                                            ty,
                                            current_namespace,
                                            local_classes,
                                            imported_classes,
                                            global_class_map,
                                            local_interfaces,
                                            imported_interfaces,
                                            global_interface_map,
                                            &collect_local_enum_names(
                                                global_enum_map,
                                                current_namespace,
                                            ),
                                            imported_enums,
                                            global_enum_map,
                                            imported_modules,
                                            entry_namespace,
                                        )
                                    })
                                    .collect::<Vec<_>>();
                                Expr::Construct {
                                    ty: format_construct_type_name(
                                        &mangle_project_symbol(
                                            &owner_ns,
                                            entry_namespace,
                                            &class_name,
                                        ),
                                        &rewritten_type_args,
                                    ),
                                    args: args
                                        .iter()
                                        .map(|arg| {
                                            ast::Spanned::new(
                                                rewrite_expr_calls_for_project(
                                                    &arg.node,
                                                    current_namespace,
                                                    entry_namespace,
                                                    local_functions,
                                                    imported_map,
                                                    global_function_map,
                                                    local_classes,
                                                    imported_classes,
                                                    global_class_map,
                                                    local_interfaces,
                                                    imported_interfaces,
                                                    global_interface_map,
                                                    imported_enums,
                                                    global_enum_map,
                                                    local_modules,
                                                    imported_modules,
                                                    global_module_map,
                                                    scopes,
                                                ),
                                                arg.span.clone(),
                                            )
                                        })
                                        .collect(),
                                }
                            } else if let Some((owner_ns, enum_name, variant_name)) =
                                resolve_module_alias_enum_candidate(
                                    ns,
                                    symbol_name,
                                    &member_parts,
                                    global_enum_map,
                                )
                            {
                                Expr::Field {
                                    object: Box::new(ast::Spanned::new(
                                        Expr::Ident(mangle_project_symbol(
                                            &owner_ns,
                                            entry_namespace,
                                            &enum_name,
                                        )),
                                        object.span.clone(),
                                    )),
                                    field: variant_name,
                                }
                            } else {
                                let candidate = if chain_parts.len() > 1 {
                                    format!("{}__{}", chain_parts[1..].join("__"), field)
                                } else {
                                    field.to_string()
                                };
                                if let Some(owner_ns) = global_function_map.get(&candidate) {
                                    if owner_ns == ns {
                                        Expr::Ident(mangle_project_symbol(
                                            owner_ns,
                                            entry_namespace,
                                            &candidate,
                                        ))
                                    } else {
                                        rewrite_expr_calls_for_project(
                                            &callee.node,
                                            current_namespace,
                                            entry_namespace,
                                            local_functions,
                                            imported_map,
                                            global_function_map,
                                            local_classes,
                                            imported_classes,
                                            global_class_map,
                                            local_interfaces,
                                            imported_interfaces,
                                            global_interface_map,
                                            imported_enums,
                                            global_enum_map,
                                            local_modules,
                                            imported_modules,
                                            global_module_map,
                                            scopes,
                                        )
                                    }
                                } else {
                                    rewrite_expr_calls_for_project(
                                        &callee.node,
                                        current_namespace,
                                        entry_namespace,
                                        local_functions,
                                        imported_map,
                                        global_function_map,
                                        local_classes,
                                        imported_classes,
                                        global_class_map,
                                        local_interfaces,
                                        imported_interfaces,
                                        global_interface_map,
                                        imported_enums,
                                        global_enum_map,
                                        local_modules,
                                        imported_modules,
                                        global_module_map,
                                        scopes,
                                    )
                                }
                            }
                        } else {
                            rewrite_expr_calls_for_project(
                                &callee.node,
                                current_namespace,
                                entry_namespace,
                                local_functions,
                                imported_map,
                                global_function_map,
                                local_classes,
                                imported_classes,
                                global_class_map,
                                local_interfaces,
                                imported_interfaces,
                                global_interface_map,
                                imported_enums,
                                global_enum_map,
                                local_modules,
                                imported_modules,
                                global_module_map,
                                scopes,
                            )
                        }
                    } else {
                        rewrite_expr_calls_for_project(
                            &callee.node,
                            current_namespace,
                            entry_namespace,
                            local_functions,
                            imported_map,
                            global_function_map,
                            local_classes,
                            imported_classes,
                            global_class_map,
                            local_interfaces,
                            imported_interfaces,
                            global_interface_map,
                            imported_enums,
                            global_enum_map,
                            local_modules,
                            imported_modules,
                            global_module_map,
                            scopes,
                        )
                    }
                }
                Expr::Field { object, field } if !matches!(&object.node, Expr::Ident(name) if is_shadowed(name, scopes)) => {
                    if let Some(path_parts) = flatten_field_chain(&callee.node) {
                        let module_alias = &path_parts[0];
                        let member_parts = &path_parts[1..];
                        if let Some((ns, enum_name)) = imported_enums.get(module_alias) {
                            if member_parts.len() == 1 && member_parts[0] == *field {
                                Expr::Field {
                                    object: Box::new(ast::Spanned::new(
                                        Expr::Ident(mangle_project_symbol(
                                            ns,
                                            entry_namespace,
                                            enum_name,
                                        )),
                                        object.span.clone(),
                                    )),
                                    field: field.clone(),
                                }
                            } else if let Some((ns, symbol_name)) =
                                imported_modules.get(module_alias)
                            {
                                if member_parts.is_empty() {
                                    return rewrite_expr_calls_for_project(
                                        &callee.node,
                                        current_namespace,
                                        entry_namespace,
                                        local_functions,
                                        imported_map,
                                        global_function_map,
                                        local_classes,
                                        imported_classes,
                                        global_class_map,
                                        local_interfaces,
                                        imported_interfaces,
                                        global_interface_map,
                                        imported_enums,
                                        global_enum_map,
                                        local_modules,
                                        imported_modules,
                                        global_module_map,
                                        scopes,
                                    );
                                }
                                let field = member_parts.last().expect("non-empty member parts");
                                if let Some((owner_ns, class_name)) =
                                    resolve_module_alias_class_candidate(
                                        ns,
                                        symbol_name,
                                        member_parts,
                                        global_class_map,
                                    )
                                {
                                    let rewritten_type_args = type_args
                                        .iter()
                                        .map(|ty| {
                                            rewrite_type_for_project_with_interfaces(
                                                ty,
                                                current_namespace,
                                                local_classes,
                                                imported_classes,
                                                global_class_map,
                                                local_interfaces,
                                                imported_interfaces,
                                                global_interface_map,
                                                &collect_local_enum_names(
                                                    global_enum_map,
                                                    current_namespace,
                                                ),
                                                imported_enums,
                                                global_enum_map,
                                                imported_modules,
                                                entry_namespace,
                                            )
                                        })
                                        .collect::<Vec<_>>();
                                    return Expr::Construct {
                                        ty: format_construct_type_name(
                                            &mangle_project_symbol(
                                                &owner_ns,
                                                entry_namespace,
                                                &class_name,
                                            ),
                                            &rewritten_type_args,
                                        ),
                                        args: args
                                            .iter()
                                            .map(|arg| {
                                                ast::Spanned::new(
                                                    rewrite_expr_calls_for_project(
                                                        &arg.node,
                                                        current_namespace,
                                                        entry_namespace,
                                                        local_functions,
                                                        imported_map,
                                                        global_function_map,
                                                        local_classes,
                                                        imported_classes,
                                                        global_class_map,
                                                        local_interfaces,
                                                        imported_interfaces,
                                                        global_interface_map,
                                                        imported_enums,
                                                        global_enum_map,
                                                        local_modules,
                                                        imported_modules,
                                                        global_module_map,
                                                        scopes,
                                                    ),
                                                    arg.span.clone(),
                                                )
                                            })
                                            .collect(),
                                    };
                                } else if let Some((owner_ns, enum_name, variant_name)) =
                                    resolve_module_alias_enum_candidate(
                                        ns,
                                        symbol_name,
                                        member_parts,
                                        global_enum_map,
                                    )
                                {
                                    Expr::Field {
                                        object: Box::new(ast::Spanned::new(
                                            Expr::Ident(mangle_project_symbol(
                                                &owner_ns,
                                                entry_namespace,
                                                &enum_name,
                                            )),
                                            object.span.clone(),
                                        )),
                                        field: variant_name,
                                    }
                                } else {
                                    let namespace_path = if symbol_name.is_empty() {
                                        ns.clone()
                                    } else {
                                        format!("{}.{}", ns, symbol_name)
                                    };
                                    if let Some(canonical) =
                                        stdlib_registry().resolve_alias_call(&namespace_path, field)
                                    {
                                        if let Some((owner, method)) = canonical.split_once("__") {
                                            Expr::Field {
                                                object: Box::new(ast::Spanned::new(
                                                    Expr::Ident(owner.to_string()),
                                                    object.span.clone(),
                                                )),
                                                field: method.to_string(),
                                            }
                                        } else {
                                            Expr::Ident(canonical)
                                        }
                                    } else if symbol_name.is_empty() && member_parts.len() == 1 {
                                        if let Some(owner_ns) = global_function_map.get(field) {
                                            if owner_ns == ns {
                                                Expr::Ident(mangle_project_function_symbol(
                                                    owner_ns,
                                                    entry_namespace,
                                                    field,
                                                ))
                                            } else {
                                                rewrite_expr_calls_for_project(
                                                    &callee.node,
                                                    current_namespace,
                                                    entry_namespace,
                                                    local_functions,
                                                    imported_map,
                                                    global_function_map,
                                                    local_classes,
                                                    imported_classes,
                                                    global_class_map,
                                                    local_interfaces,
                                                    imported_interfaces,
                                                    global_interface_map,
                                                    imported_enums,
                                                    global_enum_map,
                                                    local_modules,
                                                    imported_modules,
                                                    global_module_map,
                                                    scopes,
                                                )
                                            }
                                        } else {
                                            rewrite_expr_calls_for_project(
                                                &callee.node,
                                                current_namespace,
                                                entry_namespace,
                                                local_functions,
                                                imported_map,
                                                global_function_map,
                                                local_classes,
                                                imported_classes,
                                                global_class_map,
                                                local_interfaces,
                                                imported_interfaces,
                                                global_interface_map,
                                                imported_enums,
                                                global_enum_map,
                                                local_modules,
                                                imported_modules,
                                                global_module_map,
                                                scopes,
                                            )
                                        }
                                    } else if let Some((owner_ns, candidate)) =
                                        resolve_module_alias_function_candidate(
                                            ns,
                                            symbol_name,
                                            member_parts,
                                            global_function_map,
                                        )
                                    {
                                        Expr::Ident(mangle_project_function_symbol(
                                            &owner_ns,
                                            entry_namespace,
                                            &candidate,
                                        ))
                                    } else {
                                        rewrite_expr_calls_for_project(
                                            &callee.node,
                                            current_namespace,
                                            entry_namespace,
                                            local_functions,
                                            imported_map,
                                            global_function_map,
                                            local_classes,
                                            imported_classes,
                                            global_class_map,
                                            local_interfaces,
                                            imported_interfaces,
                                            global_interface_map,
                                            imported_enums,
                                            global_enum_map,
                                            local_modules,
                                            imported_modules,
                                            global_module_map,
                                            scopes,
                                        )
                                    }
                                }
                            } else {
                                rewrite_expr_calls_for_project(
                                    &callee.node,
                                    current_namespace,
                                    entry_namespace,
                                    local_functions,
                                    imported_map,
                                    global_function_map,
                                    local_classes,
                                    imported_classes,
                                    global_class_map,
                                    local_interfaces,
                                    imported_interfaces,
                                    global_interface_map,
                                    imported_enums,
                                    global_enum_map,
                                    local_modules,
                                    imported_modules,
                                    global_module_map,
                                    scopes,
                                )
                            }
                        } else if let Some((ns, symbol_name)) = imported_modules.get(module_alias) {
                            if member_parts.is_empty() {
                                return rewrite_expr_calls_for_project(
                                    &callee.node,
                                    current_namespace,
                                    entry_namespace,
                                    local_functions,
                                    imported_map,
                                    global_function_map,
                                    local_classes,
                                    imported_classes,
                                    global_class_map,
                                    local_interfaces,
                                    imported_interfaces,
                                    global_interface_map,
                                    imported_enums,
                                    global_enum_map,
                                    local_modules,
                                    imported_modules,
                                    global_module_map,
                                    scopes,
                                );
                            }
                            let field = member_parts.last().expect("non-empty member parts");
                            if let Some((owner_ns, class_name)) =
                                resolve_module_alias_class_candidate(
                                    ns,
                                    symbol_name,
                                    member_parts,
                                    global_class_map,
                                )
                            {
                                let rewritten_type_args = type_args
                                    .iter()
                                    .map(|ty| {
                                        rewrite_type_for_project_with_interfaces(
                                            ty,
                                            current_namespace,
                                            local_classes,
                                            imported_classes,
                                            global_class_map,
                                            local_interfaces,
                                            imported_interfaces,
                                            global_interface_map,
                                            &collect_local_enum_names(
                                                global_enum_map,
                                                current_namespace,
                                            ),
                                            imported_enums,
                                            global_enum_map,
                                            imported_modules,
                                            entry_namespace,
                                        )
                                    })
                                    .collect::<Vec<_>>();
                                return Expr::Construct {
                                    ty: format_construct_type_name(
                                        &mangle_project_symbol(
                                            &owner_ns,
                                            entry_namespace,
                                            &class_name,
                                        ),
                                        &rewritten_type_args,
                                    ),
                                    args: args
                                        .iter()
                                        .map(|arg| {
                                            ast::Spanned::new(
                                                rewrite_expr_calls_for_project(
                                                    &arg.node,
                                                    current_namespace,
                                                    entry_namespace,
                                                    local_functions,
                                                    imported_map,
                                                    global_function_map,
                                                    local_classes,
                                                    imported_classes,
                                                    global_class_map,
                                                    local_interfaces,
                                                    imported_interfaces,
                                                    global_interface_map,
                                                    imported_enums,
                                                    global_enum_map,
                                                    local_modules,
                                                    imported_modules,
                                                    global_module_map,
                                                    scopes,
                                                ),
                                                arg.span.clone(),
                                            )
                                        })
                                        .collect(),
                                };
                            } else if let Some((owner_ns, enum_name, variant_name)) =
                                resolve_module_alias_enum_candidate(
                                    ns,
                                    symbol_name,
                                    member_parts,
                                    global_enum_map,
                                )
                            {
                                Expr::Field {
                                    object: Box::new(ast::Spanned::new(
                                        Expr::Ident(mangle_project_symbol(
                                            &owner_ns,
                                            entry_namespace,
                                            &enum_name,
                                        )),
                                        object.span.clone(),
                                    )),
                                    field: variant_name,
                                }
                            } else {
                                let namespace_path = if symbol_name.is_empty() {
                                    ns.clone()
                                } else {
                                    format!("{}.{}", ns, symbol_name)
                                };
                                if let Some(canonical) =
                                    stdlib_registry().resolve_alias_call(&namespace_path, field)
                                {
                                    if let Some((owner, method)) = canonical.split_once("__") {
                                        Expr::Field {
                                            object: Box::new(ast::Spanned::new(
                                                Expr::Ident(owner.to_string()),
                                                object.span.clone(),
                                            )),
                                            field: method.to_string(),
                                        }
                                    } else {
                                        Expr::Ident(canonical)
                                    }
                                } else if symbol_name.is_empty() && member_parts.len() == 1 {
                                    if let Some(owner_ns) = global_function_map.get(field) {
                                        if owner_ns == ns {
                                            Expr::Ident(mangle_project_function_symbol(
                                                owner_ns,
                                                entry_namespace,
                                                field,
                                            ))
                                        } else {
                                            rewrite_expr_calls_for_project(
                                                &callee.node,
                                                current_namespace,
                                                entry_namespace,
                                                local_functions,
                                                imported_map,
                                                global_function_map,
                                                local_classes,
                                                imported_classes,
                                                global_class_map,
                                                local_interfaces,
                                                imported_interfaces,
                                                global_interface_map,
                                                imported_enums,
                                                global_enum_map,
                                                local_modules,
                                                imported_modules,
                                                global_module_map,
                                                scopes,
                                            )
                                        }
                                    } else {
                                        rewrite_expr_calls_for_project(
                                            &callee.node,
                                            current_namespace,
                                            entry_namespace,
                                            local_functions,
                                            imported_map,
                                            global_function_map,
                                            local_classes,
                                            imported_classes,
                                            global_class_map,
                                            local_interfaces,
                                            imported_interfaces,
                                            global_interface_map,
                                            imported_enums,
                                            global_enum_map,
                                            local_modules,
                                            imported_modules,
                                            global_module_map,
                                            scopes,
                                        )
                                    }
                                } else if let Some((owner_ns, candidate)) =
                                    resolve_module_alias_function_candidate(
                                        ns,
                                        symbol_name,
                                        member_parts,
                                        global_function_map,
                                    )
                                {
                                    Expr::Ident(mangle_project_function_symbol(
                                        &owner_ns,
                                        entry_namespace,
                                        &candidate,
                                    ))
                                } else {
                                    rewrite_expr_calls_for_project(
                                        &callee.node,
                                        current_namespace,
                                        entry_namespace,
                                        local_functions,
                                        imported_map,
                                        global_function_map,
                                        local_classes,
                                        imported_classes,
                                        global_class_map,
                                        local_interfaces,
                                        imported_interfaces,
                                        global_interface_map,
                                        imported_enums,
                                        global_enum_map,
                                        local_modules,
                                        imported_modules,
                                        global_module_map,
                                        scopes,
                                    )
                                }
                            }
                        } else {
                            rewrite_expr_calls_for_project(
                                &callee.node,
                                current_namespace,
                                entry_namespace,
                                local_functions,
                                imported_map,
                                global_function_map,
                                local_classes,
                                imported_classes,
                                global_class_map,
                                local_interfaces,
                                imported_interfaces,
                                global_interface_map,
                                imported_enums,
                                global_enum_map,
                                local_modules,
                                imported_modules,
                                global_module_map,
                                scopes,
                            )
                        }
                    } else {
                        rewrite_expr_calls_for_project(
                            &callee.node,
                            current_namespace,
                            entry_namespace,
                            local_functions,
                            imported_map,
                            global_function_map,
                            local_classes,
                            imported_classes,
                            global_class_map,
                            local_interfaces,
                            imported_interfaces,
                            global_interface_map,
                            imported_enums,
                            global_enum_map,
                            local_modules,
                            imported_modules,
                            global_module_map,
                            scopes,
                        )
                    }
                }
                Expr::Ident(name) => {
                    if is_shadowed(name, scopes) {
                        Expr::Ident(name.clone())
                    } else if local_functions.contains(name) {
                        Expr::Ident(mangle_project_function_symbol(
                            current_namespace,
                            entry_namespace,
                            name,
                        ))
                    } else if let Some((ns, symbol_name)) = imported_map.get(name) {
                        if stdlib_registry()
                            .get_namespace(symbol_name)
                            .is_some_and(|owner| owner == ns)
                        {
                            Expr::Ident(symbol_name.clone())
                        } else {
                            Expr::Ident(mangle_project_function_symbol(
                                ns,
                                entry_namespace,
                                symbol_name,
                            ))
                        }
                    } else if let Some(ns) = global_function_map.get(name) {
                        Expr::Ident(mangle_project_function_symbol(ns, entry_namespace, name))
                    } else {
                        Expr::Ident(name.clone())
                    }
                }
                other => rewrite_expr_calls_for_project(
                    other,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
            };
            if let Expr::Ident(name) = &rewritten_callee {
                let is_class_symbol = local_classes.iter().any(|class_name| {
                    mangle_project_symbol(current_namespace, entry_namespace, class_name) == *name
                }) || global_class_map.iter().any(
                    |(class_name, owner_ns)| {
                        mangle_project_symbol(owner_ns, entry_namespace, class_name) == *name
                    },
                );
                if is_class_symbol {
                    let rewritten_type_args = type_args
                        .iter()
                        .map(|ty| {
                            rewrite_type_for_project_with_interfaces(
                                ty,
                                current_namespace,
                                local_classes,
                                imported_classes,
                                global_class_map,
                                local_interfaces,
                                imported_interfaces,
                                global_interface_map,
                                &collect_local_enum_names(global_enum_map, current_namespace),
                                imported_enums,
                                global_enum_map,
                                imported_modules,
                                entry_namespace,
                            )
                        })
                        .collect::<Vec<_>>();
                    return Expr::Construct {
                        ty: format_construct_type_name(name, &rewritten_type_args),
                        args: args
                            .iter()
                            .map(|a| {
                                ast::Spanned::new(
                                    rewrite_expr_calls_for_project(
                                        &a.node,
                                        current_namespace,
                                        entry_namespace,
                                        local_functions,
                                        imported_map,
                                        global_function_map,
                                        local_classes,
                                        imported_classes,
                                        global_class_map,
                                        local_interfaces,
                                        imported_interfaces,
                                        global_interface_map,
                                        imported_enums,
                                        global_enum_map,
                                        local_modules,
                                        imported_modules,
                                        global_module_map,
                                        scopes,
                                    ),
                                    a.span.clone(),
                                )
                            })
                            .collect(),
                    };
                }
            }
            Expr::Call {
                callee: Box::new(ast::Spanned::new(rewritten_callee, callee.span.clone())),
                args: args
                    .iter()
                    .map(|a| {
                        ast::Spanned::new(
                            rewrite_expr_calls_for_project(
                                &a.node,
                                current_namespace,
                                entry_namespace,
                                local_functions,
                                imported_map,
                                global_function_map,
                                local_classes,
                                imported_classes,
                                global_class_map,
                                local_interfaces,
                                imported_interfaces,
                                global_interface_map,
                                imported_enums,
                                global_enum_map,
                                local_modules,
                                imported_modules,
                                global_module_map,
                                scopes,
                            ),
                            a.span.clone(),
                        )
                    })
                    .collect(),
                type_args: type_args
                    .iter()
                    .map(|ty| {
                        rewrite_type_for_project_with_interfaces(
                            ty,
                            current_namespace,
                            local_classes,
                            imported_classes,
                            global_class_map,
                            local_interfaces,
                            imported_interfaces,
                            global_interface_map,
                            &collect_local_enum_names(global_enum_map, current_namespace),
                            imported_enums,
                            global_enum_map,
                            imported_modules,
                            entry_namespace,
                        )
                    })
                    .collect(),
            }
        }
        Expr::Binary { op, left, right } => Expr::Binary {
            op: *op,
            left: Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &left.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                left.span.clone(),
            )),
            right: Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &right.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                right.span.clone(),
            )),
        },
        Expr::Unary { op, expr } => Expr::Unary {
            op: *op,
            expr: Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &expr.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                expr.span.clone(),
            )),
        },
        Expr::Field { object, field } => {
            if let Some(path_parts) = flatten_field_chain(expr) {
                let module_alias = &path_parts[0];
                let member_parts = &path_parts[1..];
                if !member_parts.is_empty() && !is_shadowed(module_alias, scopes) {
                    if local_modules.contains(module_alias) {
                        if let Some((owner_ns, enum_name, variant_name)) =
                            resolve_module_alias_enum_candidate(
                                current_namespace,
                                module_alias,
                                member_parts,
                                global_enum_map,
                            )
                        {
                            return Expr::Field {
                                object: Box::new(ast::Spanned::new(
                                    Expr::Ident(mangle_project_symbol(
                                        &owner_ns,
                                        entry_namespace,
                                        &enum_name,
                                    )),
                                    object.span.clone(),
                                )),
                                field: variant_name,
                            };
                        }
                        if let Some((owner_ns, class_name)) = resolve_module_alias_class_candidate(
                            current_namespace,
                            module_alias,
                            member_parts,
                            global_class_map,
                        ) {
                            return Expr::Ident(mangle_project_symbol(
                                &owner_ns,
                                entry_namespace,
                                &class_name,
                            ));
                        }
                        if let Some((owner_ns, candidate)) = resolve_module_alias_function_candidate(
                            current_namespace,
                            module_alias,
                            member_parts,
                            global_function_map,
                        ) {
                            return Expr::Ident(mangle_project_function_symbol(
                                &owner_ns,
                                entry_namespace,
                                &candidate,
                            ));
                        }
                    }
                    if let Some((ns, symbol_name)) = imported_modules.get(module_alias) {
                        if let Some((owner_ns, enum_name, variant_name)) =
                            resolve_module_alias_enum_candidate(
                                ns,
                                symbol_name,
                                member_parts,
                                global_enum_map,
                            )
                        {
                            return Expr::Field {
                                object: Box::new(ast::Spanned::new(
                                    Expr::Ident(mangle_project_symbol(
                                        &owner_ns,
                                        entry_namespace,
                                        &enum_name,
                                    )),
                                    object.span.clone(),
                                )),
                                field: variant_name,
                            };
                        }
                        if let Some((owner_ns, class_name)) = resolve_module_alias_class_candidate(
                            ns,
                            symbol_name,
                            member_parts,
                            global_class_map,
                        ) {
                            return Expr::Ident(mangle_project_symbol(
                                &owner_ns,
                                entry_namespace,
                                &class_name,
                            ));
                        }
                        let field = member_parts.last().expect("non-empty member parts");
                        let namespace_path = if symbol_name.is_empty() {
                            ns.clone()
                        } else {
                            format!("{}.{}", ns, symbol_name)
                        };
                        if let Some(canonical) =
                            stdlib_registry().resolve_alias_call(&namespace_path, field)
                        {
                            return if let Some((owner, method)) = canonical.split_once("__") {
                                Expr::Field {
                                    object: Box::new(ast::Spanned::new(
                                        Expr::Ident(owner.to_string()),
                                        object.span.clone(),
                                    )),
                                    field: method.to_string(),
                                }
                            } else {
                                Expr::Ident(canonical)
                            };
                        }

                        if symbol_name.is_empty() && member_parts.len() == 1 {
                            if let Some(owner_ns) = global_function_map.get(field) {
                                if owner_ns == ns {
                                    return Expr::Ident(mangle_project_function_symbol(
                                        owner_ns,
                                        entry_namespace,
                                        field,
                                    ));
                                }
                            }
                        }
                        if let Some((owner_ns, candidate)) = resolve_module_alias_function_candidate(
                            ns,
                            symbol_name,
                            member_parts,
                            global_function_map,
                        ) {
                            return Expr::Ident(mangle_project_function_symbol(
                                &owner_ns,
                                entry_namespace,
                                &candidate,
                            ));
                        }
                    }
                }
            }

            if let Expr::Ident(module_alias) = &object.node {
                if !is_shadowed(module_alias, scopes) {
                    if local_modules.contains(module_alias) {
                        if let Some((owner_ns, enum_name)) =
                            resolve_module_alias_enum_type_candidate(
                                current_namespace,
                                module_alias,
                                std::slice::from_ref(field),
                                global_enum_map,
                            )
                        {
                            return Expr::Ident(mangle_project_symbol(
                                &owner_ns,
                                entry_namespace,
                                &enum_name,
                            ));
                        }
                        if let Some((owner_ns, class_name)) = resolve_module_alias_class_candidate(
                            current_namespace,
                            module_alias,
                            std::slice::from_ref(field),
                            global_class_map,
                        ) {
                            return Expr::Ident(mangle_project_symbol(
                                &owner_ns,
                                entry_namespace,
                                &class_name,
                            ));
                        }
                        if let Some((owner_ns, candidate)) = resolve_module_alias_function_candidate(
                            current_namespace,
                            module_alias,
                            std::slice::from_ref(field),
                            global_function_map,
                        ) {
                            return Expr::Ident(mangle_project_function_symbol(
                                &owner_ns,
                                entry_namespace,
                                &candidate,
                            ));
                        }
                    }
                    if let Some((ns, symbol_name)) = imported_modules.get(module_alias) {
                        if let Some((owner_ns, enum_name)) = imported_enums.get(module_alias) {
                            if field == enum_name {
                                return Expr::Ident(mangle_project_symbol(
                                    owner_ns,
                                    entry_namespace,
                                    enum_name,
                                ));
                            }
                        }
                        if let Some((owner_ns, class_name)) = imported_classes.get(module_alias) {
                            if field == class_name {
                                return Expr::Ident(mangle_project_symbol(
                                    owner_ns,
                                    entry_namespace,
                                    class_name,
                                ));
                            }
                        }
                        let namespace_path = if symbol_name.is_empty() {
                            ns.clone()
                        } else {
                            format!("{}.{}", ns, symbol_name)
                        };
                        if let Some(canonical) =
                            stdlib_registry().resolve_alias_call(&namespace_path, field)
                        {
                            return if let Some((owner, method)) = canonical.split_once("__") {
                                Expr::Field {
                                    object: Box::new(ast::Spanned::new(
                                        Expr::Ident(owner.to_string()),
                                        object.span.clone(),
                                    )),
                                    field: method.to_string(),
                                }
                            } else {
                                Expr::Ident(canonical)
                            };
                        }

                        if symbol_name.is_empty() {
                            if let Some(owner_ns) = global_function_map.get(field) {
                                if owner_ns == ns {
                                    return Expr::Ident(mangle_project_function_symbol(
                                        owner_ns,
                                        entry_namespace,
                                        field,
                                    ));
                                }
                            }
                        }
                        if let Some((owner_ns, candidate)) = resolve_module_alias_function_candidate(
                            ns,
                            symbol_name,
                            std::slice::from_ref(field),
                            global_function_map,
                        ) {
                            return Expr::Ident(mangle_project_symbol(
                                &owner_ns,
                                entry_namespace,
                                &candidate,
                            ));
                        }
                    }
                }
            }

            let rewritten_object = match &object.node {
                Expr::Ident(name) if !is_shadowed(name, scopes) => {
                    if global_enum_map
                        .get(name)
                        .is_some_and(|owner_ns| owner_ns == current_namespace)
                    {
                        Expr::Ident(mangle_project_symbol(
                            current_namespace,
                            entry_namespace,
                            name,
                        ))
                    } else if let Some((ns, symbol_name)) = imported_enums.get(name) {
                        Expr::Ident(mangle_project_symbol(ns, entry_namespace, symbol_name))
                    } else if let Some(ns) = global_enum_map.get(name) {
                        Expr::Ident(mangle_project_symbol(ns, entry_namespace, name))
                    } else if local_modules.contains(name) {
                        Expr::Ident(mangle_project_symbol(
                            current_namespace,
                            entry_namespace,
                            name,
                        ))
                    } else if let Some((ns, symbol_name)) = imported_modules.get(name) {
                        if symbol_name.is_empty() {
                            Expr::Ident(name.clone())
                        } else {
                            Expr::Ident(mangle_project_symbol(ns, entry_namespace, symbol_name))
                        }
                    } else if let Some(ns) = global_module_map.get(name) {
                        Expr::Ident(mangle_project_symbol(ns, entry_namespace, name))
                    } else {
                        Expr::Ident(name.clone())
                    }
                }
                _ => rewrite_expr_calls_for_project(
                    &object.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
            };
            Expr::Field {
                object: Box::new(ast::Spanned::new(rewritten_object, object.span.clone())),
                field: field.clone(),
            }
        }
        Expr::Index { object, index } => Expr::Index {
            object: Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &object.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                object.span.clone(),
            )),
            index: Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &index.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                index.span.clone(),
            )),
        },
        Expr::Construct { ty, args } => {
            if let Some((import_ns, symbol_name)) = imported_modules.get(ty) {
                if let Some((owner_ns, enum_name, variant_name)) =
                    resolve_exact_imported_variant_alias(import_ns, symbol_name, global_enum_map)
                {
                    return Expr::Call {
                        callee: Box::new(ast::Spanned::new(
                            Expr::Field {
                                object: Box::new(ast::Spanned::new(
                                    Expr::Ident(mangle_project_symbol(
                                        &owner_ns,
                                        entry_namespace,
                                        &enum_name,
                                    )),
                                    ast::Span::default(),
                                )),
                                field: variant_name,
                            },
                            ast::Span::default(),
                        )),
                        args: args
                            .iter()
                            .map(|a| {
                                ast::Spanned::new(
                                    rewrite_expr_calls_for_project(
                                        &a.node,
                                        current_namespace,
                                        entry_namespace,
                                        local_functions,
                                        imported_map,
                                        global_function_map,
                                        local_classes,
                                        imported_classes,
                                        global_class_map,
                                        local_interfaces,
                                        imported_interfaces,
                                        global_interface_map,
                                        imported_enums,
                                        global_enum_map,
                                        local_modules,
                                        imported_modules,
                                        global_module_map,
                                        scopes,
                                    ),
                                    a.span.clone(),
                                )
                            })
                            .collect(),
                        type_args: vec![],
                    };
                }
            }

            Expr::Construct {
                ty: rewrite_construct_type_name_for_project(
                    ty,
                    current_namespace,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    &collect_local_enum_names(global_enum_map, current_namespace),
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    entry_namespace,
                ),
                args: args
                    .iter()
                    .map(|a| {
                        ast::Spanned::new(
                            rewrite_expr_calls_for_project(
                                &a.node,
                                current_namespace,
                                entry_namespace,
                                local_functions,
                                imported_map,
                                global_function_map,
                                local_classes,
                                imported_classes,
                                global_class_map,
                                local_interfaces,
                                imported_interfaces,
                                global_interface_map,
                                imported_enums,
                                global_enum_map,
                                local_modules,
                                imported_modules,
                                global_module_map,
                                scopes,
                            ),
                            a.span.clone(),
                        )
                    })
                    .collect(),
            }
        }
        Expr::GenericFunctionValue { callee, type_args } => Expr::GenericFunctionValue {
            callee: Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &callee.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                callee.span.clone(),
            )),
            type_args: type_args
                .iter()
                .map(|ty| {
                    rewrite_type_for_project_with_interfaces(
                        ty,
                        current_namespace,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        local_interfaces,
                        imported_interfaces,
                        global_interface_map,
                        &collect_local_enum_names(global_enum_map, current_namespace),
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        entry_namespace,
                    )
                })
                .collect(),
        },
        Expr::Lambda { params, body } => {
            push_scope(scopes);
            if let Some(scope) = scopes.last_mut() {
                for param in params {
                    scope.insert(param.name.clone());
                }
            }
            let rewritten_body = rewrite_expr_calls_for_project(
                &body.node,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_interfaces,
                imported_interfaces,
                global_interface_map,
                imported_enums,
                global_enum_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            );
            pop_scope(scopes);
            Expr::Lambda {
                params: params
                    .iter()
                    .map(|p| ast::Parameter {
                        name: p.name.clone(),
                        ty: rewrite_type_for_project_with_interfaces(
                            &p.ty,
                            current_namespace,
                            local_classes,
                            imported_classes,
                            global_class_map,
                            local_interfaces,
                            imported_interfaces,
                            global_interface_map,
                            &collect_local_enum_names(global_enum_map, current_namespace),
                            imported_enums,
                            global_enum_map,
                            imported_modules,
                            entry_namespace,
                        ),
                        mutable: p.mutable,
                        mode: p.mode,
                    })
                    .collect(),
                body: Box::new(ast::Spanned::new(rewritten_body, body.span.clone())),
            }
        }
        Expr::Block(stmts) => Expr::Block(rewrite_block_calls_for_project(
            stmts,
            current_namespace,
            entry_namespace,
            local_functions,
            imported_map,
            global_function_map,
            local_classes,
            imported_classes,
            global_class_map,
            local_interfaces,
            imported_interfaces,
            global_interface_map,
            imported_enums,
            global_enum_map,
            local_modules,
            imported_modules,
            global_module_map,
            scopes,
        )),
        Expr::IfExpr {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition = Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &condition.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                condition.span.clone(),
            ));
            push_scope(scopes);
            let then_branch = rewrite_block_calls_for_project(
                then_branch,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_interfaces,
                imported_interfaces,
                global_interface_map,
                imported_enums,
                global_enum_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            );
            pop_scope(scopes);
            let else_branch = else_branch.as_ref().map(|branch| {
                push_scope(scopes);
                let rewritten = rewrite_block_calls_for_project(
                    branch,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                );
                pop_scope(scopes);
                rewritten
            });
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            }
        }
        Expr::Match { expr, arms } => Expr::Match {
            expr: Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &expr.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                expr.span.clone(),
            )),
            arms: arms
                .iter()
                .map(|arm| {
                    let rewritten_pattern = rewrite_pattern_for_project(
                        &arm.pattern,
                        current_namespace,
                        entry_namespace,
                        local_modules,
                        imported_modules,
                        global_enum_map,
                    );
                    push_scope(scopes);
                    if let Some(scope) = scopes.last_mut() {
                        bind_pattern_locals(&rewritten_pattern, scope);
                    }
                    let body = rewrite_block_calls_for_project(
                        &arm.body,
                        current_namespace,
                        entry_namespace,
                        local_functions,
                        imported_map,
                        global_function_map,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        local_interfaces,
                        imported_interfaces,
                        global_interface_map,
                        imported_enums,
                        global_enum_map,
                        local_modules,
                        imported_modules,
                        global_module_map,
                        scopes,
                    );
                    pop_scope(scopes);
                    ast::MatchArm {
                        pattern: rewritten_pattern,
                        body,
                    }
                })
                .collect(),
        },
        Expr::Await(inner) => Expr::Await(Box::new(ast::Spanned::new(
            rewrite_expr_calls_for_project(
                &inner.node,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_interfaces,
                imported_interfaces,
                global_interface_map,
                imported_enums,
                global_enum_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            ),
            inner.span.clone(),
        ))),
        Expr::Try(inner) => Expr::Try(Box::new(ast::Spanned::new(
            rewrite_expr_calls_for_project(
                &inner.node,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_interfaces,
                imported_interfaces,
                global_interface_map,
                imported_enums,
                global_enum_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            ),
            inner.span.clone(),
        ))),
        Expr::Borrow(inner) => Expr::Borrow(Box::new(ast::Spanned::new(
            rewrite_expr_calls_for_project(
                &inner.node,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_interfaces,
                imported_interfaces,
                global_interface_map,
                imported_enums,
                global_enum_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            ),
            inner.span.clone(),
        ))),
        Expr::MutBorrow(inner) => Expr::MutBorrow(Box::new(ast::Spanned::new(
            rewrite_expr_calls_for_project(
                &inner.node,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_interfaces,
                imported_interfaces,
                global_interface_map,
                imported_enums,
                global_enum_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            ),
            inner.span.clone(),
        ))),
        Expr::Deref(inner) => Expr::Deref(Box::new(ast::Spanned::new(
            rewrite_expr_calls_for_project(
                &inner.node,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_interfaces,
                imported_interfaces,
                global_interface_map,
                imported_enums,
                global_enum_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            ),
            inner.span.clone(),
        ))),
        Expr::StringInterp(parts) => Expr::StringInterp(
            parts
                .iter()
                .map(|part| match part {
                    ast::StringPart::Literal(text) => ast::StringPart::Literal(text.clone()),
                    ast::StringPart::Expr(expr) => ast::StringPart::Expr(ast::Spanned::new(
                        rewrite_expr_calls_for_project(
                            &expr.node,
                            current_namespace,
                            entry_namespace,
                            local_functions,
                            imported_map,
                            global_function_map,
                            local_classes,
                            imported_classes,
                            global_class_map,
                            local_interfaces,
                            imported_interfaces,
                            global_interface_map,
                            imported_enums,
                            global_enum_map,
                            local_modules,
                            imported_modules,
                            global_module_map,
                            scopes,
                        ),
                        expr.span.clone(),
                    )),
                })
                .collect(),
        ),
        Expr::AsyncBlock(body) => Expr::AsyncBlock(rewrite_block_calls_for_project(
            body,
            current_namespace,
            entry_namespace,
            local_functions,
            imported_map,
            global_function_map,
            local_classes,
            imported_classes,
            global_class_map,
            local_interfaces,
            imported_interfaces,
            global_interface_map,
            imported_enums,
            global_enum_map,
            local_modules,
            imported_modules,
            global_module_map,
            scopes,
        )),
        Expr::Require { condition, message } => Expr::Require {
            condition: Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &condition.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces,
                    global_interface_map,
                    imported_enums,
                    global_enum_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                condition.span.clone(),
            )),
            message: message.as_ref().map(|expr| {
                Box::new(ast::Spanned::new(
                    rewrite_expr_calls_for_project(
                        &expr.node,
                        current_namespace,
                        entry_namespace,
                        local_functions,
                        imported_map,
                        global_function_map,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        local_interfaces,
                        imported_interfaces,
                        global_interface_map,
                        imported_enums,
                        global_enum_map,
                        local_modules,
                        imported_modules,
                        global_module_map,
                        scopes,
                    ),
                    expr.span.clone(),
                ))
            }),
        },
        Expr::Range {
            start,
            end,
            inclusive,
        } => Expr::Range {
            start: start.as_ref().map(|expr| {
                Box::new(ast::Spanned::new(
                    rewrite_expr_calls_for_project(
                        &expr.node,
                        current_namespace,
                        entry_namespace,
                        local_functions,
                        imported_map,
                        global_function_map,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        local_interfaces,
                        imported_interfaces,
                        global_interface_map,
                        imported_enums,
                        global_enum_map,
                        local_modules,
                        imported_modules,
                        global_module_map,
                        scopes,
                    ),
                    expr.span.clone(),
                ))
            }),
            end: end.as_ref().map(|expr| {
                Box::new(ast::Spanned::new(
                    rewrite_expr_calls_for_project(
                        &expr.node,
                        current_namespace,
                        entry_namespace,
                        local_functions,
                        imported_map,
                        global_function_map,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        local_interfaces,
                        imported_interfaces,
                        global_interface_map,
                        imported_enums,
                        global_enum_map,
                        local_modules,
                        imported_modules,
                        global_module_map,
                        scopes,
                    ),
                    expr.span.clone(),
                ))
            }),
            inclusive: *inclusive,
        },
        Expr::This => Expr::This,
        Expr::Ident(name) => {
            if is_shadowed(name, scopes) {
                Expr::Ident(name.clone())
            } else if local_functions.contains(name) {
                Expr::Ident(mangle_project_symbol(
                    current_namespace,
                    entry_namespace,
                    name,
                ))
            } else if let Some((ns, symbol_name)) = imported_map.get(name) {
                if stdlib_registry()
                    .get_namespace(symbol_name)
                    .is_some_and(|owner| owner == ns)
                {
                    Expr::Ident(symbol_name.clone())
                } else {
                    Expr::Ident(mangle_project_function_symbol(
                        ns,
                        entry_namespace,
                        symbol_name,
                    ))
                }
            } else if let Some((import_ns, symbol_name)) = imported_modules.get(name) {
                if let Some((owner_ns, enum_name, variant_name)) =
                    resolve_exact_imported_variant_alias(import_ns, symbol_name, global_enum_map)
                {
                    Expr::Field {
                        object: Box::new(ast::Spanned::new(
                            Expr::Ident(mangle_project_symbol(
                                &owner_ns,
                                entry_namespace,
                                &enum_name,
                            )),
                            ast::Span::default(),
                        )),
                        field: variant_name,
                    }
                } else {
                    Expr::Ident(name.clone())
                }
            } else if let Some(ns) = global_function_map.get(name) {
                Expr::Ident(mangle_project_function_symbol(ns, entry_namespace, name))
            } else {
                Expr::Ident(name.clone())
            }
        }
        _ => expr.clone(),
    };
    REWRITE_INTERNAL_TIMING_TOTALS
        .expr_rewrite_ns
        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
    rewritten
}

#[cfg(test)]
mod tests;
