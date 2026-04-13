use super::*;

impl TypeChecker {
    pub(super) fn check_expr(&mut self, expr: &Expr, span: Span) -> ResolvedType {
        match expr {
            Expr::Literal(lit) => Self::literal_type(lit),

            Expr::Ident(name) => {
                if let Some(var) = self.lookup_variable(name) {
                    var.ty.clone()
                } else if let Some(canonical_name) = self
                    .resolve_import_alias_symbol(name)
                    .filter(|canonical_name| self.functions.contains_key(canonical_name))
                {
                    self.function_value_type_or_error(&canonical_name, span)
                } else if let Some(canonical_name) = self.resolve_import_alias_symbol(name) {
                    if let Some(ty) = Self::builtin_function_value_type(&canonical_name) {
                        ty
                    } else {
                        self.error(format!("Undefined variable: {}", name), span);
                        ResolvedType::Unknown
                    }
                } else if let Some((enum_name, variant_name)) =
                    self.resolve_import_alias_variant(name)
                {
                    if let Some(enum_info) = self.enums.get(&enum_name) {
                        if let Some(variant_fields) = enum_info.variants.get(&variant_name) {
                            if variant_fields.is_empty() {
                                ResolvedType::Class(enum_name)
                            } else {
                                self.error(
                                    format!(
                                        "Enum variant '{}.{}' requires {} argument(s)",
                                        enum_name,
                                        variant_name,
                                        variant_fields.len()
                                    ),
                                    span,
                                );
                                ResolvedType::Unknown
                            }
                        } else {
                            self.error(
                                format!(
                                    "Unknown variant '{}' for enum '{}'",
                                    variant_name,
                                    format_diagnostic_class_name(&enum_name)
                                ),
                                span,
                            );
                            ResolvedType::Unknown
                        }
                    } else {
                        self.error(format!("Undefined variable: {}", name), span);
                        ResolvedType::Unknown
                    }
                } else if let Some(function_name) = self
                    .resolve_wildcard_import_symbol(name)
                    .or_else(|| self.resolve_function_value_name(name).map(str::to_string))
                {
                    self.function_value_type_or_error(&function_name, span)
                } else if let Some(actual_ty) = self.resolve_class_constructor_function_value_type(
                    expr,
                    None,
                    None,
                    span.clone(),
                ) {
                    actual_ty
                } else {
                    self.error(format!("Undefined variable: {}", name), span);
                    ResolvedType::Unknown
                }
            }

            Expr::GenericFunctionValue { callee, type_args } => match &callee.node {
                Expr::Ident(name) => {
                    if let Some((enum_name, variant_name)) = self.resolve_import_alias_variant(name)
                    {
                        self.error(
                            format!(
                                "Enum variant '{}.{}' does not accept type arguments",
                                format_diagnostic_class_name(&enum_name),
                                variant_name
                            ),
                            span,
                        );
                        return ResolvedType::Unknown;
                    }
                    if let Some(canonical_name) = self.resolve_import_alias_symbol(name) {
                        if let Some(ty) = Self::builtin_function_value_type(&canonical_name) {
                            let builtin_label = match canonical_name.as_str() {
                                "Option__some" => "Option.some",
                                "Option__none" => "Option.none",
                                "Result__ok" => "Result.ok",
                                "Result__error" => "Result.error",
                                _ => canonical_name.as_str(),
                            };
                            let _ = ty;
                            self.error(
                                format!(
                                    "Built-in function '{}' does not accept type arguments",
                                    builtin_label
                                ),
                                span,
                            );
                            return ResolvedType::Unknown;
                        }
                    }
                    if let Some(canonical_name) = self
                        .resolve_import_alias_symbol(name)
                        .filter(|canonical_name| self.functions.contains_key(canonical_name))
                    {
                        let Some(sig) = self.functions.get(&canonical_name).cloned() else {
                            self.error(format!("Undefined variable: {}", name), span);
                            return ResolvedType::Unknown;
                        };
                        self.instantiate_function_value_type(&canonical_name, &sig, type_args, span)
                    } else if let Some(function_name) =
                        self.resolve_function_value_name(name).map(str::to_string)
                    {
                        let Some(sig) = self.functions.get(&function_name).cloned() else {
                            self.error(format!("Undefined variable: {}", name), span);
                            return ResolvedType::Unknown;
                        };
                        self.instantiate_function_value_type(&function_name, &sig, type_args, span)
                    } else if let Some(actual_ty) = self
                        .resolve_class_constructor_function_value_type(
                            expr,
                            Some(type_args),
                            None,
                            span.clone(),
                        )
                    {
                        actual_ty
                    } else {
                        self.error(format!("Undefined variable: {}", name), span);
                        ResolvedType::Unknown
                    }
                }
                Expr::Field { object, field } => {
                    if let Some((enum_name, field_types)) =
                        self.resolve_enum_variant_function_value(&callee.node)
                    {
                        let _ = field_types;
                        self.error(
                            format!(
                                "Enum variant '{}.{}' does not accept type arguments",
                                format_diagnostic_class_name(&enum_name),
                                field
                            ),
                            span,
                        );
                        return ResolvedType::Unknown;
                    }
                    if let Some(canonical_name) =
                        self.resolve_contextual_function_value_name(&callee.node)
                    {
                        if Self::builtin_function_value_type(&canonical_name).is_some() {
                            let builtin_label = canonical_name.replace("__", ".");
                            self.error(
                                format!(
                                    "Built-in function '{}' does not accept type arguments",
                                    builtin_label
                                ),
                                span,
                            );
                            return ResolvedType::Unknown;
                        }
                    }
                    if let Some(path_parts) = flatten_field_chain(&callee.node) {
                        if path_parts.len() >= 2 {
                            if let Some(candidate) = self.resolve_import_alias_module_candidate(
                                &path_parts[0],
                                &path_parts[1..],
                            ) {
                                let resolved = self
                                    .resolve_function_value_name(&candidate)
                                    .unwrap_or(&candidate)
                                    .to_string();
                                if let Some(sig) = self.functions.get(&resolved).cloned() {
                                    return self.instantiate_function_value_type(
                                        &resolved, &sig, type_args, span,
                                    );
                                }
                            }
                            if let Some(candidate) = self
                                .resolve_wildcard_import_module_function_candidate(
                                    &path_parts[0],
                                    &path_parts[1..],
                                )
                            {
                                if let Some(sig) = self.functions.get(&candidate).cloned() {
                                    return self.instantiate_function_value_type(
                                        &candidate, &sig, type_args, span,
                                    );
                                }
                            }
                            let mangled = path_parts.join("__");
                            if let Some(sig) = self.functions.get(&mangled).cloned() {
                                return self.instantiate_function_value_type(
                                    &mangled, &sig, type_args, span,
                                );
                            }
                            if let Some(canonical) =
                                crate::ast::builtin_exact_import_alias_canonical(
                                    &path_parts.join("."),
                                )
                            {
                                let builtin_label = canonical.replace("__", ".");
                                self.error(
                                    format!(
                                        "Built-in function '{}' does not accept type arguments",
                                        builtin_label
                                    ),
                                    span,
                                );
                                return ResolvedType::Unknown;
                            }
                        }
                    }

                    let obj_type = self.check_expr(&object.node, object.span.clone());
                    let receiver_type = Self::peel_reference_type(&obj_type);
                    match receiver_type {
                        ResolvedType::Class(name) => {
                            let (base_name, class_substitutions) =
                                self.instantiated_class_substitutions(name);
                            if let Some((owner, sig, visibility)) =
                                self.lookup_class_method(&base_name, field)
                            {
                                self.check_member_visibility(
                                    &owner,
                                    visibility,
                                    "Method",
                                    field,
                                    span.clone(),
                                );
                                let sig = FuncSig {
                                    params: sig
                                        .params
                                        .iter()
                                        .map(|(name, ty)| {
                                            (
                                                name.clone(),
                                                Self::substitute_type_vars(
                                                    ty,
                                                    &class_substitutions,
                                                ),
                                            )
                                        })
                                        .collect(),
                                    return_type: Self::substitute_type_vars(
                                        &sig.return_type,
                                        &class_substitutions,
                                    ),
                                    ..sig
                                };
                                let method_name = format!("{}.{}", owner, field);
                                self.instantiate_function_value_type(
                                    &method_name,
                                    &sig,
                                    type_args,
                                    span,
                                )
                            } else {
                                self.error(
                                    format!(
                                        "Unknown field '{}' on class '{}'",
                                        field,
                                        format_diagnostic_class_name(name)
                                    ),
                                    span,
                                );
                                ResolvedType::Unknown
                            }
                        }
                        _ => {
                            self.error(
                                format!(
                                    "Cannot access field on type {}",
                                    Self::format_resolved_type_for_diagnostic(&obj_type)
                                ),
                                span,
                            );
                            ResolvedType::Unknown
                        }
                    }
                }
                _ => {
                    self.error(
                        "Explicit generic function values require a named function or method"
                            .to_string(),
                        span,
                    );
                    ResolvedType::Unknown
                }
            },

            Expr::Binary { op, left, right } => {
                let left_type = self.check_builtin_argument_expr(&left.node, left.span.clone());
                let right_type = self.check_builtin_argument_expr(&right.node, right.span.clone());

                if matches!(op, BinOp::Div | BinOp::Mod)
                    && matches!(left_type, ResolvedType::Integer)
                    && matches!(right_type, ResolvedType::Integer)
                    && matches!(
                        Self::eval_numeric_const_expr(&right.node),
                        Some(NumericConst::Integer(0))
                    )
                {
                    let message = if matches!(op, BinOp::Div) {
                        "Integer division by zero"
                    } else {
                        "Integer modulo by zero"
                    };
                    self.error(message.to_string(), right.span.clone());
                }

                self.check_binary_op(*op, &left_type, &right_type, span)
            }

            Expr::Unary { op, expr: inner } => {
                let inner_type = self.check_builtin_argument_expr(&inner.node, inner.span.clone());

                match op {
                    UnaryOp::Neg => {
                        if !matches!(inner_type, ResolvedType::Unknown) && !inner_type.is_numeric()
                        {
                            self.error(
                                format!(
                                    "Cannot negate non-numeric type {}",
                                    Self::format_resolved_type_for_diagnostic(&inner_type)
                                ),
                                span,
                            );
                        }
                        inner_type
                    }
                    UnaryOp::Not => {
                        if !matches!(inner_type, ResolvedType::Unknown)
                            && !matches!(inner_type, ResolvedType::Boolean)
                        {
                            self.error(
                                format!(
                                    "Cannot apply '!' to non-boolean type {}",
                                    Self::format_resolved_type_for_diagnostic(&inner_type)
                                ),
                                span,
                            );
                        }
                        ResolvedType::Boolean
                    }
                }
            }

            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                if let Some(path_parts) = flatten_field_chain(&callee.node) {
                    if path_parts.len() >= 2 {
                        if let Some(alias_path) = self.lookup_import_alias_path(&path_parts[0]) {
                            let full_alias_path =
                                format!("{}.{}", alias_path, path_parts[1..].join("."));
                            if let Some(canonical) =
                                crate::ast::builtin_exact_import_alias_canonical(&full_alias_path)
                            {
                                if !type_args.is_empty() {
                                    self.error(
                                        format!(
                                            "Built-in function '{}' does not accept type arguments",
                                            canonical.replace("__", ".")
                                        ),
                                        span.clone(),
                                    );
                                }
                                if let Some(return_type) =
                                    self.check_builtin_call(canonical, args, span.clone())
                                {
                                    return return_type;
                                }
                            }
                        }
                    }
                }
                self.check_call(&callee.node, args, type_args, span)
            }

            Expr::Field { object, field } => {
                if let Some(path_parts) = flatten_field_chain(expr) {
                    if path_parts.len() >= 2 {
                        let owner_source = path_parts[..path_parts.len() - 1].join(".");
                        if let Some(resolved_owner) =
                            self.resolve_nominal_reference_name(&owner_source)
                        {
                            if let Some(enum_info) = self.enums.get(&resolved_owner) {
                                if let Some(variant_name) = path_parts.last() {
                                    if let Some(variant_fields) =
                                        enum_info.variants.get(variant_name)
                                    {
                                        if variant_fields.is_empty() {
                                            return ResolvedType::Class(resolved_owner);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Expr::Ident(owner_name) = &object.node {
                    let resolved_owner = self
                        .resolve_import_alias_symbol(owner_name)
                        .or_else(|| self.resolve_nominal_reference_name(owner_name))
                        .or_else(|| self.resolve_enum_name(owner_name))
                        .unwrap_or_else(|| owner_name.clone());
                    if let Some(enum_info) = self.enums.get(&resolved_owner) {
                        if let Some(variant_fields) = enum_info.variants.get(field) {
                            if variant_fields.is_empty() {
                                return ResolvedType::Class(resolved_owner);
                            }
                            self.error(
                                format!(
                                    "Enum variant '{}.{}' requires {} argument(s)",
                                    resolved_owner,
                                    field,
                                    variant_fields.len()
                                ),
                                span.clone(),
                            );
                            return ResolvedType::Unknown;
                        }
                    }
                }
                if let Some(path_parts) = flatten_field_chain(expr) {
                    if path_parts.len() >= 2 {
                        if let Some(alias_path) = self.lookup_import_alias_path(&path_parts[0]) {
                            let full_alias_path =
                                format!("{}.{}", alias_path, path_parts[1..].join("."));
                            if let Some(canonical) =
                                crate::ast::builtin_exact_import_alias_canonical(&full_alias_path)
                            {
                                if let Some(ty) = Self::builtin_function_value_type(canonical) {
                                    return ty;
                                }
                            }
                        }
                        if let Some(candidate) = self
                            .resolve_import_alias_module_candidate(&path_parts[0], &path_parts[1..])
                        {
                            let resolved = self
                                .resolve_function_value_name(&candidate)
                                .unwrap_or(&candidate);
                            if self.functions.contains_key(resolved) {
                                let resolved = resolved.to_owned();
                                return self.function_value_type_or_error(&resolved, span.clone());
                            }
                            if let Some(ty) = Self::builtin_function_value_type(resolved) {
                                return ty;
                            }
                        }
                        if let Some(candidate) = self
                            .resolve_wildcard_import_module_function_candidate(
                                &path_parts[0],
                                &path_parts[1..],
                            )
                        {
                            return self.function_value_type_or_error(&candidate, span.clone());
                        }

                        let mangled = path_parts.join("__");
                        let resolved = self
                            .resolve_function_value_name(&mangled)
                            .unwrap_or(&mangled);
                        if self.functions.contains_key(resolved) {
                            let resolved = resolved.to_owned();
                            return self.function_value_type_or_error(&resolved, span.clone());
                        }
                        if let Some(canonical) =
                            crate::ast::builtin_exact_import_alias_canonical(&path_parts.join("."))
                        {
                            if let Some(ty) = Self::builtin_function_value_type(canonical) {
                                return ty;
                            }
                        }
                        if let Some(ty) = Self::builtin_function_value_type(&mangled) {
                            return ty;
                        }
                    }
                }
                let obj_type = self.check_expr(&object.node, object.span.clone());
                self.check_field_access(&obj_type, field, span)
            }

            Expr::Index { object, index } => {
                let obj_type = self.check_builtin_argument_expr(&object.node, object.span.clone());
                let idx_type = self.check_builtin_argument_expr(&index.node, index.span.clone());
                let indexed_type = Self::peel_reference_type(&obj_type);

                if matches!(obj_type, ResolvedType::Unknown)
                    || matches!(idx_type, ResolvedType::Unknown)
                {
                    return match indexed_type {
                        ResolvedType::List(inner) => (**inner).clone(),
                        ResolvedType::String => ResolvedType::Char,
                        ResolvedType::Map(_, value) => (**value).clone(),
                        _ => ResolvedType::Unknown,
                    };
                }

                match indexed_type {
                    ResolvedType::List(inner) => {
                        if !matches!(idx_type, ResolvedType::Integer) {
                            self.error(
                                format!(
                                    "Index must be Integer, found {}",
                                    Self::format_resolved_type_for_diagnostic(&idx_type)
                                ),
                                index.span.clone(),
                            );
                        } else {
                            self.check_non_negative_integer_const(
                                &index.node,
                                index.span.clone(),
                                "List index cannot be negative",
                            );
                        }
                        (**inner).clone()
                    }
                    ResolvedType::String => {
                        if !matches!(idx_type, ResolvedType::Integer) {
                            self.error(
                                format!(
                                    "Index must be Integer, found {}",
                                    Self::format_resolved_type_for_diagnostic(&idx_type)
                                ),
                                index.span.clone(),
                            );
                        } else {
                            self.check_non_negative_integer_const(
                                &index.node,
                                index.span.clone(),
                                "String index cannot be negative",
                            );
                            if let (Some(string_len), Some(NumericConst::Integer(value))) = (
                                Self::eval_const_string_len(&object.node),
                                Self::eval_numeric_const_expr(&index.node),
                            ) {
                                if value >= 0 && (value as usize) >= string_len {
                                    self.error(
                                        "String index out of bounds".to_string(),
                                        index.span.clone(),
                                    );
                                }
                            }
                        }
                        ResolvedType::Char
                    }
                    ResolvedType::Map(k, v) => {
                        if !self.types_compatible(k, &idx_type) {
                            self.error(
                                format!(
                                    "Map index type mismatch: expected {}, got {}",
                                    Self::format_resolved_type_for_diagnostic(k),
                                    Self::format_resolved_type_for_diagnostic(&idx_type)
                                ),
                                index.span.clone(),
                            );
                        }
                        (**v).clone()
                    }
                    _ => {
                        self.error(
                            format!(
                                "Cannot index type {}",
                                Self::format_resolved_type_for_diagnostic(&obj_type)
                            ),
                            span,
                        );
                        ResolvedType::Unknown
                    }
                }
            }

            Expr::Construct { ty, args } => {
                if let Some((base_name, explicit_type_args)) =
                    Self::parse_construct_nominal_type_source(ty)
                {
                    if let Some(canonical_name) = self
                        .resolve_import_alias_symbol(&base_name)
                        .filter(|name| Self::builtin_function_value_type(name).is_some())
                    {
                        if !explicit_type_args.is_empty() {
                            self.error(
                                format!(
                                    "Built-in function '{}' does not accept type arguments",
                                    canonical_name.replace("__", ".")
                                ),
                                span.clone(),
                            );
                            return ResolvedType::Unknown;
                        }
                        if let Some(value_ty) =
                            Self::concrete_zero_arg_builtin_value_type(&canonical_name)
                        {
                            self.error(
                                format!(
                                    "Cannot call non-function type {}",
                                    Self::format_resolved_type_for_diagnostic(&value_ty)
                                ),
                                span.clone(),
                            );
                            return ResolvedType::Unknown;
                        }
                        return self
                            .check_builtin_call(&canonical_name, args, span.clone())
                            .unwrap_or(ResolvedType::Unknown);
                    }
                    if let Some((enum_name, variant_name)) =
                        self.resolve_import_alias_variant(&base_name)
                    {
                        if !explicit_type_args.is_empty() {
                            self.error(
                                format!(
                                    "Enum variant '{}.{}' does not accept type arguments",
                                    format_diagnostic_class_name(&enum_name),
                                    variant_name
                                ),
                                span.clone(),
                            );
                            return ResolvedType::Unknown;
                        }
                    }
                }

                let resolved_construct_type = self.resolve_type_source(ty);
                let scoped_ty = resolved_construct_type
                    .clone()
                    .map(|resolved| resolved.to_string())
                    .unwrap_or_else(|| self.resolve_type_source_string(ty));

                if let Some((enum_name, variant_name)) = self.resolve_import_alias_variant(ty) {
                    if let Some(enum_info) = self.enums.get(&enum_name).cloned() {
                        if let Some(field_types) = enum_info.variants.get(&variant_name) {
                            if args.len() != field_types.len() {
                                self.error(
                                    format!(
                                        "Enum variant '{}.{}' expects {} argument(s), got {}",
                                        enum_name,
                                        variant_name,
                                        field_types.len(),
                                        args.len()
                                    ),
                                    span.clone(),
                                );
                            } else {
                                for (arg, expected_ty) in args.iter().zip(field_types.iter()) {
                                    let actual = self.check_expr_with_expected_type(
                                        &arg.node,
                                        arg.span.clone(),
                                        Some(expected_ty),
                                    );
                                    if !self.types_compatible(expected_ty, &actual) {
                                        self.error(
                                            format!(
                                                "Enum variant argument type mismatch: expected {}, got {}",
                                                Self::format_resolved_type_for_diagnostic(expected_ty),
                                                Self::format_resolved_type_for_diagnostic(&actual)
                                            ),
                                            arg.span.clone(),
                                        );
                                    }
                                }
                            }
                            return ResolvedType::Class(enum_name);
                        }
                    }
                }

                // Handle generic built-in types (e.g., List<Integer>, Set<String>)
                if scoped_ty.contains('<') && scoped_ty.ends_with('>') {
                    if scoped_ty.starts_with("Ptr<")
                        || scoped_ty.starts_with("Task<")
                        || scoped_ty.starts_with("Range<")
                    {
                        self.error(
                            format!(
                                "Cannot construct built-in type '{}'",
                                format_diagnostic_class_name(&scoped_ty)
                            ),
                            span.clone(),
                        );
                        return self.parse_type_string(&scoped_ty);
                    }
                    let resolved = resolved_construct_type
                        .clone()
                        .unwrap_or_else(|| self.parse_type_string(&scoped_ty));
                    if !matches!(resolved, ResolvedType::Class(_))
                        && !matches!(resolved, ResolvedType::Unknown)
                    {
                        self.check_builtin_constructor_args(
                            &scoped_ty,
                            &resolved,
                            args,
                            span.clone(),
                        );
                        return resolved;
                    }
                }

                let (class_name, class_substitutions) =
                    self.instantiated_class_substitutions(&scoped_ty);
                self.validate_class_type_argument_bounds(&scoped_ty, span.clone(), "Constructor");

                if self.interfaces.contains_key(&scoped_ty)
                    || self.interfaces.contains_key(&class_name)
                {
                    self.error(
                        format!(
                            "Cannot construct interface type '{}'",
                            format_diagnostic_class_name(&scoped_ty)
                        ),
                        span,
                    );
                    return ResolvedType::Unknown;
                }

                // Check if it's a class constructor
                if let Some(class) = self.classes.get(&class_name).cloned() {
                    self.check_class_visibility(&class_name, span.clone());
                    if let Some(ctor_params) = &class.constructor {
                        let ctor_params = ctor_params
                            .iter()
                            .map(|(name, ty)| {
                                (
                                    name.clone(),
                                    Self::substitute_type_vars(ty, &class_substitutions),
                                )
                            })
                            .collect::<Vec<_>>();
                        if args.len() != ctor_params.len() {
                            self.error(
                                format!(
                                    "Constructor {} expects {} arguments, got {}",
                                    scoped_ty,
                                    ctor_params.len(),
                                    args.len()
                                ),
                                span,
                            );
                        } else {
                            for (arg, (_, expected)) in args.iter().zip(ctor_params.iter()) {
                                let arg_type = self.check_expr_with_expected_type(
                                    &arg.node,
                                    arg.span.clone(),
                                    Some(expected),
                                );
                                if !self.types_compatible(expected, &arg_type) {
                                    self.error(
                                            format!(
                                                "Constructor argument type mismatch: expected {}, got {}",
                                                Self::format_resolved_type_for_diagnostic(expected),
                                                Self::format_resolved_type_for_diagnostic(&arg_type)
                                            ),
                                            arg.span.clone(),
                                        );
                                }
                            }
                        }
                    }
                    resolved_construct_type.unwrap_or_else(|| self.parse_type_string(&scoped_ty))
                } else if scoped_ty == "List"
                    || scoped_ty == "Map"
                    || scoped_ty == "Set"
                    || scoped_ty == "Option"
                    || scoped_ty == "Result"
                {
                    // Validate arguments for non-parameterized built-in constructor calls too.
                    // Keep return as inference var for backwards compatibility.
                    self.check_builtin_constructor_args(
                        &scoped_ty,
                        &ResolvedType::Class(scoped_ty.clone()),
                        args,
                        span.clone(),
                    );
                    // Non-parameterized version - needs inference
                    self.fresh_type_var()
                } else {
                    self.error(
                        format!("Unknown type: {}", format_diagnostic_class_name(&scoped_ty)),
                        span,
                    );
                    ResolvedType::Unknown
                }
            }

            Expr::Lambda { params, body } => {
                self.enter_scope();
                let saved_return_type = self.current_return_type.clone();
                self.current_return_type = None;

                let param_types: Vec<ResolvedType> = params
                    .iter()
                    .map(|p| {
                        let ty = self.resolve_type(&p.ty);
                        self.declare_variable(&p.name, ty.clone(), p.mutable, span.clone());
                        ty
                    })
                    .collect();

                let return_type = self.check_builtin_argument_expr(&body.node, body.span.clone());

                self.current_return_type = saved_return_type;
                self.exit_scope();

                ResolvedType::Function(param_types, Box::new(return_type))
            }

            Expr::This => {
                if let Some(var) = self.lookup_variable("this") {
                    var.ty.clone()
                } else {
                    self.error("'this' used outside of class context".to_string(), span);
                    ResolvedType::Unknown
                }
            }

            Expr::StringInterp(parts) => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        let ty = self.check_builtin_argument_expr(&e.node, e.span.clone());
                        if matches!(ty, ResolvedType::Unknown) {
                            continue;
                        }
                        if !self.supports_display_expr(&e.node, &ty) {
                            self.error(
                                format!(
                                    "String interpolation currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got {}",
                                    Self::format_resolved_type_for_diagnostic(&ty)
                                ),
                                e.span.clone(),
                            );
                        }
                    }
                }
                ResolvedType::String
            }

            Expr::Try(inner) => {
                let inner_type = self.check_builtin_argument_expr(&inner.node, inner.span.clone());
                match inner_type {
                    ResolvedType::Option(inner) => {
                        if !matches!(self.current_return_type, Some(ResolvedType::Option(_))) {
                            self.error(
                                "'?' on Option requires the enclosing function to return Option"
                                    .to_string(),
                                span,
                            );
                            return ResolvedType::Unknown;
                        }
                        *inner
                    }
                    ResolvedType::Result(ok, err) => {
                        match &self.current_return_type {
                            Some(ResolvedType::Result(_, outer_err)) => {
                                if !self.types_compatible(outer_err, &err) {
                                    self.error(
                                        format!(
                                            "'?' error type mismatch: cannot propagate Result error {} into {}",
                                            Self::format_resolved_type_for_diagnostic(&err),
                                            Self::format_resolved_type_for_diagnostic(outer_err)
                                        ),
                                        span,
                                    );
                                    return ResolvedType::Unknown;
                                }
                            }
                            _ => {
                                self.error(
                                    "'?' on Result requires the enclosing function to return Result"
                                        .to_string(),
                                    span,
                                );
                                return ResolvedType::Unknown;
                            }
                        }
                        *ok
                    }
                    ResolvedType::Unknown => ResolvedType::Unknown,
                    _ => {
                        self.error(
                            format!(
                                "'?' operator can only be used on Option or Result, got {}",
                                Self::format_resolved_type_for_diagnostic(&inner_type)
                            ),
                            span,
                        );
                        ResolvedType::Unknown
                    }
                }
            }

            Expr::Borrow(inner) => {
                let inner_type = self.check_builtin_argument_expr(&inner.node, inner.span.clone());
                ResolvedType::Ref(Box::new(inner_type))
            }

            Expr::MutBorrow(inner) => {
                let inner_type = self.check_builtin_argument_expr(&inner.node, inner.span.clone());

                // Check that we're borrowing something mutable
                if let Expr::Ident(name) = &inner.node {
                    if let Some(var) = self.lookup_variable(name) {
                        if !var.mutable {
                            self.error(
                                format!("Cannot mutably borrow immutable variable '{}'", name),
                                inner.span.clone(),
                            );
                        }
                    }
                }

                ResolvedType::MutRef(Box::new(inner_type))
            }

            Expr::Deref(inner) => {
                let inner_type = self.check_expr(&inner.node, inner.span.clone());
                match inner_type {
                    ResolvedType::Ref(inner)
                    | ResolvedType::MutRef(inner)
                    | ResolvedType::Ptr(inner) => *inner,
                    ResolvedType::Unknown => ResolvedType::Unknown,
                    _ => {
                        self.error(
                            format!(
                                "Cannot dereference non-pointer type {}",
                                Self::format_resolved_type_for_diagnostic(&inner_type)
                            ),
                            span,
                        );
                        ResolvedType::Unknown
                    }
                }
            }

            Expr::Match { expr, arms } => {
                let match_type = self.check_builtin_argument_expr(&expr.node, expr.span.clone());
                let mut result_type: Option<ResolvedType> = None;

                for arm in arms {
                    self.enter_scope();
                    self.check_pattern(&arm.pattern, &match_type, span.clone());
                    let mut arm_type = ResolvedType::None;
                    for stmt in &arm.body {
                        match &stmt.node {
                            Stmt::Expr(expr) => {
                                arm_type = self.check_expr(&expr.node, expr.span.clone());
                            }
                            _ => self.check_stmt(&stmt.node, stmt.span.clone()),
                        }
                    }
                    self.exit_scope();

                    if let Some(expected) = &result_type {
                        if let Some(common_type) = self.common_compatible_type(expected, &arm_type)
                        {
                            result_type = Some(common_type);
                        } else {
                            self.error(
                                format!(
                                    "Match expression arm type mismatch: expected {}, got {}",
                                    Self::format_resolved_type_for_diagnostic(expected),
                                    Self::format_resolved_type_for_diagnostic(&arm_type)
                                ),
                                span.clone(),
                            );
                        }
                    } else {
                        result_type = Some(arm_type);
                    }
                }

                if !self.match_expression_exhaustive(&match_type, arms) {
                    self.error(
                        format!(
                            "Non-exhaustive match expression for type {}",
                            Self::format_resolved_type_for_diagnostic(&match_type)
                        ),
                        span,
                    );
                }

                result_type.unwrap_or(ResolvedType::None)
            }

            Expr::Await(inner) => {
                let inner_type = self.check_expr(&inner.node, inner.span.clone());
                // await on Task<T> yields T
                match inner_type {
                    ResolvedType::Task(inner) => *inner,
                    ResolvedType::Unknown => ResolvedType::Unknown,
                    _ => {
                        self.error(
                            format!(
                                "'await' can only be used on Task types, got {}",
                                Self::format_resolved_type_for_diagnostic(&inner_type)
                            ),
                            span,
                        );
                        ResolvedType::Unknown
                    }
                }
            }

            Expr::AsyncBlock(body) => self.check_async_block_expr(body, span, None),

            Expr::Require { condition, message } => {
                let cond_type = self.check_expr(&condition.node, condition.span.clone());
                if !matches!(cond_type, ResolvedType::Unknown)
                    && !matches!(cond_type, ResolvedType::Boolean)
                {
                    self.error(
                        format!(
                            "require() condition must be Boolean, got {}",
                            Self::format_resolved_type_for_diagnostic(&cond_type)
                        ),
                        condition.span.clone(),
                    );
                }
                if let Some(msg) = message {
                    let msg_type = self.check_builtin_argument_expr(&msg.node, msg.span.clone());
                    if !matches!(msg_type, ResolvedType::Unknown)
                        && !matches!(msg_type, ResolvedType::String)
                    {
                        self.error(
                            format!(
                                "require() message must be String, got {}",
                                Self::format_resolved_type_for_diagnostic(&msg_type)
                            ),
                            msg.span.clone(),
                        );
                    }
                }
                ResolvedType::None
            }

            Expr::Range {
                start,
                end,
                inclusive: _,
            } => {
                if let Some(s) = start {
                    let start_type = self.check_builtin_argument_expr(&s.node, s.span.clone());
                    if !matches!(start_type, ResolvedType::Unknown)
                        && !matches!(start_type, ResolvedType::Integer)
                    {
                        self.error(
                            format!(
                                "Range start must be Integer, got {}",
                                Self::format_resolved_type_for_diagnostic(&start_type)
                            ),
                            s.span.clone(),
                        );
                    }
                }
                if let Some(e) = end {
                    let end_type = self.check_builtin_argument_expr(&e.node, e.span.clone());
                    if !matches!(end_type, ResolvedType::Unknown)
                        && !matches!(end_type, ResolvedType::Integer)
                    {
                        self.error(
                            format!(
                                "Range end must be Integer, got {}",
                                Self::format_resolved_type_for_diagnostic(&end_type)
                            ),
                            e.span.clone(),
                        );
                    }
                }
                ResolvedType::Range(Box::new(ResolvedType::Integer))
            }

            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_type = self.check_expr(&condition.node, condition.span.clone());
                if !matches!(cond_type, ResolvedType::Unknown)
                    && !matches!(cond_type, ResolvedType::Boolean)
                {
                    self.error(
                        format!(
                            "If condition must be Boolean, got {}",
                            Self::format_resolved_type_for_diagnostic(&cond_type)
                        ),
                        condition.span.clone(),
                    );
                }

                self.enter_scope();
                let mut then_type = ResolvedType::None;
                for stmt in then_branch {
                    match &stmt.node {
                        Stmt::Expr(expr) => {
                            then_type = self.check_expr(&expr.node, expr.span.clone());
                        }
                        _ => self.check_stmt(&stmt.node, stmt.span.clone()),
                    }
                }
                self.exit_scope();

                let has_else = else_branch.is_some();
                if let Some(else_stmts) = else_branch {
                    self.enter_scope();
                    let mut else_type = ResolvedType::None;
                    for stmt in else_stmts {
                        match &stmt.node {
                            Stmt::Expr(expr) => {
                                else_type = self.check_expr(&expr.node, expr.span.clone());
                            }
                            _ => self.check_stmt(&stmt.node, stmt.span.clone()),
                        }
                    }
                    self.exit_scope();

                    if let Some(common_type) = self.common_compatible_type(&then_type, &else_type) {
                        then_type = common_type;
                    } else {
                        self.error(
                            format!(
                                "If expression branch type mismatch: then is {}, else is {}",
                                Self::format_resolved_type_for_diagnostic(&then_type),
                                Self::format_resolved_type_for_diagnostic(&else_type)
                            ),
                            condition.span.clone(),
                        );
                        then_type = ResolvedType::Unknown;
                    }
                }

                if has_else {
                    then_type
                } else {
                    ResolvedType::None
                }
            }

            Expr::Block(body) => {
                self.enter_scope();
                let mut result_type = ResolvedType::None;
                for stmt in body {
                    if let Stmt::Expr(expr) = &stmt.node {
                        result_type = self.check_expr(&expr.node, expr.span.clone());
                    }
                    self.check_stmt(&stmt.node, stmt.span.clone());
                }
                self.exit_scope();
                result_type
            }
        }
    }

    pub(super) fn check_call(
        &mut self,
        callee: &Expr,
        args: &[Spanned<Expr>],
        type_args: &[Type],
        span: Span,
    ) -> ResolvedType {
        let canonical_ident_call = match callee {
            Expr::Ident(name) => self
                .resolve_import_alias_symbol(name)
                .or_else(|| self.resolve_wildcard_import_symbol(name)),
            _ => None,
        };
        let aliased_variant_call = match callee {
            Expr::Ident(name)
                if canonical_ident_call.as_deref().is_none_or(|resolved| {
                    Self::builtin_function_value_type(resolved).is_none()
                }) =>
            {
                self.resolve_import_alias_variant(name)
            }
            _ => None,
        };

        // 1. Built-in functions (special handling for println, etc.)
        if let Expr::Ident(name) = callee {
            if let Some((enum_name, variant_name)) = &aliased_variant_call {
                if !type_args.is_empty() {
                    self.error(
                        format!(
                            "Enum variant '{}.{}' does not accept type arguments",
                            enum_name, variant_name
                        ),
                        span.clone(),
                    );
                }
                if let Some(enum_info) = self.enums.get(enum_name).cloned() {
                    if let Some(field_types) = enum_info.variants.get(variant_name) {
                        if args.len() != field_types.len() {
                            self.error(
                                format!(
                                    "Enum variant '{}.{}' expects {} argument(s), got {}",
                                    enum_name,
                                    variant_name,
                                    field_types.len(),
                                    args.len()
                                ),
                                span.clone(),
                            );
                        } else {
                            for (arg, expected_ty) in args.iter().zip(field_types.iter()) {
                                let actual = self.check_expr_with_expected_type(
                                    &arg.node,
                                    arg.span.clone(),
                                    Some(expected_ty),
                                );
                                if !self.types_compatible(expected_ty, &actual) {
                                    self.error(
                                        format!(
                                            "Enum variant argument type mismatch: expected {}, got {}",
                                            Self::format_resolved_type_for_diagnostic(expected_ty),
                                            Self::format_resolved_type_for_diagnostic(&actual)
                                        ),
                                        arg.span.clone(),
                                    );
                                }
                            }
                        }
                        return ResolvedType::Class(enum_name.clone());
                    }
                }
            }
            if self.lookup_variable(name).is_none()
                && self
                    .resolve_nominal_reference_name(name)
                    .is_some_and(|resolved| self.classes.contains_key(&resolved))
            {
                let call_type_source = if type_args.is_empty() {
                    name.clone()
                } else {
                    format!(
                        "{}<{}>",
                        name,
                        type_args
                            .iter()
                            .map(Self::format_ast_type_source)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                return self.check_expr(
                    &Expr::Construct {
                        ty: call_type_source,
                        args: args.to_vec(),
                    },
                    span.clone(),
                );
            }
            let resolved_name = canonical_ident_call.as_deref().unwrap_or(name);
            if !type_args.is_empty() && Self::builtin_required_effect(resolved_name).is_some() {
                self.error(
                    format!(
                        "Built-in function '{}' does not accept type arguments",
                        resolved_name
                    ),
                    span.clone(),
                );
            }
            if let Some(required) = Self::builtin_required_effect(resolved_name) {
                self.enforce_required_effect(required, span.clone(), resolved_name);
            }
            if let Some(return_type) = self.check_builtin_call(resolved_name, args, span.clone()) {
                return return_type;
            }
        }

        // 2. Method call
        if let Expr::Field { object, field } = callee {
            if let Some(path_parts) = flatten_field_chain(callee) {
                if path_parts.len() >= 2 {
                    if let Some(alias_path) = self.lookup_import_alias_path(&path_parts[0]) {
                        let full_alias_path =
                            format!("{}.{}", alias_path, path_parts[1..].join("."));
                        if let Some(canonical) =
                            crate::ast::builtin_exact_import_alias_canonical(&full_alias_path)
                        {
                            if let Some(return_type) =
                                self.check_builtin_call(canonical, args, span.clone())
                            {
                                return return_type;
                            }
                        }
                    }
                    let full_path = path_parts.join(".");
                    let call_type_source = if type_args.is_empty() {
                        full_path.clone()
                    } else {
                        format!(
                            "{}<{}>",
                            full_path,
                            type_args
                                .iter()
                                .map(Self::format_ast_type_source)
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    };
                    if self
                        .resolve_nominal_reference_name(&full_path)
                        .is_some_and(|resolved| {
                            self.classes.contains_key(&resolved)
                                || self.interfaces.contains_key(&resolved)
                        })
                    {
                        return self.check_expr(
                            &Expr::Construct {
                                ty: call_type_source,
                                args: args.to_vec(),
                            },
                            span.clone(),
                        );
                    }

                    let owner_source = path_parts[..path_parts.len() - 1].join(".");
                    if let Some(resolved_owner) = self.resolve_nominal_reference_name(&owner_source)
                    {
                        if let Some(enum_info) = self.enums.get(&resolved_owner).cloned() {
                            if let Some(field_types) = enum_info.variants.get(field) {
                                if !type_args.is_empty() {
                                    self.error(
                                        format!(
                                            "Enum variant '{}.{}' does not accept type arguments",
                                            owner_source, field
                                        ),
                                        span.clone(),
                                    );
                                    return ResolvedType::Unknown;
                                }
                                if args.len() != field_types.len() {
                                    self.error(
                                        format!(
                                            "Enum variant '{}.{}' expects {} argument(s), got {}",
                                            owner_source,
                                            field,
                                            field_types.len(),
                                            args.len()
                                        ),
                                        span.clone(),
                                    );
                                } else {
                                    for (arg, expected_ty) in args.iter().zip(field_types.iter()) {
                                        let actual = self.check_expr_with_expected_type(
                                            &arg.node,
                                            arg.span.clone(),
                                            Some(expected_ty),
                                        );
                                        if !self.types_compatible(expected_ty, &actual) {
                                            self.error(
                                                format!(
                                                    "Enum variant argument type mismatch: expected {}, got {}",
                                                    Self::format_resolved_type_for_diagnostic(expected_ty),
                                                    Self::format_resolved_type_for_diagnostic(&actual)
                                                ),
                                                arg.span.clone(),
                                            );
                                        }
                                    }
                                }
                                return ResolvedType::Class(resolved_owner);
                            }
                        }
                    }

                    if let Some(candidate) =
                        self.resolve_import_alias_module_candidate(&path_parts[0], &path_parts[1..])
                    {
                        let resolved = self
                            .resolve_function_value_name(&candidate)
                            .unwrap_or(&candidate)
                            .to_string();
                        if let Some(sig) = self.functions.get(&resolved).cloned() {
                            self.enforce_call_effects(&sig, span.clone(), &resolved);
                            let (inst_params, inst_return_type, valid_explicit_type_args) = self
                                .instantiate_signature_for_call(
                                    &resolved,
                                    &sig,
                                    type_args,
                                    span.clone(),
                                );
                            if !valid_explicit_type_args {
                                return ResolvedType::Unknown;
                            }
                            let expected = inst_params.len();
                            let bad_arity = if sig.is_variadic {
                                args.len() < expected
                            } else {
                                args.len() != expected
                            };
                            if bad_arity {
                                self.error(
                                    format!(
                                        "Function '{}' expects {} arguments, got {}",
                                        resolved,
                                        if sig.is_variadic {
                                            format!("at least {}", expected)
                                        } else {
                                            expected.to_string()
                                        },
                                        args.len()
                                    ),
                                    span.clone(),
                                );
                            } else {
                                for (arg, (_, param_type)) in args.iter().zip(inst_params.iter()) {
                                    let arg_type = self.check_expr_with_expected_type(
                                        &arg.node,
                                        arg.span.clone(),
                                        Some(param_type),
                                    );
                                    self.enforce_function_argument_effect_contract(
                                        param_type,
                                        &arg.node,
                                        arg.span.clone(),
                                    );
                                    if !self.types_compatible(param_type, &arg_type) {
                                        self.error(
                                            format!(
                                                "Argument type mismatch: expected {}, got {}",
                                                Self::format_resolved_type_for_diagnostic(
                                                    param_type
                                                ),
                                                Self::format_resolved_type_for_diagnostic(
                                                    &arg_type
                                                )
                                            ),
                                            arg.span.clone(),
                                        );
                                    }
                                }
                                if sig.is_variadic && sig.is_extern {
                                    self.check_variadic_ffi_tail_args(&resolved, args, expected);
                                }
                            }
                            return inst_return_type;
                        }
                        if let Some(return_type) =
                            self.check_builtin_call(&resolved, args, span.clone())
                        {
                            return return_type;
                        }
                    }
                    if let Some(candidate) = self.resolve_wildcard_import_module_function_candidate(
                        &path_parts[0],
                        &path_parts[1..],
                    ) {
                        if let Some(sig) = self.functions.get(&candidate).cloned() {
                            self.enforce_call_effects(&sig, span.clone(), &candidate);
                            let (inst_params, inst_return_type, valid_explicit_type_args) = self
                                .instantiate_signature_for_call(
                                    &candidate,
                                    &sig,
                                    type_args,
                                    span.clone(),
                                );
                            if !valid_explicit_type_args {
                                return ResolvedType::Unknown;
                            }
                            let expected = inst_params.len();
                            let bad_arity = if sig.is_variadic {
                                args.len() < expected
                            } else {
                                args.len() != expected
                            };
                            if bad_arity {
                                self.error(
                                    format!(
                                        "Function '{}' expects {} arguments, got {}",
                                        candidate,
                                        if sig.is_variadic {
                                            format!("at least {}", expected)
                                        } else {
                                            expected.to_string()
                                        },
                                        args.len()
                                    ),
                                    span.clone(),
                                );
                            } else {
                                for (arg, (_, param_type)) in args.iter().zip(inst_params.iter()) {
                                    let arg_type = self.check_expr_with_expected_type(
                                        &arg.node,
                                        arg.span.clone(),
                                        Some(param_type),
                                    );
                                    self.enforce_function_argument_effect_contract(
                                        param_type,
                                        &arg.node,
                                        arg.span.clone(),
                                    );
                                    if !self.types_compatible(param_type, &arg_type) {
                                        self.error(
                                            format!(
                                                "Argument type mismatch: expected {}, got {}",
                                                Self::format_resolved_type_for_diagnostic(
                                                    param_type
                                                ),
                                                Self::format_resolved_type_for_diagnostic(
                                                    &arg_type
                                                )
                                            ),
                                            arg.span.clone(),
                                        );
                                    }
                                }
                                if sig.is_variadic && sig.is_extern {
                                    self.check_variadic_ffi_tail_args(&candidate, args, expected);
                                }
                            }
                            return inst_return_type;
                        }
                    }

                    let mangled = path_parts.join("__");
                    if let Some(sig) = self.functions.get(&mangled).cloned() {
                        self.enforce_call_effects(&sig, span.clone(), &mangled);
                        let (inst_params, inst_return_type, valid_explicit_type_args) = self
                            .instantiate_signature_for_call(
                                &mangled,
                                &sig,
                                type_args,
                                span.clone(),
                            );
                        if !valid_explicit_type_args {
                            return ResolvedType::Unknown;
                        }
                        let expected = inst_params.len();
                        let bad_arity = if sig.is_variadic {
                            args.len() < expected
                        } else {
                            args.len() != expected
                        };
                        if bad_arity {
                            self.error(
                                format!(
                                    "Function '{}' expects {} arguments, got {}",
                                    mangled,
                                    if sig.is_variadic {
                                        format!("at least {}", expected)
                                    } else {
                                        expected.to_string()
                                    },
                                    args.len()
                                ),
                                span.clone(),
                            );
                        } else {
                            for (arg, (_, param_type)) in args.iter().zip(inst_params.iter()) {
                                let arg_type = self.check_expr_with_expected_type(
                                    &arg.node,
                                    arg.span.clone(),
                                    Some(param_type),
                                );
                                self.enforce_function_argument_effect_contract(
                                    param_type,
                                    &arg.node,
                                    arg.span.clone(),
                                );
                                if !self.types_compatible(param_type, &arg_type) {
                                    self.error(
                                        format!(
                                            "Argument type mismatch: expected {}, got {}",
                                            Self::format_resolved_type_for_diagnostic(param_type),
                                            Self::format_resolved_type_for_diagnostic(&arg_type)
                                        ),
                                        arg.span.clone(),
                                    );
                                }
                            }
                            if sig.is_variadic && sig.is_extern {
                                self.check_variadic_ffi_tail_args(&mangled, args, expected);
                            }
                        }
                        return inst_return_type;
                    }
                }
            }

            // Special handling for static calls (e.g. File.read, Time.now)
            if let Expr::Ident(name) = &object.node {
                if let Some(canonical_builtin) =
                    crate::ast::builtin_exact_import_alias_canonical(&format!("{}.{}", name, field))
                {
                    if !type_args.is_empty() {
                        self.error(
                            format!(
                                "Built-in function '{}' does not accept type arguments",
                                canonical_builtin.replace("__", ".")
                            ),
                            span.clone(),
                        );
                    }
                    match canonical_builtin {
                        "Option__some" => {
                            self.check_arg_count("Option.some", args, 1, span.clone());
                            let inner = if let Some(arg) = args.first() {
                                self.check_builtin_argument_expr(&arg.node, arg.span.clone())
                            } else {
                                ResolvedType::Unknown
                            };
                            return ResolvedType::Option(Box::new(inner));
                        }
                        "Option__none" => {
                            self.check_arg_count("Option.none", args, 0, span.clone());
                            return ResolvedType::Option(Box::new(self.fresh_type_var()));
                        }
                        "Result__ok" => {
                            self.check_arg_count("Result.ok", args, 1, span.clone());
                            let ok_ty = if let Some(arg) = args.first() {
                                self.check_builtin_argument_expr(&arg.node, arg.span.clone())
                            } else {
                                ResolvedType::Unknown
                            };
                            return ResolvedType::Result(
                                Box::new(ok_ty),
                                Box::new(self.fresh_type_var()),
                            );
                        }
                        "Result__error" => {
                            self.check_arg_count("Result.error", args, 1, span.clone());
                            let err_ty = if let Some(arg) = args.first() {
                                self.check_builtin_argument_expr(&arg.node, arg.span.clone())
                            } else {
                                ResolvedType::Unknown
                            };
                            return ResolvedType::Result(
                                Box::new(self.fresh_type_var()),
                                Box::new(err_ty),
                            );
                        }
                        _ => {}
                    }
                }
                match name.as_str() {
                    "Option" => {
                        if !type_args.is_empty() {
                            self.error(
                                "Option static methods do not accept explicit type arguments"
                                    .to_string(),
                                span.clone(),
                            );
                        }
                        match field.as_str() {
                            "some" => {
                                self.check_arg_count("Option.some", args, 1, span.clone());
                                let inner = if let Some(arg) = args.first() {
                                    self.check_builtin_argument_expr(&arg.node, arg.span.clone())
                                } else {
                                    ResolvedType::Unknown
                                };
                                return ResolvedType::Option(Box::new(inner));
                            }
                            "none" => {
                                self.check_arg_count("Option.none", args, 0, span.clone());
                                return ResolvedType::Option(Box::new(self.fresh_type_var()));
                            }
                            _ => {}
                        }
                    }
                    "Result" => {
                        if !type_args.is_empty() {
                            self.error(
                                "Result static methods do not accept explicit type arguments"
                                    .to_string(),
                                span.clone(),
                            );
                        }
                        match field.as_str() {
                            "ok" => {
                                self.check_arg_count("Result.ok", args, 1, span.clone());
                                let ok_ty = if let Some(arg) = args.first() {
                                    self.check_builtin_argument_expr(&arg.node, arg.span.clone())
                                } else {
                                    ResolvedType::Unknown
                                };
                                return ResolvedType::Result(
                                    Box::new(ok_ty),
                                    Box::new(self.fresh_type_var()),
                                );
                            }
                            "error" => {
                                self.check_arg_count("Result.error", args, 1, span.clone());
                                let err_ty = if let Some(arg) = args.first() {
                                    self.check_builtin_argument_expr(&arg.node, arg.span.clone())
                                } else {
                                    ResolvedType::Unknown
                                };
                                return ResolvedType::Result(
                                    Box::new(self.fresh_type_var()),
                                    Box::new(err_ty),
                                );
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }

                if let Some(canonical) = self.resolve_stdlib_alias_call_name(name, field) {
                    if let Some(required) = Self::builtin_required_effect(&canonical) {
                        self.enforce_required_effect(required, span.clone(), &canonical);
                    }
                    if let Some(ret) = self.check_builtin_call(&canonical, args, span.clone()) {
                        return ret;
                    }
                }

                let resolved_module = self
                    .resolve_import_alias_symbol(name)
                    .or_else(|| self.resolve_nominal_reference_name(name))
                    .or_else(|| self.resolve_enum_name(name))
                    .unwrap_or_else(|| name.clone());

                if matches!(
                    resolved_module.as_str(),
                    "File" | "Time" | "System" | "Math" | "Str" | "Args"
                ) {
                    if !type_args.is_empty() {
                        self.error(
                            format!(
                                "Built-in function '{}.{}' does not accept type arguments",
                                resolved_module, field
                            ),
                            span.clone(),
                        );
                    }
                    let builtin_name = format!("{}__{}", resolved_module, field);
                    if let Some(required) = Self::builtin_required_effect(&builtin_name) {
                        self.enforce_required_effect(required, span.clone(), &builtin_name);
                    }
                    if let Some(ret) = self.check_builtin_call(&builtin_name, args, span.clone()) {
                        return ret;
                    }
                }

                // Enum variant constructor call: `Enum.Variant(...)`
                if let Some(enum_info) = self.enums.get(&resolved_module).cloned() {
                    if let Some(field_types) = enum_info.variants.get(field) {
                        if !type_args.is_empty() {
                            self.error(
                                format!(
                                    "Enum variant '{}.{}' does not accept type arguments",
                                    name, field
                                ),
                                span.clone(),
                            );
                            return ResolvedType::Unknown;
                        }
                        if args.len() != field_types.len() {
                            self.error(
                                format!(
                                    "Enum variant '{}.{}' expects {} argument(s), got {}",
                                    name,
                                    field,
                                    field_types.len(),
                                    args.len()
                                ),
                                span.clone(),
                            );
                        } else {
                            for (arg, expected_ty) in args.iter().zip(field_types.iter()) {
                                let actual = self.check_expr_with_expected_type(
                                    &arg.node,
                                    arg.span.clone(),
                                    Some(expected_ty),
                                );
                                if !self.types_compatible(expected_ty, &actual) {
                                    self.error(
                                        format!(
                                            "Enum variant argument type mismatch: expected {}, got {}",
                                            Self::format_resolved_type_for_diagnostic(expected_ty),
                                            Self::format_resolved_type_for_diagnostic(&actual)
                                        ),
                                        arg.span.clone(),
                                    );
                                }
                            }
                        }
                        return ResolvedType::Class(resolved_module.clone());
                    }
                }

                // Module dot syntax: `Module.func(...)` -> `Module__func(...)`
                let mangled = format!("{}__{}", resolved_module, field);
                if let Some(sig) = self.functions.get(&mangled).cloned() {
                    self.enforce_call_effects(&sig, span.clone(), &mangled);
                    let (inst_params, inst_return_type, valid_explicit_type_args) = self
                        .instantiate_signature_for_call(&mangled, &sig, type_args, span.clone());
                    if !valid_explicit_type_args {
                        return ResolvedType::Unknown;
                    }
                    let expected = inst_params.len();
                    let bad_arity = if sig.is_variadic {
                        args.len() < expected
                    } else {
                        args.len() != expected
                    };
                    if bad_arity {
                        self.error(
                            format!(
                                "Function '{}' expects {} arguments, got {}",
                                mangled,
                                if sig.is_variadic {
                                    format!("at least {}", expected)
                                } else {
                                    expected.to_string()
                                },
                                args.len()
                            ),
                            span.clone(),
                        );
                    } else {
                        for (arg, (_, param_type)) in args.iter().zip(inst_params.iter()) {
                            let arg_type = self.check_expr_with_expected_type(
                                &arg.node,
                                arg.span.clone(),
                                Some(param_type),
                            );
                            self.enforce_function_argument_effect_contract(
                                param_type,
                                &arg.node,
                                arg.span.clone(),
                            );
                            if !self.types_compatible(param_type, &arg_type) {
                                self.error(
                                    format!(
                                        "Argument type mismatch: expected {}, got {}",
                                        Self::format_resolved_type_for_diagnostic(param_type),
                                        Self::format_resolved_type_for_diagnostic(&arg_type)
                                    ),
                                    arg.span.clone(),
                                );
                            }
                        }
                        if sig.is_variadic && sig.is_extern {
                            self.check_variadic_ffi_tail_args(&mangled, args, expected);
                        }
                    }
                    return inst_return_type;
                }
            }

            let obj_type = self.check_builtin_argument_expr(&object.node, object.span.clone());
            if let ResolvedType::Class(name) = &obj_type {
                let (base_name, class_substitutions) = self.instantiated_class_substitutions(name);
                if let Some((owner, field_type, _, visibility)) =
                    self.lookup_class_field(&base_name, field)
                {
                    self.check_member_visibility(&owner, visibility, "Field", field, span.clone());
                    let field_type = Self::substitute_type_vars(&field_type, &class_substitutions);
                    if let ResolvedType::Function(param_types, return_type) = field_type {
                        if !type_args.is_empty() {
                            self.error(
                                format!(
                                    "Function-valued field '{}.{}' does not accept explicit type arguments",
                                    name, field
                                ),
                                span.clone(),
                            );
                        }
                        if args.len() != param_types.len() {
                            self.error(
                                format!(
                                    "Function field call expects {} arguments, got {}",
                                    param_types.len(),
                                    args.len()
                                ),
                                span.clone(),
                            );
                        } else {
                            for (arg, param_type) in args.iter().zip(param_types.iter()) {
                                let arg_type = self.check_expr_with_expected_type(
                                    &arg.node,
                                    arg.span.clone(),
                                    Some(param_type),
                                );
                                self.enforce_function_argument_effect_contract(
                                    param_type,
                                    &arg.node,
                                    arg.span.clone(),
                                );
                                if !self.types_compatible(param_type, &arg_type) {
                                    self.error(
                                        format!(
                                            "Argument type mismatch: expected {}, got {}",
                                            Self::format_resolved_type_for_diagnostic(param_type),
                                            Self::format_resolved_type_for_diagnostic(&arg_type)
                                        ),
                                        arg.span.clone(),
                                    );
                                }
                            }
                        }
                        return *return_type;
                    }
                }
            }
            return self.check_method_call(&obj_type, field, args, type_args, span);
        }

        // 3. Evaluate callee to see if it's a function type (handles global functions and local variables/params)
        if let Expr::Ident(name) = callee {
            let resolved_name = canonical_ident_call.as_deref().unwrap_or(name);
            if let Some(sig) = self.functions.get(resolved_name).cloned() {
                self.enforce_call_effects(&sig, span.clone(), resolved_name);
                let (inst_params, inst_return_type, valid_explicit_type_args) = self
                    .instantiate_signature_for_call(resolved_name, &sig, type_args, span.clone());
                if !valid_explicit_type_args {
                    return ResolvedType::Unknown;
                }
                let expected = inst_params.len();
                let bad_arity = if sig.is_variadic {
                    args.len() < expected
                } else {
                    args.len() != expected
                };
                if bad_arity {
                    self.error(
                        format!(
                            "Function '{}' expects {} arguments, got {}",
                            resolved_name,
                            if sig.is_variadic {
                                format!("at least {}", expected)
                            } else {
                                expected.to_string()
                            },
                            args.len()
                        ),
                        span.clone(),
                    );
                } else {
                    for (arg, (_, param_type)) in args.iter().zip(inst_params.iter()) {
                        let arg_type = self.check_expr_with_expected_type(
                            &arg.node,
                            arg.span.clone(),
                            Some(param_type),
                        );
                        self.enforce_function_argument_effect_contract(
                            param_type,
                            &arg.node,
                            arg.span.clone(),
                        );
                        if !self.types_compatible(param_type, &arg_type) {
                            self.error(
                                format!(
                                    "Argument type mismatch: expected {}, got {}",
                                    Self::format_resolved_type_for_diagnostic(param_type),
                                    Self::format_resolved_type_for_diagnostic(&arg_type)
                                ),
                                arg.span.clone(),
                            );
                        }
                    }
                    if sig.is_variadic && sig.is_extern {
                        self.check_variadic_ffi_tail_args(resolved_name, args, expected);
                    }
                }
                return inst_return_type;
            }
        }

        if !type_args.is_empty() {
            self.error(
                "Explicit type arguments are only supported on named function calls".to_string(),
                span.clone(),
            );
        }
        let callee_type = self.check_expr(callee, span.clone());
        if let ResolvedType::Function(param_types, return_type) = callee_type {
            if let Some(contract) = self.infer_function_value_effect_contract_from_expr(callee) {
                let callee_label = match callee {
                    Expr::Ident(name) => name.clone(),
                    _ => "function value".to_string(),
                };
                self.enforce_function_value_effect_contract(&contract, span.clone(), &callee_label);
            }
            if args.len() != param_types.len() {
                self.error(
                    format!(
                        "Function call expects {} arguments, got {}",
                        param_types.len(),
                        args.len()
                    ),
                    span,
                );
            } else {
                for (arg, param_type) in args.iter().zip(param_types.iter()) {
                    let arg_type = self.check_expr_with_expected_type(
                        &arg.node,
                        arg.span.clone(),
                        Some(param_type),
                    );
                    self.enforce_function_argument_effect_contract(
                        param_type,
                        &arg.node,
                        arg.span.clone(),
                    );
                    if !self.types_compatible(param_type, &arg_type) {
                        self.error(
                            format!(
                                "Argument type mismatch: expected {}, got {}",
                                Self::format_resolved_type_for_diagnostic(param_type),
                                Self::format_resolved_type_for_diagnostic(&arg_type)
                            ),
                            arg.span.clone(),
                        );
                    }
                }
            }
            return (*return_type).clone();
        }

        if callee_type != ResolvedType::Unknown {
            self.error(
                format!(
                    "Cannot call non-function type {}",
                    Self::format_resolved_type_for_diagnostic(&callee_type)
                ),
                span,
            );
        }
        ResolvedType::Unknown
    }

    pub(super) fn check_builtin_call(
        &mut self,
        name: &str,
        args: &[Spanned<Expr>],
        span: Span,
    ) -> Option<ResolvedType> {
        match name {
            "Option__some" => {
                self.check_arg_count("Option.some", args, 1, span.clone());
                let inner = if let Some(arg) = args.first() {
                    self.check_builtin_argument_expr(&arg.node, arg.span.clone())
                } else {
                    ResolvedType::Unknown
                };
                Some(ResolvedType::Option(Box::new(inner)))
            }
            "Option__none" => {
                self.check_arg_count("Option.none", args, 0, span);
                Some(ResolvedType::Option(Box::new(self.fresh_type_var())))
            }
            "Result__ok" => {
                self.check_arg_count("Result.ok", args, 1, span.clone());
                let ok_ty = if let Some(arg) = args.first() {
                    self.check_builtin_argument_expr(&arg.node, arg.span.clone())
                } else {
                    ResolvedType::Unknown
                };
                Some(ResolvedType::Result(
                    Box::new(ok_ty),
                    Box::new(self.fresh_type_var()),
                ))
            }
            "Result__error" => {
                self.check_arg_count("Result.error", args, 1, span.clone());
                let err_ty = if let Some(arg) = args.first() {
                    self.check_builtin_argument_expr(&arg.node, arg.span.clone())
                } else {
                    ResolvedType::Unknown
                };
                Some(ResolvedType::Result(
                    Box::new(self.fresh_type_var()),
                    Box::new(err_ty),
                ))
            }
            "println" | "print" => {
                for arg in args {
                    let ty = self.check_builtin_argument_expr(&arg.node, arg.span.clone());
                    if matches!(ty, ResolvedType::Unknown) {
                        continue;
                    }
                    if !self.supports_display_expr(&arg.node, &ty) {
                        self.error(
                            format!(
                                "{}() currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got {}",
                                name,
                                Self::format_resolved_type_for_diagnostic(&ty)
                            ),
                            arg.span.clone(),
                        );
                    }
                }
                Some(ResolvedType::None)
            }
            "read_line" => {
                self.check_arg_count(name, args, 0, span);
                Some(ResolvedType::String)
            }
            "Math__abs" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !t.is_numeric() {
                        self.error(
                            format!(
                                "Math.abs() requires numeric type, got {}",
                                Self::format_resolved_type_for_diagnostic(&t)
                            ),
                            span,
                        );
                    }
                    Some(t)
                } else {
                    Some(ResolvedType::Unknown)
                }
            }
            "Math__min" | "Math__max" => {
                let func_name = if name.contains("min") {
                    "Math.min"
                } else {
                    "Math.max"
                };
                self.check_arg_count(name, args, 2, span.clone());
                if args.len() >= 2 {
                    let t1 = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    let t2 = self.check_builtin_argument_expr(&args[1].node, args[1].span.clone());
                    if matches!(t1, ResolvedType::Unknown) || matches!(t2, ResolvedType::Unknown) {
                        Some(ResolvedType::Unknown)
                    } else if !t1.is_numeric() || !t2.is_numeric() {
                        self.error(
                            format!(
                                "{}() arguments must be numeric types, got {} and {}",
                                func_name,
                                Self::format_resolved_type_for_diagnostic(&t1),
                                Self::format_resolved_type_for_diagnostic(&t2)
                            ),
                            span,
                        );
                        Some(ResolvedType::Unknown)
                    } else if let Some(common_type) = self.common_compatible_type(&t1, &t2) {
                        Some(common_type)
                    } else {
                        self.error(
                            format!(
                                "{}() arguments must have same type: {} vs {}",
                                func_name,
                                Self::format_resolved_type_for_diagnostic(&t1),
                                Self::format_resolved_type_for_diagnostic(&t2)
                            ),
                            span,
                        );
                        Some(ResolvedType::Unknown)
                    }
                } else {
                    Some(ResolvedType::Unknown)
                }
            }
            "Math__sqrt" | "Math__sin" | "Math__cos" | "Math__tan" | "Math__floor"
            | "Math__ceil" | "Math__round" | "Math__log" | "Math__log10" | "Math__exp" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !t.is_numeric() {
                        self.error(
                            format!(
                                "{}() requires numeric type, got {}",
                                name.replace("__", "."),
                                Self::format_resolved_type_for_diagnostic(&t)
                            ),
                            span,
                        );
                    }
                }
                Some(ResolvedType::Float)
            }
            "Math__pow" => {
                self.check_arg_count(name, args, 2, span.clone());
                if args.len() >= 2 {
                    let t1 = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    let t2 = self.check_builtin_argument_expr(&args[1].node, args[1].span.clone());
                    if !matches!(t1, ResolvedType::Unknown)
                        && !matches!(t2, ResolvedType::Unknown)
                        && (!t1.is_numeric() || !t2.is_numeric())
                    {
                        self.error("Math.pow() requires numeric types".to_string(), span);
                    }
                }
                Some(ResolvedType::Float)
            }
            "to_float" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown)
                        && !matches!(t, ResolvedType::Integer | ResolvedType::Float)
                    {
                        self.error(
                            format!(
                                "to_float() requires Integer or Float, got {}",
                                Self::format_resolved_type_for_diagnostic(&t)
                            ),
                            span,
                        );
                    }
                }
                Some(ResolvedType::Float)
            }
            "to_int" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown)
                        && !matches!(
                            t,
                            ResolvedType::Integer | ResolvedType::Float | ResolvedType::String
                        )
                    {
                        self.error(
                            format!(
                                "to_int() requires Integer, Float, or String, got {}",
                                Self::format_resolved_type_for_diagnostic(&t)
                            ),
                            span,
                        );
                    }
                }
                Some(ResolvedType::Integer)
            }
            "to_string" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if matches!(t, ResolvedType::Unknown) {
                        return Some(ResolvedType::String);
                    }
                    if !self.supports_display_expr(&args[0].node, &t) {
                        self.error(
                            format!(
                                "to_string() currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got {}",
                                Self::format_resolved_type_for_diagnostic(&t)
                            ),
                            span,
                        );
                    }
                }
                Some(ResolvedType::String)
            }
            "Str__len" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::String) {
                        self.error(
                            format!(
                                "Str.len() requires String, got {}",
                                Self::format_resolved_type_for_diagnostic(&t)
                            ),
                            span,
                        );
                    }
                }
                Some(ResolvedType::Integer)
            }
            "Str__compare" => {
                self.check_arg_count(name, args, 2, span.clone());
                if args.len() >= 2 {
                    for arg in &args[..2] {
                        let t = self.check_builtin_argument_expr(&arg.node, arg.span.clone());
                        if matches!(t, ResolvedType::Unknown) {
                            continue;
                        }
                        if !matches!(t, ResolvedType::String) {
                            self.error(
                                "Str.compare() requires String arguments".to_string(),
                                arg.span.clone(),
                            );
                        }
                    }
                }
                Some(ResolvedType::Integer)
            }
            "Str__concat" => {
                self.check_arg_count(name, args, 2, span.clone());
                if args.len() >= 2 {
                    for arg in &args[..2] {
                        let t = self.check_builtin_argument_expr(&arg.node, arg.span.clone());
                        if matches!(t, ResolvedType::Unknown) {
                            continue;
                        }
                        if !matches!(t, ResolvedType::String) {
                            self.error(
                                "Str.concat() requires String arguments".to_string(),
                                arg.span.clone(),
                            );
                        }
                    }
                }
                Some(ResolvedType::String)
            }
            "Str__upper" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::String) {
                        self.error("Str.upper() requires String".to_string(), span.clone());
                    }
                }
                Some(ResolvedType::String)
            }
            "Str__lower" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::String) {
                        self.error("Str.lower() requires String".to_string(), span.clone());
                    }
                }
                Some(ResolvedType::String)
            }
            "Str__trim" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::String) {
                        self.error("Str.trim() requires String".to_string(), span.clone());
                    }
                }
                Some(ResolvedType::String)
            }
            "Str__contains" => {
                self.check_arg_count(name, args, 2, span.clone());
                if args.len() >= 2 {
                    let t1 = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    let t2 = self.check_builtin_argument_expr(&args[1].node, args[1].span.clone());
                    if matches!(t1, ResolvedType::Unknown) || matches!(t2, ResolvedType::Unknown) {
                        return Some(ResolvedType::Boolean);
                    }
                    if !matches!(t1, ResolvedType::String) || !matches!(t2, ResolvedType::String) {
                        self.error(
                            "Str.contains() requires two String arguments".to_string(),
                            span.clone(),
                        );
                    }
                }
                Some(ResolvedType::Boolean)
            }
            "Str__startsWith" | "Str__endsWith" => {
                self.check_arg_count(name, args, 2, span.clone());
                if args.len() >= 2 {
                    let t1 = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    let t2 = self.check_builtin_argument_expr(&args[1].node, args[1].span.clone());
                    if matches!(t1, ResolvedType::Unknown) || matches!(t2, ResolvedType::Unknown) {
                        return Some(ResolvedType::Boolean);
                    }
                    if !matches!(t1, ResolvedType::String) || !matches!(t2, ResolvedType::String) {
                        let mut parts = name.split("__");
                        let owner = parts.next().unwrap_or("Str");
                        let method = parts.next().unwrap_or(name);
                        self.error(
                            format!("{}.{}() requires two String arguments", owner, method),
                            span.clone(),
                        );
                    }
                }
                Some(ResolvedType::Boolean)
            }
            "System__exit" | "exit" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::Integer) {
                        self.error("exit() requires Integer code".to_string(), span);
                    }
                }
                Some(ResolvedType::None)
            }
            "range" => {
                // range(start, end) or range(start, end, step) -> Range<Integer|Float>
                if args.len() < 2 || args.len() > 3 {
                    self.error("range() requires 2 or 3 arguments: range(start, end) or range(start, end, step)".to_string(), span.clone());
                }
                let mut range_ty = ResolvedType::Unknown;
                if let Some(first_arg) = args.first() {
                    let first_ty =
                        self.check_builtin_argument_expr(&first_arg.node, first_arg.span.clone());
                    if matches!(first_ty, ResolvedType::Unknown) {
                        return Some(ResolvedType::Range(Box::new(ResolvedType::Unknown)));
                    }
                    if !matches!(first_ty, ResolvedType::Integer | ResolvedType::Float) {
                        self.error(
                            "range() arguments must be all Integer or all Float".to_string(),
                            span.clone(),
                        );
                    } else {
                        range_ty = first_ty.clone();
                    }
                    for arg in &args[1..] {
                        let arg_ty = self.check_builtin_argument_expr(&arg.node, arg.span.clone());
                        if matches!(arg_ty, ResolvedType::Unknown) {
                            return Some(ResolvedType::Range(Box::new(range_ty)));
                        }
                        if !matches!(arg_ty, ResolvedType::Integer | ResolvedType::Float) {
                            self.error(
                                "range() arguments must be all Integer or all Float".to_string(),
                                span.clone(),
                            );
                            continue;
                        }
                        if !matches!(range_ty, ResolvedType::Unknown) && arg_ty != range_ty {
                            self.error(
                                format!(
                                    "range() arguments must use the same numeric type, got {} and {}",
                                    Self::format_resolved_type_for_diagnostic(&range_ty),
                                    Self::format_resolved_type_for_diagnostic(&arg_ty)
                                ),
                                arg.span.clone(),
                            );
                        }
                    }
                }
                if let Some(step) = args.get(2) {
                    if Self::eval_numeric_const_expr(&step.node).is_some_and(NumericConst::is_zero)
                    {
                        self.error("range() step cannot be 0".to_string(), step.span.clone());
                    }
                }
                Some(ResolvedType::Range(Box::new(range_ty)))
            }
            // File I/O
            "File__read" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::String) {
                        self.error(
                            format!(
                                "File.read() requires String path, got {}",
                                Self::format_resolved_type_for_diagnostic(&t)
                            ),
                            span,
                        );
                    }
                }
                Some(ResolvedType::String)
            }
            "File__write" => {
                self.check_arg_count(name, args, 2, span.clone());
                if args.len() >= 2 {
                    let path_t =
                        self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    let content_t =
                        self.check_builtin_argument_expr(&args[1].node, args[1].span.clone());
                    if !matches!(path_t, ResolvedType::Unknown)
                        && !matches!(path_t, ResolvedType::String)
                    {
                        self.error(
                            "File.write() path must be String".to_string(),
                            args[0].span.clone(),
                        );
                    }
                    if !matches!(content_t, ResolvedType::Unknown)
                        && !matches!(content_t, ResolvedType::String)
                    {
                        self.error(
                            "File.write() content must be String".to_string(),
                            args[1].span.clone(),
                        );
                    }
                }
                Some(ResolvedType::Boolean)
            }
            "File__exists" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::String) {
                        self.error(
                            format!(
                                "File.exists() requires String path, got {}",
                                Self::format_resolved_type_for_diagnostic(&t)
                            ),
                            span,
                        );
                    }
                }
                Some(ResolvedType::Boolean)
            }
            "File__delete" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::String) {
                        self.error(
                            format!(
                                "File.delete() requires String path, got {}",
                                Self::format_resolved_type_for_diagnostic(&t)
                            ),
                            span,
                        );
                    }
                }
                Some(ResolvedType::Boolean)
            }
            // Time Functions
            "Time__now" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::String) {
                        self.error(
                            "Time.now() requires String format".to_string(),
                            span.clone(),
                        );
                    }
                }
                Some(ResolvedType::String)
            }
            "Time__unix" => {
                self.check_arg_count(name, args, 0, span);
                Some(ResolvedType::Integer)
            }
            "Time__sleep" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::Integer) {
                        self.error(
                            "Time.sleep() requires Integer milliseconds".to_string(),
                            span,
                        );
                    } else {
                        self.check_non_negative_integer_const(
                            &args[0].node,
                            args[0].span.clone(),
                            "Time.sleep() milliseconds must be non-negative",
                        );
                    }
                }
                Some(ResolvedType::None)
            }
            // System Functions
            "System__getenv" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::String) {
                        self.error(
                            "System.getenv() requires String name".to_string(),
                            span.clone(),
                        );
                    }
                }
                Some(ResolvedType::String)
            }
            "System__shell" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::String) {
                        self.error(
                            "System.shell() requires String command".to_string(),
                            span.clone(),
                        );
                    }
                }
                Some(ResolvedType::Integer)
            }
            "System__exec" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::String) {
                        self.error(
                            "System.exec() requires String command".to_string(),
                            span.clone(),
                        );
                    }
                }
                Some(ResolvedType::String)
            }
            "System__cwd" | "System__os" => {
                self.check_arg_count(name, args, 0, span);
                Some(ResolvedType::String)
            }
            // Math Functions
            "Math__random" => {
                self.check_arg_count(name, args, 0, span);
                Some(ResolvedType::Float)
            }
            "Math__pi" => {
                self.check_arg_count(name, args, 0, span);
                Some(ResolvedType::Float)
            }
            "Math__e" => {
                self.check_arg_count(name, args, 0, span);
                Some(ResolvedType::Float)
            }
            // Args Functions
            "Args__count" => {
                self.check_arg_count(name, args, 0, span);
                Some(ResolvedType::Integer)
            }
            "Args__get" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::Integer) {
                        self.error(
                            "Args.get() requires Integer index".to_string(),
                            span.clone(),
                        );
                    } else {
                        self.check_non_negative_integer_const(
                            &args[0].node,
                            args[0].span.clone(),
                            "Args.get() index cannot be negative",
                        );
                    }
                }
                Some(ResolvedType::String)
            }
            // Assertion functions for testing
            "assert" => {
                // assert(condition: Boolean): None
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::Boolean) {
                        self.error(
                            "assert() requires boolean condition".to_string(),
                            span.clone(),
                        );
                    }
                }
                Some(ResolvedType::None)
            }
            "assert_eq" | "assert_ne" => {
                // assert_eq(a: T, b: T): None
                // assert_ne(a: T, b: T): None
                self.check_arg_count(name, args, 2, span.clone());
                if args.len() >= 2 {
                    let t1 = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    let t2 = self.check_builtin_argument_expr(&args[1].node, args[1].span.clone());
                    if self.common_compatible_type(&t1, &t2).is_none() {
                        self.error(
                            format!(
                                "{}() arguments must have compatible types: {} vs {}",
                                name,
                                Self::format_resolved_type_for_diagnostic(&t1),
                                Self::format_resolved_type_for_diagnostic(&t2)
                            ),
                            span,
                        );
                    }
                }
                Some(ResolvedType::None)
            }
            "assert_true" => {
                // assert_true(condition: Boolean): None
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::Boolean) {
                        self.error("assert_true() requires boolean".to_string(), span.clone());
                    }
                }
                Some(ResolvedType::None)
            }
            "assert_false" => {
                // assert_false(condition: Boolean): None
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::Boolean) {
                        self.error("assert_false() requires boolean".to_string(), span.clone());
                    }
                }
                Some(ResolvedType::None)
            }
            "fail" => {
                // fail(message: String): None - unconditionally fails
                if !args.is_empty() {
                    self.check_arg_count(name, args, 1, span.clone());
                    let t = self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Unknown) && !matches!(t, ResolvedType::String) {
                        self.error("fail() requires String message".to_string(), span.clone());
                    }
                }
                Some(ResolvedType::None)
            }
            _ => None,
        }
    }

    pub(super) fn check_method_call(
        &mut self,
        obj_type: &ResolvedType,
        method: &str,
        args: &[Spanned<Expr>],
        type_args: &[Type],
        span: Span,
    ) -> ResolvedType {
        let receiver_type = Self::peel_reference_type(obj_type);
        if !type_args.is_empty() && !matches!(receiver_type, ResolvedType::Class(_)) {
            self.error(
                format!(
                    "Method '{}' on type '{}' does not accept explicit type arguments",
                    method,
                    Self::format_resolved_type_for_diagnostic(obj_type)
                ),
                span.clone(),
            );
        }

        match receiver_type {
            ResolvedType::List(inner) => match method {
                "push" => {
                    self.check_arg_count(method, args, 1, span.clone());
                    if !args.is_empty() {
                        let arg_type = self.check_expr_with_expected_type(
                            &args[0].node,
                            args[0].span.clone(),
                            Some(inner),
                        );
                        if !self.types_compatible(inner, &arg_type) {
                            self.error(
                                format!(
                                    "List.push() type mismatch: expected {}, got {}",
                                    Self::format_resolved_type_for_diagnostic(inner),
                                    Self::format_resolved_type_for_diagnostic(&arg_type)
                                ),
                                args[0].span.clone(),
                            );
                        }
                    }
                    ResolvedType::None
                }
                "get" => {
                    self.check_arg_count(method, args, 1, span.clone());
                    if !args.is_empty() {
                        let idx_type =
                            self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                        if matches!(idx_type, ResolvedType::Unknown) {
                            return (**inner).clone();
                        }
                        if !matches!(idx_type, ResolvedType::Integer) {
                            self.error(
                                format!(
                                    "List.get() index must be Integer, got {}",
                                    Self::format_resolved_type_for_diagnostic(&idx_type)
                                ),
                                args[0].span.clone(),
                            );
                        } else {
                            self.check_non_negative_integer_const(
                                &args[0].node,
                                args[0].span.clone(),
                                "List.get() index cannot be negative",
                            );
                        }
                    }
                    (**inner).clone()
                }
                "set" => {
                    self.check_arg_count(method, args, 2, span.clone());
                    if args.len() >= 2 {
                        let idx_type =
                            self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                        let val_type = self.check_expr_with_expected_type(
                            &args[1].node,
                            args[1].span.clone(),
                            Some(inner),
                        );
                        if !matches!(idx_type, ResolvedType::Unknown)
                            && !matches!(idx_type, ResolvedType::Integer)
                        {
                            self.error(
                                "List.set() index must be Integer".to_string(),
                                args[0].span.clone(),
                            );
                        } else if matches!(idx_type, ResolvedType::Integer) {
                            self.check_non_negative_integer_const(
                                &args[0].node,
                                args[0].span.clone(),
                                "List.set() index cannot be negative",
                            );
                        }
                        if !self.types_compatible(inner, &val_type) {
                            self.error(
                                format!(
                                    "List.set() value type mismatch: expected {}, got {}",
                                    Self::format_resolved_type_for_diagnostic(inner),
                                    Self::format_resolved_type_for_diagnostic(&val_type)
                                ),
                                args[1].span.clone(),
                            );
                        }
                    }
                    ResolvedType::None
                }
                "length" => {
                    self.check_arg_count(method, args, 0, span);
                    ResolvedType::Integer
                }
                "pop" => {
                    self.check_arg_count(method, args, 0, span);
                    (**inner).clone()
                }
                _ => {
                    self.error(format!("Unknown List method: {}", method), span);
                    ResolvedType::Unknown
                }
            },
            ResolvedType::Map(key_type, val_type) => match method {
                "insert" | "set" => {
                    self.check_arg_count(method, args, 2, span.clone());
                    if args.len() >= 2 {
                        let k = self.check_expr_with_expected_type(
                            &args[0].node,
                            args[0].span.clone(),
                            Some(key_type),
                        );
                        let v = self.check_expr_with_expected_type(
                            &args[1].node,
                            args[1].span.clone(),
                            Some(val_type),
                        );
                        if !self.types_compatible(key_type, &k) {
                            self.error("Map key type mismatch".to_string(), args[0].span.clone());
                        }
                        if !self.types_compatible(val_type, &v) {
                            self.error("Map value type mismatch".to_string(), args[1].span.clone());
                        }
                    }
                    ResolvedType::None
                }
                "get" => {
                    self.check_arg_count(method, args, 1, span.clone());
                    if !args.is_empty() {
                        let k = self.check_expr_with_expected_type(
                            &args[0].node,
                            args[0].span.clone(),
                            Some(key_type),
                        );
                        if !self.types_compatible(key_type, &k) {
                            self.error("Map key type mismatch".to_string(), args[0].span.clone());
                        }
                    }
                    (**val_type).clone()
                }
                "contains" => {
                    self.check_arg_count(method, args, 1, span.clone());
                    if !args.is_empty() {
                        let k = self.check_expr_with_expected_type(
                            &args[0].node,
                            args[0].span.clone(),
                            Some(key_type),
                        );
                        if !self.types_compatible(key_type, &k) {
                            self.error("Map key type mismatch".to_string(), args[0].span.clone());
                        }
                    }
                    ResolvedType::Boolean
                }
                "length" => {
                    self.check_arg_count(method, args, 0, span);
                    ResolvedType::Integer
                }
                _ => {
                    self.error(format!("Unknown Map method: {}", method), span);
                    ResolvedType::Unknown
                }
            },
            ResolvedType::Set(inner) => match method {
                "add" | "contains" | "remove" => {
                    self.check_arg_count(method, args, 1, span.clone());
                    if !args.is_empty() {
                        let arg_type = self.check_expr_with_expected_type(
                            &args[0].node,
                            args[0].span.clone(),
                            Some(inner),
                        );
                        if !self.types_compatible(inner, &arg_type) {
                            self.error(
                                format!(
                                    "Set.{}() type mismatch: expected {}, got {}",
                                    method,
                                    Self::format_resolved_type_for_diagnostic(inner),
                                    Self::format_resolved_type_for_diagnostic(&arg_type)
                                ),
                                args[0].span.clone(),
                            );
                        }
                    }
                    ResolvedType::Boolean
                }
                "length" => {
                    self.check_arg_count(method, args, 0, span);
                    ResolvedType::Integer
                }
                _ => {
                    self.error(format!("Unknown Set method: {}", method), span);
                    ResolvedType::Unknown
                }
            },
            ResolvedType::Option(inner) => match method {
                "is_some" | "is_none" => {
                    self.check_arg_count(method, args, 0, span);
                    ResolvedType::Boolean
                }
                "unwrap" => {
                    self.check_arg_count(method, args, 0, span);
                    (**inner).clone()
                }
                _ => {
                    self.error(format!("Unknown Option method: {}", method), span);
                    ResolvedType::Unknown
                }
            },
            ResolvedType::Result(ok, _err) => match method {
                "is_ok" | "is_error" => {
                    self.check_arg_count(method, args, 0, span);
                    ResolvedType::Boolean
                }
                "unwrap" => {
                    self.check_arg_count(method, args, 0, span);
                    (**ok).clone()
                }
                _ => {
                    self.error(format!("Unknown Result method: {}", method), span);
                    ResolvedType::Unknown
                }
            },
            ResolvedType::Class(name) => {
                let (base_name, class_substitutions) = self.instantiated_class_substitutions(name);
                if self.interfaces.contains_key(&base_name) {
                    if let Some(sig) = self.lookup_interface_method(name, method) {
                        if !type_args.is_empty() {
                            self.error(
                                format!(
                                    "Interface method '{}.{}' is not generic",
                                    base_name, method
                                ),
                                span.clone(),
                            );
                        }
                        if args.len() != sig.params.len() {
                            self.error(
                                format!(
                                    "Method '{}' expects {} arguments",
                                    method,
                                    sig.params.len()
                                ),
                                span,
                            );
                        } else {
                            for (arg, (_, param_type)) in args.iter().zip(sig.params.iter()) {
                                let arg_type = self.check_expr_with_expected_type(
                                    &arg.node,
                                    arg.span.clone(),
                                    Some(param_type),
                                );
                                self.enforce_function_argument_effect_contract(
                                    param_type,
                                    &arg.node,
                                    arg.span.clone(),
                                );
                                if !self.types_compatible(param_type, &arg_type) {
                                    self.error(
                                        format!(
                                            "Argument type mismatch: expected {}, got {}",
                                            Self::format_resolved_type_for_diagnostic(param_type),
                                            Self::format_resolved_type_for_diagnostic(&arg_type)
                                        ),
                                        arg.span.clone(),
                                    );
                                }
                            }
                        }
                        sig.return_type.clone()
                    } else {
                        self.error(
                            format!(
                                "Unknown method '{}' on interface '{}'",
                                method,
                                format_diagnostic_class_name(&base_name)
                            ),
                            span,
                        );
                        ResolvedType::Unknown
                    }
                } else if let Some((owner, sig, visibility)) =
                    self.lookup_class_method(&base_name, method)
                {
                    let sig = FuncSig {
                        params: sig
                            .params
                            .iter()
                            .map(|(name, ty)| {
                                (
                                    name.clone(),
                                    Self::substitute_type_vars(ty, &class_substitutions),
                                )
                            })
                            .collect(),
                        return_type: Self::substitute_type_vars(
                            &sig.return_type,
                            &class_substitutions,
                        ),
                        ..sig
                    };
                    self.check_member_visibility(
                        &owner,
                        visibility,
                        "Method",
                        method,
                        span.clone(),
                    );
                    self.enforce_call_effects(&sig, span.clone(), method);
                    let method_name = format!("{}.{}", owner, method);
                    let (inst_params, inst_return_type, valid_explicit_type_args) = self
                        .instantiate_signature_for_call(
                            &method_name,
                            &sig,
                            type_args,
                            span.clone(),
                        );
                    if !valid_explicit_type_args {
                        return ResolvedType::Unknown;
                    }
                    if args.len() != inst_params.len() {
                        self.error(
                            format!(
                                "Method '{}' expects {} arguments",
                                method,
                                inst_params.len()
                            ),
                            span,
                        );
                    } else {
                        for (arg, (_, param_type)) in args.iter().zip(inst_params.iter()) {
                            let arg_type = self.check_expr_with_expected_type(
                                &arg.node,
                                arg.span.clone(),
                                Some(param_type),
                            );
                            self.enforce_function_argument_effect_contract(
                                param_type,
                                &arg.node,
                                arg.span.clone(),
                            );
                            if !self.types_compatible(param_type, &arg_type) {
                                self.error(
                                    format!(
                                        "Argument type mismatch: expected {}, got {}",
                                        Self::format_resolved_type_for_diagnostic(param_type),
                                        Self::format_resolved_type_for_diagnostic(&arg_type)
                                    ),
                                    arg.span.clone(),
                                );
                            }
                        }
                    }
                    inst_return_type
                } else {
                    let diagnostic_class = format_diagnostic_class_name(name);
                    if self.classes.contains_key(&base_name) {
                        self.error(
                            format!(
                                "Unknown method '{}' for class '{}'",
                                method, diagnostic_class
                            ),
                            span,
                        );
                    } else {
                        self.error(format!("Unknown class: {}", diagnostic_class), span);
                    }
                    ResolvedType::Unknown
                }
            }
            ResolvedType::String => match method {
                "length" => {
                    self.check_arg_count(method, args, 0, span);
                    ResolvedType::Integer
                }
                _ => {
                    self.error(format!("Unknown String method: {}", method), span);
                    ResolvedType::Unknown
                }
            },
            ResolvedType::Range(inner) => match method {
                "has_next" => {
                    self.check_arg_count(method, args, 0, span);
                    ResolvedType::Boolean
                }
                "next" => {
                    self.check_arg_count(method, args, 0, span);
                    (**inner).clone()
                }
                _ => {
                    self.error(format!("Unknown Range method: {}", method), span);
                    ResolvedType::Unknown
                }
            },
            ResolvedType::Task(inner) => match method {
                "is_done" => {
                    self.check_arg_count(method, args, 0, span);
                    ResolvedType::Boolean
                }
                "cancel" => {
                    self.check_arg_count(method, args, 0, span);
                    ResolvedType::None
                }
                "await_timeout" => {
                    self.check_arg_count(method, args, 1, span.clone());
                    if let Some(arg) = args.first() {
                        let t = self.check_builtin_argument_expr(&arg.node, arg.span.clone());
                        if !matches!(t, ResolvedType::Unknown)
                            && !matches!(t, ResolvedType::Integer)
                        {
                            self.error(
                                format!(
                                    "Task.await_timeout() expects Integer milliseconds, got {}",
                                    Self::format_resolved_type_for_diagnostic(&t)
                                ),
                                arg.span.clone(),
                            );
                        } else if matches!(
                            Self::eval_numeric_const_expr(&arg.node),
                            Some(NumericConst::Integer(value)) if value < 0
                        ) {
                            self.error(
                                "Task.await_timeout() timeout must be non-negative".to_string(),
                                arg.span.clone(),
                            );
                        }
                    }
                    ResolvedType::Option(Box::new((**inner).clone()))
                }
                _ => {
                    self.error(format!("Unknown Task method: {}", method), span);
                    ResolvedType::Unknown
                }
            },
            ResolvedType::TypeVar(id) => match self.lookup_type_var_bound_method(*id, method) {
                Ok(Some(sig)) => {
                    if !type_args.is_empty() {
                        self.error(
                            format!("Bounded generic method '{}' is not generic", method),
                            span.clone(),
                        );
                    }
                    if args.len() != sig.params.len() {
                        self.error(
                            format!("Method '{}' expects {} arguments", method, sig.params.len()),
                            span,
                        );
                    } else {
                        for (arg, (_, param_type)) in args.iter().zip(sig.params.iter()) {
                            let arg_type = self.check_expr_with_expected_type(
                                &arg.node,
                                arg.span.clone(),
                                Some(param_type),
                            );
                            self.enforce_function_argument_effect_contract(
                                param_type,
                                &arg.node,
                                arg.span.clone(),
                            );
                            if !self.types_compatible(param_type, &arg_type) {
                                self.error(
                                    format!(
                                        "Argument type mismatch: expected {}, got {}",
                                        Self::format_resolved_type_for_diagnostic(param_type),
                                        Self::format_resolved_type_for_diagnostic(&arg_type)
                                    ),
                                    arg.span.clone(),
                                );
                            }
                        }
                    }
                    sig.return_type
                }
                Ok(None) => {
                    if matches!(obj_type, ResolvedType::Unknown) {
                        return ResolvedType::Unknown;
                    }
                    self.error(
                        format!(
                            "Cannot call method on type {}",
                            Self::format_resolved_type_for_diagnostic(obj_type)
                        ),
                        span,
                    );
                    ResolvedType::Unknown
                }
                Err(message) => {
                    self.error(message, span);
                    ResolvedType::Unknown
                }
            },
            ResolvedType::Unknown => ResolvedType::Unknown,
            _ => {
                self.error(
                    format!(
                        "Cannot call method on type {}",
                        Self::format_resolved_type_for_diagnostic(obj_type)
                    ),
                    span,
                );
                ResolvedType::Unknown
            }
        }
    }
}
