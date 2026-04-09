use super::*;

fn sp<T>(node: T) -> ast::Spanned<T> {
    ast::Spanned::new(node, 0..0)
}

#[allow(clippy::too_many_arguments)]
fn rewrite_program_for_project(
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
    super::rewrite_program_for_project(
        program,
        &super::ProjectRewriteContext {
            current_namespace,
            entry_namespace,
            namespace_functions,
            global_function_map,
            namespace_classes,
            global_class_map,
            namespace_interfaces: namespace_modules,
            global_interface_map: global_module_map,
            namespace_enums,
            global_enum_map,
            namespace_modules,
            global_module_map,
            imports,
        },
    )
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
    imported_classes: &HashMap<String, (String, String)>,
    global_class_map: &HashMap<String, String>,
    imported_enums: &HashMap<String, (String, String)>,
    global_enum_map: &HashMap<String, String>,
    imported_modules: &HashMap<String, (String, String)>,
    global_interface_map: &HashMap<String, String>,
) -> Expr {
    let class_symbol_names = super::collect_global_class_symbols(global_class_map, entry_namespace);
    super::fix_module_local_expr(
        expr,
        super::ModuleRewriteContext {
            module_prefix,
            call_ctx: super::CallRewriteContext {
                local_functions,
                local_modules,
                class_symbol_names: &class_symbol_names,
                imported_map: &HashMap::new(),
                global_function_map: &HashMap::new(),
                global_module_map: &HashMap::new(),
                expected_return_type: None,
                type_ctx: super::RewriteTypeContext {
                    current_namespace,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_interfaces,
                    imported_interfaces: &HashMap::new(),
                    global_interface_map,
                    local_enums,
                    imported_enums,
                    global_enum_map,
                    imported_modules,
                    entry_namespace,
                },
            },
        },
    )
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
fn rewrites_namespace_alias_enum_variant_patterns() {
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
                        pattern: ast::Pattern::Variant("u.E.A".to_string(), vec!["v".to_string()]),
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
    let Stmt::Match { arms, .. } = &func.body[0].node else {
        panic!("expected match statement");
    };
    assert!(matches!(
        &arms[0].pattern,
        ast::Pattern::Variant(name, bindings)
            if name == "util__E.A" && bindings == &vec!["v".to_string()]
    ));
}

#[test]
fn rewrites_root_namespace_alias_builtin_variant_patterns() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "app".to_string(),
                alias: Some("root".to_string()),
            })),
            sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![ast::Parameter {
                    name: "value".to_string(),
                    ty: ast::Type::Named("Option".to_string()),
                    mutable: false,
                    mode: ast::ParamMode::Owned,
                }],
                is_variadic: false,
                extern_abi: None,
                extern_link_name: None,
                return_type: ast::Type::None,
                body: vec![sp(Stmt::Match {
                    expr: sp(Expr::Ident("value".to_string())),
                    arms: vec![ast::MatchArm {
                        pattern: ast::Pattern::Variant("root.Option.None".to_string(), vec![]),
                        body: vec![sp(Stmt::Expr(sp(Expr::Literal(ast::Literal::Integer(0)))))],
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
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[ImportDecl {
            path: "app".to_string(),
            alias: Some("root".to_string()),
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
        ast::Pattern::Variant(name, bindings) if name == "None" && bindings.is_empty()
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
                                    body: vec![sp(Stmt::Expr(sp(Expr::Ident("v".to_string()))))],
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
fn rewrites_namespace_alias_nested_module_function_call_type_args() {
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
                            field: "M".to_string(),
                        })),
                        field: "make".to_string(),
                    })),
                    args: vec![],
                    type_args: vec![ast::Type::Named("u.Api.Named".to_string())],
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
        &HashMap::from([
            ("app".to_string(), HashSet::from(["main".to_string()])),
            ("util".to_string(), HashSet::from(["M__make".to_string()])),
        ]),
        &HashMap::from([("M__make".to_string(), "util".to_string())]),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([(
            "util".to_string(),
            HashSet::from(["Api__Named".to_string()]),
        )]),
        &HashMap::from([("Api__Named".to_string(), "util".to_string())]),
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
        panic!("expected expression statement");
    };
    let Expr::Call {
        callee, type_args, ..
    } = &expr_stmt.node
    else {
        panic!("expected rewritten call expression");
    };
    let Expr::Ident(name) = &callee.node else {
        panic!("expected rewritten ident callee: {:?}", callee.node);
    };
    assert_eq!(name, "util__M__make");
    assert_eq!(
        type_args,
        &vec![ast::Type::Named("util__Api__Named".to_string())]
    );
}

