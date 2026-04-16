use super::*;

impl<'ctx> Codegen<'ctx> {
    pub fn compile_for(
        &mut self,
        var: &str,
        var_type: Option<&Type>,
        iterable: &Spanned<Expr>,
        body: &Block,
    ) -> Result<()> {
        let saved_variables = self.variables.clone();
        let func = self
            .current_function
            .ok_or_else(|| CodegenError::new("for loop used outside function"))?;
        let iterable_ty = self.infer_builtin_argument_type(&iterable.node);
        let deref_iterable_ty = self.deref_codegen_type(&iterable_ty).clone();

        if let Type::List(inner) = deref_iterable_ty.clone() {
            let iter_ty = var_type.cloned().unwrap_or((*inner).clone());
            let var_alloca = self
                .builder
                .build_alloca(self.llvm_type(&iter_ty), var)
                .map_err(|_| {
                    CodegenError::new(format!("failed to allocate for-loop variable '{}'", var))
                })?;
            self.variables.insert(
                var.to_string(),
                Variable {
                    ptr: var_alloca,
                    ty: iter_ty.clone(),
                    mutable: false,
                },
            );

            let list_value = self.compile_expr_with_expected_type(&iterable.node, &iterable_ty)?;
            let list_ptr = self.materialize_value_pointer_for_type(
                list_value,
                &deref_iterable_ty,
                "for_list_tmp",
            )?;
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

            let idx_alloca = self
                .builder
                .build_alloca(i64_type, "for_list_idx")
                .map_err(|_| CodegenError::new("failed to allocate list loop index"))?;
            self.builder
                .build_store(idx_alloca, zero_i64)
                .map_err(|_| CodegenError::new("failed to initialize list loop index"))?;

            let len_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                self.builder
                    .build_gep(
                        list_type.as_basic_type_enum(),
                        list_ptr,
                        &[i32_type.const_int(0, false), i32_type.const_int(1, false)],
                        "list_len_ptr",
                    )
                    .map_err(|_| CodegenError::new("failed to compute list loop length pointer"))?
            };
            let data_ptr_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                self.builder
                    .build_gep(
                        list_type.as_basic_type_enum(),
                        list_ptr,
                        &[i32_type.const_int(0, false), i32_type.const_int(2, false)],
                        "list_data_ptr_ptr",
                    )
                    .map_err(|_| CodegenError::new("failed to compute list loop data pointer"))?
            };

            let cond_bb = self.context.append_basic_block(func, "for_list.cond");
            let body_bb = self.context.append_basic_block(func, "for_list.body");
            let inc_bb = self.context.append_basic_block(func, "for_list.inc");
            let after_bb = self.context.append_basic_block(func, "for_list.after");
            self.builder
                .build_unconditional_branch(cond_bb)
                .map_err(|_| CodegenError::new("failed to enter list for-loop"))?;

            self.builder.position_at_end(cond_bb);
            let idx_val = self
                .builder
                .build_load(i64_type, idx_alloca, "for_list_idx_val")
                .map_err(|_| CodegenError::new("failed to load list loop index"))?
                .into_int_value();
            let len_val = self
                .builder
                .build_load(i64_type, len_ptr, "for_list_len")
                .map_err(|_| CodegenError::new("failed to load list loop length"))?
                .into_int_value();
            let cond = self
                .builder
                .build_int_compare(IntPredicate::SLT, idx_val, len_val, "for_list_cmp")
                .map_err(|_| CodegenError::new("failed to compare list loop bounds"))?;
            self.builder
                .build_conditional_branch(cond, body_bb, after_bb)
                .map_err(|_| CodegenError::new("failed to branch in list for-loop"))?;

