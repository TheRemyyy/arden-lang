use super::*;

impl TypeChecker {
    fn register_function_leaf_name(&mut self, key: &str) {
        let leaf_name = key.rsplit("__").next().unwrap_or(key);
        match self.function_leaf_names.get_mut(leaf_name) {
            Some(existing) => {
                if existing.as_deref() != Some(key) {
                    *existing = None;
                }
            }
            None => {
                self.function_leaf_names
                    .insert(leaf_name.to_string(), Some(key.to_string()));
            }
        }
    }

    pub(crate) fn collect_declarations(&mut self, program: &Program) {
        self.predeclare_nominal_types(program);
        for decl in &program.declarations {
            match &decl.node {
                Decl::Import(_) => {}
                Decl::Function(func) => {
                    self.insert_function_signature(func, &func.name, decl.span.clone(), None);
                }
                Decl::Class(class) => {
                    self.insert_class_info(class, &class.name, decl.span.clone());
                }
                Decl::Interface(interface) => {
                    self.insert_interface_info(interface, &interface.name, decl.span.clone());
                }
                Decl::Enum(en) => {
                    self.insert_enum_info(en, &en.name, decl.span.clone());
                }
                Decl::Module(module) => {
                    self.collect_module_declarations(module, &module.name, decl.span.clone());
                }
            }
        }
    }

    pub(crate) fn predeclare_nominal_types(&mut self, program: &Program) {
        for decl in &program.declarations {
            self.predeclare_decl_nominal_types(&decl.node, None, decl.span.clone());
        }
    }

    pub(crate) fn predeclare_decl_nominal_types(
        &mut self,
        decl: &Decl,
        module_prefix: Option<&str>,
        span: Span,
    ) {
        match decl {
            Decl::Class(class) => {
                let key = module_prefix
                    .map(|prefix| format!("{}__{}", prefix, class.name))
                    .unwrap_or_else(|| class.name.clone());
                self.classes.entry(key).or_insert_with(|| ClassInfo {
                    fields: HashMap::new(),
                    methods: HashMap::new(),
                    method_visibilities: HashMap::new(),
                    constructor: None,
                    generic_type_vars: Vec::new(),
                    visibility: class.visibility,
                    extends: class.extends.clone(),
                    implements: class.implements.clone(),
                    span: span.clone(),
                });
            }
            Decl::Enum(en) => {
                let key = module_prefix
                    .map(|prefix| format!("{}__{}", prefix, en.name))
                    .unwrap_or_else(|| en.name.clone());
                self.enums.entry(key).or_insert_with(|| EnumInfo {
                    variants: HashMap::new(),
                    span: span.clone(),
                });
            }
            Decl::Interface(interface) => {
                let key = module_prefix
                    .map(|prefix| format!("{}__{}", prefix, interface.name))
                    .unwrap_or_else(|| interface.name.clone());
                self.interfaces.entry(key).or_insert_with(|| InterfaceInfo {
                    methods: HashMap::new(),
                    generic_param_names: Vec::new(),
                    generic_type_vars: Vec::new(),
                    extends: interface.extends.clone(),
                    span: span.clone(),
                });
            }
            Decl::Module(module) => {
                let nested_prefix = module_prefix
                    .map(|prefix| format!("{}__{}", prefix, module.name))
                    .unwrap_or_else(|| module.name.clone());
                for inner_decl in &module.declarations {
                    self.predeclare_decl_nominal_types(
                        &inner_decl.node,
                        Some(&nested_prefix),
                        inner_decl.span.clone(),
                    );
                }
            }
            Decl::Function(_) | Decl::Import(_) => {}
        }
    }

    pub(crate) fn normalize_inheritance_references(&mut self) {
        let class_inputs = self
            .classes
            .iter()
            .map(|(name, info)| (name.clone(), info.extends.clone(), info.implements.clone()))
            .collect::<Vec<_>>();
        let mut class_updates = Vec::new();
        for (name, extends, implements) in class_inputs {
            self.current_module_prefix =
                name.rsplit_once("__").map(|(prefix, _)| prefix.to_string());
            class_updates.push((
                name,
                extends.as_ref().map(|parent| {
                    self.resolve_nominal_reference_name(parent)
                        .unwrap_or_else(|| parent.clone())
                }),
                implements
                    .iter()
                    .map(|interface_name| {
                        self.resolve_nominal_reference_name(interface_name)
                            .unwrap_or_else(|| interface_name.clone())
                    })
                    .collect::<Vec<_>>(),
            ));
        }
        self.current_module_prefix = None;
        for (name, extends, implements) in class_updates {
            if let Some(class) = self.classes.get_mut(&name) {
                class.extends = extends;
                class.implements = implements;
            }
        }

        let interface_inputs = self
            .interfaces
            .iter()
            .map(|(name, info)| (name.clone(), info.extends.clone()))
            .collect::<Vec<_>>();
        let mut interface_updates = Vec::new();
        for (name, extends) in interface_inputs {
            self.current_module_prefix =
                name.rsplit_once("__").map(|(prefix, _)| prefix.to_string());
            interface_updates.push((
                name,
                extends
                    .iter()
                    .map(|parent| {
                        self.resolve_nominal_reference_name(parent)
                            .unwrap_or_else(|| parent.clone())
                    })
                    .collect::<Vec<_>>(),
            ));
        }
        self.current_module_prefix = None;
        for (name, extends) in interface_updates {
            if let Some(interface) = self.interfaces.get_mut(&name) {
                interface.extends = extends;
            }
        }
    }

