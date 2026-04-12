use super::*;

pub(super) fn rewrite_expr_calls_for_project(
    expr: &Expr,
    ctx: CallRewriteContext<'_>,
    scopes: &mut Vec<HashSet<String>>,
) -> Expr {
    let current_namespace = ctx.current_namespace();
    let entry_namespace = ctx.entry_namespace();
    let local_functions = ctx.local_functions;
    let imported_map = ctx.imported_map;
    let global_function_map = ctx.global_function_map;
    let local_classes = ctx.type_ctx.local_classes;
    let imported_classes = ctx.type_ctx.imported_classes;
    let global_class_map = ctx.type_ctx.global_class_map;
    let local_interfaces = ctx.type_ctx.local_interfaces;
    let imported_interfaces = ctx.type_ctx.imported_interfaces;
    let global_interface_map = ctx.type_ctx.global_interface_map;
    let local_enums = ctx.type_ctx.local_enums;
    let imported_enums = ctx.type_ctx.imported_enums;
    let global_enum_map = ctx.type_ctx.global_enum_map;
    let local_modules = ctx.local_modules;
    let class_symbol_names = ctx.class_symbol_names;
    let imported_modules = ctx.type_ctx.imported_modules;
    let global_module_map = ctx.global_module_map;
    let rewrite_type_for_project_with_interfaces =
        |ty: &ast::Type,
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
         entry_namespace: &str| {
            self::rewrite_type_for_project_with_interfaces(
                ty,
                RewriteTypeContext {
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
                },
            )
        };
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
                                            self::rewrite_expr_calls_for_project(
                                                &arg.node, ctx, scopes,
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
                                            local_enums,
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
                                            rewrite_expr_calls_for_project(&arg.node, ctx, scopes),
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
                                        local_enums,
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
                                            rewrite_expr_calls_for_project(&arg.node, ctx, scopes),
                                            arg.span.clone(),
                                        )
                                    })
                                    .collect(),
                            };
                        }
                    }
                    if let Some((ns, symbol_name)) = imported_modules.get(module_alias) {
                        if let Some((owner_name, field_name)) =
                            builtin_module_alias_static_container_parts(
                                ns,
                                symbol_name,
                                member_parts,
                            )
                        {
                            return Expr::Call {
                                callee: Box::new(ast::Spanned::new(
                                    Expr::Field {
                                        object: Box::new(ast::Spanned::new(
                                            Expr::Ident(owner_name.to_string()),
                                            callee.span.clone(),
                                        )),
                                        field: field_name.to_string(),
                                    },
                                    callee.span.clone(),
                                )),
                                args: args
                                    .iter()
                                    .map(|arg| {
                                        ast::Spanned::new(
                                            rewrite_expr_calls_for_project(&arg.node, ctx, scopes),
                                            arg.span.clone(),
                                        )
                                    })
                                    .collect(),
                                type_args: vec![],
                            };
                        }
                        if member_parts.len() > 1 {
                            let receiver_parts = &member_parts[..member_parts.len() - 1];
                            let Some(receiver_field) = member_parts.last() else {
                                return rewrite_expr_calls_for_project(&callee.node, ctx, scopes);
                            };
                            if let Some(receiver_value) =
                                builtin_module_alias_value_expr(ns, symbol_name, receiver_parts)
                            {
                                return Expr::Call {
                                    callee: Box::new(ast::Spanned::new(
                                        Expr::Field {
                                            object: Box::new(ast::Spanned::new(
                                                receiver_value,
                                                callee.span.clone(),
                                            )),
                                            field: receiver_field.clone(),
                                        },
                                        callee.span.clone(),
                                    )),
                                    args: args
                                        .iter()
                                        .map(|arg| {
                                            ast::Spanned::new(
                                                rewrite_expr_calls_for_project(
                                                    &arg.node, ctx, scopes,
                                                ),
                                                arg.span.clone(),
                                            )
                                        })
                                        .collect(),
                                    type_args: vec![],
                                };
                            }
                        }
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
                                        local_enums,
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
                                            rewrite_expr_calls_for_project(&arg.node, ctx, scopes),
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
                                    local_enums,
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
                                        rewrite_expr_calls_for_project(&arg.node, ctx, scopes),
                                        arg.span.clone(),
                                    )
                                })
                                .collect(),
                        };
                    }
                    if let Some((import_ns, symbol_name)) = imported_modules.get(name) {
                        if let Some(canonical) =
                            resolve_exact_builtin_imported_symbol(import_ns, symbol_name)
                        {
                            return Expr::Call {
                                callee: Box::new(ast::Spanned::new(
                                    Expr::Ident(canonical),
                                    callee.span.clone(),
                                )),
                                args: args
                                    .iter()
                                    .map(|arg| {
                                        ast::Spanned::new(
                                            rewrite_expr_calls_for_project(&arg.node, ctx, scopes),
                                            arg.span.clone(),
                                        )
                                    })
                                    .collect(),
                                type_args: vec![],
                            };
                        }
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
                                            rewrite_expr_calls_for_project(&arg.node, ctx, scopes),
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
                    let Some(chain_parts) = flatten_field_chain(&object.node) else {
                        return self::rewrite_expr_calls_for_project(&callee.node, ctx, scopes);
                    };
                    let alias_ident = &chain_parts[0];

                    if is_shadowed(alias_ident, scopes) {
                        self::rewrite_expr_calls_for_project(&callee.node, ctx, scopes)
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
                                            local_enums,
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
                                                    &arg.node, ctx, scopes,
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
                                        rewrite_expr_calls_for_project(&callee.node, ctx, scopes)
                                    }
                                } else {
                                    rewrite_expr_calls_for_project(&callee.node, ctx, scopes)
                                }
                            }
                        } else {
                            self::rewrite_expr_calls_for_project(&callee.node, ctx, scopes)
                        }
                    } else {
                        self::rewrite_expr_calls_for_project(&callee.node, ctx, scopes)
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
                                        ctx,
                                        scopes,
                                    );
                                }
                                let Some(field) = member_parts.last() else {
                                    return rewrite_expr_calls_for_project(
                                        &callee.node,
                                        ctx,
                                        scopes,
                                    );
                                };
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
                                                local_enums,
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
                                                        &arg.node, ctx, scopes,
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
                                                    ctx,
                                                    scopes,
                                                )
                                            }
                                        } else {
                                            rewrite_expr_calls_for_project(
                                                &callee.node,
                                                ctx,
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
                                        rewrite_expr_calls_for_project(&callee.node, ctx, scopes)
                                    }
                                }
                            } else {
                                rewrite_expr_calls_for_project(&callee.node, ctx, scopes)
                            }
                        } else if let Some((ns, symbol_name)) = imported_modules.get(module_alias) {
                            if member_parts.is_empty() {
                                return rewrite_expr_calls_for_project(&callee.node, ctx, scopes);
                            }
                            let Some(field) = member_parts.last() else {
                                return rewrite_expr_calls_for_project(&callee.node, ctx, scopes);
                            };
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
                                            local_enums,
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
                                                    &arg.node, ctx, scopes,
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
                                                ctx,
                                                scopes,
                                            )
                                        }
                                    } else {
                                        rewrite_expr_calls_for_project(&callee.node, ctx, scopes)
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
                                    rewrite_expr_calls_for_project(&callee.node, ctx, scopes)
                                }
                            }
                        } else {
                            self::rewrite_expr_calls_for_project(&callee.node, ctx, scopes)
                        }
                    } else {
                        self::rewrite_expr_calls_for_project(&callee.node, ctx, scopes)
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
                        if is_builtin_exact_import_canonical(symbol_name)
                            || stdlib_registry()
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
                other => self::rewrite_expr_calls_for_project(other, ctx, scopes),
            };
            if let Expr::Ident(name) = &rewritten_callee {
                if class_symbol_names.contains(name) {
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
                                local_enums,
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
                                    rewrite_expr_calls_for_project(&a.node, ctx, scopes),
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
                            self::rewrite_expr_calls_for_project(&a.node, ctx, scopes),
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
                            local_enums,
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
                self::rewrite_expr_calls_for_project(&left.node, ctx, scopes),
                left.span.clone(),
            )),
            right: Box::new(ast::Spanned::new(
                self::rewrite_expr_calls_for_project(&right.node, ctx, scopes),
                right.span.clone(),
            )),
        },
        Expr::Unary { op, expr } => Expr::Unary {
            op: *op,
            expr: Box::new(ast::Spanned::new(
                self::rewrite_expr_calls_for_project(&expr.node, ctx, scopes),
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
                        if let Some(canonical) =
                            resolve_module_alias_builtin_canonical(ns, symbol_name, member_parts)
                        {
                            return Expr::Ident(canonical.to_string());
                        }
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
                        let Some(field) = member_parts.last() else {
                            return rewrite_expr_calls_for_project(expr, ctx, scopes);
                        };
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
                        if let Some(value_expr) = builtin_exact_import_value_expr(ns, symbol_name) {
                            value_expr
                        } else if let Some(canonical) =
                            resolve_exact_stdlib_imported_value_symbol(ns, symbol_name)
                        {
                            Expr::Ident(canonical)
                        } else if symbol_name.is_empty() {
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
                _ => self::rewrite_expr_calls_for_project(&object.node, ctx, scopes),
            };
            Expr::Field {
                object: Box::new(ast::Spanned::new(rewritten_object, object.span.clone())),
                field: field.clone(),
            }
        }
        Expr::Index { object, index } => Expr::Index {
            object: Box::new(ast::Spanned::new(
                self::rewrite_expr_calls_for_project(&object.node, ctx, scopes),
                object.span.clone(),
            )),
            index: Box::new(ast::Spanned::new(
                self::rewrite_expr_calls_for_project(&index.node, ctx, scopes),
                index.span.clone(),
            )),
        },
        Expr::Construct { ty, args } => {
            if let Some((import_ns, symbol_name)) = imported_modules.get(ty) {
                if let Some((owner_name, field_name)) =
                    builtin_exact_import_static_container_parts(import_ns, symbol_name)
                {
                    return Expr::Call {
                        callee: Box::new(ast::Spanned::new(
                            Expr::Field {
                                object: Box::new(ast::Spanned::new(
                                    Expr::Ident(owner_name.to_string()),
                                    ast::Span::default(),
                                )),
                                field: field_name.to_string(),
                            },
                            ast::Span::default(),
                        )),
                        args: args
                            .iter()
                            .map(|a| {
                                ast::Spanned::new(
                                    rewrite_expr_calls_for_project(&a.node, ctx, scopes),
                                    a.span.clone(),
                                )
                            })
                            .collect(),
                        type_args: vec![],
                    };
                }
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
                                    rewrite_expr_calls_for_project(&a.node, ctx, scopes),
                                    a.span.clone(),
                                )
                            })
                            .collect(),
                        type_args: vec![],
                    };
                }
            }

            Expr::Construct {
                ty: self::rewrite_construct_type_name_for_project(
                    ty,
                    RewriteTypeContext {
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
                    },
                ),
                args: args
                    .iter()
                    .map(|a| {
                        ast::Spanned::new(
                            self::rewrite_expr_calls_for_project(&a.node, ctx, scopes),
                            a.span.clone(),
                        )
                    })
                    .collect(),
            }
        }
        Expr::GenericFunctionValue { callee, type_args } => Expr::GenericFunctionValue {
            callee: Box::new(ast::Spanned::new(
                self::rewrite_expr_calls_for_project(&callee.node, ctx, scopes),
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
                        local_enums,
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
            let lambda_ctx = match ctx.expected_return_type {
                Some(ast::Type::Function(_, return_type)) => {
                    ctx.with_expected_return_type(return_type.as_ref())
                }
                _ => ctx,
            };
            let rewritten_body =
                self::rewrite_expr_calls_for_project(&body.node, lambda_ctx, scopes);
            let rewritten_body = lambda_ctx
                .expected_return_type
                .and_then(|ty| {
                    materialize_builtin_import_value_for_type(
                        &body.node,
                        ty,
                        imported_map,
                        imported_modules,
                        scopes,
                    )
                })
                .unwrap_or(rewritten_body);
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
                            local_enums,
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
        Expr::Block(stmts) => Expr::Block(rewrite_block_calls_for_project(stmts, ctx, scopes)),
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition = Box::new(ast::Spanned::new(
                self::rewrite_expr_calls_for_project(&condition.node, ctx, scopes),
                condition.span.clone(),
            ));
            push_scope(scopes);
            let then_branch = rewrite_block_calls_for_project(then_branch, ctx, scopes);
            pop_scope(scopes);
            let else_branch = else_branch.as_ref().map(|branch| {
                push_scope(scopes);
                let rewritten = rewrite_block_calls_for_project(branch, ctx, scopes);
                pop_scope(scopes);
                rewritten
            });
            Expr::If {
                condition,
                then_branch,
                else_branch,
            }
        }
        Expr::Match { expr, arms } => Expr::Match {
            expr: Box::new(ast::Spanned::new(
                materialize_builtin_import_value(
                    &expr.node,
                    imported_map,
                    imported_modules,
                    scopes,
                )
                .unwrap_or_else(|| self::rewrite_expr_calls_for_project(&expr.node, ctx, scopes)),
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
                    let body = rewrite_block_calls_for_project(&arm.body, ctx, scopes);
                    pop_scope(scopes);
                    ast::MatchArm {
                        pattern: rewritten_pattern,
                        body,
                    }
                })
                .collect(),
        },
        Expr::Await(inner) => Expr::Await(Box::new(ast::Spanned::new(
            self::rewrite_expr_calls_for_project(&inner.node, ctx, scopes),
            inner.span.clone(),
        ))),
        Expr::Try(inner) => Expr::Try(Box::new(ast::Spanned::new(
            self::rewrite_expr_calls_for_project(&inner.node, ctx, scopes),
            inner.span.clone(),
        ))),
        Expr::Borrow(inner) => Expr::Borrow(Box::new(ast::Spanned::new(
            self::rewrite_expr_calls_for_project(&inner.node, ctx, scopes),
            inner.span.clone(),
        ))),
        Expr::MutBorrow(inner) => Expr::MutBorrow(Box::new(ast::Spanned::new(
            self::rewrite_expr_calls_for_project(&inner.node, ctx, scopes),
            inner.span.clone(),
        ))),
        Expr::Deref(inner) => Expr::Deref(Box::new(ast::Spanned::new(
            self::rewrite_expr_calls_for_project(&inner.node, ctx, scopes),
            inner.span.clone(),
        ))),
        Expr::StringInterp(parts) => Expr::StringInterp(
            parts
                .iter()
                .map(|part| match part {
                    ast::StringPart::Literal(text) => ast::StringPart::Literal(text.clone()),
                    ast::StringPart::Expr(expr) => ast::StringPart::Expr(ast::Spanned::new(
                        self::rewrite_expr_calls_for_project(&expr.node, ctx, scopes),
                        expr.span.clone(),
                    )),
                })
                .collect(),
        ),
        Expr::AsyncBlock(body) => {
            let mut rewritten_body = rewrite_block_calls_for_project(body, ctx, scopes);
            if let Some(materialized) = materialize_builtin_async_tail_block(
                &rewritten_body,
                imported_map,
                imported_modules,
                scopes,
            ) {
                rewritten_body = materialized;
            }
            Expr::AsyncBlock(rewritten_body)
        }
        Expr::Require { condition, message } => Expr::Require {
            condition: Box::new(ast::Spanned::new(
                self::rewrite_expr_calls_for_project(&condition.node, ctx, scopes),
                condition.span.clone(),
            )),
            message: message.as_ref().map(|expr| {
                Box::new(ast::Spanned::new(
                    self::rewrite_expr_calls_for_project(&expr.node, ctx, scopes),
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
                    self::rewrite_expr_calls_for_project(&expr.node, ctx, scopes),
                    expr.span.clone(),
                ))
            }),
            end: end.as_ref().map(|expr| {
                Box::new(ast::Spanned::new(
                    self::rewrite_expr_calls_for_project(&expr.node, ctx, scopes),
                    expr.span.clone(),
                ))
            }),
            inclusive: *inclusive,
        },
        Expr::This => Expr::This,
        Expr::Ident(name) => {
            if is_shadowed(name, scopes) {
                Expr::Ident(name.clone())
            } else if local_classes.contains(name) {
                Expr::Ident(mangle_project_symbol(
                    current_namespace,
                    entry_namespace,
                    name,
                ))
            } else if let Some((owner_ns, class_name)) = imported_classes.get(name) {
                Expr::Ident(mangle_project_symbol(owner_ns, entry_namespace, class_name))
            } else if let Some(owner_ns) = global_class_map.get(name) {
                Expr::Ident(mangle_project_symbol(owner_ns, entry_namespace, name))
            } else if local_functions.contains(name) {
                Expr::Ident(mangle_project_symbol(
                    current_namespace,
                    entry_namespace,
                    name,
                ))
            } else if let Some((ns, symbol_name)) = imported_map.get(name) {
                if is_builtin_exact_import_canonical(symbol_name)
                    || stdlib_registry()
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