            self.builder.position_at_end(body_bb);
            let data_ptr = self
                .builder
                .build_load(
                    self.context.ptr_type(AddressSpace::default()),
                    data_ptr_ptr,
                    "for_list_data",
                )
                .map_err(|_| CodegenError::new("failed to load list loop data"))?
                .into_pointer_value();
            let byte_offset = self
                .builder
                .build_int_mul(
                    idx_val,
                    i64_type.const_int(elem_size, false),
                    "for_list_off",
                )
                .map_err(|_| CodegenError::new("failed to compute list loop offset"))?;
            let elem_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                self.builder
                    .build_gep(
                        self.context.i8_type(),
                        data_ptr,
                        &[byte_offset],
                        "for_list_elem_ptr",
                    )
                    .map_err(|_| CodegenError::new("failed to compute list element pointer"))?
            };
            let typed_ptr = self
                .builder
                .build_pointer_cast(
                    elem_ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "for_list_typed_ptr",
                )
                .map_err(|_| CodegenError::new("failed to cast list element pointer"))?;
            let elem_val = self
                .builder
                .build_load(elem_llvm, typed_ptr, "for_list_elem")
                .map_err(|_| CodegenError::new("failed to load list loop element"))?;
            let iter_val =
                self.adapt_for_loop_binding_value(elem_val, &inner, &iter_ty, "for_list_iter")?;
            self.builder
                .build_store(var_alloca, iter_val)
                .map_err(|_| CodegenError::new("failed to store list loop binding"))?;

            self.loop_stack.push(LoopContext {
                loop_block: inc_bb,
                after_block: after_bb,
            });
            self.with_variable_scope(|this| {
                for stmt in body {
                    this.compile_stmt(&stmt.node)?;
                }
                Ok(())
            })?;
            self.loop_stack.pop();
            if self.needs_terminator() {
                self.builder
                    .build_unconditional_branch(inc_bb)
                    .map_err(|_| CodegenError::new("failed to branch to list loop increment"))?;
            }

            self.builder.position_at_end(inc_bb);
            let next_idx = self
                .builder
                .build_int_add(idx_val, one_i64, "for_list_next")
                .map_err(|_| CodegenError::new("failed to increment list loop index"))?;
            self.builder
                .build_store(idx_alloca, next_idx)
                .map_err(|_| CodegenError::new("failed to store list loop index"))?;
            self.builder
                .build_unconditional_branch(cond_bb)
                .map_err(|_| CodegenError::new("failed to continue list for-loop"))?;

            self.builder.position_at_end(after_bb);
            self.variables = saved_variables;
            return Ok(());
        }

        if !matches!(iterable.node, Expr::Range { .. })
            && matches!(deref_iterable_ty, Type::Range(_))
        {
            let Type::Range(inner) = deref_iterable_ty.clone() else {
                return Err(CodegenError::new(
                    "internal error: expected range type for range iteration",
                ));
            };
            let iter_ty = var_type.cloned().unwrap_or((*inner).clone());
            let var_alloca = self
                .builder
                .build_alloca(self.llvm_type(&iter_ty), var)
                .map_err(|_| {
                    CodegenError::new(format!("failed to allocate range loop variable '{}'", var))
                })?;
            self.variables.insert(
                var.to_string(),
                Variable {
                    ptr: var_alloca,
                    ty: iter_ty.clone(),
                    mutable: false,
                },
            );

            let range_alloca = self
                .builder
                .build_alloca(self.llvm_type(&iterable_ty), &format!("{var}_range"))
                .map_err(|_| CodegenError::new("failed to allocate range loop storage"))?;
            let range_value = if matches!(iterable_ty, Type::Ref(_) | Type::MutRef(_)) {
                self.compile_deref(&iterable.node)?
            } else {
                self.compile_expr_with_expected_type(&iterable.node, &iterable_ty)?
            };
            self.builder
                .build_store(range_alloca, range_value)
                .map_err(|_| CodegenError::new("failed to store range loop value"))?;

            let cond_bb = self.context.append_basic_block(func, "for_range_obj.cond");
            let body_bb = self.context.append_basic_block(func, "for_range_obj.body");
            let inc_bb = self.context.append_basic_block(func, "for_range_obj.inc");
            let after_bb = self.context.append_basic_block(func, "for_range_obj.after");

            self.builder
                .build_unconditional_branch(cond_bb)
                .map_err(|_| CodegenError::new("failed to enter range-object for-loop"))?;

            self.builder.position_at_end(cond_bb);
            let loaded_range = self
                .builder
                .build_load(
                    self.llvm_type(&iterable_ty),
                    range_alloca,
                    "for_range_obj_val",
                )
                .map_err(|_| CodegenError::new("failed to load range object for has_next"))?;
            let has_next = self
                .compile_range_method_on_value(loaded_range, &iterable_ty, "has_next", &[])?
                .into_int_value();
            self.builder
                .build_conditional_branch(has_next, body_bb, after_bb)
                .map_err(|_| CodegenError::new("failed to branch in range-object for-loop"))?;

            self.builder.position_at_end(body_bb);
            let loaded_range = self
                .builder
                .build_load(
                    self.llvm_type(&iterable_ty),
                    range_alloca,
                    "for_range_obj_next",
                )
                .map_err(|_| CodegenError::new("failed to load range object for next"))?;
            let next_value =
                self.compile_range_method_on_value(loaded_range, &iterable_ty, "next", &[])?;
            let iter_value =
                self.adapt_for_loop_binding_value(next_value, &inner, &iter_ty, "for_range_obj")?;
            self.builder
                .build_store(var_alloca, iter_value)
                .map_err(|_| CodegenError::new("failed to store range loop binding"))?;

            self.loop_stack.push(LoopContext {
                loop_block: inc_bb,
                after_block: after_bb,
            });
            self.with_variable_scope(|this| {
                for stmt in body {
                    this.compile_stmt(&stmt.node)?;
                }
                Ok(())
            })?;
            self.loop_stack.pop();
            if self.needs_terminator() {
                self.builder
                    .build_unconditional_branch(inc_bb)
                    .map_err(|_| CodegenError::new("failed to branch to range loop increment"))?;
            }

            self.builder.position_at_end(inc_bb);
            self.builder
                .build_unconditional_branch(cond_bb)
                .map_err(|_| CodegenError::new("failed to continue range-object for-loop"))?;

            self.builder.position_at_end(after_bb);
            self.variables = saved_variables;
            return Ok(());
        }

        if matches!(deref_iterable_ty, Type::String) {
            let iter_ty = var_type.cloned().unwrap_or(Type::Char);
            let var_alloca = self
                .builder
                .build_alloca(self.llvm_type(&iter_ty), var)
                .map_err(|_| {
                    CodegenError::new(format!("failed to allocate string loop variable '{}'", var))
                })?;
            self.variables.insert(
                var.to_string(),
                Variable {
                    ptr: var_alloca,
                    ty: iter_ty.clone(),
                    mutable: false,
                },
            );

            let string_value = if matches!(iterable_ty, Type::Ref(_) | Type::MutRef(_)) {
                self.compile_deref(&iterable.node)?.into_pointer_value()
            } else {
                self.compile_expr_with_expected_type(&iterable.node, &iterable_ty)?
                    .into_pointer_value()
            };
            let len_alloca = self
                .builder
                .build_alloca(self.context.i64_type(), &format!("{var}_string_len"))
                .map_err(|_| CodegenError::new("failed to allocate string loop length slot"))?;
            let idx_alloca = self
                .builder
                .build_alloca(self.context.i64_type(), &format!("{var}_string_idx"))
                .map_err(|_| CodegenError::new("failed to allocate string loop index slot"))?;
            let length = self.compile_utf8_string_length_runtime(string_value)?;
            self.builder
                .build_store(len_alloca, length)
                .map_err(|_| CodegenError::new("failed to store string loop length"))?;
            self.builder
                .build_store(idx_alloca, self.context.i64_type().const_zero())
                .map_err(|_| CodegenError::new("failed to initialize string loop index"))?;

            let cond_bb = self.context.append_basic_block(func, "for_string.cond");
            let body_bb = self.context.append_basic_block(func, "for_string.body");
            let inc_bb = self.context.append_basic_block(func, "for_string.inc");
            let after_bb = self.context.append_basic_block(func, "for_string.after");
            self.builder
                .build_unconditional_branch(cond_bb)
                .map_err(|_| CodegenError::new("failed to enter string for-loop"))?;

            self.builder.position_at_end(cond_bb);
            let idx_val = self
                .builder
                .build_load(self.context.i64_type(), idx_alloca, "for_string_idx")
                .map_err(|_| CodegenError::new("failed to load string loop index"))?
                .into_int_value();
            let len_val = self
                .builder
                .build_load(self.context.i64_type(), len_alloca, "for_string_len")
                .map_err(|_| CodegenError::new("failed to load string loop length"))?
                .into_int_value();
            let cond = self
                .builder
                .build_int_compare(IntPredicate::SLT, idx_val, len_val, "for_string_cmp")
                .map_err(|_| CodegenError::new("failed to compare string loop bounds"))?;
            self.builder
                .build_conditional_branch(cond, body_bb, after_bb)
                .map_err(|_| CodegenError::new("failed to branch in string for-loop"))?;

            self.builder.position_at_end(body_bb);
            let ch = self.compile_utf8_string_index_runtime(string_value, idx_val)?;
            let iter_val =
                self.adapt_for_loop_binding_value(ch, &Type::Char, &iter_ty, "for_string_iter")?;
            self.builder
                .build_store(var_alloca, iter_val)
                .map_err(|_| CodegenError::new("failed to store string loop binding"))?;

            self.loop_stack.push(LoopContext {
                loop_block: inc_bb,
                after_block: after_bb,
            });
            self.with_variable_scope(|this| {
                for stmt in body {
                    this.compile_stmt(&stmt.node)?;
                }
                Ok(())
            })?;
            self.loop_stack.pop();
            if self.needs_terminator() {
                self.builder
                    .build_unconditional_branch(inc_bb)
                    .map_err(|_| CodegenError::new("failed to branch to string loop increment"))?;
            }

            self.builder.position_at_end(inc_bb);
            let next_idx = self
                .builder
                .build_int_add(
                    idx_val,
                    self.context.i64_type().const_int(1, false),
                    "for_string_next",
                )
                .map_err(|_| CodegenError::new("failed to increment string loop index"))?;
            self.builder
                .build_store(idx_alloca, next_idx)
                .map_err(|_| CodegenError::new("failed to store string loop index"))?;
            self.builder
                .build_unconditional_branch(cond_bb)
                .map_err(|_| CodegenError::new("failed to continue string for-loop"))?;

            self.builder.position_at_end(after_bb);
            self.variables = saved_variables;
            return Ok(());
        }

        let ty = var_type.cloned().unwrap_or(Type::Integer);
        let var_alloca = self
            .builder
            .build_alloca(self.llvm_type(&ty), var)
            .map_err(|_| {
                CodegenError::new(format!(
                    "failed to allocate integer for-loop variable '{}'",
                    var
                ))
            })?;
        let counter_alloca = self
            .builder
            .build_alloca(self.context.i64_type(), &format!("{var}_counter"))
            .map_err(|_| CodegenError::new("failed to allocate integer for-loop counter"))?;

        // Default range values
        let mut start_val: BasicValueEnum<'ctx> =
            self.context.i64_type().const_int(0, false).into();
        let mut end_val: BasicValueEnum<'ctx> = self.context.i64_type().const_int(0, false).into();
        let mut inclusive = false;

        match &iterable.node {
            Expr::Range {
                start,
                end,
                inclusive: inc,
            } => {
                if let Some(s) = start {
                    start_val = self.compile_integer_iteration_bound(&s.node)?.into();
                }
                if let Some(e) = end {
                    end_val = self.compile_integer_iteration_bound(&e.node)?.into();
                }
                inclusive = *inc;
            }
            _ => {
                // Treat as 0..N where N is the expression value
                end_val = self.compile_integer_iteration_bound(&iterable.node)?.into();
            }
        }

        let start_val = start_val.into_int_value();
        let end_val = end_val.into_int_value();
        self.builder
            .build_store(counter_alloca, start_val)
            .map_err(|_| CodegenError::new("failed to initialize integer for-loop counter"))?;

        self.variables.insert(
            var.to_string(),
            Variable {
                ptr: var_alloca,
                ty: ty.clone(),
                mutable: false,
            },
        );

        let cond_bb = self.context.append_basic_block(func, "for.cond");
        let body_bb = self.context.append_basic_block(func, "for.body");
        let inc_bb = self.context.append_basic_block(func, "for.inc");
        let after_bb = self.context.append_basic_block(func, "for.after");

        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::new("failed to enter integer for-loop"))?;

        // Condition
        self.builder.position_at_end(cond_bb);
        let current = self
            .builder
            .build_load(
                self.context.i64_type(),
                counter_alloca,
                &format!("{var}_current"),
            )
            .map_err(|_| CodegenError::new("failed to load integer for-loop counter"))?
            .into_int_value();

        let cond = if inclusive {
            self.builder
                .build_int_compare(IntPredicate::SLE, current, end_val, "cmp")
                .map_err(|_| CodegenError::new("failed to compare inclusive for-loop bounds"))?
        } else {
            self.builder
                .build_int_compare(IntPredicate::SLT, current, end_val, "cmp")
                .map_err(|_| CodegenError::new("failed to compare for-loop bounds"))?
        };

        self.builder
            .build_conditional_branch(cond, body_bb, after_bb)
            .map_err(|_| CodegenError::new("failed to branch in integer for-loop"))?;

        // Body
        self.builder.position_at_end(body_bb);
        let iter_val =
            self.adapt_for_loop_binding_value(current.into(), &Type::Integer, &ty, "for_range")?;
        self.builder
            .build_store(var_alloca, iter_val)
            .map_err(|_| CodegenError::new("failed to store integer for-loop binding"))?;
        self.loop_stack.push(LoopContext {
            loop_block: inc_bb,
            after_block: after_bb,
        });
        self.with_variable_scope(|this| {
            for stmt in body {
                this.compile_stmt(&stmt.node)?;
            }
            Ok(())
        })?;
        self.loop_stack.pop();
        if self.needs_terminator() {
            self.builder
                .build_unconditional_branch(inc_bb)
                .map_err(|_| CodegenError::new("failed to branch to integer loop increment"))?;
        }

        // Increment
        self.builder.position_at_end(inc_bb);
        let current = self
            .builder
            .build_load(
                self.context.i64_type(),
                counter_alloca,
                &format!("{var}_counter"),
            )
            .map_err(|_| CodegenError::new("failed to reload integer for-loop counter"))?
            .into_int_value();
        let one = self.context.i64_type().const_int(1, false);
        let next = self
            .builder
            .build_int_add(current, one, "inc")
            .map_err(|_| CodegenError::new("failed to increment integer for-loop counter"))?;
        self.builder
            .build_store(counter_alloca, next)
            .map_err(|_| CodegenError::new("failed to store integer for-loop counter"))?;
        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::new("failed to continue integer for-loop"))?;

        self.builder.position_at_end(after_bb);
        self.variables = saved_variables;
        Ok(())
    }

    pub fn compile_match_stmt(&mut self, expr: &Spanned<Expr>, arms: &[MatchArm]) -> Result<()> {
        let imported_variant = |this: &Self, name: &str| -> Option<(String, String, bool)> {
            this.resolve_pattern_variant_alias(name)
        };
        let imported_unit_variant = |this: &Self, name: &str| -> Option<(String, String, u8)> {
            let (enum_name, variant_name, is_unit) = this.resolve_pattern_variant_alias(name)?;
            if !is_unit {
                return None;
            }
            let variant_info = this.enums.get(&enum_name)?.variants.get(&variant_name)?;
            Some((enum_name, variant_name, variant_info.tag))
        };
        let match_ty = self.infer_builtin_argument_type(&expr.node);
        let option_inner_ty = match &match_ty {
            Type::Option(inner) => Some((**inner).clone()),
            _ => None,
        };
        let is_option_match = option_inner_ty.is_some();
        let result_inner_tys = match &match_ty {
            Type::Result(ok, err) => Some(((**ok).clone(), (**err).clone())),
            _ => None,
        };
        let is_result_match = result_inner_tys.is_some();
        let enum_match_name = match &match_ty {
            Type::Named(name) if self.enums.contains_key(name) => Some(name.clone()),
            _ => None,
        };

        for arm in arms {
            fn pattern_variant_leaf(name: &str) -> &str {
                name.rsplit('.').next().unwrap_or(name)
            }

            match &arm.pattern {
                Pattern::Literal(lit) => {
                    let pattern_ty = self.infer_expr_type(&Expr::Literal(lit.clone()), &[]);
                    if self
                        .common_compatible_codegen_type(&match_ty, &pattern_ty)
                        .is_none()
                    {
                        return Err(CodegenError::new(format!(
                            "Pattern type mismatch: expected {}, found {}",
                            Self::format_diagnostic_type(&match_ty),
                            Self::format_diagnostic_type(&pattern_ty)
                        )));
                    }
                }
                Pattern::Variant(variant_name, _) => {
                    let resolved_variant = if !variant_name.contains('.') {
                        imported_variant(self, variant_name)
                    } else {
                        None
                    };
                    let variant_leaf = resolved_variant
                        .as_ref()
                        .map(|(_, resolved_variant_name, _)| resolved_variant_name.as_str())
                        .unwrap_or_else(|| pattern_variant_leaf(variant_name));
                    let resolved_enum_name =
                        resolved_variant.as_ref().map(|(enum_name, _, _)| enum_name);
                    let matches_builtin_variant = (is_option_match
                        && matches!(variant_leaf, "Some" | "None"))
                        || (is_result_match && matches!(variant_leaf, "Ok" | "Error"));
                    let enum_variant_exists = resolved_enum_name
                        .or(enum_match_name.as_ref())
                        .and_then(|enum_name| self.enums.get(enum_name))
                        .is_some_and(|enum_info| enum_info.variants.contains_key(variant_leaf));
                    if !matches_builtin_variant && !enum_variant_exists {
                        return Err(CodegenError::new(format!(
                            "Cannot match variant {} on type {}",
                            variant_leaf,
                            Self::format_diagnostic_type(&match_ty)
                        )));
                    }
                }
                _ => {}
            }
        }

        let val = self.compile_expr_with_expected_type(&expr.node, &match_ty)?;
        let func = self
            .current_function
            .ok_or_else(|| CodegenError::new("match statement used outside function"))?;
        let merge_bb = self.context.append_basic_block(func, "match.merge");

        let mut dispatch_bb = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::new("match statement insert block missing"))?;

        for arm in arms {
            let arm_bb = self.context.append_basic_block(func, "match.arm");
            let next_bb = self.context.append_basic_block(func, "match.next");

            self.builder.position_at_end(dispatch_bb);
            fn pattern_variant_leaf(name: &str) -> &str {
                name.rsplit('.').next().unwrap_or(name)
            }

            match &arm.pattern {
                Pattern::Wildcard => {
                    self.builder
                        .build_unconditional_branch(arm_bb)
                        .map_err(|_| CodegenError::new("failed to branch to wildcard match arm"))?;
                }
                Pattern::Ident(name) => {
                    if let Some((enum_name, variant_name, variant_tag)) =
                        imported_unit_variant(self, name)
                    {
                        let is_builtin_variant = (is_option_match
                            && matches!(variant_name.as_str(), "Some" | "None"))
                            || (is_result_match && matches!(variant_name.as_str(), "Ok" | "Error"));
                        let enum_matches = enum_match_name
                            .as_ref()
                            .is_some_and(|expected_enum| expected_enum == &enum_name);
                        if is_builtin_variant || enum_matches {
                            let tag = self
                                .builder
                                .build_extract_value(val.into_struct_value(), 0, "tag")
                                .map_err(|_| {
                                    CodegenError::new(
                                        "failed to extract identifier-pattern variant tag",
                                    )
                                })?
                                .into_int_value();
                            let cond = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::EQ,
                                    tag,
                                    self.context.i8_type().const_int(variant_tag as u64, false),
                                    "match_ident_variant_eq",
                                )
                                .map_err(|_| {
                                    CodegenError::new(
                                        "failed to compare identifier-pattern variant tag",
                                    )
                                })?;
                            self.builder
                                .build_conditional_branch(cond, arm_bb, next_bb)
                                .map_err(|_| {
                                    CodegenError::new(
                                        "failed to branch for identifier-pattern variant arm",
                                    )
                                })?;
                        } else {
                            self.builder
                                .build_unconditional_branch(next_bb)
                                .map_err(|_| {
                                    CodegenError::new(
                                        "failed to skip non-matching identifier-pattern arm",
                                    )
                                })?;
                        }
                    } else {
                        self.builder
                            .build_unconditional_branch(arm_bb)
                            .map_err(|_| {
                                CodegenError::new("failed to branch to identifier match arm")
                            })?;
                    }
                }
                Pattern::Literal(lit) => {
                    let pattern_ty = self.infer_expr_type(&Expr::Literal(lit.clone()), &[]);
                    if self
                        .common_compatible_codegen_type(&match_ty, &pattern_ty)
                        .is_none()
                    {
                        return Err(CodegenError::new(format!(
                            "Pattern type mismatch: expected {}, found {}",
                            Self::format_diagnostic_type(&match_ty),
                            Self::format_diagnostic_type(&pattern_ty)
                        )));
                    }
                    let pattern_val = self.compile_literal(lit)?;
                    let cond = if val.is_float_value() || pattern_val.is_float_value() {
                        let match_val = if val.is_float_value() {
                            val.into_float_value()
                        } else {
                            self.builder
                                .build_signed_int_to_float(
                                    val.into_int_value(),
                                    self.context.f64_type(),
                                    "match_lit_lf",
                                )
                                .map_err(|_| {
                                    CodegenError::new(
                                        "failed to cast match-stmt lhs literal to float",
                                    )
                                })?
                        };
                        let pattern_float = if pattern_val.is_float_value() {
                            pattern_val.into_float_value()
                        } else {
                            self.builder
                                .build_signed_int_to_float(
                                    pattern_val.into_int_value(),
                                    self.context.f64_type(),
                                    "match_lit_rf",
                                )
                                .map_err(|_| {
                                    CodegenError::new(
                                        "failed to cast match-stmt rhs literal to float",
                                    )
                                })?
                        };
                        self.builder
                            .build_float_compare(
                                FloatPredicate::OEQ,
                                match_val,
                                pattern_float,
                                "match_float_eq",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compare float match-stmt literals")
                            })?
                    } else if val.is_int_value() && pattern_val.is_int_value() {
                        self.builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                val.into_int_value(),
                                pattern_val.into_int_value(),
                                "match_lit_eq",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compare integer match-stmt literals")
                            })?
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
                            .map_err(|_| {
                                CodegenError::new("failed to emit strcmp for match statement")
                            })?;
                        let cmp_val = self.extract_call_value(cmp)?.into_int_value();
                        self.builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                cmp_val,
                                self.context.i32_type().const_int(0, false),
                                "match_str_eq",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compare string match-stmt literals")
                            })?
                    } else {
                        self.context.bool_type().const_int(0, false)
                    };
                    self.builder
                        .build_conditional_branch(cond, arm_bb, next_bb)
                        .map_err(|_| CodegenError::new("failed to branch for literal match arm"))?;
                }
                Pattern::Variant(variant_name, _) => {
                    let resolved_variant = if !variant_name.contains('.') {
                        imported_variant(self, variant_name)
                    } else {
                        None
                    };
                    let variant_leaf = resolved_variant
                        .as_ref()
                        .map(|(_, resolved_variant_name, _)| resolved_variant_name.as_str())
                        .unwrap_or_else(|| pattern_variant_leaf(variant_name));
                    let resolved_enum_name =
                        resolved_variant.as_ref().map(|(enum_name, _, _)| enum_name);
                    let matches_builtin_variant = (is_option_match
                        && matches!(variant_leaf, "Some" | "None"))
                        || (is_result_match && matches!(variant_leaf, "Ok" | "Error"));
                    let enum_variant_info = resolved_enum_name
                        .or(enum_match_name.as_ref())
                        .and_then(|enum_name| {
                            self.enums
                                .get(enum_name)
                                .and_then(|enum_info| enum_info.variants.get(variant_leaf))
                                .map(|variant_info| (enum_name, variant_info))
                        });
                    // Built-in Option / Result matching
                    if matches_builtin_variant {
                        let expected_tag = match variant_leaf {
                            "Some" | "Ok" => 1u64,
                            _ => 0u64,
                        };
                        let tag = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 0, "tag")
                            .map_err(|_| {
                                CodegenError::new("failed to extract builtin variant tag")
                            })?
                            .into_int_value();
                        let cond = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                tag,
                                self.context.i8_type().const_int(expected_tag, false),
                                "match_variant_eq",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compare builtin variant tag")
                            })?;
                        self.builder
                            .build_conditional_branch(cond, arm_bb, next_bb)
                            .map_err(|_| {
                                CodegenError::new("failed to branch for builtin variant arm")
                            })?;
                    } else if let Some((_, variant_info)) = enum_variant_info {
                        let tag = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 0, "tag")
                            .map_err(|_| CodegenError::new("failed to extract enum variant tag"))?
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
                            .map_err(|_| CodegenError::new("failed to compare enum variant tag"))?;
                        self.builder
                            .build_conditional_branch(cond, arm_bb, next_bb)
                            .map_err(|_| {
                                CodegenError::new("failed to branch for enum variant arm")
                            })?;
                    } else {
                        return Err(CodegenError::new(format!(
                            "Cannot match variant {} on type {}",
                            variant_leaf,
                            Self::format_diagnostic_type(&match_ty)
                        )));
                    }
                }
            }

            self.builder.position_at_end(arm_bb);
            self.with_variable_scope(|this| {
                match &arm.pattern {
                    Pattern::Ident(binding) if imported_unit_variant(this, binding).is_none() => {
                        let alloca =
                            this.builder
                                .build_alloca(val.get_type(), binding)
                                .map_err(|_| {
                                    CodegenError::new(format!(
                                        "failed to allocate match binding '{}'",
                                        binding
                                    ))
                                })?;
                        this.builder.build_store(alloca, val).map_err(|_| {
                            CodegenError::new(format!(
                                "failed to store match binding '{}'",
                                binding
                            ))
                        })?;
                        this.variables.insert(
                            binding.clone(),
                            Variable {
                                ptr: alloca,
                                ty: match_ty.clone(),
                                mutable: false,
                            },
                        );
                    }
                    Pattern::Ident(_) => {}
                    Pattern::Variant(variant_name, bindings) => {
                        let resolved_variant = if !variant_name.contains('.') {
                            imported_variant(this, variant_name)
                        } else {
                            None
                        };
                        let variant_leaf = resolved_variant
                            .as_ref()
                            .map(|(_, resolved_variant_name, _)| resolved_variant_name.as_str())
                            .unwrap_or_else(|| pattern_variant_leaf(variant_name));
                        let resolved_enum_name =
                            resolved_variant.as_ref().map(|(enum_name, _, _)| enum_name);
                        if is_option_match && variant_leaf == "Some" && !bindings.is_empty() {
                            let inner = this
                                .builder
                                .build_extract_value(val.into_struct_value(), 1, "some_inner")
                                .map_err(|_| CodegenError::new("failed to extract Some payload"))?;
                            let alloca = this
                                .builder
                                .build_alloca(inner.get_type(), &bindings[0])
                                .map_err(|_| {
                                    CodegenError::new(format!(
                                        "failed to allocate Some binding '{}'",
                                        bindings[0]
                                    ))
                                })?;
                            this.builder.build_store(alloca, inner).map_err(|_| {
                                CodegenError::new(format!(
                                    "failed to store Some binding '{}'",
                                    bindings[0]
                                ))
                            })?;
                            this.variables.insert(
                                bindings[0].clone(),
                                Variable {
                                    ptr: alloca,
                                    ty: option_inner_ty.clone().unwrap_or(Type::Integer),
                                    mutable: false,
                                },
                            );
                        } else if is_result_match && variant_leaf == "Ok" && !bindings.is_empty() {
                            let inner = this
                                .builder
                                .build_extract_value(val.into_struct_value(), 1, "ok_inner")
                                .map_err(|_| CodegenError::new("failed to extract Ok payload"))?;
                            let alloca = this
                                .builder
                                .build_alloca(inner.get_type(), &bindings[0])
                                .map_err(|_| {
                                    CodegenError::new(format!(
                                        "failed to allocate Ok binding '{}'",
                                        bindings[0]
                                    ))
                                })?;
                            this.builder.build_store(alloca, inner).map_err(|_| {
                                CodegenError::new(format!(
                                    "failed to store Ok binding '{}'",
                                    bindings[0]
                                ))
                            })?;
                            this.variables.insert(
                                bindings[0].clone(),
                                Variable {
                                    ptr: alloca,
                                    ty: result_inner_tys
                                        .as_ref()
                                        .map(|(ok, _)| ok.clone())
                                        .unwrap_or(Type::Integer),
                                    mutable: false,
                                },
                            );
                        } else if is_result_match && variant_leaf == "Error" && !bindings.is_empty()
                        {
                            let inner = this
                                .builder
                                .build_extract_value(val.into_struct_value(), 2, "err_inner")
                                .map_err(|_| {
                                    CodegenError::new("failed to extract Error payload")
                                })?;
                            let alloca = this
                                .builder
                                .build_alloca(inner.get_type(), &bindings[0])
                                .map_err(|_| {
                                    CodegenError::new(format!(
                                        "failed to allocate Error binding '{}'",
                                        bindings[0]
                                    ))
                                })?;
                            this.builder.build_store(alloca, inner).map_err(|_| {
                                CodegenError::new(format!(
                                    "failed to store Error binding '{}'",
                                    bindings[0]
                                ))
                            })?;
                            this.variables.insert(
                                bindings[0].clone(),
                                Variable {
                                    ptr: alloca,
                                    ty: result_inner_tys
                                        .as_ref()
                                        .map(|(_, err)| err.clone())
                                        .unwrap_or(Type::String),
                                    mutable: false,
                                },
                            );
                        } else if let Some(enum_name) =
                            resolved_enum_name.or(enum_match_name.as_ref())
                        {
                            if let Some(enum_info) = this.enums.get(enum_name) {
                                if let Some(variant_info) = enum_info.variants.get(variant_leaf) {
                                    for (idx, binding) in bindings.iter().enumerate() {
                                        if let Some(field_ty) = variant_info.fields.get(idx) {
                                            let raw = this
                                                .builder
                                                .build_extract_value(
                                                    val.into_struct_value(),
                                                    (idx + 1) as u32,
                                                    "enum_payload_raw",
                                                )
                                                .map_err(|_| {
                                                    CodegenError::new(format!(
                                                        "failed to extract enum payload for '{}'",
                                                        binding
                                                    ))
                                                })?
                                                .into_int_value();
                                            let decoded =
                                                this.decode_enum_payload(raw, field_ty)?;
                                            let alloca = this
                                                .builder
                                                .build_alloca(decoded.get_type(), binding)
                                                .map_err(|_| {
                                                    CodegenError::new(format!(
                                                        "failed to allocate enum binding '{}'",
                                                        binding
                                                    ))
                                                })?;
                                            this.builder.build_store(alloca, decoded).map_err(
                                                |_| {
                                                    CodegenError::new(format!(
                                                        "failed to store enum binding '{}'",
                                                        binding
                                                    ))
                                                },
                                            )?;
                                            this.variables.insert(
                                                binding.clone(),
                                                Variable {
                                                    ptr: alloca,
                                                    ty: field_ty.clone(),
                                                    mutable: false,
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
                    this.compile_stmt(&stmt.node)?;
                }
                Ok(())
            })?;
            if self.needs_terminator() {
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| CodegenError::new("failed to branch to match merge block"))?;
            }

            dispatch_bb = next_bb;
            self.builder.position_at_end(dispatch_bb);
        }

        if self.needs_terminator() {
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|_| CodegenError::new("failed to branch to final match merge block"))?;
        }

        self.builder.position_at_end(merge_bb);
        Ok(())
    }
}
