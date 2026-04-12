use super::*;

impl<'ctx> Codegen<'ctx> {
    pub fn compile_expr(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>> {
        match expr {
            Expr::Literal(lit) => {
                let expr_started_at = Instant::now();
                let result = self.compile_literal(lit);
                CODEGEN_PHASE_TIMING_TOTALS
                    .expr_literal_ns
                    .fetch_add(elapsed_nanos_u64(expr_started_at), Ordering::Relaxed);
                result
            }

            Expr::Ident(name) => {
                let expr_started_at = Instant::now();
                if let Some(var) = self.variables.get(name) {
                    let val = self
                        .builder
                        .build_load(self.llvm_type(&var.ty), var.ptr, name)
                        .map_err(|_| {
                            CodegenError::new(format!("failed to load variable '{}'", name))
                        })?;
                    let result = Ok(val);
                    CODEGEN_PHASE_TIMING_TOTALS
                        .expr_ident_ns
                        .fetch_add(elapsed_nanos_u64(expr_started_at), Ordering::Relaxed);
                    result
                } else {
                    let resolved_name = self.resolve_function_alias(name);
                    let lookup_name = resolved_name.as_str();
                    let result = if let Some((func, ty)) = self.functions.get(lookup_name) {
                        if self.extern_functions.contains(lookup_name) {
                            Err(CodegenError::new(format!(
                                "extern function '{}' cannot be used as a first-class value yet",
                                lookup_name
                            )))
                        } else {
                            // Create a closure struct { fn_ptr, null_env }
                            let struct_ty = self.llvm_type(ty).into_struct_type();
                            let mut closure = struct_ty.get_undef();

                            let fn_ptr = func.as_global_value().as_pointer_value();
                            let null_env =
                                self.context.ptr_type(AddressSpace::default()).const_null();

                            closure = self
                                .builder
                                .build_insert_value(closure, fn_ptr, 0, "fn")
                                .map_err(|_| {
                                    CodegenError::new(format!(
                                        "failed to build closure function pointer for '{}'",
                                        lookup_name
                                    ))
                                })?
                                .into_struct_value();
                            closure = self
                                .builder
                                .build_insert_value(closure, null_env, 1, "env")
                                .map_err(|_| {
                                    CodegenError::new(format!(
                                        "failed to build closure environment for '{}'",
                                        lookup_name
                                    ))
                                })?
                                .into_struct_value();

                            Ok(closure.into())
                        }
                    } else if let Some((enum_name, variant_name)) =
                        self.resolve_import_alias_variant(name)
                    {
                        if let Some(enum_info) = self.enums.get(&enum_name) {
                            if let Some(variant_info) =
                                enum_info.variants.get(&variant_name).cloned()
                            {
                                if variant_info.fields.is_empty() {
                                    self.build_enum_value(&enum_name, &variant_info, &[])
                                } else {
                                    Err(CodegenError::new(format!(
                                        "Enum variant '{}.{}' requires constructor arguments",
                                        enum_name, variant_name
                                    )))
                                }
                            } else {
                                Err(CodegenError::new(format!(
                                    "Unknown variant '{}' for enum '{}'",
                                    variant_name, enum_name
                                )))
                            }
                        } else {
                            Err(CodegenError::new(format!("Unknown enum '{}'", enum_name)))
                        }
                    } else {
                        Err(Self::undefined_variable_error(name))
                    };
                    CODEGEN_PHASE_TIMING_TOTALS
                        .expr_ident_ns
                        .fetch_add(elapsed_nanos_u64(expr_started_at), Ordering::Relaxed);
                    result
                }
            }

            Expr::Binary { op, left, right } => {
                let expr_started_at = Instant::now();
                let result = self.compile_binary(*op, &left.node, &right.node);
                CODEGEN_PHASE_TIMING_TOTALS
                    .expr_binary_ns
                    .fetch_add(elapsed_nanos_u64(expr_started_at), Ordering::Relaxed);
                result
            }

            Expr::Unary { op, expr } => self.compile_unary(*op, &expr.node),

            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                let expr_started_at = Instant::now();
                if !type_args.is_empty() {
                    if let Some(err) = self.explicit_generic_field_access_error(&callee.node, true)
                    {
                        return Err(err);
                    }
                    if let Some(path_parts) = flatten_field_chain(&callee.node) {
                        let full_path = path_parts.join(".");
                        if self
                            .resolve_alias_qualified_codegen_type_name(&full_path)
                            .is_some_and(|resolved| self.classes.contains_key(&resolved))
                        {
                            let ty_source = format!(
                                "{}<{}>",
                                full_path,
                                type_args
                                    .iter()
                                    .map(Self::format_type_string)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            );
                            return self.compile_construct(&ty_source, args);
                        }
                    }
                    if let Some((enum_name, variant_info)) =
                        self.resolve_enum_variant_function_value(&callee.node)
                    {
                        let variant_name = self
                            .enums
                            .get(&enum_name)
                            .and_then(|enum_info| {
                                enum_info.variants.iter().find_map(|(name, info)| {
                                    (info.tag == variant_info.tag
                                        && info.fields == variant_info.fields)
                                        .then(|| name.clone())
                                })
                            })
                            .unwrap_or_else(|| "<unknown>".to_string());
                        return Err(CodegenError::new(format!(
                            "Enum variant '{}.{}' does not accept type arguments",
                            Self::format_diagnostic_name(&enum_name),
                            variant_name
                        )));
                    }
                    if let Expr::Field { object, field } = &callee.node {
                        if let Expr::Ident(owner_name) = &object.node {
                            let owner = self.resolve_module_alias(owner_name);
                            match (owner.as_str(), field.as_str()) {
                                ("Option", "some" | "none") => {
                                    return Err(CodegenError::new(
                                        "Option static methods do not accept explicit type arguments",
                                    ));
                                }
                                ("Result", "ok" | "error") => {
                                    return Err(CodegenError::new(
                                        "Result static methods do not accept explicit type arguments",
                                    ));
                                }
                                _ => {}
                            }
                        }
                    }
                    if let Some(canonical_name) =
                        self.resolve_contextual_function_value_name(&callee.node)
                    {
                        match canonical_name.as_str() {
                            _ if Self::is_supported_builtin_function_name(&canonical_name) => {
                                return Err(CodegenError::new(format!(
                                    "Built-in function '{}' does not accept type arguments",
                                    canonical_name.replace("__", ".")
                                )));
                            }
                            _ => {}
                        }
                    }
                    let _ = self.compile_call(&callee.node, args)?;
                    let result = Err(CodegenError::new(
                        "Explicit generic call code generation is not supported yet".to_string(),
                    ));
                    CODEGEN_PHASE_TIMING_TOTALS
                        .expr_call_ns
                        .fetch_add(elapsed_nanos_u64(expr_started_at), Ordering::Relaxed);
                    return result;
                }
                let result = self.compile_call(&callee.node, args);
                CODEGEN_PHASE_TIMING_TOTALS
                    .expr_call_ns
                    .fetch_add(elapsed_nanos_u64(expr_started_at), Ordering::Relaxed);
                result
            }

            Expr::GenericFunctionValue { callee, .. } => {
                if let Some((enum_name, variant_info)) =
                    self.resolve_enum_variant_function_value(&callee.node)
                {
                    let variant_name = self
                        .enums
                        .get(&enum_name)
                        .and_then(|enum_info| {
                            enum_info.variants.iter().find_map(|(name, info)| {
                                (info.tag == variant_info.tag && info.fields == variant_info.fields)
                                    .then(|| name.clone())
                            })
                        })
                        .unwrap_or_else(|| "<unknown>".to_string());
                    return Err(CodegenError::new(format!(
                        "Enum variant '{}.{}' does not accept type arguments",
                        Self::format_diagnostic_name(&enum_name),
                        variant_name
                    )));
                }
                if let Some(canonical_name) =
                    self.resolve_contextual_function_value_name(&callee.node)
                {
                    if Self::is_supported_builtin_function_name(&canonical_name) {
                        return Err(CodegenError::new(format!(
                            "Built-in function '{}' does not accept type arguments",
                            canonical_name.replace("__", ".")
                        )));
                    }
                }
                if let Some(err) = self.explicit_generic_field_access_error(&callee.node, false) {
                    return Err(err);
                }
                self.compile_expr(&callee.node)?;
                Err(CodegenError::new(
                    "Explicit generic function value should be specialized before code generation"
                        .to_string(),
                ))
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
                        .map_err(|_| CodegenError::new("failed to load 'this' pointer"))?;
                    Ok(val)
                } else {
                    Err(CodegenError::new("'this' not available"))
                }
            }

            Expr::StringInterp(parts) => self.compile_string_interp(parts),

            Expr::Lambda { params, body } => self.compile_lambda(params, body, None),

            Expr::Match { expr, arms } => self.compile_match_expr(&expr.node, arms, None),

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
                let task_ty = self.infer_expr_type(&inner.node, &[]);
                let Type::Task(inner_ty) = task_ty else {
                    return Err(CodegenError::new(format!(
                        "'await' can only be used on Task types, got {}",
                        Self::format_diagnostic_type(&task_ty)
                    )));
                };

                let task = self.compile_expr(&inner.node)?;
                if !task.is_pointer_value() {
                    return Err(CodegenError::new("await expects lowered Task<T> value"));
                }
                self.await_task(task.into_pointer_value(), &inner_ty)
            }

