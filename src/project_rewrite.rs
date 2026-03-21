use std::collections::{HashMap, HashSet};

use crate::ast::{self, Decl, Expr, ImportDecl, Program, Stmt};
use crate::parser::parse_type_source;
use crate::stdlib::stdlib_registry;

type ImportedMap = HashMap<String, (String, String)>;

struct RewriteTypeContext<'a> {
    current_namespace: &'a str,
    local_classes: &'a HashSet<String>,
    imported_classes: &'a ImportedMap,
    global_class_map: &'a HashMap<String, String>,
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
    if global_symbol_map
        .get(symbol_name)
        .is_some_and(|owner_ns| owner_ns == namespace_path)
    {
        return Some((namespace_path.to_string(), symbol_name.to_string()));
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
    (matches.len() == 1).then(|| matches.swap_remove(0))
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

#[allow(clippy::too_many_arguments)]
fn rewrite_construct_type_name_for_project(
    ty: &str,
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
    let ctx = RewriteTypeContext {
        current_namespace,
        local_classes,
        imported_classes,
        global_class_map,
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
pub fn rewrite_program_for_project(
    program: &Program,
    current_namespace: &str,
    entry_namespace: &str,
    namespace_functions: &HashMap<String, HashSet<String>>,
    global_function_map: &HashMap<String, String>,
    namespace_classes: &HashMap<String, HashSet<String>>,
    global_class_map: &HashMap<String, String>,
    namespace_enums: &HashMap<String, HashSet<String>>,
    global_enum_map: &HashMap<String, String>,
    namespace_modules: &HashMap<String, HashSet<String>>,
    global_module_map: &HashMap<String, String>,
    imports: &[ImportDecl],
) -> Program {
    let local_functions = namespace_functions
        .get(current_namespace)
        .cloned()
        .unwrap_or_default();
    let local_classes = namespace_classes
        .get(current_namespace)
        .cloned()
        .unwrap_or_default();
    let local_enums = namespace_enums
        .get(current_namespace)
        .cloned()
        .unwrap_or_default();
    let local_modules = namespace_modules
        .get(current_namespace)
        .cloned()
        .unwrap_or_default();

    let mut imported_map: ImportedMap = HashMap::new();
    let mut imported_classes: ImportedMap = HashMap::new();
    let mut imported_enums: ImportedMap = HashMap::new();
    let mut imported_modules: ImportedMap = HashMap::new();
    for import in imports {
        let import_key = import
            .alias
            .as_ref()
            .cloned()
            .unwrap_or_else(|| import.path.rsplit('.').next().unwrap_or("").to_string());
        if import.path.ends_with(".*") {
            let ns = import.path.trim_end_matches(".*");
            if let Some(funcs) = namespace_functions.get(ns) {
                for name in funcs {
                    imported_map.insert(name.clone(), (ns.to_string(), name.clone()));
                }
            }
            if let Some(classes) = namespace_classes.get(ns) {
                for name in classes {
                    imported_classes.insert(name.clone(), (ns.to_string(), name.clone()));
                }
            }
            if let Some(enums) = namespace_enums.get(ns) {
                for name in enums {
                    imported_enums.insert(name.clone(), (ns.to_string(), name.clone()));
                }
            }
            if let Some(modules) = namespace_modules.get(ns) {
                for name in modules {
                    imported_modules.insert(name.clone(), (ns.to_string(), name.clone()));
                }
            }
        } else if import.path.contains('.') {
            let mut parts = import.path.split('.').collect::<Vec<_>>();
            if let Some(source_name) = parts.pop() {
                let ns = parts.join(".");
                if let Some((owner_ns, function_name)) =
                    resolve_exact_imported_symbol_path(&ns, source_name, global_function_map)
                {
                    imported_map.insert(import_key.clone(), (owner_ns, function_name));
                } else {
                    imported_map.insert(import_key.clone(), (ns.clone(), source_name.to_string()));
                }
                if global_class_map
                    .get(source_name)
                    .is_some_and(|owner_ns| owner_ns == &ns)
                {
                    imported_classes
                        .insert(import_key.clone(), (ns.clone(), source_name.to_string()));
                } else if let Some((owner_ns, class_name)) =
                    resolve_exact_imported_symbol_path(&ns, source_name, global_class_map)
                {
                    imported_classes.insert(import_key.clone(), (owner_ns, class_name));
                }
                if global_enum_map
                    .get(source_name)
                    .is_some_and(|owner_ns| owner_ns == &ns)
                {
                    imported_enums
                        .insert(import_key.clone(), (ns.clone(), source_name.to_string()));
                } else if let Some((owner_ns, enum_name)) =
                    resolve_exact_imported_symbol_path(&ns, source_name, global_enum_map)
                {
                    imported_enums.insert(import_key.clone(), (owner_ns, enum_name));
                }
                imported_modules.insert(import_key, (ns, source_name.to_string()));
            }
        } else if namespace_functions.contains_key(&import.path)
            || namespace_classes.contains_key(&import.path)
            || namespace_enums.contains_key(&import.path)
            || namespace_modules.contains_key(&import.path)
        {
            // Namespace import without explicit symbol (e.g. `import math_utils as mu`)
            // should allow `mu.someFunction()` rewrite resolution.
            imported_modules.insert(import_key.clone(), (import.path.clone(), String::new()));
            for (symbol_name, owner_ns) in global_class_map {
                if owner_ns == &import.path {
                    imported_classes.insert(
                        alias_qualified_symbol_name(&import_key, symbol_name),
                        (import.path.clone(), symbol_name.clone()),
                    );
                }
            }
            for (symbol_name, owner_ns) in global_enum_map {
                if owner_ns == &import.path {
                    imported_enums.insert(
                        alias_qualified_symbol_name(&import_key, symbol_name),
                        (import.path.clone(), symbol_name.clone()),
                    );
                }
            }
        }
    }

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
                        let mut scopes = vec![f.params.iter().map(|p| p.name.clone()).collect()];
                        f.params = f
                            .params
                            .iter()
                            .map(|p| ast::Parameter {
                                name: p.name.clone(),
                                ty: rewrite_type_for_project(
                                    &p.ty,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
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
                        f.return_type = rewrite_type_for_project(
                            &f.return_type,
                            current_namespace,
                            &local_classes,
                            &imported_classes,
                            global_class_map,
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
                            &imported_enums,
                            global_enum_map,
                            &local_modules,
                            &imported_modules,
                            global_module_map,
                            &mut scopes,
                        );
                        f.name = mangle_project_symbol(current_namespace, entry_namespace, &f.name);
                        Decl::Function(f)
                    }
                    Decl::Class(class) => {
                        let mut c = class.clone();
                        c.name = mangle_project_symbol(current_namespace, entry_namespace, &c.name);
                        c.fields = c
                            .fields
                            .iter()
                            .map(|field| ast::Field {
                                name: field.name.clone(),
                                ty: rewrite_type_for_project(
                                    &field.ty,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
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
                                    ty: rewrite_type_for_project(
                                        &p.ty,
                                        current_namespace,
                                        &local_classes,
                                        &imported_classes,
                                        global_class_map,
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
                                &imported_enums,
                                global_enum_map,
                                &local_modules,
                                &imported_modules,
                                global_module_map,
                                &mut scopes,
                            );
                            c.constructor = Some(new_ctor);
                        }
                        c.methods = class
                            .methods
                            .iter()
                            .map(|m| {
                                let mut nm = m.clone();
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
                                        ty: rewrite_type_for_project(
                                            &p.ty,
                                            current_namespace,
                                            &local_classes,
                                            &imported_classes,
                                            global_class_map,
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
                                nm.return_type = rewrite_type_for_project(
                                    &nm.return_type,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
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
                                                    &module_local_enums,
                                                    &module_local_modules,
                                                    &imported_classes,
                                                    global_class_map,
                                                    &imported_enums,
                                                    global_enum_map,
                                                    &imported_modules,
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
                                            &module_local_enums,
                                            &module_local_modules,
                                            &imported_classes,
                                            global_class_map,
                                            &imported_enums,
                                            global_enum_map,
                                            &imported_modules,
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
                                        );
                                        Decl::Function(f)
                                    }
                                    Decl::Class(class) => {
                                        let mut c = class.clone();
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
                                                    &module_local_enums,
                                                    &module_local_modules,
                                                    &imported_classes,
                                                    global_class_map,
                                                    &imported_enums,
                                                    global_enum_map,
                                                    &imported_modules,
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
                                                        &module_local_enums,
                                                        &module_local_modules,
                                                        &imported_classes,
                                                        global_class_map,
                                                        &imported_enums,
                                                        global_enum_map,
                                                        &imported_modules,
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
                                            );
                                            c.constructor = Some(new_ctor);
                                        }
                                        c.methods = class
                                            .methods
                                            .iter()
                                            .map(|method| {
                                                let mut nm = method.clone();
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
                                                            &module_local_enums,
                                                            &module_local_modules,
                                                            &imported_classes,
                                                            global_class_map,
                                                            &imported_enums,
                                                            global_enum_map,
                                                            &imported_modules,
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
                                                    &module_local_enums,
                                                    &module_local_modules,
                                                    &imported_classes,
                                                    global_class_map,
                                                    &imported_enums,
                                                    global_enum_map,
                                                    &imported_modules,
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
                                                );
                                                nm
                                            })
                                            .collect();
                                        Decl::Class(c)
                                    }
                                    Decl::Module(module) => {
                                        let nested_prefix =
                                            module_prefixed_symbol(&module_prefix, &module.name);
                                        let (
                                            nested_local_functions,
                                            nested_local_classes,
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
                                            &nested_local_enums,
                                            &nested_local_modules,
                                            &imported_map,
                                            global_function_map,
                                            &imported_classes,
                                            global_class_map,
                                            &imported_enums,
                                            global_enum_map,
                                            &imported_modules,
                                            global_module_map,
                                        )
                                        .node
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
                                        ty: rewrite_type_for_project(
                                            &f.ty,
                                            current_namespace,
                                            &local_classes,
                                            &imported_classes,
                                            global_class_map,
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
                    _ => d.node.clone(),
                };
                ast::Spanned::new(node, d.span.clone())
            })
            .collect(),
    }
}

fn mangle_project_symbol(namespace: &str, entry_namespace: &str, name: &str) -> String {
    if name == "main" && namespace == entry_namespace {
        "main".to_string()
    } else {
        format!("{}__{}", namespace.replace('.', "__"), name)
    }
}

fn rewrite_type_for_project_with_ctx(ty: &ast::Type, ctx: &RewriteTypeContext<'_>) -> ast::Type {
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

    match ty {
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
    }
}

#[allow(clippy::too_many_arguments)]
fn rewrite_type_for_project(
    ty: &ast::Type,
    current_namespace: &str,
    local_classes: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
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
        local_enums,
        imported_enums,
        global_enum_map,
        imported_modules,
        entry_namespace,
    };
    rewrite_type_for_project_with_ctx(ty, &ctx)
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

    let full_parts = if symbol_name.is_empty() {
        member_parts.to_vec()
    } else {
        let mut parts = vec![symbol_name.to_string()];
        parts.extend_from_slice(member_parts);
        parts
    };
    let candidate = full_parts.join("__");

    global_class_map
        .get(&candidate)
        .map(|owner_ns| (owner_ns.clone(), candidate))
        .filter(|(owner_ns, _)| owner_ns == import_ns)
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
    let enum_name = if symbol_name.is_empty() {
        enum_parts.join("__")
    } else {
        format!("{}__{}", symbol_name, enum_parts.join("__"))
    };
    global_enum_map
        .get(&enum_name)
        .filter(|owner_ns| *owner_ns == import_ns)
        .map(|owner_ns| (owner_ns.clone(), enum_name, variant_name[0].clone()))
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

    let enum_name = if symbol_name.is_empty() {
        member_parts.join("__")
    } else {
        format!("{}__{}", symbol_name, member_parts.join("__"))
    };
    global_enum_map
        .get(&enum_name)
        .filter(|owner_ns| *owner_ns == import_ns)
        .map(|owner_ns| (owner_ns.clone(), enum_name))
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
                if local_modules.contains(module_alias) {
                    let member_parts = rest
                        .split('.')
                        .map(|part| part.to_string())
                        .collect::<Vec<_>>();
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

fn collect_direct_module_symbol_names(
    declarations: &[ast::Spanned<Decl>],
) -> (
    HashSet<String>,
    HashSet<String>,
    HashSet<String>,
    HashSet<String>,
) {
    let mut functions = HashSet::new();
    let mut classes = HashSet::new();
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
            Decl::Enum(en) => {
                enums.insert(en.name.clone());
            }
            Decl::Module(module) => {
                modules.insert(module.name.clone());
            }
            Decl::Interface(_) | Decl::Import(_) => {}
        }
    }

    (functions, classes, enums, modules)
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
    module_local_enums: &HashSet<String>,
    module_local_modules: &HashSet<String>,
    imported_map: &ImportedMap,
    global_function_map: &HashMap<String, String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &ImportedMap,
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
                        module_local_enums,
                        module_local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
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
                module_local_enums,
                module_local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
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
            );
            Decl::Function(f)
        }
        Decl::Class(class) => {
            let mut c = class.clone();
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
                        module_local_enums,
                        module_local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
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
                            module_local_enums,
                            module_local_modules,
                            imported_classes,
                            global_class_map,
                            imported_enums,
                            global_enum_map,
                            imported_modules,
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
                                module_local_enums,
                                module_local_modules,
                                imported_classes,
                                global_class_map,
                                imported_enums,
                                global_enum_map,
                                imported_modules,
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
                        module_local_enums,
                        module_local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
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
                                module_local_enums,
                                module_local_modules,
                                imported_classes,
                                global_class_map,
                                imported_enums,
                                global_enum_map,
                                imported_modules,
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
                        &nested_local_enums,
                        &nested_local_modules,
                        imported_map,
                        global_function_map,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        global_module_map,
                    )
                })
                .collect();
            Decl::Module(nested)
        }
        Decl::Interface(_) | Decl::Import(_) => decl.node.clone(),
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