#[test]
fn rewrites_local_module_function_call_type_args() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Module(ast::ModuleDecl {
                name: "M".to_string(),
                declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                    name: "make".to_string(),
                    generic_params: vec![ast::GenericParam {
                        name: "T".to_string(),
                        bounds: vec![],
                    }],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                }))],
            })),
            sp(Decl::Class(ast::ClassDecl {
                name: "Box".to_string(),
                generic_params: vec![],
                extends: None,
                implements: vec![],
                fields: vec![],
                constructor: None,
                destructor: None,
                methods: vec![],
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
                body: vec![sp(Stmt::Expr(sp(Expr::Call {
                    callee: Box::new(sp(Expr::Field {
                        object: Box::new(sp(Expr::Ident("M".to_string()))),
                        field: "make".to_string(),
                    })),
                    args: vec![],
                    type_args: vec![ast::Type::Named("Box".to_string())],
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
        &HashMap::from([(
            "app".to_string(),
            HashSet::from(["main".to_string(), "M__make".to_string()]),
        )]),
        &HashMap::from([("M__make".to_string(), "app".to_string())]),
        &HashMap::from([("app".to_string(), HashSet::from(["Box".to_string()]))]),
        &HashMap::from([("Box".to_string(), "app".to_string())]),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([("app".to_string(), HashSet::from(["M".to_string()]))]),
        &HashMap::from([("M".to_string(), "app".to_string())]),
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

    let Stmt::Expr(expr_stmt) = &func.body[0].node else {
        panic!("expected expression statement");
    };
    let Expr::Call {
        callee, type_args, ..
    } = &expr_stmt.node
    else {
        panic!("expected rewritten call expression");
    };
    let Expr::Ident(name) = &callee.node else {
        panic!("expected rewritten ident callee: {:?}", callee.node);
    };
    assert_eq!(name, "app__M__make");
    assert_eq!(type_args, &vec![ast::Type::Named("app__Box".to_string())]);
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
fn rewrites_namespace_alias_nested_function_interface_types_inside_construct_strings() {
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
                    name: "values".to_string(),
                    ty: ast::Type::List(Box::new(ast::Type::Function(
                        vec![ast::Type::Named("u.M.Api.Named".to_string())],
                        Box::new(ast::Type::Integer),
                    ))),
                    value: sp(Expr::Construct {
                        ty: "List<(u.M.Api.Named) -> Integer>".to_string(),
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
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([(
            "util".to_string(),
            HashSet::from(["M__Api__Named".to_string()]),
        )]),
        &HashMap::from([("M__Api__Named".to_string(), "util".to_string())]),
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
        &ast::Type::List(Box::new(ast::Type::Function(
            vec![ast::Type::Named("util__M__Api__Named".to_string())],
            Box::new(ast::Type::Integer),
        )))
    );
    let Expr::Construct { ty, .. } = &value.node else {
        panic!("expected construct expression");
    };
    assert_eq!(ty, "List<(util__M__Api__Named) -> Integer>");
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
    let global_function_map = HashMap::from([("factorial".to_string(), "math_utils".to_string())]);

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
fn rewrites_exact_stdlib_value_alias_method_receivers_to_canonical_builtin_idents() {
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
                    object: Box::new(sp(Expr::Ident("CurrentDir".to_string()))),
                    field: "length".to_string(),
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
        path: "std.system.cwd".to_string(),
        alias: Some("CurrentDir".to_string()),
    }];

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
    let Stmt::Expr(call_stmt) = &func.body[0].node else {
        panic!("expected expr statement");
    };
    let Expr::Call { callee, .. } = &call_stmt.node else {
        panic!("expected call expression");
    };
    let Expr::Field { object, field } = &callee.node else {
        panic!("expected field receiver call");
    };
    let Expr::Ident(name) = &object.node else {
        panic!("expected rewritten builtin receiver ident");
    };
    assert_eq!(name, "System__cwd");
    assert_eq!(field, "length");
}

#[test]
fn rewrites_builtin_exact_value_alias_method_receivers_to_materialized_values() {
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
                    object: Box::new(sp(Expr::Ident("Empty".to_string()))),
                    field: "is_none".to_string(),
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
        path: "Option.None".to_string(),
        alias: Some("Empty".to_string()),
    }];

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
    let Stmt::Expr(call_stmt) = &func.body[0].node else {
        panic!("expected expr statement");
    };
    let Expr::Call { callee, .. } = &call_stmt.node else {
        panic!("expected call expression");
    };
    let Expr::Field { object, field } = &callee.node else {
        panic!("expected field receiver call");
    };
    let Expr::Call {
        callee,
        args,
        type_args,
    } = &object.node
    else {
        panic!("expected materialized builtin value call");
    };
    assert!(args.is_empty(), "Option.none() should stay zero-arg");
    assert!(
        type_args.is_empty(),
        "Option.none() should not gain type args"
    );
    let Expr::Field {
        object,
        field: builtin_field,
    } = &callee.node
    else {
        panic!("expected Option.none() field callee");
    };
    let Expr::Ident(owner) = &object.node else {
        panic!("expected Option static owner ident");
    };
    assert_eq!(owner, "Option");
    assert_eq!(builtin_field, "none");
    assert_eq!(field, "is_none");
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
                    ty: ast::Type::Function(vec![ast::Type::Integer], Box::new(ast::Type::Integer)),
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
        &HashMap::from([("app".to_string(), HashSet::from(["M".to_string()]))]),
        &HashMap::from([("M".to_string(), "app".to_string())]),
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
                    ty: ast::Type::Function(vec![ast::Type::Integer], Box::new(ast::Type::Integer)),
                    value: sp(Expr::IfExpr {
                        condition: Box::new(sp(Expr::Literal(ast::Literal::Boolean(true)))),
                        then_branch: vec![sp(Stmt::Expr(sp(Expr::Ident("inc".to_string()))))],
                        else_branch: Some(vec![sp(Stmt::Expr(sp(Expr::Ident("dec".to_string()))))]),
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
            Decl::Function(func) if func.name == "app__main" || func.name == "main" => Some(func),
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
                    ty: ast::Type::Function(vec![ast::Type::Integer], Box::new(ast::Type::Integer)),
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
                    ty: ast::Type::Function(vec![ast::Type::Integer], Box::new(ast::Type::Integer)),
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
fn rewrites_module_local_generic_function_value_type_args() {
    let rewritten = fix_module_local_expr(
        &Expr::GenericFunctionValue {
            callee: Box::new(sp(Expr::Ident("id".to_string()))),
            type_args: vec![ast::Type::Named("Box".to_string())],
        },
        "app",
        "app",
        "M",
        &HashSet::from(["id".to_string()]),
        &HashSet::from(["Box".to_string()]),
        &HashSet::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashMap::from([("M__Box".to_string(), "app".to_string())]),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    );

    let Expr::GenericFunctionValue { callee, type_args } = rewritten else {
        panic!("expected generic function value");
    };
    let Expr::Ident(name) = &callee.node else {
        panic!("expected rewritten function identifier");
    };
    assert_eq!(name, "id");
    assert_eq!(type_args, vec![ast::Type::Named("app__M__Box".to_string())]);
}

#[test]
fn rewrites_already_mangled_module_local_nested_module_members() {
    let rewritten_ctor = fix_module_local_expr(
        &Expr::Call {
            callee: Box::new(sp(Expr::Field {
                object: Box::new(sp(Expr::Ident("app__M__N".to_string()))),
                field: "Box".to_string(),
            })),
            args: vec![sp(Expr::Literal(ast::Literal::Integer(55)))],
            type_args: vec![],
        },
        "app",
        "app",
        "M",
        &HashSet::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashSet::from(["N".to_string()]),
        &HashMap::new(),
        &HashMap::from([("M__N__Box".to_string(), "app".to_string())]),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    );

    let Expr::Construct { ty, args } = rewritten_ctor else {
        panic!("expected nested module constructor rewrite");
    };
    assert_eq!(ty, "app__M__N__Box");
    assert_eq!(args.len(), 1);

    let rewritten_fn_value = fix_module_local_expr(
        &Expr::GenericFunctionValue {
            callee: Box::new(sp(Expr::Field {
                object: Box::new(sp(Expr::Ident("app__M__N".to_string()))),
                field: "id".to_string(),
            })),
            type_args: vec![ast::Type::Named("N.Box".to_string())],
        },
        "app",
        "app",
        "M",
        &HashSet::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashSet::new(),
        &HashSet::from(["N".to_string()]),
        &HashMap::new(),
        &HashMap::from([("M__N__Box".to_string(), "app".to_string())]),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    );

    let Expr::GenericFunctionValue { callee, type_args } = rewritten_fn_value else {
        panic!("expected generic function value");
    };
    let Expr::Ident(name) = &callee.node else {
        panic!("expected rewritten nested module function identifier");
    };
    assert_eq!(name, "app__M__N__id");
    assert_eq!(
        type_args,
        vec![ast::Type::Named("app__M__N__Box".to_string())]
    );
}

#[test]
fn rewrites_program_for_project_module_local_nested_module_members() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "M".to_string(),
            declarations: vec![
                sp(Decl::Module(ast::ModuleDecl {
                    name: "N".to_string(),
                    declarations: vec![
                        sp(Decl::Class(ast::ClassDecl {
                            name: "Box".to_string(),
                            generic_params: vec![],
                            extends: None,
                            implements: vec![],
                            fields: vec![ast::Field {
                                name: "value".to_string(),
                                ty: ast::Type::Integer,
                                mutable: false,
                                visibility: ast::Visibility::Private,
                            }],
                            constructor: None,
                            destructor: None,
                            methods: vec![],
                            visibility: ast::Visibility::Private,
                        })),
                        sp(Decl::Function(ast::FunctionDecl {
                            name: "id".to_string(),
                            generic_params: vec![ast::GenericParam {
                                name: "T".to_string(),
                                bounds: vec![],
                            }],
                            params: vec![ast::Parameter {
                                name: "value".to_string(),
                                ty: ast::Type::Named("T".to_string()),
                                mutable: false,
                                mode: ast::ParamMode::Owned,
                            }],
                            is_variadic: false,
                            extern_abi: None,
                            extern_link_name: None,
                            return_type: ast::Type::Named("T".to_string()),
                            body: vec![sp(Stmt::Return(Some(sp(Expr::Ident(
                                "value".to_string(),
                            )))))],
                            is_async: false,
                            is_extern: false,
                            visibility: ast::Visibility::Private,
                            attributes: vec![],
                        })),
                    ],
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "run".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::Integer,
                    body: vec![
                        sp(Stmt::Let {
                            name: "value".to_string(),
                            ty: ast::Type::Named("N.Box".to_string()),
                            value: sp(Expr::Call {
                                callee: Box::new(sp(Expr::Field {
                                    object: Box::new(sp(Expr::Ident("N".to_string()))),
                                    field: "Box".to_string(),
                                })),
                                args: vec![sp(Expr::Literal(ast::Literal::Integer(55)))],
                                type_args: vec![],
                            }),
                            mutable: false,
                        }),
                        sp(Stmt::Let {
                            name: "f".to_string(),
                            ty: ast::Type::Function(
                                vec![ast::Type::Named("N.Box".to_string())],
                                Box::new(ast::Type::Named("N.Box".to_string())),
                            ),
                            value: sp(Expr::GenericFunctionValue {
                                callee: Box::new(sp(Expr::Field {
                                    object: Box::new(sp(Expr::Ident("N".to_string()))),
                                    field: "id".to_string(),
                                })),
                                type_args: vec![ast::Type::Named("N.Box".to_string())],
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
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::from([("app".to_string(), HashSet::from(["M__N__id".to_string()]))]),
        &HashMap::from([("M__N__id".to_string(), "app".to_string())]),
        &HashMap::from([("app".to_string(), HashSet::from(["M__N__Box".to_string()]))]),
        &HashMap::from([("M__N__Box".to_string(), "app".to_string())]),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) if module.name == "app__M" => Some(module),
            _ => None,
        })
        .expect("expected rewritten module declaration");
    let run = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Function(func) if func.name == "run" => Some(func),
            _ => None,
        })
        .expect("expected run function");

    let Stmt::Let { value, .. } = &run.body[0].node else {
        panic!("expected constructor let statement");
    };
    let Expr::Construct { ty, .. } = &value.node else {
        panic!("expected rewritten nested module constructor");
    };
    assert_eq!(ty, "app__M__N__Box");

    let Stmt::Let { value, .. } = &run.body[1].node else {
        panic!("expected generic function value let statement");
    };
    let Expr::GenericFunctionValue { callee, type_args } = &value.node else {
        panic!("expected rewritten generic function value");
    };
    let Expr::Ident(name) = &callee.node else {
        panic!("expected rewritten nested module function ident");
    };
    assert_eq!(name, "app__M__N__id");
    assert_eq!(
        type_args,
        &vec![ast::Type::Named("app__M__N__Box".to_string())]
    );
}

#[test]
fn rewrites_module_local_lambda_parameter_types() {
    let rewritten = fix_module_local_expr(
        &Expr::Lambda {
            params: vec![ast::Parameter {
                name: "value".to_string(),
                ty: ast::Type::Named("Named".to_string()),
                mutable: false,
                mode: ast::ParamMode::Owned,
            }],
            body: Box::new(sp(Expr::Literal(ast::Literal::Integer(0)))),
        },
        "app",
        "app",
        "M",
        &HashSet::new(),
        &HashSet::new(),
        &HashSet::from(["Named".to_string()]),
        &HashSet::new(),
        &HashSet::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([("M__Named".to_string(), "app".to_string())]),
    );

    let Expr::Lambda { params, .. } = rewritten else {
        panic!("expected lambda expression");
    };
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].ty, ast::Type::Named("app__M__Named".to_string()));
}

#[test]
fn rewrites_module_local_nested_enum_variant_patterns() {
    let rewritten = rewrite_pattern_for_module(
        &ast::Pattern::Variant("N.E.A".to_string(), vec!["v".to_string()]),
        "M",
        "app",
        "app",
        &HashSet::from(["N".to_string()]),
        &HashMap::new(),
        &HashMap::from([("M__N__E".to_string(), "app".to_string())]),
    );

    assert!(matches!(
        rewritten,
        ast::Pattern::Variant(name, bindings)
            if name == "app__M__N__E.A" && bindings == vec!["v".to_string()]
    ));
}

#[test]
fn rewrites_root_namespace_alias_builtin_static_constructor_calls() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "app".to_string(),
                alias: Some("root".to_string()),
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
                            object: Box::new(sp(Expr::Ident("root".to_string()))),
                            field: "Option".to_string(),
                        })),
                        field: "None".to_string(),
                    })),
                    args: vec![],
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
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[ImportDecl {
            path: "app".to_string(),
            alias: Some("root".to_string()),
        }],
    );

    let func = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Function(func) if func.name == "main" => Some(func),
            _ => None,
        })
        .expect("expected function declaration");
    let Stmt::Expr(call_stmt) = &func.body[0].node else {
        panic!("expected expr statement");
    };
    let Expr::Call {
        callee,
        args,
        type_args,
    } = &call_stmt.node
    else {
        panic!("expected call expression");
    };
    assert!(args.is_empty());
    assert!(type_args.is_empty());
    let Expr::Field { object, field } = &callee.node else {
        panic!("expected rewritten builtin field call");
    };
    assert!(matches!(&object.node, Expr::Ident(name) if name == "Option"));
    assert_eq!(field, "none");
}