            Expr::AsyncBlock(body) => self.compile_async_block(body, None),

            Expr::Require { condition, message } => {
                // Compile require(condition) as an assert
                let cond = self.compile_condition_expr(&condition.node)?;

                let current_fn = self
                    .current_function
                    .ok_or(CodegenError::new("require outside of function"))?;

                let assert_block = self.context.append_basic_block(current_fn, "require.ok");
                let fail_block = self.context.append_basic_block(current_fn, "require.fail");

                self.builder
                    .build_conditional_branch(cond, assert_block, fail_block)
                    .map_err(|_| CodegenError::new("failed to branch for require()"))?;

                // Fail block - call abort or print message
                self.builder.position_at_end(fail_block);
                if let Some(msg) = message {
                    let msg_ty = self.infer_builtin_argument_type(&msg.node);
                    if !matches!(msg_ty, Type::String) {
                        return Err(CodegenError::new(format!(
                            "require() message must be String, got {}",
                            Self::format_diagnostic_type(&msg_ty)
                        )));
                    }
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
                        .map_err(|_| CodegenError::new("failed to emit require() exit call"))?;
                }
                self.builder
                    .build_unreachable()
                    .map_err(|_| CodegenError::new("failed to terminate require() failure path"))?;

                // Continue in assert block
                self.builder.position_at_end(assert_block);
                Ok(self.context.i8_type().const_int(0, false).into())
            }

