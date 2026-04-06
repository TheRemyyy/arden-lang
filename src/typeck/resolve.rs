use super::*;

impl TypeChecker {
    fn current_import_scope_prefixes(&self) -> Vec<Option<&str>> {
        let mut scopes = Vec::new();
        let mut current = self.current_module_prefix.as_deref();
        while let Some(prefix) = current {
            scopes.push(Some(prefix));
            current = prefix.rsplit_once("__").map(|(parent, _)| parent);
        }
        scopes.push(None);
        scopes
    }

    fn lookup_import_alias_path(&self, alias_ident: &str) -> Option<&str> {
        let scoped_paths = self.import_aliases.get(alias_ident)?;
        for scope_prefix in self.current_import_scope_prefixes() {
            if let Some((_, path)) = scoped_paths
                .iter()
                .rev()
                .find(|(scope, _)| scope.as_deref() == scope_prefix)
            {
                return Some(path.as_str());
            }
        }
        None
    }

    fn visible_wildcard_import_paths(&self) -> Vec<&str> {
        let mut paths = Vec::new();
        for scope_prefix in self.current_import_scope_prefixes() {
            for scoped_paths in self.import_aliases.values() {
                for (scope, path) in scoped_paths {
                    if scope.as_deref() == scope_prefix && path.ends_with(".*") {
                        paths.push(path.as_str());
                    }
                }
            }
        }
        paths.sort_unstable();
        paths.dedup();
        paths
    }

    pub(crate) fn resolve_stdlib_alias_call_name(
        &self,
        alias_ident: &str,
        member: &str,
    ) -> Option<String> {
        // Local bindings must shadow import aliases.
        if self.lookup_variable(alias_ident).is_some() {
            return None;
        }
        let namespace_path = self.lookup_import_alias_path(alias_ident)?;
        stdlib_registry().resolve_alias_call(namespace_path, member)
    }

    pub(crate) fn resolve_import_alias_symbol(&self, alias_ident: &str) -> Option<String> {
        // Local bindings must shadow import aliases.
        if self.lookup_variable(alias_ident).is_some() {
            return None;
        }
        let path = self.lookup_import_alias_path(alias_ident)?;
        if let Some(canonical) = crate::ast::builtin_exact_import_alias_canonical(path) {
            return Some(canonical.to_string());
        }
        if path.ends_with(".*") {
            return None;
        }
        let mut parts = path.split('.').collect::<Vec<_>>();
        let symbol = parts.pop()?;
        let namespace = parts.join(".");
        if let Some(canonical) = stdlib_registry().resolve_alias_call(&namespace, symbol) {
            return Some(canonical);
        }
        let full_mangled = path.replace('.', "__");
        if self.functions.contains_key(&full_mangled) {
            return Some(full_mangled);
        }
        if stdlib_registry()
            .get_namespace(symbol)
            .is_some_and(|owner| owner == &namespace)
        {
            return Some(symbol.to_string());
        }
        None
    }

    pub(crate) fn resolve_import_alias_variant(
        &self,
        alias_ident: &str,
    ) -> Option<(String, String)> {
        if self.lookup_variable(alias_ident).is_some() {
            return None;
        }
        let path = self.lookup_import_alias_path(alias_ident)?;
        if path.ends_with(".*") {
            return None;
        }
        let (enum_path, variant_name) = path.rsplit_once('.')?;
        let (namespace, enum_name) = enum_path
            .rsplit_once('.')
            .map_or((String::new(), enum_path.to_string()), |(ns, name)| {
                (ns.to_string(), name.to_string())
            });
        if matches!(enum_name.as_str(), "Option" | "Result") {
            return Some((enum_name, variant_name.to_string()));
        }
        if self.enums.contains_key(&enum_name) {
            return Some((enum_name, variant_name.to_string()));
        }
        let mangled = if namespace.is_empty() {
            enum_name.clone()
        } else {
            format!("{}__{}", namespace.replace('.', "__"), enum_name)
        };
        if self.enums.contains_key(&mangled) {
            return Some((mangled, variant_name.to_string()));
        }
        let suffix = format!("__{}", enum_name);
        let mut matches = self
            .enums
            .keys()
            .filter(|candidate| *candidate == &enum_name || candidate.ends_with(&suffix))
            .cloned()
            .collect::<Vec<_>>();
        matches.sort_unstable();
        matches.dedup();
        (matches.len() == 1).then(|| (matches[0].clone(), variant_name.to_string()))
    }

    pub(crate) fn parse_construct_nominal_type_source(ty: &str) -> Option<(String, Vec<Type>)> {
        match parse_type_source(ty).ok()? {
            Type::Named(name) => Some((name, Vec::new())),
            Type::Generic(name, args) => Some((name, args)),
            _ => None,
        }
    }

    pub(crate) fn resolve_enum_name(&self, name: &str) -> Option<String> {
        if self.enums.contains_key(name) {
            return Some(name.to_string());
        }
        if let Some(leaf) = name.rsplit("__").next() {
            if leaf != name && self.enums.contains_key(leaf) {
                return Some(leaf.to_string());
            }
        }
        let suffix = format!("__{}", name);
        let mut matches = self
            .enums
            .keys()
            .filter(|candidate| *candidate == name || candidate.ends_with(&suffix))
            .cloned()
            .collect::<Vec<_>>();
        matches.sort_unstable();
        matches.dedup();
        (matches.len() == 1).then(|| matches[0].clone())
    }

