use crate::ast::{self, Block, Decl, Expr, Program, Spanned, Stmt, Type};
use crate::cache::*;
use crate::formatter;
use crate::parser::parse_type_source;
use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;
pub(crate) fn empty_block() -> Block {
    Vec::new()
}

pub(crate) fn api_projection_decl(decl: &Spanned<Decl>) -> Spanned<Decl> {
    let projected = match &decl.node {
        Decl::Function(func) => {
            let mut func = func.clone();
            if !func.is_extern {
                func.body = empty_block();
            }
            Decl::Function(func)
        }
        Decl::Class(class) => {
            let mut class = class.clone();
            if let Some(constructor) = &mut class.constructor {
                constructor.body = empty_block();
            }
            if let Some(destructor) = &mut class.destructor {
                destructor.body = empty_block();
            }
            class.methods = class
                .methods
                .into_iter()
                .map(|mut method| {
                    method.body = empty_block();
                    method
                })
                .collect();
            Decl::Class(class)
        }
        Decl::Interface(interface) => {
            let mut interface = interface.clone();
            interface.methods = interface
                .methods
                .into_iter()
                .map(|mut method| {
                    method.default_impl = method.default_impl.map(|_| empty_block());
                    method
                })
                .collect();
            Decl::Interface(interface)
        }
        Decl::Module(module) => {
            let mut module = module.clone();
            module.declarations = module
                .declarations
                .iter()
                .map(api_projection_decl)
                .collect();
            Decl::Module(module)
        }
        Decl::Enum(en) => Decl::Enum(en.clone()),
        Decl::Import(import) => Decl::Import(import.clone()),
    };
    Spanned::new(projected, decl.span.clone())
}

pub(crate) fn api_projection_program(program: &Program) -> Program {
    Program {
        package: program.package.clone(),
        declarations: program
            .declarations
            .iter()
            .map(api_projection_decl)
            .collect(),
    }
}

pub(crate) fn api_program_fingerprint(program: &Program) -> String {
    let projected = api_projection_program(program);
    let canonical = formatter::format_program_canonical(&projected);
    source_fingerprint(&canonical)
}

pub(crate) fn type_has_codegen_specialization_demand(ty: &Type) -> bool {
    match ty {
        Type::Generic(_, _) => true,
        Type::Function(params, ret) => {
            params.iter().any(type_has_codegen_specialization_demand)
                || type_has_codegen_specialization_demand(ret)
        }
        Type::Option(inner)
        | Type::Result(inner, _)
        | Type::List(inner)
        | Type::Set(inner)
        | Type::Ref(inner)
        | Type::MutRef(inner)
        | Type::Box(inner)
        | Type::Rc(inner)
        | Type::Arc(inner)
        | Type::Ptr(inner)
        | Type::Task(inner)
        | Type::Range(inner) => type_has_codegen_specialization_demand(inner),
        Type::Map(key, value) => {
            type_has_codegen_specialization_demand(key)
                || type_has_codegen_specialization_demand(value)
        }
        Type::Integer
        | Type::Float
        | Type::Boolean
        | Type::String
        | Type::Char
        | Type::None
        | Type::Named(_) => false,
    }
}