#[test]
fn preserves_root_namespace_alias_builtin_function_values() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "app".to_string(),
                alias: Some("root".to_string()),
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
                    name: "empty".to_string(),
                    ty: ast::Type::Function(
                        vec![],
                        Box::new(ast::Type::Named("Option".to_string())),
                    ),
                    value: sp(Expr::Field {
                        object: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("root".to_string()))),
                            field: "Option".to_string(),
                        })),
                        field: "None".to_string(),
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
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[ImportDecl {
            path: "app".to_string(),
            alias: Some("root".to_string()),
        }],
    );

    let func = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Function(func) if func.name == "main" => Some(func),
            _ => None,
        })
        .expect("expected function declaration");
    let Stmt::Let { value, .. } = &func.body[0].node else {
        panic!("expected let statement");
    };
    assert!(matches!(&value.node, Expr::Ident(name) if name == "Option__none"));
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

#[test]
fn rewrites_class_extends_namespace_alias_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Class(ast::ClassDecl {
                name: "Child".to_string(),
                generic_params: vec![],
                extends: Some("u.Base".to_string()),
                implements: vec![],
                fields: vec![],
                constructor: None,
                destructor: None,
                methods: vec![],
                visibility: ast::Visibility::Private,
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([("util".to_string(), HashSet::from(["Base".to_string()]))]),
        &HashMap::from([("Base".to_string(), "util".to_string())]),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let class = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Class(class) => Some(class),
            _ => None,
        })
        .expect("expected class declaration");
    assert_eq!(class.extends.as_deref(), Some("util__Base"));
}

#[test]
fn rewrites_class_extends_nested_namespace_alias_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Class(ast::ClassDecl {
                name: "Child".to_string(),
                generic_params: vec![],
                extends: Some("u.Api.Base".to_string()),
                implements: vec![],
                fields: vec![],
                constructor: None,
                destructor: None,
                methods: vec![],
                visibility: ast::Visibility::Private,
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([("util".to_string(), HashSet::from(["Api__Base".to_string()]))]),
        &HashMap::from([("Api__Base".to_string(), "util".to_string())]),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let class = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Class(class) => Some(class),
            _ => None,
        })
        .expect("expected class declaration");
    assert_eq!(class.extends.as_deref(), Some("util__Api__Base"));
}

#[test]
fn rewrites_class_implements_namespace_alias_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Class(ast::ClassDecl {
                name: "Child".to_string(),
                generic_params: vec![],
                extends: None,
                implements: vec!["u.Printable".to_string()],
                fields: vec![],
                constructor: None,
                destructor: None,
                methods: vec![],
                visibility: ast::Visibility::Private,
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([("util".to_string(), HashSet::from(["Printable".to_string()]))]),
        &HashMap::from([("Printable".to_string(), "util".to_string())]),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let class = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Class(class) => Some(class),
            _ => None,
        })
        .expect("expected class declaration");
    assert_eq!(class.implements, vec!["util__Printable".to_string()]);
}

#[test]
fn rewrites_class_implements_nested_namespace_alias_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Class(ast::ClassDecl {
                name: "Child".to_string(),
                generic_params: vec![],
                extends: None,
                implements: vec!["u.Api.Printable".to_string()],
                fields: vec![],
                constructor: None,
                destructor: None,
                methods: vec![],
                visibility: ast::Visibility::Private,
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([(
            "util".to_string(),
            HashSet::from(["Api__Printable".to_string()]),
        )]),
        &HashMap::from([("Api__Printable".to_string(), "util".to_string())]),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let class = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Class(class) => Some(class),
            _ => None,
        })
        .expect("expected class declaration");
    assert_eq!(class.implements, vec!["util__Api__Printable".to_string()]);
}

