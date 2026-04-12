use super::*;

pub(super) fn rewrite_nested_module_decl_for_project(
    decl: &ast::Spanned<Decl>,
    ctx: ModuleRewriteContext<'_>,
) -> ast::Spanned<Decl> {
    let module_rewrite_ctx = ctx;
    let module_call_ctx = ctx.call_ctx;
    let module_type_ctx = module_call_ctx.type_ctx;
    let module_prefix = ctx.module_prefix;
    let current_namespace = module_type_ctx.current_namespace;
    let entry_namespace = module_type_ctx.entry_namespace;
    let module_local_functions = module_call_ctx.local_functions;
    let module_local_classes = module_type_ctx.local_classes;
    let module_local_interfaces = module_type_ctx.local_interfaces;
    let module_local_enums = module_type_ctx.local_enums;
    let module_local_modules = module_call_ctx.local_modules;
    let imported_map = module_call_ctx.imported_map;
    let global_function_map = module_call_ctx.global_function_map;
    let imported_classes = module_type_ctx.imported_classes;
    let global_class_map = module_type_ctx.global_class_map;
    let imported_interfaces = module_type_ctx.imported_interfaces;
    let imported_enums = module_type_ctx.imported_enums;
    let global_enum_map = module_type_ctx.global_enum_map;
    let imported_modules = module_type_ctx.imported_modules;
    let global_interface_map = module_type_ctx.global_interface_map;
    let global_module_map = module_call_ctx.global_module_map;
    let module_imports = collect_import_decls(match &decl.node {
        Decl::Module(module) => &module.declarations,
        _ => &[],
    });
    let mut module_imported_map = imported_map.clone();
    let mut module_imported_classes = imported_classes.clone();
    let mut module_imported_interfaces = imported_interfaces.clone();
    let mut module_imported_enums = imported_enums.clone();
    let mut module_imported_modules = imported_modules.clone();
    extend_import_maps_with_imports(
        &module_imports,
        ImportMapSources {
            namespace_functions: &HashMap::new(),
            global_function_map,
            namespace_classes: &HashMap::new(),
            global_class_map,
            namespace_interfaces: &HashMap::new(),
            global_interface_map,
            namespace_enums: &HashMap::new(),
            global_enum_map,
            namespace_modules: &HashMap::new(),
            global_module_map,
        },
        ImportedMapsMut {
            imported_map: &mut module_imported_map,
            imported_classes: &mut module_imported_classes,
            imported_interfaces: &mut module_imported_interfaces,
            imported_enums: &mut module_imported_enums,
            imported_modules: &mut module_imported_modules,
        },
    );
    let global_class_symbols = collect_global_class_symbols(global_class_map, entry_namespace);
    let resolved_ctx = ResolvedRewriteContext {
        current_namespace,
        entry_namespace,
        imported_map: &module_imported_map,
        imported_classes: &module_imported_classes,
        imported_interfaces: &module_imported_interfaces,
        imported_enums: &module_imported_enums,
        imported_modules: &module_imported_modules,
        global_function_map,
        global_class_map,
        global_class_symbols: &global_class_symbols,
        global_interface_map,
        global_enum_map,
        global_module_map,
    };
    let rewrite_interface_reference_for_module =
        |name: &str,
         _module_prefix: &str,
         _current_namespace: &str,
         _entry_namespace: &str,
         _local_classes: &HashSet<String>,
         _local_interfaces: &HashSet<String>,
         _local_enums: &HashSet<String>,
         _local_modules: &HashSet<String>,
         _imported_classes: &ImportedMap,
         _imported_interfaces: &ImportedMap,
         _imported_enums: &ImportedMap,
         _imported_modules: &ImportedMap,
         _global_class_map: &HashMap<String, String>,
         _global_interface_map: &HashMap<String, String>,
         _global_enum_map: &HashMap<String, String>| {
            self::rewrite_interface_reference_for_module(name, module_rewrite_ctx)
        };
    let rewrite_module_local_type =
        |ty: &ast::Type,
         _module_prefix: &str,
         _current_namespace: &str,
         _entry_namespace: &str,
         _local_classes: &HashSet<String>,
         _local_interfaces: &HashSet<String>,
         _local_enums: &HashSet<String>,
         _local_modules: &HashSet<String>,
         _imported_classes: &ImportedMap,
         _global_class_map: &HashMap<String, String>,
         _imported_enums: &ImportedMap,
         _global_enum_map: &HashMap<String, String>,
         _imported_modules: &ImportedMap,
         _global_interface_map: &HashMap<String, String>| {
            self::rewrite_module_local_type(ty, module_rewrite_ctx)
        };
    let rewrite_block_calls_for_project =
        |block: &ast::Block,
         _current_namespace: &str,
         _entry_namespace: &str,
         _local_functions: &HashSet<String>,
         _imported_map: &ImportedMap,
         _global_function_map: &HashMap<String, String>,
         _local_classes: &HashSet<String>,
         _imported_classes: &ImportedMap,
         _global_class_map: &HashMap<String, String>,
         _local_interfaces: &HashSet<String>,
         _imported_interfaces: &ImportedMap,
         _global_interface_map: &HashMap<String, String>,
         _imported_enums: &ImportedMap,
         _global_enum_map: &HashMap<String, String>,
         _local_modules: &HashSet<String>,
         _imported_modules: &ImportedMap,
         _global_module_map: &HashMap<String, String>,
         scopes: &mut Vec<HashSet<String>>| {
            self::rewrite_block_calls_for_project(block, module_call_ctx, scopes)
        };
    let fix_module_local_block =
        |block: &ast::Block,
         _current_namespace: &str,
         _entry_namespace: &str,
         _module_prefix: &str,
         _local_functions: &HashSet<String>,
         _local_classes: &HashSet<String>,
         _local_interfaces: &HashSet<String>,
         _local_enums: &HashSet<String>,
         _local_modules: &HashSet<String>,
         _imported_classes: &ImportedMap,
         _global_class_map: &HashMap<String, String>,
         _imported_enums: &ImportedMap,
         _global_enum_map: &HashMap<String, String>,
         _imported_modules: &ImportedMap,
         _global_interface_map: &HashMap<String, String>| {
            self::fix_module_local_block(block, module_rewrite_ctx)
        };
    let rewrite_nested_module_decl_for_project =
        |decl: &ast::Spanned<Decl>,
         inner_prefix: &str,
         _current_namespace: &str,
         _entry_namespace: &str,
         nested_local_functions: &HashSet<String>,
         nested_local_classes: &HashSet<String>,
         nested_local_interfaces: &HashSet<String>,
         nested_local_enums: &HashSet<String>,
         nested_local_modules: &HashSet<String>,
         _imported_map: &ImportedMap,
         _global_function_map: &HashMap<String, String>,
         _imported_classes: &ImportedMap,
         _global_class_map: &HashMap<String, String>,
         _imported_interfaces: &ImportedMap,
         _imported_enums: &ImportedMap,
         _global_enum_map: &HashMap<String, String>,
         _imported_modules: &ImportedMap,
         _global_interface_map: &HashMap<String, String>,
         _global_module_map: &HashMap<String, String>| {
            let nested_call_ctx = resolved_ctx.with_locals(
                nested_local_functions,
                nested_local_classes,
                nested_local_interfaces,
                nested_local_enums,
                nested_local_modules,
            );
            self::rewrite_nested_module_decl_for_project(
                decl,
                ModuleRewriteContext {
                    module_prefix: inner_prefix,
                    call_ctx: nested_call_ctx,
                },
            )
        };
    let _ = fix_module_local_block;
    let node = match &decl.node {
        Decl::Function(func) => {
            let mut f = func.clone();
            if f.is_extern && f.extern_link_name.is_none() {
                f.extern_link_name = Some(f.name.clone());
            }
            f.generic_params = rewrite_generic_params_for_project(&f.generic_params, |bound| {
                rewrite_interface_reference_for_module(
                    bound,
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
            });
            let param_scope: HashSet<String> = f.params.iter().map(|p| p.name.clone()).collect();
            let mut scopes = vec![param_scope.clone()];
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
            f.body = self::fix_module_local_block_with_scopes(
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
                ModuleRewriteContext {
                    module_prefix,
                    call_ctx: module_call_ctx.with_expected_return_type(&f.return_type),
                },
                &[param_scope],
            );
            (f.params, f.body) = rename_shadowed_module_imports_in_callable(
                &f.params,
                &f.body,
                module_call_ctx.imported_map,
            );
            Decl::Function(f)
        }
        Decl::Class(class) => {
            let mut c = class.clone();
            c.generic_params = rewrite_generic_params_for_project(&c.generic_params, |bound| {
                rewrite_interface_reference_for_module(
                    bound,
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
            });
            c.extends = class
                .extends
                .as_ref()
                .map(|extends| rewrite_nominal_type_source_for_module(extends, module_rewrite_ctx));
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
                let mut param_scope: HashSet<String> =
                    new_ctor.params.iter().map(|p| p.name.clone()).collect();
                param_scope.insert("this".to_string());
                let mut scopes = vec![param_scope.clone()];
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
                new_ctor.body = self::fix_module_local_block_with_scopes(
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
                    module_rewrite_ctx,
                    &[param_scope],
                );
                (new_ctor.params, new_ctor.body) = rename_shadowed_module_imports_in_callable(
                    &new_ctor.params,
                    &new_ctor.body,
                    module_call_ctx.imported_map,
                );
                c.constructor = Some(new_ctor);
            }
            if let Some(dtor) = &class.destructor {
                let mut new_dtor = dtor.clone();
                let mut scopes: Vec<HashSet<String>> = vec![HashSet::from(["this".to_string()])];
                new_dtor.body = self::fix_module_local_block(
                    &rewrite_block_calls_for_project(
                        &new_dtor.body,
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
                    module_rewrite_ctx,
                );
                c.destructor = Some(new_dtor);
            }
            c.methods = class
                .methods
                .iter()
                .map(|method| {
                    let mut nm = method.clone();
                    nm.generic_params =
                        rewrite_generic_params_for_project(&nm.generic_params, |bound| {
                            rewrite_interface_reference_for_module(
                                bound,
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
                        });
                    let mut param_scope: HashSet<String> =
                        nm.params.iter().map(|p| p.name.clone()).collect();
                    param_scope.insert("this".to_string());
                    let mut scopes = vec![param_scope.clone()];
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
                    nm.body = self::fix_module_local_block_with_scopes(
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
                        ModuleRewriteContext {
                            module_prefix,
                            call_ctx: module_call_ctx.with_expected_return_type(&nm.return_type),
                        },
                        &[param_scope],
                    );
                    (nm.params, nm.body) = rename_shadowed_module_imports_in_callable(
                        &nm.params,
                        &nm.body,
                        module_call_ctx.imported_map,
                    );
                    nm
                })
                .collect();
            Decl::Class(c)
        }
        Decl::Enum(en) => {
            let mut e = en.clone();
            e.generic_params = rewrite_generic_params_for_project(&e.generic_params, |bound| {
                rewrite_interface_reference_for_module(
                    bound,
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
            rewritten.generic_params =
                rewrite_generic_params_for_project(&rewritten.generic_params, |bound| {
                    rewrite_interface_reference_for_module(
                        bound,
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
                });
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
                    let mut param_scope: HashSet<String> =
                        new_method.params.iter().map(|p| p.name.clone()).collect();
                    param_scope.insert("this".to_string());
                    let mut scopes = vec![param_scope.clone()];
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
                        self::fix_module_local_block_with_scopes(
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
                            module_rewrite_ctx,
                            &[param_scope.clone()],
                        )
                    });
                    if let Some(default_impl) = &new_method.default_impl {
                        let (params, default_impl) = rename_shadowed_module_imports_in_callable(
                            &new_method.params,
                            default_impl,
                            module_call_ctx.imported_map,
                        );
                        new_method.params = params;
                        new_method.default_impl = Some(default_impl);
                    }
                    new_method
                })
                .collect();
            Decl::Interface(rewritten)
        }
        Decl::Import(_) => decl.node.clone(),
    };

    ast::Spanned::new(node, decl.span.clone())
}
