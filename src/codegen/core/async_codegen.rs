use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn compile_async_block(
        &mut self,
        body: &[Spanned<Stmt>],
        expected_inner_return_type: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let mut captures: Vec<(String, Type)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut params = std::collections::HashSet::new();
        for stmt in body {
            self.walk_stmt_for_captures(&stmt.node, &mut params, &mut captures, &mut seen);
        }

        let inner_return_type = expected_inner_return_type.cloned().unwrap_or_else(|| {
            self.infer_async_block_return_type(body, expected_inner_return_type)
        });
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
            .map_err(|_| CodegenError::new("failed to allocate async block environment"))?;
        let env_raw =
            self.extract_call_pointer_value(env_alloc, "malloc failed for async block env")?;
        let env_cast = self
            .builder
            .build_pointer_cast(
                env_raw,
                self.context.ptr_type(AddressSpace::default()),
                "async_block_env_cast",
            )
            .map_err(|_| CodegenError::new("failed to cast async block environment"))?;

        for (i, (name, ty)) in captures.iter().enumerate() {
            let var = self.variables.get(name).ok_or_else(|| {
                CodegenError::new(format!("async block capture '{}' not found", name))
            })?;
            let val = self
                .builder
                .build_load(self.llvm_type(ty), var.ptr, name)
                .map_err(|_| {
                    CodegenError::new(format!("failed to load async block capture '{}'", name))
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
                        "async_block_capture",
                    )
                    .map_err(|_| {
                        CodegenError::new(format!(
                            "failed to compute async block capture '{}'",
                            name
                        ))
                    })?
            };
            self.builder.build_store(field_ptr, val).map_err(|_| {
                CodegenError::new(format!("failed to store async block capture '{}'", name))
            })?;
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
                .map_err(|_| CodegenError::new("failed to compute async block task slot"))?
        };
        self.builder
            .build_store(task_slot_ptr, ptr_ty.const_null())
            .map_err(|_| CodegenError::new("failed to initialize async block task slot"))?;

        let saved_function = self.current_function;
        let saved_return_type = self.current_return_type.clone();
        let saved_variables = std::mem::take(&mut self.variables);
        let saved_loop_stack = std::mem::take(&mut self.loop_stack);
        let saved_insert_block = self.builder.get_insert_block();

        let id = self.async_counter;
        self.async_counter += 1;

        let body_name = format!("__arden_async_block_body_{}", id);
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
            .map_err(|_| CodegenError::new("failed to cast async block body environment"))?;

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
                    .map_err(|_| {
                        CodegenError::new(format!(
                            "failed to compute async block body field '{}'",
                            name
                        ))
                    })?
            };
            let loaded = self
                .builder
                .build_load(self.llvm_type(ty), field_ptr, "async_capture_load")
                .map_err(|_| {
                    CodegenError::new(format!("failed to load async block body field '{}'", name))
                })?;
            let alloca = self
                .builder
                .build_alloca(self.llvm_type(ty), name)
                .map_err(|_| {
                    CodegenError::new(format!("failed to allocate async block local '{}'", name))
                })?;
            self.builder.build_store(alloca, loaded).map_err(|_| {
                CodegenError::new(format!("failed to store async block local '{}'", name))
            })?;
            self.variables.insert(
                name.clone(),
                Variable {
                    ptr: alloca,
                    ty: ty.clone(),
                    mutable: false,
                },
            );
        }

        for (index, stmt) in body.iter().enumerate() {
            let is_last = index + 1 == body.len();
            if is_last && self.needs_terminator() {
                if let Stmt::Expr(expr) = &stmt.node {
                    if !matches!(inner_return_type, Type::None) {
                        let inferred_expr_ty = self.infer_expr_type(&expr.node, &[]);
                        let value =
                            self.compile_expr_with_expected_type(&expr.node, &inner_return_type)?;
                        self.reject_incompatible_expected_type_value(
                            &inner_return_type,
                            &inferred_expr_ty,
                            value,
                        )?;
                        self.builder.build_return(Some(&value)).map_err(|_| {
                            CodegenError::new("failed to emit async block return value")
                        })?;
                        continue;
                    }
                }
            }
            self.compile_stmt(&stmt.node)?;
        }
        if self.needs_terminator() {
            if matches!(inner_return_type, Type::None) {
                self.builder
                    .build_return(None)
                    .map_err(|_| CodegenError::new("failed to emit async block return"))?;
            } else {
                self.builder
                    .build_unreachable()
                    .map_err(|_| CodegenError::new("failed to terminate async block"))?;
            }
        }

        let thunk_name = format!("__arden_async_block_thunk_{}", id);
        #[cfg(windows)]
        let thunk_fn_type = self.context.i32_type().fn_type(&[ptr_ty.into()], false);
        #[cfg(not(windows))]
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
            .map_err(|_| CodegenError::new("failed to cast async block thunk environment"))?;
        let body_call = self
            .builder
            .build_call(body_fn, &[thunk_env.into()], "async_block_call")
            .map_err(|_| CodegenError::new("failed to emit async block body call"))?;

        let result_ptr = if matches!(inner_return_type, Type::None) {
            let alloc = self
                .builder
                .build_call(
                    malloc,
                    &[self.context.i64_type().const_int(1, false).into()],
                    "async_block_none_alloc",
                )
                .map_err(|_| CodegenError::new("failed to allocate async block none result"))?;
            let ptr = self
                .extract_call_pointer_value(alloc, "malloc failed for async block none result")?;
            let none_ptr = self
                .builder
                .build_pointer_cast(
                    ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "async_block_none_ptr",
                )
                .map_err(|_| CodegenError::new("failed to cast async block none result pointer"))?;
            self.builder
                .build_store(none_ptr, self.context.i8_type().const_int(0, false))
                .map_err(|_| CodegenError::new("failed to initialize async block none result"))?;
            ptr
        } else {
            let ret_ty = self.llvm_type(&inner_return_type);
            let size = ret_ty
                .size_of()
                .ok_or_else(|| CodegenError::new("failed to compute async block result size"))?;
            let alloc = self
                .builder
                .build_call(malloc, &[size.into()], "async_block_alloc")
                .map_err(|_| CodegenError::new("failed to allocate async block result"))?;
            let ptr =
                self.extract_call_pointer_value(alloc, "malloc failed for async block result")?;
            let typed_ptr = self
                .builder
                .build_pointer_cast(
                    ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "async_block_result_ptr",
                )
                .map_err(|_| CodegenError::new("failed to cast async block result pointer"))?;
            let result_val = self.extract_call_value_with_context(
                body_call,
                "async block body should return value for non-None Task",
            )?;
            self.builder
                .build_store(typed_ptr, result_val)
                .map_err(|_| CodegenError::new("failed to store async block result"))?;
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
                .map_err(|_| CodegenError::new("failed to compute async block task field"))?
        };
        let task_ptr = self
            .builder
            .build_load(ptr_ty, task_field_ptr, "async_block_task_ptr")
            .map_err(|_| CodegenError::new("failed to load async block task pointer"))?
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
                .map_err(|_| CodegenError::new("failed to compute async block result field"))?
        };
        self.builder
            .build_store(result_field, result_ptr)
            .map_err(|_| CodegenError::new("failed to store async block task result"))?;
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
                .map_err(|_| CodegenError::new("failed to compute async block completed field"))?
        };
        self.build_atomic_bool_store(
            completed_field,
            self.context.i8_type().const_int(1, false),
            AtomicOrdering::Release,
        )?;
        #[cfg(windows)]
        self.builder
            .build_return(Some(&self.context.i32_type().const_int(0, false)))
            .map_err(|_| CodegenError::new("failed to emit async block thunk Windows return"))?;
        #[cfg(not(windows))]
        self.builder
            .build_return(Some(&result_ptr))
            .map_err(|_| CodegenError::new("failed to emit async block thunk return"))?;

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

    pub(super) fn compile_async_function(&mut self, func: &FunctionDecl) -> Result<()> {
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

        // 1) Compile body function: __arden_async_body__*
        self.current_function = Some(body);
        self.current_return_type = Some(inner_return_type.clone());
        self.variables.clear();
        self.loop_stack.clear();
        self.reset_current_generic_bounds();
        self.extend_current_generic_bounds(&func.generic_params);
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
            .map_err(|_| CodegenError::new("failed to cast async environment pointer"))?;

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
                    .map_err(|_| {
                        CodegenError::new(format!(
                            "failed to compute async parameter field '{}'",
                            param.name
                        ))
                    })?
            };
            let loaded = self
                .builder
                .build_load(self.llvm_type(&param.ty), field_ptr, &param.name)
                .map_err(|_| {
                    CodegenError::new(format!("failed to load async parameter '{}'", param.name))
                })?;
            let alloca = self
                .builder
                .build_alloca(self.llvm_type(&param.ty), &param.name)
                .map_err(|_| {
                    CodegenError::new(format!(
                        "failed to allocate async parameter '{}'",
                        param.name
                    ))
                })?;
            self.builder.build_store(alloca, loaded).map_err(|_| {
                CodegenError::new(format!("failed to store async parameter '{}'", param.name))
            })?;
            self.variables.insert(
                param.name.clone(),
                Variable {
                    ptr: alloca,
                    ty: param.ty.clone(),
                    mutable: param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                },
            );
        }

        for stmt in &func.body {
            self.compile_stmt(&stmt.node)?;
        }
        if self.needs_terminator() {
            if matches!(inner_return_type, Type::None) {
                self.builder
                    .build_return(None)
                    .map_err(|_| CodegenError::new("failed to emit async body return"))?;
            } else {
                self.builder
                    .build_unreachable()
                    .map_err(|_| CodegenError::new("failed to terminate async body"))?;
            }
        }

        // 2) Compile thunk: __arden_async_thunk__*
        self.current_function = Some(thunk);
        self.current_return_type = None;
        self.variables.clear();
        self.loop_stack.clear();
        self.reset_current_generic_bounds();
        let thunk_entry = self.context.append_basic_block(thunk, "entry");
        self.builder.position_at_end(thunk_entry);

        let thunk_env = thunk
            .get_nth_param(0)
            .ok_or_else(|| CodegenError::new("Async thunk missing env parameter"))?
            .into_pointer_value();
        let thunk_env_cast = self
            .builder
            .build_pointer_cast(thunk_env, ptr_type, "async_thunk_env_cast")
            .map_err(|_| CodegenError::new("failed to cast async thunk environment"))?;

        let body_call = self
            .builder
            .build_call(body, &[thunk_env.into()], "async_body_call")
            .map_err(|_| CodegenError::new("failed to emit async body call"))?;

        let malloc = self.get_or_declare_malloc();
        let result_storage = if matches!(inner_return_type, Type::None) {
            let raw = self
                .builder
                .build_call(
                    malloc,
                    &[self.context.i64_type().const_int(1, false).into()],
                    "async_none_alloc",
                )
                .map_err(|_| CodegenError::new("failed to allocate async Task<None> result"))?;
            let ptr =
                self.extract_call_pointer_value(raw, "malloc failed for async Task<None> result")?;
            let none_ptr = self
                .builder
                .build_pointer_cast(
                    ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "async_none_ptr",
                )
                .map_err(|_| CodegenError::new("failed to cast async Task<None> result pointer"))?;
            self.builder
                .build_store(none_ptr, self.context.i8_type().const_int(0, false))
                .map_err(|_| CodegenError::new("failed to initialize async Task<None> result"))?;
            ptr
        } else {
            let ret_ty = self.llvm_type(&inner_return_type);
            let size = ret_ty
                .size_of()
                .ok_or_else(|| CodegenError::new("failed to compute async result size"))?;
            let raw = self
                .builder
                .build_call(malloc, &[size.into()], "async_result_alloc")
                .map_err(|_| CodegenError::new("failed to allocate async result storage"))?;
            let ptr = self.extract_call_pointer_value(raw, "malloc failed for async result")?;
            let typed_ptr = self
                .builder
                .build_pointer_cast(
                    ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "async_result_ptr",
                )
                .map_err(|_| CodegenError::new("failed to cast async result pointer"))?;
            let result = self.extract_call_value_with_context(
                body_call,
                "async body should return value for non-None Task",
            )?;
            self.builder
                .build_store(typed_ptr, result)
                .map_err(|_| CodegenError::new("failed to store async result value"))?;
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
                .map_err(|_| CodegenError::new("failed to compute async task field pointer"))?
        };
        let task_ptr = self
            .builder
            .build_load(ptr_type, task_field_ptr, "async_task_ptr")
            .map_err(|_| CodegenError::new("failed to load async task pointer"))?
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
                .map_err(|_| CodegenError::new("failed to compute async task result pointer"))?
        };
        self.builder
            .build_store(result_field, result_storage)
            .map_err(|_| CodegenError::new("failed to store async task result"))?;
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
                .map_err(|_| CodegenError::new("failed to compute async task completed pointer"))?
        };
        self.build_atomic_bool_store(
            completed_field,
            self.context.i8_type().const_int(1, false),
            AtomicOrdering::Release,
        )?;
        #[cfg(windows)]
        self.builder
            .build_return(Some(&self.context.i32_type().const_int(0, false)))
            .map_err(|_| CodegenError::new("failed to emit async thunk Windows return"))?;
        #[cfg(not(windows))]
        self.builder
            .build_return(Some(&result_storage))
            .map_err(|_| CodegenError::new("failed to emit async thunk return"))?;

        // 3) Compile public wrapper: function name(...)
        self.current_function = Some(wrapper);
        self.current_return_type = Some(Type::Task(Box::new(inner_return_type.clone())));
        self.variables.clear();
        self.loop_stack.clear();
        self.reset_current_generic_bounds();
        let wrapper_entry = self.context.append_basic_block(wrapper, "entry");
        self.builder.position_at_end(wrapper_entry);

        let env_size = env_type
            .size_of()
            .ok_or_else(|| CodegenError::new("failed to compute async environment size"))?;
        let env_alloc = self
            .builder
            .build_call(malloc, &[env_size.into()], "async_env_alloc")
            .map_err(|_| CodegenError::new("failed to allocate async environment"))?;
        let env_raw_ptr =
            self.extract_call_pointer_value(env_alloc, "malloc failed for async environment")?;
        let env_cast = self
            .builder
            .build_pointer_cast(
                env_raw_ptr,
                self.context.ptr_type(AddressSpace::default()),
                "async_env_store",
            )
            .map_err(|_| CodegenError::new("failed to cast async environment storage"))?;

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
                    .map_err(|_| {
                        CodegenError::new(format!(
                            "failed to compute async wrapper field '{}'",
                            param.name
                        ))
                    })?
            };
            self.builder
                .build_store(field_ptr, param_val)
                .map_err(|_| {
                    CodegenError::new(format!(
                        "failed to store async wrapper field '{}'",
                        param.name
                    ))
                })?;
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
                .map_err(|_| CodegenError::new("failed to compute async task slot pointer"))?
        };
        self.builder
            .build_store(task_slot_ptr, ptr_type.const_null())
            .map_err(|_| CodegenError::new("failed to initialize async task slot"))?;

        let task = self.create_task(
            thunk.as_global_value().as_pointer_value(),
            env_raw_ptr,
            task_slot_ptr,
        )?;
        self.builder
            .build_return(Some(&task))
            .map_err(|_| CodegenError::new("failed to emit async wrapper return"))?;

        self.current_function = None;
        self.current_return_type = None;
        self.reset_current_generic_bounds();
        Ok(())
    }

    pub(super) fn compile_task_method(
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
        let object_ty = self.infer_object_type(object);
        let task_raw = if matches!(object_ty, Some(Type::Ref(_)) | Some(Type::MutRef(_))) {
            self.compile_deref(object)?
        } else {
            self.compile_expr(object)?
        };
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
            .map_err(|_| CodegenError::new("failed to cast task pointer for method call"))?;

        let i32_ty = self.context.i32_type();
        let zero = i32_ty.const_int(0, false);
        let thread_idx = i32_ty.const_int(0, false);
        let result_idx = i32_ty.const_int(1, false);
        let done_idx = i32_ty.const_int(2, false);
        let completed_idx = i32_ty.const_int(3, false);
        let done_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done_ptr")
                .map_err(|_| CodegenError::new("failed to access task done field"))?
        };
        let completed_field = unsafe {
            self.builder
                .build_gep(
                    task_ty,
                    task_ptr,
                    &[zero, completed_idx],
                    "task_completed_ptr",
                )
                .map_err(|_| CodegenError::new("failed to access task completed field"))?
        };
        let result_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, result_idx], "task_result_field")
                .map_err(|_| CodegenError::new("failed to access task result field"))?
        };

        match method {
            "is_done" => {
                let done_val = self
                    .builder
                    .build_load(self.context.i8_type(), done_field, "task_done")
                    .map_err(|_| CodegenError::new("failed to load task done flag"))?
                    .into_int_value();
                let done_bool = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        done_val,
                        self.context.i8_type().const_zero(),
                        "task_done_bool",
                    )
                    .map_err(|_| CodegenError::new("failed to compare task done flag"))?;
                let completed_val = self.build_atomic_bool_load(
                    completed_field,
                    "task_completed",
                    AtomicOrdering::Acquire,
                )?;
                Ok(self
                    .builder
                    .build_or(done_bool, completed_val, "task_is_done")
                    .map_err(|_| CodegenError::new("failed to combine task done state"))?
                    .into())
            }
            "cancel" => {
                let done_val = self
                    .builder
                    .build_load(self.context.i8_type(), done_field, "task_done")
                    .map_err(|_| CodegenError::new("failed to load task done flag"))?
                    .into_int_value();
                let done_bool = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        done_val,
                        self.context.i8_type().const_zero(),
                        "task_done_bool",
                    )
                    .map_err(|_| CodegenError::new("failed to compare task done flag"))?;
                let completed_val = self.build_atomic_bool_load(
                    completed_field,
                    "task_completed",
                    AtomicOrdering::Acquire,
                )?;
                let already_done = self
                    .builder
                    .build_or(done_bool, completed_val, "task_already_done")
                    .map_err(|_| CodegenError::new("failed to compute task cancel readiness"))?;

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Task.cancel used outside function"))?;
                let cancel_bb = self.context.append_basic_block(current_fn, "task_cancel");
                let merge_bb = self
                    .context
                    .append_basic_block(current_fn, "task_cancel_merge");
                self.builder
                    .build_conditional_branch(already_done, merge_bb, cancel_bb)
                    .map_err(|_| CodegenError::new("failed to branch for task cancellation"))?;

                self.builder.position_at_end(cancel_bb);
                let thread_field = unsafe {
                    self.builder
                        .build_gep(task_ty, task_ptr, &[zero, thread_idx], "task_thread_ptr")
                        .map_err(|_| CodegenError::new("failed to access task thread field"))?
                };
                let thread_id = self
                    .builder
                    .build_load(self.context.i64_type(), thread_field, "task_thread_id")
                    .map_err(|_| CodegenError::new("failed to load task thread identifier"))?;

                #[cfg(windows)]
                {
                    let close_fn = self.get_or_declare_close_handle_win();
                    let handle = self
                        .builder
                        .build_int_to_ptr(
                            thread_id.into_int_value(),
                            self.context.ptr_type(AddressSpace::default()),
                            "task_cancel_handle",
                        )
                        .map_err(|_| CodegenError::new("failed to convert task thread handle"))?;
                    // Do not forcefully kill Windows threads with TerminateThread():
                    // it can leave the process in an inconsistent state and has been
                    // observed to hang follow-up runtime tests. Treat cancel as a
                    // detached cancellation signal instead: publish a safe default
                    // result, mark the task done, and close our handle reference.
                    self.builder
                        .build_call(close_fn, &[handle.into()], "")
                        .map_err(|_| {
                            CodegenError::new("failed to emit CloseHandle for canceled task")
                        })?;
                    self.builder
                        .build_store(thread_field, self.context.i64_type().const_zero())
                        .map_err(|_| {
                            CodegenError::new("failed to clear canceled task thread handle")
                        })?;
                }
                #[cfg(not(windows))]
                {
                    let pthread_cancel = self.get_or_declare_pthread_cancel();
                    let pthread_t_ty = self.libc_ulong_type();
                    let pthread_thread_id = self
                        .builder
                        .build_int_cast(
                            thread_id.into_int_value(),
                            pthread_t_ty,
                            "task_cancel_pthread_t",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to cast task thread id to pthread_t")
                        })?;
                    self.builder
                        .build_call(pthread_cancel, &[pthread_thread_id.into()], "task_cancel")
                        .map_err(|_| CodegenError::new("failed to emit pthread_cancel for task"))?;
                }

                let done_field = unsafe {
                    self.builder
                        .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done_ptr")
                        .map_err(|_| CodegenError::new("failed to access task done field"))?
                };
                self.builder
                    .build_store(done_field, self.context.i8_type().const_int(1, false))
                    .map_err(|_| CodegenError::new("failed to store canceled task done flag"))?;
                self.build_atomic_bool_store(
                    completed_field,
                    self.context.i8_type().const_int(1, false),
                    AtomicOrdering::Release,
                )?;

                // Store a safe default payload so await after cancel stays valid for heap-backed
                // values like user classes and ranges instead of returning a null object pointer.
                let malloc = self.get_or_declare_malloc();
                let llvm_inner = self.llvm_type(inner);
                let size = llvm_inner
                    .size_of()
                    .ok_or_else(|| CodegenError::new("failed to size Task inner type"))?;
                let raw = self
                    .builder
                    .build_call(malloc, &[size.into()], "task_cancel_alloc")
                    .map_err(|_| CodegenError::new("failed to allocate canceled task result"))?;
                let result_ptr = self.extract_call_pointer_value(
                    raw,
                    "malloc failed while creating canceled task value",
                )?;
                let typed_ptr = self
                    .builder
                    .build_pointer_cast(
                        result_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "task_cancel_result_ptr",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to cast canceled task result pointer")
                    })?;
                let default_value = self.create_default_value_for_type(inner)?;
                self.builder
                    .build_store(typed_ptr, default_value)
                    .map_err(|_| {
                        CodegenError::new("failed to store canceled task default value")
                    })?;
                self.builder
                    .build_store(result_field, result_ptr)
                    .map_err(|_| {
                        CodegenError::new("failed to store canceled task result pointer")
                    })?;
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| CodegenError::new("failed to branch after task cancellation"))?;

                self.builder.position_at_end(merge_bb);
                Ok(self.context.i8_type().const_int(0, false).into())
            }
            "await_timeout" => {
                if matches!(
                    TypeChecker::eval_numeric_const_expr(&args[0].node),
                    Some(NumericConst::Integer(value)) if value < 0
                ) {
                    return Err(CodegenError::new(
                        "Task.await_timeout() timeout must be non-negative",
                    ));
                }
                let ms_ty = self.infer_builtin_argument_type(&args[0].node);
                if !matches!(ms_ty, Type::Integer) {
                    return Err(CodegenError::new(
                        "Task.await_timeout(ms) requires Integer milliseconds",
                    ));
                }
                let ms = self.compile_expr_with_expected_type(&args[0].node, &ms_ty)?;
                if !ms.is_int_value() {
                    return Err(CodegenError::new(
                        "Task.await_timeout(ms) requires Integer milliseconds",
                    ));
                }
                let ms_i64 = self
                    .builder
                    .build_int_cast(ms.into_int_value(), self.context.i64_type(), "timeout_ms")
                    .map_err(|_| CodegenError::new("failed to cast task timeout to i64"))?;

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Task.await_timeout used outside function"))?;
                let timeout_valid_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_valid");
                let timeout_invalid_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_invalid");
                let timeout_negative = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        ms_i64,
                        self.context.i64_type().const_zero(),
                        "task_timeout_negative",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compare task timeout against zero")
                    })?;
                self.builder
                    .build_conditional_branch(
                        timeout_negative,
                        timeout_invalid_bb,
                        timeout_valid_bb,
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to branch for task timeout validation")
                    })?;

                self.builder.position_at_end(timeout_invalid_bb);
                self.emit_runtime_error(
                    "Task.await_timeout() timeout must be non-negative",
                    "task_timeout_negative_runtime_error",
                )?;

                self.builder.position_at_end(timeout_valid_bb);

                let done_field = unsafe {
                    self.builder
                        .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done_ptr")
                        .map_err(|_| CodegenError::new("failed to access task done field"))?
                };
                let completed_field = unsafe {
                    self.builder
                        .build_gep(
                            task_ty,
                            task_ptr,
                            &[zero, completed_idx],
                            "task_completed_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to access task completed field"))?
                };
                let done_val = self
                    .builder
                    .build_load(self.context.i8_type(), done_field, "task_done")
                    .map_err(|_| CodegenError::new("failed to load task done flag"))?
                    .into_int_value();
                let done_ready = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        done_val,
                        self.context.i8_type().const_zero(),
                        "task_done_ready",
                    )
                    .map_err(|_| CodegenError::new("failed to compare task done flag"))?;

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
                        .map_err(|_| CodegenError::new("failed to access task thread field"))?
                };
                let thread_id = self
                    .builder
                    .build_load(self.context.i64_type(), thread_field, "task_thread_id")
                    .map_err(|_| CodegenError::new("failed to load task thread identifier"))?;
                let join_result_ptr = self
                    .builder
                    .build_alloca(
                        self.context.ptr_type(AddressSpace::default()),
                        "timed_join_out",
                    )
                    .map_err(|_| CodegenError::new("failed to allocate timed join result slot"))?;
                self.builder
                    .build_store(
                        join_result_ptr,
                        self.context.ptr_type(AddressSpace::default()).const_null(),
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to initialize timed join result slot")
                    })?;
                let iter_ptr = self
                    .builder
                    .build_alloca(self.context.i64_type(), "task_timeout_iter")
                    .map_err(|_| {
                        CodegenError::new("failed to allocate task timeout loop counter")
                    })?;
                self.builder
                    .build_store(iter_ptr, self.context.i64_type().const_zero())
                    .map_err(|_| {
                        CodegenError::new("failed to initialize task timeout loop counter")
                    })?;
                let max_iters = ms_i64;

                self.builder
                    .build_conditional_branch(done_ready, done_bb, check_bb)
                    .map_err(|_| CodegenError::new("failed to branch into task timeout flow"))?;

                // done -> Some(result)
                self.builder.position_at_end(done_bb);
                let existing_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        result_field,
                        "task_existing_result",
                    )
                    .map_err(|_| CodegenError::new("failed to load existing task result pointer"))?
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
                        .map_err(|_| {
                            CodegenError::new("failed to cast completed task result pointer")
                        })?;
                    self.builder
                        .build_load(inner_llvm, typed_ptr, "task_done_value")
                        .map_err(|_| CodegenError::new("failed to load completed task value"))?
                };
                let done_some = self.create_option_some(done_value)?;
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch from completed task timeout path")
                    })?;

                self.builder.position_at_end(check_bb);
                let completed_val = self.build_atomic_bool_load(
                    completed_field,
                    "task_completed",
                    AtomicOrdering::Acquire,
                )?;
                self.builder
                    .build_conditional_branch(completed_val, join_bb, loop_bb)
                    .map_err(|_| CodegenError::new("failed to branch on task completion state"))?;

                self.builder.position_at_end(loop_bb);
                let iter_val = self
                    .builder
                    .build_load(self.context.i64_type(), iter_ptr, "task_timeout_iter_val")
                    .map_err(|_| CodegenError::new("failed to load task timeout loop counter"))?
                    .into_int_value();
                let timed_out = self
                    .builder
                    .build_int_compare(
                        IntPredicate::UGE,
                        iter_val,
                        max_iters,
                        "task_timeout_reached",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compare task timeout loop counter")
                    })?;
                self.builder
                    .build_conditional_branch(timed_out, timeout_bb, sleep_bb)
                    .map_err(|_| CodegenError::new("failed to branch task timeout loop"))?;

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
                        .map_err(|_| {
                            CodegenError::new("failed to emit Sleep during task timeout")
                        })?;
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
                        .map_err(|_| {
                            CodegenError::new("failed to emit usleep during task timeout")
                        })?;
                }
                let next_iter = self
                    .builder
                    .build_int_add(
                        iter_val,
                        self.context.i64_type().const_int(1, false),
                        "task_timeout_next_iter",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to increment task timeout loop counter")
                    })?;
                self.builder
                    .build_store(iter_ptr, next_iter)
                    .map_err(|_| CodegenError::new("failed to store task timeout loop counter"))?;
                self.builder
                    .build_unconditional_branch(check_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch back to task timeout check")
                    })?;

                self.builder.position_at_end(join_bb);
                #[cfg(windows)]
                let joined_ptr = {
                    let wait_fn = self.get_or_declare_wait_for_single_object_win();
                    let close_fn = self.get_or_declare_close_handle_win();
                    let handle = self
                        .builder
                        .build_int_to_ptr(
                            thread_id.into_int_value(),
                            self.context.ptr_type(AddressSpace::default()),
                            "timed_join_handle",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to convert timed join thread handle")
                        })?;
                    self.builder
                        .build_call(
                            wait_fn,
                            &[
                                handle.into(),
                                self.context.i32_type().const_all_ones().into(),
                            ],
                            "timed_join_finalize",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to emit WaitForSingleObject for task join")
                        })?;
                    self.builder
                        .build_call(close_fn, &[handle.into()], "")
                        .map_err(|_| {
                            CodegenError::new("failed to emit CloseHandle after timed join")
                        })?;
                    self.builder
                        .build_store(thread_field, self.context.i64_type().const_zero())
                        .map_err(|_| {
                            CodegenError::new("failed to clear joined task thread handle")
                        })?;
                    self.builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            result_field,
                            "joined_result",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to load joined task result pointer")
                        })?
                        .into_pointer_value()
                };
                #[cfg(not(windows))]
                let joined_ptr = {
                    let pthread_join = self.get_or_declare_pthread_join();
                    let pthread_t_ty = self.libc_ulong_type();
                    let pthread_thread_id = self
                        .builder
                        .build_int_cast(
                            thread_id.into_int_value(),
                            pthread_t_ty,
                            "timed_join_pthread_t",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to cast timed join thread id to pthread_t")
                        })?;
                    self.builder
                        .build_call(
                            pthread_join,
                            &[pthread_thread_id.into(), join_result_ptr.into()],
                            "timed_join_finalize",
                        )
                        .map_err(|_| CodegenError::new("failed to emit pthread_join for task"))?;
                    self.builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            join_result_ptr,
                            "joined_result",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to load pthread join result pointer")
                        })?
                        .into_pointer_value()
                };
                self.builder
                    .build_store(result_field, joined_ptr)
                    .map_err(|_| CodegenError::new("failed to store joined task result pointer"))?;
                self.builder
                    .build_store(done_field, self.context.i8_type().const_int(1, false))
                    .map_err(|_| CodegenError::new("failed to store joined task done flag"))?;
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
                        .map_err(|_| {
                            CodegenError::new("failed to cast joined task result pointer")
                        })?;
                    self.builder
                        .build_load(inner_llvm, typed_ptr, "joined_value")
                        .map_err(|_| CodegenError::new("failed to load joined task value"))?
                };
                let succ_some = self.create_option_some(succ_value)?;
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch from joined task timeout path")
                    })?;

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
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| CodegenError::new("failed to branch from timed out task path"))?;

                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(
                        self.llvm_type(&Type::Option(Box::new(inner.clone()))),
                        "timeout_phi",
                    )
                    .map_err(|_| CodegenError::new("failed to build task timeout result phi"))?;
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
}