pub(crate) fn expr_has_codegen_specialization_demand(expr: &Expr) -> bool {
    match expr {
        Expr::Call {
            callee,
            args,
            type_args,
        } => {
            !type_args.is_empty()
                || expr_has_codegen_specialization_demand(&callee.node)
                || args
                    .iter()
                    .any(|arg| expr_has_codegen_specialization_demand(&arg.node))
                || type_args.iter().any(type_has_codegen_specialization_demand)
        }
        Expr::GenericFunctionValue { callee, type_args } => {
            !type_args.is_empty()
                || expr_has_codegen_specialization_demand(&callee.node)
                || type_args.iter().any(type_has_codegen_specialization_demand)
        }
        Expr::Construct { ty, args } => {
            parse_type_source(ty)
                .ok()
                .is_some_and(|ty| type_has_codegen_specialization_demand(&ty))
                || args
                    .iter()
                    .any(|arg| expr_has_codegen_specialization_demand(&arg.node))
        }
        Expr::Binary { left, right, .. } => {
            expr_has_codegen_specialization_demand(&left.node)
                || expr_has_codegen_specialization_demand(&right.node)
        }
        Expr::Unary { expr, .. }
        | Expr::Try(expr)
        | Expr::Borrow(expr)
        | Expr::MutBorrow(expr)
        | Expr::Deref(expr)
        | Expr::Await(expr) => expr_has_codegen_specialization_demand(&expr.node),
        Expr::Field { object, .. } => expr_has_codegen_specialization_demand(&object.node),
        Expr::Index { object, index } => {
            expr_has_codegen_specialization_demand(&object.node)
                || expr_has_codegen_specialization_demand(&index.node)
        }
        Expr::Lambda { params, body } => {
            params
                .iter()
                .any(|param| type_has_codegen_specialization_demand(&param.ty))
                || expr_has_codegen_specialization_demand(&body.node)
        }
        Expr::Match { expr, arms } => {
            expr_has_codegen_specialization_demand(&expr.node)
                || arms.iter().any(|arm| {
                    arm.body
                        .iter()
                        .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
        }
        Expr::StringInterp(parts) => parts.iter().any(|part| match part {
            ast::StringPart::Literal(_) => false,
            ast::StringPart::Expr(expr) => expr_has_codegen_specialization_demand(&expr.node),
        }),
        Expr::AsyncBlock(block) | Expr::Block(block) => block
            .iter()
            .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node)),
        Expr::Require { condition, message } => {
            expr_has_codegen_specialization_demand(&condition.node)
                || message
                    .as_ref()
                    .is_some_and(|msg| expr_has_codegen_specialization_demand(&msg.node))
        }
        Expr::Range { start, end, .. } => {
            start
                .as_ref()
                .is_some_and(|expr| expr_has_codegen_specialization_demand(&expr.node))
                || end
                    .as_ref()
                    .is_some_and(|expr| expr_has_codegen_specialization_demand(&expr.node))
        }
        Expr::IfExpr {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_has_codegen_specialization_demand(&condition.node)
                || then_branch
                    .iter()
                    .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                || else_branch.as_ref().is_some_and(|block| {
                    block
                        .iter()
                        .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
        }
        Expr::Literal(_) | Expr::Ident(_) | Expr::This => false,
    }
}

pub(crate) fn stmt_has_codegen_specialization_demand(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Let { ty, value, .. } => {
            type_has_codegen_specialization_demand(ty)
                || expr_has_codegen_specialization_demand(&value.node)
        }
        Stmt::Assign { target, value } => {
            expr_has_codegen_specialization_demand(&target.node)
                || expr_has_codegen_specialization_demand(&value.node)
        }
        Stmt::Expr(expr) => expr_has_codegen_specialization_demand(&expr.node),
        Stmt::Return(expr) => expr
            .as_ref()
            .is_some_and(|expr| expr_has_codegen_specialization_demand(&expr.node)),
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            expr_has_codegen_specialization_demand(&condition.node)
                || then_block
                    .iter()
                    .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                || else_block.as_ref().is_some_and(|block| {
                    block
                        .iter()
                        .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
        }
        Stmt::While { condition, body } => {
            expr_has_codegen_specialization_demand(&condition.node)
                || body
                    .iter()
                    .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
        }
        Stmt::For {
            var_type,
            iterable,
            body,
            ..
        } => {
            var_type
                .as_ref()
                .is_some_and(type_has_codegen_specialization_demand)
                || expr_has_codegen_specialization_demand(&iterable.node)
                || body
                    .iter()
                    .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
        }
        Stmt::Match { expr, arms } => {
            expr_has_codegen_specialization_demand(&expr.node)
                || arms.iter().any(|arm| {
                    arm.body
                        .iter()
                        .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
        }
        Stmt::Break | Stmt::Continue => false,
    }
}

pub(crate) fn specialization_projection_stmt(stmt: &Stmt) -> Option<Stmt> {
    match stmt {
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            let projected_then = then_block
                .iter()
                .filter_map(|stmt| {
                    specialization_projection_stmt(&stmt.node)
                        .map(|node| Spanned::new(node, stmt.span.clone()))
                })
                .collect::<Vec<_>>();
            let projected_else = else_block.as_ref().map(|block| {
                block
                    .iter()
                    .filter_map(|stmt| {
                        specialization_projection_stmt(&stmt.node)
                            .map(|node| Spanned::new(node, stmt.span.clone()))
                    })
                    .collect::<Vec<_>>()
            });
            if expr_has_codegen_specialization_demand(&condition.node)
                || !projected_then.is_empty()
                || projected_else
                    .as_ref()
                    .is_some_and(|block| !block.is_empty())
            {
                Some(Stmt::If {
                    condition: condition.clone(),
                    then_block: projected_then,
                    else_block: projected_else.filter(|block| !block.is_empty()),
                })
            } else {
                None
            }
        }
        Stmt::While { condition, body } => {
            let projected_body = body
                .iter()
                .filter_map(|stmt| {
                    specialization_projection_stmt(&stmt.node)
                        .map(|node| Spanned::new(node, stmt.span.clone()))
                })
                .collect::<Vec<_>>();
            if expr_has_codegen_specialization_demand(&condition.node) || !projected_body.is_empty()
            {
                Some(Stmt::While {
                    condition: condition.clone(),
                    body: projected_body,
                })
            } else {
                None
            }
        }
        Stmt::For {
            var,
            var_type,
            iterable,
            body,
        } => {
            let projected_body = body
                .iter()
                .filter_map(|stmt| {
                    specialization_projection_stmt(&stmt.node)
                        .map(|node| Spanned::new(node, stmt.span.clone()))
                })
                .collect::<Vec<_>>();
            if var_type
                .as_ref()
                .is_some_and(type_has_codegen_specialization_demand)
                || expr_has_codegen_specialization_demand(&iterable.node)
                || !projected_body.is_empty()
            {
                Some(Stmt::For {
                    var: var.clone(),
                    var_type: var_type.clone(),
                    iterable: iterable.clone(),
                    body: projected_body,
                })
            } else {
                None
            }
        }
        Stmt::Match { expr, arms } => {
            let projected_arms = arms
                .iter()
                .filter_map(|arm| {
                    let projected_body = arm
                        .body
                        .iter()
                        .filter_map(|stmt| {
                            specialization_projection_stmt(&stmt.node)
                                .map(|node| Spanned::new(node, stmt.span.clone()))
                        })
                        .collect::<Vec<_>>();
                    if !projected_body.is_empty() {
                        Some(ast::MatchArm {
                            pattern: arm.pattern.clone(),
                            body: projected_body,
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if expr_has_codegen_specialization_demand(&expr.node) || !projected_arms.is_empty() {
                Some(Stmt::Match {
                    expr: expr.clone(),
                    arms: projected_arms,
                })
            } else {
                None
            }
        }
        _ if stmt_has_codegen_specialization_demand(stmt) => Some(stmt.clone()),
        _ => None,
    }
}

pub(crate) fn specialization_projection_decl(decl: &Spanned<Decl>) -> Spanned<Decl> {
    let projected = match &decl.node {
        Decl::Function(func) => {
            let mut func = func.clone();
            if !func.is_extern {
                func.body = func
                    .body
                    .iter()
                    .filter_map(|stmt| {
                        specialization_projection_stmt(&stmt.node)
                            .map(|node| Spanned::new(node, stmt.span.clone()))
                    })
                    .collect();
            }
            Decl::Function(func)
        }
        Decl::Class(class) => {
            let mut class = class.clone();
            if let Some(constructor) = &mut class.constructor {
                constructor.body = constructor
                    .body
                    .iter()
                    .filter_map(|stmt| {
                        specialization_projection_stmt(&stmt.node)
                            .map(|node| Spanned::new(node, stmt.span.clone()))
                    })
                    .collect();
            }
            if let Some(destructor) = &mut class.destructor {
                destructor.body = destructor
                    .body
                    .iter()
                    .filter_map(|stmt| {
                        specialization_projection_stmt(&stmt.node)
                            .map(|node| Spanned::new(node, stmt.span.clone()))
                    })
                    .collect();
            }
            class.methods = class
                .methods
                .into_iter()
                .map(|mut method| {
                    method.body = method
                        .body
                        .iter()
                        .filter_map(|stmt| {
                            specialization_projection_stmt(&stmt.node)
                                .map(|node| Spanned::new(node, stmt.span.clone()))
                        })
                        .collect();
                    method
                })
                .collect();
            Decl::Class(class)
        }
        Decl::Interface(interface) => {
            let mut interface = interface.clone();
            interface.methods = interface
                .methods
                .into_iter()
                .map(|mut method| {
                    method.default_impl = method.default_impl.map(|body| {
                        body.iter()
                            .filter_map(|stmt| {
                                specialization_projection_stmt(&stmt.node)
                                    .map(|node| Spanned::new(node, stmt.span.clone()))
                            })
                            .collect()
                    });
                    method
                })
                .collect();
            Decl::Interface(interface)
        }
        Decl::Module(module) => {
            let mut module = module.clone();
            module.declarations = module
                .declarations
                .iter()
                .map(specialization_projection_decl)
                .collect();
            Decl::Module(module)
        }
        Decl::Enum(en) => Decl::Enum(en.clone()),
        Decl::Import(import) => Decl::Import(import.clone()),
    };
    Spanned::new(projected, decl.span.clone())
}

pub(crate) fn specialization_projection_program(program: &Program) -> Program {
    Program {
        package: program.package.clone(),
        declarations: program
            .declarations
            .iter()
            .map(specialization_projection_decl)
            .collect(),
    }
}

pub(crate) fn decl_has_codegen_specialization_demand(decl: &Decl) -> bool {
    match decl {
        Decl::Function(func) => {
            func.params
                .iter()
                .any(|param| type_has_codegen_specialization_demand(&param.ty))
                || type_has_codegen_specialization_demand(&func.return_type)
                || func
                    .body
                    .iter()
                    .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
        }
        Decl::Class(class) => {
            class.extends.as_ref().is_some_and(|parent| {
                parse_type_source(parent)
                    .ok()
                    .is_some_and(|ty| type_has_codegen_specialization_demand(&ty))
            }) || class.implements.iter().any(|implemented| {
                parse_type_source(implemented)
                    .ok()
                    .is_some_and(|ty| type_has_codegen_specialization_demand(&ty))
            }) || class
                .fields
                .iter()
                .any(|field| type_has_codegen_specialization_demand(&field.ty))
                || class.constructor.as_ref().is_some_and(|ctor| {
                    ctor.params
                        .iter()
                        .any(|param| type_has_codegen_specialization_demand(&param.ty))
                        || ctor
                            .body
                            .iter()
                            .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
                || class.destructor.as_ref().is_some_and(|dtor| {
                    dtor.body
                        .iter()
                        .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
                || class.methods.iter().any(|method| {
                    method
                        .params
                        .iter()
                        .any(|param| type_has_codegen_specialization_demand(&param.ty))
                        || type_has_codegen_specialization_demand(&method.return_type)
                        || method
                            .body
                            .iter()
                            .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
        }
        Decl::Enum(en) => en.variants.iter().any(|variant| {
            variant
                .fields
                .iter()
                .any(|field| type_has_codegen_specialization_demand(&field.ty))
        }),
        Decl::Interface(interface) => interface.methods.iter().any(|method| {
            method
                .params
                .iter()
                .any(|param| type_has_codegen_specialization_demand(&param.ty))
                || type_has_codegen_specialization_demand(&method.return_type)
                || method.default_impl.as_ref().is_some_and(|body| {
                    body.iter()
                        .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
        }),
        Decl::Module(module) => module
            .declarations
            .iter()
            .any(|decl| decl_has_codegen_specialization_demand(&decl.node)),
        Decl::Import(_) => false,
    }
}

pub(crate) fn program_has_codegen_specialization_demand(program: &Program) -> bool {
    program
        .declarations
        .iter()
        .any(|decl| decl_has_codegen_specialization_demand(&decl.node))
}

#[cfg(test)]
pub(crate) fn codegen_program_for_unit(
    rewritten_files: &[RewrittenProjectUnit],
    rewritten_file_indices: &HashMap<PathBuf, usize>,
    active_file: &Path,
    _dependency_closure: Option<&HashSet<PathBuf>>,
    _declaration_symbols: Option<&HashSet<String>>,
) -> Program {
    codegen_program_for_units(
        rewritten_files,
        rewritten_file_indices,
        &[active_file.to_path_buf()],
        _dependency_closure,
    )
}

pub(crate) fn codegen_program_for_units(
    rewritten_files: &[RewrittenProjectUnit],
    rewritten_file_indices: &HashMap<PathBuf, usize>,
    active_files: &[PathBuf],
    dependency_closure: Option<&HashSet<PathBuf>>,
) -> Program {
    fn merge_codegen_declarations(
        output: &mut Vec<Spanned<Decl>>,
        incoming: &[Spanned<Decl>],
        seen_specializations: &mut HashSet<String>,
    ) {
        for decl in incoming {
            match &decl.node {
                Decl::Function(func) => {
                    if func.name.contains("__spec__")
                        && !seen_specializations.insert(func.name.clone())
                    {
                        continue;
                    }
                    output.push(decl.clone());
                }
                Decl::Class(class) => {
                    if class.name.contains("__spec__")
                        && !seen_specializations.insert(class.name.clone())
                    {
                        continue;
                    }
                    output.push(decl.clone());
                }
                Decl::Enum(en) => {
                    if en.name.contains("__spec__") && !seen_specializations.insert(en.name.clone())
                    {
                        continue;
                    }
                    output.push(decl.clone());
                }
                Decl::Module(module) => {
                    if let Some(existing_module) =
                        output
                            .iter_mut()
                            .find_map(|existing| match &mut existing.node {
                                Decl::Module(existing_module)
                                    if existing_module.name == module.name =>
                                {
                                    Some(existing_module)
                                }
                                _ => None,
                            })
                    {
                        merge_codegen_declarations(
                            &mut existing_module.declarations,
                            &module.declarations,
                            seen_specializations,
                        );
                    } else {
                        let mut merged_module = module.clone();
                        merged_module.declarations.clear();
                        merge_codegen_declarations(
                            &mut merged_module.declarations,
                            &module.declarations,
                            seen_specializations,
                        );
                        output.push(Spanned::new(Decl::Module(merged_module), decl.span.clone()));
                    }
                }
                Decl::Interface(_) | Decl::Import(_) => output.push(decl.clone()),
            }
        }
    }

    let mut program = Program {
        package: None,
        declarations: Vec::new(),
    };
    let mut seen_specializations = HashSet::new();
    let active_file_set = active_files.iter().cloned().collect::<HashSet<_>>();

    let specialization_demand_files = rewritten_files
        .iter()
        .filter(|unit| unit.has_specialization_demand)
        .map(|unit| unit.file.clone())
        .collect::<HashSet<_>>();
    let active_file_has_specialization_demand = active_files.iter().any(|active_file| {
        rewritten_file_indices
            .get(active_file)
            .and_then(|index| rewritten_files.get(*index))
            .is_some_and(|unit| unit.has_specialization_demand)
    });
    let mut relevant_files = dependency_closure
        .map(|closure| closure.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_else(|| {
            rewritten_files
                .iter()
                .map(|unit| unit.file.clone())
                .collect::<Vec<_>>()
        });
    relevant_files.extend(
        specialization_demand_files
            .iter()
            .filter(|file| !active_file_set.contains(*file))
            .cloned(),
    );
    for active_file in active_files {
        if !relevant_files.iter().any(|file| file == active_file) {
            relevant_files.push(active_file.clone());
        }
    }
    relevant_files.sort();
    relevant_files.dedup();

    for file in relevant_files {
        let Some(index) = rewritten_file_indices.get(&file).copied() else {
            continue;
        };
        let unit = &rewritten_files[index];
        let source_program = if active_file_set.contains(&file) {
            unit.program.clone()
        } else if active_file_has_specialization_demand {
            // Explicit generic specialization in the active unit may depend on full generic
            // template bodies from dependency files; projections are not sufficient here.
            unit.program.clone()
        } else if specialization_demand_files.contains(&file) {
            unit.specialization_projection.clone()
        } else {
            unit.api_program.clone()
        };
        merge_codegen_declarations(
            &mut program.declarations,
            &source_program.declarations,
            &mut seen_specializations,
        );
    }

    program
}

pub(crate) fn combined_program_for_files(rewritten_files: &[RewrittenProjectUnit]) -> Program {
    let mut program = Program {
        package: None,
        declarations: Vec::new(),
    };

    for unit in rewritten_files {
        program
            .declarations
            .extend(unit.program.declarations.clone());
    }

    program
}

pub(crate) fn mangle_project_symbol_for_codegen(
    namespace: &str,
    entry_namespace: &str,
    name: &str,
) -> String {
    if name == "main" && namespace == entry_namespace {
        "main".to_string()
    } else {
        format!("{}__{}", namespace.replace('.', "__"), name)
    }
}

pub(crate) fn mangle_project_nominal_symbol_for_codegen(namespace: &str, name: &str) -> String {
    format!("{}__{}", namespace.replace('.', "__"), name)
}