    pub(crate) fn resolve_import_alias_module_candidate(
        &self,
        alias_ident: &str,
        member_parts: &[String],
    ) -> Option<String> {
        if member_parts.is_empty() || self.lookup_variable(alias_ident).is_some() {
            return None;
        }
        if self.resolve_import_alias_symbol(alias_ident).is_some() {
            return None;
        }
        let path = self.lookup_import_alias_path(alias_ident)?;
        if path.ends_with(".*") {
            return None;
        }
        Some(format!(
            "{}__{}",
            path.replace('.', "__"),
            member_parts.join("__")
        ))
    }

    pub(crate) fn resolve_wildcard_import_module_function_candidate(
        &self,
        module_name: &str,
        member_parts: &[String],
    ) -> Option<String> {
        if member_parts.is_empty() || self.lookup_variable(module_name).is_some() {
            return None;
        }
        let mut matches = self
            .visible_wildcard_import_paths()
            .into_iter()
            .filter_map(|path| path.strip_suffix(".*"))
            .map(|namespace| {
                format!(
                    "{}__{}__{}",
                    namespace.replace('.', "__"),
                    module_name,
                    member_parts.join("__")
                )
            })
            .filter(|candidate| self.functions.contains_key(candidate))
            .collect::<Vec<_>>();
        matches.sort_unstable();
        matches.dedup();
        (matches.len() == 1).then(|| matches[0].clone())
    }

    pub(crate) fn resolve_function_value_name<'a>(&'a self, name: &'a str) -> Option<&'a str> {
        if self.functions.contains_key(name) {
            return Some(name);
        }

        let suffix = if name.contains("__") {
            name.rsplit("__").next().unwrap_or(name)
        } else {
            name
        };

        let mut matches = self
            .functions
            .keys()
            .filter(|candidate| {
                *candidate == suffix || candidate.ends_with(&format!("__{}", suffix))
            })
            .map(|candidate| candidate.as_str())
            .collect::<Vec<_>>();
        matches.sort_unstable();
        matches.dedup();
        if matches.len() == 1 {
            Some(matches[0])
        } else {
            None
        }
    }

    pub(crate) fn module_scoped_type_name(&self, name: &str) -> Option<String> {
        let prefix = self.current_module_prefix.as_deref()?;
        let (base_name, generic_suffix) = name
            .find('<')
            .map(|idx| (&name[..idx], &name[idx..]))
            .unwrap_or((name, ""));
        let candidate = format!("{}__{}", prefix, base_name.replace('.', "__"));
        (self.classes.contains_key(&candidate)
            || self.enums.contains_key(&candidate)
            || self.interfaces.contains_key(&candidate))
        .then_some(format!("{}{}", candidate, generic_suffix))
    }

    pub(crate) fn module_scoped_generic_type(
        &self,
        name: &str,
        args: &[ResolvedType],
    ) -> Option<ResolvedType> {
        let scoped_name = self.module_scoped_type_name(name)?;
        let rendered_args = args
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        Some(ResolvedType::Class(format!(
            "{}<{}>",
            scoped_name, rendered_args
        )))
    }

    pub(crate) fn known_type_exists(&self, name: &str) -> bool {
        self.classes.contains_key(name)
            || self.enums.contains_key(name)
            || self.interfaces.contains_key(name)
    }

    pub(crate) fn resolve_known_type_name(&self, name: &str) -> Option<String> {
        if self.known_type_exists(name) {
            return Some(name.to_string());
        }

        if let Some(module_scoped) = self.module_scoped_type_name(name) {
            let scoped_name = module_scoped
                .split_once('<')
                .map_or(module_scoped.as_str(), |(base, _)| base);
            if self.known_type_exists(scoped_name) {
                return Some(scoped_name.to_string());
            }
        }

        if name.contains('.') {
            let mangled = name.replace('.', "__");
            if self.known_type_exists(&mangled) {
                return Some(mangled);
            }
        }

        let suffix = format!("__{}", name.replace('.', "__"));
        let mut matches = self
            .classes
            .keys()
            .chain(self.enums.keys())
            .chain(self.interfaces.keys())
            .filter(|candidate| *candidate == name || candidate.ends_with(&suffix))
            .cloned()
            .collect::<Vec<_>>();
        matches.sort_unstable();
        matches.dedup();
        (matches.len() == 1).then(|| matches[0].clone())
    }