fn fix_module_local_expr(
    expr: &Expr,
    current_namespace: &str,
    entry_namespace: &str,
    module_prefix: &str,
    local_functions: &HashSet<String>,
    local_classes: &HashSet<String>,
) -> Expr {
    match expr {
        Expr::Ident(name) => remap_module_local_mangled_name(
            name,
            current_namespace,
            entry_namespace,
            module_prefix,
            local_functions,
        )
        .map_or_else(|| Expr::Ident(name.clone()), Expr::Ident),
        Expr::Call {
            callee,
            args,
            type_args,
        } => {
            let module_name =
                mangle_project_symbol(current_namespace, entry_namespace, module_prefix);
            if let Some(parts) = flatten_field_chain(&callee.node) {
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
                                        ),
                                        arg.span.clone(),
                                    )
                                })
                                .collect(),
                            type_args: type_args.clone(),
                        };
                    }
                    if local_classes.contains(member) {
                        return Expr::Construct {
                            ty: format_construct_type_name(
                                &mangle_project_symbol(
                                    current_namespace,
                                    entry_namespace,
                                    &module_prefixed_symbol(module_prefix, member),
                                ),
                                type_args,
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
                            ),
                            arg.span.clone(),
                        )
                    })
                    .collect(),
                type_args: type_args.clone(),
            }
        }
        Expr::Construct { ty, args } => {
            let ty = remap_module_local_mangled_name(
                ty,
                current_namespace,
                entry_namespace,
                module_prefix,
                local_classes,
            )
            .unwrap_or_else(|| ty.clone());
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
                            ),
                            arg.span.clone(),
                        )
                    })
                    .collect(),
            }
        }
        Expr::Field { object, field } => Expr::Field {
            object: Box::new(ast::Spanned::new(
                fix_module_local_expr(
                    &object.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
                ),
                object.span.clone(),
            )),
            field: field.clone(),
        },
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
            ),
            else_branch: else_branch.as_ref().map(|block| {
                fix_module_local_block(
                    block,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
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
        )),
        Expr::AsyncBlock(body) => Expr::AsyncBlock(fix_module_local_block(
            body,
            current_namespace,
            entry_namespace,
            module_prefix,
            local_functions,
            local_classes,
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
                ),
                expr.span.clone(),
            )),
            arms: arms
                .iter()
                .map(|arm| ast::MatchArm {
                    pattern: arm.pattern.clone(),
                    body: fix_module_local_block(
                        &arm.body,
                        current_namespace,
                        entry_namespace,
                        module_prefix,
                        local_functions,
                        local_classes,
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
            ),
            inner.span.clone(),
        ))),
        Expr::Lambda { params, body } => Expr::Lambda {
            params: params.clone(),
            body: Box::new(ast::Spanned::new(
                fix_module_local_expr(
                    &body.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
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
                        ),
                        expr.span.clone(),
                    )),
                })
                .collect(),
        ),
        _ => expr.clone(),
    }
}