#[test]
fn rewrites_class_implements_multiple_namespace_alias_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Class(ast::ClassDecl {
                name: "Child".to_string(),
                generic_params: vec![],
                extends: None,
                implements: vec!["u.Named".to_string(), "u.Printable".to_string()],
                fields: vec![],
                constructor: None,
                destructor: None,
                methods: vec![],
                visibility: ast::Visibility::Private,
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([(
            "util".to_string(),
            HashSet::from(["Named".to_string(), "Printable".to_string()]),
        )]),
        &HashMap::from([
            ("Named".to_string(), "util".to_string()),
            ("Printable".to_string(), "util".to_string()),
        ]),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let class = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Class(class) => Some(class),
            _ => None,
        })
        .expect("expected class declaration");
    assert_eq!(
        class.implements,
        vec!["util__Named".to_string(), "util__Printable".to_string()]
    );
}

#[test]
fn rewrites_interface_extends_namespace_alias_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Interface(ast::InterfaceDecl {
                name: "Child".to_string(),
                generic_params: vec![],
                extends: vec!["u.Named".to_string()],
                methods: vec![],
                visibility: ast::Visibility::Private,
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([("util".to_string(), HashSet::from(["Named".to_string()]))]),
        &HashMap::from([("Named".to_string(), "util".to_string())]),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let interface = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Interface(interface) => Some(interface),
            _ => None,
        })
        .expect("expected interface declaration");
    assert_eq!(interface.extends, vec!["util__Named".to_string()]);
}

#[test]
fn rewrites_interface_extends_nested_namespace_alias_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Interface(ast::InterfaceDecl {
                name: "Child".to_string(),
                generic_params: vec![],
                extends: vec!["u.Api.Named".to_string()],
                methods: vec![],
                visibility: ast::Visibility::Private,
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([(
            "util".to_string(),
            HashSet::from(["Api__Named".to_string()]),
        )]),
        &HashMap::from([("Api__Named".to_string(), "util".to_string())]),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let interface = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Interface(interface) => Some(interface),
            _ => None,
        })
        .expect("expected interface declaration");
    assert_eq!(interface.extends, vec!["util__Api__Named".to_string()]);
}

#[test]
fn rewrites_interface_extends_generic_namespace_alias_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Interface(ast::InterfaceDecl {
                name: "Child".to_string(),
                generic_params: vec![],
                extends: vec!["u.Api.Reader<String>".to_string()],
                methods: vec![],
                visibility: ast::Visibility::Private,
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([(
            "util".to_string(),
            HashSet::from(["Api__Reader".to_string()]),
        )]),
        &HashMap::from([("Api__Reader".to_string(), "util".to_string())]),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let interface = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Interface(interface) => Some(interface),
            _ => None,
        })
        .expect("expected interface declaration");
    assert_eq!(
        interface.extends,
        vec!["util__Api__Reader<String>".to_string()]
    );
}

#[test]
fn rewrites_interface_extends_generic_exact_import_alias_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util.Api.Reader".to_string(),
                alias: Some("ReaderAlias".to_string()),
            })),
            sp(Decl::Interface(ast::InterfaceDecl {
                name: "Child".to_string(),
                generic_params: vec![],
                extends: vec!["ReaderAlias<String>".to_string()],
                methods: vec![],
                visibility: ast::Visibility::Private,
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([(
            "util".to_string(),
            HashSet::from(["Api__Reader".to_string()]),
        )]),
        &HashMap::from([("Api__Reader".to_string(), "util".to_string())]),
        &[ImportDecl {
            path: "util.Api.Reader".to_string(),
            alias: Some("ReaderAlias".to_string()),
        }],
    );

    let interface = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Interface(interface) => Some(interface),
            _ => None,
        })
        .expect("expected interface declaration");
    assert_eq!(
        interface.extends,
        vec!["util__Api__Reader<String>".to_string()]
    );
}

#[test]
fn rewrites_interface_extends_multiple_namespace_alias_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Interface(ast::InterfaceDecl {
                name: "Child".to_string(),
                generic_params: vec![],
                extends: vec!["u.Named".to_string(), "u.Printable".to_string()],
                methods: vec![],
                visibility: ast::Visibility::Private,
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([(
            "util".to_string(),
            HashSet::from(["Named".to_string(), "Printable".to_string()]),
        )]),
        &HashMap::from([
            ("Named".to_string(), "util".to_string()),
            ("Printable".to_string(), "util".to_string()),
        ]),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let interface = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Interface(interface) => Some(interface),
            _ => None,
        })
        .expect("expected interface declaration");
    assert_eq!(
        interface.extends,
        vec!["util__Named".to_string(), "util__Printable".to_string()]
    );
}

#[test]
fn rewrites_nested_module_class_extends_namespace_alias_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Module(ast::ModuleDecl {
                name: "M".to_string(),
                declarations: vec![sp(Decl::Class(ast::ClassDecl {
                    name: "Child".to_string(),
                    generic_params: vec![],
                    extends: Some("u.Base".to_string()),
                    implements: vec![],
                    fields: vec![],
                    constructor: None,
                    destructor: None,
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                }))],
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([("util".to_string(), HashSet::from(["Base".to_string()]))]),
        &HashMap::from([("Base".to_string(), "util".to_string())]),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let Decl::Class(class) = &module.declarations[0].node else {
        panic!("expected nested class declaration");
    };
    assert_eq!(class.extends.as_deref(), Some("util__Base"));
}

#[test]
fn rewrites_nested_module_interface_extends_namespace_alias_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Module(ast::ModuleDecl {
                name: "M".to_string(),
                declarations: vec![sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Child".to_string(),
                    generic_params: vec![],
                    extends: vec!["u.Named".to_string()],
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                }))],
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([("util".to_string(), HashSet::from(["Named".to_string()]))]),
        &HashMap::from([("Named".to_string(), "util".to_string())]),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let Decl::Interface(interface) = &module.declarations[0].node else {
        panic!("expected nested interface declaration");
    };
    assert_eq!(interface.extends, vec!["util__Named".to_string()]);
}

#[test]
fn rewrites_nested_module_construct_type_strings_with_local_function_interface_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "M".to_string(),
            declarations: vec![
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Named".to_string(),
                    generic_params: vec![],
                    extends: vec![],
                    methods: vec![ast::InterfaceMethod {
                        name: "name".to_string(),
                        params: vec![],
                        return_type: ast::Type::Integer,
                        default_impl: None,
                    }],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "make".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Let {
                        name: "values".to_string(),
                        ty: ast::Type::List(Box::new(ast::Type::Function(
                            vec![ast::Type::Named("Named".to_string())],
                            Box::new(ast::Type::Integer),
                        ))),
                        value: sp(Expr::Construct {
                            ty: "List<(Named) -> Integer>".to_string(),
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
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let function = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Function(func) if func.name == "make" => Some(func),
            _ => None,
        })
        .expect("expected nested function declaration");
    let Stmt::Let { ty, value, .. } = &function.body[0].node else {
        panic!("expected let statement");
    };
    assert_eq!(
        ty,
        &ast::Type::List(Box::new(ast::Type::Function(
            vec![ast::Type::Named("app__M__Named".to_string())],
            Box::new(ast::Type::Integer),
        )))
    );
    let Expr::Construct { ty, .. } = &value.node else {
        panic!("expected construct expression");
    };
    assert_eq!(ty, "List<(app__M__Named) -> Integer>");
}

#[test]
fn rewrites_nested_module_constructor_type_args_with_local_interface_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "M".to_string(),
            declarations: vec![
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Named".to_string(),
                    generic_params: vec![],
                    extends: vec![],
                    methods: vec![ast::InterfaceMethod {
                        name: "name".to_string(),
                        params: vec![],
                        return_type: ast::Type::Integer,
                        default_impl: None,
                    }],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Class(ast::ClassDecl {
                    name: "Box".to_string(),
                    generic_params: vec![ast::GenericParam {
                        name: "T".to_string(),
                        bounds: vec![],
                    }],
                    extends: None,
                    implements: vec![],
                    fields: vec![],
                    constructor: Some(ast::Constructor {
                        params: vec![],
                        body: vec![],
                    }),
                    destructor: None,
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "make".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::None,
                    body: vec![sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("app__M".to_string()))),
                            field: "Box".to_string(),
                        })),
                        args: vec![],
                        type_args: vec![ast::Type::Named("Named".to_string())],
                    })))],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let function = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Function(func) if func.name == "make" => Some(func),
            _ => None,
        })
        .expect("expected nested function declaration");
    let Stmt::Expr(expr) = &function.body[0].node else {
        panic!("expected expression statement");
    };
    let Expr::Construct { ty, .. } = &expr.node else {
        panic!("expected rewritten constructor expression");
    };
    assert_eq!(ty, "app__M__Box<app__M__Named>");
}