    pub(crate) fn resolve_nominal_reference_name(&self, name: &str) -> Option<String> {
        if let Ok(Type::Generic(base, args)) = parse_type_source(name) {
            let resolved_base = self.resolve_nominal_reference_name(&base)?;
            let resolved_args = args
                .iter()
                .map(|arg| self.resolve_type(arg).to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Some(format!("{}<{}>", resolved_base, resolved_args));
        }

        if let Some(resolved) = self.resolve_known_type_name(name) {
            return Some(resolved);
        }

        if !name.contains('.') {
            let mut wildcard_matches = self
                .visible_wildcard_import_paths()
                .into_iter()
                .filter_map(|path| path.strip_suffix(".*"))
                .filter_map(|module_path| {
                    self.resolve_known_type_name(&format!("{}.{}", module_path, name))
                })
                .collect::<Vec<_>>();
            wildcard_matches.sort_unstable();
            wildcard_matches.dedup();
            if wildcard_matches.len() == 1 {
                return Some(wildcard_matches[0].clone());
            }
        }

        if let Some(path) = self.lookup_import_alias_path(name) {
            if !path.ends_with(".*") {
                if let Some(resolved) = self.resolve_known_type_name(path) {
                    return Some(resolved);
                }
            }
        }

        let (alias, rest) = name.split_once('.')?;
        let member_parts = rest
            .split('.')
            .map(|part| part.to_string())
            .collect::<Vec<_>>();
        let candidate = self.resolve_import_alias_module_candidate(alias, &member_parts)?;
        self.known_type_exists(&candidate).then_some(candidate)
    }

    pub(crate) fn resolve_user_defined_generic_type(
        &self,
        name: &str,
        args: &[ResolvedType],
    ) -> Option<ResolvedType> {
        let resolved_name = self.resolve_nominal_reference_name(name)?;
        let rendered_args = args
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        Some(ResolvedType::Class(format!(
            "{}<{}>",
            resolved_name, rendered_args
        )))
    }

    pub(crate) fn make_generic_type_bindings(
        &mut self,
        generic_params: &[GenericParam],
    ) -> HashMap<String, ResolvedType> {
        generic_params
            .iter()
            .map(|p| {
                let type_var = self.fresh_type_var();
                if let ResolvedType::TypeVar(id) = type_var {
                    self.type_var_bounds.insert(id, p.bounds.clone());
                }
                (p.name.clone(), type_var)
            })
            .collect()
    }

    pub(crate) fn validate_generic_param_bounds(
        &mut self,
        generic_params: &[GenericParam],
        span: Span,
        owner: &str,
    ) {
        for param in generic_params {
            for bound in &param.bounds {
                let resolved = self
                    .resolve_nominal_reference_name(bound)
                    .unwrap_or_else(|| bound.clone());
                if self.interfaces.contains_key(&resolved) {
                    continue;
                }
                if self.classes.contains_key(&resolved) || self.enums.contains_key(&resolved) {
                    self.error(
                        format!(
                            "{} generic parameter '{}' must use an interface bound, found '{}'",
                            owner, param.name, bound
                        ),
                        span.clone(),
                    );
                } else {
                    self.error(
                        format!(
                            "{} generic parameter '{}' extends unknown interface '{}'",
                            owner, param.name, bound
                        ),
                        span.clone(),
                    );
                }
            }
        }
    }

    pub(crate) fn type_satisfies_interface_bound(
        &self,
        actual: &ResolvedType,
        bound: &str,
    ) -> bool {
        if matches!(actual, ResolvedType::Unknown | ResolvedType::TypeVar(_)) {
            return true;
        }
        let resolved_bound = self
            .resolve_nominal_reference_name(bound)
            .unwrap_or_else(|| bound.to_string());
        let ResolvedType::Class(actual_name) = actual else {
            return false;
        };
        let actual_base = self.class_base_name(actual_name);
        if actual_base == resolved_bound {
            return true;
        }
        self.class_implements_interface(actual_base, &resolved_bound)
            || self.interface_extends(actual_base, &resolved_bound)
    }

    pub(crate) fn type_var_satisfies_bounds(
        &self,
        type_var_id: usize,
        actual: &ResolvedType,
    ) -> bool {
        self.type_var_bounds.get(&type_var_id).is_none_or(|bounds| {
            bounds
                .iter()
                .all(|bound| self.type_satisfies_interface_bound(actual, bound))
        })
    }

    pub(crate) fn validate_class_type_argument_bounds(
        &mut self,
        class_name: &str,
        span: Span,
        context: &str,
    ) {
        let (base_name, substitutions) = self.instantiated_class_substitutions(class_name);
        let Some(type_var_ids) = self
            .classes
            .get(&base_name)
            .map(|class| class.generic_type_vars.clone())
        else {
            return;
        };
        for type_var_id in &type_var_ids {
            let Some(actual) = substitutions.get(type_var_id) else {
                continue;
            };
            if self.type_var_satisfies_bounds(*type_var_id, actual) {
                continue;
            }
            let bounds = self
                .type_var_bounds
                .get(type_var_id)
                .cloned()
                .unwrap_or_default()
                .join(", ");
            self.error(
                format!(
                    "{} type argument {} does not satisfy bound(s) {}",
                    context,
                    Self::format_resolved_type_for_diagnostic(actual),
                    format_diagnostic_class_name(&bounds)
                ),
                span.clone(),
            );
        }
    }

    pub(crate) fn resolve_type_with_bindings(
        &self,
        ty: &Type,
        bindings: &HashMap<String, ResolvedType>,
    ) -> ResolvedType {
        self.resolve_type_with_bindings_and_self_class(ty, bindings, None)
    }

    pub(crate) fn resolve_self_named_generic_class(
        self_class: Option<(&str, &str)>,
        builtin_like_name: &str,
        resolved_args: &[ResolvedType],
    ) -> Option<ResolvedType> {
        let (self_name, self_key) = self_class?;
        if self_name != builtin_like_name {
            return None;
        }
        let rendered_args = resolved_args
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        Some(ResolvedType::Class(format!(
            "{}<{}>",
            self_key, rendered_args
        )))
    }

    pub(crate) fn resolve_type_with_bindings_and_self_class(
        &self,
        ty: &Type,
        bindings: &HashMap<String, ResolvedType>,
        self_class: Option<(&str, &str)>,
    ) -> ResolvedType {
        match ty {
            Type::Integer => ResolvedType::Integer,
            Type::Float => ResolvedType::Float,
            Type::Boolean => ResolvedType::Boolean,
            Type::String => ResolvedType::String,
            Type::Char => ResolvedType::Char,
            Type::None => ResolvedType::None,
            Type::Named(name) => {
                if let Some(bound) = bindings.get(name) {
                    return bound.clone();
                }
                if let Some((self_name, self_key)) = self_class {
                    if name == self_name {
                        return ResolvedType::Class(self_key.to_string());
                    }
                }
                if let Some(resolved_name) = self.resolve_nominal_reference_name(name) {
                    return ResolvedType::Class(resolved_name);
                }
                match name.as_str() {
                    "Range" => ResolvedType::Class("Range".to_string()),
                    _ => ResolvedType::Class(name.clone()),
                }
            }
            Type::Option(inner) => {
                let inner =
                    self.resolve_type_with_bindings_and_self_class(inner, bindings, self_class);
                if let Some(resolved) = Self::resolve_self_named_generic_class(
                    self_class,
                    "Option",
                    std::slice::from_ref(&inner),
                ) {
                    return resolved;
                }
                self.resolve_user_defined_generic_type("Option", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Option", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Option(Box::new(inner)))
            }
            Type::Result(ok, err) => {
                let ok = self.resolve_type_with_bindings_and_self_class(ok, bindings, self_class);
                let err = self.resolve_type_with_bindings_and_self_class(err, bindings, self_class);
                if let Some(resolved) = Self::resolve_self_named_generic_class(
                    self_class,
                    "Result",
                    &[ok.clone(), err.clone()],
                ) {
                    return resolved;
                }
                self.resolve_user_defined_generic_type("Result", &[ok.clone(), err.clone()])
                    .or_else(|| {
                        self.module_scoped_generic_type("Result", &[ok.clone(), err.clone()])
                    })
                    .unwrap_or_else(|| ResolvedType::Result(Box::new(ok), Box::new(err)))
            }
            Type::List(inner) => {
                let inner =
                    self.resolve_type_with_bindings_and_self_class(inner, bindings, self_class);
                if let Some(resolved) = Self::resolve_self_named_generic_class(
                    self_class,
                    "List",
                    std::slice::from_ref(&inner),
                ) {
                    return resolved;
                }
                self.resolve_user_defined_generic_type("List", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("List", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::List(Box::new(inner)))
            }
            Type::Map(k, v) => {
                let key = self.resolve_type_with_bindings_and_self_class(k, bindings, self_class);
                let value = self.resolve_type_with_bindings_and_self_class(v, bindings, self_class);
                if let Some(resolved) = Self::resolve_self_named_generic_class(
                    self_class,
                    "Map",
                    &[key.clone(), value.clone()],
                ) {
                    return resolved;
                }
                self.resolve_user_defined_generic_type("Map", &[key.clone(), value.clone()])
                    .or_else(|| {
                        self.module_scoped_generic_type("Map", &[key.clone(), value.clone()])
                    })
                    .unwrap_or_else(|| ResolvedType::Map(Box::new(key), Box::new(value)))
            }
            Type::Set(inner) => {
                let inner =
                    self.resolve_type_with_bindings_and_self_class(inner, bindings, self_class);
                if let Some(resolved) = Self::resolve_self_named_generic_class(
                    self_class,
                    "Set",
                    std::slice::from_ref(&inner),
                ) {
                    return resolved;
                }
                self.resolve_user_defined_generic_type("Set", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Set", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Set(Box::new(inner)))
            }
            Type::Ref(inner) => ResolvedType::Ref(Box::new(
                self.resolve_type_with_bindings_and_self_class(inner, bindings, self_class),
            )),
            Type::MutRef(inner) => ResolvedType::MutRef(Box::new(
                self.resolve_type_with_bindings_and_self_class(inner, bindings, self_class),
            )),
            Type::Box(inner) => {
                let inner =
                    self.resolve_type_with_bindings_and_self_class(inner, bindings, self_class);
                if let Some(resolved) = Self::resolve_self_named_generic_class(
                    self_class,
                    "Box",
                    std::slice::from_ref(&inner),
                ) {
                    return resolved;
                }
                self.resolve_user_defined_generic_type("Box", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Box", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Box(Box::new(inner)))
            }
            Type::Rc(inner) => {
                let inner =
                    self.resolve_type_with_bindings_and_self_class(inner, bindings, self_class);
                if let Some(resolved) = Self::resolve_self_named_generic_class(
                    self_class,
                    "Rc",
                    std::slice::from_ref(&inner),
                ) {
                    return resolved;
                }
                self.resolve_user_defined_generic_type("Rc", std::slice::from_ref(&inner))
                    .or_else(|| self.module_scoped_generic_type("Rc", std::slice::from_ref(&inner)))
                    .unwrap_or_else(|| ResolvedType::Rc(Box::new(inner)))
            }
            Type::Arc(inner) => {
                let inner =
                    self.resolve_type_with_bindings_and_self_class(inner, bindings, self_class);
                if let Some(resolved) = Self::resolve_self_named_generic_class(
                    self_class,
                    "Arc",
                    std::slice::from_ref(&inner),
                ) {
                    return resolved;
                }
                self.resolve_user_defined_generic_type("Arc", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Arc", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Arc(Box::new(inner)))
            }
            Type::Ptr(inner) => {
                let inner =
                    self.resolve_type_with_bindings_and_self_class(inner, bindings, self_class);
                if let Some(resolved) = Self::resolve_self_named_generic_class(
                    self_class,
                    "Ptr",
                    std::slice::from_ref(&inner),
                ) {
                    return resolved;
                }
                self.resolve_user_defined_generic_type("Ptr", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Ptr", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Ptr(Box::new(inner)))
            }
            Type::Task(inner) => {
                let inner =
                    self.resolve_type_with_bindings_and_self_class(inner, bindings, self_class);
                if let Some(resolved) = Self::resolve_self_named_generic_class(
                    self_class,
                    "Task",
                    std::slice::from_ref(&inner),
                ) {
                    return resolved;
                }
                self.resolve_user_defined_generic_type("Task", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Task", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Task(Box::new(inner)))
            }
            Type::Range(inner) => {
                let inner =
                    self.resolve_type_with_bindings_and_self_class(inner, bindings, self_class);
                if let Some(resolved) = Self::resolve_self_named_generic_class(
                    self_class,
                    "Range",
                    std::slice::from_ref(&inner),
                ) {
                    return resolved;
                }
                self.resolve_user_defined_generic_type("Range", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Range", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Range(Box::new(inner)))
            }
            Type::Function(params, ret) => ResolvedType::Function(
                params
                    .iter()
                    .map(|p| {
                        self.resolve_type_with_bindings_and_self_class(p, bindings, self_class)
                    })
                    .collect(),
                Box::new(self.resolve_type_with_bindings_and_self_class(ret, bindings, self_class)),
            ),
            Type::Generic(name, args) => {
                let resolved_args = args
                    .iter()
                    .map(|arg| {
                        self.resolve_type_with_bindings_and_self_class(arg, bindings, self_class)
                    })
                    .collect::<Vec<_>>();
                if let Some((self_name, self_key)) = self_class {
                    if name == self_name {
                        let args = resolved_args
                            .iter()
                            .map(std::string::ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ");
                        return ResolvedType::Class(format!("{}<{}>", self_key, args));
                    }
                }
                if let Some(resolved) = self.resolve_user_defined_generic_type(name, &resolved_args)
                {
                    return resolved;
                }
                match name.as_str() {
                    "Option" if resolved_args.len() == 1 => {
                        ResolvedType::Option(Box::new(resolved_args[0].clone()))
                    }
                    "Result" if resolved_args.len() == 2 => ResolvedType::Result(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    ),
                    "List" if resolved_args.len() == 1 => {
                        ResolvedType::List(Box::new(resolved_args[0].clone()))
                    }
                    "Map" if resolved_args.len() == 2 => ResolvedType::Map(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    ),
                    "Set" if resolved_args.len() == 1 => {
                        ResolvedType::Set(Box::new(resolved_args[0].clone()))
                    }
                    "Box" if resolved_args.len() == 1 => {
                        ResolvedType::Box(Box::new(resolved_args[0].clone()))
                    }
                    "Rc" if resolved_args.len() == 1 => {
                        ResolvedType::Rc(Box::new(resolved_args[0].clone()))
                    }
                    "Arc" if resolved_args.len() == 1 => {
                        ResolvedType::Arc(Box::new(resolved_args[0].clone()))
                    }
                    "Ptr" if resolved_args.len() == 1 => {
                        ResolvedType::Ptr(Box::new(resolved_args[0].clone()))
                    }
                    "Task" if resolved_args.len() == 1 => {
                        ResolvedType::Task(Box::new(resolved_args[0].clone()))
                    }
                    "Range" if resolved_args.len() == 1 => {
                        ResolvedType::Range(Box::new(resolved_args[0].clone()))
                    }
                    _ => {
                        let args = resolved_args
                            .iter()
                            .map(std::string::ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ");
                        ResolvedType::Class(format!("{}<{}>", name, args))
                    }
                }
            }
        }
    }

    pub(crate) fn resolve_builtin_module_alias(&self, name: &str) -> String {
        let Some(path) = self.lookup_import_alias_path(name) else {
            return name.to_string();
        };
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
        owner.unwrap_or_else(|| name.to_string())
    }

    pub(crate) fn resolve_contextual_function_value_name(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Ident(name) => self
                .resolve_import_alias_symbol(name)
                .or_else(|| {
                    self.resolve_function_value_name(name)
                        .map(|resolved| resolved.to_string())
                })
                .or_else(|| Self::builtin_function_value_type(name).map(|_| name.clone())),
            Expr::Field { object, field } => {
                if let Some(path_parts) = flatten_field_chain(expr) {
                    if path_parts.len() >= 2 {
                        if let Some(path) = self.lookup_import_alias_path(&path_parts[0]) {
                            let namespace_path = if path_parts.len() == 2 {
                                path.to_string()
                            } else {
                                format!(
                                    "{}.{}",
                                    path,
                                    path_parts[1..path_parts.len() - 1].join(".")
                                )
                            };
                            if let Some(canonical) = stdlib_registry()
                                .resolve_alias_call(&namespace_path, path_parts.last()?)
                            {
                                return Some(canonical);
                            }
                            let candidate = format!(
                                "{}__{}",
                                path.replace('.', "__"),
                                path_parts[1..].join("__")
                            );
                            let resolved = self
                                .resolve_function_value_name(&candidate)
                                .unwrap_or(&candidate);
                            if self.functions.contains_key(resolved) {
                                return Some(resolved.to_string());
                            }
                        }

                        if path_parts.len() == 2 {
                            let builtin_owner = self.resolve_builtin_module_alias(&path_parts[0]);
                            if matches!(builtin_owner.as_str(), "Option" | "Result") {
                                let static_container_name = format!("{}__{}", builtin_owner, field);
                                if Self::is_contextual_static_container_function_value(
                                    &static_container_name,
                                ) {
                                    return Some(static_container_name);
                                }
                            }
                            let builtin_name = format!("{}__{}", builtin_owner, field);
                            if Self::builtin_matches_expected_function_type(
                                &builtin_name,
                                &ResolvedType::Unknown,
                            ) {
                                return Some(builtin_name);
                            }
                        }

                        let mangled = path_parts.join("__");
                        let resolved = self
                            .resolve_function_value_name(&mangled)
                            .unwrap_or(&mangled);
                        if self.functions.contains_key(resolved) {
                            return Some(resolved.to_string());
                        }
                    }
                }
                if let Expr::Ident(owner_name) = &object.node {
                    let resolved_owner = self.resolve_builtin_module_alias(owner_name);
                    if matches!(resolved_owner.as_str(), "Option" | "Result") {
                        let static_container_name = format!("{}__{}", resolved_owner, field);
                        if Self::is_contextual_static_container_function_value(
                            &static_container_name,
                        ) {
                            return Some(static_container_name);
                        }
                    }
                    let builtin_name = format!("{}__{}", resolved_owner, field);
                    if Self::builtin_matches_expected_function_type(
                        &builtin_name,
                        &ResolvedType::Unknown,
                    ) {
                        return Some(builtin_name);
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub(crate) fn resolve_class_constructor_function_value_type(
        &mut self,
        expr: &Expr,
        explicit_type_args: Option<&[Type]>,
        expected: Option<&ResolvedType>,
        span: Span,
    ) -> Option<ResolvedType> {
        let mut type_source = Self::nominal_function_value_type_source(expr)?;
        if let Some(type_args) = explicit_type_args {
            type_source = format!(
                "{}<{}>",
                type_source,
                type_args
                    .iter()
                    .map(Self::format_ast_type_source)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        } else if let Some(ResolvedType::Function(_, ret)) = expected {
            if let ResolvedType::Class(expected_class_name) = ret.as_ref() {
                let resolved_base = self.resolve_nominal_reference_name(&type_source)?;
                if self.class_base_name(expected_class_name) == self.class_base_name(&resolved_base)
                {
                    type_source = expected_class_name.clone();
                }
            }
        }

        let resolved_ctor_type = self.resolve_type_source(&type_source);
        let scoped_ty = resolved_ctor_type
            .clone()
            .map(|resolved| resolved.to_string())
            .unwrap_or_else(|| self.resolve_type_source_string(&type_source));
        let (class_name, class_substitutions) = self.instantiated_class_substitutions(&scoped_ty);
        let class = self.classes.get(&class_name).cloned()?;

        self.validate_class_type_argument_bounds(&scoped_ty, span.clone(), "Constructor");
        self.check_class_visibility(&class_name, span.clone());

        let ctor_params = class
            .constructor
            .as_ref()
            .map(|params| {
                params
                    .iter()
                    .map(|(_, ty)| Self::substitute_type_vars(ty, &class_substitutions))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let actual_ty = ResolvedType::Function(
            ctor_params,
            Box::new(resolved_ctor_type.unwrap_or_else(|| self.parse_type_string(&scoped_ty))),
        );

        if let Some(expected_ty) = expected {
            if self.types_compatible(expected_ty, &actual_ty) {
                return Some(actual_ty);
            }
            self.error(
                format!(
                    "Type mismatch: expected {}, got {}",
                    Self::format_resolved_type_for_diagnostic(expected_ty),
                    Self::format_resolved_type_for_diagnostic(&actual_ty)
                ),
                span,
            );
            return Some(ResolvedType::Unknown);
        }

        Some(actual_ty)
    }

    pub(crate) fn resolve_enum_variant_function_value(
        &self,
        expr: &Expr,
    ) -> Option<(String, Vec<ResolvedType>)> {
        if let Expr::Ident(name) = expr {
            if let Some((enum_name, variant_name)) = name.rsplit_once("__") {
                if let Some(enum_info) = self.enums.get(enum_name) {
                    if let Some(fields) = enum_info.variants.get(variant_name) {
                        return Some((enum_name.to_string(), fields.clone()));
                    }
                }
            }
            let (enum_name, variant_name) = self.resolve_import_alias_variant(name)?;
            let enum_info = self.enums.get(&enum_name)?;
            let fields = enum_info.variants.get(&variant_name)?.clone();
            return Some((enum_name, fields));
        }

        let Expr::Field { object, field } = expr else {
            return None;
        };

        if let Some(path_parts) = flatten_field_chain(expr) {
            if path_parts.len() >= 2 {
                let owner_source = path_parts[..path_parts.len() - 1].join(".");
                if let Some(resolved_owner) = self.resolve_nominal_reference_name(&owner_source) {
                    if let Some(enum_info) = self.enums.get(&resolved_owner) {
                        let variant_name = path_parts.last()?;
                        if let Some(fields) = enum_info.variants.get(variant_name) {
                            return Some((resolved_owner, fields.clone()));
                        }
                    }
                }
            }
        }

        let Expr::Ident(owner_name) = &object.node else {
            return None;
        };
        let resolved_owner = self
            .resolve_import_alias_symbol(owner_name)
            .or_else(|| self.resolve_nominal_reference_name(owner_name))
            .or_else(|| self.resolve_enum_name(owner_name))?;
        let enum_info = self.enums.get(&resolved_owner)?;
        let fields = enum_info.variants.get(field)?.clone();
        Some((resolved_owner, fields))
    }

    pub(crate) fn resolve_type(&self, ty: &Type) -> ResolvedType {
        match ty {
            Type::Integer => ResolvedType::Integer,
            Type::Float => ResolvedType::Float,
            Type::Boolean => ResolvedType::Boolean,
            Type::String => ResolvedType::String,
            Type::Char => ResolvedType::Char,
            Type::None => ResolvedType::None,
            Type::Named(name) => {
                if let Some(bound) = self.current_generic_type_bindings.get(name) {
                    return bound.clone();
                }
                if let Some(resolved_name) = self.resolve_nominal_reference_name(name) {
                    return ResolvedType::Class(resolved_name);
                }
                // Check for built-in types that might be parsed as Named
                match name.as_str() {
                    "Range" => ResolvedType::Class("Range".to_string()),
                    _ => ResolvedType::Class(name.clone()),
                }
            }
            Type::Option(inner) => {
                let inner = self.resolve_type(inner);
                self.resolve_user_defined_generic_type("Option", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Option", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Option(Box::new(inner)))
            }
            Type::Result(ok, err) => {
                let ok = self.resolve_type(ok);
                let err = self.resolve_type(err);
                self.resolve_user_defined_generic_type("Result", &[ok.clone(), err.clone()])
                    .or_else(|| {
                        self.module_scoped_generic_type("Result", &[ok.clone(), err.clone()])
                    })
                    .unwrap_or_else(|| ResolvedType::Result(Box::new(ok), Box::new(err)))
            }
            Type::List(inner) => {
                let inner = self.resolve_type(inner);
                self.resolve_user_defined_generic_type("List", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("List", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::List(Box::new(inner)))
            }
            Type::Map(k, v) => {
                let key = self.resolve_type(k);
                let value = self.resolve_type(v);
                self.resolve_user_defined_generic_type("Map", &[key.clone(), value.clone()])
                    .or_else(|| {
                        self.module_scoped_generic_type("Map", &[key.clone(), value.clone()])
                    })
                    .unwrap_or_else(|| ResolvedType::Map(Box::new(key), Box::new(value)))
            }
            Type::Set(inner) => {
                let inner = self.resolve_type(inner);
                self.resolve_user_defined_generic_type("Set", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Set", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Set(Box::new(inner)))
            }
            Type::Ref(inner) => ResolvedType::Ref(Box::new(self.resolve_type(inner))),
            Type::MutRef(inner) => ResolvedType::MutRef(Box::new(self.resolve_type(inner))),
            Type::Box(inner) => {
                let inner = self.resolve_type(inner);
                self.resolve_user_defined_generic_type("Box", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Box", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Box(Box::new(inner)))
            }
            Type::Rc(inner) => {
                let inner = self.resolve_type(inner);
                self.resolve_user_defined_generic_type("Rc", std::slice::from_ref(&inner))
                    .or_else(|| self.module_scoped_generic_type("Rc", std::slice::from_ref(&inner)))
                    .unwrap_or_else(|| ResolvedType::Rc(Box::new(inner)))
            }
            Type::Arc(inner) => {
                let inner = self.resolve_type(inner);
                self.resolve_user_defined_generic_type("Arc", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Arc", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Arc(Box::new(inner)))
            }
            Type::Ptr(inner) => {
                let inner = self.resolve_type(inner);
                self.resolve_user_defined_generic_type("Ptr", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Ptr", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Ptr(Box::new(inner)))
            }
            Type::Task(inner) => {
                let inner = self.resolve_type(inner);
                self.resolve_user_defined_generic_type("Task", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Task", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Task(Box::new(inner)))
            }
            Type::Range(inner) => {
                let inner = self.resolve_type(inner);
                self.resolve_user_defined_generic_type("Range", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Range", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Range(Box::new(inner)))
            }
            Type::Function(params, ret) => ResolvedType::Function(
                params.iter().map(|p| self.resolve_type(p)).collect(),
                Box::new(self.resolve_type(ret)),
            ),
            Type::Generic(name, args) => {
                if let Some(bound) = self.current_generic_type_bindings.get(name) {
                    return bound.clone();
                }
                let resolved_args = args
                    .iter()
                    .map(|arg| self.resolve_type(arg))
                    .collect::<Vec<_>>();
                if let Some(resolved) = self.resolve_user_defined_generic_type(name, &resolved_args)
                {
                    return resolved;
                }
                // Handle generic types
                match name.as_str() {
                    "Option" if resolved_args.len() == 1 => {
                        ResolvedType::Option(Box::new(resolved_args[0].clone()))
                    }
                    "Result" if resolved_args.len() == 2 => ResolvedType::Result(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    ),
                    "List" if resolved_args.len() == 1 => {
                        ResolvedType::List(Box::new(resolved_args[0].clone()))
                    }
                    "Map" if resolved_args.len() == 2 => ResolvedType::Map(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    ),
                    "Set" if resolved_args.len() == 1 => {
                        ResolvedType::Set(Box::new(resolved_args[0].clone()))
                    }
                    "Box" if resolved_args.len() == 1 => {
                        ResolvedType::Box(Box::new(resolved_args[0].clone()))
                    }
                    "Rc" if resolved_args.len() == 1 => {
                        ResolvedType::Rc(Box::new(resolved_args[0].clone()))
                    }
                    "Arc" if resolved_args.len() == 1 => {
                        ResolvedType::Arc(Box::new(resolved_args[0].clone()))
                    }
                    "Ptr" if resolved_args.len() == 1 => {
                        ResolvedType::Ptr(Box::new(resolved_args[0].clone()))
                    }
                    "Task" if resolved_args.len() == 1 => {
                        ResolvedType::Task(Box::new(resolved_args[0].clone()))
                    }
                    "Range" if resolved_args.len() == 1 => {
                        ResolvedType::Range(Box::new(resolved_args[0].clone()))
                    }
                    _ => {
                        let args = resolved_args
                            .iter()
                            .map(std::string::ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ");
                        ResolvedType::Class(format!("{}<{}>", name, args))
                    }
                }
            }
        }
    }

    pub(crate) fn parse_type_string(&self, s: &str) -> ResolvedType {
        let s = s.trim();
        match s {
            "Integer" => ResolvedType::Integer,
            "Float" => ResolvedType::Float,
            "Boolean" => ResolvedType::Boolean,
            "String" => ResolvedType::String,
            "Char" => ResolvedType::Char,
            "None" => ResolvedType::None,
            _ => {
                if let Some((params, ret)) = self.parse_function_type_string(s) {
                    return ResolvedType::Function(
                        params.iter().map(|p| self.parse_type_string(p)).collect(),
                        Box::new(self.parse_type_string(&ret)),
                    );
                }
                if let Some(open_bracket) = s.find('<') {
                    if s.ends_with('>') {
                        let base = &s[..open_bracket];
                        let inner_str = &s[open_bracket + 1..s.len() - 1];
                        let generic_args = self
                            .split_generic_args(inner_str)
                            .into_iter()
                            .map(|part| self.parse_type_string(&part))
                            .collect::<Vec<_>>();

                        if let Some(resolved) =
                            self.resolve_user_defined_generic_type(base, &generic_args)
                        {
                            return resolved;
                        }

                        match base {
                            "List" if generic_args.len() == 1 => {
                                ResolvedType::List(Box::new(generic_args[0].clone()))
                            }
                            "Set" if generic_args.len() == 1 => {
                                ResolvedType::Set(Box::new(generic_args[0].clone()))
                            }
                            "Option" if generic_args.len() == 1 => {
                                ResolvedType::Option(Box::new(generic_args[0].clone()))
                            }
                            "Task" if generic_args.len() == 1 => {
                                ResolvedType::Task(Box::new(generic_args[0].clone()))
                            }
                            "Box" if generic_args.len() == 1 => {
                                ResolvedType::Box(Box::new(generic_args[0].clone()))
                            }
                            "Rc" if generic_args.len() == 1 => {
                                ResolvedType::Rc(Box::new(generic_args[0].clone()))
                            }
                            "Arc" if generic_args.len() == 1 => {
                                ResolvedType::Arc(Box::new(generic_args[0].clone()))
                            }
                            "Ptr" if generic_args.len() == 1 => {
                                ResolvedType::Ptr(Box::new(generic_args[0].clone()))
                            }
                            "Map" => {
                                if generic_args.len() == 2 {
                                    ResolvedType::Map(
                                        Box::new(generic_args[0].clone()),
                                        Box::new(generic_args[1].clone()),
                                    )
                                } else {
                                    ResolvedType::Unknown
                                }
                            }
                            "Result" => {
                                if generic_args.len() == 2 {
                                    ResolvedType::Result(
                                        Box::new(generic_args[0].clone()),
                                        Box::new(generic_args[1].clone()),
                                    )
                                } else {
                                    ResolvedType::Unknown
                                }
                            }
                            _ => self
                                .resolve_nominal_reference_name(s)
                                .map(ResolvedType::Class)
                                .unwrap_or_else(|| ResolvedType::Class(s.to_string())),
                        }
                    } else {
                        self.resolve_nominal_reference_name(s)
                            .map(ResolvedType::Class)
                            .unwrap_or_else(|| ResolvedType::Class(s.to_string()))
                    }
                } else {
                    self.resolve_nominal_reference_name(s)
                        .map(ResolvedType::Class)
                        .unwrap_or_else(|| ResolvedType::Class(s.to_string()))
                }
            }
        }
    }

    pub(crate) fn resolve_type_source_string(&self, s: &str) -> String {
        self.resolve_type_source(s)
            .map(|resolved| resolved.to_string())
            .unwrap_or_else(|| {
                self.resolve_nominal_reference_name(s).unwrap_or_else(|| {
                    self.module_scoped_type_name(s)
                        .unwrap_or_else(|| s.to_string())
                })
            })
    }

    pub(crate) fn resolve_type_source(&self, s: &str) -> Option<ResolvedType> {
        parse_type_source(s)
            .ok()
            .map(|parsed| self.resolve_type(&parsed))
    }

    pub(crate) fn parse_function_type_string(&self, s: &str) -> Option<(Vec<String>, String)> {
        if !s.starts_with('(') {
            return None;
        }

        let mut paren_depth = 0usize;
        let mut angle_depth = 0usize;
        let mut close_idx = None;
        for (idx, ch) in s.char_indices() {
            match ch {
                '(' => paren_depth += 1,
                ')' => {
                    paren_depth = paren_depth.saturating_sub(1);
                    if paren_depth == 0 && angle_depth == 0 {
                        close_idx = Some(idx);
                        break;
                    }
                }
                '<' => angle_depth += 1,
                '>' => angle_depth = angle_depth.saturating_sub(1),
                _ => {}
            }
        }

        let close_idx = close_idx?;
        let rest = s[close_idx + 1..].trim();
        let rest = rest.strip_prefix("->")?.trim();
        let params_str = &s[1..close_idx];
        let params = if params_str.trim().is_empty() {
            Vec::new()
        } else {
            self.split_type_list(params_str)
        };
        Some((params, rest.to_string()))
    }

    pub(crate) fn split_type_list(&self, s: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut angle_depth = 0usize;
        let mut paren_depth = 0usize;

        for ch in s.chars() {
            match ch {
                ',' if angle_depth == 0 && paren_depth == 0 => {
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        parts.push(trimmed.to_string());
                    }
                    current.clear();
                }
                '<' => {
                    angle_depth += 1;
                    current.push(ch);
                }
                '>' => {
                    angle_depth = angle_depth.saturating_sub(1);
                    current.push(ch);
                }
                '(' => {
                    paren_depth += 1;
                    current.push(ch);
                }
                ')' => {
                    paren_depth = paren_depth.saturating_sub(1);
                    current.push(ch);
                }
                _ => current.push(ch),
            }
        }

        let trimmed = current.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed.to_string());
        }
        parts
    }

    pub(crate) fn split_generic_args(&self, s: &str) -> Vec<String> {
        split_generic_args_static(s)
    }
}