            Expr::Range {
                start,
                end,
                inclusive,
            } => {
                let start_val = if let Some(s) = start {
                    self.compile_expr_with_expected_type(&s.node, &Type::Integer)?
                } else {
                    self.context.i64_type().const_int(0, false).into()
                };
                let end_val = if let Some(e) = end {
                    self.compile_expr_with_expected_type(&e.node, &Type::Integer)?
                } else {
                    self.context.i64_type().const_int(0, false).into()
                };
                let step = self.context.i64_type().const_int(1, false).into();
                let end_val = if *inclusive {
                    let incremented = self
                        .builder
                        .build_int_add(
                            end_val.into_int_value(),
                            self.context.i64_type().const_int(1, false),
                            "range_inclusive_end",
                        )
                        .map_err(|_| CodegenError::new("failed to adjust inclusive range end"))?;
                    incremented.into()
                } else {
                    end_val
                };
                Ok(self.create_range(start_val, end_val, step)?.into())
            }

            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => self.compile_if_expr(&condition.node, then_branch, else_branch.as_ref(), None),

            Expr::Block(body) => self.with_variable_scope(|this| {
                let mut result = this.context.i8_type().const_int(0, false).into();
                for stmt in body {
                    if let Stmt::Expr(expr) = &stmt.node {
                        result = this.compile_expr(&expr.node)?;
                    } else {
                        this.compile_stmt(&stmt.node)?;
                    }
                }
                Ok(result)
            }),
        }
    }
}