#[test]
fn rewrites_module_class_implements_local_interface_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "M".to_string(),
            declarations: vec![
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Named".to_string(),
                    generic_params: vec![],
                    extends: vec![],
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Class(ast::ClassDecl {
                    name: "Child".to_string(),
                    generic_params: vec![],
                    extends: None,
                    implements: vec!["Named".to_string()],
                    fields: vec![],
                    constructor: None,
                    destructor: None,
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let class = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Class(class) => Some(class),
            _ => None,
        })
        .expect("expected class declaration");
    assert_eq!(class.implements, vec!["app__M__Named".to_string()]);
}

#[test]
fn rewrites_module_interface_extends_local_interface_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "M".to_string(),
            declarations: vec![
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Named".to_string(),
                    generic_params: vec![],
                    extends: vec![],
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Child".to_string(),
                    generic_params: vec![],
                    extends: vec!["Named".to_string()],
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let interface = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Interface(interface) if interface.name == "Child" => Some(interface),
            _ => None,
        })
        .expect("expected child interface declaration");
    assert_eq!(interface.extends, vec!["app__M__Named".to_string()]);
}

#[test]
fn rewrites_module_class_implements_local_generic_interface_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "M".to_string(),
            declarations: vec![
                sp(Decl::Class(ast::ClassDecl {
                    name: "Payload".to_string(),
                    generic_params: vec![],
                    extends: None,
                    implements: vec![],
                    fields: vec![],
                    constructor: None,
                    destructor: None,
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Named".to_string(),
                    generic_params: vec![ast::GenericParam {
                        name: "T".to_string(),
                        bounds: vec![],
                    }],
                    extends: vec![],
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Class(ast::ClassDecl {
                    name: "Child".to_string(),
                    generic_params: vec![],
                    extends: None,
                    implements: vec!["Named<Payload>".to_string()],
                    fields: vec![],
                    constructor: None,
                    destructor: None,
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let class = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Class(class) if class.name == "Child" => Some(class),
            _ => None,
        })
        .expect("expected child class declaration");
    assert_eq!(
        class.implements,
        vec!["app__M__Named<app__M__Payload>".to_string()]
    );
}

#[test]
fn rewrites_module_interface_extends_local_generic_interface_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "M".to_string(),
            declarations: vec![
                sp(Decl::Class(ast::ClassDecl {
                    name: "Payload".to_string(),
                    generic_params: vec![],
                    extends: None,
                    implements: vec![],
                    fields: vec![],
                    constructor: None,
                    destructor: None,
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Named".to_string(),
                    generic_params: vec![ast::GenericParam {
                        name: "T".to_string(),
                        bounds: vec![],
                    }],
                    extends: vec![],
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Child".to_string(),
                    generic_params: vec![],
                    extends: vec!["Named<Payload>".to_string()],
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let interface = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Interface(interface) if interface.name == "Child" => Some(interface),
            _ => None,
        })
        .expect("expected child interface declaration");
    assert_eq!(
        interface.extends,
        vec!["app__M__Named<app__M__Payload>".to_string()]
    );
}

#[test]
fn rewrites_module_class_implements_local_nested_interface_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "M".to_string(),
            declarations: vec![
                sp(Decl::Module(ast::ModuleDecl {
                    name: "Api".to_string(),
                    declarations: vec![sp(Decl::Interface(ast::InterfaceDecl {
                        name: "Named".to_string(),
                        generic_params: vec![],
                        extends: vec![],
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    }))],
                })),
                sp(Decl::Class(ast::ClassDecl {
                    name: "Child".to_string(),
                    generic_params: vec![],
                    extends: None,
                    implements: vec!["Api.Named".to_string()],
                    fields: vec![],
                    constructor: None,
                    destructor: None,
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let class = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Class(class) => Some(class),
            _ => None,
        })
        .expect("expected class declaration");
    assert_eq!(class.implements, vec!["app__M__Api__Named".to_string()]);
}

#[test]
fn rewrites_module_interface_extends_local_nested_interface_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "M".to_string(),
            declarations: vec![
                sp(Decl::Module(ast::ModuleDecl {
                    name: "Api".to_string(),
                    declarations: vec![sp(Decl::Interface(ast::InterfaceDecl {
                        name: "Named".to_string(),
                        generic_params: vec![],
                        extends: vec![],
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    }))],
                })),
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Child".to_string(),
                    generic_params: vec![],
                    extends: vec!["Api.Named".to_string()],
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let interface = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Interface(interface) if interface.name == "Child" => Some(interface),
            _ => None,
        })
        .expect("expected child interface declaration");
    assert_eq!(interface.extends, vec!["app__M__Api__Named".to_string()]);
}

#[test]
fn rewrites_module_class_implements_multiple_local_interfaces() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "M".to_string(),
            declarations: vec![
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Named".to_string(),
                    generic_params: vec![],
                    extends: vec![],
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Printable".to_string(),
                    generic_params: vec![],
                    extends: vec![],
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Class(ast::ClassDecl {
                    name: "Child".to_string(),
                    generic_params: vec![],
                    extends: None,
                    implements: vec!["Named".to_string(), "Printable".to_string()],
                    fields: vec![],
                    constructor: None,
                    destructor: None,
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let class = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Class(class) => Some(class),
            _ => None,
        })
        .expect("expected class declaration");
    assert_eq!(
        class.implements,
        vec!["app__M__Named".to_string(), "app__M__Printable".to_string()]
    );
}

#[test]
fn rewrites_module_interface_extends_multiple_local_interfaces() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "M".to_string(),
            declarations: vec![
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Named".to_string(),
                    generic_params: vec![],
                    extends: vec![],
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Printable".to_string(),
                    generic_params: vec![],
                    extends: vec![],
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
                sp(Decl::Interface(ast::InterfaceDecl {
                    name: "Child".to_string(),
                    generic_params: vec![],
                    extends: vec!["Named".to_string(), "Printable".to_string()],
                    methods: vec![],
                    visibility: ast::Visibility::Private,
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let interface = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Interface(interface) if interface.name == "Child" => Some(interface),
            _ => None,
        })
        .expect("expected child interface declaration");
    assert_eq!(
        interface.extends,
        vec!["app__M__Named".to_string(), "app__M__Printable".to_string()]
    );
}

#[test]
fn rewrites_nested_module_class_implements_local_interface_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "Outer".to_string(),
            declarations: vec![sp(Decl::Module(ast::ModuleDecl {
                name: "Inner".to_string(),
                declarations: vec![
                    sp(Decl::Interface(ast::InterfaceDecl {
                        name: "Named".to_string(),
                        generic_params: vec![],
                        extends: vec![],
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    })),
                    sp(Decl::Class(ast::ClassDecl {
                        name: "Child".to_string(),
                        generic_params: vec![],
                        extends: None,
                        implements: vec!["Named".to_string()],
                        fields: vec![],
                        constructor: None,
                        destructor: None,
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    })),
                ],
            }))],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let outer = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected outer module declaration");
    let inner = outer
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected inner module declaration");
    let class = inner
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Class(class) => Some(class),
            _ => None,
        })
        .expect("expected class declaration");
    assert_eq!(
        class.implements,
        vec!["app__Outer__Inner__Named".to_string()]
    );
}

#[test]
fn rewrites_nested_module_interface_extends_local_interface_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "Outer".to_string(),
            declarations: vec![sp(Decl::Module(ast::ModuleDecl {
                name: "Inner".to_string(),
                declarations: vec![
                    sp(Decl::Interface(ast::InterfaceDecl {
                        name: "Named".to_string(),
                        generic_params: vec![],
                        extends: vec![],
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    })),
                    sp(Decl::Interface(ast::InterfaceDecl {
                        name: "Child".to_string(),
                        generic_params: vec![],
                        extends: vec!["Named".to_string()],
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    })),
                ],
            }))],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let outer = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected outer module declaration");
    let inner = outer
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected inner module declaration");
    let interface = inner
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Interface(interface) if interface.name == "Child" => Some(interface),
            _ => None,
        })
        .expect("expected child interface declaration");
    assert_eq!(
        interface.extends,
        vec!["app__Outer__Inner__Named".to_string()]
    );
}