    pub(crate) fn insert_class_info(&mut self, class: &ClassDecl, key: &str, span: Span) {
        let class_generic_bindings = self.make_generic_type_bindings(&class.generic_params);
        let class_generic_type_vars: Vec<usize> = class
            .generic_params
            .iter()
            .filter_map(|p| match class_generic_bindings.get(&p.name) {
                Some(ResolvedType::TypeVar(id)) => Some(*id),
                _ => None,
            })
            .collect();
        let mut fields = HashMap::new();
        for field in &class.fields {
            fields.insert(
                field.name.clone(),
                (
                    self.resolve_type_with_bindings_and_self_class(
                        &field.ty,
                        &class_generic_bindings,
                        Some((&class.name, key)),
                    ),
                    field.mutable,
                    field.visibility,
                ),
            );
        }

        let mut methods = HashMap::new();
        let mut method_visibilities = HashMap::new();
        for method in &class.methods {
            self.validate_effect_attributes(
                &method.attributes,
                span.clone(),
                &format!("Method '{}.{}'", key, method.name),
            );
            let mut generic_bindings = class_generic_bindings.clone();
            let method_generic_bindings = self.make_generic_type_bindings(&method.generic_params);
            let generic_type_vars: Vec<usize> = method
                .generic_params
                .iter()
                .filter_map(|p| match method_generic_bindings.get(&p.name) {
                    Some(ResolvedType::TypeVar(id)) => Some(*id),
                    _ => None,
                })
                .collect();
            generic_bindings.extend(method_generic_bindings);
            let params: Vec<(String, ResolvedType)> = method
                .params
                .iter()
                .map(|p| {
                    (
                        p.name.clone(),
                        self.resolve_type_with_bindings_and_self_class(
                            &p.ty,
                            &generic_bindings,
                            Some((&class.name, key)),
                        ),
                    )
                })
                .collect();

            let mut return_type = self.resolve_type_with_bindings_and_self_class(
                &method.return_type,
                &generic_bindings,
                Some((&class.name, key)),
            );
            if method.is_async && !matches!(return_type, ResolvedType::Task(_)) {
                return_type = ResolvedType::Task(Box::new(return_type));
            }
            let (effects, is_pure, allow_any, has_explicit_effects) =
                self.parse_effects_from_attributes(&method.attributes);

            methods.insert(
                method.name.clone(),
                FuncSig {
                    params,
                    return_type,
                    generic_type_vars,
                    is_variadic: method.is_variadic,
                    is_extern: method.is_extern,
                    effects,
                    is_pure,
                    allow_any,
                    has_explicit_effects,
                    span: span.clone(),
                },
            );
            method_visibilities.insert(method.name.clone(), method.visibility);
        }

        let constructor = class.constructor.as_ref().map(|c| {
            c.params
                .iter()
                .map(|p| {
                    (
                        p.name.clone(),
                        self.resolve_type_with_bindings_and_self_class(
                            &p.ty,
                            &class_generic_bindings,
                            Some((&class.name, key)),
                        ),
                    )
                })
                .collect()
        });

        self.classes.insert(
            key.to_string(),
            ClassInfo {
                fields,
                methods,
                method_visibilities,
                constructor,
                generic_type_vars: class_generic_type_vars,
                visibility: class.visibility,
                extends: class.extends.clone(),
                implements: class.implements.clone(),
                span,
            },
        );
    }

    pub(crate) fn insert_interface_info(
        &mut self,
        interface: &InterfaceDecl,
        key: &str,
        span: Span,
    ) {
        let interface_generic_bindings = self.make_generic_type_bindings(&interface.generic_params);
        let interface_generic_type_vars: Vec<usize> = interface
            .generic_params
            .iter()
            .filter_map(|p| match interface_generic_bindings.get(&p.name) {
                Some(ResolvedType::TypeVar(id)) => Some(*id),
                _ => None,
            })
            .collect();
        let mut methods = HashMap::new();
        for method in &interface.methods {
            let params: Vec<(String, ResolvedType)> = method
                .params
                .iter()
                .map(|p| {
                    (
                        p.name.clone(),
                        self.resolve_type_with_bindings(&p.ty, &interface_generic_bindings),
                    )
                })
                .collect();
            methods.insert(
                method.name.clone(),
                FuncSig {
                    params,
                    return_type: self.resolve_type_with_bindings(
                        &method.return_type,
                        &interface_generic_bindings,
                    ),
                    generic_type_vars: Vec::new(),
                    is_variadic: false,
                    is_extern: false,
                    effects: Vec::new(),
                    is_pure: false,
                    allow_any: false,
                    has_explicit_effects: false,
                    span: span.clone(),
                },
            );
        }
        self.interfaces.insert(
            key.to_string(),
            InterfaceInfo {
                methods,
                generic_param_names: interface
                    .generic_params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect(),
                generic_type_vars: interface_generic_type_vars,
                extends: interface.extends.clone(),
                span,
            },
        );
    }