fn fix_module_local_stmt(
    stmt: &Stmt,
    current_namespace: &str,
    entry_namespace: &str,
    module_prefix: &str,
    local_functions: &HashSet<String>,
    local_classes: &HashSet<String>,
) -> Stmt {
    match stmt {
        Stmt::Let {
            name,
            ty,
            value,
            mutable,
        } => Stmt::Let {
            name: name.clone(),
            ty: ty.clone(),
            value: ast::Spanned::new(
                fix_module_local_expr(
                    &value.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
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
            ),
            else_block: else_block.as_ref().map(|block| {
                fix_module_local_block(
                    block,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
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
            ),
        },
        Stmt::For {
            var,
            var_type,
            iterable,
            body,
        } => Stmt::For {
            var: var.clone(),
            var_type: var_type.clone(),
            iterable: ast::Spanned::new(
                fix_module_local_expr(
                    &iterable.node,
                    current_namespace,
                    entry_namespace,
                    module_prefix,
                    local_functions,
                    local_classes,
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
                ),
                expr.span.clone(),
            ),
            arms: arms
                .iter()
                .map(|arm| ast::MatchArm {
                    pattern: arm.pattern.clone(),
                    body: fix_module_local_block(
                        &arm.body,
                        current_namespace,
                        entry_namespace,
                        module_prefix,
                        local_functions,
                        local_classes,
                    ),
                })
                .collect(),
        },
        _ => stmt.clone(),
    }
}

fn fix_module_local_block(
    block: &ast::Block,
    current_namespace: &str,
    entry_namespace: &str,
    module_prefix: &str,
    local_functions: &HashSet<String>,
    local_classes: &HashSet<String>,
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
    local_enums: &HashSet<String>,
    local_modules: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &ImportedMap,
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
            if local_classes.contains(name) || local_enums.contains(name) {
                ast::Type::Named(mangle_project_symbol(
                    current_namespace,
                    entry_namespace,
                    &module_prefixed_symbol(module_prefix, name),
                ))
            } else if let Some((head, _tail)) = name.split_once('.') {
                if local_modules.contains(head) {
                    ast::Type::Named(mangle_project_symbol(
                        current_namespace,
                        entry_namespace,
                        &module_prefixed_symbol(module_prefix, &name.replace('.', "__")),
                    ))
                } else {
                    rewrite_type_for_project(
                        ty,
                        current_namespace,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        local_enums,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                        entry_namespace,
                    )
                }
            } else {
                rewrite_type_for_project(
                    ty,
                    current_namespace,
                    local_classes,
                    imported_classes,
                    global_class_map,
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
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
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
                        local_enums,
                        local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
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
                        local_enums,
                        local_modules,
                        imported_classes,
                        global_class_map,
                        imported_enums,
                        global_enum_map,
                        imported_modules,
                    )
                })
                .collect(),
            Box::new(rewrite_module_local_type(
                ret,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
            )),
        ),
        ast::Type::Option(inner) => {
            let rewritten_inner = rewrite_module_local_type(
                inner,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
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
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
            );
            let rewritten_err = rewrite_module_local_type(
                err,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
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
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
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
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
            );
            let rewritten_value = rewrite_module_local_type(
                v,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
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
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
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
            local_enums,
            local_modules,
            imported_classes,
            global_class_map,
            imported_enums,
            global_enum_map,
            imported_modules,
        ))),
        ast::Type::MutRef(inner) => ast::Type::MutRef(Box::new(rewrite_module_local_type(
            inner,
            module_prefix,
            current_namespace,
            entry_namespace,
            local_classes,
            local_enums,
            local_modules,
            imported_classes,
            global_class_map,
            imported_enums,
            global_enum_map,
            imported_modules,
        ))),
        ast::Type::Box(inner) => {
            let rewritten_inner = rewrite_module_local_type(
                inner,
                module_prefix,
                current_namespace,
                entry_namespace,
                local_classes,
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
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
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
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
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
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
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
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
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
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
                local_enums,
                local_modules,
                imported_classes,
                global_class_map,
                imported_enums,
                global_enum_map,
                imported_modules,
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
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    local_modules: &HashSet<String>,
    imported_modules: &ImportedMap,
    global_module_map: &HashMap<String, String>,
    scopes: &mut Vec<HashSet<String>>,
) -> ast::Block {
    block
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
        .collect()
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
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    local_modules: &HashSet<String>,
    imported_modules: &ImportedMap,
    global_module_map: &HashMap<String, String>,
    scopes: &mut Vec<HashSet<String>>,
) -> Stmt {
    match stmt {
        Stmt::Let {
            name,
            ty,
            value,
            mutable,
        } => {
            let rewritten = Stmt::Let {
                name: name.clone(),
                ty: rewrite_type_for_project(
                    ty,
                    current_namespace,
                    local_classes,
                    imported_classes,
                    global_class_map,
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
                    rewrite_type_for_project(
                        t,
                        current_namespace,
                        local_classes,
                        imported_classes,
                        global_class_map,
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
    }
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
    imported_enums: &ImportedMap,
    global_enum_map: &HashMap<String, String>,
    local_modules: &HashSet<String>,
    imported_modules: &ImportedMap,
    global_module_map: &HashMap<String, String>,
    scopes: &mut Vec<HashSet<String>>,
) -> Expr {
    match expr {
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
                                    Expr::Ident(mangle_project_symbol(
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
                                type_args: type_args.clone(),
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
                                    rewrite_type_for_project(
                                        ty,
                                        current_namespace,
                                        local_classes,
                                        imported_classes,
                                        global_class_map,
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
                                    rewrite_type_for_project(
                                        ty,
                                        current_namespace,
                                        local_classes,
                                        imported_classes,
                                        global_class_map,
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
                                rewrite_type_for_project(
                                    ty,
                                    current_namespace,
                                    local_classes,
                                    imported_classes,
                                    global_class_map,
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
                                        rewrite_type_for_project(
                                            ty,
                                            current_namespace,
                                            local_classes,
                                            imported_classes,
                                            global_class_map,
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
                                            rewrite_type_for_project(
                                                ty,
                                                current_namespace,
                                                local_classes,
                                                imported_classes,
                                                global_class_map,
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
                                                Expr::Ident(mangle_project_symbol(
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
                                        Expr::Ident(mangle_project_symbol(
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
                                        rewrite_type_for_project(
                                            ty,
                                            current_namespace,
                                            local_classes,
                                            imported_classes,
                                            global_class_map,
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
                                            Expr::Ident(mangle_project_symbol(
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
                                    Expr::Ident(mangle_project_symbol(
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
                            Expr::Ident(mangle_project_symbol(ns, entry_namespace, symbol_name))
                        }
                    } else if let Some(ns) = global_function_map.get(name) {
                        Expr::Ident(mangle_project_symbol(ns, entry_namespace, name))
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
                            rewrite_type_for_project(
                                ty,
                                current_namespace,
                                local_classes,
                                imported_classes,
                                global_class_map,
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
                        rewrite_type_for_project(
                            ty,
                            current_namespace,
                            local_classes,
                            imported_classes,
                            global_class_map,
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
                            return Expr::Ident(mangle_project_symbol(
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
                                    return Expr::Ident(mangle_project_symbol(
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
                            return Expr::Ident(mangle_project_symbol(
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
                            return Expr::Ident(mangle_project_symbol(
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
                                    return Expr::Ident(mangle_project_symbol(
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
                        ty: rewrite_type_for_project(
                            &p.ty,
                            current_namespace,
                            local_classes,
                            imported_classes,
                            global_class_map,
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
                    Expr::Ident(mangle_project_symbol(ns, entry_namespace, symbol_name))
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
                Expr::Ident(mangle_project_symbol(ns, entry_namespace, name))
            } else {
                Expr::Ident(name.clone())
            }
        }
        _ => expr.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp<T>(node: T) -> ast::Spanned<T> {
        ast::Spanned::new(node, 0..0)
    }

    #[test]
    fn keeps_shadowed_function_call_unmangled() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                is_variadic: false,
                extern_abi: None,
                extern_link_name: None,
                return_type: ast::Type::None,
                body: vec![
                    sp(Stmt::Let {
                        name: "foo".to_string(),
                        ty: ast::Type::Integer,
                        value: sp(Expr::Literal(ast::Literal::Integer(1))),
                        mutable: false,
                    }),
                    sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Ident("foo".to_string()))),
                        args: vec![],
                        type_args: vec![],
                    }))),
                ],
                is_async: false,
                is_extern: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![ast::ImportDecl {
            path: "lib.foo".to_string(),
            alias: None,
        }];
        let namespace_functions = HashMap::from([
            ("app".to_string(), HashSet::from(["main".to_string()])),
            ("lib".to_string(), HashSet::from(["foo".to_string()])),
        ]);
        let global_function_map = HashMap::from([("foo".to_string(), "lib".to_string())]);

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &namespace_functions,
            &global_function_map,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };
        let Stmt::Expr(expr_stmt) = &func.body[1].node else {
            panic!("expected call statement");
        };
        let Expr::Call { callee, .. } = &expr_stmt.node else {
            panic!("expected call expression");
        };
        let Expr::Ident(name) = &callee.node else {
            panic!("expected ident callee");
        };
        assert_eq!(name, "foo");
    }

    #[test]
    fn rewrites_imported_class_construct_and_module_field() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                is_variadic: false,
                extern_abi: None,
                extern_link_name: None,
                return_type: ast::Type::None,
                body: vec![
                    sp(Stmt::Let {
                        name: "w".to_string(),
                        ty: ast::Type::Named("Widget".to_string()),
                        value: sp(Expr::Construct {
                            ty: "Widget".to_string(),
                            args: vec![],
                        }),
                        mutable: false,
                    }),
                    sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("Utils".to_string()))),
                            field: "make".to_string(),
                        })),
                        args: vec![],
                        type_args: vec![],
                    }))),
                ],
                is_async: false,
                is_extern: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![
            ast::ImportDecl {
                path: "lib.Widget".to_string(),
                alias: None,
            },
            ast::ImportDecl {
                path: "lib.Utils".to_string(),
                alias: None,
            },
        ];
        let namespace_functions =
            HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]);
        let namespace_classes =
            HashMap::from([("lib".to_string(), HashSet::from(["Widget".to_string()]))]);
        let namespace_modules =
            HashMap::from([("lib".to_string(), HashSet::from(["Utils".to_string()]))]);
        let global_class_map = HashMap::from([("Widget".to_string(), "lib".to_string())]);
        let global_module_map = HashMap::from([("Utils".to_string(), "lib".to_string())]);

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &namespace_functions,
            &HashMap::new(),
            &namespace_classes,
            &global_class_map,
            &HashMap::new(),
            &HashMap::new(),
            &namespace_modules,
            &global_module_map,
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };
        let Stmt::Let { ty, value, .. } = &func.body[0].node else {
            panic!("expected let statement");
        };
        assert_eq!(ty, &ast::Type::Named("lib__Widget".to_string()));
        let Expr::Construct { ty, .. } = &value.node else {
            panic!("expected construct expression");
        };
        assert_eq!(ty, "lib__Widget");

        let Stmt::Expr(expr_stmt) = &func.body[1].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &expr_stmt.node else {
            panic!("expected call expression");
        };
        let Expr::Field { object, .. } = &callee.node else {
            panic!("expected field expression");
        };
        let Expr::Ident(name) = &object.node else {
            panic!("expected module ident");
        };
        assert_eq!(name, "lib__Utils");
    }

    #[test]
    fn rewrites_namespace_alias_class_constructor_calls() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "util".to_string(),
                    alias: Some("u".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("u".to_string()))),
                            field: "Box".to_string(),
                        })),
                        args: vec![sp(Expr::Literal(ast::Literal::Integer(2)))],
                        type_args: vec![],
                    })))],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]),
            &HashMap::new(),
            &HashMap::from([("util".to_string(), HashSet::from(["Box".to_string()]))]),
            &HashMap::from([("Box".to_string(), "util".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &[ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            }],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");
        let Stmt::Expr(expr_stmt) = &func.body[0].node else {
            panic!("expected expr statement");
        };
        let Expr::Construct { ty, .. } = &expr_stmt.node else {
            panic!("expected construct expression");
        };
        assert_eq!(ty, "util__Box");
    }

    #[test]
    fn rewrites_exact_imported_class_alias_constructor_calls() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "util.Box".to_string(),
                    alias: Some("B".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Ident("B".to_string()))),
                        args: vec![sp(Expr::Literal(ast::Literal::Integer(2)))],
                        type_args: vec![],
                    })))],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]),
            &HashMap::new(),
            &HashMap::from([("util".to_string(), HashSet::from(["Box".to_string()]))]),
            &HashMap::from([("Box".to_string(), "util".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &[ImportDecl {
                path: "util.Box".to_string(),
                alias: Some("B".to_string()),
            }],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");
        let Stmt::Expr(expr_stmt) = &func.body[0].node else {
            panic!("expected expr statement");
        };
        let Expr::Construct { ty, .. } = &expr_stmt.node else {
            panic!("expected construct expression");
        };
        assert_eq!(ty, "util__Box");
    }

    #[test]
    fn rewrites_namespace_alias_enum_variant_calls() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "util".to_string(),
                    alias: Some("u".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Field {
                                object: Box::new(sp(Expr::Ident("u".to_string()))),
                                field: "E".to_string(),
                            })),
                            field: "A".to_string(),
                        })),
                        args: vec![sp(Expr::Literal(ast::Literal::Integer(1)))],
                        type_args: vec![],
                    })))],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::from([("util".to_string(), HashSet::from(["E".to_string()]))]),
            &HashMap::from([("E".to_string(), "util".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &[ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            }],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");
        let Stmt::Expr(expr_stmt) = &func.body[0].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, args, .. } = &expr_stmt.node else {
            panic!("expected enum variant call expression");
        };
        let Expr::Field { object, field } = &callee.node else {
            panic!("expected enum variant field callee");
        };
        let Expr::Ident(name) = &object.node else {
            panic!("expected rewritten enum ident");
        };
        assert_eq!(name, "util__E");
        assert_eq!(field, "A");
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn rewrites_exact_imported_enum_alias_variant_calls() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "util.E".to_string(),
                    alias: Some("Enum".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("Enum".to_string()))),
                            field: "A".to_string(),
                        })),
                        args: vec![sp(Expr::Literal(ast::Literal::Integer(1)))],
                        type_args: vec![],
                    })))],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::from([("util".to_string(), HashSet::from(["E".to_string()]))]),
            &HashMap::from([("E".to_string(), "util".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &[ImportDecl {
                path: "util.E".to_string(),
                alias: Some("Enum".to_string()),
            }],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");
        let Stmt::Expr(expr_stmt) = &func.body[0].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &expr_stmt.node else {
            panic!("expected enum variant call expression");
        };
        let Expr::Field { object, field } = &callee.node else {
            panic!("expected enum variant field callee");
        };
        let Expr::Ident(name) = &object.node else {
            panic!("expected rewritten enum ident");
        };
        assert_eq!(name, "util__E");
        assert_eq!(field, "A");
    }

    #[test]
    fn rewrites_exact_imported_enum_alias_types() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "util.E".to_string(),
                    alias: Some("Enum".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Let {
                        name: "e".to_string(),
                        ty: ast::Type::Named("Enum".to_string()),
                        value: sp(Expr::Call {
                            callee: Box::new(sp(Expr::Field {
                                object: Box::new(sp(Expr::Ident("Enum".to_string()))),
                                field: "A".to_string(),
                            })),
                            args: vec![sp(Expr::Literal(ast::Literal::Integer(1)))],
                            type_args: vec![],
                        }),
                        mutable: false,
                    })],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::from([("util".to_string(), HashSet::from(["E".to_string()]))]),
            &HashMap::from([("E".to_string(), "util".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &[ImportDecl {
                path: "util.E".to_string(),
                alias: Some("Enum".to_string()),
            }],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");
        let Stmt::Let { ty, value, .. } = &func.body[0].node else {
            panic!("expected let statement");
        };
        assert_eq!(ty, &ast::Type::Named("util__E".to_string()));
        let Expr::Call { callee, .. } = &value.node else {
            panic!("expected enum variant call expression");
        };
        let Expr::Field { object, field } = &callee.node else {
            panic!("expected enum variant field callee");
        };
        let Expr::Ident(name) = &object.node else {
            panic!("expected rewritten enum ident");
        };
        assert_eq!(name, "util__E");
        assert_eq!(field, "A");
    }

    #[test]
    fn rewrites_exact_imported_enum_variant_alias_calls() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "util.E.B".to_string(),
                    alias: Some("Variant".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Ident("Variant".to_string()))),
                        args: vec![sp(Expr::Literal(ast::Literal::Integer(2)))],
                        type_args: vec![],
                    })))],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::from([("util".to_string(), HashSet::from(["E".to_string()]))]),
            &HashMap::from([("E".to_string(), "util".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &[ImportDecl {
                path: "util.E.B".to_string(),
                alias: Some("Variant".to_string()),
            }],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");
        let Stmt::Expr(expr_stmt) = &func.body[0].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &expr_stmt.node else {
            panic!("expected rewritten enum variant call");
        };
        let Expr::Field { object, field } = &callee.node else {
            panic!("expected rewritten enum variant field callee");
        };
        let Expr::Ident(name) = &object.node else {
            panic!("expected rewritten enum ident");
        };
        assert_eq!(name, "util__E");
        assert_eq!(field, "B");
    }

    #[test]
    fn rewrites_exact_imported_enum_variant_alias_patterns() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "util.E.B".to_string(),
                    alias: Some("Variant".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![ast::Parameter {
                        name: "e".to_string(),
                        ty: ast::Type::Named("E".to_string()),
                        mutable: false,
                        mode: ast::ParamMode::Owned,
                    }],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Match {
                        expr: sp(Expr::Ident("e".to_string())),
                        arms: vec![ast::MatchArm {
                            pattern: ast::Pattern::Variant(
                                "Variant".to_string(),
                                vec!["v".to_string()],
                            ),
                            body: vec![sp(Stmt::Expr(sp(Expr::Ident("v".to_string()))))],
                        }],
                    })],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::from([("util".to_string(), HashSet::from(["E".to_string()]))]),
            &HashMap::from([("E".to_string(), "util".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &[ImportDecl {
                path: "util.E.B".to_string(),
                alias: Some("Variant".to_string()),
            }],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");
        let Stmt::Match { arms, .. } = &func.body[0].node else {
            panic!("expected match statement");
        };
        assert!(matches!(
            &arms[0].pattern,
            ast::Pattern::Variant(name, bindings)
                if name == "util__E.B" && bindings == &vec!["v".to_string()]
        ));
    }

    #[test]
    fn rewrites_local_enum_types_and_variant_calls_inside_function_bodies() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Enum(ast::EnumDecl {
                    name: "E".to_string(),
                    generic_params: vec![],
                    variants: vec![ast::EnumVariant {
                        name: "A".to_string(),
                        fields: vec![ast::EnumField {
                            name: Some("value".to_string()),
                            ty: ast::Type::Integer,
                        }],
                    }],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![
                        sp(Stmt::Let {
                            name: "e".to_string(),
                            ty: ast::Type::Named("E".to_string()),
                            value: sp(Expr::Call {
                                callee: Box::new(sp(Expr::Field {
                                    object: Box::new(sp(Expr::Ident("E".to_string()))),
                                    field: "A".to_string(),
                                })),
                                args: vec![sp(Expr::Literal(ast::Literal::Integer(1)))],
                                type_args: vec![],
                            }),
                            mutable: false,
                        }),
                        sp(Stmt::Let {
                            name: "f".to_string(),
                            ty: ast::Type::Function(
                                vec![ast::Type::Named("E".to_string())],
                                Box::new(ast::Type::Integer),
                            ),
                            value: sp(Expr::Lambda {
                                params: vec![ast::Parameter {
                                    name: "x".to_string(),
                                    ty: ast::Type::Named("E".to_string()),
                                    mutable: false,
                                    mode: ast::ParamMode::Owned,
                                }],
                                body: Box::new(sp(Expr::Match {
                                    expr: Box::new(sp(Expr::Ident("x".to_string()))),
                                    arms: vec![ast::MatchArm {
                                        pattern: ast::Pattern::Variant(
                                            "E.A".to_string(),
                                            vec!["v".to_string()],
                                        ),
                                        body: vec![sp(Stmt::Expr(sp(Expr::Ident(
                                            "v".to_string(),
                                        ))))],
                                    }],
                                })),
                            }),
                            mutable: false,
                        }),
                    ],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::from([("app".to_string(), HashSet::from(["E".to_string()]))]),
            &HashMap::from([("E".to_string(), "app".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &[],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");

        let Stmt::Let { ty, value, .. } = &func.body[0].node else {
            panic!("expected enum let statement");
        };
        assert_eq!(ty, &ast::Type::Named("app__E".to_string()));
        let Expr::Call { callee, .. } = &value.node else {
            panic!("expected enum variant constructor call");
        };
        let Expr::Field { object, field } = &callee.node else {
            panic!("expected enum variant field");
        };
        let Expr::Ident(name) = &object.node else {
            panic!("expected rewritten local enum ident");
        };
        assert_eq!(name, "app__E");
        assert_eq!(field, "A");

        let Stmt::Let { ty, value, .. } = &func.body[1].node else {
            panic!("expected lambda let statement");
        };
        assert_eq!(
            ty,
            &ast::Type::Function(
                vec![ast::Type::Named("app__E".to_string())],
                Box::new(ast::Type::Integer),
            )
        );
        let Expr::Lambda { params, .. } = &value.node else {
            panic!("expected lambda expression");
        };
        assert_eq!(params[0].ty, ast::Type::Named("app__E".to_string()));
    }

    #[test]
    fn rewrites_namespace_alias_qualified_types() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "util".to_string(),
                    alias: Some("u".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![
                        sp(Stmt::Let {
                            name: "b".to_string(),
                            ty: ast::Type::Named("u.Box".to_string()),
                            value: sp(Expr::Call {
                                callee: Box::new(sp(Expr::Field {
                                    object: Box::new(sp(Expr::Ident("u".to_string()))),
                                    field: "Box".to_string(),
                                })),
                                args: vec![sp(Expr::Literal(ast::Literal::Integer(1)))],
                                type_args: vec![],
                            }),
                            mutable: false,
                        }),
                        sp(Stmt::Let {
                            name: "e".to_string(),
                            ty: ast::Type::Named("u.E".to_string()),
                            value: sp(Expr::Call {
                                callee: Box::new(sp(Expr::Field {
                                    object: Box::new(sp(Expr::Field {
                                        object: Box::new(sp(Expr::Ident("u".to_string()))),
                                        field: "E".to_string(),
                                    })),
                                    field: "A".to_string(),
                                })),
                                args: vec![sp(Expr::Literal(ast::Literal::Integer(1)))],
                                type_args: vec![],
                            }),
                            mutable: false,
                        }),
                    ],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]),
            &HashMap::new(),
            &HashMap::from([("util".to_string(), HashSet::from(["Box".to_string()]))]),
            &HashMap::from([("Box".to_string(), "util".to_string())]),
            &HashMap::from([("util".to_string(), HashSet::from(["E".to_string()]))]),
            &HashMap::from([("E".to_string(), "util".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &[ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            }],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");

        let Stmt::Let { ty, .. } = &func.body[0].node else {
            panic!("expected first let statement");
        };
        assert_eq!(ty, &ast::Type::Named("util__Box".to_string()));

        let Stmt::Let { ty, .. } = &func.body[1].node else {
            panic!("expected second let statement");
        };
        assert_eq!(ty, &ast::Type::Named("util__E".to_string()));
    }

    #[test]
    fn rewrites_namespace_alias_qualified_call_type_args() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "util".to_string(),
                    alias: Some("u".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Let {
                        name: "g".to_string(),
                        ty: ast::Type::Generic(
                            "List".to_string(),
                            vec![ast::Type::Named("u.Box".to_string())],
                        ),
                        value: sp(Expr::Call {
                            callee: Box::new(sp(Expr::Ident("List".to_string()))),
                            args: vec![],
                            type_args: vec![ast::Type::Named("u.Box".to_string())],
                        }),
                        mutable: false,
                    })],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]),
            &HashMap::new(),
            &HashMap::from([("util".to_string(), HashSet::from(["Box".to_string()]))]),
            &HashMap::from([("Box".to_string(), "util".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &[ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            }],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");

        let Stmt::Let { ty, value, .. } = &func.body[0].node else {
            panic!("expected let statement");
        };
        assert_eq!(
            ty,
            &ast::Type::Generic(
                "List".to_string(),
                vec![ast::Type::Named("util__Box".to_string())],
            )
        );
        let Expr::Call { type_args, .. } = &value.node else {
            panic!("expected call expression");
        };
        assert_eq!(type_args, &vec![ast::Type::Named("util__Box".to_string())]);
    }

    #[test]
    fn rewrites_namespace_alias_qualified_construct_type_strings() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "util".to_string(),
                    alias: Some("u".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Let {
                        name: "g".to_string(),
                        ty: ast::Type::List(Box::new(ast::Type::Named("u.Box".to_string()))),
                        value: sp(Expr::Construct {
                            ty: "List<u.Box>".to_string(),
                            args: vec![],
                        }),
                        mutable: false,
                    })],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]),
            &HashMap::new(),
            &HashMap::from([("util".to_string(), HashSet::from(["Box".to_string()]))]),
            &HashMap::from([("Box".to_string(), "util".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &[ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            }],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");

        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("expected let statement");
        };
        let Expr::Construct { ty, .. } = &value.node else {
            panic!("expected construct expression");
        };
        assert_eq!(ty, "List<util__Box>");
    }

    #[test]
    fn keeps_shadowed_module_ident_unmangled() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                is_variadic: false,
                extern_abi: None,
                extern_link_name: None,
                return_type: ast::Type::None,
                body: vec![
                    sp(Stmt::Let {
                        name: "Utils".to_string(),
                        ty: ast::Type::Integer,
                        value: sp(Expr::Literal(ast::Literal::Integer(0))),
                        mutable: false,
                    }),
                    sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("Utils".to_string()))),
                            field: "make".to_string(),
                        })),
                        args: vec![],
                        type_args: vec![],
                    }))),
                ],
                is_async: false,
                is_extern: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![ast::ImportDecl {
            path: "lib.Utils".to_string(),
            alias: None,
        }];
        let namespace_functions =
            HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]);
        let namespace_modules =
            HashMap::from([("lib".to_string(), HashSet::from(["Utils".to_string()]))]);
        let global_module_map = HashMap::from([("Utils".to_string(), "lib".to_string())]);

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &namespace_functions,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &namespace_modules,
            &global_module_map,
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };
        let Stmt::Expr(expr_stmt) = &func.body[1].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &expr_stmt.node else {
            panic!("expected call expression");
        };
        let Expr::Field { object, .. } = &callee.node else {
            panic!("expected field expression");
        };
        let Expr::Ident(name) = &object.node else {
            panic!("expected module ident");
        };
        assert_eq!(name, "Utils");
    }

    #[test]
    fn rewrites_wildcard_imported_function_and_class_symbols() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                is_variadic: false,
                extern_abi: None,
                extern_link_name: None,
                return_type: ast::Type::None,
                body: vec![
                    sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Ident("helper".to_string()))),
                        args: vec![],
                        type_args: vec![],
                    }))),
                    sp(Stmt::Let {
                        name: "widget".to_string(),
                        ty: ast::Type::Named("Widget".to_string()),
                        value: sp(Expr::Construct {
                            ty: "Widget".to_string(),
                            args: vec![],
                        }),
                        mutable: false,
                    }),
                ],
                is_async: false,
                is_extern: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![ast::ImportDecl {
            path: "lib.*".to_string(),
            alias: None,
        }];
        let namespace_functions = HashMap::from([
            ("app".to_string(), HashSet::from(["main".to_string()])),
            ("lib".to_string(), HashSet::from(["helper".to_string()])),
        ]);
        let global_function_map = HashMap::from([("helper".to_string(), "lib".to_string())]);
        let namespace_classes =
            HashMap::from([("lib".to_string(), HashSet::from(["Widget".to_string()]))]);
        let global_class_map = HashMap::from([("Widget".to_string(), "lib".to_string())]);

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &namespace_functions,
            &global_function_map,
            &namespace_classes,
            &global_class_map,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };
        let Stmt::Expr(expr_stmt) = &func.body[0].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &expr_stmt.node else {
            panic!("expected call expression");
        };
        let Expr::Ident(name) = &callee.node else {
            panic!("expected ident callee");
        };
        assert_eq!(name, "lib__helper");

        let Stmt::Let { ty, value, .. } = &func.body[1].node else {
            panic!("expected let statement");
        };
        assert_eq!(ty, &ast::Type::Named("lib__Widget".to_string()));
        let Expr::Construct { ty, .. } = &value.node else {
            panic!("expected construct expression");
        };
        assert_eq!(ty, "lib__Widget");
    }

    #[test]
    fn rewrites_aliased_imported_symbols() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                is_variadic: false,
                extern_abi: None,
                extern_link_name: None,
                return_type: ast::Type::None,
                body: vec![
                    sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Ident("mk".to_string()))),
                        args: vec![],
                        type_args: vec![],
                    }))),
                    sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("tools".to_string()))),
                            field: "run".to_string(),
                        })),
                        args: vec![],
                        type_args: vec![],
                    }))),
                ],
                is_async: false,
                is_extern: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![
            ast::ImportDecl {
                path: "lib.make".to_string(),
                alias: Some("mk".to_string()),
            },
            ast::ImportDecl {
                path: "lib.Tools".to_string(),
                alias: Some("tools".to_string()),
            },
        ];
        let namespace_functions =
            HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]);
        let namespace_modules =
            HashMap::from([("lib".to_string(), HashSet::from(["Tools".to_string()]))]);
        let global_function_map = HashMap::from([("make".to_string(), "lib".to_string())]);
        let global_module_map = HashMap::from([("Tools".to_string(), "lib".to_string())]);

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &namespace_functions,
            &global_function_map,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &namespace_modules,
            &global_module_map,
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };
        let Stmt::Expr(call_one) = &func.body[0].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &call_one.node else {
            panic!("expected call expression");
        };
        let Expr::Ident(name) = &callee.node else {
            panic!("expected ident callee");
        };
        assert_eq!(name, "lib__make");

        let Stmt::Expr(call_two) = &func.body[1].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &call_two.node else {
            panic!("expected call expression");
        };
        let Expr::Field { object, .. } = &callee.node else {
            panic!("expected field expression");
        };
        let Expr::Ident(name) = &object.node else {
            panic!("expected module ident");
        };
        assert_eq!(name, "lib__Tools");
    }

    #[test]
    fn rewrites_namespace_alias_module_style_calls() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                is_variadic: false,
                extern_abi: None,
                extern_link_name: None,
                return_type: ast::Type::None,
                body: vec![sp(Stmt::Expr(sp(Expr::Call {
                    callee: Box::new(sp(Expr::Field {
                        object: Box::new(sp(Expr::Ident("mu".to_string()))),
                        field: "factorial".to_string(),
                    })),
                    args: vec![sp(Expr::Literal(ast::Literal::Integer(5)))],
                    type_args: vec![],
                })))],
                is_async: false,
                is_extern: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![ast::ImportDecl {
            path: "math_utils".to_string(),
            alias: Some("mu".to_string()),
        }];
        let namespace_functions = HashMap::from([
            ("app".to_string(), HashSet::from(["main".to_string()])),
            (
                "math_utils".to_string(),
                HashSet::from(["factorial".to_string()]),
            ),
        ]);
        let global_function_map =
            HashMap::from([("factorial".to_string(), "math_utils".to_string())]);

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &namespace_functions,
            &global_function_map,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };
        let Stmt::Expr(call_stmt) = &func.body[0].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &call_stmt.node else {
            panic!("expected call expression");
        };
        let Expr::Ident(name) = &callee.node else {
            panic!("expected rewritten ident callee");
        };
        assert_eq!(name, "math_utils__factorial");
    }

    #[test]
    fn rewrites_nested_namespace_alias_module_style_calls() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                is_variadic: false,
                extern_abi: None,
                extern_link_name: None,
                return_type: ast::Type::None,
                body: vec![sp(Stmt::Expr(sp(Expr::Call {
                    callee: Box::new(sp(Expr::Field {
                        object: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("u".to_string()))),
                            field: "M".to_string(),
                        })),
                        field: "add1".to_string(),
                    })),
                    args: vec![sp(Expr::Literal(ast::Literal::Integer(5)))],
                    type_args: vec![],
                })))],
                is_async: false,
                is_extern: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![ast::ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }];
        let namespace_functions = HashMap::from([
            ("app".to_string(), HashSet::from(["main".to_string()])),
            ("util".to_string(), HashSet::from(["M__add1".to_string()])),
        ]);
        let global_function_map = HashMap::from([("M__add1".to_string(), "util".to_string())]);

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &namespace_functions,
            &global_function_map,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };
        let Stmt::Expr(call_stmt) = &func.body[0].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &call_stmt.node else {
            panic!("expected call expression");
        };
        let Expr::Ident(name) = &callee.node else {
            panic!("expected rewritten ident callee");
        };
        assert_eq!(name, "util__M__add1");
    }

    #[test]
    fn rewrites_dotted_module_alias_module_style_calls() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                is_variadic: false,
                extern_abi: None,
                extern_link_name: None,
                return_type: ast::Type::None,
                body: vec![sp(Stmt::Expr(sp(Expr::Call {
                    callee: Box::new(sp(Expr::Field {
                        object: Box::new(sp(Expr::Ident("ax".to_string()))),
                        field: "f".to_string(),
                    })),
                    args: vec![],
                    type_args: vec![],
                })))],
                is_async: false,
                is_extern: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![ast::ImportDecl {
            path: "lib.A.X".to_string(),
            alias: Some("ax".to_string()),
        }];
        let namespace_functions = HashMap::from([
            ("app".to_string(), HashSet::from(["main".to_string()])),
            ("lib".to_string(), HashSet::from(["A__X__f".to_string()])),
        ]);
        let global_function_map = HashMap::from([("A__X__f".to_string(), "lib".to_string())]);

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &namespace_functions,
            &global_function_map,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };
        let Stmt::Expr(call_stmt) = &func.body[0].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &call_stmt.node else {
            panic!("expected call expression");
        };
        let Expr::Ident(name) = &callee.node else {
            panic!("expected rewritten ident callee");
        };
        assert_eq!(name, "lib__A__X__f");
    }

    #[test]
    fn rewrites_namespace_alias_nested_module_dot_calls() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                is_variadic: false,
                extern_abi: None,
                extern_link_name: None,
                return_type: ast::Type::None,
                body: vec![sp(Stmt::Expr(sp(Expr::Call {
                    callee: Box::new(sp(Expr::Field {
                        object: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("l".to_string()))),
                            field: "Tools".to_string(),
                        })),
                        field: "ping".to_string(),
                    })),
                    args: vec![],
                    type_args: vec![],
                })))],
                is_async: false,
                is_extern: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![ast::ImportDecl {
            path: "lib".to_string(),
            alias: Some("l".to_string()),
        }];
        let namespace_functions = HashMap::from([
            ("app".to_string(), HashSet::from(["main".to_string()])),
            (
                "lib".to_string(),
                HashSet::from(["Tools__ping".to_string()]),
            ),
        ]);
        let global_function_map = HashMap::from([("Tools__ping".to_string(), "lib".to_string())]);

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &namespace_functions,
            &global_function_map,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };
        let Stmt::Expr(call_stmt) = &func.body[0].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &call_stmt.node else {
            panic!("expected call expression");
        };
        let Expr::Ident(name) = &callee.node else {
            panic!("expected rewritten ident callee");
        };
        assert_eq!(name, "lib__Tools__ping");
    }

    #[test]
    fn rewrites_namespace_alias_deep_nested_module_dot_calls() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                is_variadic: false,
                extern_abi: None,
                extern_link_name: None,
                return_type: ast::Type::None,
                body: vec![sp(Stmt::Expr(sp(Expr::Call {
                    callee: Box::new(sp(Expr::Field {
                        object: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Field {
                                object: Box::new(sp(Expr::Ident("l".to_string()))),
                                field: "A".to_string(),
                            })),
                            field: "X".to_string(),
                        })),
                        field: "f".to_string(),
                    })),
                    args: vec![],
                    type_args: vec![],
                })))],
                is_async: false,
                is_extern: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![ast::ImportDecl {
            path: "lib".to_string(),
            alias: Some("l".to_string()),
        }];
        let namespace_functions = HashMap::from([
            ("app".to_string(), HashSet::from(["main".to_string()])),
            ("lib".to_string(), HashSet::from(["A__X__f".to_string()])),
        ]);
        let global_function_map = HashMap::from([("A__X__f".to_string(), "lib".to_string())]);

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &namespace_functions,
            &global_function_map,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };
        let Stmt::Expr(call_stmt) = &func.body[0].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &call_stmt.node else {
            panic!("expected call expression");
        };
        let Expr::Ident(name) = &callee.node else {
            panic!("expected rewritten ident callee");
        };
        assert_eq!(name, "lib__A__X__f");
    }

    #[test]
    fn rewrites_aliased_stdlib_module_calls() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                is_variadic: false,
                extern_abi: None,
                extern_link_name: None,
                return_type: ast::Type::None,
                body: vec![
                    sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("io".to_string()))),
                            field: "println".to_string(),
                        })),
                        args: vec![sp(Expr::Literal(ast::Literal::String("x".to_string())))],
                        type_args: vec![],
                    }))),
                    sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("math".to_string()))),
                            field: "abs".to_string(),
                        })),
                        args: vec![sp(Expr::Literal(ast::Literal::Integer(-1)))],
                        type_args: vec![],
                    }))),
                ],
                is_async: false,
                is_extern: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![
            ast::ImportDecl {
                path: "std.io".to_string(),
                alias: Some("io".to_string()),
            },
            ast::ImportDecl {
                path: "std.math".to_string(),
                alias: Some("math".to_string()),
            },
        ];

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };

        let Stmt::Expr(first) = &func.body[0].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &first.node else {
            panic!("expected call expression");
        };
        let Expr::Ident(name) = &callee.node else {
            panic!("expected io alias to rewrite to direct ident call");
        };
        assert_eq!(name, "println");

        let Stmt::Expr(second) = &func.body[1].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &second.node else {
            panic!("expected call expression");
        };
        let Expr::Field { object, field } = &callee.node else {
            panic!("expected math alias to rewrite to std module field call");
        };
        let Expr::Ident(name) = &object.node else {
            panic!("expected std module ident");
        };
        assert_eq!(name, "Math");
        assert_eq!(field, "abs");
    }

    #[test]
    fn rewrites_function_value_identifiers_outside_call_positions() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Function(ast::FunctionDecl {
                    name: "add1".to_string(),
                    generic_params: vec![],
                    params: vec![ast::Parameter {
                        name: "x".to_string(),
                        ty: ast::Type::Integer,
                        mutable: false,
                        mode: ast::ParamMode::Owned,
                    }],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::Integer,
                    body: vec![sp(Stmt::Return(Some(sp(Expr::Binary {
                        op: ast::BinOp::Add,
                        left: Box::new(sp(Expr::Ident("x".to_string()))),
                        right: Box::new(sp(Expr::Literal(ast::Literal::Integer(1)))),
                    }))))],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Let {
                        name: "f".to_string(),
                        ty: ast::Type::Function(
                            vec![ast::Type::Integer],
                            Box::new(ast::Type::Integer),
                        ),
                        value: sp(Expr::Ident("add1".to_string())),
                        mutable: false,
                    })],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([(
                "app".to_string(),
                HashSet::from(["add1".to_string(), "main".to_string()]),
            )]),
            &HashMap::from([
                ("add1".to_string(), "app".to_string()),
                ("main".to_string(), "app".to_string()),
            ]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &[],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("expected let statement");
        };
        let Expr::Ident(name) = &value.node else {
            panic!("expected rewritten function reference ident");
        };
        assert_eq!(name, "app__add1");
    }

    #[test]
    fn rewrites_if_expression_function_value_branches() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Function(ast::FunctionDecl {
                    name: "inc".to_string(),
                    generic_params: vec![],
                    params: vec![ast::Parameter {
                        name: "x".to_string(),
                        ty: ast::Type::Integer,
                        mutable: false,
                        mode: ast::ParamMode::Owned,
                    }],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::Integer,
                    body: vec![sp(Stmt::Return(Some(sp(Expr::Ident("x".to_string())))))],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "dec".to_string(),
                    generic_params: vec![],
                    params: vec![ast::Parameter {
                        name: "x".to_string(),
                        ty: ast::Type::Integer,
                        mutable: false,
                        mode: ast::ParamMode::Owned,
                    }],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::Integer,
                    body: vec![sp(Stmt::Return(Some(sp(Expr::Ident("x".to_string())))))],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Let {
                        name: "f".to_string(),
                        ty: ast::Type::Function(
                            vec![ast::Type::Integer],
                            Box::new(ast::Type::Integer),
                        ),
                        value: sp(Expr::IfExpr {
                            condition: Box::new(sp(Expr::Literal(ast::Literal::Boolean(true)))),
                            then_branch: vec![sp(Stmt::Expr(sp(Expr::Ident("inc".to_string()))))],
                            else_branch: Some(vec![sp(Stmt::Expr(sp(Expr::Ident(
                                "dec".to_string(),
                            ))))]),
                        }),
                        mutable: false,
                    })],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([(
                "app".to_string(),
                HashSet::from(["inc".to_string(), "dec".to_string(), "main".to_string()]),
            )]),
            &HashMap::from([
                ("inc".to_string(), "app".to_string()),
                ("dec".to_string(), "app".to_string()),
                ("main".to_string(), "app".to_string()),
            ]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &[],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "app__main" || func.name == "main" => {
                    Some(func)
                }
                _ => None,
            })
            .expect("expected rewritten main function");
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("expected let statement");
        };
        let Expr::IfExpr {
            then_branch,
            else_branch,
            ..
        } = &value.node
        else {
            panic!("expected rewritten if expression");
        };
        let Stmt::Expr(expr) = &then_branch[0].node else {
            panic!("expected then branch expr");
        };
        let Expr::Ident(name) = &expr.node else {
            panic!("expected rewritten then branch function ident");
        };
        assert_eq!(name, "app__inc");
        let else_branch = else_branch.as_ref().expect("expected else branch");
        let Stmt::Expr(expr) = &else_branch[0].node else {
            panic!("expected else branch expr");
        };
        let Expr::Ident(name) = &expr.node else {
            panic!("expected rewritten else branch function ident");
        };
        assert_eq!(name, "app__dec");
    }

    #[test]
    fn rewrites_module_alias_function_values_outside_call_positions() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "util".to_string(),
                    alias: Some("u".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Let {
                        name: "f".to_string(),
                        ty: ast::Type::Function(
                            vec![ast::Type::Integer],
                            Box::new(ast::Type::Integer),
                        ),
                        value: sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("u".to_string()))),
                            field: "add1".to_string(),
                        }),
                        mutable: false,
                    })],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([
                ("app".to_string(), HashSet::from(["main".to_string()])),
                ("util".to_string(), HashSet::from(["add1".to_string()])),
            ]),
            &HashMap::from([
                ("main".to_string(), "app".to_string()),
                ("add1".to_string(), "util".to_string()),
            ]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &[ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            }],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("expected let statement");
        };
        let Expr::Ident(name) = &value.node else {
            panic!("expected rewritten module alias function reference ident");
        };
        assert_eq!(name, "util__add1");
    }

    #[test]
    fn rewrites_nested_module_alias_function_values_outside_call_positions() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "util".to_string(),
                    alias: Some("u".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Let {
                        name: "f".to_string(),
                        ty: ast::Type::Function(
                            vec![ast::Type::Integer],
                            Box::new(ast::Type::Integer),
                        ),
                        value: sp(Expr::Field {
                            object: Box::new(sp(Expr::Field {
                                object: Box::new(sp(Expr::Ident("u".to_string()))),
                                field: "M".to_string(),
                            })),
                            field: "add1".to_string(),
                        }),
                        mutable: false,
                    })],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([
                ("app".to_string(), HashSet::from(["main".to_string()])),
                ("util".to_string(), HashSet::from(["M__add1".to_string()])),
            ]),
            &HashMap::from([("M__add1".to_string(), "util".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &[ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            }],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("expected let statement");
        };
        let Expr::Ident(name) = &value.node else {
            panic!("expected rewritten nested module alias function reference ident");
        };
        assert_eq!(name, "util__M__add1");
    }

    #[test]
    fn rewrites_namespace_alias_nested_module_generic_class_types_and_constructors() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "util".to_string(),
                    alias: Some("u".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "main".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Let {
                        name: "b".to_string(),
                        ty: ast::Type::Generic("u.M.Box".to_string(), vec![ast::Type::Integer]),
                        value: sp(Expr::Call {
                            callee: Box::new(sp(Expr::Field {
                                object: Box::new(sp(Expr::Field {
                                    object: Box::new(sp(Expr::Ident("u".to_string()))),
                                    field: "M".to_string(),
                                })),
                                field: "Box".to_string(),
                            })),
                            args: vec![sp(Expr::Literal(ast::Literal::Integer(1)))],
                            type_args: vec![ast::Type::Integer],
                        }),
                        mutable: false,
                    })],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]),
            &HashMap::new(),
            &HashMap::from([("util".to_string(), HashSet::from(["M__Box".to_string()]))]),
            &HashMap::from([("M__Box".to_string(), "util".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &[ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            }],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name == "main" => Some(func),
                _ => None,
            })
            .expect("expected main function declaration");
        let Stmt::Let { ty, value, .. } = &func.body[0].node else {
            panic!("expected let statement");
        };
        assert_eq!(
            ty,
            &ast::Type::Generic("util__M__Box".to_string(), vec![ast::Type::Integer])
        );
        let Expr::Construct { ty, args } = &value.node else {
            panic!("expected rewritten constructor expression");
        };
        assert_eq!(ty, "util__M__Box<Integer>");
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn rewrites_top_level_generic_classes_that_shadow_builtin_names() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![
                sp(Decl::Class(ast::ClassDecl {
                    name: "Box".to_string(),
                    generic_params: vec![ast::GenericParam {
                        name: "T".to_string(),
                        bounds: vec![],
                    }],
                    extends: None,
                    implements: vec![],
                    fields: vec![ast::Field {
                        name: "value".to_string(),
                        ty: ast::Type::Named("T".to_string()),
                        mutable: false,
                        visibility: ast::Visibility::Private,
                    }],
                    constructor: None,
                    destructor: None,
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "mk".to_string(),
                    generic_params: vec![],
                    params: vec![ast::Parameter {
                        name: "value".to_string(),
                        ty: ast::Type::Integer,
                        mutable: false,
                        mode: ast::ParamMode::Owned,
                    }],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::Box(Box::new(ast::Type::Integer)),
                    body: vec![sp(Stmt::Return(Some(sp(Expr::Construct {
                        ty: "Box<Integer>".to_string(),
                        args: vec![sp(Expr::Ident("value".to_string()))],
                    }))))],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        };

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &HashMap::from([("app".to_string(), HashSet::from(["mk".to_string()]))]),
            &HashMap::new(),
            &HashMap::from([("app".to_string(), HashSet::from(["Box".to_string()]))]),
            &HashMap::from([("Box".to_string(), "app".to_string())]),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &[],
        );

        let func = rewritten
            .declarations
            .iter()
            .find_map(|decl| match &decl.node {
                Decl::Function(func) if func.name.ends_with("mk") => Some(func),
                _ => None,
            })
            .expect("expected mk function declaration");
        assert_eq!(
            func.return_type,
            ast::Type::Generic("app__Box".to_string(), vec![ast::Type::Integer])
        );
        let Stmt::Return(Some(expr)) = &func.body[0].node else {
            panic!("expected return statement");
        };
        let Expr::Construct { ty, args } = &expr.node else {
            panic!("expected rewritten constructor expression");
        };
        assert_eq!(ty, "app__Box<Integer>");
        assert_eq!(args.len(), 1);
    }
}