#[test]
fn rewrites_nested_module_class_implements_local_nested_interface_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "Outer".to_string(),
            declarations: vec![sp(Decl::Module(ast::ModuleDecl {
                name: "Inner".to_string(),
                declarations: vec![
                    sp(Decl::Module(ast::ModuleDecl {
                        name: "Api".to_string(),
                        declarations: vec![sp(Decl::Interface(ast::InterfaceDecl {
                            name: "Named".to_string(),
                            generic_params: vec![],
                            extends: vec![],
                            methods: vec![],
                            visibility: ast::Visibility::Private,
                        }))],
                    })),
                    sp(Decl::Class(ast::ClassDecl {
                        name: "Child".to_string(),
                        generic_params: vec![],
                        extends: None,
                        implements: vec!["Api.Named".to_string()],
                        fields: vec![],
                        constructor: None,
                        destructor: None,
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    })),
                ],
            }))],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let outer = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected outer module declaration");
    let inner = outer
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected inner module declaration");
    let class = inner
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Class(class) => Some(class),
            _ => None,
        })
        .expect("expected class declaration");
    assert_eq!(
        class.implements,
        vec!["app__Outer__Inner__Api__Named".to_string()]
    );
}

#[test]
fn rewrites_nested_module_interface_extends_local_nested_interface_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "Outer".to_string(),
            declarations: vec![sp(Decl::Module(ast::ModuleDecl {
                name: "Inner".to_string(),
                declarations: vec![
                    sp(Decl::Module(ast::ModuleDecl {
                        name: "Api".to_string(),
                        declarations: vec![sp(Decl::Interface(ast::InterfaceDecl {
                            name: "Named".to_string(),
                            generic_params: vec![],
                            extends: vec![],
                            methods: vec![],
                            visibility: ast::Visibility::Private,
                        }))],
                    })),
                    sp(Decl::Interface(ast::InterfaceDecl {
                        name: "Child".to_string(),
                        generic_params: vec![],
                        extends: vec!["Api.Named".to_string()],
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    })),
                ],
            }))],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let outer = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected outer module declaration");
    let inner = outer
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected inner module declaration");
    let interface = inner
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Interface(interface) if interface.name == "Child" => Some(interface),
            _ => None,
        })
        .expect("expected child interface declaration");
    assert_eq!(
        interface.extends,
        vec!["app__Outer__Inner__Api__Named".to_string()]
    );
}

#[test]
fn rewrites_nested_module_class_implements_multiple_local_interfaces() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "Outer".to_string(),
            declarations: vec![sp(Decl::Module(ast::ModuleDecl {
                name: "Inner".to_string(),
                declarations: vec![
                    sp(Decl::Interface(ast::InterfaceDecl {
                        name: "Named".to_string(),
                        generic_params: vec![],
                        extends: vec![],
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    })),
                    sp(Decl::Interface(ast::InterfaceDecl {
                        name: "Printable".to_string(),
                        generic_params: vec![],
                        extends: vec![],
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    })),
                    sp(Decl::Class(ast::ClassDecl {
                        name: "Child".to_string(),
                        generic_params: vec![],
                        extends: None,
                        implements: vec!["Named".to_string(), "Printable".to_string()],
                        fields: vec![],
                        constructor: None,
                        destructor: None,
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    })),
                ],
            }))],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let outer = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected outer module declaration");
    let inner = outer
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected inner module declaration");
    let class = inner
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Class(class) => Some(class),
            _ => None,
        })
        .expect("expected class declaration");
    assert_eq!(
        class.implements,
        vec![
            "app__Outer__Inner__Named".to_string(),
            "app__Outer__Inner__Printable".to_string()
        ]
    );
}

#[test]
fn rewrites_nested_module_interface_extends_multiple_local_interfaces() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "Outer".to_string(),
            declarations: vec![sp(Decl::Module(ast::ModuleDecl {
                name: "Inner".to_string(),
                declarations: vec![
                    sp(Decl::Interface(ast::InterfaceDecl {
                        name: "Named".to_string(),
                        generic_params: vec![],
                        extends: vec![],
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    })),
                    sp(Decl::Interface(ast::InterfaceDecl {
                        name: "Printable".to_string(),
                        generic_params: vec![],
                        extends: vec![],
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    })),
                    sp(Decl::Interface(ast::InterfaceDecl {
                        name: "Child".to_string(),
                        generic_params: vec![],
                        extends: vec!["Named".to_string(), "Printable".to_string()],
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    })),
                ],
            }))],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let outer = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected outer module declaration");
    let inner = outer
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected inner module declaration");
    let interface = inner
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Interface(interface) if interface.name == "Child" => Some(interface),
            _ => None,
        })
        .expect("expected child interface declaration");
    assert_eq!(
        interface.extends,
        vec![
            "app__Outer__Inner__Named".to_string(),
            "app__Outer__Inner__Printable".to_string()
        ]
    );
}

#[test]
fn rewrites_module_enum_field_namespace_alias_class_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Module(ast::ModuleDecl {
                name: "M".to_string(),
                declarations: vec![sp(Decl::Enum(ast::EnumDecl {
                    name: "E".to_string(),
                    generic_params: vec![],
                    variants: vec![ast::EnumVariant {
                        name: "A".to_string(),
                        fields: vec![ast::EnumField {
                            name: None,
                            ty: ast::Type::Named("u.Box".to_string()),
                        }],
                    }],
                    visibility: ast::Visibility::Private,
                }))],
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
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

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let en = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Enum(en) => Some(en),
            _ => None,
        })
        .expect("expected enum declaration");
    assert_eq!(
        en.variants[0].fields[0].ty,
        ast::Type::Named("util__Box".to_string())
    );
}

#[test]
fn rewrites_module_enum_field_namespace_alias_nested_class_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Module(ast::ModuleDecl {
                name: "M".to_string(),
                declarations: vec![sp(Decl::Enum(ast::EnumDecl {
                    name: "E".to_string(),
                    generic_params: vec![],
                    variants: vec![ast::EnumVariant {
                        name: "A".to_string(),
                        fields: vec![ast::EnumField {
                            name: None,
                            ty: ast::Type::Named("u.Api.Box".to_string()),
                        }],
                    }],
                    visibility: ast::Visibility::Private,
                }))],
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([("util".to_string(), HashSet::from(["Api__Box".to_string()]))]),
        &HashMap::from([("Api__Box".to_string(), "util".to_string())]),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let en = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Enum(en) => Some(en),
            _ => None,
        })
        .expect("expected enum declaration");
    assert_eq!(
        en.variants[0].fields[0].ty,
        ast::Type::Named("util__Api__Box".to_string())
    );
}

