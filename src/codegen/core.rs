//! Apex Code Generator - LLVM IR generation

#![allow(dead_code)]

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};

use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, FunctionValue, IntValue, PointerValue, ValueKind,
};
use inkwell::{AddressSpace, AtomicOrdering, FloatPredicate, IntPredicate};
use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::stdlib::stdlib_registry;

/// Codegen error
#[derive(Debug)]
pub struct CodegenError {
    pub message: String,
}

impl CodegenError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, CodegenError>;

/// Variable in codegen
#[derive(Debug, Clone)]
pub struct Variable<'ctx> {
    pub ptr: PointerValue<'ctx>,
    pub ty: Type,
}

/// Class info
pub struct ClassInfo<'ctx> {
    pub struct_type: StructType<'ctx>,
    pub field_indices: HashMap<String, u32>,
    pub field_types: HashMap<String, Type>,
    pub extends: Option<String>,
}

/// Enum variant metadata
#[derive(Debug, Clone)]
pub struct EnumVariantInfo {
    pub tag: u8,
    pub fields: Vec<Type>,
}

/// Enum metadata used by codegen and pattern matching
pub struct EnumInfo<'ctx> {
    pub struct_type: StructType<'ctx>,
    pub payload_slots: usize,
    pub variants: HashMap<String, EnumVariantInfo>,
}

/// Loop context for break/continue
pub struct LoopContext<'ctx> {
    pub loop_block: BasicBlock<'ctx>,
    pub after_block: BasicBlock<'ctx>,
}

struct AsyncFunctionPlan<'ctx> {
    wrapper: FunctionValue<'ctx>,
    body: FunctionValue<'ctx>,
    thunk: FunctionValue<'ctx>,
    env_type: StructType<'ctx>,
    inner_return_type: Type,
}

#[derive(Clone)]
struct GenericTemplate {
    func: FunctionDecl,
    span: Span,
}

/// Code generator
pub struct Codegen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub variables: HashMap<String, Variable<'ctx>>,
    pub functions: HashMap<String, (FunctionValue<'ctx>, Type)>,
    pub classes: HashMap<String, ClassInfo<'ctx>>,
    pub enums: HashMap<String, EnumInfo<'ctx>>,
    pub enum_variant_to_enum: HashMap<String, String>,
    pub current_function: Option<FunctionValue<'ctx>>,
    pub current_return_type: Option<Type>,
    pub loop_stack: Vec<LoopContext<'ctx>>,
    pub str_counter: u32,
    pub lambda_counter: u32,
    async_counter: u32,
    async_functions: HashMap<String, AsyncFunctionPlan<'ctx>>,
    extern_functions: HashSet<String>,
    import_aliases: HashMap<String, String>,
}

impl<'ctx> Codegen<'ctx> {
    fn type_specialization_suffix(ty: &Type) -> String {
        match ty {
            Type::Integer => "I64".to_string(),
            Type::Float => "F64".to_string(),
            Type::Boolean => "Bool".to_string(),
            Type::String => "Str".to_string(),
            Type::Char => "Char".to_string(),
            Type::None => "None".to_string(),
            Type::Named(name) => format!("N{}", name.replace("__", "_")),
            Type::Generic(name, args) => format!(
                "G{}{}",
                name.replace("__", "_"),
                args.iter()
                    .map(Self::type_specialization_suffix)
                    .collect::<Vec<_>>()
                    .join("_")
            ),
            Type::Function(params, ret) => format!(
                "Fn{}To{}",
                params
                    .iter()
                    .map(Self::type_specialization_suffix)
                    .collect::<Vec<_>>()
                    .join("_"),
                Self::type_specialization_suffix(ret)
            ),
            Type::Option(inner) => format!("Opt{}", Self::type_specialization_suffix(inner)),
            Type::Result(ok, err) => format!(
                "Res{}_{}",
                Self::type_specialization_suffix(ok),
                Self::type_specialization_suffix(err)
            ),
            Type::List(inner) => format!("List{}", Self::type_specialization_suffix(inner)),
            Type::Map(k, v) => format!(
                "Map{}_{}",
                Self::type_specialization_suffix(k),
                Self::type_specialization_suffix(v)
            ),
            Type::Set(inner) => format!("Set{}", Self::type_specialization_suffix(inner)),
            Type::Ref(inner) => format!("Ref{}", Self::type_specialization_suffix(inner)),
            Type::MutRef(inner) => format!("MutRef{}", Self::type_specialization_suffix(inner)),
            Type::Box(inner) => format!("Box{}", Self::type_specialization_suffix(inner)),
            Type::Rc(inner) => format!("Rc{}", Self::type_specialization_suffix(inner)),
            Type::Arc(inner) => format!("Arc{}", Self::type_specialization_suffix(inner)),
            Type::Ptr(inner) => format!("Ptr{}", Self::type_specialization_suffix(inner)),
            Type::Task(inner) => format!("Task{}", Self::type_specialization_suffix(inner)),
            Type::Range(inner) => format!("Range{}", Self::type_specialization_suffix(inner)),
        }
    }

    fn substitute_type(ty: &Type, bindings: &HashMap<String, Type>) -> Type {
        match ty {
            Type::Named(name) => bindings.get(name).cloned().unwrap_or_else(|| ty.clone()),
            Type::Generic(name, args) => Type::Generic(
                name.clone(),
                args.iter()
                    .map(|arg| Self::substitute_type(arg, bindings))
                    .collect(),
            ),
            Type::Function(params, ret) => Type::Function(
                params
                    .iter()
                    .map(|p| Self::substitute_type(p, bindings))
                    .collect(),
                Box::new(Self::substitute_type(ret, bindings)),
            ),
            Type::Option(inner) => Type::Option(Box::new(Self::substitute_type(inner, bindings))),
            Type::Result(ok, err) => Type::Result(
                Box::new(Self::substitute_type(ok, bindings)),
                Box::new(Self::substitute_type(err, bindings)),
            ),
            Type::List(inner) => Type::List(Box::new(Self::substitute_type(inner, bindings))),
            Type::Map(k, v) => Type::Map(
                Box::new(Self::substitute_type(k, bindings)),
                Box::new(Self::substitute_type(v, bindings)),
            ),
            Type::Set(inner) => Type::Set(Box::new(Self::substitute_type(inner, bindings))),
            Type::Ref(inner) => Type::Ref(Box::new(Self::substitute_type(inner, bindings))),
            Type::MutRef(inner) => Type::MutRef(Box::new(Self::substitute_type(inner, bindings))),
            Type::Box(inner) => Type::Box(Box::new(Self::substitute_type(inner, bindings))),
            Type::Rc(inner) => Type::Rc(Box::new(Self::substitute_type(inner, bindings))),
            Type::Arc(inner) => Type::Arc(Box::new(Self::substitute_type(inner, bindings))),
            Type::Ptr(inner) => Type::Ptr(Box::new(Self::substitute_type(inner, bindings))),
            Type::Task(inner) => Type::Task(Box::new(Self::substitute_type(inner, bindings))),
            Type::Range(inner) => Type::Range(Box::new(Self::substitute_type(inner, bindings))),
            _ => ty.clone(),
        }
    }