    pub(crate) fn insert_enum_info(&mut self, en: &EnumDecl, key: &str, span: Span) {
        let mut variants = HashMap::new();
        for variant in &en.variants {
            let fields = variant
                .fields
                .iter()
                .map(|f| self.resolve_type(&f.ty))
                .collect::<Vec<_>>();
            variants.insert(variant.name.clone(), fields);
            self.enum_variant_to_enum
                .insert(variant.name.clone(), key.to_string());
        }
        self.enums
            .insert(key.to_string(), EnumInfo { variants, span });
    }

    pub(crate) fn collect_module_declarations(
        &mut self,
        module: &ModuleDecl,
        prefix: &str,
        span: Span,
    ) {
        let saved_module_prefix = self.current_module_prefix.clone();
        self.current_module_prefix = Some(prefix.to_string());
        for inner_decl in &module.declarations {
            match &inner_decl.node {
                Decl::Function(func) => {
                    let prefixed_name = format!("{}__{}", prefix, func.name);
                    self.insert_function_signature(
                        func,
                        &prefixed_name,
                        inner_decl.span.clone(),
                        Some(format!("Function '{}'", prefixed_name)),
                    );
                }
                Decl::Class(class) => {
                    let prefixed_name = format!("{}__{}", prefix, class.name);
                    self.insert_class_info(class, &prefixed_name, inner_decl.span.clone());
                }
                Decl::Interface(interface) => {
                    let prefixed_name = format!("{}__{}", prefix, interface.name);
                    self.insert_interface_info(interface, &prefixed_name, inner_decl.span.clone());
                }
                Decl::Enum(en) => {
                    let prefixed_name = format!("{}__{}", prefix, en.name);
                    self.insert_enum_info(en, &prefixed_name, inner_decl.span.clone());
                }
                Decl::Module(nested) => {
                    let nested_prefix = format!("{}__{}", prefix, nested.name);
                    self.collect_module_declarations(nested, &nested_prefix, span.clone());
                }
                Decl::Import(_) => {}
            }
        }
        self.current_module_prefix = saved_module_prefix;
    }

    pub(crate) fn insert_function_signature(
        &mut self,
        func: &FunctionDecl,
        key: &str,
        span: Span,
        label_override: Option<String>,
    ) {
        let label = label_override.unwrap_or_else(|| format!("Function '{}'", key));
        self.validate_effect_attributes(&func.attributes, span.clone(), &label);
        self.validate_extern_signature(func, span.clone());

        let generic_bindings = self.make_generic_type_bindings(&func.generic_params);
        let generic_type_vars: Vec<usize> = func
            .generic_params
            .iter()
            .filter_map(|p| match generic_bindings.get(&p.name) {
                Some(ResolvedType::TypeVar(id)) => Some(*id),
                _ => None,
            })
            .collect();
        let params: Vec<(String, ResolvedType)> = func
            .params
            .iter()
            .map(|p| {
                (
                    p.name.clone(),
                    self.resolve_type_with_bindings(&p.ty, &generic_bindings),
                )
            })
            .collect();
        let mut return_type = self.resolve_type_with_bindings(&func.return_type, &generic_bindings);
        if func.is_async && !matches!(return_type, ResolvedType::Task(_)) {
            return_type = ResolvedType::Task(Box::new(return_type));
        }
        let (effects, is_pure, allow_any, has_explicit_effects) =
            self.parse_effects_from_attributes(&func.attributes);

        self.functions.insert(
            key.to_string(),
            FuncSig {
                params,
                return_type,
                generic_type_vars,
                is_variadic: func.is_variadic,
                is_extern: func.is_extern,
                effects,
                is_pure,
                allow_any,
                has_explicit_effects,
                span,
            },
        );
        self.register_function_leaf_name(key);
    }

    pub(crate) fn collect_module_function_signatures(&mut self, module: &ModuleDecl, prefix: &str) {
        for inner_decl in &module.declarations {
            match &inner_decl.node {
                Decl::Function(func) => {
                    let prefixed_name = format!("{}__{}", prefix, func.name);
                    self.insert_function_signature(
                        func,
                        &prefixed_name,
                        inner_decl.span.clone(),
                        Some(format!("Function '{}'", prefixed_name)),
                    );
                }
                Decl::Module(nested) => {
                    let nested_prefix = format!("{}__{}", prefix, nested.name);
                    self.collect_module_function_signatures(nested, &nested_prefix);
                }
                Decl::Class(_) | Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
            }
        }
    }
}