#[test]
fn rewrites_module_enum_field_namespace_alias_enum_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Module(ast::ModuleDecl {
                name: "M".to_string(),
                declarations: vec![sp(Decl::Enum(ast::EnumDecl {
                    name: "E".to_string(),
                    generic_params: vec![],
                    variants: vec![ast::EnumVariant {
                        name: "A".to_string(),
                        fields: vec![ast::EnumField {
                            name: None,
                            ty: ast::Type::Named("u.Result".to_string()),
                        }],
                    }],
                    visibility: ast::Visibility::Private,
                }))],
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([("util".to_string(), HashSet::from(["Result".to_string()]))]),
        &HashMap::from([("Result".to_string(), "util".to_string())]),
        &HashMap::new(),
        &HashMap::new(),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let en = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Enum(en) => Some(en),
            _ => None,
        })
        .expect("expected enum declaration");
    assert_eq!(
        en.variants[0].fields[0].ty,
        ast::Type::Named("util__Result".to_string())
    );
}

#[test]
fn rewrites_module_enum_field_local_nested_class_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "M".to_string(),
            declarations: vec![
                sp(Decl::Module(ast::ModuleDecl {
                    name: "Api".to_string(),
                    declarations: vec![sp(Decl::Class(ast::ClassDecl {
                        name: "Box".to_string(),
                        generic_params: vec![],
                        extends: None,
                        implements: vec![],
                        fields: vec![],
                        constructor: None,
                        destructor: None,
                        methods: vec![],
                        visibility: ast::Visibility::Private,
                    }))],
                })),
                sp(Decl::Enum(ast::EnumDecl {
                    name: "E".to_string(),
                    generic_params: vec![],
                    variants: vec![ast::EnumVariant {
                        name: "A".to_string(),
                        fields: vec![ast::EnumField {
                            name: None,
                            ty: ast::Type::Named("Api.Box".to_string()),
                        }],
                    }],
                    visibility: ast::Visibility::Private,
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let en = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Enum(en) => Some(en),
            _ => None,
        })
        .expect("expected enum declaration");
    assert_eq!(
        en.variants[0].fields[0].ty,
        ast::Type::Named("app__M__Api__Box".to_string())
    );
}

#[test]
fn rewrites_module_enum_field_local_nested_enum_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "M".to_string(),
            declarations: vec![
                sp(Decl::Module(ast::ModuleDecl {
                    name: "Api".to_string(),
                    declarations: vec![sp(Decl::Enum(ast::EnumDecl {
                        name: "Result".to_string(),
                        generic_params: vec![],
                        variants: vec![],
                        visibility: ast::Visibility::Private,
                    }))],
                })),
                sp(Decl::Enum(ast::EnumDecl {
                    name: "E".to_string(),
                    generic_params: vec![],
                    variants: vec![ast::EnumVariant {
                        name: "A".to_string(),
                        fields: vec![ast::EnumField {
                            name: None,
                            ty: ast::Type::Named("Api.Result".to_string()),
                        }],
                    }],
                    visibility: ast::Visibility::Private,
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let en = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Enum(en) => Some(en),
            _ => None,
        })
        .expect("expected enum declaration");
    assert_eq!(
        en.variants[0].fields[0].ty,
        ast::Type::Named("app__M__Api__Result".to_string())
    );
}

#[test]
fn rewrites_module_enum_field_generic_namespace_alias_class_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Module(ast::ModuleDecl {
                name: "M".to_string(),
                declarations: vec![sp(Decl::Enum(ast::EnumDecl {
                    name: "E".to_string(),
                    generic_params: vec![],
                    variants: vec![ast::EnumVariant {
                        name: "A".to_string(),
                        fields: vec![ast::EnumField {
                            name: None,
                            ty: ast::Type::Generic(
                                "List".to_string(),
                                vec![ast::Type::Named("u.Box".to_string())],
                            ),
                        }],
                    }],
                    visibility: ast::Visibility::Private,
                }))],
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
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

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let en = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Enum(en) => Some(en),
            _ => None,
        })
        .expect("expected enum declaration");
    let ast::Type::Generic(_, args) = &en.variants[0].fields[0].ty else {
        panic!("expected generic type");
    };
    assert_eq!(args[0], ast::Type::Named("util__Box".to_string()));
}

#[test]
fn rewrites_module_enum_named_field_namespace_alias_class_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Module(ast::ModuleDecl {
                name: "M".to_string(),
                declarations: vec![sp(Decl::Enum(ast::EnumDecl {
                    name: "E".to_string(),
                    generic_params: vec![],
                    variants: vec![ast::EnumVariant {
                        name: "A".to_string(),
                        fields: vec![ast::EnumField {
                            name: Some("value".to_string()),
                            ty: ast::Type::Named("u.Box".to_string()),
                        }],
                    }],
                    visibility: ast::Visibility::Private,
                }))],
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
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

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let en = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Enum(en) => Some(en),
            _ => None,
        })
        .expect("expected enum declaration");
    assert_eq!(
        en.variants[0].fields[0].ty,
        ast::Type::Named("util__Box".to_string())
    );
}

#[test]
fn rewrites_module_enum_multiple_variant_field_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Module(ast::ModuleDecl {
                name: "M".to_string(),
                declarations: vec![sp(Decl::Enum(ast::EnumDecl {
                    name: "E".to_string(),
                    generic_params: vec![],
                    variants: vec![
                        ast::EnumVariant {
                            name: "A".to_string(),
                            fields: vec![ast::EnumField {
                                name: None,
                                ty: ast::Type::Named("u.Box".to_string()),
                            }],
                        },
                        ast::EnumVariant {
                            name: "B".to_string(),
                            fields: vec![ast::EnumField {
                                name: None,
                                ty: ast::Type::Named("u.Result".to_string()),
                            }],
                        },
                    ],
                    visibility: ast::Visibility::Private,
                }))],
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([("util".to_string(), HashSet::from(["Box".to_string()]))]),
        &HashMap::from([("Box".to_string(), "util".to_string())]),
        &HashMap::from([("util".to_string(), HashSet::from(["Result".to_string()]))]),
        &HashMap::from([("Result".to_string(), "util".to_string())]),
        &HashMap::new(),
        &HashMap::new(),
        &[ImportDecl {
            path: "util".to_string(),
            alias: Some("u".to_string()),
        }],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let en = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Enum(en) => Some(en),
            _ => None,
        })
        .expect("expected enum declaration");
    assert_eq!(
        en.variants[0].fields[0].ty,
        ast::Type::Named("util__Box".to_string())
    );
    assert_eq!(
        en.variants[1].fields[0].ty,
        ast::Type::Named("util__Result".to_string())
    );
}

#[test]
fn rewrites_nested_module_enum_field_namespace_alias_class_types() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![
            sp(Decl::Import(ast::ImportDecl {
                path: "util".to_string(),
                alias: Some("u".to_string()),
            })),
            sp(Decl::Module(ast::ModuleDecl {
                name: "Outer".to_string(),
                declarations: vec![sp(Decl::Module(ast::ModuleDecl {
                    name: "Inner".to_string(),
                    declarations: vec![sp(Decl::Enum(ast::EnumDecl {
                        name: "E".to_string(),
                        generic_params: vec![],
                        variants: vec![ast::EnumVariant {
                            name: "A".to_string(),
                            fields: vec![ast::EnumField {
                                name: None,
                                ty: ast::Type::Named("u.Box".to_string()),
                            }],
                        }],
                        visibility: ast::Visibility::Private,
                    }))],
                }))],
            })),
        ],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
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

    let outer = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected outer module declaration");
    let inner = outer
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected inner module declaration");
    let en = inner
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Enum(en) => Some(en),
            _ => None,
        })
        .expect("expected enum declaration");
    assert_eq!(
        en.variants[0].fields[0].ty,
        ast::Type::Named("util__Box".to_string())
    );
}

