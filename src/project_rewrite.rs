use std::collections::{HashMap, HashSet};

use crate::ast::{self, Decl, Expr, ImportDecl, Program, Stmt};
use crate::stdlib::StdLib;

type ImportedMap = HashMap<String, (String, String)>;

#[allow(clippy::too_many_arguments)]
pub fn rewrite_program_for_project(
    program: &Program,
    current_namespace: &str,
    entry_namespace: &str,
    namespace_functions: &HashMap<String, HashSet<String>>,
    global_function_map: &HashMap<String, String>,
    namespace_classes: &HashMap<String, HashSet<String>>,
    global_class_map: &HashMap<String, String>,
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
    let local_modules = namespace_modules
        .get(current_namespace)
        .cloned()
        .unwrap_or_default();

    let mut imported_map: ImportedMap = HashMap::new();
    let mut imported_classes: ImportedMap = HashMap::new();
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
            if let Some(modules) = namespace_modules.get(ns) {
                for name in modules {
                    imported_modules.insert(name.clone(), (ns.to_string(), name.clone()));
                }
            }
        } else if import.path.contains('.') {
            let mut parts = import.path.split('.').collect::<Vec<_>>();
            if let Some(source_name) = parts.pop() {
                let ns = parts.join(".");
                imported_map.insert(import_key.clone(), (ns.clone(), source_name.to_string()));
                imported_classes.insert(import_key.clone(), (ns.clone(), source_name.to_string()));
                imported_modules.insert(import_key, (ns, source_name.to_string()));
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

fn rewrite_type_for_project(
    ty: &ast::Type,
    current_namespace: &str,
    local_classes: &HashSet<String>,
    imported_classes: &ImportedMap,
    global_class_map: &HashMap<String, String>,
    entry_namespace: &str,
) -> ast::Type {
    match ty {
        ast::Type::Named(name) => {
            if local_classes.contains(name) {
                ast::Type::Named(mangle_project_symbol(
                    current_namespace,
                    entry_namespace,
                    name,
                ))
            } else if let Some((ns, symbol_name)) = imported_classes.get(name) {
                ast::Type::Named(mangle_project_symbol(ns, entry_namespace, symbol_name))
            } else if let Some(ns) = global_class_map.get(name) {
                ast::Type::Named(mangle_project_symbol(ns, entry_namespace, name))
            } else {
                ast::Type::Named(name.clone())
            }
        }
        ast::Type::Generic(name, args) => ast::Type::Generic(
            if local_classes.contains(name) {
                mangle_project_symbol(current_namespace, entry_namespace, name)
            } else if let Some((ns, symbol_name)) = imported_classes.get(name) {
                mangle_project_symbol(ns, entry_namespace, symbol_name)
            } else if let Some(ns) = global_class_map.get(name) {
                mangle_project_symbol(ns, entry_namespace, name)
            } else {
                name.clone()
            },
            args.iter()
                .map(|a| {
                    rewrite_type_for_project(
                        a,
                        current_namespace,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        entry_namespace,
                    )
                })
                .collect(),
        ),
        ast::Type::Function(params, ret) => ast::Type::Function(
            params
                .iter()
                .map(|p| {
                    rewrite_type_for_project(
                        p,
                        current_namespace,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        entry_namespace,
                    )
                })
                .collect(),
            Box::new(rewrite_type_for_project(
                ret,
                current_namespace,
                local_classes,
                imported_classes,
                global_class_map,
                entry_namespace,
            )),
        ),
        ast::Type::Option(inner) => ast::Type::Option(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Result(ok, err) => ast::Type::Result(
            Box::new(rewrite_type_for_project(
                ok,
                current_namespace,
                local_classes,
                imported_classes,
                global_class_map,
                entry_namespace,
            )),
            Box::new(rewrite_type_for_project(
                err,
                current_namespace,
                local_classes,
                imported_classes,
                global_class_map,
                entry_namespace,
            )),
        ),
        ast::Type::List(inner) => ast::Type::List(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Map(k, v) => ast::Type::Map(
            Box::new(rewrite_type_for_project(
                k,
                current_namespace,
                local_classes,
                imported_classes,
                global_class_map,
                entry_namespace,
            )),
            Box::new(rewrite_type_for_project(
                v,
                current_namespace,
                local_classes,
                imported_classes,
                global_class_map,
                entry_namespace,
            )),
        ),
        ast::Type::Set(inner) => ast::Type::Set(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Ref(inner) => ast::Type::Ref(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::MutRef(inner) => ast::Type::MutRef(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Box(inner) => ast::Type::Box(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Rc(inner) => ast::Type::Rc(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Arc(inner) => ast::Type::Arc(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Ptr(inner) => ast::Type::Ptr(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Task(inner) => ast::Type::Task(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Range(inner) => ast::Type::Range(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        _ => ty.clone(),
    }
}

fn is_shadowed(name: &str, scopes: &[HashSet<String>]) -> bool {
    scopes.iter().rev().any(|scope| scope.contains(name))
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
                    push_scope(scopes);
                    if let Some(scope) = scopes.last_mut() {
                        bind_pattern_locals(&arm.pattern, scope);
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
                        local_modules,
                        imported_modules,
                        global_module_map,
                        scopes,
                    );
                    pop_scope(scopes);
                    ast::MatchArm {
                        pattern: arm.pattern.clone(),
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
            let rewritten_callee = match &callee.node {
                Expr::Field { object, field }
                    if matches!(&object.node, Expr::Ident(_))
                        && !matches!(&object.node, Expr::Ident(name) if is_shadowed(name, scopes)) =>
                {
                    let Expr::Ident(module_alias) = &object.node else {
                        unreachable!()
                    };
                    if let Some((ns, symbol_name)) = imported_modules.get(module_alias) {
                        let namespace_path = format!("{}.{}", ns, symbol_name);
                        if let Some(canonical) =
                            StdLib::new().resolve_alias_call(&namespace_path, field)
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
                        if StdLib::new()
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
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
            };
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
                                local_modules,
                                imported_modules,
                                global_module_map,
                                scopes,
                            ),
                            a.span.clone(),
                        )
                    })
                    .collect(),
                type_args: type_args.clone(),
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
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                expr.span.clone(),
            )),
        },
        Expr::Field { object, field } => {
            let rewritten_object = match &object.node {
                Expr::Ident(name) if !is_shadowed(name, scopes) => {
                    if local_modules.contains(name) {
                        Expr::Ident(mangle_project_symbol(
                            current_namespace,
                            entry_namespace,
                            name,
                        ))
                    } else if let Some((ns, symbol_name)) = imported_modules.get(name) {
                        Expr::Ident(mangle_project_symbol(ns, entry_namespace, symbol_name))
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
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                index.span.clone(),
            )),
        },
        Expr::Construct { ty, args } => Expr::Construct {
            ty: if local_classes.contains(ty) {
                mangle_project_symbol(current_namespace, entry_namespace, ty)
            } else if let Some((ns, symbol_name)) = imported_classes.get(ty) {
                mangle_project_symbol(ns, entry_namespace, symbol_name)
            } else if let Some(ns) = global_class_map.get(ty) {
                mangle_project_symbol(ns, entry_namespace, ty)
            } else {
                ty.clone()
            },
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
                            local_modules,
                            imported_modules,
                            global_module_map,
                            scopes,
                        ),
                        a.span.clone(),
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
                            entry_namespace,
                        ),
                        mutable: p.mutable,
                        mode: p.mode,
                    })
                    .collect(),
                body: Box::new(ast::Spanned::new(rewritten_body, body.span.clone())),
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
}
