use super::*;

impl<'ctx> Codegen<'ctx> {
    pub fn compile_call(
        &mut self,
        callee: &Expr,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let ident_name = match callee {
            Expr::Ident(name) => Some(name.as_str()),
            _ => None,
        };
        let ident_is_local_var = ident_name.is_some_and(|name| self.variables.contains_key(name));
        let resolved_ident = ident_name
            .filter(|_| !ident_is_local_var)
            .map(|name| self.resolve_function_alias(name));

        // Check for built-in functions
        if let Expr::Ident(name) = callee {
            let builtin_name = resolved_ident.as_deref().unwrap_or(name.as_str());
            if !ident_is_local_var
                && self
                    .resolve_alias_qualified_codegen_type_name(builtin_name)
                    .is_some_and(|resolved| self.classes.contains_key(&resolved))
            {
                return self.compile_construct(builtin_name, args);
            }
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
                let call_expr = Expr::Call {
                    callee: Box::new(Spanned::new(callee.clone(), Span::default())),
                    args: args.to_vec(),
                    type_args: Vec::new(),
                };
                let inferred_expr_ty = self.infer_expr_type(&call_expr, &[]);
                if let Some(canonical_builtin) =
                    builtin_exact_import_alias_canonical(&format!("{}.{}", type_name, field))
                {
                    match canonical_builtin {
                        "Option__some" => {
                            if args.len() != 1 {
                                return Err(CodegenError::new(
                                    "Option.some() requires exactly 1 argument",
                                ));
                            }
                            if let Type::Option(inner_ty) = &inferred_expr_ty {
                                let val = self.compile_expr_for_concrete_class_payload(
                                    &args[0].node,
                                    inner_ty,
                                )?;
                                return self.create_option_some_typed(val, inner_ty);
                            }
                            let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                            let val =
                                self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                            return self.create_option_some(val);
                        }
                        "Option__none" => {
                            if !args.is_empty() {
                                return Err(CodegenError::new(format!(
                                    "Option.none() expects 0 argument(s), got {}",
                                    args.len()
                                )));
                            }
                            if let Type::Option(inner_ty) = &inferred_expr_ty {
                                return self.create_option_none_typed(inner_ty);
                            }
                            return self.create_option_none();
                        }
                        "Result__ok" => {
                            if args.len() != 1 {
                                return Err(CodegenError::new(
                                    "Result.ok() requires exactly 1 argument",
                                ));
                            }
                            if let Type::Result(ok_ty, err_ty) = &inferred_expr_ty {
                                let val = self.compile_expr_for_concrete_class_payload(
                                    &args[0].node,
                                    ok_ty,
                                )?;
                                return self.create_result_ok_typed(val, ok_ty, err_ty);
                            }
                            let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                            let val =
                                self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                            return self.create_result_ok(val);
                        }
                        "Result__error" => {
                            if args.len() != 1 {
                                return Err(CodegenError::new(
                                    "Result.error() requires exactly 1 argument",
                                ));
                            }
                            if let Type::Result(ok_ty, err_ty) = &inferred_expr_ty {
                                let val = self.compile_expr_for_concrete_class_payload(
                                    &args[0].node,
                                    err_ty,
                                )?;
                                return self.create_result_error_typed(val, ok_ty, err_ty);
                            }
                            let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                            let val =
                                self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                            return self.create_result_error(val);
                        }
                        _ => {}
                    }
                }
                match (type_name.as_str(), field.as_str()) {
                    ("Option", "some") => {
                        if args.len() != 1 {
                            return Err(CodegenError::new(
                                "Option.some() requires exactly 1 argument",
                            ));
                        }
                        if let Type::Option(inner_ty) = &inferred_expr_ty {
                            let val = self
                                .compile_expr_for_concrete_class_payload(&args[0].node, inner_ty)?;
                            return self.create_option_some_typed(val, inner_ty);
                        }
                        let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                        let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                        return self.create_option_some(val);
                    }
                    ("Option", "none") => {
                        if !args.is_empty() {
                            return Err(CodegenError::new(format!(
                                "Option.none() expects 0 argument(s), got {}",
                                args.len()
                            )));
                        }
                        if let Type::Option(inner_ty) = &inferred_expr_ty {
                            return self.create_option_none_typed(inner_ty);
                        }
                        return self.create_option_none();
                    }
                    ("Result", "ok") => {
                        if args.len() != 1 {
                            return Err(CodegenError::new(
                                "Result.ok() requires exactly 1 argument",
                            ));
                        }
                        if let Type::Result(ok_ty, err_ty) = &inferred_expr_ty {
                            let val =
                                self.compile_expr_for_concrete_class_payload(&args[0].node, ok_ty)?;
                            return self.create_result_ok_typed(val, ok_ty, err_ty);
                        }
                        let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                        let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                        return self.create_result_ok(val);
                    }
                    ("Result", "error") => {
                        if args.len() != 1 {
                            return Err(CodegenError::new(
                                "Result.error() requires exactly 1 argument",
                            ));
                        }
                        if let Type::Result(ok_ty, err_ty) = &inferred_expr_ty {
                            let val = self
                                .compile_expr_for_concrete_class_payload(&args[0].node, err_ty)?;
                            return self.create_result_error_typed(val, ok_ty, err_ty);
                        }
                        let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                        let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                        return self.create_result_error(val);
                    }
                    _ => {}
                }
            }
        }