#[test]
fn renames_module_local_exact_import_alias_shadow_in_return() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "Inner".to_string(),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "Option.None".to_string(),
                    alias: Some("Empty".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "keep".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::Generic("Option".to_string(), vec![ast::Type::Integer]),
                    body: vec![
                        sp(Stmt::Let {
                            name: "Empty".to_string(),
                            ty: ast::Type::Generic("Option".to_string(), vec![ast::Type::Integer]),
                            value: sp(Expr::Call {
                                callee: Box::new(sp(Expr::Field {
                                    object: Box::new(sp(Expr::Ident("Option".to_string()))),
                                    field: "Some".to_string(),
                                })),
                                args: vec![sp(Expr::Literal(ast::Literal::Integer(7)))],
                                type_args: vec![],
                            }),
                            mutable: false,
                        }),
                        sp(Stmt::Return(Some(sp(Expr::Ident("Empty".to_string()))))),
                    ],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let function = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Function(func) if func.name == "keep" => Some(func),
            _ => None,
        })
        .expect("expected nested function declaration");
    let Stmt::Let { name, .. } = &function.body[0].node else {
        panic!("expected let statement");
    };
    assert_eq!(name, "__module_local_Empty");
    let Stmt::Return(Some(expr)) = &function.body[1].node else {
        panic!("expected return statement");
    };
    let Expr::Ident(name) = &expr.node else {
        panic!("expected returned ident");
    };
    assert_eq!(name, "__module_local_Empty");
}

#[test]
fn renames_module_local_exact_import_alias_shadowed_param_in_return() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "Inner".to_string(),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "Option.None".to_string(),
                    alias: Some("Empty".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "keep".to_string(),
                    generic_params: vec![],
                    params: vec![ast::Parameter {
                        name: "Empty".to_string(),
                        ty: ast::Type::Generic("Option".to_string(), vec![ast::Type::Integer]),
                        mutable: false,
                        mode: ast::ParamMode::Owned,
                    }],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::Generic("Option".to_string(), vec![ast::Type::Integer]),
                    body: vec![sp(Stmt::Return(Some(sp(Expr::Ident("Empty".to_string())))))],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let function = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Function(func) if func.name == "keep" => Some(func),
            _ => None,
        })
        .expect("expected nested function declaration");
    assert_eq!(function.params[0].name, "__module_local_Empty");
    let Stmt::Return(Some(expr)) = &function.body[0].node else {
        panic!("expected return statement");
    };
    let Expr::Ident(name) = &expr.node else {
        panic!("expected returned ident");
    };
    assert_eq!(name, "__module_local_Empty");
}

#[test]
fn renames_module_local_exact_import_alias_shadowed_for_var_in_lambda() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "Inner".to_string(),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "Option.None".to_string(),
                    alias: Some("Empty".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "keep".to_string(),
                    generic_params: vec![],
                    params: vec![],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::Integer,
                    body: vec![sp(Stmt::For {
                        var: "Empty".to_string(),
                        var_type: Some(ast::Type::Integer),
                        iterable: sp(Expr::Range {
                            start: Some(Box::new(sp(Expr::Literal(ast::Literal::Integer(7))))),
                            end: Some(Box::new(sp(Expr::Literal(ast::Literal::Integer(8))))),
                            inclusive: false,
                        }),
                        body: vec![
                            sp(Stmt::Let {
                                name: "f".to_string(),
                                ty: ast::Type::Function(vec![], Box::new(ast::Type::Integer)),
                                value: sp(Expr::Lambda {
                                    params: vec![],
                                    body: Box::new(sp(Expr::Ident("Empty".to_string()))),
                                }),
                                mutable: false,
                            }),
                            sp(Stmt::Return(Some(sp(Expr::Binary {
                                op: ast::BinOp::Sub,
                                left: Box::new(sp(Expr::Call {
                                    callee: Box::new(sp(Expr::Ident("f".to_string()))),
                                    args: vec![],
                                    type_args: vec![],
                                })),
                                right: Box::new(sp(Expr::Literal(ast::Literal::Integer(7)))),
                            })))),
                        ],
                    })],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let function = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Function(func) if func.name == "keep" => Some(func),
            _ => None,
        })
        .expect("expected nested function declaration");
    let Stmt::For { var, body, .. } = &function.body[0].node else {
        panic!("expected for statement");
    };
    assert_eq!(var, "__module_local_Empty");
    let Stmt::Let { value, .. } = &body[0].node else {
        panic!("expected let statement");
    };
    let Expr::Lambda { body, .. } = &value.node else {
        panic!("expected lambda expression");
    };
    let Expr::Ident(name) = &body.node else {
        panic!("expected lambda body ident");
    };
    assert_eq!(name, "__module_local_Empty");
}

#[test]
fn renames_module_local_exact_import_alias_shadowed_match_binding_in_lambda() {
    let program = Program {
        package: Some("app".to_string()),
        declarations: vec![sp(Decl::Module(ast::ModuleDecl {
            name: "Inner".to_string(),
            declarations: vec![
                sp(Decl::Import(ast::ImportDecl {
                    path: "Option.None".to_string(),
                    alias: Some("Empty".to_string()),
                })),
                sp(Decl::Function(ast::FunctionDecl {
                    name: "keep".to_string(),
                    generic_params: vec![],
                    params: vec![ast::Parameter {
                        name: "value".to_string(),
                        ty: ast::Type::Generic("Option".to_string(), vec![ast::Type::Integer]),
                        mutable: false,
                        mode: ast::ParamMode::Owned,
                    }],
                    is_variadic: false,
                    extern_abi: None,
                    extern_link_name: None,
                    return_type: ast::Type::Integer,
                    body: vec![sp(Stmt::Return(Some(sp(Expr::Match {
                        expr: Box::new(sp(Expr::Ident("value".to_string()))),
                        arms: vec![
                            ast::MatchArm {
                                pattern: ast::Pattern::Variant(
                                    "Some".to_string(),
                                    vec!["Empty".to_string()],
                                ),
                                body: vec![sp(Stmt::Expr(sp(Expr::Lambda {
                                    params: vec![],
                                    body: Box::new(sp(Expr::Ident("Empty".to_string()))),
                                })))],
                            },
                            ast::MatchArm {
                                pattern: ast::Pattern::Variant("None".to_string(), vec![]),
                                body: vec![sp(Stmt::Expr(sp(Expr::Literal(
                                    ast::Literal::Integer(1),
                                ))))],
                            },
                        ],
                    }))))],
                    is_async: false,
                    is_extern: false,
                    visibility: ast::Visibility::Private,
                    attributes: vec![],
                })),
            ],
        }))],
    };

    let rewritten = rewrite_program_for_project(
        &program,
        "app",
        "app",
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &[],
    );

    let module = rewritten
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Module(module) => Some(module),
            _ => None,
        })
        .expect("expected module declaration");
    let function = module
        .declarations
        .iter()
        .find_map(|decl| match &decl.node {
            Decl::Function(func) if func.name == "keep" => Some(func),
            _ => None,
        })
        .expect("expected nested function declaration");
    let Stmt::Return(Some(expr)) = &function.body[0].node else {
        panic!("expected return statement");
    };
    let Expr::Match { arms, .. } = &expr.node else {
        panic!("expected match expression");
    };
    let ast::Pattern::Variant(_, bindings) = &arms[0].pattern else {
        panic!("expected variant pattern");
    };
    assert_eq!(bindings[0], "__module_local_Empty");
    let Stmt::Expr(expr) = &arms[0].body[0].node else {
        panic!("expected expr statement");
    };
    let Expr::Lambda { body, .. } = &expr.node else {
        panic!("expected lambda expression");
    };
    let Expr::Ident(name) = &body.node else {
        panic!("expected lambda body ident");
    };
    assert_eq!(name, "__module_local_Empty");
}