    fn collect_generic_templates_from_decl(
        decl: &Spanned<Decl>,
        module_prefix: Option<&str>,
        templates: &mut HashMap<String, GenericTemplate>,
    ) {
        match &decl.node {
            Decl::Function(func) => {
                if func.generic_params.is_empty() {
                    return;
                }
                let key = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, func.name)
                } else {
                    func.name.clone()
                };
                templates.insert(
                    key,
                    GenericTemplate {
                        func: func.clone(),
                        span: decl.span.clone(),
                    },
                );
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                for inner in &module.declarations {
                    Self::collect_generic_templates_from_decl(inner, Some(&next_prefix), templates);
                }
            }
            _ => {}
        }
    }

    fn template_key_for_callee(callee: &Expr) -> Option<String> {
        match callee {
            Expr::Ident(name) => Some(name.clone()),
            _ => Self::flatten_field_chain(callee).and_then(|parts| {
                if parts.len() >= 2 {
                    Some(parts.join("__"))
                } else {
                    None
                }
            }),
        }
    }

    fn rewrite_stmt_generic_calls(
        stmt: &Stmt,
        templates: &HashMap<String, GenericTemplate>,
        emitted: &mut HashSet<String>,
        generated: &mut Vec<Spanned<Decl>>,
    ) -> Result<Stmt> {
        Ok(match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
            } => Stmt::Let {
                name: name.clone(),
                ty: ty.clone(),
                value: Spanned::new(
                    Self::rewrite_expr_generic_calls(&value.node, templates, emitted, generated)?,
                    value.span.clone(),
                ),
                mutable: *mutable,
            },
            Stmt::Assign { target, value } => Stmt::Assign {
                target: Spanned::new(
                    Self::rewrite_expr_generic_calls(&target.node, templates, emitted, generated)?,
                    target.span.clone(),
                ),
                value: Spanned::new(
                    Self::rewrite_expr_generic_calls(&value.node, templates, emitted, generated)?,
                    value.span.clone(),
                ),
            },
            Stmt::Expr(expr) => Stmt::Expr(Spanned::new(
                Self::rewrite_expr_generic_calls(&expr.node, templates, emitted, generated)?,
                expr.span.clone(),
            )),
            Stmt::Return(expr) => Stmt::Return(
                expr.as_ref()
                    .map(|e| {
                        Ok(Spanned::new(
                            Self::rewrite_expr_generic_calls(
                                &e.node, templates, emitted, generated,
                            )?,
                            e.span.clone(),
                        ))
                    })
                    .transpose()?,
            ),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => Stmt::If {
                condition: Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &condition.node,
                        templates,
                        emitted,
                        generated,
                    )?,
                    condition.span.clone(),
                ),
                then_block: then_block
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(
                                &s.node, templates, emitted, generated,
                            )?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
                else_block: else_block
                    .as_ref()
                    .map(|blk| {
                        blk.iter()
                            .map(|s| {
                                Ok(Spanned::new(
                                    Self::rewrite_stmt_generic_calls(
                                        &s.node, templates, emitted, generated,
                                    )?,
                                    s.span.clone(),
                                ))
                            })
                            .collect::<Result<Vec<_>>>()
                    })
                    .transpose()?,
            },
            Stmt::While { condition, body } => Stmt::While {
                condition: Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &condition.node,
                        templates,
                        emitted,
                        generated,
                    )?,
                    condition.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(
                                &s.node, templates, emitted, generated,
                            )?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
            Stmt::For {
                var,
                var_type,
                iterable,
                body,
            } => Stmt::For {
                var: var.clone(),
                var_type: var_type.clone(),
                iterable: Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &iterable.node,
                        templates,
                        emitted,
                        generated,
                    )?,
                    iterable.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(
                                &s.node, templates, emitted, generated,
                            )?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
            Stmt::Match { expr, arms } => Stmt::Match {
                expr: Spanned::new(
                    Self::rewrite_expr_generic_calls(&expr.node, templates, emitted, generated)?,
                    expr.span.clone(),
                ),
                arms: arms
                    .iter()
                    .map(|arm| {
                        Ok(MatchArm {
                            pattern: arm.pattern.clone(),
                            body: arm
                                .body
                                .iter()
                                .map(|s| {
                                    Ok(Spanned::new(
                                        Self::rewrite_stmt_generic_calls(
                                            &s.node, templates, emitted, generated,
                                        )?,
                                        s.span.clone(),
                                    ))
                                })
                                .collect::<Result<Vec<_>>>()?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
            Stmt::Break => Stmt::Break,
            Stmt::Continue => Stmt::Continue,
        })
    }

    fn substitute_expr_types(expr: &Expr, bindings: &HashMap<String, Type>) -> Expr {
        match expr {
            Expr::Call {
                callee,
                args,
                type_args,
            } => Expr::Call {
                callee: Box::new(Spanned::new(
                    Self::substitute_expr_types(&callee.node, bindings),
                    callee.span.clone(),
                )),
                args: args
                    .iter()
                    .map(|a| {
                        Spanned::new(
                            Self::substitute_expr_types(&a.node, bindings),
                            a.span.clone(),
                        )
                    })
                    .collect(),
                type_args: type_args
                    .iter()
                    .map(|t| Self::substitute_type(t, bindings))
                    .collect(),
            },
            Expr::Binary { op, left, right } => Expr::Binary {
                op: *op,
                left: Box::new(Spanned::new(
                    Self::substitute_expr_types(&left.node, bindings),
                    left.span.clone(),
                )),
                right: Box::new(Spanned::new(
                    Self::substitute_expr_types(&right.node, bindings),
                    right.span.clone(),
                )),
            },
            Expr::Unary { op, expr } => Expr::Unary {
                op: *op,
                expr: Box::new(Spanned::new(
                    Self::substitute_expr_types(&expr.node, bindings),
                    expr.span.clone(),
                )),
            },
            Expr::Field { object, field } => Expr::Field {
                object: Box::new(Spanned::new(
                    Self::substitute_expr_types(&object.node, bindings),
                    object.span.clone(),
                )),
                field: field.clone(),
            },
            Expr::Index { object, index } => Expr::Index {
                object: Box::new(Spanned::new(
                    Self::substitute_expr_types(&object.node, bindings),
                    object.span.clone(),
                )),
                index: Box::new(Spanned::new(
                    Self::substitute_expr_types(&index.node, bindings),
                    index.span.clone(),
                )),
            },
            Expr::Lambda { params, body } => Expr::Lambda {
                params: params
                    .iter()
                    .map(|p| Parameter {
                        name: p.name.clone(),
                        ty: Self::substitute_type(&p.ty, bindings),
                        mutable: p.mutable,
                        mode: p.mode,
                    })
                    .collect(),
                body: Box::new(Spanned::new(
                    Self::substitute_expr_types(&body.node, bindings),
                    body.span.clone(),
                )),
            },
            Expr::StringInterp(parts) => Expr::StringInterp(
                parts
                    .iter()
                    .map(|p| match p {
                        StringPart::Literal(s) => StringPart::Literal(s.clone()),
                        StringPart::Expr(e) => StringPart::Expr(Spanned::new(
                            Self::substitute_expr_types(&e.node, bindings),
                            e.span.clone(),
                        )),
                    })
                    .collect(),
            ),
            Expr::Try(inner) => Expr::Try(Box::new(Spanned::new(
                Self::substitute_expr_types(&inner.node, bindings),
                inner.span.clone(),
            ))),
            Expr::Borrow(inner) => Expr::Borrow(Box::new(Spanned::new(
                Self::substitute_expr_types(&inner.node, bindings),
                inner.span.clone(),
            ))),
            Expr::MutBorrow(inner) => Expr::MutBorrow(Box::new(Spanned::new(
                Self::substitute_expr_types(&inner.node, bindings),
                inner.span.clone(),
            ))),
            Expr::Deref(inner) => Expr::Deref(Box::new(Spanned::new(
                Self::substitute_expr_types(&inner.node, bindings),
                inner.span.clone(),
            ))),
            Expr::Await(inner) => Expr::Await(Box::new(Spanned::new(
                Self::substitute_expr_types(&inner.node, bindings),
                inner.span.clone(),
            ))),
            Expr::Require { condition, message } => Expr::Require {
                condition: Box::new(Spanned::new(
                    Self::substitute_expr_types(&condition.node, bindings),
                    condition.span.clone(),
                )),
                message: message.as_ref().map(|m| {
                    Box::new(Spanned::new(
                        Self::substitute_expr_types(&m.node, bindings),
                        m.span.clone(),
                    ))
                }),
            },
            Expr::Range {
                start,
                end,
                inclusive,
            } => Expr::Range {
                start: start.as_ref().map(|s| {
                    Box::new(Spanned::new(
                        Self::substitute_expr_types(&s.node, bindings),
                        s.span.clone(),
                    ))
                }),
                end: end.as_ref().map(|e| {
                    Box::new(Spanned::new(
                        Self::substitute_expr_types(&e.node, bindings),
                        e.span.clone(),
                    ))
                }),
                inclusive: *inclusive,
            },
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => Expr::IfExpr {
                condition: Box::new(Spanned::new(
                    Self::substitute_expr_types(&condition.node, bindings),
                    condition.span.clone(),
                )),
                then_branch: then_branch
                    .iter()
                    .map(|s| {
                        Spanned::new(
                            Self::substitute_stmt_types(&s.node, bindings),
                            s.span.clone(),
                        )
                    })
                    .collect(),
                else_branch: else_branch.as_ref().map(|blk| {
                    blk.iter()
                        .map(|s| {
                            Spanned::new(
                                Self::substitute_stmt_types(&s.node, bindings),
                                s.span.clone(),
                            )
                        })
                        .collect()
                }),
            },
            Expr::Block(block) => Expr::Block(
                block
                    .iter()
                    .map(|s| {
                        Spanned::new(
                            Self::substitute_stmt_types(&s.node, bindings),
                            s.span.clone(),
                        )
                    })
                    .collect(),
            ),
            Expr::AsyncBlock(block) => Expr::AsyncBlock(
                block
                    .iter()
                    .map(|s| {
                        Spanned::new(
                            Self::substitute_stmt_types(&s.node, bindings),
                            s.span.clone(),
                        )
                    })
                    .collect(),
            ),
            Expr::Match { expr, arms } => Expr::Match {
                expr: Box::new(Spanned::new(
                    Self::substitute_expr_types(&expr.node, bindings),
                    expr.span.clone(),
                )),
                arms: arms
                    .iter()
                    .map(|arm| MatchArm {
                        pattern: arm.pattern.clone(),
                        body: arm
                            .body
                            .iter()
                            .map(|s| {
                                Spanned::new(
                                    Self::substitute_stmt_types(&s.node, bindings),
                                    s.span.clone(),
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            },
            _ => expr.clone(),
        }
    }

    fn substitute_stmt_types(stmt: &Stmt, bindings: &HashMap<String, Type>) -> Stmt {
        match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
            } => Stmt::Let {
                name: name.clone(),
                ty: Self::substitute_type(ty, bindings),
                value: Spanned::new(
                    Self::substitute_expr_types(&value.node, bindings),
                    value.span.clone(),
                ),
                mutable: *mutable,
            },
            Stmt::Assign { target, value } => Stmt::Assign {
                target: Spanned::new(
                    Self::substitute_expr_types(&target.node, bindings),
                    target.span.clone(),
                ),
                value: Spanned::new(
                    Self::substitute_expr_types(&value.node, bindings),
                    value.span.clone(),
                ),
            },
            Stmt::Expr(expr) => Stmt::Expr(Spanned::new(
                Self::substitute_expr_types(&expr.node, bindings),
                expr.span.clone(),
            )),
            Stmt::Return(expr) => Stmt::Return(expr.as_ref().map(|e| {
                Spanned::new(
                    Self::substitute_expr_types(&e.node, bindings),
                    e.span.clone(),
                )
            })),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => Stmt::If {
                condition: Spanned::new(
                    Self::substitute_expr_types(&condition.node, bindings),
                    condition.span.clone(),
                ),
                then_block: then_block
                    .iter()
                    .map(|s| {
                        Spanned::new(
                            Self::substitute_stmt_types(&s.node, bindings),
                            s.span.clone(),
                        )
                    })
                    .collect(),
                else_block: else_block.as_ref().map(|blk| {
                    blk.iter()
                        .map(|s| {
                            Spanned::new(
                                Self::substitute_stmt_types(&s.node, bindings),
                                s.span.clone(),
                            )
                        })
                        .collect()
                }),
            },
            Stmt::While { condition, body } => Stmt::While {
                condition: Spanned::new(
                    Self::substitute_expr_types(&condition.node, bindings),
                    condition.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|s| {
                        Spanned::new(
                            Self::substitute_stmt_types(&s.node, bindings),
                            s.span.clone(),
                        )
                    })
                    .collect(),
            },
            Stmt::For {
                var,
                var_type,
                iterable,
                body,
            } => Stmt::For {
                var: var.clone(),
                var_type: var_type
                    .as_ref()
                    .map(|t| Self::substitute_type(t, bindings)),
                iterable: Spanned::new(
                    Self::substitute_expr_types(&iterable.node, bindings),
                    iterable.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|s| {
                        Spanned::new(
                            Self::substitute_stmt_types(&s.node, bindings),
                            s.span.clone(),
                        )
                    })
                    .collect(),
            },
            Stmt::Match { expr, arms } => Stmt::Match {
                expr: Spanned::new(
                    Self::substitute_expr_types(&expr.node, bindings),
                    expr.span.clone(),
                ),
                arms: arms
                    .iter()
                    .map(|arm| MatchArm {
                        pattern: arm.pattern.clone(),
                        body: arm
                            .body
                            .iter()
                            .map(|s| {
                                Spanned::new(
                                    Self::substitute_stmt_types(&s.node, bindings),
                                    s.span.clone(),
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            },
            Stmt::Break => Stmt::Break,
            Stmt::Continue => Stmt::Continue,
        }
    }

    fn rewrite_expr_generic_calls(
        expr: &Expr,
        templates: &HashMap<String, GenericTemplate>,
        emitted: &mut HashSet<String>,
        generated: &mut Vec<Spanned<Decl>>,
    ) -> Result<Expr> {
        Ok(match expr {
            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                let rewritten_callee = Spanned::new(
                    Self::rewrite_expr_generic_calls(&callee.node, templates, emitted, generated)?,
                    callee.span.clone(),
                );
                let rewritten_args = args
                    .iter()
                    .map(|arg| {
                        Ok(Spanned::new(
                            Self::rewrite_expr_generic_calls(
                                &arg.node, templates, emitted, generated,
                            )?,
                            arg.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;

                if !type_args.is_empty() {
                    if let Some(template_key) = Self::template_key_for_callee(&callee.node) {
                        if let Some(template) = templates.get(&template_key) {
                            if template.func.generic_params.len() != type_args.len() {
                                return Ok(Expr::Call {
                                    callee: Box::new(rewritten_callee),
                                    args: rewritten_args,
                                    type_args: type_args.clone(),
                                });
                            }
                            let suffix = type_args
                                .iter()
                                .map(Self::type_specialization_suffix)
                                .collect::<Vec<_>>()
                                .join("_");
                            let spec_name = format!("{}__spec__{}", template_key, suffix);

                            if emitted.insert(spec_name.clone()) {
                                let mut bindings: HashMap<String, Type> = HashMap::new();
                                for (param, ty) in
                                    template.func.generic_params.iter().zip(type_args.iter())
                                {
                                    bindings.insert(param.name.clone(), ty.clone());
                                }

                                let mut spec_func = template.func.clone();
                                spec_func.name = spec_name.clone();
                                spec_func.generic_params.clear();
                                for param in &mut spec_func.params {
                                    param.ty = Self::substitute_type(&param.ty, &bindings);
                                }
                                spec_func.return_type =
                                    Self::substitute_type(&spec_func.return_type, &bindings);
                                spec_func.body = spec_func
                                    .body
                                    .iter()
                                    .map(|s| {
                                        Spanned::new(
                                            Self::substitute_stmt_types(&s.node, &bindings),
                                            s.span.clone(),
                                        )
                                    })
                                    .collect();

                                let rewritten_body = spec_func
                                    .body
                                    .iter()
                                    .map(|s| {
                                        Ok(Spanned::new(
                                            Self::rewrite_stmt_generic_calls(
                                                &s.node, templates, emitted, generated,
                                            )?,
                                            s.span.clone(),
                                        ))
                                    })
                                    .collect::<Result<Vec<_>>>()?;
                                spec_func.body = rewritten_body;
                                generated.push(Spanned::new(
                                    Decl::Function(spec_func),
                                    template.span.clone(),
                                ));
                            }

                            Expr::Call {
                                callee: Box::new(Spanned::new(
                                    Expr::Ident(spec_name),
                                    rewritten_callee.span,
                                )),
                                args: rewritten_args,
                                type_args: Vec::new(),
                            }
                        } else {
                            Expr::Call {
                                callee: Box::new(rewritten_callee),
                                args: rewritten_args,
                                type_args: type_args.clone(),
                            }
                        }
                    } else {
                        Expr::Call {
                            callee: Box::new(rewritten_callee),
                            args: rewritten_args,
                            type_args: type_args.clone(),
                        }
                    }
                } else {
                    Expr::Call {
                        callee: Box::new(rewritten_callee),
                        args: rewritten_args,
                        type_args: type_args.clone(),
                    }
                }
            }
            Expr::Binary { op, left, right } => Expr::Binary {
                op: *op,
                left: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&left.node, templates, emitted, generated)?,
                    left.span.clone(),
                )),
                right: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&right.node, templates, emitted, generated)?,
                    right.span.clone(),
                )),
            },
            Expr::Unary { op, expr } => Expr::Unary {
                op: *op,
                expr: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&expr.node, templates, emitted, generated)?,
                    expr.span.clone(),
                )),
            },
            Expr::Field { object, field } => Expr::Field {
                object: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&object.node, templates, emitted, generated)?,
                    object.span.clone(),
                )),
                field: field.clone(),
            },
            Expr::Index { object, index } => Expr::Index {
                object: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&object.node, templates, emitted, generated)?,
                    object.span.clone(),
                )),
                index: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&index.node, templates, emitted, generated)?,
                    index.span.clone(),
                )),
            },
            Expr::Lambda { params, body } => Expr::Lambda {
                params: params.clone(),
                body: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&body.node, templates, emitted, generated)?,
                    body.span.clone(),
                )),
            },
            Expr::Match { expr, arms } => Expr::Match {
                expr: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&expr.node, templates, emitted, generated)?,
                    expr.span.clone(),
                )),
                arms: arms
                    .iter()
                    .map(|arm| {
                        Ok(MatchArm {
                            pattern: arm.pattern.clone(),
                            body: arm
                                .body
                                .iter()
                                .map(|s| {
                                    Ok(Spanned::new(
                                        Self::rewrite_stmt_generic_calls(
                                            &s.node, templates, emitted, generated,
                                        )?,
                                        s.span.clone(),
                                    ))
                                })
                                .collect::<Result<Vec<_>>>()?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
            Expr::StringInterp(parts) => Expr::StringInterp(
                parts
                    .iter()
                    .map(|p| match p {
                        StringPart::Literal(s) => Ok(StringPart::Literal(s.clone())),
                        StringPart::Expr(e) => Ok(StringPart::Expr(Spanned::new(
                            Self::rewrite_expr_generic_calls(
                                &e.node, templates, emitted, generated,
                            )?,
                            e.span.clone(),
                        ))),
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            Expr::Try(inner) => Expr::Try(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(&inner.node, templates, emitted, generated)?,
                inner.span.clone(),
            ))),
            Expr::Borrow(inner) => Expr::Borrow(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(&inner.node, templates, emitted, generated)?,
                inner.span.clone(),
            ))),
            Expr::MutBorrow(inner) => Expr::MutBorrow(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(&inner.node, templates, emitted, generated)?,
                inner.span.clone(),
            ))),
            Expr::Deref(inner) => Expr::Deref(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(&inner.node, templates, emitted, generated)?,
                inner.span.clone(),
            ))),
            Expr::Await(inner) => Expr::Await(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(&inner.node, templates, emitted, generated)?,
                inner.span.clone(),
            ))),
            Expr::AsyncBlock(block) => Expr::AsyncBlock(
                block
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(
                                &s.node, templates, emitted, generated,
                            )?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            Expr::Require { condition, message } => Expr::Require {
                condition: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &condition.node,
                        templates,
                        emitted,
                        generated,
                    )?,
                    condition.span.clone(),
                )),
                message: message
                    .as_ref()
                    .map(|m| {
                        Ok(Box::new(Spanned::new(
                            Self::rewrite_expr_generic_calls(
                                &m.node, templates, emitted, generated,
                            )?,
                            m.span.clone(),
                        )))
                    })
                    .transpose()?,
            },
            Expr::Range {
                start,
                end,
                inclusive,
            } => Expr::Range {
                start: start
                    .as_ref()
                    .map(|s| {
                        Ok(Box::new(Spanned::new(
                            Self::rewrite_expr_generic_calls(
                                &s.node, templates, emitted, generated,
                            )?,
                            s.span.clone(),
                        )))
                    })
                    .transpose()?,
                end: end
                    .as_ref()
                    .map(|e| {
                        Ok(Box::new(Spanned::new(
                            Self::rewrite_expr_generic_calls(
                                &e.node, templates, emitted, generated,
                            )?,
                            e.span.clone(),
                        )))
                    })
                    .transpose()?,
                inclusive: *inclusive,
            },
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => Expr::IfExpr {
                condition: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &condition.node,
                        templates,
                        emitted,
                        generated,
                    )?,
                    condition.span.clone(),
                )),
                then_branch: then_branch
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(
                                &s.node, templates, emitted, generated,
                            )?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
                else_branch: else_branch
                    .as_ref()
                    .map(|blk| {
                        blk.iter()
                            .map(|s| {
                                Ok(Spanned::new(
                                    Self::rewrite_stmt_generic_calls(
                                        &s.node, templates, emitted, generated,
                                    )?,
                                    s.span.clone(),
                                ))
                            })
                            .collect::<Result<Vec<_>>>()
                    })
                    .transpose()?,
            },
            Expr::Block(block) => Expr::Block(
                block
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(
                                &s.node, templates, emitted, generated,
                            )?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            _ => expr.clone(),
        })
    }

    fn rewrite_decl_generic_calls(
        decl: &Spanned<Decl>,
        templates: &HashMap<String, GenericTemplate>,
        emitted: &mut HashSet<String>,
        generated: &mut Vec<Spanned<Decl>>,
    ) -> Result<Spanned<Decl>> {
        Ok(match &decl.node {
            Decl::Function(func) => {
                let mut f = func.clone();
                f.body = f
                    .body
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(
                                &s.node, templates, emitted, generated,
                            )?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;
                Spanned::new(Decl::Function(f), decl.span.clone())
            }
            Decl::Module(module) => {
                let mut m = module.clone();
                m.declarations = m
                    .declarations
                    .iter()
                    .map(|inner| {
                        Self::rewrite_decl_generic_calls(inner, templates, emitted, generated)
                    })
                    .collect::<Result<Vec<_>>>()?;
                Spanned::new(Decl::Module(m), decl.span.clone())
            }
            Decl::Class(class) => {
                let mut c = class.clone();
                if let Some(ctor) = &mut c.constructor {
                    ctor.body = ctor
                        .body
                        .iter()
                        .map(|s| {
                            Ok(Spanned::new(
                                Self::rewrite_stmt_generic_calls(
                                    &s.node, templates, emitted, generated,
                                )?,
                                s.span.clone(),
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?;
                }
                if let Some(dtor) = &mut c.destructor {
                    dtor.body = dtor
                        .body
                        .iter()
                        .map(|s| {
                            Ok(Spanned::new(
                                Self::rewrite_stmt_generic_calls(
                                    &s.node, templates, emitted, generated,
                                )?,
                                s.span.clone(),
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?;
                }
                c.methods = c
                    .methods
                    .iter()
                    .map(|method| {
                        let mut nm = method.clone();
                        nm.body = nm
                            .body
                            .iter()
                            .map(|s| {
                                Ok(Spanned::new(
                                    Self::rewrite_stmt_generic_calls(
                                        &s.node, templates, emitted, generated,
                                    )?,
                                    s.span.clone(),
                                ))
                            })
                            .collect::<Result<Vec<_>>>()?;
                        Ok(nm)
                    })
                    .collect::<Result<Vec<_>>>()?;
                Spanned::new(Decl::Class(c), decl.span.clone())
            }
            _ => decl.clone(),
        })
    }

    fn expr_has_explicit_generic_calls(expr: &Expr) -> bool {
        match expr {
            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                !type_args.is_empty()
                    || Self::expr_has_explicit_generic_calls(&callee.node)
                    || args
                        .iter()
                        .any(|arg| Self::expr_has_explicit_generic_calls(&arg.node))
            }
            Expr::Binary { left, right, .. } => {
                Self::expr_has_explicit_generic_calls(&left.node)
                    || Self::expr_has_explicit_generic_calls(&right.node)
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => Self::expr_has_explicit_generic_calls(&expr.node),
            Expr::Field { object, .. } => Self::expr_has_explicit_generic_calls(&object.node),
            Expr::Index { object, index } => {
                Self::expr_has_explicit_generic_calls(&object.node)
                    || Self::expr_has_explicit_generic_calls(&index.node)
            }
            Expr::Construct { args, .. } => args
                .iter()
                .any(|arg| Self::expr_has_explicit_generic_calls(&arg.node)),
            Expr::Lambda { body, .. } => Self::expr_has_explicit_generic_calls(&body.node),
            Expr::Match { expr, arms } => {
                Self::expr_has_explicit_generic_calls(&expr.node)
                    || arms.iter().any(|arm| {
                        arm.body
                            .iter()
                            .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                    })
            }
            Expr::StringInterp(parts) => parts.iter().any(|part| match part {
                StringPart::Literal(_) => false,
                StringPart::Expr(expr) => Self::expr_has_explicit_generic_calls(&expr.node),
            }),
            Expr::AsyncBlock(block) | Expr::Block(block) => block
                .iter()
                .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node)),
            Expr::Require { condition, message } => {
                Self::expr_has_explicit_generic_calls(&condition.node)
                    || message
                        .as_ref()
                        .is_some_and(|msg| Self::expr_has_explicit_generic_calls(&msg.node))
            }
            Expr::Range { start, end, .. } => {
                start
                    .as_ref()
                    .is_some_and(|expr| Self::expr_has_explicit_generic_calls(&expr.node))
                    || end
                        .as_ref()
                        .is_some_and(|expr| Self::expr_has_explicit_generic_calls(&expr.node))
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expr_has_explicit_generic_calls(&condition.node)
                    || then_branch
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                    || else_branch.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                    })
            }
            Expr::Literal(_) | Expr::Ident(_) | Expr::This => false,
        }
    }

    fn stmt_has_explicit_generic_calls(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Let { value, .. } => Self::expr_has_explicit_generic_calls(&value.node),
            Stmt::Assign { target, value } => {
                Self::expr_has_explicit_generic_calls(&target.node)
                    || Self::expr_has_explicit_generic_calls(&value.node)
            }
            Stmt::Expr(expr) => Self::expr_has_explicit_generic_calls(&expr.node),
            Stmt::Return(expr) => expr
                .as_ref()
                .is_some_and(|expr| Self::expr_has_explicit_generic_calls(&expr.node)),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                Self::expr_has_explicit_generic_calls(&condition.node)
                    || then_block
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                    })
            }
            Stmt::While { condition, body } => {
                Self::expr_has_explicit_generic_calls(&condition.node)
                    || body
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
            }
            Stmt::For { iterable, body, .. } => {
                Self::expr_has_explicit_generic_calls(&iterable.node)
                    || body
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
            }
            Stmt::Match { expr, arms } => {
                Self::expr_has_explicit_generic_calls(&expr.node)
                    || arms.iter().any(|arm| {
                        arm.body
                            .iter()
                            .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                    })
            }
            Stmt::Break | Stmt::Continue => false,
        }
    }

    fn decl_has_explicit_generic_calls(decl: &Decl) -> bool {
        match decl {
            Decl::Function(func) => func
                .body
                .iter()
                .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node)),
            Decl::Class(class) => {
                class.constructor.as_ref().is_some_and(|ctor| {
                    ctor.body
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                }) || class.destructor.as_ref().is_some_and(|dtor| {
                    dtor.body
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                }) || class.methods.iter().any(|method| {
                    method
                        .body
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                })
            }
            Decl::Module(module) => module
                .declarations
                .iter()
                .any(|decl| Self::decl_has_explicit_generic_calls(&decl.node)),
            Decl::Interface(interface) => interface.methods.iter().any(|method| {
                method.default_impl.as_ref().is_some_and(|body| {
                    body.iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                })
            }),
            Decl::Enum(_) | Decl::Import(_) => false,
        }
    }

    fn program_has_explicit_generic_calls(program: &Program) -> bool {
        program
            .declarations
            .iter()
            .any(|decl| Self::decl_has_explicit_generic_calls(&decl.node))
    }

    fn specialize_explicit_generic_calls(program: &Program) -> Result<Program> {
        let mut templates: HashMap<String, GenericTemplate> = HashMap::new();
        for decl in &program.declarations {
            Self::collect_generic_templates_from_decl(decl, None, &mut templates);
        }
        if templates.is_empty() {
            return Ok(program.clone());
        }

        let mut emitted_specs: HashSet<String> = HashSet::new();
        let mut generated: Vec<Spanned<Decl>> = Vec::new();
        let rewritten = program
            .declarations
            .iter()
            .map(|decl| {
                Self::rewrite_decl_generic_calls(
                    decl,
                    &templates,
                    &mut emitted_specs,
                    &mut generated,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let mut all_decls = rewritten;
        all_decls.extend(generated);
        Ok(Program {
            package: program.package.clone(),
            declarations: all_decls,
        })
    }

    pub fn new(context: &'ctx Context, name: &str) -> Self {
        let module = context.create_module(name);
        let builder = context.create_builder();

        Self {
            context,
            module,
            builder,
            variables: HashMap::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            enums: HashMap::new(),
            enum_variant_to_enum: HashMap::new(),
            current_function: None,
            current_return_type: None,
            loop_stack: Vec::new(),
            str_counter: 0,
            lambda_counter: 0,
            async_counter: 0,
            async_functions: HashMap::new(),
            extern_functions: HashSet::new(),
            import_aliases: HashMap::new(),
        }
    }

    /// Check if a function name is a stdlib function
    pub fn is_stdlib_function(name: &str) -> bool {
        matches!(
            name,
            // Math functions
            "Math__abs" | "Math__min" | "Math__max" | "Math__sqrt" | "Math__pow" |
            "Math__sin" | "Math__cos" | "Math__tan" | "Math__floor" | "Math__ceil" |
            "Math__round" | "Math__log" | "Math__log10" | "Math__exp" | "Math__pi" |
            "Math__e" | "Math__random" |
            // Type conversions
            "to_int" | "to_float" | "to_string" |
            // String functions
            "Str__len" | "Str__compare" | "Str__concat" | "Str__upper" | "Str__lower" |
            "Str__trim" | "Str__contains" | "Str__startsWith" | "Str__endsWith" |
            // I/O functions
            "read_line" |
            // System functions
            "System__exit" | "exit" | "System__cwd" | "System__os" | "System__shell" |
            "System__exec" | "System__getenv" |
            // Time functions
            "Time__now" | "Time__unix" | "Time__sleep" |
            // File functions
            "File__read" | "File__write" | "File__exists" | "File__delete" |
            // Args functions
            "Args__get" | "Args__len" |
            // Assertion functions
            "assert" | "assert_eq" | "assert_ne" | "assert_true" | "assert_false" | "fail" |
            // Range function
            "range"
        )
    }

    /// Compile full program.
    pub fn compile(&mut self, program: &Program) -> Result<()> {
        self.compile_internal(program, None, None)
    }

    /// Compile program while only emitting bodies for selected top-level symbols.
    /// Declarations are still emitted for the whole program to keep cross-file references valid.
    pub fn compile_filtered(
        &mut self,
        program: &Program,
        active_symbols: &HashSet<String>,
    ) -> Result<()> {
        self.compile_internal(program, Some(active_symbols), None)
    }

    pub fn compile_filtered_with_decl_symbols(
        &mut self,
        program: &Program,
        active_symbols: &HashSet<String>,
        declaration_symbols: &HashSet<String>,
    ) -> Result<()> {
        self.compile_internal(program, Some(active_symbols), Some(declaration_symbols))
    }

    fn compile_internal(
        &mut self,
        program: &Program,
        active_symbols: Option<&HashSet<String>>,
        declaration_symbols: Option<&HashSet<String>>,
    ) -> Result<()> {
        let specialized_program;
        let program = if Self::program_has_explicit_generic_calls(program) {
            specialized_program = Self::specialize_explicit_generic_calls(program)?;
            &specialized_program
        } else {
            program
        };

        self.import_aliases.clear();
        for decl in &program.declarations {
            if let Decl::Import(import) = &decl.node {
                if let Some(alias) = &import.alias {
                    self.import_aliases
                        .insert(alias.clone(), import.path.clone());
                }
            }
        }

        // First pass (0): declare all enums first so Named(Enum) resolves correctly.
        for decl in &program.declarations {
            let should_declare = declaration_symbols
                .map(|symbols| self.should_compile_decl(&decl.node, symbols))
                .unwrap_or(true);
            if !should_declare {
                continue;
            }
            if let Decl::Enum(en) = &decl.node {
                self.declare_enum(en)?;
            }
        }

        // First pass: declare all classes and functions
        for decl in &program.declarations {
            let should_declare = declaration_symbols
                .map(|symbols| self.should_compile_decl(&decl.node, symbols))
                .unwrap_or(true);
            if !should_declare {
                continue;
            }
            match &decl.node {
                Decl::Class(class) => self.declare_class(class)?,
                Decl::Function(func) => {
                    self.declare_function(func)?;
                }
                Decl::Enum(_) => {}
                Decl::Interface(_) => {} // Interfaces don't generate code
                Decl::Module(module) => self.declare_module(module)?,
                Decl::Import(_) => {} // Handled at file level
            }
        }

        // Second pass: compile function bodies
        for decl in &program.declarations {
            let should_compile = active_symbols
                .map(|symbols| self.should_compile_decl(&decl.node, symbols))
                .unwrap_or(true);
            if !should_compile {
                continue;
            }
            match &decl.node {
                Decl::Function(func) => self.compile_function(func)?,
                Decl::Class(class) => self.compile_class(class)?,
                Decl::Enum(_) => {}
                Decl::Interface(_) => {} // Interfaces don't generate code
                Decl::Module(module) => {
                    if let Some(symbols) = active_symbols {
                        self.compile_module_filtered(module, symbols)?;
                    } else {
                        self.compile_module(module)?;
                    }
                }
                Decl::Import(_) => {} // Handled at file level
            }
        }

        Ok(())
    }

    fn should_compile_decl(&self, decl: &Decl, active_symbols: &HashSet<String>) -> bool {
        fn module_has_active_symbol(
            module: &ModuleDecl,
            prefix: &str,
            active_symbols: &HashSet<String>,
        ) -> bool {
            if active_symbols.contains(prefix) {
                return true;
            }
            for inner in &module.declarations {
                match &inner.node {
                    Decl::Function(func) => {
                        if active_symbols.contains(&format!("{}__{}", prefix, func.name)) {
                            return true;
                        }
                    }
                    Decl::Class(class) => {
                        if active_symbols.contains(&format!("{}__{}", prefix, class.name)) {
                            return true;
                        }
                    }
                    Decl::Enum(en) => {
                        if active_symbols.contains(&format!("{}__{}", prefix, en.name)) {
                            return true;
                        }
                    }
                    Decl::Module(nested) => {
                        let nested_prefix = format!("{}__{}", prefix, nested.name);
                        if module_has_active_symbol(nested, &nested_prefix, active_symbols) {
                            return true;
                        }
                    }
                    Decl::Interface(_) | Decl::Import(_) => {}
                }
            }
            false
        }

        match decl {
            Decl::Function(func) => {
                active_symbols.contains(&func.name) || func.name.contains("__spec__")
            }
            Decl::Class(class) => active_symbols.contains(&class.name),
            Decl::Module(module) => module_has_active_symbol(module, &module.name, active_symbols),
            Decl::Enum(en) => active_symbols.contains(&en.name),
            Decl::Interface(_) | Decl::Import(_) => false,
        }
    }

    fn resolve_module_alias(&self, name: &str) -> String {
        if let Some(path) = self.import_aliases.get(name) {
            let mut owner: Option<String> = None;
            for (func, ns) in stdlib_registry().get_functions() {
                if ns == path {
                    if let Some((candidate_owner, _)) = func.split_once("__") {
                        let candidate_owner = candidate_owner.to_string();
                        if let Some(existing) = &owner {
                            if existing != &candidate_owner {
                                return name.to_string();
                            }
                        } else {
                            owner = Some(candidate_owner);
                        }
                    }
                }
            }
            if let Some(owner) = owner {
                return owner;
            }
        }
        name.to_string()
    }

    fn resolve_function_alias(&self, name: &str) -> String {
        let Some(path) = self.import_aliases.get(name) else {
            return name.to_string();
        };
        if path.ends_with(".*") {
            return name.to_string();
        }
        let mut parts = path.split('.').collect::<Vec<_>>();
        let Some(symbol) = parts.pop() else {
            return name.to_string();
        };
        let namespace = parts.join(".");

        if stdlib_registry()
            .get_namespace(symbol)
            .is_some_and(|owner| owner == &namespace)
            || self.functions.contains_key(symbol)
        {
            return symbol.to_string();
        }

        name.to_string()
    }

    fn resolve_method_function_name(&self, class_name: &str, method: &str) -> Option<String> {
        let mut current = class_name.to_string();
        let mut depth = 0usize;
        while depth < 64 {
            let candidate = format!("{}__{}", current, method);
            if self.functions.contains_key(&candidate) {
                return Some(candidate);
            }
            let next = self.classes.get(&current)?.extends.clone();
            match next {
                Some(parent) => current = parent,
                None => break,
            }
            depth += 1;
        }
        None
    }

    fn flatten_field_chain(expr: &Expr) -> Option<Vec<String>> {
        match expr {
            Expr::Ident(name) => Some(vec![name.clone()]),
            Expr::Field { object, field } => {
                let mut parts = Self::flatten_field_chain(&object.node)?;
                parts.push(field.clone());
                Some(parts)
            }
            _ => None,
        }
    }

    pub fn declare_enum(&mut self, en: &EnumDecl) -> Result<()> {
        let payload_slots = en
            .variants
            .iter()
            .map(|v| v.fields.len())
            .max()
            .unwrap_or(0);

        // Runtime representation:
        // { tag: i8, payload_0: i64, payload_1: i64, ... }
        let mut fields: Vec<BasicTypeEnum<'ctx>> = vec![self.context.i8_type().into()];
        for _ in 0..payload_slots {
            fields.push(self.context.i64_type().into());
        }
        let struct_type = self.context.struct_type(&fields, false);

        let mut variants = HashMap::new();
        for (i, variant) in en.variants.iter().enumerate() {
            variants.insert(
                variant.name.clone(),
                EnumVariantInfo {
                    tag: i as u8,
                    fields: variant.fields.iter().map(|f| f.ty.clone()).collect(),
                },
            );
            self.enum_variant_to_enum
                .insert(variant.name.clone(), en.name.clone());
        }

        self.enums.insert(
            en.name.clone(),
            EnumInfo {
                struct_type,
                payload_slots,
                variants,
            },
        );

        Ok(())
    }

    pub fn declare_module(&mut self, module: &ModuleDecl) -> Result<()> {
        self.declare_module_with_prefix(module, &module.name)
    }

    pub fn compile_module(&mut self, module: &ModuleDecl) -> Result<()> {
        self.compile_module_with_prefix(module, &module.name)
    }

    fn compile_module_filtered(
        &mut self,
        module: &ModuleDecl,
        active_symbols: &HashSet<String>,
    ) -> Result<()> {
        self.compile_module_filtered_with_prefix(module, &module.name, active_symbols)
    }

    fn declare_module_with_prefix(&mut self, module: &ModuleDecl, prefix: &str) -> Result<()> {
        for decl in &module.declarations {
            match &decl.node {
                Decl::Function(func) => {
                    let mut prefixed_func = func.clone();
                    prefixed_func.name = format!("{}__{}", prefix, func.name);
                    self.declare_function(&prefixed_func)?;
                }
                Decl::Class(class) => {
                    let mut prefixed_class = class.clone();
                    prefixed_class.name = format!("{}__{}", prefix, class.name);
                    self.declare_class(&prefixed_class)?;
                }
                Decl::Enum(en) => {
                    let mut prefixed_enum = en.clone();
                    prefixed_enum.name = format!("{}__{}", prefix, en.name);
                    self.declare_enum(&prefixed_enum)?;
                }
                Decl::Module(nested) => {
                    let nested_prefix = format!("{}__{}", prefix, nested.name);
                    self.declare_module_with_prefix(nested, &nested_prefix)?;
                }
                Decl::Interface(_) | Decl::Import(_) => {}
            }
        }
        Ok(())
    }

    fn compile_module_with_prefix(&mut self, module: &ModuleDecl, prefix: &str) -> Result<()> {
        for decl in &module.declarations {
            match &decl.node {
                Decl::Function(func) => {
                    let mut prefixed_func = func.clone();
                    prefixed_func.name = format!("{}__{}", prefix, func.name);
                    self.compile_function(&prefixed_func)?;
                }
                Decl::Class(class) => {
                    let mut prefixed_class = class.clone();
                    prefixed_class.name = format!("{}__{}", prefix, class.name);
                    self.compile_class(&prefixed_class)?;
                }
                Decl::Enum(_) => {}
                Decl::Module(nested) => {
                    let nested_prefix = format!("{}__{}", prefix, nested.name);
                    self.compile_module_with_prefix(nested, &nested_prefix)?;
                }
                Decl::Interface(_) | Decl::Import(_) => {}
            }
        }
        Ok(())
    }

    fn compile_module_filtered_with_prefix(
        &mut self,
        module: &ModuleDecl,
        prefix: &str,
        active_symbols: &HashSet<String>,
    ) -> Result<()> {
        if active_symbols.contains(prefix) {
            return self.compile_module_with_prefix(module, prefix);
        }

        for decl in &module.declarations {
            match &decl.node {
                Decl::Function(func) => {
                    let prefixed = format!("{}__{}", prefix, func.name);
                    if active_symbols.contains(&prefixed) {
                        let mut prefixed_func = func.clone();
                        prefixed_func.name = prefixed;
                        self.compile_function(&prefixed_func)?;
                    }
                }
                Decl::Class(class) => {
                    let prefixed = format!("{}__{}", prefix, class.name);
                    if active_symbols.contains(&prefixed) {
                        let mut prefixed_class = class.clone();
                        prefixed_class.name = prefixed;
                        self.compile_class(&prefixed_class)?;
                    }
                }
                Decl::Enum(_) => {}
                Decl::Module(nested) => {
                    let nested_prefix = format!("{}__{}", prefix, nested.name);
                    self.compile_module_filtered_with_prefix(
                        nested,
                        &nested_prefix,
                        active_symbols,
                    )?;
                }
                Decl::Interface(_) | Decl::Import(_) => {}
            }
        }
        Ok(())
    }

    // === Type System ===

    pub fn llvm_type(&self, ty: &Type) -> BasicTypeEnum<'ctx> {
        match ty {
            Type::Integer => self.context.i64_type().into(),
            Type::Float => self.context.f64_type().into(),
            Type::Boolean => self.context.bool_type().into(),
            Type::String => self.context.ptr_type(AddressSpace::default()).into(),
            Type::Char => self.context.i8_type().into(),
            Type::None => self.context.i8_type().into(),
            Type::Named(name) => {
                if let Some(enum_info) = self.enums.get(name) {
                    enum_info.struct_type.into()
                } else {
                    self.context.ptr_type(AddressSpace::default()).into()
                }
            }
            Type::Generic(_, _) => self.context.ptr_type(AddressSpace::default()).into(),
            Type::Function(_, _) => self
                .context
                .struct_type(
                    &[
                        self.context.ptr_type(AddressSpace::default()).into(), // function pointer
                        self.context.ptr_type(AddressSpace::default()).into(), // environment pointer
                    ],
                    false,
                )
                .into(),
            // Option<T> is represented as a struct { is_some: i8, value: T }
            Type::Option(inner) => {
                let inner_ty = self.llvm_type(inner);
                self.context
                    .struct_type(
                        &[
                            self.context.i8_type().into(), // tag: 0=None, 1=Some
                            inner_ty,                      // value
                        ],
                        false,
                    )
                    .into()
            }
            // Result<T, E> is represented as struct { is_ok: i8, ok_value: T, err_value: E }
            Type::Result(ok_ty, err_ty) => {
                let ok_llvm = self.llvm_type(ok_ty);
                let err_llvm = self.llvm_type(err_ty);
                self.context
                    .struct_type(
                        &[
                            self.context.i8_type().into(), // tag: 1=Ok, 0=Error
                            ok_llvm,                       // ok value
                            err_llvm,                      // error value
                        ],
                        false,
                    )
                    .into()
            }
            // List<T> is represented as struct { capacity: i64, length: i64, data: ptr }
            Type::List(_) => {
                self.context
                    .struct_type(
                        &[
                            self.context.i64_type().into(),                        // capacity
                            self.context.i64_type().into(),                        // length
                            self.context.ptr_type(AddressSpace::default()).into(), // data pointer
                        ],
                        false,
                    )
                    .into()
            }
            // Map<K, V> - for now just a pointer (will need proper implementation)
            Type::Map(_, _) => {
                self.context
                    .struct_type(
                        &[
                            self.context.i64_type().into(),                        // capacity
                            self.context.i64_type().into(),                        // length
                            self.context.ptr_type(AddressSpace::default()).into(), // keys pointer
                            self.context.ptr_type(AddressSpace::default()).into(), // values pointer
                        ],
                        false,
                    )
                    .into()
            }
            // Set<T> - similar to List
            Type::Set(_) => {
                self.context
                    .struct_type(
                        &[
                            self.context.i64_type().into(),                        // capacity
                            self.context.i64_type().into(),                        // length
                            self.context.ptr_type(AddressSpace::default()).into(), // data pointer
                        ],
                        false,
                    )
                    .into()
            }
            // Reference types - represented as pointers
            Type::Ref(_) | Type::MutRef(_) => self.context.ptr_type(AddressSpace::default()).into(),
            // Smart pointers - all represented as pointers
            Type::Box(_) | Type::Rc(_) | Type::Arc(_) => {
                self.context.ptr_type(AddressSpace::default()).into()
            }
            Type::Ptr(_) => self.context.ptr_type(AddressSpace::default()).into(),
            // Task<T> - runtime task handle pointer
            Type::Task(_) => self.context.ptr_type(AddressSpace::default()).into(),
            // Range<T> - represented as a struct { start, end, step }
            Type::Range(_) => self.context.ptr_type(AddressSpace::default()).into(),
        }
    }

    // === Classes ===

    pub fn declare_class(&mut self, class: &ClassDecl) -> Result<()> {
        let mut field_llvm_types: Vec<BasicTypeEnum<'ctx>> = Vec::new();
        let mut field_indices: HashMap<String, u32> = HashMap::new();
        let mut field_types_map: HashMap<String, Type> = HashMap::new();

        let mut next_index = 0u32;
        if let Some(parent) = &class.extends {
            let parent_info = self
                .classes
                .get(parent)
                .ok_or_else(|| CodegenError::new(format!("Unknown base class: {}", parent)))?;
            let mut parent_fields = parent_info
                .field_indices
                .iter()
                .map(|(name, idx)| (name.clone(), *idx))
                .collect::<Vec<_>>();
            parent_fields.sort_by_key(|(_, idx)| *idx);

            for (name, idx) in parent_fields {
                let ty = parent_info
                    .field_types
                    .get(&name)
                    .ok_or_else(|| CodegenError::new("Missing inherited field type"))?
                    .clone();
                field_llvm_types.push(self.llvm_type(&ty));
                field_indices.insert(name.clone(), idx);
                field_types_map.insert(name, ty);
                next_index = next_index.max(idx + 1);
            }
        }

        for field in &class.fields {
            if field_indices.contains_key(&field.name) {
                return Err(CodegenError::new(format!(
                    "Field '{}' already exists in base class",
                    field.name
                )));
            }
            let i = next_index;
            field_llvm_types.push(self.llvm_type(&field.ty));
            field_indices.insert(field.name.clone(), i);
            field_types_map.insert(field.name.clone(), field.ty.clone());
            next_index += 1;
        }

        let struct_type = self.context.struct_type(&field_llvm_types, false);
        self.classes.insert(
            class.name.clone(),
            ClassInfo {
                struct_type,
                field_indices,
                field_types: field_types_map,
                extends: class.extends.clone(),
            },
        );

        // Declare constructor
        if class.constructor.is_some() {
            self.declare_class_constructor(class)?;
        }

        // Declare methods
        for method in &class.methods {
            self.declare_class_method(class, method)?;
        }

        Ok(())
    }

    pub fn declare_class_constructor(&mut self, class: &ClassDecl) -> Result<()> {
        let constructor = class.constructor.as_ref().ok_or_else(|| {
            CodegenError::new(format!("Class '{}' has no constructor", class.name))
        })?;
        let param_types: Vec<BasicMetadataTypeEnum> = constructor
            .params
            .iter()
            .map(|p| self.llvm_type(&p.ty).into())
            .collect();

        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![
            self.context.ptr_type(AddressSpace::default()).into(), // env_ptr
        ];
        llvm_params.extend(param_types);

        let ret_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ret_type.fn_type(&llvm_params, false);

        let name = format!("{}__new", class.name);
        let func = self.module.add_function(&name, fn_type, None);
        self.functions.insert(
            name,
            (
                func,
                Type::Function(
                    constructor.params.iter().map(|p| p.ty.clone()).collect(),
                    Box::new(Type::Named(class.name.clone())),
                ),
            ),
        );

        Ok(())
    }

    pub fn declare_class_method(&mut self, class: &ClassDecl, method: &FunctionDecl) -> Result<()> {
        let self_type = self.context.ptr_type(AddressSpace::default());

        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![
            self.context.ptr_type(AddressSpace::default()).into(), // env_ptr
            self_type.into(),                                      // this
        ];
        for param in &method.params {
            llvm_params.push(self.llvm_type(&param.ty).into());
        }

        let fn_type = match &method.return_type {
            Type::None => self.context.void_type().fn_type(&llvm_params, false),
            ty => self.llvm_type(ty).fn_type(&llvm_params, false),
        };

        let name = format!("{}__{}", class.name, method.name);
        let func = self.module.add_function(&name, fn_type, None);
        self.functions.insert(
            name,
            (
                func,
                Type::Function(
                    method.params.iter().map(|p| p.ty.clone()).collect(),
                    Box::new(method.return_type.clone()),
                ),
            ),
        );

        Ok(())
    }

    pub fn compile_class(&mut self, class: &ClassDecl) -> Result<()> {
        if let Some(constructor) = &class.constructor {
            self.compile_constructor(class, constructor)?;
        }

        for method in &class.methods {
            self.compile_method(class, method)?;
        }

        Ok(())
    }

    pub fn compile_constructor(
        &mut self,
        class: &ClassDecl,
        constructor: &Constructor,
    ) -> Result<()> {
        let name = format!("{}__new", class.name);
        let (func, _) =
            self.functions.get(&name).cloned().ok_or_else(|| {
                CodegenError::new(format!("Missing declared constructor: {}", name))
            })?;

        self.current_function = Some(func);
        let entry = self.context.append_basic_block(func, "entry");
        self.builder.position_at_end(entry);
        self.variables.clear();

        // Allocate parameters
        // Param 0 is env_ptr, constructor params start at 1
        for (i, param) in constructor.params.iter().enumerate() {
            let llvm_param = func.get_nth_param((i + 1) as u32).ok_or_else(|| {
                CodegenError::new(format!(
                    "Missing constructor parameter {} for {}",
                    i + 1,
                    class.name
                ))
            })?;
            let alloca = self
                .builder
                .build_alloca(self.llvm_type(&param.ty), &param.name)
                .map_err(|e| {
                    CodegenError::new(format!("alloca failed for '{}': {}", param.name, e))
                })?;
            self.builder.build_store(alloca, llvm_param).map_err(|e| {
                CodegenError::new(format!("store failed for '{}': {}", param.name, e))
            })?;
            self.variables.insert(
                param.name.clone(),
                Variable {
                    ptr: alloca,
                    ty: param.ty.clone(),
                },
            );
        }

        // Allocate instance
        let class_info = self
            .classes
            .get(&class.name)
            .ok_or_else(|| CodegenError::new(format!("Unknown class: {}", class.name)))?;
        let struct_type = class_info.struct_type;
        let malloc = self.get_or_declare_malloc();
        let size = struct_type
            .size_of()
            .ok_or_else(|| CodegenError::new("Failed to compute class struct size"))?;
        let ptr = self
            .builder
            .build_call(malloc, &[size.into()], "instance")
            .map_err(|e| CodegenError::new(format!("malloc call failed: {}", e)))?;
        let instance = match ptr.try_as_basic_value() {
            ValueKind::Basic(val) => val.into_pointer_value(),
            _ => {
                return Err(CodegenError::new(
                    "malloc call did not produce a pointer result",
                ))
            }
        };

        // Store 'this'
        let this_alloca = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "this")
            .map_err(|e| CodegenError::new(format!("alloca failed for this: {}", e)))?;
        self.builder
            .build_store(this_alloca, instance)
            .map_err(|e| CodegenError::new(format!("store failed for this: {}", e)))?;
        self.variables.insert(
            "this".to_string(),
            Variable {
                ptr: this_alloca,
                ty: Type::Named(class.name.clone()),
            },
        );

        // Compile body
        for stmt in &constructor.body {
            self.compile_stmt(&stmt.node)?;
        }

        // Return instance
        let this_val = self
            .builder
            .build_load(
                self.context.ptr_type(AddressSpace::default()),
                this_alloca,
                "this",
            )
            .map_err(|e| CodegenError::new(format!("load failed for this: {}", e)))?;
        self.builder
            .build_return(Some(&this_val))
            .map_err(|e| CodegenError::new(format!("return failed in constructor: {}", e)))?;

        self.current_function = None;
        Ok(())
    }

    pub fn compile_method(&mut self, class: &ClassDecl, method: &FunctionDecl) -> Result<()> {
        let name = format!("{}__{}", class.name, method.name);
        let (func, _) = self
            .functions
            .get(&name)
            .cloned()
            .ok_or_else(|| CodegenError::new(format!("Missing declared method: {}", name)))?;

        self.current_function = Some(func);
        let entry = self.context.append_basic_block(func, "entry");
        self.builder.position_at_end(entry);
        self.variables.clear();

        // Param 0 is env_ptr
        // Store 'this' (Param 1)
        let this_param = func.get_nth_param(1).ok_or_else(|| {
            CodegenError::new(format!("Missing 'this' param for method {}", name))
        })?;
        let class_info = self
            .classes
            .get(&class.name)
            .ok_or_else(|| CodegenError::new(format!("Unknown class: {}", class.name)))?;
        let _struct_type = class_info.struct_type;
        let this_alloca = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "this")
            .map_err(|e| CodegenError::new(format!("alloca failed for this: {}", e)))?;
        self.builder
            .build_store(this_alloca, this_param)
            .map_err(|e| CodegenError::new(format!("store failed for this: {}", e)))?;
        self.variables.insert(
            "this".to_string(),
            Variable {
                ptr: this_alloca,
                ty: Type::Named(class.name.clone()),
            },
        );

        // Store parameters
        // Start from index 2 because 0=env_ptr, 1=this
        for (i, param) in method.params.iter().enumerate() {
            let llvm_param = func.get_nth_param((i + 2) as u32).ok_or_else(|| {
                CodegenError::new(format!("Missing method parameter {} for {}", i + 2, name))
            })?;
            let alloca = self
                .builder
                .build_alloca(self.llvm_type(&param.ty), &param.name)
                .map_err(|e| {
                    CodegenError::new(format!("alloca failed for '{}': {}", param.name, e))
                })?;
            self.builder.build_store(alloca, llvm_param).map_err(|e| {
                CodegenError::new(format!("store failed for '{}': {}", param.name, e))
            })?;
            self.variables.insert(
                param.name.clone(),
                Variable {
                    ptr: alloca,
                    ty: param.ty.clone(),
                },
            );
        }

        // Compile body
        for stmt in &method.body {
            self.compile_stmt(&stmt.node)?;
        }

        // Add implicit return
        if self.needs_terminator() {
            match &method.return_type {
                Type::None => {
                    self.builder.build_return(None).map_err(|e| {
                        CodegenError::new(format!("return failed in method {}: {}", name, e))
                    })?;
                }
                _ => {
                    self.builder.build_unreachable().map_err(|e| {
                        CodegenError::new(format!(
                            "unreachable emit failed in method {}: {}",
                            name, e
                        ))
                    })?;
                }
            }
        }

        self.current_function = None;
        Ok(())
    }

    // === Functions ===

    pub fn declare_function(&mut self, func: &FunctionDecl) -> Result<FunctionValue<'ctx>> {
        if func.is_extern {
            return self.declare_extern_function(func);
        }

        if func.is_async {
            if func.name == "main" {
                return Err(CodegenError::new(
                    "async main is not supported; use a sync main() and await tasks inside async functions",
                ));
            }
            return self.declare_async_function(func);
        }

        let param_types: Vec<BasicMetadataTypeEnum> = func
            .params
            .iter()
            .map(|p| self.llvm_type(&p.ty).into())
            .collect();

        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![
            self.context.ptr_type(AddressSpace::default()).into(), // env_ptr
        ];
        llvm_params.extend(param_types);

        // Main function always returns i32 for C runtime compatibility
        let fn_type = if func.name == "main" {
            // main(argc: i32, argv: i8**)
            let main_params: Vec<BasicMetadataTypeEnum> = vec![
                self.context.i32_type().into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ];
            self.context.i32_type().fn_type(&main_params, false)
        } else {
            match &func.return_type {
                Type::None => self.context.void_type().fn_type(&llvm_params, false),
                ty => self.llvm_type(ty).fn_type(&llvm_params, false),
            }
        };

        let function = self.module.add_function(&func.name, fn_type, None);

        // Add optimization attributes
        // Always inline small functions
        if func.params.len() <= 3 && !func.name.starts_with("main") {
            let always_inline = self
                .context
                .create_enum_attribute(Attribute::get_named_enum_kind_id("alwaysinline"), 0);
            function.add_attribute(AttributeLoc::Function, always_inline);
        }

        // Function doesn't unwind (no exceptions)
        let no_unwind = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("nounwind"), 0);
        function.add_attribute(AttributeLoc::Function, no_unwind);

        // Function will return (no infinite loops in analyzed functions)
        let will_return = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("willreturn"), 0);
        function.add_attribute(AttributeLoc::Function, will_return);

        self.functions.insert(
            func.name.clone(),
            (
                function,
                Type::Function(
                    func.params.iter().map(|p| p.ty.clone()).collect(),
                    Box::new(func.return_type.clone()),
                ),
            ),
        );
        Ok(function)
    }

    fn declare_extern_function(&mut self, func: &FunctionDecl) -> Result<FunctionValue<'ctx>> {
        let param_types: Vec<BasicMetadataTypeEnum> = func
            .params
            .iter()
            .map(|p| self.llvm_type(&p.ty).into())
            .collect();

        let fn_type = match &func.return_type {
            Type::None => self
                .context
                .void_type()
                .fn_type(&param_types, func.is_variadic),
            ty => self.llvm_type(ty).fn_type(&param_types, func.is_variadic),
        };

        let symbol_name = func.extern_link_name.as_deref().unwrap_or(&func.name);
        let function = self.module.add_function(symbol_name, fn_type, None);
        match func.extern_abi.as_deref().unwrap_or("c") {
            // On current targets, C/system are both emitted as default C calling convention.
            "c" | "system" => {
                function.set_call_conventions(0);
            }
            _ => {}
        }
        self.extern_functions.insert(func.name.clone());
        self.functions.insert(
            func.name.clone(),
            (
                function,
                Type::Function(
                    func.params.iter().map(|p| p.ty.clone()).collect(),
                    Box::new(func.return_type.clone()),
                ),
            ),
        );
        Ok(function)
    }

    fn task_struct_type(&self) -> StructType<'ctx> {
        let ptr = self.context.ptr_type(AddressSpace::default());
        self.context.struct_type(
            &[
                self.context.i64_type().into(),
                ptr.into(),
                self.context.i8_type().into(),
                self.context.i8_type().into(),
            ],
            false,
        )
    }

    fn async_inner_return_type(&self, ty: &Type) -> Type {
        if let Type::Task(inner) = ty {
            (**inner).clone()
        } else {
            ty.clone()
        }
    }

    fn build_atomic_bool_load(
        &mut self,
        ptr: PointerValue<'ctx>,
        name: &str,
        ordering: AtomicOrdering,
    ) -> Result<IntValue<'ctx>> {
        let raw = self
            .builder
            .build_load(self.context.i8_type(), ptr, name)
            .unwrap()
            .into_int_value();
        let block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::new("atomic load used outside basic block"))?;
        let inst = block
            .get_last_instruction()
            .ok_or_else(|| CodegenError::new("failed to capture atomic load instruction"))?;
        inst.set_atomic_ordering(ordering)
            .map_err(|e| CodegenError::new(format!("failed to set atomic load ordering: {e}")))?;
        Ok(self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                raw,
                self.context.i8_type().const_zero(),
                &format!("{name}_bool"),
            )
            .unwrap())
    }

    fn build_atomic_bool_store(
        &mut self,
        ptr: PointerValue<'ctx>,
        value: IntValue<'ctx>,
        ordering: AtomicOrdering,
    ) -> Result<()> {
        let byte_value = self
            .builder
            .build_int_cast(value, self.context.i8_type(), "atomic_flag_store")
            .unwrap();
        let inst = self.builder.build_store(ptr, byte_value).unwrap();
        inst.set_atomic_ordering(ordering)
            .map_err(|e| CodegenError::new(format!("failed to set atomic store ordering: {e}")))?;
        Ok(())
    }

    fn declare_async_function(&mut self, func: &FunctionDecl) -> Result<FunctionValue<'ctx>> {
        let inner_return = self.async_inner_return_type(&func.return_type);
        let task_return = Type::Task(Box::new(inner_return.clone()));
        let ptr_type = self.context.ptr_type(AddressSpace::default());

        let mut wrapper_params: Vec<BasicMetadataTypeEnum> = vec![ptr_type.into()];
        let mut env_fields: Vec<BasicTypeEnum<'ctx>> = Vec::new();
        for param in &func.params {
            let llvm = self.llvm_type(&param.ty);
            wrapper_params.push(llvm.into());
            env_fields.push(llvm);
        }
        env_fields.push(ptr_type.into());
        let env_type = self.context.struct_type(&env_fields, false);

        let wrapper_fn_type = ptr_type.fn_type(&wrapper_params, false);
        let wrapper = self.module.add_function(&func.name, wrapper_fn_type, None);

        let body_name = format!("__apex_async_body__{}", func.name);
        let body_fn_type = match &inner_return {
            Type::None => self.context.void_type().fn_type(&[ptr_type.into()], false),
            ty => self.llvm_type(ty).fn_type(&[ptr_type.into()], false),
        };
        let body = self.module.add_function(&body_name, body_fn_type, None);

        let thunk_name = format!("__apex_async_thunk__{}", func.name);
        let thunk_fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        let thunk = self.module.add_function(&thunk_name, thunk_fn_type, None);

        self.functions.insert(
            func.name.clone(),
            (
                wrapper,
                Type::Function(
                    func.params.iter().map(|p| p.ty.clone()).collect(),
                    Box::new(task_return),
                ),
            ),
        );

        self.async_functions.insert(
            func.name.clone(),
            AsyncFunctionPlan {
                wrapper,
                body,
                thunk,
                env_type,
                inner_return_type: inner_return,
            },
        );

        Ok(wrapper)
    }

    fn create_task(
        &mut self,
        runner_fn: PointerValue<'ctx>,
        env_ptr: PointerValue<'ctx>,
        env_task_slot_ptr: PointerValue<'ctx>,
    ) -> Result<PointerValue<'ctx>> {
        let task_ty = self.task_struct_type();
        let malloc = self.get_or_declare_malloc();
        let pthread_create = self.get_or_declare_pthread_create();
        let size = task_ty
            .size_of()
            .ok_or_else(|| CodegenError::new("failed to compute Task runtime size"))?;

        let raw = self
            .builder
            .build_call(malloc, &[size.into()], "task_alloc")
            .unwrap()
            .try_as_basic_value();
        let task_raw = match raw {
            ValueKind::Basic(BasicValueEnum::PointerValue(p)) => p,
            _ => return Err(CodegenError::new("malloc should return pointer for Task")),
        };

        let task_ptr = self
            .builder
            .build_pointer_cast(
                task_raw,
                self.context.ptr_type(AddressSpace::default()),
                "task_ptr",
            )
            .unwrap();

        let i32_ty = self.context.i32_type();
        let zero = i32_ty.const_int(0, false);
        let thread_idx = i32_ty.const_int(0, false);
        let result_idx = i32_ty.const_int(1, false);
        let done_idx = i32_ty.const_int(2, false);
        let completed_idx = i32_ty.const_int(3, false);

        let thread_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, thread_idx], "task_thread_field")
                .unwrap()
        };
        self.builder
            .build_store(thread_field, self.context.i64_type().const_int(0, false))
            .unwrap();

        let result_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, result_idx], "task_result_ptr")
                .unwrap()
        };
        self.builder
            .build_store(
                result_field,
                self.context.ptr_type(AddressSpace::default()).const_null(),
            )
            .unwrap();

        let done_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done")
                .unwrap()
        };
        self.builder
            .build_store(done_field, self.context.i8_type().const_int(0, false))
            .unwrap();
        let completed_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, completed_idx], "task_completed")
                .unwrap()
        };
        self.builder
            .build_store(completed_field, self.context.i8_type().const_int(0, false))
            .unwrap();
        self.builder
            .build_store(env_task_slot_ptr, task_ptr)
            .unwrap();

        let thread_tmp = self
            .builder
            .build_alloca(self.context.i64_type(), "task_thread_tmp")
            .unwrap();
        self.builder
            .build_store(thread_tmp, self.context.i64_type().const_int(0, false))
            .unwrap();
        let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
        let start_fn = self
            .builder
            .build_pointer_cast(
                runner_fn,
                self.context.ptr_type(AddressSpace::default()),
                "task_start_fn",
            )
            .unwrap();
        let _spawn_status = self
            .builder
            .build_call(
                pthread_create,
                &[
                    thread_tmp.into(),
                    null_ptr.into(),
                    start_fn.into(),
                    env_ptr.into(),
                ],
                "task_spawn",
            )
            .unwrap();

        let thread_val = self
            .builder
            .build_load(self.context.i64_type(), thread_tmp, "task_thread")
            .unwrap();
        self.builder.build_store(thread_field, thread_val).unwrap();

        Ok(self
            .builder
            .build_pointer_cast(
                task_ptr,
                self.context.ptr_type(AddressSpace::default()),
                "task_raw",
            )
            .unwrap())
    }

    fn await_task(
        &mut self,
        task_raw: PointerValue<'ctx>,
        inner_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let task_ty = self.task_struct_type();
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let pthread_join = self.get_or_declare_pthread_join();
        let task_ptr = self
            .builder
            .build_pointer_cast(
                task_raw,
                self.context.ptr_type(AddressSpace::default()),
                "task_cast",
            )
            .unwrap();

        let i32_ty = self.context.i32_type();
        let zero = i32_ty.const_int(0, false);
        let thread_idx = i32_ty.const_int(0, false);
        let result_idx = i32_ty.const_int(1, false);
        let done_idx = i32_ty.const_int(2, false);

        let done_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done_ptr")
                .unwrap()
        };
        let done_val = self
            .builder
            .build_load(self.context.i8_type(), done_field, "task_done")
            .unwrap()
            .into_int_value();
        let done_ready = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                done_val,
                self.context.i8_type().const_zero(),
                "task_done_ready",
            )
            .unwrap();

        let result_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, result_idx], "task_result_field")
                .unwrap()
        };
        let existing_result = self
            .builder
            .build_load(ptr_ty, result_field, "task_result_existing")
            .unwrap()
            .into_pointer_value();

        let current_bb = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::new("await used outside of basic block"))?;
        let current_fn = self
            .current_function
            .ok_or_else(|| CodegenError::new("await used outside of function"))?;
        let join_bb = self.context.append_basic_block(current_fn, "task_join");
        let cont_bb = self.context.append_basic_block(current_fn, "task_cont");

        self.builder
            .build_conditional_branch(done_ready, cont_bb, join_bb)
            .unwrap();

        self.builder.position_at_end(join_bb);
        let thread_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, thread_idx], "task_thread_ptr")
                .unwrap()
        };
        let thread_id = self
            .builder
            .build_load(self.context.i64_type(), thread_field, "task_thread_id")
            .unwrap()
            .into_int_value();
        let join_result_ptr = self
            .builder
            .build_alloca(ptr_ty, "task_join_result")
            .unwrap();
        self.builder
            .build_store(join_result_ptr, ptr_ty.const_null())
            .unwrap();
        let _join_status = self
            .builder
            .build_call(
                pthread_join,
                &[thread_id.into(), join_result_ptr.into()],
                "task_join_call",
            )
            .unwrap();
        let new_result = self
            .builder
            .build_load(ptr_ty, join_result_ptr, "task_joined_result")
            .unwrap()
            .into_pointer_value();
        self.builder.build_store(result_field, new_result).unwrap();
        self.builder
            .build_store(done_field, self.context.i8_type().const_int(1, false))
            .unwrap();
        self.builder.build_unconditional_branch(cont_bb).unwrap();

        self.builder.position_at_end(cont_bb);
        let phi = self.builder.build_phi(ptr_ty, "task_result_phi").unwrap();
        phi.add_incoming(&[(&existing_result, current_bb), (&new_result, join_bb)]);
        let result_ptr = phi.as_basic_value().into_pointer_value();

        if matches!(inner_ty, Type::None) {
            return Ok(self.context.i8_type().const_int(0, false).into());
        }

        let result_ty = self.llvm_type(inner_ty);
        let typed_ptr = self
            .builder
            .build_pointer_cast(
                result_ptr,
                self.context.ptr_type(AddressSpace::default()),
                "task_result_typed",
            )
            .unwrap();
        Ok(self
            .builder
            .build_load(result_ty, typed_ptr, "task_result")
            .unwrap())
    }

    fn compile_async_function(&mut self, func: &FunctionDecl) -> Result<()> {
        let plan = self
            .async_functions
            .get(&func.name)
            .ok_or_else(|| CodegenError::new(format!("Missing async plan for {}", func.name)))?;
        let wrapper = plan.wrapper;
        let body = plan.body;
        let thunk = plan.thunk;
        let env_type = plan.env_type;
        let inner_return_type = plan.inner_return_type.clone();
        let ptr_type = self.context.ptr_type(AddressSpace::default());

        // 1) Compile body function: __apex_async_body__*
        self.current_function = Some(body);
        self.current_return_type = Some(inner_return_type.clone());
        self.variables.clear();
        self.loop_stack.clear();
        let body_entry = self.context.append_basic_block(body, "entry");
        self.builder.position_at_end(body_entry);

        let env_raw = body
            .get_nth_param(0)
            .ok_or_else(|| CodegenError::new("Async body missing env parameter"))?
            .into_pointer_value();
        let env_ptr = self
            .builder
            .build_pointer_cast(
                env_raw,
                self.context.ptr_type(AddressSpace::default()),
                "async_env_cast",
            )
            .unwrap();

        for (i, param) in func.params.iter().enumerate() {
            let field_ptr = unsafe {
                self.builder
                    .build_gep(
                        env_type,
                        env_ptr,
                        &[
                            self.context.i32_type().const_int(0, false),
                            self.context.i32_type().const_int(i as u64, false),
                        ],
                        "async_param_field",
                    )
                    .unwrap()
            };
            let loaded = self
                .builder
                .build_load(self.llvm_type(&param.ty), field_ptr, &param.name)
                .unwrap();
            let alloca = self
                .builder
                .build_alloca(self.llvm_type(&param.ty), &param.name)
                .unwrap();
            self.builder.build_store(alloca, loaded).unwrap();
            self.variables.insert(
                param.name.clone(),
                Variable {
                    ptr: alloca,
                    ty: param.ty.clone(),
                },
            );
        }

        for stmt in &func.body {
            self.compile_stmt(&stmt.node)?;
        }
        if self.needs_terminator() {
            if matches!(inner_return_type, Type::None) {
                self.builder.build_return(None).unwrap();
            } else {
                self.builder.build_unreachable().unwrap();
            }
        }

        // 2) Compile thunk: __apex_async_thunk__*
        self.current_function = Some(thunk);
        self.current_return_type = None;
        self.variables.clear();
        self.loop_stack.clear();
        let thunk_entry = self.context.append_basic_block(thunk, "entry");
        self.builder.position_at_end(thunk_entry);

        let thunk_env = thunk
            .get_nth_param(0)
            .ok_or_else(|| CodegenError::new("Async thunk missing env parameter"))?
            .into_pointer_value();
        let thunk_env_cast = self
            .builder
            .build_pointer_cast(thunk_env, ptr_type, "async_thunk_env_cast")
            .unwrap();

        let body_call = self
            .builder
            .build_call(body, &[thunk_env.into()], "async_body_call")
            .unwrap();

        let malloc = self.get_or_declare_malloc();
        let result_storage = if matches!(inner_return_type, Type::None) {
            let raw = self
                .builder
                .build_call(
                    malloc,
                    &[self.context.i64_type().const_int(1, false).into()],
                    "async_none_alloc",
                )
                .unwrap()
                .try_as_basic_value();
            let ptr = match raw {
                ValueKind::Basic(BasicValueEnum::PointerValue(p)) => p,
                _ => {
                    return Err(CodegenError::new(
                        "malloc failed for async Task<None> result",
                    ))
                }
            };
            let none_ptr = self
                .builder
                .build_pointer_cast(
                    ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "async_none_ptr",
                )
                .unwrap();
            self.builder
                .build_store(none_ptr, self.context.i8_type().const_int(0, false))
                .unwrap();
            ptr
        } else {
            let ret_ty = self.llvm_type(&inner_return_type);
            let size = ret_ty
                .size_of()
                .ok_or_else(|| CodegenError::new("failed to compute async result size"))?;
            let raw = self
                .builder
                .build_call(malloc, &[size.into()], "async_result_alloc")
                .unwrap()
                .try_as_basic_value();
            let ptr = match raw {
                ValueKind::Basic(BasicValueEnum::PointerValue(p)) => p,
                _ => return Err(CodegenError::new("malloc failed for async result")),
            };
            let typed_ptr = self
                .builder
                .build_pointer_cast(
                    ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "async_result_ptr",
                )
                .unwrap();
            let result = match body_call.try_as_basic_value() {
                ValueKind::Basic(v) => v,
                ValueKind::Instruction(_) => {
                    return Err(CodegenError::new(
                        "async body should return value for non-None Task",
                    ));
                }
            };
            self.builder.build_store(typed_ptr, result).unwrap();
            ptr
        };
        let task_field_ptr = unsafe {
            self.builder
                .build_gep(
                    env_type,
                    thunk_env_cast,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context
                            .i32_type()
                            .const_int(func.params.len() as u64, false),
                    ],
                    "async_task_field",
                )
                .unwrap()
        };
        let task_ptr = self
            .builder
            .build_load(ptr_type, task_field_ptr, "async_task_ptr")
            .unwrap()
            .into_pointer_value();
        let result_field = unsafe {
            self.builder
                .build_gep(
                    self.task_struct_type(),
                    task_ptr,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context.i32_type().const_int(1, false),
                    ],
                    "async_task_result_ptr",
                )
                .unwrap()
        };
        self.builder
            .build_store(result_field, result_storage)
            .unwrap();
        let completed_field = unsafe {
            self.builder
                .build_gep(
                    self.task_struct_type(),
                    task_ptr,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context.i32_type().const_int(3, false),
                    ],
                    "async_task_completed_ptr",
                )
                .unwrap()
        };
        self.build_atomic_bool_store(
            completed_field,
            self.context.i8_type().const_int(1, false),
            AtomicOrdering::Release,
        )?;
        self.builder.build_return(Some(&result_storage)).unwrap();

        // 3) Compile public wrapper: function name(...)
        self.current_function = Some(wrapper);
        self.current_return_type = Some(Type::Task(Box::new(inner_return_type.clone())));
        self.variables.clear();
        self.loop_stack.clear();
        let wrapper_entry = self.context.append_basic_block(wrapper, "entry");
        self.builder.position_at_end(wrapper_entry);

        let env_size = env_type
            .size_of()
            .ok_or_else(|| CodegenError::new("failed to compute async environment size"))?;
        let env_alloc = self
            .builder
            .build_call(malloc, &[env_size.into()], "async_env_alloc")
            .unwrap()
            .try_as_basic_value();
        let env_raw_ptr = match env_alloc {
            ValueKind::Basic(BasicValueEnum::PointerValue(p)) => p,
            _ => return Err(CodegenError::new("malloc failed for async environment")),
        };
        let env_cast = self
            .builder
            .build_pointer_cast(
                env_raw_ptr,
                self.context.ptr_type(AddressSpace::default()),
                "async_env_store",
            )
            .unwrap();

        for (i, param) in func.params.iter().enumerate() {
            let param_val = wrapper.get_nth_param((i + 1) as u32).ok_or_else(|| {
                CodegenError::new(format!("Missing async wrapper parameter {}", param.name))
            })?;
            let field_ptr = unsafe {
                self.builder
                    .build_gep(
                        env_type,
                        env_cast,
                        &[
                            self.context.i32_type().const_int(0, false),
                            self.context.i32_type().const_int(i as u64, false),
                        ],
                        "async_env_field",
                    )
                    .unwrap()
            };
            self.builder.build_store(field_ptr, param_val).unwrap();
        }
        let task_slot_ptr = unsafe {
            self.builder
                .build_gep(
                    env_type,
                    env_cast,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context
                            .i32_type()
                            .const_int(func.params.len() as u64, false),
                    ],
                    "async_env_task_slot",
                )
                .unwrap()
        };
        self.builder
            .build_store(task_slot_ptr, ptr_type.const_null())
            .unwrap();

        let task = self.create_task(
            thunk.as_global_value().as_pointer_value(),
            env_raw_ptr,
            task_slot_ptr,
        )?;
        self.builder.build_return(Some(&task)).unwrap();

        self.current_function = None;
        self.current_return_type = None;
        Ok(())
    }

    pub fn compile_function(&mut self, func: &FunctionDecl) -> Result<()> {
        if func.is_extern {
            return Ok(());
        }

        if func.is_async {
            return self.compile_async_function(func);
        }

        let (function, _) = self.functions.get(&func.name).unwrap().clone();

        self.current_function = Some(function);
        self.current_return_type = Some(func.return_type.clone());
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);
        self.variables.clear();
        self.loop_stack.clear();

        // Special handling for main: store argc/argv in globals
        if func.name == "main" {
            let argc = function.get_nth_param(0).unwrap().into_int_value();
            let argv = function.get_nth_param(1).unwrap().into_pointer_value();

            let argc_global = match self.module.get_global("_apex_argc") {
                Some(g) => g,
                None => {
                    let g = self
                        .module
                        .add_global(self.context.i32_type(), None, "_apex_argc");
                    g.set_initializer(&self.context.i32_type().const_int(0, false));
                    g
                }
            };
            self.builder
                .build_store(argc_global.as_pointer_value(), argc)
                .unwrap();

            let argv_global = match self.module.get_global("_apex_argv") {
                Some(g) => g,
                None => {
                    let g = self.module.add_global(
                        self.context.ptr_type(AddressSpace::default()),
                        None,
                        "_apex_argv",
                    );
                    g.set_initializer(&self.context.ptr_type(AddressSpace::default()).const_null());
                    g
                }
            };
            self.builder
                .build_store(argv_global.as_pointer_value(), argv)
                .unwrap();
        }

        // Allocate parameters
        // Param 0 is argc for main, but for other functions 0 is env_ptr
        // We skip argc/argv for main in the regular parameter allocation loop
        // because main() in Apex is usually main(): None
        let start_idx = if func.name == "main" { 2 } else { 1 };
        for (i, param) in func.params.iter().enumerate() {
            let llvm_param = function.get_nth_param((i + start_idx) as u32).unwrap();
            let alloca = self
                .builder
                .build_alloca(self.llvm_type(&param.ty), &param.name)
                .unwrap();
            self.builder.build_store(alloca, llvm_param).unwrap();
            self.variables.insert(
                param.name.clone(),
                Variable {
                    ptr: alloca,
                    ty: param.ty.clone(),
                },
            );
        }

        // Compile body
        for stmt in &func.body {
            self.compile_stmt(&stmt.node)?;
        }

        // Add implicit return
        if self.needs_terminator() {
            if func.name == "main" {
                // Main returns 0 for success
                let zero = self.context.i32_type().const_int(0, false);
                self.builder.build_return(Some(&zero)).unwrap();
            } else {
                match &func.return_type {
                    Type::None => {
                        self.builder.build_return(None).unwrap();
                    }
                    _ => {
                        self.builder.build_unreachable().unwrap();
                    }
                }
            }
        }

        self.current_function = None;
        Ok(())
    }

    // === Statements ===

    pub fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                mutable: _,
            } => {
                let val = self.compile_expr(&value.node)?;
                let alloca = self.builder.build_alloca(self.llvm_type(ty), name).unwrap();
                self.builder.build_store(alloca, val).unwrap();
                self.variables.insert(
                    name.clone(),
                    Variable {
                        ptr: alloca,
                        ty: ty.clone(),
                    },
                );
            }

            Stmt::Assign { target, value } => {
                let val = self.compile_expr(&value.node)?;
                let ptr = self.compile_lvalue(&target.node)?;
                self.builder.build_store(ptr, val).unwrap();
            }

            Stmt::Expr(expr) => {
                self.compile_expr(&expr.node)?;
            }

            Stmt::Return(value) => {
                // Check if we're in main function (returns i32)
                let is_main = self
                    .current_function
                    .map(|f| f.get_name().to_str().unwrap_or("") == "main")
                    .unwrap_or(false);

                match value {
                    Some(expr) => {
                        // Check if returning None literal
                        if matches!(&expr.node, Expr::Literal(Literal::None)) {
                            if is_main {
                                let zero = self.context.i32_type().const_int(0, false);
                                self.builder.build_return(Some(&zero)).unwrap();
                            } else {
                                self.builder.build_return(None).unwrap();
                            }
                        } else {
                            let val = self.compile_expr(&expr.node)?;
                            // Main function must return i32 for C compatibility
                            let ret_val = if is_main && val.is_int_value() {
                                let int_val = val.into_int_value();
                                if int_val.get_type().get_bit_width() != 32 {
                                    self.builder
                                        .build_int_truncate(
                                            int_val,
                                            self.context.i32_type(),
                                            "ret_cast",
                                        )
                                        .unwrap()
                                        .into()
                                } else {
                                    val
                                }
                            } else {
                                val
                            };
                            self.builder.build_return(Some(&ret_val)).unwrap();
                        }
                    }
                    None => {
                        if is_main {
                            let zero = self.context.i32_type().const_int(0, false);
                            self.builder.build_return(Some(&zero)).unwrap();
                        } else {
                            self.builder.build_return(None).unwrap();
                        }
                    }
                }
            }

            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.compile_if(condition, then_block, else_block.as_ref())?;
            }

            Stmt::While { condition, body } => {
                self.compile_while(condition, body)?;
            }

            Stmt::For {
                var,
                var_type,
                iterable,
                body,
            } => {
                self.compile_for(var, var_type.as_ref(), iterable, body)?;
            }

            Stmt::Break => {
                if let Some(loop_ctx) = self.loop_stack.last() {
                    self.builder
                        .build_unconditional_branch(loop_ctx.after_block)
                        .unwrap();
                }
            }

            Stmt::Continue => {
                if let Some(loop_ctx) = self.loop_stack.last() {
                    self.builder
                        .build_unconditional_branch(loop_ctx.loop_block)
                        .unwrap();
                }
            }

            Stmt::Match { expr, arms } => {
                self.compile_match_stmt(expr, arms)?;
            }
        }

        Ok(())
    }

    pub fn compile_if(
        &mut self,
        cond: &Spanned<Expr>,
        then_block: &Block,
        else_block: Option<&Block>,
    ) -> Result<()> {
        let cond_val = self.compile_expr(&cond.node)?.into_int_value();
        let func = self.current_function.unwrap();

        let then_bb = self.context.append_basic_block(func, "then");
        let else_bb = self.context.append_basic_block(func, "else");
        let merge_bb = self.context.append_basic_block(func, "merge");

        self.builder
            .build_conditional_branch(cond_val, then_bb, else_bb)
            .unwrap();

        // Then
        self.builder.position_at_end(then_bb);
        for stmt in then_block {
            self.compile_stmt(&stmt.node)?;
        }
        if self.needs_terminator() {
            self.builder.build_unconditional_branch(merge_bb).unwrap();
        }

        // Else
        self.builder.position_at_end(else_bb);
        if let Some(else_stmts) = else_block {
            for stmt in else_stmts {
                self.compile_stmt(&stmt.node)?;
            }
        }
        if self.needs_terminator() {
            self.builder.build_unconditional_branch(merge_bb).unwrap();
        }

        self.builder.position_at_end(merge_bb);
        Ok(())
    }

    pub fn compile_while(&mut self, cond: &Spanned<Expr>, body: &Block) -> Result<()> {
        let func = self.current_function.unwrap();

        // LOOP ROTATION OPTIMIZATION:
        // Instead of: while (cond) { body }
        // We generate: if (cond) { do { body } while (cond) }
        // This eliminates one branch per iteration!

        let entry_bb = self.context.append_basic_block(func, "while.entry");
        let body_bb = self.context.append_basic_block(func, "while.body");
        let cond_bb = self.context.append_basic_block(func, "while.cond");
        let after_bb = self.context.append_basic_block(func, "while.after");

        // First, check condition (entry test)
        self.builder.build_unconditional_branch(entry_bb).unwrap();
        self.builder.position_at_end(entry_bb);
        let entry_cond = self.compile_expr(&cond.node)?.into_int_value();
        self.builder
            .build_conditional_branch(entry_cond, body_bb, after_bb)
            .unwrap();

        // Body (executed at least once if we get here)
        self.builder.position_at_end(body_bb);
        self.loop_stack.push(LoopContext {
            loop_block: cond_bb,
            after_block: after_bb,
        });
        for stmt in body {
            self.compile_stmt(&stmt.node)?;
        }
        self.loop_stack.pop();
        if self.needs_terminator() {
            self.builder.build_unconditional_branch(cond_bb).unwrap();
        }

        // Loop condition check at end (loop rotation)
        self.builder.position_at_end(cond_bb);
        let loop_cond = self.compile_expr(&cond.node)?.into_int_value();

        // Branch prediction: likely to continue looping
        self.builder
            .build_conditional_branch(loop_cond, body_bb, after_bb)
            .unwrap();

        self.builder.position_at_end(after_bb);
        Ok(())
    }

    pub fn compile_for(
        &mut self,
        var: &str,
        var_type: Option<&Type>,
        iterable: &Spanned<Expr>,
        body: &Block,
    ) -> Result<()> {
        let func = self.current_function.unwrap();

        if let Expr::Ident(list_name) = &iterable.node {
            if let Some(list_var) = self.variables.get(list_name).cloned() {
                if let Type::List(inner) = list_var.ty {
                    let iter_ty = var_type.cloned().unwrap_or((*inner).clone());
                    let var_alloca = self
                        .builder
                        .build_alloca(self.llvm_type(&iter_ty), var)
                        .unwrap();
                    self.variables.insert(
                        var.to_string(),
                        Variable {
                            ptr: var_alloca,
                            ty: iter_ty.clone(),
                        },
                    );

                    let i64_type = self.context.i64_type();
                    let i32_type = self.context.i32_type();
                    let zero_i64 = i64_type.const_zero();
                    let one_i64 = i64_type.const_int(1, false);
                    let elem_size = if matches!(*inner, Type::Boolean) {
                        1
                    } else {
                        8
                    };
                    let elem_llvm = self.llvm_type(&inner);
                    let list_type = self.context.struct_type(
                        &[
                            i64_type.into(),
                            i64_type.into(),
                            self.context.ptr_type(AddressSpace::default()).into(),
                        ],
                        false,
                    );

                    let idx_alloca = self.builder.build_alloca(i64_type, "for_list_idx").unwrap();
                    self.builder.build_store(idx_alloca, zero_i64).unwrap();

                    let len_ptr = unsafe {
                        self.builder
                            .build_gep(
                                list_type.as_basic_type_enum(),
                                list_var.ptr,
                                &[i32_type.const_int(0, false), i32_type.const_int(1, false)],
                                "list_len_ptr",
                            )
                            .unwrap()
                    };
                    let data_ptr_ptr = unsafe {
                        self.builder
                            .build_gep(
                                list_type.as_basic_type_enum(),
                                list_var.ptr,
                                &[i32_type.const_int(0, false), i32_type.const_int(2, false)],
                                "list_data_ptr_ptr",
                            )
                            .unwrap()
                    };

                    let cond_bb = self.context.append_basic_block(func, "for_list.cond");
                    let body_bb = self.context.append_basic_block(func, "for_list.body");
                    let inc_bb = self.context.append_basic_block(func, "for_list.inc");
                    let after_bb = self.context.append_basic_block(func, "for_list.after");
                    self.builder.build_unconditional_branch(cond_bb).unwrap();

                    self.builder.position_at_end(cond_bb);
                    let idx_val = self
                        .builder
                        .build_load(i64_type, idx_alloca, "for_list_idx_val")
                        .unwrap()
                        .into_int_value();
                    let len_val = self
                        .builder
                        .build_load(i64_type, len_ptr, "for_list_len")
                        .unwrap()
                        .into_int_value();
                    let cond = self
                        .builder
                        .build_int_compare(IntPredicate::SLT, idx_val, len_val, "for_list_cmp")
                        .unwrap();
                    self.builder
                        .build_conditional_branch(cond, body_bb, after_bb)
                        .unwrap();

                    self.builder.position_at_end(body_bb);
                    let data_ptr = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            data_ptr_ptr,
                            "for_list_data",
                        )
                        .unwrap()
                        .into_pointer_value();
                    let byte_offset = self
                        .builder
                        .build_int_mul(
                            idx_val,
                            i64_type.const_int(elem_size, false),
                            "for_list_off",
                        )
                        .unwrap();
                    let elem_ptr = unsafe {
                        self.builder
                            .build_gep(
                                self.context.i8_type(),
                                data_ptr,
                                &[byte_offset],
                                "for_list_elem_ptr",
                            )
                            .unwrap()
                    };
                    let typed_ptr = self
                        .builder
                        .build_pointer_cast(
                            elem_ptr,
                            self.context.ptr_type(AddressSpace::default()),
                            "for_list_typed_ptr",
                        )
                        .unwrap();
                    let elem_val = self
                        .builder
                        .build_load(elem_llvm, typed_ptr, "for_list_elem")
                        .unwrap();
                    self.builder.build_store(var_alloca, elem_val).unwrap();

                    self.loop_stack.push(LoopContext {
                        loop_block: inc_bb,
                        after_block: after_bb,
                    });
                    for stmt in body {
                        self.compile_stmt(&stmt.node)?;
                    }
                    self.loop_stack.pop();
                    if self.needs_terminator() {
                        self.builder.build_unconditional_branch(inc_bb).unwrap();
                    }

                    self.builder.position_at_end(inc_bb);
                    let next_idx = self
                        .builder
                        .build_int_add(idx_val, one_i64, "for_list_next")
                        .unwrap();
                    self.builder.build_store(idx_alloca, next_idx).unwrap();
                    self.builder.build_unconditional_branch(cond_bb).unwrap();

                    self.builder.position_at_end(after_bb);
                    return Ok(());
                }
            }
        }

        let ty = var_type.cloned().unwrap_or(Type::Integer);
        let var_alloca = self.builder.build_alloca(self.llvm_type(&ty), var).unwrap();

        // Default range values
        let mut start_val = self.context.i64_type().const_int(0, false).into();
        let mut end_val = self.context.i64_type().const_int(0, false).into();
        let mut inclusive = false;

        match &iterable.node {
            Expr::Range {
                start,
                end,
                inclusive: inc,
            } => {
                if let Some(s) = start {
                    start_val = self.compile_expr(&s.node)?;
                }
                if let Some(e) = end {
                    end_val = self.compile_expr(&e.node)?;
                }
                inclusive = *inc;
            }
            _ => {
                // Treat as 0..N where N is the expression value
                end_val = self.compile_expr(&iterable.node)?;
            }
        }

        let end_val = end_val.into_int_value();
        self.builder.build_store(var_alloca, start_val).unwrap();

        self.variables.insert(
            var.to_string(),
            Variable {
                ptr: var_alloca,
                ty: ty.clone(),
            },
        );

        let cond_bb = self.context.append_basic_block(func, "for.cond");
        let body_bb = self.context.append_basic_block(func, "for.body");
        let inc_bb = self.context.append_basic_block(func, "for.inc");
        let after_bb = self.context.append_basic_block(func, "for.after");

        self.builder.build_unconditional_branch(cond_bb).unwrap();

        // Condition
        self.builder.position_at_end(cond_bb);
        let current = self
            .builder
            .build_load(self.context.i64_type(), var_alloca, var)
            .unwrap()
            .into_int_value();

        let cond = if inclusive {
            self.builder
                .build_int_compare(IntPredicate::SLE, current, end_val, "cmp")
                .unwrap()
        } else {
            self.builder
                .build_int_compare(IntPredicate::SLT, current, end_val, "cmp")
                .unwrap()
        };

        self.builder
            .build_conditional_branch(cond, body_bb, after_bb)
            .unwrap();

        // Body
        self.builder.position_at_end(body_bb);
        self.loop_stack.push(LoopContext {
            loop_block: inc_bb,
            after_block: after_bb,
        });
        for stmt in body {
            self.compile_stmt(&stmt.node)?;
        }
        self.loop_stack.pop();
        if self.needs_terminator() {
            self.builder.build_unconditional_branch(inc_bb).unwrap();
        }

        // Increment
        self.builder.position_at_end(inc_bb);
        let current = self
            .builder
            .build_load(self.context.i64_type(), var_alloca, var)
            .unwrap()
            .into_int_value();
        let one = self.context.i64_type().const_int(1, false);
        let next = self.builder.build_int_add(current, one, "inc").unwrap();
        self.builder.build_store(var_alloca, next).unwrap();
        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(after_bb);
        Ok(())
    }

    fn encode_enum_payload(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<IntValue<'ctx>> {
        let i64_type = self.context.i64_type();
        let encoded = match ty {
            Type::Integer => value.into_int_value(),
            Type::Boolean => self
                .builder
                .build_int_z_extend(value.into_int_value(), i64_type, "bool_to_i64")
                .unwrap(),
            Type::Char => self
                .builder
                .build_int_z_extend(value.into_int_value(), i64_type, "char_to_i64")
                .unwrap(),
            Type::Float => self
                .builder
                .build_bit_cast(value.into_float_value(), i64_type, "float_bits")
                .unwrap()
                .into_int_value(),
            Type::String | Type::Named(_) | Type::Ref(_) | Type::MutRef(_) | Type::Ptr(_) => self
                .builder
                .build_ptr_to_int(value.into_pointer_value(), i64_type, "ptr_to_i64")
                .unwrap(),
            _ => {
                return Err(CodegenError::new(
                    "Unsupported enum payload type for codegen",
                ));
            }
        };
        Ok(encoded)
    }

    pub(crate) fn decode_enum_payload(
        &self,
        raw: IntValue<'ctx>,
        ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let i64_type = self.context.i64_type();
        let decoded = match ty {
            Type::Integer => raw.into(),
            Type::Boolean => self
                .builder
                .build_int_truncate(raw, self.context.bool_type(), "i64_to_bool")
                .unwrap()
                .into(),
            Type::Char => self
                .builder
                .build_int_truncate(raw, self.context.i8_type(), "i64_to_char")
                .unwrap()
                .into(),
            Type::Float => self
                .builder
                .build_bit_cast(raw, self.context.f64_type(), "bits_to_float")
                .unwrap(),
            Type::String | Type::Named(_) | Type::Ref(_) | Type::MutRef(_) | Type::Ptr(_) => self
                .builder
                .build_int_to_ptr(
                    raw,
                    self.context.ptr_type(AddressSpace::default()),
                    "i64_to_ptr",
                )
                .unwrap()
                .into(),
            _ => {
                return Err(CodegenError::new(
                    "Unsupported enum payload type for codegen",
                ));
            }
        };
        let _ = i64_type; // keep layout assumptions explicit
        Ok(decoded)
    }

    fn build_enum_value(
        &mut self,
        enum_name: &str,
        variant_info: &EnumVariantInfo,
        values: &[BasicValueEnum<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let enum_info = self
            .enums
            .get(enum_name)
            .ok_or_else(|| CodegenError::new(format!("Unknown enum '{}'", enum_name)))?;
        let mut value = enum_info.struct_type.get_undef();

        value = self
            .builder
            .build_insert_value(
                value,
                self.context
                    .i8_type()
                    .const_int(variant_info.tag as u64, false),
                0,
                "enum_tag",
            )
            .unwrap()
            .into_struct_value();

        for (i, field_ty) in variant_info.fields.iter().enumerate() {
            let encoded = self.encode_enum_payload(values[i], field_ty)?;
            value = self
                .builder
                .build_insert_value(value, encoded, (i + 1) as u32, "enum_payload")
                .unwrap()
                .into_struct_value();
        }

        Ok(value.into())
    }

    pub fn compile_match_stmt(&mut self, expr: &Spanned<Expr>, arms: &[MatchArm]) -> Result<()> {
        let val = self.compile_expr(&expr.node)?;
        let func = self.current_function.unwrap();
        let merge_bb = self.context.append_basic_block(func, "match.merge");

        let match_ty = self.infer_expr_type(&expr.node, &[]);
        let option_inner_ty = match &match_ty {
            Type::Option(inner) => Some((**inner).clone()),
            _ => None,
        };
        let result_inner_tys = match &match_ty {
            Type::Result(ok, err) => Some(((**ok).clone(), (**err).clone())),
            _ => None,
        };
        let enum_match_name = match &match_ty {
            Type::Named(name) if self.enums.contains_key(name) => Some(name.clone()),
            _ => None,
        };

        let mut dispatch_bb = self.builder.get_insert_block().unwrap();

        for arm in arms {
            let arm_bb = self.context.append_basic_block(func, "match.arm");
            let next_bb = self.context.append_basic_block(func, "match.next");

            self.builder.position_at_end(dispatch_bb);
            match &arm.pattern {
                Pattern::Wildcard | Pattern::Ident(_) => {
                    self.builder.build_unconditional_branch(arm_bb).unwrap();
                }
                Pattern::Literal(lit) => {
                    let pattern_val = self.compile_literal(lit)?;
                    let cond = if val.is_int_value() && pattern_val.is_int_value() {
                        self.builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                val.into_int_value(),
                                pattern_val.into_int_value(),
                                "match_lit_eq",
                            )
                            .unwrap()
                    } else if val.is_float_value() && pattern_val.is_float_value() {
                        self.builder
                            .build_float_compare(
                                FloatPredicate::OEQ,
                                val.into_float_value(),
                                pattern_val.into_float_value(),
                                "match_float_eq",
                            )
                            .unwrap()
                    } else if val.is_pointer_value() && pattern_val.is_pointer_value() {
                        let strcmp = self.get_or_declare_strcmp();
                        let cmp = self
                            .builder
                            .build_call(
                                strcmp,
                                &[
                                    val.into_pointer_value().into(),
                                    pattern_val.into_pointer_value().into(),
                                ],
                                "match_strcmp",
                            )
                            .unwrap();
                        let cmp_val = self.extract_call_value(cmp).into_int_value();
                        self.builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                cmp_val,
                                self.context.i32_type().const_int(0, false),
                                "match_str_eq",
                            )
                            .unwrap()
                    } else {
                        self.context.bool_type().const_int(0, false)
                    };
                    self.builder
                        .build_conditional_branch(cond, arm_bb, next_bb)
                        .unwrap();
                }
                Pattern::Variant(variant_name, _) => {
                    // Built-in Option / Result matching
                    if matches!(variant_name.as_str(), "Some" | "None" | "Ok" | "Error") {
                        let expected_tag = match variant_name.as_str() {
                            "Some" | "Ok" => 1u64,
                            _ => 0u64,
                        };
                        let tag = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 0, "tag")
                            .unwrap()
                            .into_int_value();
                        let cond = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                tag,
                                self.context.i8_type().const_int(expected_tag, false),
                                "match_variant_eq",
                            )
                            .unwrap();
                        self.builder
                            .build_conditional_branch(cond, arm_bb, next_bb)
                            .unwrap();
                    } else if let Some(enum_name) = &enum_match_name {
                        if let Some(enum_info) = self.enums.get(enum_name) {
                            if let Some(variant_info) = enum_info.variants.get(variant_name) {
                                let tag = self
                                    .builder
                                    .build_extract_value(val.into_struct_value(), 0, "tag")
                                    .unwrap()
                                    .into_int_value();
                                let cond = self
                                    .builder
                                    .build_int_compare(
                                        IntPredicate::EQ,
                                        tag,
                                        self.context
                                            .i8_type()
                                            .const_int(variant_info.tag as u64, false),
                                        "match_enum_variant_eq",
                                    )
                                    .unwrap();
                                self.builder
                                    .build_conditional_branch(cond, arm_bb, next_bb)
                                    .unwrap();
                            } else {
                                self.builder.build_unconditional_branch(next_bb).unwrap();
                            }
                        } else {
                            self.builder.build_unconditional_branch(next_bb).unwrap();
                        }
                    } else {
                        self.builder.build_unconditional_branch(next_bb).unwrap();
                    }
                }
            }

            self.builder.position_at_end(arm_bb);
            match &arm.pattern {
                Pattern::Ident(binding) => {
                    let alloca = self.builder.build_alloca(val.get_type(), binding).unwrap();
                    self.builder.build_store(alloca, val).unwrap();
                    self.variables.insert(
                        binding.clone(),
                        Variable {
                            ptr: alloca,
                            ty: match_ty.clone(),
                        },
                    );
                }
                Pattern::Variant(variant_name, bindings) => {
                    if variant_name == "Some" && !bindings.is_empty() {
                        let inner = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 1, "some_inner")
                            .unwrap();
                        let alloca = self
                            .builder
                            .build_alloca(inner.get_type(), &bindings[0])
                            .unwrap();
                        self.builder.build_store(alloca, inner).unwrap();
                        self.variables.insert(
                            bindings[0].clone(),
                            Variable {
                                ptr: alloca,
                                ty: option_inner_ty.clone().unwrap_or(Type::Integer),
                            },
                        );
                    } else if variant_name == "Ok" && !bindings.is_empty() {
                        let inner = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 1, "ok_inner")
                            .unwrap();
                        let alloca = self
                            .builder
                            .build_alloca(inner.get_type(), &bindings[0])
                            .unwrap();
                        self.builder.build_store(alloca, inner).unwrap();
                        self.variables.insert(
                            bindings[0].clone(),
                            Variable {
                                ptr: alloca,
                                ty: result_inner_tys
                                    .as_ref()
                                    .map(|(ok, _)| ok.clone())
                                    .unwrap_or(Type::Integer),
                            },
                        );
                    } else if variant_name == "Error" && !bindings.is_empty() {
                        let inner = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 2, "err_inner")
                            .unwrap();
                        let alloca = self
                            .builder
                            .build_alloca(inner.get_type(), &bindings[0])
                            .unwrap();
                        self.builder.build_store(alloca, inner).unwrap();
                        self.variables.insert(
                            bindings[0].clone(),
                            Variable {
                                ptr: alloca,
                                ty: result_inner_tys
                                    .as_ref()
                                    .map(|(_, err)| err.clone())
                                    .unwrap_or(Type::String),
                            },
                        );
                    } else if let Some(enum_name) = &enum_match_name {
                        if let Some(enum_info) = self.enums.get(enum_name) {
                            if let Some(variant_info) = enum_info.variants.get(variant_name) {
                                for (idx, binding) in bindings.iter().enumerate() {
                                    if let Some(field_ty) = variant_info.fields.get(idx) {
                                        let raw = self
                                            .builder
                                            .build_extract_value(
                                                val.into_struct_value(),
                                                (idx + 1) as u32,
                                                "enum_payload_raw",
                                            )
                                            .unwrap()
                                            .into_int_value();
                                        let decoded = self.decode_enum_payload(raw, field_ty)?;
                                        let alloca = self
                                            .builder
                                            .build_alloca(decoded.get_type(), binding)
                                            .unwrap();
                                        self.builder.build_store(alloca, decoded).unwrap();
                                        self.variables.insert(
                                            binding.clone(),
                                            Variable {
                                                ptr: alloca,
                                                ty: field_ty.clone(),
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }

            for stmt in &arm.body {
                self.compile_stmt(&stmt.node)?;
            }
            if self.needs_terminator() {
                self.builder.build_unconditional_branch(merge_bb).unwrap();
            }

            dispatch_bb = next_bb;
            self.builder.position_at_end(dispatch_bb);
        }

        if self.needs_terminator() {
            self.builder.build_unconditional_branch(merge_bb).unwrap();
        }

        self.builder.position_at_end(merge_bb);
        Ok(())
    }

    // === Expressions ===

    fn infer_await_inner_type(&self, expr: &Expr) -> Type {
        let inferred = self.infer_expr_type(expr, &[]);
        if let Type::Task(inner) = inferred {
            *inner
        } else {
            Type::Integer
        }
    }

    fn infer_async_block_return_type(&self, body: &[Spanned<Stmt>]) -> Type {
        let mut ret: Option<Type> = None;
        for stmt in body {
            if let Stmt::Return(Some(expr)) = &stmt.node {
                let ty = self.infer_expr_type(&expr.node, &[]);
                if ret.is_none() {
                    ret = Some(ty);
                }
            }
        }
        ret.unwrap_or(Type::None)
    }

    fn compile_async_block(&mut self, body: &[Spanned<Stmt>]) -> Result<BasicValueEnum<'ctx>> {
        let mut captures: Vec<(String, Type)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let params = std::collections::HashSet::new();
        for stmt in body {
            self.walk_stmt_for_captures(&stmt.node, &params, &mut captures, &mut seen);
        }

        let inner_return_type = self.infer_async_block_return_type(body);
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let mut env_fields = Vec::new();
        for (_, ty) in &captures {
            env_fields.push(self.llvm_type(ty));
        }
        env_fields.push(ptr_ty.into());
        let env_type = self.context.struct_type(&env_fields, false);

        let malloc = self.get_or_declare_malloc();
        let env_size = env_type
            .size_of()
            .ok_or_else(|| CodegenError::new("failed to compute async block env size"))?;
        let env_alloc = self
            .builder
            .build_call(malloc, &[env_size.into()], "async_block_env")
            .unwrap()
            .try_as_basic_value();
        let env_raw = match env_alloc {
            ValueKind::Basic(BasicValueEnum::PointerValue(p)) => p,
            _ => return Err(CodegenError::new("malloc failed for async block env")),
        };
        let env_cast = self
            .builder
            .build_pointer_cast(
                env_raw,
                self.context.ptr_type(AddressSpace::default()),
                "async_block_env_cast",
            )
            .unwrap();

        for (i, (name, ty)) in captures.iter().enumerate() {
            let var = self.variables.get(name).ok_or_else(|| {
                CodegenError::new(format!("async block capture '{}' not found", name))
            })?;
            let val = self
                .builder
                .build_load(self.llvm_type(ty), var.ptr, name)
                .unwrap();
            let field_ptr = unsafe {
                self.builder
                    .build_gep(
                        env_type,
                        env_cast,
                        &[
                            self.context.i32_type().const_int(0, false),
                            self.context.i32_type().const_int(i as u64, false),
                        ],
                        "async_block_capture",
                    )
                    .unwrap()
            };
            self.builder.build_store(field_ptr, val).unwrap();
        }
        let task_slot_ptr = unsafe {
            self.builder
                .build_gep(
                    env_type,
                    env_cast,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context
                            .i32_type()
                            .const_int(captures.len() as u64, false),
                    ],
                    "async_block_task_slot",
                )
                .unwrap()
        };
        self.builder
            .build_store(task_slot_ptr, ptr_ty.const_null())
            .unwrap();

        let saved_function = self.current_function;
        let saved_return_type = self.current_return_type.clone();
        let saved_variables = std::mem::take(&mut self.variables);
        let saved_loop_stack = std::mem::take(&mut self.loop_stack);
        let saved_insert_block = self.builder.get_insert_block();

        let id = self.async_counter;
        self.async_counter += 1;

        let body_name = format!("__apex_async_block_body_{}", id);
        let body_fn_type = match &inner_return_type {
            Type::None => self.context.void_type().fn_type(&[ptr_ty.into()], false),
            ty => self.llvm_type(ty).fn_type(&[ptr_ty.into()], false),
        };
        let body_fn = self.module.add_function(&body_name, body_fn_type, None);

        self.current_function = Some(body_fn);
        self.current_return_type = Some(inner_return_type.clone());
        self.loop_stack.clear();
        let body_entry = self.context.append_basic_block(body_fn, "entry");
        self.builder.position_at_end(body_entry);

        let body_env_raw = body_fn
            .get_nth_param(0)
            .ok_or_else(|| CodegenError::new("async block body missing env param"))?
            .into_pointer_value();
        let body_env = self
            .builder
            .build_pointer_cast(
                body_env_raw,
                self.context.ptr_type(AddressSpace::default()),
                "async_block_body_env",
            )
            .unwrap();

        for (i, (name, ty)) in captures.iter().enumerate() {
            let field_ptr = unsafe {
                self.builder
                    .build_gep(
                        env_type,
                        body_env,
                        &[
                            self.context.i32_type().const_int(0, false),
                            self.context.i32_type().const_int(i as u64, false),
                        ],
                        "async_block_body_field",
                    )
                    .unwrap()
            };
            let loaded = self
                .builder
                .build_load(self.llvm_type(ty), field_ptr, "async_capture_load")
                .unwrap();
            let alloca = self.builder.build_alloca(self.llvm_type(ty), name).unwrap();
            self.builder.build_store(alloca, loaded).unwrap();
            self.variables.insert(
                name.clone(),
                Variable {
                    ptr: alloca,
                    ty: ty.clone(),
                },
            );
        }

        for stmt in body {
            self.compile_stmt(&stmt.node)?;
        }
        if self.needs_terminator() {
            if matches!(inner_return_type, Type::None) {
                self.builder.build_return(None).unwrap();
            } else {
                self.builder.build_unreachable().unwrap();
            }
        }

        let thunk_name = format!("__apex_async_block_thunk_{}", id);
        let thunk_fn_type = ptr_ty.fn_type(&[ptr_ty.into()], false);
        let thunk_fn = self.module.add_function(&thunk_name, thunk_fn_type, None);

        self.current_function = Some(thunk_fn);
        self.current_return_type = None;
        self.variables.clear();
        self.loop_stack.clear();
        let thunk_entry = self.context.append_basic_block(thunk_fn, "entry");
        self.builder.position_at_end(thunk_entry);

        let thunk_env = thunk_fn
            .get_nth_param(0)
            .ok_or_else(|| CodegenError::new("async block thunk missing env param"))?
            .into_pointer_value();
        let thunk_env_cast = self
            .builder
            .build_pointer_cast(thunk_env, ptr_ty, "async_block_thunk_env")
            .unwrap();
        let body_call = self
            .builder
            .build_call(body_fn, &[thunk_env.into()], "async_block_call")
            .unwrap();

        let result_ptr = if matches!(inner_return_type, Type::None) {
            let alloc = self
                .builder
                .build_call(
                    malloc,
                    &[self.context.i64_type().const_int(1, false).into()],
                    "async_block_none_alloc",
                )
                .unwrap()
                .try_as_basic_value();
            let ptr = match alloc {
                ValueKind::Basic(BasicValueEnum::PointerValue(p)) => p,
                _ => {
                    return Err(CodegenError::new(
                        "malloc failed for async block none result",
                    ))
                }
            };
            let none_ptr = self
                .builder
                .build_pointer_cast(
                    ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "async_block_none_ptr",
                )
                .unwrap();
            self.builder
                .build_store(none_ptr, self.context.i8_type().const_int(0, false))
                .unwrap();
            ptr
        } else {
            let ret_ty = self.llvm_type(&inner_return_type);
            let size = ret_ty
                .size_of()
                .ok_or_else(|| CodegenError::new("failed to compute async block result size"))?;
            let alloc = self
                .builder
                .build_call(malloc, &[size.into()], "async_block_alloc")
                .unwrap()
                .try_as_basic_value();
            let ptr = match alloc {
                ValueKind::Basic(BasicValueEnum::PointerValue(p)) => p,
                _ => return Err(CodegenError::new("malloc failed for async block result")),
            };
            let typed_ptr = self
                .builder
                .build_pointer_cast(
                    ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "async_block_result_ptr",
                )
                .unwrap();
            let result_val = match body_call.try_as_basic_value() {
                ValueKind::Basic(v) => v,
                ValueKind::Instruction(_) => {
                    return Err(CodegenError::new(
                        "async block body should return value for non-None Task",
                    ));
                }
            };
            self.builder.build_store(typed_ptr, result_val).unwrap();
            ptr
        };
        let task_field_ptr = unsafe {
            self.builder
                .build_gep(
                    env_type,
                    thunk_env_cast,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context
                            .i32_type()
                            .const_int(captures.len() as u64, false),
                    ],
                    "async_block_task_field",
                )
                .unwrap()
        };
        let task_ptr = self
            .builder
            .build_load(ptr_ty, task_field_ptr, "async_block_task_ptr")
            .unwrap()
            .into_pointer_value();
        let result_field = unsafe {
            self.builder
                .build_gep(
                    self.task_struct_type(),
                    task_ptr,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context.i32_type().const_int(1, false),
                    ],
                    "async_block_result_field",
                )
                .unwrap()
        };
        self.builder.build_store(result_field, result_ptr).unwrap();
        let completed_field = unsafe {
            self.builder
                .build_gep(
                    self.task_struct_type(),
                    task_ptr,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context.i32_type().const_int(3, false),
                    ],
                    "async_block_completed_field",
                )
                .unwrap()
        };
        self.build_atomic_bool_store(
            completed_field,
            self.context.i8_type().const_int(1, false),
            AtomicOrdering::Release,
        )?;
        self.builder.build_return(Some(&result_ptr)).unwrap();

        self.current_function = saved_function;
        self.current_return_type = saved_return_type;
        self.variables = saved_variables;
        self.loop_stack = saved_loop_stack;
        if let Some(block) = saved_insert_block {
            self.builder.position_at_end(block);
        }

        let task = self.create_task(
            thunk_fn.as_global_value().as_pointer_value(),
            env_raw,
            task_slot_ptr,
        )?;
        Ok(task.into())
    }

    pub fn compile_expr(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>> {
        match expr {
            Expr::Literal(lit) => self.compile_literal(lit),

            Expr::Ident(name) => {
                if let Some(var) = self.variables.get(name) {
                    let val = self
                        .builder
                        .build_load(self.llvm_type(&var.ty), var.ptr, name)
                        .unwrap();
                    Ok(val)
                } else if let Some((func, ty)) = self.functions.get(name) {
                    if self.extern_functions.contains(name) {
                        return Err(CodegenError::new(format!(
                            "extern function '{}' cannot be used as a first-class value yet",
                            name
                        )));
                    }
                    // Create a closure struct { fn_ptr, null_env }
                    let struct_ty = self.llvm_type(ty).into_struct_type();
                    let mut closure = struct_ty.get_undef();

                    let fn_ptr = func.as_global_value().as_pointer_value();
                    let null_env = self.context.ptr_type(AddressSpace::default()).const_null();

                    closure = self
                        .builder
                        .build_insert_value(closure, fn_ptr, 0, "fn")
                        .unwrap()
                        .into_struct_value();
                    closure = self
                        .builder
                        .build_insert_value(closure, null_env, 1, "env")
                        .unwrap()
                        .into_struct_value();

                    Ok(closure.into())
                } else {
                    Err(CodegenError::new(format!("Unknown variable: {}", name)))
                }
            }

            Expr::Binary { op, left, right } => self.compile_binary(*op, &left.node, &right.node),

            Expr::Unary { op, expr } => self.compile_unary(*op, &expr.node),

            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                if !type_args.is_empty() {
                    return Err(CodegenError::new(
                        "Explicit generic call code generation is not supported yet".to_string(),
                    ));
                }
                self.compile_call(&callee.node, args)
            }

            Expr::Field { object, field } => self.compile_field(&object.node, field),

            Expr::Index { object, index } => self.compile_index(&object.node, &index.node),

            Expr::Construct { ty, args } => self.compile_construct(ty, args),

            Expr::This => {
                if let Some(var) = self.variables.get("this") {
                    let val = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            var.ptr,
                            "this",
                        )
                        .unwrap();
                    Ok(val)
                } else {
                    Err(CodegenError::new("'this' not available"))
                }
            }

            Expr::StringInterp(parts) => self.compile_string_interp(parts),

            Expr::Lambda { params, body } => self.compile_lambda(params, body),

            Expr::Match { expr, arms } => self.compile_match_expr(&expr.node, arms),

            Expr::Try(inner) => self.compile_try(&inner.node),

            Expr::Borrow(inner) | Expr::MutBorrow(inner) => {
                // Get pointer to the expression (lvalue)
                self.compile_borrow(&inner.node)
            }

            Expr::Deref(inner) => {
                // Dereference the pointer
                self.compile_deref(&inner.node)
            }

            Expr::Await(inner) => {
                let task = self.compile_expr(&inner.node)?;
                let inner_ty = self.infer_await_inner_type(&inner.node);
                if !task.is_pointer_value() {
                    return Err(CodegenError::new("await expects Task<T> value"));
                }
                self.await_task(task.into_pointer_value(), &inner_ty)
            }

            Expr::AsyncBlock(body) => self.compile_async_block(body),

            Expr::Require { condition, message } => {
                // Compile require(condition) as an assert
                let cond_val = self.compile_expr(&condition.node)?;
                let cond = cond_val.into_int_value();

                let current_fn = self
                    .current_function
                    .ok_or(CodegenError::new("require outside of function"))?;

                let assert_block = self.context.append_basic_block(current_fn, "require.ok");
                let fail_block = self.context.append_basic_block(current_fn, "require.fail");

                self.builder
                    .build_conditional_branch(cond, assert_block, fail_block)
                    .unwrap();

                // Fail block - call abort or print message
                self.builder.position_at_end(fail_block);
                if let Some(msg) = message {
                    // Print the error message
                    let msg_spanned = Spanned::new(msg.node.clone(), msg.span.clone());
                    self.compile_print(&[msg_spanned], true)?;
                }
                // Call exit(1) or abort
                if let Some(exit_fn) = self.module.get_function("exit") {
                    self.builder
                        .build_call(
                            exit_fn,
                            &[self.context.i32_type().const_int(1, false).into()],
                            "exit",
                        )
                        .unwrap();
                }
                self.builder.build_unreachable().unwrap();

                // Continue in assert block
                self.builder.position_at_end(assert_block);
                Ok(self.context.i8_type().const_int(0, false).into())
            }

            Expr::Range {
                start,
                end,
                inclusive: _,
            } => {
                // Ranges are handled specially in for loops
                // For now, return a dummy value
                let start_val = if let Some(s) = start {
                    self.compile_expr(&s.node)?
                } else {
                    self.context.i64_type().const_int(0, false).into()
                };
                let _end_val = if let Some(e) = end {
                    self.compile_expr(&e.node)?
                } else {
                    self.context.i64_type().const_int(0, false).into()
                };
                Ok(start_val)
            }

            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => self.compile_if_expr(&condition.node, then_branch, else_branch.as_ref()),

            Expr::Block(body) => {
                let mut result = self.context.i8_type().const_int(0, false).into();
                for stmt in body {
                    if let Stmt::Expr(expr) = &stmt.node {
                        result = self.compile_expr(&expr.node)?;
                    } else {
                        self.compile_stmt(&stmt.node)?;
                    }
                }
                Ok(result)
            }
        }
    }

    pub fn compile_if_expr(
        &mut self,
        condition: &Expr,
        then_branch: &[Spanned<Stmt>],
        else_branch: Option<&Vec<Spanned<Stmt>>>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let cond_val = self.compile_expr(condition)?;
        let cond = cond_val.into_int_value();

        let current_fn = self
            .current_function
            .ok_or(CodegenError::new("if expression outside of function"))?;

        let then_block = self.context.append_basic_block(current_fn, "if.then");
        let else_block = self.context.append_basic_block(current_fn, "if.else");
        let merge_block = self.context.append_basic_block(current_fn, "if.merge");

        self.builder
            .build_conditional_branch(cond, then_block, else_block)
            .unwrap();

        // Then branch
        self.builder.position_at_end(then_block);
        let mut then_result = self.context.i8_type().const_int(0, false).into();
        for stmt in then_branch {
            if let Stmt::Expr(expr) = &stmt.node {
                then_result = self.compile_expr(&expr.node)?;
            } else {
                self.compile_stmt(&stmt.node)?;
            }
        }
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();
        let then_block = self.builder.get_insert_block().unwrap();

        // Else branch
        self.builder.position_at_end(else_block);
        let mut else_result = self.context.i8_type().const_int(0, false).into();
        if let Some(else_stmts) = else_branch {
            for stmt in else_stmts {
                if let Stmt::Expr(expr) = &stmt.node {
                    else_result = self.compile_expr(&expr.node)?;
                } else {
                    self.compile_stmt(&stmt.node)?;
                }
            }
        }
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();
        let else_block = self.builder.get_insert_block().unwrap();

        // Merge block with phi node
        self.builder.position_at_end(merge_block);
        if then_result.get_type() == else_result.get_type() {
            let phi = self
                .builder
                .build_phi(then_result.get_type(), "if.result")
                .unwrap();
            phi.add_incoming(&[(&then_result, then_block), (&else_result, else_block)]);
            Ok(phi.as_basic_value())
        } else {
            Ok(then_result)
        }
    }

    pub fn compile_literal(&mut self, lit: &Literal) -> Result<BasicValueEnum<'ctx>> {
        match lit {
            Literal::Integer(n) => Ok(self.context.i64_type().const_int(*n as u64, true).into()),
            Literal::Float(n) => Ok(self.context.f64_type().const_float(*n).into()),
            Literal::Boolean(b) => Ok(self.context.bool_type().const_int(*b as u64, false).into()),
            Literal::String(s) => {
                let str_val = self.context.const_string(s.as_bytes(), true);
                let name = format!("str.{}", self.str_counter);
                self.str_counter += 1;
                let global = self.module.add_global(str_val.get_type(), None, &name);
                global.set_linkage(Linkage::Private);
                global.set_initializer(&str_val);
                global.set_constant(true);
                Ok(global.as_pointer_value().into())
            }
            Literal::Char(c) => Ok(self.context.i8_type().const_int(*c as u64, false).into()),
            Literal::None => Ok(self.context.i8_type().const_int(0, false).into()),
        }
    }

    pub fn compile_binary(
        &mut self,
        op: BinOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<BasicValueEnum<'ctx>> {
        let lhs = self.compile_expr(left)?;
        let rhs = self.compile_expr(right)?;

        // Integer operations
        if lhs.is_int_value() && rhs.is_int_value() {
            let l = lhs.into_int_value();
            let r = rhs.into_int_value();

            let result = match op {
                BinOp::Add => self.builder.build_int_add(l, r, "add").unwrap(),
                BinOp::Sub => self.builder.build_int_sub(l, r, "sub").unwrap(),
                BinOp::Mul => self.builder.build_int_mul(l, r, "mul").unwrap(),
                BinOp::Div => self.builder.build_int_signed_div(l, r, "div").unwrap(),
                BinOp::Mod => self.builder.build_int_signed_rem(l, r, "mod").unwrap(),
                BinOp::Eq => self
                    .builder
                    .build_int_compare(IntPredicate::EQ, l, r, "eq")
                    .unwrap(),
                BinOp::NotEq => self
                    .builder
                    .build_int_compare(IntPredicate::NE, l, r, "ne")
                    .unwrap(),
                BinOp::Lt => self
                    .builder
                    .build_int_compare(IntPredicate::SLT, l, r, "lt")
                    .unwrap(),
                BinOp::LtEq => self
                    .builder
                    .build_int_compare(IntPredicate::SLE, l, r, "le")
                    .unwrap(),
                BinOp::Gt => self
                    .builder
                    .build_int_compare(IntPredicate::SGT, l, r, "gt")
                    .unwrap(),
                BinOp::GtEq => self
                    .builder
                    .build_int_compare(IntPredicate::SGE, l, r, "ge")
                    .unwrap(),
                BinOp::And => self.builder.build_and(l, r, "and").unwrap(),
                BinOp::Or => self.builder.build_or(l, r, "or").unwrap(),
            };
            return Ok(result.into());
        }

        // Float operations
        if lhs.is_float_value() && rhs.is_float_value() {
            let l = lhs.into_float_value();
            let r = rhs.into_float_value();

            let result = match op {
                BinOp::Add => self.builder.build_float_add(l, r, "fadd").unwrap().into(),
                BinOp::Sub => self.builder.build_float_sub(l, r, "fsub").unwrap().into(),
                BinOp::Mul => self.builder.build_float_mul(l, r, "fmul").unwrap().into(),
                BinOp::Div => self.builder.build_float_div(l, r, "fdiv").unwrap().into(),
                BinOp::Eq => self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, "feq")
                    .unwrap()
                    .into(),
                BinOp::NotEq => self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, l, r, "fne")
                    .unwrap()
                    .into(),
                BinOp::Lt => self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, l, r, "flt")
                    .unwrap()
                    .into(),
                BinOp::LtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, l, r, "fle")
                    .unwrap()
                    .into(),
                BinOp::Gt => self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, l, r, "fgt")
                    .unwrap()
                    .into(),
                BinOp::GtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, l, r, "fge")
                    .unwrap()
                    .into(),
                _ => return Err(CodegenError::new("Invalid float operation")),
            };
            return Ok(result);
        }

        // String concatenation
        if matches!(op, BinOp::Add) && lhs.is_pointer_value() && rhs.is_pointer_value() {
            // Re-use Str__concat logic
            // Since we don't have Spanned<Expr> here easily, we call compile_builtin_call with dummy spans
            let args = vec![
                Spanned::new(left.clone(), Span::default()),
                Spanned::new(right.clone(), Span::default()),
            ];
            return self
                .compile_stdlib_function("Str__concat", &args)
                .map(|v| v.unwrap());
        }

        Err(CodegenError::new("Type mismatch in binary operation"))
    }

    pub fn compile_unary(&mut self, op: UnaryOp, expr: &Expr) -> Result<BasicValueEnum<'ctx>> {
        let val = self.compile_expr(expr)?;

        match op {
            UnaryOp::Neg => {
                if val.is_int_value() {
                    Ok(self
                        .builder
                        .build_int_neg(val.into_int_value(), "neg")
                        .unwrap()
                        .into())
                } else if val.is_float_value() {
                    Ok(self
                        .builder
                        .build_float_neg(val.into_float_value(), "fneg")
                        .unwrap()
                        .into())
                } else {
                    Err(CodegenError::new("Cannot negate non-numeric value"))
                }
            }
            UnaryOp::Not => Ok(self
                .builder
                .build_not(val.into_int_value(), "not")
                .unwrap()
                .into()),
        }
    }

    pub fn compile_call(
        &mut self,
        callee: &Expr,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let resolved_ident = if let Expr::Ident(name) = callee {
            if self.variables.contains_key(name) {
                name.clone()
            } else {
                self.resolve_function_alias(name)
            }
        } else {
            String::new()
        };

        // Check for built-in functions
        if let Expr::Ident(name) = callee {
            let builtin_name = if resolved_ident.is_empty() {
                name.as_str()
            } else {
                resolved_ident.as_str()
            };
            if builtin_name == "println" || builtin_name == "print" {
                return self.compile_print(args, builtin_name == "println");
            }

            // Standard library functions
            if Self::is_stdlib_function(builtin_name) {
                if let Some(result) = self.compile_stdlib_function(builtin_name, args)? {
                    return Ok(result);
                } else {
                    // Void stdlib function - return dummy value
                    return Ok(self.context.i8_type().const_int(0, false).into());
                }
            }
        }

        // Check for Option/Result static methods
        if let Expr::Field { object, field } = callee {
            if let Expr::Ident(type_name) = &object.node {
                match (type_name.as_str(), field.as_str()) {
                    ("Option", "some") => {
                        if args.len() != 1 {
                            return Err(CodegenError::new(
                                "Option.some() requires exactly 1 argument",
                            ));
                        }
                        let val = self.compile_expr(&args[0].node)?;
                        return self.create_option_some(val);
                    }
                    ("Option", "none") => {
                        return self.create_option_none();
                    }
                    ("Result", "ok") => {
                        if args.len() != 1 {
                            return Err(CodegenError::new(
                                "Result.ok() requires exactly 1 argument",
                            ));
                        }
                        let val = self.compile_expr(&args[0].node)?;
                        return self.create_result_ok(val);
                    }
                    ("Result", "error") => {
                        if args.len() != 1 {
                            return Err(CodegenError::new(
                                "Result.error() requires exactly 1 argument",
                            ));
                        }
                        let val = self.compile_expr(&args[0].node)?;
                        return self.create_result_error(val);
                    }
                    _ => {}
                }
            }
        }

        // Check for enum variant constructors and module-qualified functions.
        if let Expr::Field { object, field } = callee {
            if let Expr::Ident(owner_name) = &object.node {
                let resolved_owner = self.resolve_module_alias(owner_name);
                // Enum constructor: `MyEnum.Variant(...)`
                if let Some(enum_info) = self.enums.get(&resolved_owner) {
                    if let Some(variant_info) = enum_info.variants.get(field).cloned() {
                        if args.len() != variant_info.fields.len() {
                            return Err(CodegenError::new(format!(
                                "Enum variant '{}.{}' expects {} argument(s), got {}",
                                resolved_owner,
                                field,
                                variant_info.fields.len(),
                                args.len()
                            )));
                        }
                        let mut values = Vec::with_capacity(args.len());
                        for arg in args {
                            values.push(self.compile_expr(&arg.node)?);
                        }
                        return self.build_enum_value(&resolved_owner, &variant_info, &values);
                    }
                }

                // Module dot syntax: Module.func(...) -> Module__func(...)
                let mangled = format!("{}__{}", resolved_owner, field);
                if let Some((func, _)) = self.functions.get(&mangled).cloned() {
                    let mut compiled_args: Vec<BasicValueEnum> = vec![self
                        .context
                        .ptr_type(AddressSpace::default())
                        .const_null()
                        .into()];
                    for a in args {
                        compiled_args.push(self.compile_expr(&a.node)?);
                    }
                    let args_meta: Vec<BasicMetadataValueEnum> =
                        compiled_args.iter().map(|a| (*a).into()).collect();
                    let call = self.builder.build_call(func, &args_meta, "call").unwrap();
                    return match call.try_as_basic_value() {
                        ValueKind::Basic(val) => Ok(val),
                        ValueKind::Instruction(_) => {
                            Ok(self.context.i8_type().const_int(0, false).into())
                        }
                    };
                }
            }
        }

        // Nested module-style calls: A.X.f(...) -> A__X__f(...)
        if let Some(path_parts) = Self::flatten_field_chain(callee) {
            if path_parts.len() >= 3 {
                let candidate = path_parts.join("__");
                if let Some((func, _)) = self.functions.get(&candidate).cloned() {
                    let mut compiled_args: Vec<BasicValueEnum> = vec![self
                        .context
                        .ptr_type(AddressSpace::default())
                        .const_null()
                        .into()];
                    for a in args {
                        compiled_args.push(self.compile_expr(&a.node)?);
                    }
                    let args_meta: Vec<BasicMetadataValueEnum> =
                        compiled_args.iter().map(|a| (*a).into()).collect();
                    let call = self.builder.build_call(func, &args_meta, "call").unwrap();
                    return match call.try_as_basic_value() {
                        ValueKind::Basic(val) => Ok(val),
                        ValueKind::Instruction(_) => {
                            Ok(self.context.i8_type().const_int(0, false).into())
                        }
                    };
                }
            }
        }

        // Method call on object
        if let Expr::Field { object, field } = callee {
            // Check for File static methods
            if let Expr::Ident(name) = &object.node {
                let resolved_name = self.resolve_module_alias(name);
                if matches!(
                    resolved_name.as_str(),
                    "File" | "Time" | "System" | "Math" | "Str" | "Args"
                ) {
                    let builtin_name = format!("{}__{}", resolved_name, field);
                    if let Some(result) = self.compile_stdlib_function(&builtin_name, args)? {
                        return Ok(result);
                    }
                }
                if resolved_name == "io" {
                    if field == "println" || field == "print" {
                        return self.compile_print(args, field == "println");
                    }
                    if let Some(result) = self.compile_stdlib_function(field, args)? {
                        return Ok(result);
                    }
                }
            }
            return self.compile_method_call(&object.node, field, args);
        }

        // Regular function call
        let callee_name = if let Expr::Ident(name) = callee {
            if resolved_ident.is_empty() {
                Some(name.clone())
            } else {
                Some(resolved_ident.clone())
            }
        } else {
            None
        };
        let func = match callee {
            Expr::Ident(name) => {
                // First check if it's a function pointer/local variable
                if let Some(var) = self.variables.get(name) {
                    if let Type::Function(param_types, ret_type) = &var.ty {
                        let closure_val = self
                            .builder
                            .build_load(self.llvm_type(&var.ty), var.ptr, name)
                            .unwrap()
                            .into_struct_value();

                        let ptr = self
                            .builder
                            .build_extract_value(closure_val, 0, "fn_ptr")
                            .unwrap()
                            .into_pointer_value();
                        let env_ptr = self
                            .builder
                            .build_extract_value(closure_val, 1, "env_ptr")
                            .unwrap();

                        // Construct FunctionType (including env_ptr as first arg)
                        let llvm_ret = self.llvm_type(ret_type);
                        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![
                            self.context.ptr_type(AddressSpace::default()).into(), // env_ptr
                        ];
                        for p in param_types {
                            llvm_params.push(self.llvm_type(p).into());
                        }

                        let fn_type = match llvm_ret {
                            BasicTypeEnum::IntType(i) => i.fn_type(&llvm_params, false),
                            BasicTypeEnum::FloatType(f) => f.fn_type(&llvm_params, false),
                            BasicTypeEnum::PointerType(p) => p.fn_type(&llvm_params, false),
                            BasicTypeEnum::StructType(s) => s.fn_type(&llvm_params, false),
                            _ => {
                                // Default to i8 type for void-like returns if needed
                                self.context.i8_type().fn_type(&llvm_params, false)
                            }
                        };

                        let mut compiled_args: Vec<BasicValueEnum> = vec![env_ptr];
                        for a in args {
                            compiled_args.push(self.compile_expr(&a.node)?);
                        }

                        let args_meta: Vec<BasicMetadataValueEnum> =
                            compiled_args.iter().map(|a| (*a).into()).collect();

                        let call = self
                            .builder
                            .build_indirect_call(fn_type, ptr, &args_meta, "call")
                            .unwrap();

                        let result = match call.try_as_basic_value() {
                            ValueKind::Basic(val) => val,
                            ValueKind::Instruction(_) => {
                                self.context.i8_type().const_int(0, false).into()
                            }
                        };
                        return Ok(result);
                    }
                }

                let looked_up_name = if resolved_ident.is_empty() {
                    name
                } else {
                    resolved_ident.as_str()
                };

                // Fall back to global function lookup
                if let Some((f, _)) = self.functions.get(looked_up_name) {
                    *f
                } else if let Some(f) = self.module.get_function(looked_up_name) {
                    f
                } else {
                    return Err(CodegenError::new(format!(
                        "Unknown function: {}",
                        looked_up_name
                    )));
                }
            }
            _ => return Err(CodegenError::new("Invalid callee")),
        };

        let mut compiled_args: Vec<BasicValueEnum> = Vec::new();
        let func_name = func.get_name().to_str().unwrap_or_default().to_string();
        let is_extern_call = callee_name
            .as_deref()
            .map(|n| self.extern_functions.contains(n))
            .unwrap_or(false);
        // Add null env_ptr for direct Apex calls (except main / extern C ABI)
        if func_name != "main" && !is_extern_call {
            compiled_args.push(
                self.context
                    .ptr_type(AddressSpace::default())
                    .const_null()
                    .into(),
            );
        }

        for a in args {
            compiled_args.push(self.compile_expr(&a.node)?);
        }

        let args_meta: Vec<BasicMetadataValueEnum> =
            compiled_args.iter().map(|a| (*a).into()).collect();
        let call = self.builder.build_call(func, &args_meta, "call").unwrap();

        // Tail Call Optimization - mark as tail call
        call.set_tail_call(true);

        match call.try_as_basic_value() {
            ValueKind::Basic(val) => Ok(val),
            ValueKind::Instruction(_) => Ok(self.context.i8_type().const_int(0, false).into()),
        }
    }

    pub fn compile_method_call(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        // Infer object type first
        let obj_ty = self.infer_object_type(object);

        // Handle built-in types (List, Map, Set, Option, Result) for any expression
        if let Some(ref ty) = obj_ty {
            match ty {
                Type::List(_) => {
                    // Get pointer to the list
                    let list_ptr = match object {
                        Expr::Ident(name) => self.variables.get(name).map(|v| v.ptr),
                        Expr::Field { object: obj, field } => {
                            self.compile_field_ptr(&obj.node, field).ok()
                        }
                        Expr::This => self.variables.get("this").map(|v| v.ptr),
                        _ => None,
                    };
                    if let Some(ptr) = list_ptr {
                        return self.compile_list_method_ptr(ptr, ty, method, args);
                    }
                }
                Type::Map(_, _) => {
                    if let Expr::Ident(name) = object {
                        return self.compile_map_method(name, method, args);
                    }
                }
                Type::Set(_) => {
                    if let Expr::Ident(name) = object {
                        return self.compile_set_method(name, method, args);
                    }
                }
                Type::Option(_) => {
                    if let Expr::Ident(name) = object {
                        return self.compile_option_method(name, method, args);
                    }
                }
                Type::Result(_, _) => {
                    if let Expr::Ident(name) = object {
                        return self.compile_result_method(name, method, args);
                    }
                }
                Type::Range(_) => {
                    if let Expr::Ident(name) = object {
                        return self.compile_range_method(name, method, args);
                    }
                }
                Type::Task(inner) => {
                    return self.compile_task_method(object, inner, method, args);
                }
                _ => {}
            }
        }

        let obj_val = self.compile_expr(object)?;

        // Get class name from inferred type
        let class_name = obj_ty
            .as_ref()
            .and_then(|ty| self.type_to_class_name(ty))
            .ok_or_else(|| {
                CodegenError::new(format!(
                    "Cannot determine object type for method call: {:?}",
                    object
                ))
            })?;

        let (func, _) = if self.classes.contains_key(&class_name) {
            let func_name = self
                .resolve_method_function_name(&class_name, method)
                .ok_or_else(|| {
                    CodegenError::new(format!(
                        "Unknown method '{}' for class '{}'",
                        method, class_name
                    ))
                })?;
            self.functions
                .get(&func_name)
                .ok_or_else(|| CodegenError::new(format!("Unknown method: {}", func_name)))?
                .clone()
        } else {
            // Interface-typed object (or unknown Named type): no vtable yet.
            // We allow codegen only when there is a single unambiguous method implementation.
            let suffix = format!("__{}", method);
            let mut candidates = self
                .functions
                .iter()
                .filter_map(|(name, sig)| name.ends_with(&suffix).then_some((name.clone(), sig.0)))
                .collect::<Vec<_>>();
            if candidates.len() == 1 {
                let (_, func) = candidates.pop().unwrap();
                (func, Type::None)
            } else if candidates.is_empty() {
                return Err(CodegenError::new(format!(
                    "Unknown interface method implementation: {}",
                    method
                )));
            } else {
                return Err(CodegenError::new(format!(
                    "Ambiguous interface dispatch for method '{}': {} candidates",
                    method,
                    candidates.len()
                )));
            }
        };

        let mut compiled_args: Vec<BasicValueEnum> = vec![
            self.context
                .ptr_type(AddressSpace::default())
                .const_null()
                .into(), // env_ptr
            obj_val, // this
        ];
        for a in args {
            compiled_args.push(self.compile_expr(&a.node)?);
        }

        let args_meta: Vec<BasicMetadataValueEnum> =
            compiled_args.iter().map(|a| (*a).into()).collect();
        let call = self.builder.build_call(func, &args_meta, "call").unwrap();

        match call.try_as_basic_value() {
            ValueKind::Basic(val) => Ok(val),
            ValueKind::Instruction(_) => Ok(self.context.i8_type().const_int(0, false).into()),
        }
    }

    fn compile_task_method(
        &mut self,
        object: &Expr,
        inner: &Type,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let expected_args = match method {
            "await_timeout" => 1,
            "is_done" | "cancel" => 0,
            _ => 0,
        };
        if args.len() != expected_args {
            return Err(CodegenError::new(format!(
                "Task.{}() expects {} argument(s), got {}",
                method,
                expected_args,
                args.len()
            )));
        }

        let task_ty = self.task_struct_type();
        let task_raw = self.compile_expr(object)?;
        if !task_raw.is_pointer_value() {
            return Err(CodegenError::new("Task method call on non-task value"));
        }
        let task_ptr = self
            .builder
            .build_pointer_cast(
                task_raw.into_pointer_value(),
                self.context.ptr_type(AddressSpace::default()),
                "task_method_ptr",
            )
            .unwrap();

        let i32_ty = self.context.i32_type();
        let zero = i32_ty.const_int(0, false);
        let thread_idx = i32_ty.const_int(0, false);
        let result_idx = i32_ty.const_int(1, false);
        let done_idx = i32_ty.const_int(2, false);
        let completed_idx = i32_ty.const_int(3, false);
        let done_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done_ptr")
                .unwrap()
        };
        let completed_field = unsafe {
            self.builder
                .build_gep(
                    task_ty,
                    task_ptr,
                    &[zero, completed_idx],
                    "task_completed_ptr",
                )
                .unwrap()
        };
        let result_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, result_idx], "task_result_field")
                .unwrap()
        };

        match method {
            "is_done" => {
                let done_val = self
                    .builder
                    .build_load(self.context.i8_type(), done_field, "task_done")
                    .unwrap()
                    .into_int_value();
                let done_bool = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        done_val,
                        self.context.i8_type().const_zero(),
                        "task_done_bool",
                    )
                    .unwrap();
                let completed_val = self.build_atomic_bool_load(
                    completed_field,
                    "task_completed",
                    AtomicOrdering::Acquire,
                )?;
                Ok(self
                    .builder
                    .build_or(done_bool, completed_val, "task_is_done")
                    .unwrap()
                    .into())
            }
            "cancel" => {
                let done_val = self
                    .builder
                    .build_load(self.context.i8_type(), done_field, "task_done")
                    .unwrap()
                    .into_int_value();
                let done_bool = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        done_val,
                        self.context.i8_type().const_zero(),
                        "task_done_bool",
                    )
                    .unwrap();
                let completed_val = self.build_atomic_bool_load(
                    completed_field,
                    "task_completed",
                    AtomicOrdering::Acquire,
                )?;
                let already_done = self
                    .builder
                    .build_or(done_bool, completed_val, "task_already_done")
                    .unwrap();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Task.cancel used outside function"))?;
                let cancel_bb = self.context.append_basic_block(current_fn, "task_cancel");
                let merge_bb = self
                    .context
                    .append_basic_block(current_fn, "task_cancel_merge");
                self.builder
                    .build_conditional_branch(already_done, merge_bb, cancel_bb)
                    .unwrap();

                self.builder.position_at_end(cancel_bb);
                let thread_field = unsafe {
                    self.builder
                        .build_gep(task_ty, task_ptr, &[zero, thread_idx], "task_thread_ptr")
                        .unwrap()
                };
                let thread_id = self
                    .builder
                    .build_load(self.context.i64_type(), thread_field, "task_thread_id")
                    .unwrap();

                let pthread_cancel = self.get_or_declare_pthread_cancel();
                self.builder
                    .build_call(pthread_cancel, &[thread_id.into()], "task_cancel")
                    .unwrap();

                let done_field = unsafe {
                    self.builder
                        .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done_ptr")
                        .unwrap()
                };
                self.builder
                    .build_store(done_field, self.context.i8_type().const_int(1, false))
                    .unwrap();
                self.build_atomic_bool_store(
                    completed_field,
                    self.context.i8_type().const_int(1, false),
                    AtomicOrdering::Release,
                )?;

                // Store default zero value so await after cancel doesn't dereference null.
                let malloc = self.get_or_declare_malloc();
                let result_ptr = if matches!(inner, Type::None) {
                    let raw = self
                        .builder
                        .build_call(
                            malloc,
                            &[self.context.i64_type().const_int(1, false).into()],
                            "task_cancel_none_alloc",
                        )
                        .unwrap()
                        .try_as_basic_value();
                    let ptr = match raw {
                        ValueKind::Basic(BasicValueEnum::PointerValue(p)) => p,
                        _ => {
                            return Err(CodegenError::new(
                                "malloc failed while creating canceled task value",
                            ));
                        }
                    };
                    let typed = self
                        .builder
                        .build_pointer_cast(
                            ptr,
                            self.context.ptr_type(AddressSpace::default()),
                            "task_cancel_none_ptr",
                        )
                        .unwrap();
                    self.builder
                        .build_store(typed, self.context.i8_type().const_int(0, false))
                        .unwrap();
                    ptr
                } else {
                    let llvm_inner = self.llvm_type(inner);
                    let size = llvm_inner
                        .size_of()
                        .ok_or_else(|| CodegenError::new("failed to size Task inner type"))?;
                    let raw = self
                        .builder
                        .build_call(malloc, &[size.into()], "task_cancel_alloc")
                        .unwrap()
                        .try_as_basic_value();
                    let ptr = match raw {
                        ValueKind::Basic(BasicValueEnum::PointerValue(p)) => p,
                        _ => {
                            return Err(CodegenError::new(
                                "malloc failed while creating canceled task value",
                            ));
                        }
                    };
                    let typed_ptr = self
                        .builder
                        .build_pointer_cast(
                            ptr,
                            self.context.ptr_type(AddressSpace::default()),
                            "task_cancel_result_ptr",
                        )
                        .unwrap();

                    let zero_value: BasicValueEnum = match llvm_inner {
                        BasicTypeEnum::IntType(t) => t.const_zero().into(),
                        BasicTypeEnum::FloatType(t) => t.const_float(0.0).into(),
                        BasicTypeEnum::PointerType(t) => t.const_null().into(),
                        BasicTypeEnum::StructType(t) => t.const_zero().into(),
                        _ => self.context.i8_type().const_int(0, false).into(),
                    };
                    self.builder.build_store(typed_ptr, zero_value).unwrap();
                    ptr
                };
                self.builder.build_store(result_field, result_ptr).unwrap();
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(merge_bb);
                Ok(self.context.i8_type().const_int(0, false).into())
            }
            "await_timeout" => {
                let ms = self.compile_expr(&args[0].node)?;
                if !ms.is_int_value() {
                    return Err(CodegenError::new(
                        "Task.await_timeout(ms) requires Integer milliseconds",
                    ));
                }
                let ms_i64 = self
                    .builder
                    .build_int_cast(ms.into_int_value(), self.context.i64_type(), "timeout_ms")
                    .unwrap();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Task.await_timeout used outside function"))?;

                let done_field = unsafe {
                    self.builder
                        .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done_ptr")
                        .unwrap()
                };
                let completed_field = unsafe {
                    self.builder
                        .build_gep(
                            task_ty,
                            task_ptr,
                            &[zero, completed_idx],
                            "task_completed_ptr",
                        )
                        .unwrap()
                };
                let done_val = self
                    .builder
                    .build_load(self.context.i8_type(), done_field, "task_done")
                    .unwrap()
                    .into_int_value();
                let done_ready = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        done_val,
                        self.context.i8_type().const_zero(),
                        "task_done_ready",
                    )
                    .unwrap();

                let done_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_done");
                let check_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_check");
                let join_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_join");
                let loop_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_loop");
                let sleep_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_sleep");
                let timeout_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_fail");
                let merge_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_merge");
                let thread_field = unsafe {
                    self.builder
                        .build_gep(task_ty, task_ptr, &[zero, thread_idx], "task_thread_ptr")
                        .unwrap()
                };
                let thread_id = self
                    .builder
                    .build_load(self.context.i64_type(), thread_field, "task_thread_id")
                    .unwrap();
                let join_result_ptr = self
                    .builder
                    .build_alloca(
                        self.context.ptr_type(AddressSpace::default()),
                        "timed_join_out",
                    )
                    .unwrap();
                self.builder
                    .build_store(
                        join_result_ptr,
                        self.context.ptr_type(AddressSpace::default()).const_null(),
                    )
                    .unwrap();
                let iter_ptr = self
                    .builder
                    .build_alloca(self.context.i64_type(), "task_timeout_iter")
                    .unwrap();
                self.builder
                    .build_store(iter_ptr, self.context.i64_type().const_zero())
                    .unwrap();
                let max_iters = self
                    .builder
                    .build_int_add(
                        ms_i64,
                        self.context.i64_type().const_int(1, false),
                        "task_timeout_iters",
                    )
                    .unwrap();

                self.builder
                    .build_conditional_branch(done_ready, done_bb, check_bb)
                    .unwrap();

                // done -> Some(result)
                self.builder.position_at_end(done_bb);
                let existing_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        result_field,
                        "task_existing_result",
                    )
                    .unwrap()
                    .into_pointer_value();
                let done_value: BasicValueEnum = if matches!(inner, Type::None) {
                    self.context.i8_type().const_int(0, false).into()
                } else {
                    let inner_llvm = self.llvm_type(inner);
                    let typed_ptr = self
                        .builder
                        .build_pointer_cast(
                            existing_ptr,
                            self.context.ptr_type(AddressSpace::default()),
                            "task_done_typed_ptr",
                        )
                        .unwrap();
                    self.builder
                        .build_load(inner_llvm, typed_ptr, "task_done_value")
                        .unwrap()
                };
                let done_some = self.create_option_some(done_value)?;
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(check_bb);
                let completed_val = self.build_atomic_bool_load(
                    completed_field,
                    "task_completed",
                    AtomicOrdering::Acquire,
                )?;
                self.builder
                    .build_conditional_branch(completed_val, join_bb, loop_bb)
                    .unwrap();

                self.builder.position_at_end(loop_bb);
                let iter_val = self
                    .builder
                    .build_load(self.context.i64_type(), iter_ptr, "task_timeout_iter_val")
                    .unwrap()
                    .into_int_value();
                let timed_out = self
                    .builder
                    .build_int_compare(
                        IntPredicate::UGE,
                        iter_val,
                        max_iters,
                        "task_timeout_reached",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(timed_out, timeout_bb, sleep_bb)
                    .unwrap();

                self.builder.position_at_end(sleep_bb);
                #[cfg(windows)]
                {
                    let sleep_fn = self.get_or_declare_sleep_win();
                    self.builder
                        .build_call(
                            sleep_fn,
                            &[self.context.i32_type().const_int(1, false).into()],
                            "task_timeout_sleep_call",
                        )
                        .unwrap();
                }
                #[cfg(not(windows))]
                {
                    let usleep_fn = self.get_or_declare_usleep();
                    self.builder
                        .build_call(
                            usleep_fn,
                            &[self.context.i32_type().const_int(1000, false).into()],
                            "task_timeout_usleep_call",
                        )
                        .unwrap();
                }
                let next_iter = self
                    .builder
                    .build_int_add(
                        iter_val,
                        self.context.i64_type().const_int(1, false),
                        "task_timeout_next_iter",
                    )
                    .unwrap();
                self.builder.build_store(iter_ptr, next_iter).unwrap();
                self.builder.build_unconditional_branch(check_bb).unwrap();

                self.builder.position_at_end(join_bb);
                let pthread_join = self.get_or_declare_pthread_join();
                let _join_status = self
                    .builder
                    .build_call(
                        pthread_join,
                        &[thread_id.into(), join_result_ptr.into()],
                        "timed_join_finalize",
                    )
                    .unwrap();
                let joined_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        join_result_ptr,
                        "joined_result",
                    )
                    .unwrap()
                    .into_pointer_value();
                self.builder.build_store(result_field, joined_ptr).unwrap();
                self.builder
                    .build_store(done_field, self.context.i8_type().const_int(1, false))
                    .unwrap();
                self.build_atomic_bool_store(
                    completed_field,
                    self.context.i8_type().const_int(1, false),
                    AtomicOrdering::Release,
                )?;
                let succ_value: BasicValueEnum = if matches!(inner, Type::None) {
                    self.context.i8_type().const_int(0, false).into()
                } else {
                    let inner_llvm = self.llvm_type(inner);
                    let typed_ptr = self
                        .builder
                        .build_pointer_cast(
                            joined_ptr,
                            self.context.ptr_type(AddressSpace::default()),
                            "joined_typed_ptr",
                        )
                        .unwrap();
                    self.builder
                        .build_load(inner_llvm, typed_ptr, "joined_value")
                        .unwrap()
                };
                let succ_some = self.create_option_some(succ_value)?;
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                // timeout -> None
                self.builder.position_at_end(timeout_bb);
                let option_ty = self.context.struct_type(
                    &[self.context.i8_type().into(), self.llvm_type(inner)],
                    false,
                );
                let timeout_none: BasicValueEnum<'ctx> = option_ty
                    .const_named_struct(&[
                        self.context.i8_type().const_int(0, false).into(),
                        match self.llvm_type(inner) {
                            BasicTypeEnum::IntType(t) => t.const_zero().into(),
                            BasicTypeEnum::FloatType(t) => t.const_float(0.0).into(),
                            BasicTypeEnum::PointerType(t) => t.const_null().into(),
                            BasicTypeEnum::StructType(t) => t.const_zero().into(),
                            _ => self.context.i8_type().const_int(0, false).into(),
                        },
                    ])
                    .into();
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(
                        self.llvm_type(&Type::Option(Box::new(inner.clone()))),
                        "timeout_phi",
                    )
                    .unwrap();
                phi.add_incoming(&[
                    (&done_some, done_bb),
                    (&succ_some, join_bb),
                    (&timeout_none, timeout_bb),
                ]);
                Ok(phi.as_basic_value())
            }
            _ => Err(CodegenError::new(format!(
                "Unknown Task method: {}",
                method
            ))),
        }
    }

    pub fn compile_field(&mut self, object: &Expr, field: &str) -> Result<BasicValueEnum<'ctx>> {
        let obj_ptr = self.compile_expr(object)?.into_pointer_value();

        // Get class name using type inference
        let obj_ty = self.infer_object_type(object);
        let class_name = obj_ty
            .as_ref()
            .and_then(|ty| self.type_to_class_name(ty))
            .ok_or_else(|| {
                CodegenError::new(format!(
                    "Cannot determine object type for field access: {:?}.{}",
                    object, field
                ))
            })?;

        let class_info = self
            .classes
            .get(&class_name)
            .ok_or_else(|| CodegenError::new(format!("Unknown class: {}", class_name)))?;

        let field_idx = *class_info
            .field_indices
            .get(field)
            .ok_or_else(|| CodegenError::new(format!("Unknown field: {}", field)))?;

        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);
        let idx = i32_type.const_int(field_idx as u64, false);

        let field_ptr = unsafe {
            self.builder
                .build_gep(
                    class_info.struct_type.as_basic_type_enum(),
                    obj_ptr,
                    &[zero, idx],
                    field,
                )
                .unwrap()
        };

        let field_type = class_info
            .struct_type
            .get_field_type_at_index(field_idx)
            .unwrap();
        Ok(self
            .builder
            .build_load(field_type, field_ptr, field)
            .unwrap())
    }

    /// Get pointer to a field (for in-place modifications on collections)
    pub fn compile_field_ptr(&mut self, object: &Expr, field: &str) -> Result<PointerValue<'ctx>> {
        let obj_ptr = self.compile_expr(object)?.into_pointer_value();

        let obj_ty = self.infer_object_type(object);
        let class_name = obj_ty
            .as_ref()
            .and_then(|ty| self.type_to_class_name(ty))
            .ok_or_else(|| CodegenError::new("Cannot determine object type for field ptr"))?;

        let class_info = self
            .classes
            .get(&class_name)
            .ok_or_else(|| CodegenError::new(format!("Unknown class: {}", class_name)))?;

        let field_idx = *class_info
            .field_indices
            .get(field)
            .ok_or_else(|| CodegenError::new(format!("Unknown field: {}", field)))?;

        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);
        let idx = i32_type.const_int(field_idx as u64, false);

        let field_ptr = unsafe {
            self.builder
                .build_gep(
                    class_info.struct_type.as_basic_type_enum(),
                    obj_ptr,
                    &[zero, idx],
                    field,
                )
                .unwrap()
        };

        Ok(field_ptr)
    }

    pub fn compile_index(&mut self, object: &Expr, index: &Expr) -> Result<BasicValueEnum<'ctx>> {
        let obj_val = self.compile_expr(object)?;
        let idx = self.compile_expr(index)?.into_int_value();

        // List indexing may come either as:
        // 1) direct data pointer, or
        // 2) materialized list struct value {capacity, length, data_ptr}.
        if let BasicValueEnum::StructValue(list_struct) = obj_val {
            let data_ptr = self
                .builder
                .build_extract_value(list_struct, 2, "list_data")
                .map_err(|_| CodegenError::new("Invalid list value for index access"))?
                .into_pointer_value();
            let elem_ty = match self.infer_object_type(object) {
                Some(Type::List(inner)) if matches!(*inner, Type::Boolean) => {
                    self.context.bool_type().as_basic_type_enum()
                }
                _ => self.context.i64_type().as_basic_type_enum(),
            };
            let typed_data_ptr = self
                .builder
                .build_pointer_cast(
                    data_ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "list_typed_data",
                )
                .unwrap();
            let elem_ptr = unsafe {
                self.builder
                    .build_gep(elem_ty, typed_data_ptr, &[idx], "elem")
                    .unwrap()
            };
            return Ok(self.builder.build_load(elem_ty, elem_ptr, "load").unwrap());
        }

        let obj_ptr = obj_val.into_pointer_value();
        let elem_ty = match self.infer_object_type(object) {
            Some(Type::List(inner)) if matches!(*inner, Type::Boolean) => {
                self.context.bool_type().as_basic_type_enum()
            }
            _ => self.context.i64_type().as_basic_type_enum(),
        };
        let elem_ptr = unsafe {
            self.builder
                .build_gep(elem_ty, obj_ptr, &[idx], "elem")
                .unwrap()
        };
        Ok(self.builder.build_load(elem_ty, elem_ptr, "load").unwrap())
    }

    pub fn compile_construct(
        &mut self,
        ty: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        // Handle List<T> construction
        if ty == "List" || ty.starts_with("List<") {
            let list_ty = if ty == "List<Boolean>" {
                Some(Type::List(Box::new(Type::Boolean)))
            } else {
                None
            };
            if args.len() == 1 {
                if let Expr::Literal(Literal::Integer(size)) = &args[0].node {
                    if *size > 0 {
                        return self.create_fixed_list(*size as u64, list_ty.as_ref());
                    }
                }
            }
            return self.create_empty_list(list_ty.as_ref());
        }

        // Handle Map<K,V> construction
        if ty == "Map" || ty.starts_with("Map<") {
            return self.create_empty_map();
        }

        // Handle Option<T> construction (default to None)
        if ty == "Option" || ty.starts_with("Option<") {
            return self.create_option_none();
        }

        // Handle Result<T,E> construction (default to Error with zeroed memory)
        if ty == "Result" || ty.starts_with("Result<") {
            return self.create_default_result();
        }

        // Handle Set<T> construction
        if ty == "Set" || ty.starts_with("Set<") {
            return self.create_empty_set();
        }

        // Handle Smart Pointer construction
        if ty == "Box" || ty.starts_with("Box<") {
            return self.create_empty_box();
        }
        if ty == "Rc" || ty.starts_with("Rc<") {
            return self.create_empty_rc();
        }
        if ty == "Arc" || ty.starts_with("Arc<") {
            return self.create_empty_arc();
        }

        let func_name = format!("{}__new", ty);

        let (func, _) = self
            .functions
            .get(&func_name)
            .ok_or_else(|| CodegenError::new(format!("Unknown type: {}", ty)))?
            .clone();

        let mut compiled_args: Vec<BasicValueEnum> = vec![
            self.context
                .ptr_type(AddressSpace::default())
                .const_null()
                .into(), // env_ptr
        ];
        for a in args {
            compiled_args.push(self.compile_expr(&a.node)?);
        }

        let args_meta: Vec<BasicMetadataValueEnum> =
            compiled_args.iter().map(|a| (*a).into()).collect();
        let call = self.builder.build_call(func, &args_meta, "new").unwrap();

        match call.try_as_basic_value() {
            ValueKind::Basic(val) => Ok(val),
            _ => panic!("Constructor should return a value"),
        }
    }

    pub fn compile_print(
        &mut self,
        args: &[Spanned<Expr>],
        newline: bool,
    ) -> Result<BasicValueEnum<'ctx>> {
        let printf = self.get_or_declare_printf();

        for arg in args {
            let val = self.compile_expr(&arg.node)?;

            let (fmt, print_args): (&str, Vec<BasicMetadataValueEnum>) = if val.is_int_value() {
                ("%lld", vec![val.into()])
            } else if val.is_float_value() {
                ("%f", vec![val.into()])
            } else {
                ("%s", vec![val.into()])
            };

            let fmt_str = self.context.const_string(fmt.as_bytes(), true);
            let fmt_name = format!("fmt.{}", self.str_counter);
            self.str_counter += 1;
            let fmt_global = self.module.add_global(fmt_str.get_type(), None, &fmt_name);
            fmt_global.set_linkage(Linkage::Private);
            fmt_global.set_initializer(&fmt_str);

            let mut call_args: Vec<BasicMetadataValueEnum> =
                vec![fmt_global.as_pointer_value().into()];
            call_args.extend(print_args);

            self.builder
                .build_call(printf, &call_args, "printf")
                .unwrap();
        }

        if newline {
            let nl_str = self.context.const_string(b"\n", true);
            let nl_name = format!("nl.{}", self.str_counter);
            self.str_counter += 1;
            let nl_global = self.module.add_global(nl_str.get_type(), None, &nl_name);
            nl_global.set_linkage(Linkage::Private);
            nl_global.set_initializer(&nl_str);
            self.builder
                .build_call(printf, &[nl_global.as_pointer_value().into()], "printf")
                .unwrap();
        }

        Ok(self.context.i32_type().const_int(0, false).into())
    }

    pub fn compile_string_interp(&mut self, parts: &[StringPart]) -> Result<BasicValueEnum<'ctx>> {
        // Build format string and collect arguments
        let mut fmt_str = String::new();
        let mut args: Vec<BasicMetadataValueEnum> = Vec::new();

        for part in parts {
            match part {
                StringPart::Literal(s) => {
                    // Escape % characters for printf
                    fmt_str.push_str(&s.replace('%', "%%"));
                }
                StringPart::Expr(expr) => {
                    let val = self.compile_expr(&expr.node)?;
                    if val.is_int_value() {
                        fmt_str.push_str("%lld");
                        args.push(val.into());
                    } else if val.is_float_value() {
                        fmt_str.push_str("%f");
                        args.push(val.into());
                    } else {
                        fmt_str.push_str("%s");
                        args.push(val.into());
                    }
                }
            }
        }

        // Allocate buffer for result (simplified: fixed size)
        let sprintf = self.get_or_declare_sprintf();
        let malloc = self.get_or_declare_malloc();

        let buffer_size = self.context.i64_type().const_int(4096, false);
        let buffer_call = self
            .builder
            .build_call(malloc, &[buffer_size.into()], "strbuf")
            .unwrap();
        let buffer = match buffer_call.try_as_basic_value() {
            ValueKind::Basic(val) => val.into_pointer_value(),
            _ => panic!("malloc should return a value"),
        };

        // Create format string
        let fmt_val = self.context.const_string(fmt_str.as_bytes(), true);
        let fmt_name = format!("fmt.{}", self.str_counter);
        self.str_counter += 1;
        let fmt_global = self.module.add_global(fmt_val.get_type(), None, &fmt_name);
        fmt_global.set_linkage(Linkage::Private);
        fmt_global.set_initializer(&fmt_val);

        // Call sprintf
        let mut sprintf_args: Vec<BasicMetadataValueEnum> =
            vec![buffer.into(), fmt_global.as_pointer_value().into()];
        sprintf_args.extend(args);
        self.builder
            .build_call(sprintf, &sprintf_args, "sprintf")
            .unwrap();

        Ok(buffer.into())
    }

    pub fn compile_try(&mut self, inner: &Expr) -> Result<BasicValueEnum<'ctx>> {
        // Get current function and return type
        let function = self
            .current_function
            .ok_or_else(|| CodegenError::new("? operator used outside function"))?;
        let return_type = self
            .current_return_type
            .clone()
            .ok_or_else(|| CodegenError::new("? operator used outside function"))?;

        // Compile the inner expression (should be Option<T> or Result<T, E>)
        let value = self.compile_expr(inner)?;
        let struct_val = value.into_struct_value();

        // Extract the tag (field 0): 0 = None/Error, 1 = Some/Ok
        let tag = self
            .builder
            .build_extract_value(struct_val, 0, "tag")
            .unwrap();
        let tag_int = tag.into_int_value();

        // Compare tag with 1 (Some/Ok)
        let is_some_or_ok = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                tag_int,
                self.context.i8_type().const_int(1, false),
                "is_some_or_ok",
            )
            .unwrap();

        // Create basic blocks
        let success_block = self.context.append_basic_block(function, "try.success");
        let error_block = self.context.append_basic_block(function, "try.error");
        let merge_block = self.context.append_basic_block(function, "try.merge");

        // Branch based on tag
        self.builder
            .build_conditional_branch(is_some_or_ok, success_block, error_block)
            .unwrap();

        // Error block: return early with None/Error
        self.builder.position_at_end(error_block);
        match &return_type {
            Type::Option(inner_ty) => {
                // Return None - create Option with tag = 0
                let inner_llvm = self.llvm_type(inner_ty);
                let option_type = self
                    .context
                    .struct_type(&[self.context.i8_type().into(), inner_llvm], false);
                let alloca = self.builder.build_alloca(option_type, "none_ret").unwrap();
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let tag_ptr = unsafe {
                    self.builder
                        .build_gep(
                            option_type.as_basic_type_enum(),
                            alloca,
                            &[zero, i32_type.const_int(0, false)],
                            "tag",
                        )
                        .unwrap()
                };
                self.builder
                    .build_store(tag_ptr, self.context.i8_type().const_int(0, false))
                    .unwrap();
                let loaded = self.builder.build_load(option_type, alloca, "ret").unwrap();
                self.builder.build_return(Some(&loaded)).unwrap();
            }
            Type::Result(ok_ty, err_ty) => {
                // Return Error - propagate the error from the inner Result
                let ok_llvm = self.llvm_type(ok_ty);
                let err_llvm = self.llvm_type(err_ty);
                let result_type = self
                    .context
                    .struct_type(&[self.context.i8_type().into(), ok_llvm, err_llvm], false);

                // Extract error value from inner and build new Error result
                let err_val = self
                    .builder
                    .build_extract_value(struct_val, 2, "err_val")
                    .unwrap();
                let alloca = self.builder.build_alloca(result_type, "err_ret").unwrap();
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let tag_ptr = unsafe {
                    self.builder
                        .build_gep(
                            result_type.as_basic_type_enum(),
                            alloca,
                            &[zero, i32_type.const_int(0, false)],
                            "tag",
                        )
                        .unwrap()
                };
                self.builder
                    .build_store(tag_ptr, self.context.i8_type().const_int(0, false))
                    .unwrap();
                let err_ptr = unsafe {
                    self.builder
                        .build_gep(
                            result_type.as_basic_type_enum(),
                            alloca,
                            &[zero, i32_type.const_int(2, false)],
                            "err",
                        )
                        .unwrap()
                };
                self.builder.build_store(err_ptr, err_val).unwrap();
                let loaded = self.builder.build_load(result_type, alloca, "ret").unwrap();
                self.builder.build_return(Some(&loaded)).unwrap();
            }
            _ => {
                return Err(CodegenError::new(
                    "? operator can only be used in functions returning Option or Result",
                ));
            }
        }

        // Success block: extract the value and continue
        self.builder.position_at_end(success_block);
        let extracted = self
            .builder
            .build_extract_value(struct_val, 1, "unwrapped")
            .unwrap();
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();

        // Merge block: return the extracted value
        self.builder.position_at_end(merge_block);

        Ok(extracted)
    }

    pub fn compile_stdlib_function(
        &mut self,
        name: &str,
        args: &[Spanned<Expr>],
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        match name {
            // Math functions
            "Math__abs" => {
                let val = self.compile_expr(&args[0].node)?;
                if val.is_int_value() {
                    let v = val.into_int_value();
                    let is_neg = self
                        .builder
                        .build_int_compare(
                            IntPredicate::SLT,
                            v,
                            self.context.i64_type().const_int(0, false),
                            "is_neg",
                        )
                        .unwrap();
                    let neg = self.builder.build_int_neg(v, "neg").unwrap();
                    let result = self.builder.build_select(is_neg, neg, v, "abs").unwrap();
                    Ok(Some(result))
                } else {
                    let fabs = self.get_or_declare_math_func("fabs", true);
                    let call = self.builder.build_call(fabs, &[val.into()], "abs").unwrap();
                    Ok(Some(self.extract_call_value(call)))
                }
            }
            "Math__min" => {
                let a = self.compile_expr(&args[0].node)?;
                let b = self.compile_expr(&args[1].node)?;
                if a.is_int_value() {
                    let av = a.into_int_value();
                    let bv = b.into_int_value();
                    let cond = self
                        .builder
                        .build_int_compare(IntPredicate::SLT, av, bv, "cmp")
                        .unwrap();
                    let result = self.builder.build_select(cond, av, bv, "min").unwrap();
                    Ok(Some(result))
                } else {
                    let fmin = self.get_or_declare_math_func2("fmin");
                    let call = self
                        .builder
                        .build_call(fmin, &[a.into(), b.into()], "min")
                        .unwrap();
                    Ok(Some(self.extract_call_value(call)))
                }
            }
            "Math__max" => {
                let a = self.compile_expr(&args[0].node)?;
                let b = self.compile_expr(&args[1].node)?;
                if a.is_int_value() {
                    let av = a.into_int_value();
                    let bv = b.into_int_value();
                    let cond = self
                        .builder
                        .build_int_compare(IntPredicate::SGT, av, bv, "cmp")
                        .unwrap();
                    let result = self.builder.build_select(cond, av, bv, "max").unwrap();
                    Ok(Some(result))
                } else {
                    let fmax = self.get_or_declare_math_func2("fmax");
                    let call = self
                        .builder
                        .build_call(fmax, &[a.into(), b.into()], "max")
                        .unwrap();
                    Ok(Some(self.extract_call_value(call)))
                }
            }
            "Math__sqrt" => {
                let val = self.compile_expr(&args[0].node)?;
                let sqrt = self.get_or_declare_math_func("sqrt", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(sqrt, &[fval.into()], "sqrt")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)))
            }
            "Math__pow" => {
                let base = self.compile_expr(&args[0].node)?;
                let exp = self.compile_expr(&args[1].node)?;
                let pow_fn = self.get_or_declare_math_func2("pow");
                let fbase = if base.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            base.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    base
                };
                let fexp = if exp.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            exp.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    exp
                };
                let call = self
                    .builder
                    .build_call(pow_fn, &[fbase.into(), fexp.into()], "pow")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)))
            }
            "Math__sin" => {
                let val = self.compile_expr(&args[0].node)?;
                let sin_fn = self.get_or_declare_math_func("sin", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(sin_fn, &[fval.into()], "sin")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)))
            }
            "Math__cos" => {
                let val = self.compile_expr(&args[0].node)?;
                let cos_fn = self.get_or_declare_math_func("cos", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(cos_fn, &[fval.into()], "cos")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)))
            }
            "Math__tan" => {
                let val = self.compile_expr(&args[0].node)?;
                let tan_fn = self.get_or_declare_math_func("tan", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(tan_fn, &[fval.into()], "tan")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)))
            }
            "Math__floor" => {
                let val = self.compile_expr(&args[0].node)?;
                let floor_fn = self.get_or_declare_math_func("floor", true);
                let call = self
                    .builder
                    .build_call(floor_fn, &[val.into()], "floor")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)))
            }
            "Math__ceil" => {
                let val = self.compile_expr(&args[0].node)?;
                let ceil_fn = self.get_or_declare_math_func("ceil", true);
                let call = self
                    .builder
                    .build_call(ceil_fn, &[val.into()], "ceil")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)))
            }
            "Math__round" => {
                let val = self.compile_expr(&args[0].node)?;
                let round_fn = self.get_or_declare_math_func("round", true);
                let call = self
                    .builder
                    .build_call(round_fn, &[val.into()], "round")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)))
            }
            "Math__log" => {
                let val = self.compile_expr(&args[0].node)?;
                let log_fn = self.get_or_declare_math_func("log", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(log_fn, &[fval.into()], "log")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)))
            }
            "Math__log10" => {
                let val = self.compile_expr(&args[0].node)?;
                let log10_fn = self.get_or_declare_math_func("log10", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(log10_fn, &[fval.into()], "log10")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)))
            }
            "Math__exp" => {
                let val = self.compile_expr(&args[0].node)?;
                let exp_fn = self.get_or_declare_math_func("exp", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(exp_fn, &[fval.into()], "exp")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)))
            }

            "Math__random" => {
                let rand_fn = self.get_or_declare_rand();
                let res = self.builder.build_call(rand_fn, &[], "r").unwrap();
                let val = self.extract_call_value(res).into_int_value();
                let fval = self
                    .builder
                    .build_unsigned_int_to_float(val, self.context.f64_type(), "rf")
                    .unwrap();
                let rand_max = self.context.f64_type().const_float(32767.0);
                let norm = self.builder.build_float_div(fval, rand_max, "rnd").unwrap();
                Ok(Some(norm.into()))
            }

            "Math__pi" => Ok(Some(
                self.context
                    .f64_type()
                    .const_float(std::f64::consts::PI)
                    .into(),
            )),
            "Math__e" => Ok(Some(
                self.context
                    .f64_type()
                    .const_float(std::f64::consts::E)
                    .into(),
            )),

            // Type conversion functions
            "to_float" => {
                let val = self.compile_expr(&args[0].node)?;
                if val.is_int_value() {
                    let result = self
                        .builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap();
                    Ok(Some(result.into()))
                } else {
                    Ok(Some(val))
                }
            }
            "to_int" => {
                let val = self.compile_expr(&args[0].node)?;
                if val.is_float_value() {
                    let result = self
                        .builder
                        .build_float_to_signed_int(
                            val.into_float_value(),
                            self.context.i64_type(),
                            "toint",
                        )
                        .unwrap();
                    Ok(Some(result.into()))
                } else {
                    Ok(Some(val))
                }
            }
            "to_string" => {
                let val = self.compile_expr(&args[0].node)?;

                // Special handling for Booleans (i1 in LLVM)
                if val.is_int_value() && val.into_int_value().get_type().get_bit_width() == 1 {
                    let int_val = val.into_int_value();
                    let true_s = self.context.const_string(b"true", true);
                    let false_s = self.context.const_string(b"false", true);

                    let t_name = format!("str.bool.true.{}", self.str_counter);
                    let f_name = format!("str.bool.false.{}", self.str_counter);
                    self.str_counter += 1;

                    let t_glob = self.module.add_global(true_s.get_type(), None, &t_name);
                    t_glob.set_linkage(Linkage::Private);
                    t_glob.set_initializer(&true_s);
                    t_glob.set_constant(true);

                    let f_glob = self.module.add_global(false_s.get_type(), None, &f_name);
                    f_glob.set_linkage(Linkage::Private);
                    f_glob.set_initializer(&false_s);
                    f_glob.set_constant(true);

                    let res = self
                        .builder
                        .build_select(
                            int_val,
                            t_glob.as_pointer_value(),
                            f_glob.as_pointer_value(),
                            "bool_str",
                        )
                        .unwrap();
                    return Ok(Some(res));
                }

                let sprintf = self.get_or_declare_sprintf();
                let malloc = self.get_or_declare_malloc();

                // Allocate buffer
                let buffer_size = self.context.i64_type().const_int(64, false);
                let buffer_call = self
                    .builder
                    .build_call(malloc, &[buffer_size.into()], "strbuf")
                    .unwrap();
                let buffer = self.extract_call_value(buffer_call).into_pointer_value();

                // Format string based on type
                let (fmt, print_args): (&str, Vec<BasicMetadataValueEnum>) = if val.is_int_value() {
                    // Promote to i64 for %lld
                    let int_val = val.into_int_value();
                    let promoted = self
                        .builder
                        .build_int_s_extend(int_val, self.context.i64_type(), "promoted")
                        .unwrap();
                    ("%lld", vec![promoted.into()])
                } else if val.is_float_value() {
                    ("%f", vec![val.into()])
                } else {
                    ("%s", vec![val.into()])
                };

                let fmt_val = self.context.const_string(fmt.as_bytes(), true);
                let fmt_name = format!("fmt.{}", self.str_counter);
                self.str_counter += 1;
                let fmt_global = self.module.add_global(fmt_val.get_type(), None, &fmt_name);
                fmt_global.set_linkage(Linkage::Private);
                fmt_global.set_initializer(&fmt_val);

                let mut sprintf_args: Vec<BasicMetadataValueEnum> =
                    vec![buffer.into(), fmt_global.as_pointer_value().into()];
                sprintf_args.extend(print_args);
                self.builder
                    .build_call(sprintf, &sprintf_args, "sprintf")
                    .unwrap();

                Ok(Some(buffer.into()))
            }

            // String functions
            "Str__len" => {
                let s = self.compile_expr(&args[0].node)?;
                let strlen_fn = self.get_or_declare_strlen();
                let call = self
                    .builder
                    .build_call(strlen_fn, &[s.into()], "len")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)))
            }
            "Str__compare" => {
                let s1 = self.compile_expr(&args[0].node)?;
                let s2 = self.compile_expr(&args[1].node)?;
                let strcmp_fn = self.get_or_declare_strcmp();
                let call = self
                    .builder
                    .build_call(strcmp_fn, &[s1.into(), s2.into()], "cmp")
                    .unwrap();
                // strcmp returns i32, extend to i64
                let result = self.extract_call_value(call).into_int_value();
                let extended = self
                    .builder
                    .build_int_s_extend(result, self.context.i64_type(), "cmp64")
                    .unwrap();
                Ok(Some(extended.into()))
            }
            "Str__concat" => {
                // Allocate new buffer and concatenate
                let s1 = self.compile_expr(&args[0].node)?;
                let s2 = self.compile_expr(&args[1].node)?;

                let strlen_fn = self.get_or_declare_strlen();
                let malloc = self.get_or_declare_malloc();
                let strcpy_fn = self.get_or_declare_strcpy();
                let strcat_fn = self.get_or_declare_strcat();

                // Get lengths
                let len1_call = self
                    .builder
                    .build_call(strlen_fn, &[s1.into()], "len1")
                    .unwrap();
                let len1 = self.extract_call_value(len1_call).into_int_value();
                let len2_call = self
                    .builder
                    .build_call(strlen_fn, &[s2.into()], "len2")
                    .unwrap();
                let len2 = self.extract_call_value(len2_call).into_int_value();

                // Allocate len1 + len2 + 1
                let total_len = self.builder.build_int_add(len1, len2, "total").unwrap();
                let buffer_size = self
                    .builder
                    .build_int_add(
                        total_len,
                        self.context.i64_type().const_int(1, false),
                        "bufsize",
                    )
                    .unwrap();

                let buffer_call = self
                    .builder
                    .build_call(malloc, &[buffer_size.into()], "buf")
                    .unwrap();
                let buffer = self.extract_call_value(buffer_call).into_pointer_value();

                // strcpy(buffer, s1)
                self.builder
                    .build_call(strcpy_fn, &[buffer.into(), s1.into()], "")
                    .unwrap();
                // strcat(buffer, s2)
                self.builder
                    .build_call(strcat_fn, &[buffer.into(), s2.into()], "")
                    .unwrap();

                Ok(Some(buffer.into()))
            }

            "Str__upper" => {
                let s = self.compile_expr(&args[0].node)?;
                let toupper_fn = self.get_or_declare_toupper();
                self.compile_string_transform(s, toupper_fn).map(Some)
            }

            "Str__lower" => {
                let s = self.compile_expr(&args[0].node)?;
                let tolower_fn = self.get_or_declare_tolower();
                self.compile_string_transform(s, tolower_fn).map(Some)
            }

            "Str__trim" => {
                let s = self.compile_expr(&args[0].node)?;
                let s_ptr = s.into_pointer_value();
                let strlen_fn = self.get_or_declare_strlen();
                let isspace_fn = self.get_or_declare_isspace();
                let malloc_fn = self.get_or_declare_malloc();
                let strncpy_fn = self.get_or_declare_strncpy();

                let len_call = self
                    .builder
                    .build_call(strlen_fn, &[s_ptr.into()], "len")
                    .unwrap();
                let len = self.extract_call_value(len_call).into_int_value();

                // Find start (first non-space)
                let start_ptr = self
                    .builder
                    .build_alloca(self.context.i64_type(), "start")
                    .unwrap();
                self.builder
                    .build_store(start_ptr, self.context.i64_type().const_int(0, false))
                    .unwrap();

                let cur_fn = self.current_function.unwrap();
                let start_cond = self.context.append_basic_block(cur_fn, "trim.start.cond");
                let start_body = self.context.append_basic_block(cur_fn, "trim.start.body");
                let start_after = self.context.append_basic_block(cur_fn, "trim.start.after");
                self.builder.build_unconditional_branch(start_cond).unwrap();

                self.builder.position_at_end(start_cond);
                let start_val = self
                    .builder
                    .build_load(self.context.i64_type(), start_ptr, "s")
                    .unwrap()
                    .into_int_value();
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, start_val, len, "bounds")
                    .unwrap();
                let char_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), s_ptr, &[start_val], "")
                        .unwrap()
                };
                let char_val = self
                    .builder
                    .build_load(self.context.i8_type(), char_ptr, "")
                    .unwrap();
                let char_i32 = self
                    .builder
                    .build_int_s_extend(char_val.into_int_value(), self.context.i32_type(), "")
                    .unwrap();
                let is_space_call = self
                    .builder
                    .build_call(isspace_fn, &[char_i32.into()], "")
                    .unwrap();
                let is_space = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        self.extract_call_value(is_space_call).into_int_value(),
                        self.context.i32_type().const_int(0, false),
                        "",
                    )
                    .unwrap();
                let cond = self.builder.build_and(in_bounds, is_space, "").unwrap();
                self.builder
                    .build_conditional_branch(cond, start_body, start_after)
                    .unwrap();

                self.builder.position_at_end(start_body);
                let next_start = self
                    .builder
                    .build_int_add(start_val, self.context.i64_type().const_int(1, false), "")
                    .unwrap();
                self.builder.build_store(start_ptr, next_start).unwrap();
                self.builder.build_unconditional_branch(start_cond).unwrap();

                self.builder.position_at_end(start_after);
                let start_final = self
                    .builder
                    .build_load(self.context.i64_type(), start_ptr, "start_f")
                    .unwrap()
                    .into_int_value();

                // Find end (last non-space)
                let end_ptr = self
                    .builder
                    .build_alloca(self.context.i64_type(), "end")
                    .unwrap();
                self.builder.build_store(end_ptr, len).unwrap();

                let end_cond = self.context.append_basic_block(cur_fn, "trim.end.cond");
                let end_body = self.context.append_basic_block(cur_fn, "trim.end.body");
                let end_after = self.context.append_basic_block(cur_fn, "trim.end.after");
                self.builder.build_unconditional_branch(end_cond).unwrap();

                self.builder.position_at_end(end_cond);
                let end_val = self
                    .builder
                    .build_load(self.context.i64_type(), end_ptr, "e")
                    .unwrap()
                    .into_int_value();
                let gt_start = self
                    .builder
                    .build_int_compare(IntPredicate::SGT, end_val, start_final, "gt_start")
                    .unwrap();
                let last_idx = self
                    .builder
                    .build_int_sub(end_val, self.context.i64_type().const_int(1, false), "")
                    .unwrap();
                let char_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), s_ptr, &[last_idx], "")
                        .unwrap()
                };
                let char_val = self
                    .builder
                    .build_load(self.context.i8_type(), char_ptr, "")
                    .unwrap();
                let char_i32 = self
                    .builder
                    .build_int_s_extend(char_val.into_int_value(), self.context.i32_type(), "")
                    .unwrap();
                let is_space_call = self
                    .builder
                    .build_call(isspace_fn, &[char_i32.into()], "")
                    .unwrap();
                let is_space = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        self.extract_call_value(is_space_call).into_int_value(),
                        self.context.i32_type().const_int(0, false),
                        "",
                    )
                    .unwrap();
                let cond = self.builder.build_and(gt_start, is_space, "").unwrap();
                self.builder
                    .build_conditional_branch(cond, end_body, end_after)
                    .unwrap();

                self.builder.position_at_end(end_body);
                let next_end = self
                    .builder
                    .build_int_sub(end_val, self.context.i64_type().const_int(1, false), "")
                    .unwrap();
                self.builder.build_store(end_ptr, next_end).unwrap();
                self.builder.build_unconditional_branch(end_cond).unwrap();

                self.builder.position_at_end(end_after);
                let end_final = self
                    .builder
                    .build_load(self.context.i64_type(), end_ptr, "end_f")
                    .unwrap()
                    .into_int_value();

                // Allocate and copy result
                let new_len = self
                    .builder
                    .build_int_sub(end_final, start_final, "new_len")
                    .unwrap();
                let alloc_size = self
                    .builder
                    .build_int_add(
                        new_len,
                        self.context.i64_type().const_int(1, false),
                        "alloc",
                    )
                    .unwrap();
                let buf_call = self
                    .builder
                    .build_call(malloc_fn, &[alloc_size.into()], "buf")
                    .unwrap();
                let buf = self.extract_call_value(buf_call).into_pointer_value();

                let src_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), s_ptr, &[start_final], "src")
                        .unwrap()
                };
                self.builder
                    .build_call(
                        strncpy_fn,
                        &[buf.into(), src_ptr.into(), new_len.into()],
                        "",
                    )
                    .unwrap();

                // Null terminate
                let term_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), buf, &[new_len], "")
                        .unwrap()
                };
                self.builder
                    .build_store(term_ptr, self.context.i8_type().const_int(0, false))
                    .unwrap();

                Ok(Some(buf.into()))
            }

            "Str__contains" => {
                let s = self.compile_expr(&args[0].node)?;
                let sub = self.compile_expr(&args[1].node)?;
                let strstr = self.get_or_declare_strstr();
                let res = self
                    .builder
                    .build_call(strstr, &[s.into(), sub.into()], "pos")
                    .unwrap();
                let ptr = self.extract_call_value(res).into_pointer_value();
                let is_null = self.builder.build_is_null(ptr, "not_found").unwrap();
                let found = self.builder.build_not(is_null, "found").unwrap();
                Ok(Some(found.into()))
            }
            "Str__startsWith" => {
                let s = self.compile_expr(&args[0].node)?;
                let pre = self.compile_expr(&args[1].node)?;
                let strlen = self.get_or_declare_strlen();
                let strncmp = self.get_or_declare_strncmp();

                let pre_len = self
                    .builder
                    .build_call(strlen, &[pre.into()], "pre_len")
                    .unwrap();
                let res = self
                    .builder
                    .build_call(
                        strncmp,
                        &[
                            s.into(),
                            pre.into(),
                            self.extract_call_value(pre_len).into_int_value().into(),
                        ],
                        "cmp",
                    )
                    .unwrap();
                let is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        self.extract_call_value(res).into_int_value(),
                        self.context.i32_type().const_int(0, false),
                        "is_zero",
                    )
                    .unwrap();
                Ok(Some(is_zero.into()))
            }
            "Str__endsWith" => {
                let s = self.compile_expr(&args[0].node)?;
                let suf = self.compile_expr(&args[1].node)?;
                let strlen = self.get_or_declare_strlen();
                let strcmp = self.get_or_declare_strcmp();

                let s_len = self
                    .builder
                    .build_call(strlen, &[s.into()], "s_len")
                    .unwrap();
                let suf_len = self
                    .builder
                    .build_call(strlen, &[suf.into()], "suf_len")
                    .unwrap();

                let s_len_val = self.extract_call_value(s_len).into_int_value();
                let suf_len_val = self.extract_call_value(suf_len).into_int_value();

                let can_end = self
                    .builder
                    .build_int_compare(IntPredicate::UGE, s_len_val, suf_len_val, "can_end")
                    .unwrap();

                let start_idx = self
                    .builder
                    .build_int_sub(s_len_val, suf_len_val, "")
                    .unwrap();
                let s_suffix_ptr = unsafe {
                    self.builder
                        .build_gep(
                            self.context.i8_type(),
                            s.into_pointer_value(),
                            &[start_idx],
                            "",
                        )
                        .unwrap()
                };

                let res = self
                    .builder
                    .build_call(strcmp, &[s_suffix_ptr.into(), suf.into()], "cmp")
                    .unwrap();
                let is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        self.extract_call_value(res).into_int_value(),
                        self.context.i32_type().const_int(0, false),
                        "is_zero",
                    )
                    .unwrap();

                let final_res = self.builder.build_and(can_end, is_zero, "").unwrap();
                Ok(Some(final_res.into()))
            }

            // I/O functions
            "read_line" => {
                // Read a line from stdin
                let malloc = self.get_or_declare_malloc();
                let fgets = self.get_or_declare_fgets();
                let stdin = self.get_or_declare_stdin();

                let buffer_size = self.context.i64_type().const_int(1024, false);
                let buffer_call = self
                    .builder
                    .build_call(malloc, &[buffer_size.into()], "linebuf")
                    .unwrap();
                let buffer = self.extract_call_value(buffer_call).into_pointer_value();

                let stdin_val = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        stdin,
                        "stdin",
                    )
                    .unwrap();

                self.builder
                    .build_call(
                        fgets,
                        &[
                            buffer.into(),
                            self.context.i32_type().const_int(1024, false).into(),
                            stdin_val.into(),
                        ],
                        "fgets",
                    )
                    .unwrap();

                Ok(Some(buffer.into()))
            }
            "System__exit" | "exit" => {
                let code = self.compile_expr(&args[0].node)?;
                let exit_fn = self.get_or_declare_exit();
                let code_i32 = self
                    .builder
                    .build_int_truncate(code.into_int_value(), self.context.i32_type(), "exitcode")
                    .unwrap();
                self.builder
                    .build_call(exit_fn, &[code_i32.into()], "")
                    .unwrap();
                Ok(None) // void function
            }
            "range" => {
                // range(start, end) or range(start, end, step)
                // Returns a Range struct { start, end, step, current }
                let start = self.compile_expr(&args[0].node)?;
                let end = self.compile_expr(&args[1].node)?;
                let step = if args.len() == 3 {
                    self.compile_expr(&args[2].node)?
                } else {
                    match start {
                        BasicValueEnum::IntValue(v) => v.get_type().const_int(1, false).into(),
                        BasicValueEnum::FloatValue(v) => v.get_type().const_float(1.0).into(),
                        _ => {
                            return Err(CodegenError::new(
                                "range() codegen supports only Integer and Float elements",
                            ));
                        }
                    }
                };

                // Allocate and initialize Range struct
                let range_ptr = self.create_range(start, end, step)?;
                Ok(Some(range_ptr.into()))
            }

            // File I/O
            "File__write" => {
                let path = self.compile_expr(&args[0].node)?;
                let content = self.compile_expr(&args[1].node)?;

                let fopen = self.get_or_declare_fopen();
                let fputs = self.get_or_declare_fputs();
                let fclose = self.get_or_declare_fclose();

                let mode = self.context.const_string(b"w", true);
                let mode_global = self.module.add_global(mode.get_type(), None, "mode_w");
                mode_global.set_linkage(Linkage::Private);
                mode_global.set_initializer(&mode);

                let file_call = self
                    .builder
                    .build_call(
                        fopen,
                        &[path.into(), mode_global.as_pointer_value().into()],
                        "file",
                    )
                    .unwrap();
                let file_ptr = self.extract_call_value(file_call).into_pointer_value();

                let _null = self.context.ptr_type(AddressSpace::default()).const_null();
                let is_null = self.builder.build_is_null(file_ptr, "is_null").unwrap();

                let success_block = self
                    .context
                    .append_basic_block(self.current_function.unwrap(), "file.success");
                let fail_block = self
                    .context
                    .append_basic_block(self.current_function.unwrap(), "file.fail");
                let merge_block = self
                    .context
                    .append_basic_block(self.current_function.unwrap(), "file.merge");

                self.builder
                    .build_conditional_branch(is_null, fail_block, success_block)
                    .unwrap();

                // Fail
                self.builder.position_at_end(fail_block);
                self.builder
                    .build_unconditional_branch(merge_block)
                    .unwrap();

                // Success
                self.builder.position_at_end(success_block);
                self.builder
                    .build_call(fputs, &[content.into(), file_ptr.into()], "write")
                    .unwrap();
                self.builder
                    .build_call(fclose, &[file_ptr.into()], "close")
                    .unwrap();
                self.builder
                    .build_unconditional_branch(merge_block)
                    .unwrap();

                // Merge
                self.builder.position_at_end(merge_block);
                let phi = self
                    .builder
                    .build_phi(self.context.bool_type(), "result")
                    .unwrap();
                let true_val = self.context.bool_type().const_int(1, false);
                let false_val = self.context.bool_type().const_int(0, false);
                phi.add_incoming(&[(&false_val, fail_block), (&true_val, success_block)]);

                Ok(Some(phi.as_basic_value()))
            }

            "File__read" => {
                let path = self.compile_expr(&args[0].node)?;

                let fopen = self.get_or_declare_fopen();
                let fseek = self.get_or_declare_fseek();
                let ftell = self.get_or_declare_ftell();
                let rewind = self.get_or_declare_rewind();
                let fread = self.get_or_declare_fread();
                let fclose = self.get_or_declare_fclose();
                let malloc = self.get_or_declare_malloc();

                let mode = self.context.const_string(b"rb", true); // Binary mode to get exact bytes
                let mode_global = self.module.add_global(mode.get_type(), None, "mode_r");
                mode_global.set_linkage(Linkage::Private);
                mode_global.set_initializer(&mode);

                let file_call = self
                    .builder
                    .build_call(
                        fopen,
                        &[path.into(), mode_global.as_pointer_value().into()],
                        "file",
                    )
                    .unwrap();
                let file_ptr = self.extract_call_value(file_call).into_pointer_value();

                let is_null = self.builder.build_is_null(file_ptr, "is_null").unwrap();

                let success_block = self
                    .context
                    .append_basic_block(self.current_function.unwrap(), "read.success");
                let fail_block = self
                    .context
                    .append_basic_block(self.current_function.unwrap(), "read.fail");
                let merge_block = self
                    .context
                    .append_basic_block(self.current_function.unwrap(), "read.merge");

                self.builder
                    .build_conditional_branch(is_null, fail_block, success_block)
                    .unwrap();

                // Fail - return empty string
                self.builder.position_at_end(fail_block);
                let empty_str = self.context.const_string(b"", true);
                let empty_global = self
                    .module
                    .add_global(empty_str.get_type(), None, "empty_s");
                empty_global.set_linkage(Linkage::Private);
                empty_global.set_initializer(&empty_str);
                let fail_res = empty_global.as_pointer_value();
                self.builder
                    .build_unconditional_branch(merge_block)
                    .unwrap();

                // Success
                self.builder.position_at_end(success_block);
                // fseek(f, 0, SEEK_END)
                let seek_end = self.context.i32_type().const_int(2, false); // SEEK_END = 2
                let zero = self.context.i64_type().const_int(0, false);
                self.builder
                    .build_call(fseek, &[file_ptr.into(), zero.into(), seek_end.into()], "")
                    .unwrap();

                // size = ftell(f)
                let size_call = self
                    .builder
                    .build_call(ftell, &[file_ptr.into()], "size")
                    .unwrap();
                let size = self.extract_call_value(size_call).into_int_value();

                // rewind(f)
                self.builder
                    .build_call(rewind, &[file_ptr.into()], "")
                    .unwrap();

                // buffer = malloc(size + 1)
                let one = self.context.i64_type().const_int(1, false);
                let alloc_size = self.builder.build_int_add(size, one, "alloc_size").unwrap();
                let buffer_call = self
                    .builder
                    .build_call(malloc, &[alloc_size.into()], "buffer")
                    .unwrap();
                let buffer = self.extract_call_value(buffer_call).into_pointer_value();

                // fread(buffer, 1, size, f)
                let size_size_t = size; // Assuming size_t is i64
                self.builder
                    .build_call(
                        fread,
                        &[
                            buffer.into(),
                            one.into(),
                            size_size_t.into(),
                            file_ptr.into(),
                        ],
                        "",
                    )
                    .unwrap();

                // buffer[size] = 0 (null terminate)
                let term_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), buffer, &[size], "term_ptr")
                        .unwrap()
                };
                self.builder
                    .build_store(term_ptr, self.context.i8_type().const_int(0, false))
                    .unwrap();

                // fclose(f)
                self.builder
                    .build_call(fclose, &[file_ptr.into()], "")
                    .unwrap();

                self.builder
                    .build_unconditional_branch(merge_block)
                    .unwrap();

                // Merge
                self.builder.position_at_end(merge_block);
                let phi = self
                    .builder
                    .build_phi(self.context.ptr_type(AddressSpace::default()), "result")
                    .unwrap();
                phi.add_incoming(&[(&fail_res, fail_block), (&buffer, success_block)]);

                Ok(Some(phi.as_basic_value()))
            }

            "File__exists" => {
                let path = self.compile_expr(&args[0].node)?;
                let fopen = self.get_or_declare_fopen();
                let fclose = self.get_or_declare_fclose();

                let mode = self.context.const_string(b"r", true);
                let mode_global = self.module.add_global(mode.get_type(), None, "mode_r");
                mode_global.set_linkage(Linkage::Private);
                mode_global.set_initializer(&mode);

                let file_call = self
                    .builder
                    .build_call(
                        fopen,
                        &[path.into(), mode_global.as_pointer_value().into()],
                        "file",
                    )
                    .unwrap();
                let file_ptr = self.extract_call_value(file_call).into_pointer_value();

                let is_null = self.builder.build_is_null(file_ptr, "is_null").unwrap();

                let exists = self.builder.build_not(is_null, "exists").unwrap();

                // Close if opened
                let close_block = self
                    .context
                    .append_basic_block(self.current_function.unwrap(), "exists.close");
                let end_block = self
                    .context
                    .append_basic_block(self.current_function.unwrap(), "exists.end");

                self.builder
                    .build_conditional_branch(exists, close_block, end_block)
                    .unwrap();

                self.builder.position_at_end(close_block);
                self.builder
                    .build_call(fclose, &[file_ptr.into()], "")
                    .unwrap();
                self.builder.build_unconditional_branch(end_block).unwrap();

                self.builder.position_at_end(end_block);

                // Cast i1 to boolean (i1) - basically same
                Ok(Some(exists.into()))
            }

            "File__delete" => {
                let path = self.compile_expr(&args[0].node)?;
                let remove = self.get_or_declare_remove();

                let res_call = self
                    .builder
                    .build_call(remove, &[path.into()], "res")
                    .unwrap();
                let res = self.extract_call_value(res_call).into_int_value();

                let zero = self.context.i32_type().const_int(0, false);
                let success = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, res, zero, "success")
                    .unwrap();

                Ok(Some(success.into()))
            }

            // Time Functions
            "Time__now" => {
                let format = self.compile_expr(&args[0].node)?;
                let time_fn = self.get_or_declare_time();
                let localtime_fn = self.get_or_declare_localtime();
                let strftime_fn = self.get_or_declare_strftime();
                let malloc = self.get_or_declare_malloc();

                // 1. Get current time
                let null = self.context.ptr_type(AddressSpace::default()).const_null();
                let t_val = self
                    .builder
                    .build_call(time_fn, &[null.into()], "t")
                    .unwrap();
                let t_raw = self.extract_call_value(t_val);

                // 2. Alloca for time_t (i64)
                let t_ptr = self
                    .builder
                    .build_alloca(self.context.i64_type(), "t_ptr")
                    .unwrap();
                self.builder.build_store(t_ptr, t_raw).unwrap();

                // 3. Get local time struct pointer
                let tm_ptr_val = self
                    .builder
                    .build_call(localtime_fn, &[t_ptr.into()], "tm")
                    .unwrap();
                let tm_ptr = self.extract_call_value(tm_ptr_val).into_pointer_value();

                // 4. Allocate buffer for string (64 bytes should be enough for time)
                let buf_size = self.context.i64_type().const_int(64, false);
                let buf_ptr_val = self
                    .builder
                    .build_call(malloc, &[buf_size.into()], "buf")
                    .unwrap();
                let buf_ptr = self.extract_call_value(buf_ptr_val).into_pointer_value();

                // 5. If format is empty string, use default "%H:%M:%S"
                let strlen_fn = self.get_or_declare_strlen();
                let is_empty = self
                    .builder
                    .build_call(strlen_fn, &[format.into()], "len")
                    .unwrap();
                let is_empty_val = self.extract_call_value(is_empty).into_int_value();
                let is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        is_empty_val,
                        self.context.i64_type().const_int(0, false),
                        "is_zero",
                    )
                    .unwrap();

                let default_fmt = self.context.const_string(b"%H:%M:%S", true);
                let default_fmt_global =
                    self.module
                        .add_global(default_fmt.get_type(), None, "default_time_fmt");
                default_fmt_global.set_linkage(Linkage::Private);
                default_fmt_global.set_initializer(&default_fmt);

                let actual_fmt = self
                    .builder
                    .build_select(
                        is_zero,
                        default_fmt_global.as_pointer_value(),
                        format.into_pointer_value(),
                        "fmt",
                    )
                    .unwrap();

                // 6. Call strftime(buf, 64, format, tm)
                self.builder
                    .build_call(
                        strftime_fn,
                        &[
                            buf_ptr.into(),
                            buf_size.into(),
                            actual_fmt.into(),
                            tm_ptr.into(),
                        ],
                        "res",
                    )
                    .unwrap();

                Ok(Some(buf_ptr.into()))
            }

            "Time__unix" => {
                let time_fn = self.get_or_declare_time();
                let null = self.context.ptr_type(AddressSpace::default()).const_null();
                let res = self
                    .builder
                    .build_call(time_fn, &[null.into()], "time")
                    .unwrap();
                Ok(Some(self.extract_call_value(res)))
            }

            "Time__sleep" => {
                let ms = self.compile_expr(&args[0].node)?;
                #[cfg(windows)]
                {
                    let sleep_fn = self.get_or_declare_sleep_win();
                    let ms_i32 = self
                        .builder
                        .build_int_truncate(ms.into_int_value(), self.context.i32_type(), "ms32")
                        .unwrap();
                    self.builder
                        .build_call(sleep_fn, &[ms_i32.into()], "")
                        .unwrap();
                }
                #[cfg(not(windows))]
                {
                    let usleep_fn = self.get_or_declare_usleep();
                    let us = self
                        .builder
                        .build_int_mul(
                            ms.into_int_value(),
                            self.context.i64_type().const_int(1000, false),
                            "us",
                        )
                        .unwrap();
                    let us_i32 = self
                        .builder
                        .build_int_truncate(us, self.context.i32_type(), "us32")
                        .unwrap();
                    self.builder
                        .build_call(usleep_fn, &[us_i32.into()], "")
                        .unwrap();
                }
                Ok(Some(self.context.i8_type().const_int(0, false).into()))
            }

            // System Functions
            "System__getenv" => {
                let name = self.compile_expr(&args[0].node)?;
                let getenv_fn = self.get_or_declare_getenv();
                let res = self
                    .builder
                    .build_call(getenv_fn, &[name.into()], "env")
                    .unwrap();
                let val = self.extract_call_value(res).into_pointer_value();

                // If NULL, return empty string
                let is_null = self.builder.build_is_null(val, "is_null").unwrap();
                let empty_str = self.get_or_create_empty_string();

                let current_fn = self.current_function.unwrap();
                let success_bb = self.context.append_basic_block(current_fn, "env.ok");
                let fail_bb = self.context.append_basic_block(current_fn, "env.fail");
                let merge_bb = self.context.append_basic_block(current_fn, "env.merge");

                self.builder
                    .build_conditional_branch(is_null, fail_bb, success_bb)
                    .unwrap();

                self.builder.position_at_end(fail_bb);
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(success_bb);
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(self.context.ptr_type(AddressSpace::default()), "res")
                    .unwrap();
                phi.add_incoming(&[(&empty_str, fail_bb), (&val, success_bb)]);
                Ok(Some(phi.as_basic_value()))
            }

            "System__shell" => {
                let cmd = self.compile_expr(&args[0].node)?;
                let system_fn = self.get_or_declare_system();
                let res = self
                    .builder
                    .build_call(system_fn, &[cmd.into()], "exit_code")
                    .unwrap();
                let code = self.extract_call_value(res).into_int_value();
                let code64 = self
                    .builder
                    .build_int_s_extend(code, self.context.i64_type(), "code64")
                    .unwrap();
                Ok(Some(code64.into()))
            }

            "System__exec" => {
                let cmd = self.compile_expr(&args[0].node)?;
                let popen_fn = self.get_or_declare_popen();
                let pclose_fn = self.get_or_declare_pclose();
                let fread_fn = self.get_or_declare_fread();
                let malloc = self.get_or_declare_malloc();

                let mode = self.context.const_string(b"r", true);
                let mode_global = self.module.add_global(mode.get_type(), None, "mode_pop_r");
                mode_global.set_linkage(Linkage::Private);
                mode_global.set_initializer(&mode);

                let pipe_val = self
                    .builder
                    .build_call(
                        popen_fn,
                        &[cmd.into(), mode_global.as_pointer_value().into()],
                        "pipe",
                    )
                    .unwrap();
                let pipe_ptr = self.extract_call_value(pipe_val).into_pointer_value();

                let is_null = self.builder.build_is_null(pipe_ptr, "is_null").unwrap();

                let current_fn = self.current_function.unwrap();
                let success_bb = self.context.append_basic_block(current_fn, "exec.ok");
                let fail_bb = self.context.append_basic_block(current_fn, "exec.fail");
                let merge_bb = self.context.append_basic_block(current_fn, "exec.merge");

                self.builder
                    .build_conditional_branch(is_null, fail_bb, success_bb)
                    .unwrap();

                // Fail - return empty string
                self.builder.position_at_end(fail_bb);
                let empty_str = self.get_or_create_empty_string();
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                // Success - Read from pipe
                self.builder.position_at_end(success_bb);
                let buf_size = self.context.i64_type().const_int(4096, false); // Cap at 4KB for simplicity
                let buf_call = self
                    .builder
                    .build_call(malloc, &[buf_size.into()], "buf")
                    .unwrap();
                let buf = self.extract_call_value(buf_call).into_pointer_value();

                let one = self.context.i64_type().const_int(1, false);
                let read_len_call = self
                    .builder
                    .build_call(
                        fread_fn,
                        &[buf.into(), one.into(), buf_size.into(), pipe_ptr.into()],
                        "read_len",
                    )
                    .unwrap();
                let read_len = self.extract_call_value(read_len_call).into_int_value();

                // Null terminate at read_len
                let term_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), buf, &[read_len], "term_ptr")
                        .unwrap()
                };
                self.builder
                    .build_store(term_ptr, self.context.i8_type().const_int(0, false))
                    .unwrap();

                self.builder
                    .build_call(pclose_fn, &[pipe_ptr.into()], "")
                    .unwrap();
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                // Merge
                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(self.context.ptr_type(AddressSpace::default()), "res")
                    .unwrap();
                phi.add_incoming(&[(&empty_str, fail_bb), (&buf, success_bb)]);
                Ok(Some(phi.as_basic_value()))
            }

            "System__cwd" => {
                let getcwd_fn = self.get_or_declare_getcwd();
                let malloc = self.get_or_declare_malloc();
                let size = self.context.i64_type().const_int(1024, false);
                let buf_call = self
                    .builder
                    .build_call(malloc, &[size.into()], "buf")
                    .unwrap();
                let buf = self.extract_call_value(buf_call).into_pointer_value();
                self.builder
                    .build_call(getcwd_fn, &[buf.into(), size.into()], "cwd")
                    .unwrap();
                Ok(Some(buf.into()))
            }
            "System__os" => {
                let os = if cfg!(target_os = "windows") {
                    "windows"
                } else if cfg!(target_os = "macos") {
                    "macos"
                } else if cfg!(target_os = "linux") {
                    "linux"
                } else {
                    "unknown"
                };
                let str_val = self.context.const_string(os.as_bytes(), true);
                let name = format!("str.os.{}", self.str_counter);
                self.str_counter += 1;
                let global = self.module.add_global(str_val.get_type(), None, &name);
                global.set_linkage(Linkage::Private);
                global.set_initializer(&str_val);
                Ok(Some(global.as_pointer_value().into()))
            }

            // Args Functions
            "Args__count" => {
                let argc_global = self.module.get_global("_apex_argc").unwrap();
                let argc = self
                    .builder
                    .build_load(
                        self.context.i32_type(),
                        argc_global.as_pointer_value(),
                        "argc",
                    )
                    .unwrap()
                    .into_int_value();
                let argc64 = self
                    .builder
                    .build_int_s_extend(argc, self.context.i64_type(), "argc64")
                    .unwrap();
                Ok(Some(argc64.into()))
            }

            "Args__get" => {
                let index = self.compile_expr(&args[0].node)?.into_int_value();
                let argv_global = self.module.get_global("_apex_argv").unwrap();
                let argv = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        argv_global.as_pointer_value(),
                        "argv",
                    )
                    .unwrap()
                    .into_pointer_value();

                // index is i64, need to truncate to i32 for GEP if needed, but ptr is 64bit
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(
                            self.context.ptr_type(AddressSpace::default()),
                            argv,
                            &[index],
                            "arg_ptr",
                        )
                        .unwrap()
                };
                let arg_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        elem_ptr,
                        "arg",
                    )
                    .unwrap();
                Ok(Some(arg_ptr))
            }

            // Assertion functions for testing
            "assert" => {
                // assert(condition: Boolean): None - panics if condition is false
                let condition = self.compile_expr(&args[0].node)?;
                let condition_bool = if condition.is_int_value() {
                    let int_val = condition.into_int_value();
                    // Handle both i1 (bool) and i64 (integer) types
                    if int_val.get_type().get_bit_width() == 1 {
                        // Already i1 (boolean)
                        int_val
                    } else {
                        // Convert i64 to i1 (boolean)
                        self.builder
                            .build_int_compare(
                                IntPredicate::NE,
                                int_val,
                                self.context.i64_type().const_int(0, false),
                                "bool_cond",
                            )
                            .unwrap()
                    }
                } else {
                    return Err(CodegenError::new("assert requires boolean condition"));
                };

                let current_fn = self.current_function.unwrap();
                let panic_bb = self.context.append_basic_block(current_fn, "assert_panic");
                let ok_bb = self.context.append_basic_block(current_fn, "assert_ok");

                self.builder
                    .build_conditional_branch(condition_bool, ok_bb, panic_bb)
                    .unwrap();

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr("Assertion failed!\\n", "assert_fail")
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .unwrap();
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .unwrap();
                self.builder.build_unreachable().unwrap();

                // OK block
                self.builder.position_at_end(ok_bb);
                Ok(None) // void function
            }

            "assert_eq" => {
                // assert_eq(a: T, b: T): None - panics if a != b
                let a = self.compile_expr(&args[0].node)?;
                let b = self.compile_expr(&args[1].node)?;

                let equal = if a.is_int_value() && b.is_int_value() {
                    self.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            a.into_int_value(),
                            b.into_int_value(),
                            "eq_cmp",
                        )
                        .unwrap()
                } else if a.is_float_value() && b.is_float_value() {
                    self.builder
                        .build_float_compare(
                            FloatPredicate::OEQ,
                            a.into_float_value(),
                            b.into_float_value(),
                            "eq_cmp",
                        )
                        .unwrap()
                } else {
                    // String comparison
                    let strcmp = self.get_or_declare_strcmp();
                    let res = self
                        .builder
                        .build_call(strcmp, &[a.into(), b.into()], "cmp")
                        .unwrap();
                    let cmp_val = self.extract_call_value(res).into_int_value();
                    self.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            cmp_val,
                            self.context.i32_type().const_int(0, false),
                            "eq_cmp",
                        )
                        .unwrap()
                };

                let current_fn = self.current_function.unwrap();
                let panic_bb = self
                    .context
                    .append_basic_block(current_fn, "assert_eq_panic");
                let ok_bb = self.context.append_basic_block(current_fn, "assert_eq_ok");

                self.builder
                    .build_conditional_branch(equal, ok_bb, panic_bb)
                    .unwrap();

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Assertion failed: values are not equal!\\n",
                        "assert_eq_fail",
                    )
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .unwrap();
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .unwrap();
                self.builder.build_unreachable().unwrap();

                // OK block
                self.builder.position_at_end(ok_bb);
                Ok(None) // void function
            }

            "assert_ne" => {
                // assert_ne(a: T, b: T): None - panics if a == b
                let a = self.compile_expr(&args[0].node)?;
                let b = self.compile_expr(&args[1].node)?;

                let not_equal = if a.is_int_value() && b.is_int_value() {
                    self.builder
                        .build_int_compare(
                            IntPredicate::NE,
                            a.into_int_value(),
                            b.into_int_value(),
                            "ne_cmp",
                        )
                        .unwrap()
                } else if a.is_float_value() && b.is_float_value() {
                    self.builder
                        .build_float_compare(
                            FloatPredicate::ONE,
                            a.into_float_value(),
                            b.into_float_value(),
                            "ne_cmp",
                        )
                        .unwrap()
                } else {
                    let strcmp = self.get_or_declare_strcmp();
                    let res = self
                        .builder
                        .build_call(strcmp, &[a.into(), b.into()], "cmp")
                        .unwrap();
                    let cmp_val = self.extract_call_value(res).into_int_value();
                    self.builder
                        .build_int_compare(
                            IntPredicate::NE,
                            cmp_val,
                            self.context.i32_type().const_int(0, false),
                            "ne_cmp",
                        )
                        .unwrap()
                };

                let current_fn = self.current_function.unwrap();
                let panic_bb = self
                    .context
                    .append_basic_block(current_fn, "assert_ne_panic");
                let ok_bb = self.context.append_basic_block(current_fn, "assert_ne_ok");

                self.builder
                    .build_conditional_branch(not_equal, ok_bb, panic_bb)
                    .unwrap();

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Assertion failed: values should not be equal!\\n",
                        "assert_ne_fail",
                    )
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .unwrap();
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .unwrap();
                self.builder.build_unreachable().unwrap();

                // OK block
                self.builder.position_at_end(ok_bb);
                Ok(None) // void function
            }

            "assert_true" => {
                // assert_true(condition: Boolean): None - panics if condition is false
                let condition = self.compile_expr(&args[0].node)?;
                let condition_bool = if condition.is_int_value() {
                    let int_val = condition.into_int_value();
                    // Handle both i1 (bool) and i64 (integer) types
                    if int_val.get_type().get_bit_width() == 1 {
                        int_val
                    } else {
                        self.builder
                            .build_int_compare(
                                IntPredicate::NE,
                                int_val,
                                self.context.i64_type().const_int(0, false),
                                "bool_cond",
                            )
                            .unwrap()
                    }
                } else {
                    return Err(CodegenError::new("assert_true requires boolean condition"));
                };

                let current_fn = self.current_function.unwrap();
                let panic_bb = self
                    .context
                    .append_basic_block(current_fn, "assert_true_panic");
                let ok_bb = self
                    .context
                    .append_basic_block(current_fn, "assert_true_ok");

                self.builder
                    .build_conditional_branch(condition_bool, ok_bb, panic_bb)
                    .unwrap();

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Assertion failed: expected true!\\n",
                        "assert_true_fail",
                    )
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .unwrap();
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .unwrap();
                self.builder.build_unreachable().unwrap();

                // OK block
                self.builder.position_at_end(ok_bb);
                Ok(None) // void function
            }

            "assert_false" => {
                // assert_false(condition: Boolean): None - panics if condition is true
                let condition = self.compile_expr(&args[0].node)?;
                let condition_bool = if condition.is_int_value() {
                    let int_val = condition.into_int_value();
                    // Handle both i1 (bool) and i64 (integer) types
                    if int_val.get_type().get_bit_width() == 1 {
                        int_val
                    } else {
                        self.builder
                            .build_int_compare(
                                IntPredicate::NE,
                                int_val,
                                self.context.i64_type().const_int(0, false),
                                "bool_cond",
                            )
                            .unwrap()
                    }
                } else {
                    return Err(CodegenError::new("assert_false requires boolean condition"));
                };

                let current_fn = self.current_function.unwrap();
                let panic_bb = self
                    .context
                    .append_basic_block(current_fn, "assert_false_panic");
                let ok_bb = self
                    .context
                    .append_basic_block(current_fn, "assert_false_ok");

                self.builder
                    .build_conditional_branch(condition_bool, panic_bb, ok_bb)
                    .unwrap();

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Assertion failed: expected false!\\n",
                        "assert_false_fail",
                    )
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .unwrap();
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .unwrap();
                self.builder.build_unreachable().unwrap();

                // OK block
                self.builder.position_at_end(ok_bb);
                Ok(None) // void function (unreachable)
            }

            "fail" => {
                // fail(message: String): None - unconditionally panics
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr("Test failed: ", "fail_prefix")
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .unwrap();

                if !args.is_empty() {
                    let msg = self.compile_expr(&args[0].node)?;
                    self.builder.build_call(printf, &[msg.into()], "").unwrap();
                }

                let newline = self.builder.build_global_string_ptr("\\n", "nl").unwrap();
                self.builder
                    .build_call(printf, &[newline.as_pointer_value().into()], "")
                    .unwrap();

                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .unwrap();
                self.builder.build_unreachable().unwrap();

                Ok(Some(self.context.i64_type().const_int(0, false).into()))
            }

            // Not a stdlib function
            _ => Ok(None),
        }
    }
}