        // Check for enum variant constructors and module-qualified functions.
        if let Expr::Field { object, field } = callee {
            if let Expr::Ident(owner_name) = &object.node {
                let resolved_owner = self
                    .resolve_alias_qualified_codegen_type_name(owner_name)
                    .unwrap_or_else(|| self.resolve_module_alias(owner_name));
                let type_path = format!("{}.{}", resolved_owner, field);
                if let Some(resolved_type_name) =
                    self.resolve_alias_qualified_codegen_type_name(&type_path)
                {
                    if self.classes.contains_key(&resolved_type_name) {
                        return self.compile_construct(&type_path, args);
                    }
                }
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
                        for (arg, expected_ty) in args.iter().zip(variant_info.fields.iter()) {
                            values.push(
                                self.compile_expr_for_concrete_class_payload(
                                    &arg.node,
                                    expected_ty,
                                )?,
                            );
                        }
                        return self.build_enum_value(&resolved_owner, &variant_info, &values);
                    }
                }

                // Module dot syntax: Module.func(...) -> Module__func(...)
                let mangled = format!("{}__{}", resolved_owner, field);
                if let Some((func, func_ty)) = self.functions.get(&mangled).cloned() {
                    if let Type::Function(params, _) = &func_ty {
                        if args.len() != params.len() {
                            return Err(Self::function_call_arity_error(&func_ty, args.len()));
                        }
                    }
                    let mut compiled_args: Vec<BasicValueEnum> = Vec::with_capacity(args.len() + 1);
                    compiled_args.push(
                        self.context
                            .ptr_type(AddressSpace::default())
                            .const_null()
                            .into(),
                    );
                    if let Type::Function(params, _) = &func_ty {
                        for (arg, param_ty) in args.iter().zip(params.iter()) {
                            compiled_args.push(
                                self.compile_expr_for_concrete_class_payload(&arg.node, param_ty)?,
                            );
                        }
                    } else {
                        for arg in args {
                            compiled_args.push(self.compile_expr(&arg.node)?);
                        }
                    }
                    let args_meta: Vec<BasicMetadataValueEnum> =
                        compiled_args.iter().map(|a| (*a).into()).collect();
                    let call = self
                        .builder
                        .build_call(func, &args_meta, "call")
                        .map_err(|_| {
                            CodegenError::new("failed to emit module-qualified function call")
                        })?;
                    return match call.try_as_basic_value() {
                        ValueKind::Basic(val) => Ok(val),
                        ValueKind::Instruction(_) => {
                            Ok(self.context.i8_type().const_int(0, false).into())
                        }
                    };
                }
                if let Some(candidate) = self.resolve_wildcard_import_module_function_candidate(
                    owner_name,
                    std::slice::from_ref(field),
                ) {
                    if let Some((func, func_ty)) = self.functions.get(&candidate).cloned() {
                        if let Type::Function(params, _) = &func_ty {
                            if args.len() != params.len() {
                                return Err(Self::function_call_arity_error(&func_ty, args.len()));
                            }
                        }
                        let mut compiled_args: Vec<BasicValueEnum> =
                            Vec::with_capacity(args.len() + 1);
                        compiled_args.push(
                            self.context
                                .ptr_type(AddressSpace::default())
                                .const_null()
                                .into(),
                        );
                        if let Type::Function(params, _) = &func_ty {
                            for (arg, param_ty) in args.iter().zip(params.iter()) {
                                compiled_args.push(self.compile_expr_for_concrete_class_payload(
                                    &arg.node, param_ty,
                                )?);
                            }
                        } else {
                            for arg in args {
                                compiled_args.push(self.compile_expr(&arg.node)?);
                            }
                        }
                        let args_meta: Vec<BasicMetadataValueEnum> =
                            compiled_args.iter().map(|a| (*a).into()).collect();
                        let call =
                            self.builder
                                .build_call(func, &args_meta, "call")
                                .map_err(|_| {
                                    CodegenError::new(
                                        "failed to emit wildcard-import function call",
                                    )
                                })?;
                        return match call.try_as_basic_value() {
                            ValueKind::Basic(val) => Ok(val),
                            ValueKind::Instruction(_) => {
                                Ok(self.context.i8_type().const_int(0, false).into())
                            }
                        };
                    }
                }
            }
        }

        // Nested module-style calls: A.X.f(...) -> A__X__f(...)
        if let Some(path_parts) = flatten_field_chain(callee) {
            if path_parts.len() >= 3 {
                let full_path = path_parts.join(".");
                if let Some(resolved_type_name) =
                    self.resolve_alias_qualified_codegen_type_name(&full_path)
                {
                    if self.classes.contains_key(&resolved_type_name) {
                        return self.compile_construct(&full_path, args);
                    }
                }

                let owner_source = path_parts[..path_parts.len() - 1].join(".");
                let variant_name = path_parts.last().cloned().unwrap_or_default();
                if let Some(resolved_owner) =
                    self.resolve_alias_qualified_codegen_type_name(&owner_source)
                {
                    if let Some(enum_info) = self.enums.get(&resolved_owner) {
                        if let Some(variant_info) = enum_info.variants.get(&variant_name).cloned() {
                            if args.len() != variant_info.fields.len() {
                                return Err(CodegenError::new(format!(
                                    "Enum variant '{}.{}' expects {} argument(s), got {}",
                                    owner_source,
                                    variant_name,
                                    variant_info.fields.len(),
                                    args.len()
                                )));
                            }
                            let mut values = Vec::with_capacity(args.len());
                            for (arg, expected_ty) in args.iter().zip(variant_info.fields.iter()) {
                                values.push(self.compile_expr_for_concrete_class_payload(
                                    &arg.node,
                                    expected_ty,
                                )?);
                            }
                            return self.build_enum_value(&resolved_owner, &variant_info, &values);
                        }
                    }
                }

                let candidate = path_parts.join("__");
                if let Some((func, func_ty)) = self.functions.get(&candidate).cloned() {
                    if let Type::Function(params, _) = &func_ty {
                        if args.len() != params.len() {
                            return Err(Self::function_call_arity_error(&func_ty, args.len()));
                        }
                    }
                    let mut compiled_args: Vec<BasicValueEnum> = Vec::with_capacity(args.len() + 1);
                    compiled_args.push(
                        self.context
                            .ptr_type(AddressSpace::default())
                            .const_null()
                            .into(),
                    );
                    if let Type::Function(params, _) = &func_ty {
                        for (arg, param_ty) in args.iter().zip(params.iter()) {
                            compiled_args.push(
                                self.compile_expr_for_concrete_class_payload(&arg.node, param_ty)?,
                            );
                        }
                    } else {
                        for arg in args {
                            compiled_args.push(self.compile_expr(&arg.node)?);
                        }
                    }
                    let args_meta: Vec<BasicMetadataValueEnum> =
                        compiled_args.iter().map(|a| (*a).into()).collect();
                    let call = self
                        .builder
                        .build_call(func, &args_meta, "call")
                        .map_err(|_| {
                            CodegenError::new("failed to emit nested module function call")
                        })?;
                    return match call.try_as_basic_value() {
                        ValueKind::Basic(val) => Ok(val),
                        ValueKind::Instruction(_) => {
                            Ok(self.context.i8_type().const_int(0, false).into())
                        }
                    };
                }
                if let Some(candidate) = self.resolve_wildcard_import_module_function_candidate(
                    &path_parts[0],
                    &path_parts[1..],
                ) {
                    if let Some((func, func_ty)) = self.functions.get(&candidate).cloned() {
                        if let Type::Function(params, _) = &func_ty {
                            if args.len() != params.len() {
                                return Err(Self::function_call_arity_error(&func_ty, args.len()));
                            }
                        }
                        let mut compiled_args: Vec<BasicValueEnum> =
                            Vec::with_capacity(args.len() + 1);
                        compiled_args.push(
                            self.context
                                .ptr_type(AddressSpace::default())
                                .const_null()
                                .into(),
                        );
                        if let Type::Function(params, _) = &func_ty {
                            for (arg, param_ty) in args.iter().zip(params.iter()) {
                                compiled_args.push(self.compile_expr_for_concrete_class_payload(
                                    &arg.node, param_ty,
                                )?);
                            }
                        } else {
                            for arg in args {
                                compiled_args.push(self.compile_expr(&arg.node)?);
                            }
                        }
                        let args_meta: Vec<BasicMetadataValueEnum> =
                            compiled_args.iter().map(|a| (*a).into()).collect();
                        let call =
                            self.builder
                                .build_call(func, &args_meta, "call")
                                .map_err(|_| {
                                    CodegenError::new(
                                        "failed to emit wildcard nested module function call",
                                    )
                                })?;
                        return match call.try_as_basic_value() {
                            ValueKind::Basic(val) => Ok(val),
                            ValueKind::Instruction(_) => {
                                Ok(self.context.i8_type().const_int(0, false).into())
                            }
                        };
                    }
                }
                if let Some(candidate) = self.resolve_import_alias_module_function_candidate(
                    &path_parts[0],
                    &path_parts[1..],
                ) {
                    if Self::is_supported_builtin_function_name(&candidate) {
                        match candidate.as_str() {
                            "Option__some" => {
                                if args.len() != 1 {
                                    return Err(CodegenError::new(
                                        "Option.some() requires exactly 1 argument",
                                    ));
                                }
                                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                                let value =
                                    self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                                return self.create_option_some(value);
                            }
                            "Option__none" => {
                                if !args.is_empty() {
                                    return Err(CodegenError::new(format!(
                                        "Option.none() expects 0 argument(s), got {}",
                                        args.len()
                                    )));
                                }
                                return self.create_option_none();
                            }
                            "Result__ok" => {
                                if args.len() != 1 {
                                    return Err(CodegenError::new(
                                        "Result.ok() requires exactly 1 argument",
                                    ));
                                }
                                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                                let value =
                                    self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                                return self.create_result_ok(value);
                            }
                            "Result__error" => {
                                if args.len() != 1 {
                                    return Err(CodegenError::new(
                                        "Result.error() requires exactly 1 argument",
                                    ));
                                }
                                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                                let value =
                                    self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                                return self.create_result_error(value);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Method call on object
        if let Expr::Field { object, field } = callee {
            let field_ty = self.infer_object_type(&object.node).and_then(|obj_ty| {
                let (class_name, generic_args) = self.unwrap_class_like_type(&obj_ty)?;
                let class_info = self.classes.get(&class_name)?;
                let field_ty = class_info.field_types.get(field)?.clone();
                if let Some(args) = generic_args {
                    if class_info.generic_params.len() == args.len() {
                        let bindings = class_info
                            .generic_params
                            .iter()
                            .cloned()
                            .zip(args)
                            .collect::<HashMap<_, _>>();
                        return Some(Self::substitute_type(&field_ty, &bindings));
                    }
                }
                Some(field_ty)
            });
            if let Some(field_ty) = field_ty {
                if let Type::Function(param_types, ret_type) = field_ty {
                    let compiled_callee = self.compile_expr(callee)?;
                    let (ptr, env_ptr) = if compiled_callee.is_struct_value() {
                        let closure_val = compiled_callee.into_struct_value();
                        let ptr = self
                            .builder
                            .build_extract_value(closure_val, 0, "fn_ptr")
                            .map_err(|_| {
                                CodegenError::new(
                                    "failed to extract function pointer from field closure",
                                )
                            })?
                            .into_pointer_value();
                        let env_ptr = self
                            .builder
                            .build_extract_value(closure_val, 1, "env_ptr")
                            .map_err(|_| {
                                CodegenError::new(
                                    "failed to extract closure environment from field closure",
                                )
                            })?;
                        (ptr, env_ptr)
                    } else if compiled_callee.is_pointer_value() {
                        (
                            compiled_callee.into_pointer_value(),
                            self.context
                                .ptr_type(AddressSpace::default())
                                .const_null()
                                .into(),
                        )
                    } else {
                        return Err(CodegenError::new(format!(
                            "Function-valued field '{}': expected closure or function pointer, got {:?}",
                            field, compiled_callee
                        )));
                    };

                    let llvm_ret = self.llvm_type(&ret_type);
                    let mut llvm_params: Vec<BasicMetadataTypeEnum> =
                        vec![self.context.ptr_type(AddressSpace::default()).into()];
                    for p in &param_types {
                        llvm_params.push(self.llvm_type(p).into());
                    }

                    let fn_type = match llvm_ret {
                        BasicTypeEnum::IntType(i) => i.fn_type(&llvm_params, false),
                        BasicTypeEnum::FloatType(f) => f.fn_type(&llvm_params, false),
                        BasicTypeEnum::PointerType(p) => p.fn_type(&llvm_params, false),
                        BasicTypeEnum::StructType(s) => s.fn_type(&llvm_params, false),
                        _ => self.context.i8_type().fn_type(&llvm_params, false),
                    };

                    let mut compiled_args: Vec<BasicValueEnum> = Vec::with_capacity(args.len() + 1);
                    compiled_args.push(env_ptr);
                    if args.len() != param_types.len() {
                        return Err(Self::function_call_arity_error(
                            &Type::Function(param_types.clone(), ret_type.clone()),
                            args.len(),
                        ));
                    }
                    for (arg, param_ty) in args.iter().zip(param_types.iter()) {
                        compiled_args.push(
                            self.compile_expr_for_concrete_class_payload(&arg.node, param_ty)?,
                        );
                    }

                    let args_meta: Vec<BasicMetadataValueEnum> =
                        compiled_args.iter().map(|a| (*a).into()).collect();
                    let call = self
                        .builder
                        .build_indirect_call(fn_type, ptr, &args_meta, "call")
                        .map_err(|_| {
                            CodegenError::new("failed to emit indirect call for field closure")
                        })?;

                    return Ok(match call.try_as_basic_value() {
                        ValueKind::Basic(val) => val,
                        ValueKind::Instruction(_) => {
                            self.context.i8_type().const_int(0, false).into()
                        }
                    });
                }

                let _ = self.compile_expr(callee)?;
                return Err(Self::non_function_call_error(&field_ty));
            }

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

        if !matches!(callee, Expr::Ident(_) | Expr::Field { .. }) {
            let callee_ty = self.infer_expr_type(callee, &[]);
            if let Type::Function(param_types, ret_type) = callee_ty {
                let closure_val = self.compile_expr(callee)?.into_struct_value();
                let ptr = self
                    .builder
                    .build_extract_value(closure_val, 0, "fn_ptr")
                    .map_err(|_| {
                        CodegenError::new("failed to extract function pointer from closure")
                    })?
                    .into_pointer_value();
                let env_ptr = self
                    .builder
                    .build_extract_value(closure_val, 1, "env_ptr")
                    .map_err(|_| CodegenError::new("failed to extract closure environment"))?;

                let llvm_ret = self.llvm_type(&ret_type);
                let mut llvm_params: Vec<BasicMetadataTypeEnum> =
                    vec![self.context.ptr_type(AddressSpace::default()).into()];
                for p in &param_types {
                    llvm_params.push(self.llvm_type(p).into());
                }

                let fn_type = match llvm_ret {
                    BasicTypeEnum::IntType(i) => i.fn_type(&llvm_params, false),
                    BasicTypeEnum::FloatType(f) => f.fn_type(&llvm_params, false),
                    BasicTypeEnum::PointerType(p) => p.fn_type(&llvm_params, false),
                    BasicTypeEnum::StructType(s) => s.fn_type(&llvm_params, false),
                    _ => self.context.i8_type().fn_type(&llvm_params, false),
                };

                if args.len() != param_types.len() {
                    return Err(Self::function_call_arity_error(
                        &Type::Function(param_types.clone(), ret_type.clone()),
                        args.len(),
                    ));
                }

                let mut compiled_args: Vec<BasicValueEnum> = Vec::with_capacity(args.len() + 1);
                compiled_args.push(env_ptr);
                for (arg, param_ty) in args.iter().zip(param_types.iter()) {
                    compiled_args
                        .push(self.compile_expr_for_concrete_class_payload(&arg.node, param_ty)?);
                }

                let args_meta: Vec<BasicMetadataValueEnum> =
                    compiled_args.iter().map(|a| (*a).into()).collect();
                let call = self
                    .builder
                    .build_indirect_call(fn_type, ptr, &args_meta, "call")
                    .map_err(|_| CodegenError::new("failed to emit indirect closure call"))?;

                return Ok(match call.try_as_basic_value() {
                    ValueKind::Basic(val) => val,
                    ValueKind::Instruction(_) => self.context.i8_type().const_int(0, false).into(),
                });
            }

            return Err(Self::non_function_call_error(&callee_ty));
        }

        // Regular function call
        let callee_name = ident_name.map(|name| resolved_ident.as_deref().unwrap_or(name));
        let (func, resolved_func_ty) = match callee {
            Expr::Ident(name) => {
                // First check if it's a function pointer/local variable
                if let Some(var) = self.variables.get(name) {
                    if let Type::Function(param_types, ret_type) = &var.ty {
                        let param_types = param_types.clone();
                        let ret_type = ret_type.clone();
                        let closure_val = self
                            .builder
                            .build_load(self.llvm_type(&var.ty), var.ptr, name)
                            .map_err(|_| {
                                CodegenError::new("failed to load function-valued variable")
                            })?
                            .into_struct_value();

                        let ptr = self
                            .builder
                            .build_extract_value(closure_val, 0, "fn_ptr")
                            .map_err(|_| {
                                CodegenError::new(
                                    "failed to extract function pointer from variable closure",
                                )
                            })?
                            .into_pointer_value();
                        let env_ptr = self
                            .builder
                            .build_extract_value(closure_val, 1, "env_ptr")
                            .map_err(|_| {
                                CodegenError::new(
                                    "failed to extract environment from variable closure",
                                )
                            })?;

                        // Construct FunctionType (including env_ptr as first arg)
                        let llvm_ret = self.llvm_type(&ret_type);
                        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![
                            self.context.ptr_type(AddressSpace::default()).into(), // env_ptr
                        ];
                        for p in &param_types {
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

                        if args.len() != param_types.len() {
                            return Err(Self::function_call_arity_error(&var.ty, args.len()));
                        }

                        let mut compiled_args: Vec<BasicValueEnum> =
                            Vec::with_capacity(args.len() + 1);
                        compiled_args.push(env_ptr);
                        for (arg, param_ty) in args.iter().zip(param_types.iter()) {
                            compiled_args.push(
                                self.compile_expr_for_concrete_class_payload(&arg.node, param_ty)?,
                            );
                        }

                        let args_meta: Vec<BasicMetadataValueEnum> =
                            compiled_args.iter().map(|a| (*a).into()).collect();

                        let call = self
                            .builder
                            .build_indirect_call(fn_type, ptr, &args_meta, "call")
                            .map_err(|_| {
                                CodegenError::new(
                                    "failed to emit indirect call for function-valued variable",
                                )
                            })?;

                        let result = match call.try_as_basic_value() {
                            ValueKind::Basic(val) => val,
                            ValueKind::Instruction(_) => {
                                self.context.i8_type().const_int(0, false).into()
                            }
                        };
                        return Ok(result);
                    }

                    return Err(Self::non_function_call_error(&var.ty));
                }

                let looked_up_name = resolved_ident.as_deref().unwrap_or(name.as_str());

                // Fall back to global function lookup
                if let Some((f, func_ty)) = self.functions.get(looked_up_name).cloned() {
                    (f, Some(func_ty))
                } else if let Some(f) = self.module.get_function(looked_up_name) {
                    (f, None)
                } else {
                    return Err(Self::undefined_function_error(looked_up_name));
                }
            }
            _ => return Err(CodegenError::new("Invalid callee")),
        };

        let is_extern_call = callee_name
            .map(|n| self.extern_functions.contains(n))
            .unwrap_or(false);
        let func_name = func.get_name();
        let is_main_function = func_name.to_str().unwrap_or_default() == "main";
        let env_arg_count = usize::from(!is_main_function && !is_extern_call);
        let mut compiled_args: Vec<BasicValueEnum> = Vec::with_capacity(args.len() + env_arg_count);
        // Add null env_ptr for direct Arden calls (except main / extern C ABI)
        if !is_main_function && !is_extern_call {
            compiled_args.push(
                self.context
                    .ptr_type(AddressSpace::default())
                    .const_null()
                    .into(),
            );
        }

        let callee_ty = resolved_func_ty.unwrap_or_else(|| self.infer_expr_type(callee, &[]));
        let expected_param_types = match &callee_ty {
            Type::Function(param_types, _) => Some(param_types.as_slice()),
            _ => None,
        };
        if let Some(param_types) = expected_param_types {
            let is_variadic_extern_call = is_extern_call && func.get_type().is_var_arg();
            let bad_arity = if is_variadic_extern_call {
                args.len() < param_types.len()
            } else {
                args.len() != param_types.len()
            };
            if bad_arity {
                return Err(Self::function_call_arity_error(&callee_ty, args.len()));
            }
            for (arg, param_ty) in args.iter().zip(param_types.iter()) {
                compiled_args
                    .push(self.compile_expr_for_concrete_class_payload(&arg.node, param_ty)?);
            }
            if is_variadic_extern_call {
                for arg in args.iter().skip(param_types.len()) {
                    let arg_ty = self.infer_builtin_argument_type(&arg.node);
                    compiled_args.push(self.compile_expr_with_expected_type(&arg.node, &arg_ty)?);
                }
            }
        } else {
            for a in args {
                compiled_args.push(self.compile_expr(&a.node)?);
            }
        }

        let args_meta: Vec<BasicMetadataValueEnum> =
            compiled_args.iter().map(|a| (*a).into()).collect();
        let call = self
            .builder
            .build_call(func, &args_meta, "call")
            .map_err(|_| CodegenError::new("failed to emit direct function call"))?;

        match call.try_as_basic_value() {
            ValueKind::Basic(val) => Ok(val),
            ValueKind::Instruction(_) => Ok(self.context.i8_type().const_int(0, false).into()),
        }
    }
}
