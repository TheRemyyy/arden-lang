//! Apex Type Checker - Semantic analysis with type inference
//!
//! This module provides:
//! - Type checking and inference
//! - Symbol table management
//! - Scope tracking
//! - Type error reporting with source locations

#![allow(dead_code)]

use crate::ast::*;
use crate::stdlib::stdlib_registry;
use std::collections::HashMap;

/// Type checking error with source location
#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
    pub hint: Option<String>,
}

impl TypeError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// Resolved type with full information
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedType {
    Integer,
    Float,
    Boolean,
    String,
    Char,
    None,
    Class(String),
    Option(Box<ResolvedType>),
    Result(Box<ResolvedType>, Box<ResolvedType>),
    List(Box<ResolvedType>),
    Map(Box<ResolvedType>, Box<ResolvedType>),
    Set(Box<ResolvedType>),
    Ref(Box<ResolvedType>),
    MutRef(Box<ResolvedType>),
    Box(Box<ResolvedType>),
    Rc(Box<ResolvedType>),
    Arc(Box<ResolvedType>),
    Ptr(Box<ResolvedType>),
    Task(Box<ResolvedType>),
    Range(Box<ResolvedType>),
    Function(Vec<ResolvedType>, Box<ResolvedType>),
    /// Type variable for inference
    TypeVar(usize),
    /// Unknown type (error recovery)
    Unknown,
}

impl ResolvedType {
    pub fn is_numeric(&self) -> bool {
        matches!(self, ResolvedType::Integer | ResolvedType::Float)
    }

    pub fn is_reference(&self) -> bool {
        matches!(self, ResolvedType::Ref(_) | ResolvedType::MutRef(_))
    }

    pub fn inner_type(&self) -> Option<&ResolvedType> {
        match self {
            ResolvedType::Ref(inner) | ResolvedType::MutRef(inner) => Some(inner),
            ResolvedType::Option(inner) | ResolvedType::List(inner) => Some(inner),
            ResolvedType::Ptr(inner) => Some(inner),
            _ => None,
        }
    }
}

impl std::fmt::Display for ResolvedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedType::Integer => write!(f, "Integer"),
            ResolvedType::Float => write!(f, "Float"),
            ResolvedType::Boolean => write!(f, "Boolean"),
            ResolvedType::String => write!(f, "String"),
            ResolvedType::Char => write!(f, "Char"),
            ResolvedType::None => write!(f, "None"),
            ResolvedType::Class(name) => write!(f, "{}", name),
            ResolvedType::Option(inner) => write!(f, "Option<{}>", inner),
            ResolvedType::Result(ok, err) => write!(f, "Result<{}, {}>", ok, err),
            ResolvedType::List(inner) => write!(f, "List<{}>", inner),
            ResolvedType::Map(k, v) => write!(f, "Map<{}, {}>", k, v),
            ResolvedType::Set(inner) => write!(f, "Set<{}>", inner),
            ResolvedType::Ref(inner) => write!(f, "&{}", inner),
            ResolvedType::MutRef(inner) => write!(f, "&mut {}", inner),
            ResolvedType::Box(inner) => write!(f, "Box<{}>", inner),
            ResolvedType::Rc(inner) => write!(f, "Rc<{}>", inner),
            ResolvedType::Arc(inner) => write!(f, "Arc<{}>", inner),
            ResolvedType::Ptr(inner) => write!(f, "Ptr<{}>", inner),
            ResolvedType::Task(inner) => write!(f, "Task<{}>", inner),
            ResolvedType::Range(inner) => write!(f, "Range<{}>", inner),
            ResolvedType::Function(params, ret) => {
                write!(f, "(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            ResolvedType::TypeVar(id) => write!(f, "?T{}", id),
            ResolvedType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Variable information in symbol table
#[derive(Debug, Clone)]
pub struct VarInfo {
    pub ty: ResolvedType,
    pub mutable: bool,
    pub initialized: bool,
    pub span: Span,
}

/// Function signature
#[derive(Debug, Clone)]
pub struct FuncSig {
    pub params: Vec<(String, ResolvedType)>,
    pub return_type: ResolvedType,
    pub generic_type_vars: Vec<usize>,
    pub is_variadic: bool,
    pub is_extern: bool,
    pub effects: Vec<String>,
    pub is_pure: bool,
    pub allow_any: bool,
    pub has_explicit_effects: bool,
    pub span: Span,
}

pub type FunctionEffectsSummary = HashMap<String, Vec<String>>;
pub type ClassMethodEffectsSummary = HashMap<String, HashMap<String, Vec<String>>>;

/// Class information
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub fields: HashMap<String, (ResolvedType, bool, Visibility)>, // (type, mutable, visibility)
    pub methods: HashMap<String, FuncSig>,
    pub method_visibilities: HashMap<String, Visibility>,
    pub constructor: Option<Vec<(String, ResolvedType)>>,
    pub generic_type_vars: Vec<usize>,
    pub visibility: Visibility,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub span: Span,
}

/// Enum metadata used for type checking variant constructors and pattern matching
#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub variants: HashMap<String, Vec<ResolvedType>>,
    pub span: Span,
}

/// Interface metadata
#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub methods: HashMap<String, FuncSig>,
    pub extends: Vec<String>,
    pub span: Span,
}

/// Scope for symbol table
#[derive(Debug, Clone)]
struct Scope {
    variables: HashMap<String, VarInfo>,
    /// Parent scope index
    parent: Option<usize>,
}

/// Type checker state
pub struct TypeChecker {
    /// All scopes (index 0 is global)
    scopes: Vec<Scope>,
    /// Current scope index
    current_scope: usize,
    /// Function signatures
    functions: HashMap<String, FuncSig>,
    /// Class definitions
    classes: HashMap<String, ClassInfo>,
    /// Enum definitions
    enums: HashMap<String, EnumInfo>,
    /// Interface definitions
    interfaces: HashMap<String, InterfaceInfo>,
    /// Reverse lookup: variant name -> enum name
    enum_variant_to_enum: HashMap<String, String>,
    /// Type variable counter for inference
    type_var_counter: usize,
    /// Type variable substitutions
    substitutions: HashMap<usize, ResolvedType>,
    /// Collected errors
    errors: Vec<TypeError>,
    /// Current function return type (for checking returns)
    current_return_type: Option<ResolvedType>,
    /// Current class context (for visibility checks)
    current_class: Option<String>,
    /// Import aliases (alias -> path)
    import_aliases: HashMap<String, String>,
    /// Current function declared effects
    current_effects: Vec<String>,
    /// Whether current function is declared pure
    current_is_pure: bool,
    /// Whether current function allows any effects
    current_allow_any: bool,
    /// Source code for error messages
    source: String,
    /// Function/method generic type parameter bindings in current checking context
    current_generic_type_bindings: HashMap<String, ResolvedType>,
    /// Current nested module prefix while collecting/checking module-scoped declarations
    current_module_prefix: Option<String>,
}

impl TypeChecker {
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

    fn resolve_stdlib_alias_call_name(&self, alias_ident: &str, member: &str) -> Option<String> {
        // Local bindings must shadow import aliases.
        if self.lookup_variable(alias_ident).is_some() {
            return None;
        }
        let namespace_path = self.import_aliases.get(alias_ident)?;
        stdlib_registry().resolve_alias_call(namespace_path, member)
    }

    fn resolve_import_alias_symbol(&self, alias_ident: &str) -> Option<String> {
        // Local bindings must shadow import aliases.
        if self.lookup_variable(alias_ident).is_some() {
            return None;
        }
        let path = self.import_aliases.get(alias_ident)?;
        if path.ends_with(".*") {
            return None;
        }
        let mut parts = path.split('.').collect::<Vec<_>>();
        let symbol = parts.pop()?;
        let namespace = parts.join(".");
        if stdlib_registry()
            .get_namespace(symbol)
            .is_some_and(|owner| owner == &namespace)
        {
            return Some(symbol.to_string());
        }
        None
    }

    fn resolve_import_alias_variant(&self, alias_ident: &str) -> Option<(String, String)> {
        if self.lookup_variable(alias_ident).is_some() {
            return None;
        }
        let path = self.import_aliases.get(alias_ident)?;
        if path.ends_with(".*") {
            return None;
        }
        let (enum_path, variant_name) = path.rsplit_once('.')?;
        let (namespace, enum_name) = enum_path
            .rsplit_once('.')
            .map_or((String::new(), enum_path.to_string()), |(ns, name)| {
                (ns.to_string(), name.to_string())
            });
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

    fn resolve_enum_name(&self, name: &str) -> Option<String> {
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

    fn resolve_import_alias_module_candidate(
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
        let path = self.import_aliases.get(alias_ident)?;
        if path.ends_with(".*") {
            return None;
        }
        Some(format!(
            "{}__{}",
            path.replace('.', "__"),
            member_parts.join("__")
        ))
    }

    fn resolve_function_value_name<'a>(&'a self, name: &'a str) -> Option<&'a str> {
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

    pub fn new(source: String) -> Self {
        let global_scope = Scope {
            variables: HashMap::new(),
            parent: None,
        };

        Self {
            scopes: vec![global_scope],
            current_scope: 0,
            functions: HashMap::new(),
            classes: HashMap::new(),
            enums: HashMap::new(),
            interfaces: HashMap::new(),
            enum_variant_to_enum: HashMap::new(),
            type_var_counter: 0,
            substitutions: HashMap::new(),
            errors: Vec::new(),
            current_return_type: None,
            current_class: None,
            import_aliases: HashMap::new(),
            current_effects: Vec::new(),
            current_is_pure: false,
            current_allow_any: false,
            source,
            current_generic_type_bindings: HashMap::new(),
            current_module_prefix: None,
        }
    }

    fn module_scoped_type_name(&self, name: &str) -> Option<String> {
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

    fn module_scoped_generic_type(
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

    fn apply_effect_seeds(
        &mut self,
        function_effects: &HashMap<String, Vec<String>>,
        class_method_effects: &HashMap<String, HashMap<String, Vec<String>>>,
    ) {
        for (name, effects) in function_effects {
            if let Some(sig) = self.functions.get_mut(name) {
                sig.effects = effects.clone();
            }
        }
        for (class_name, methods) in class_method_effects {
            if let Some(class) = self.classes.get_mut(class_name) {
                for (method_name, effects) in methods {
                    if let Some(sig) = class.methods.get_mut(method_name) {
                        sig.effects = effects.clone();
                    }
                }
            }
        }
    }

    pub fn export_effect_summary(&self) -> (FunctionEffectsSummary, ClassMethodEffectsSummary) {
        let function_effects = self
            .functions
            .iter()
            .map(|(name, sig)| (name.clone(), sig.effects.clone()))
            .collect();
        let class_method_effects = self
            .classes
            .iter()
            .map(|(class_name, class)| {
                (
                    class_name.clone(),
                    class
                        .methods
                        .iter()
                        .map(|(method_name, sig)| (method_name.clone(), sig.effects.clone()))
                        .collect(),
                )
            })
            .collect();
        (function_effects, class_method_effects)
    }

    pub fn check_with_effect_seeds(
        &mut self,
        program: &Program,
        function_effects: &FunctionEffectsSummary,
        class_method_effects: &ClassMethodEffectsSummary,
    ) -> Result<(), Vec<TypeError>> {
        self.populate_import_aliases(program);
        self.collect_declarations(program);
        self.apply_effect_seeds(function_effects, class_method_effects);
        for (name, iface) in self.interfaces.clone() {
            for parent in iface.extends {
                if !self.interfaces.contains_key(&parent) {
                    self.error(
                        format!(
                            "Interface '{}' extends unknown interface '{}'",
                            name, parent
                        ),
                        iface.span.clone(),
                    );
                }
            }
        }
        self.infer_effects(program);

        for decl in &program.declarations {
            self.check_decl_with_prefix(&decl.node, decl.span.clone(), None);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    fn make_generic_type_bindings(
        &mut self,
        generic_params: &[GenericParam],
    ) -> HashMap<String, ResolvedType> {
        generic_params
            .iter()
            .map(|p| (p.name.clone(), self.fresh_type_var()))
            .collect()
    }

    #[allow(clippy::only_used_in_recursion)]
    fn resolve_type_with_bindings(
        &self,
        ty: &Type,
        bindings: &HashMap<String, ResolvedType>,
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
                if let Some(module_scoped) = self.module_scoped_type_name(name) {
                    return ResolvedType::Class(module_scoped);
                }
                match name.as_str() {
                    "Range" => ResolvedType::Class("Range".to_string()),
                    _ => ResolvedType::Class(name.clone()),
                }
            }
            Type::Option(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.module_scoped_generic_type("Option", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Option(Box::new(inner)))
            }
            Type::Result(ok, err) => {
                let ok = self.resolve_type_with_bindings(ok, bindings);
                let err = self.resolve_type_with_bindings(err, bindings);
                self.module_scoped_generic_type("Result", &[ok.clone(), err.clone()])
                    .unwrap_or_else(|| ResolvedType::Result(Box::new(ok), Box::new(err)))
            }
            Type::List(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.module_scoped_generic_type("List", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::List(Box::new(inner)))
            }
            Type::Map(k, v) => {
                let key = self.resolve_type_with_bindings(k, bindings);
                let value = self.resolve_type_with_bindings(v, bindings);
                self.module_scoped_generic_type("Map", &[key.clone(), value.clone()])
                    .unwrap_or_else(|| ResolvedType::Map(Box::new(key), Box::new(value)))
            }
            Type::Set(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.module_scoped_generic_type("Set", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Set(Box::new(inner)))
            }
            Type::Ref(inner) => {
                ResolvedType::Ref(Box::new(self.resolve_type_with_bindings(inner, bindings)))
            }
            Type::MutRef(inner) => {
                ResolvedType::MutRef(Box::new(self.resolve_type_with_bindings(inner, bindings)))
            }
            Type::Box(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.module_scoped_generic_type("Box", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Box(Box::new(inner)))
            }
            Type::Rc(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.module_scoped_generic_type("Rc", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Rc(Box::new(inner)))
            }
            Type::Arc(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.module_scoped_generic_type("Arc", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Arc(Box::new(inner)))
            }
            Type::Ptr(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.module_scoped_generic_type("Ptr", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Ptr(Box::new(inner)))
            }
            Type::Task(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.module_scoped_generic_type("Task", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Task(Box::new(inner)))
            }
            Type::Range(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.module_scoped_generic_type("Range", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Range(Box::new(inner)))
            }
            Type::Function(params, ret) => ResolvedType::Function(
                params
                    .iter()
                    .map(|p| self.resolve_type_with_bindings(p, bindings))
                    .collect(),
                Box::new(self.resolve_type_with_bindings(ret, bindings)),
            ),
            Type::Generic(name, args) => match name.as_str() {
                _ if self.module_scoped_type_name(name).is_some() => {
                    let resolved_name = self
                        .module_scoped_type_name(name)
                        .unwrap_or_else(|| name.clone());
                    let args = args
                        .iter()
                        .map(|arg| self.resolve_type_with_bindings(arg, bindings).to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    ResolvedType::Class(format!("{}<{}>", resolved_name, args))
                }
                "Option" if args.len() == 1 => ResolvedType::Option(Box::new(
                    self.resolve_type_with_bindings(&args[0], bindings),
                )),
                "Result" if args.len() == 2 => ResolvedType::Result(
                    Box::new(self.resolve_type_with_bindings(&args[0], bindings)),
                    Box::new(self.resolve_type_with_bindings(&args[1], bindings)),
                ),
                "List" if args.len() == 1 => ResolvedType::List(Box::new(
                    self.resolve_type_with_bindings(&args[0], bindings),
                )),
                "Map" if args.len() == 2 => ResolvedType::Map(
                    Box::new(self.resolve_type_with_bindings(&args[0], bindings)),
                    Box::new(self.resolve_type_with_bindings(&args[1], bindings)),
                ),
                "Set" if args.len() == 1 => ResolvedType::Set(Box::new(
                    self.resolve_type_with_bindings(&args[0], bindings),
                )),
                "Box" if args.len() == 1 => ResolvedType::Box(Box::new(
                    self.resolve_type_with_bindings(&args[0], bindings),
                )),
                "Rc" if args.len() == 1 => ResolvedType::Rc(Box::new(
                    self.resolve_type_with_bindings(&args[0], bindings),
                )),
                "Arc" if args.len() == 1 => ResolvedType::Arc(Box::new(
                    self.resolve_type_with_bindings(&args[0], bindings),
                )),
                "Ptr" if args.len() == 1 => ResolvedType::Ptr(Box::new(
                    self.resolve_type_with_bindings(&args[0], bindings),
                )),
                "Task" if args.len() == 1 => ResolvedType::Task(Box::new(
                    self.resolve_type_with_bindings(&args[0], bindings),
                )),
                "Range" if args.len() == 1 => ResolvedType::Range(Box::new(
                    self.resolve_type_with_bindings(&args[0], bindings),
                )),
                _ => {
                    let args = args
                        .iter()
                        .map(|arg| self.resolve_type_with_bindings(arg, bindings).to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    ResolvedType::Class(format!("{}<{}>", name, args))
                }
            },
        }
    }

    fn parse_effects_from_attributes(
        &self,
        attrs: &[Attribute],
    ) -> (Vec<String>, bool, bool, bool) {
        let mut effects = Vec::new();
        let mut pure = false;
        let mut any = false;
        let mut has_explicit_effects = false;

        for attr in attrs {
            match attr {
                Attribute::Pure => pure = true,
                Attribute::EffectIo => {
                    has_explicit_effects = true;
                    effects.push("io".to_string());
                }
                Attribute::EffectNet => {
                    has_explicit_effects = true;
                    effects.push("net".to_string());
                }
                Attribute::EffectAlloc => {
                    has_explicit_effects = true;
                    effects.push("alloc".to_string());
                }
                Attribute::EffectUnsafe => {
                    has_explicit_effects = true;
                    effects.push("unsafe".to_string());
                }
                Attribute::EffectThread => {
                    has_explicit_effects = true;
                    effects.push("thread".to_string());
                }
                Attribute::EffectAny => {
                    has_explicit_effects = true;
                    any = true;
                }
                _ => {}
            }
        }

        effects.sort();
        effects.dedup();
        (effects, pure, any, has_explicit_effects)
    }

    fn validate_effect_attributes(&mut self, attrs: &[Attribute], span: Span, subject: &str) {
        let (effects, pure, any, _) = self.parse_effects_from_attributes(attrs);
        if pure && any {
            self.error(
                format!(
                    "{} cannot use both @Pure and @Any; pick one effect policy",
                    subject
                ),
                span.clone(),
            );
        }
        if pure && !effects.is_empty() {
            self.error(
                format!(
                    "{} cannot combine @Pure with explicit effects ({})",
                    subject,
                    effects.join(", ")
                ),
                span,
            );
        }
    }

    fn validate_extern_signature(&mut self, func: &FunctionDecl, span: Span) {
        if !func.is_extern {
            return;
        }

        if func.is_async {
            self.error(
                format!("Extern function '{}' cannot be async", func.name),
                span.clone(),
            );
        }

        if func.is_variadic && func.params.is_empty() {
            self.error(
                format!(
                    "Variadic extern function '{}' must declare at least one fixed parameter",
                    func.name
                ),
                span.clone(),
            );
        }

        if let Some(abi) = &func.extern_abi {
            if abi != "c" && abi != "system" {
                self.error(
                    format!(
                        "Extern function '{}' uses unsupported ABI '{}'",
                        func.name, abi
                    ),
                    span.clone(),
                );
            }
        }

        for param in &func.params {
            let resolved = self.resolve_type(&param.ty);
            if !self.is_ffi_safe_type(&resolved) {
                self.error(
                    format!(
                        "Extern function '{}' has non-FFI-safe parameter '{}: {}'",
                        func.name, param.name, resolved
                    ),
                    span.clone(),
                );
            }
        }
        let ret = self.resolve_type(&func.return_type);
        if !self.is_ffi_safe_type(&ret) {
            self.error(
                format!(
                    "Extern function '{}' has non-FFI-safe return type '{}'",
                    func.name, ret
                ),
                span,
            );
        }
    }

    fn is_ffi_safe_type(&self, ty: &ResolvedType) -> bool {
        matches!(
            ty,
            ResolvedType::Integer
                | ResolvedType::Float
                | ResolvedType::Boolean
                | ResolvedType::Char
                | ResolvedType::String
                | ResolvedType::None
                | ResolvedType::Ptr(_)
        )
    }

    fn builtin_required_effect(name: &str) -> Option<&'static str> {
        if matches!(
            name,
            "println"
                | "print"
                | "read_line"
                | "File__read"
                | "File__write"
                | "File__delete"
                | "File__exists"
                | "System__exec"
                | "System__shell"
        ) {
            return Some("io");
        }
        if matches!(
            name,
            "System__getenv" | "System__cwd" | "System__os" | "Args__count" | "Args__get"
        ) {
            return Some("io");
        }
        if matches!(name, "Time__sleep" | "Time__now" | "Time__unix") {
            return Some("thread");
        }
        None
    }

    fn enforce_required_effect(&mut self, effect: &str, span: Span, callee: &str) {
        if self.current_is_pure {
            self.error(
                format!(
                    "Pure function cannot call effectful function '{}', required effect: {}",
                    callee, effect
                ),
                span,
            );
            return;
        }

        if self.current_allow_any {
            return;
        }

        if !self.current_effects.iter().any(|e| e == effect) {
            self.error(
                format!(
                    "Missing effect '{}' for call to '{}'. Add @{} (or @Any) on the caller function",
                    effect,
                    callee,
                    match effect {
                        "io" => "Io",
                        "net" => "Net",
                        "alloc" => "Alloc",
                        "unsafe" => "Unsafe",
                        "thread" => "Thread",
                        _ => "Io",
                    }
                ),
                span,
            );
        }
    }

    fn enforce_call_effects(&mut self, sig: &FuncSig, span: Span, callee: &str) {
        if sig.is_pure || sig.allow_any {
            return;
        }
        for eff in &sig.effects {
            self.enforce_required_effect(eff, span.clone(), callee);
        }
    }

    fn infer_effects(&mut self, program: &Program) {
        // Fixed-point inference over the call graph for declarations without explicit effects.
        let mut changed = true;
        let mut passes = 0usize;
        while changed && passes < 24 {
            changed = false;
            passes += 1;
            for decl in &program.declarations {
                changed |= self.infer_effects_decl(&decl.node, None);
            }
        }
    }

    fn infer_effects_decl(&mut self, decl: &Decl, module_prefix: Option<&str>) -> bool {
        match decl {
            Decl::Function(func) => {
                let key = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, func.name)
                } else {
                    func.name.clone()
                };
                self.infer_effects_for_function_key(&key, &func.body, None)
            }
            Decl::Class(class) => {
                let mut changed = false;
                for method in &class.methods {
                    changed |=
                        self.infer_effects_for_method(&class.name, &method.name, &method.body);
                }
                changed
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                let mut changed = false;
                for inner in &module.declarations {
                    changed |= self.infer_effects_decl(&inner.node, Some(&next_prefix));
                }
                changed
            }
            _ => false,
        }
    }

    fn infer_effects_for_function_key(
        &mut self,
        key: &str,
        body: &Block,
        current_class: Option<&str>,
    ) -> bool {
        let Some(sig) = self.functions.get(key).cloned() else {
            return false;
        };
        if sig.is_pure || sig.allow_any || sig.has_explicit_effects {
            return false;
        }

        let mut inferred: Vec<String> = self
            .infer_effects_in_block(body, current_class)
            .into_iter()
            .collect();
        inferred.sort();
        inferred.dedup();

        if inferred != sig.effects {
            if let Some(edit_sig) = self.functions.get_mut(key) {
                edit_sig.effects = inferred;
                return true;
            }
        }
        false
    }

    fn infer_effects_for_method(
        &mut self,
        class_name: &str,
        method_name: &str,
        body: &Block,
    ) -> bool {
        let Some(class_info) = self.classes.get(class_name).cloned() else {
            return false;
        };
        let Some(method_sig) = class_info.methods.get(method_name).cloned() else {
            return false;
        };
        if method_sig.is_pure || method_sig.allow_any || method_sig.has_explicit_effects {
            return false;
        }

        let mut inferred: Vec<String> = self
            .infer_effects_in_block(body, Some(class_name))
            .into_iter()
            .collect();
        inferred.sort();
        inferred.dedup();

        if inferred != method_sig.effects {
            if let Some(class_edit) = self.classes.get_mut(class_name) {
                if let Some(sig_edit) = class_edit.methods.get_mut(method_name) {
                    sig_edit.effects = inferred;
                    return true;
                }
            }
        }
        false
    }

    fn infer_effects_in_block(
        &self,
        block: &Block,
        current_class: Option<&str>,
    ) -> std::collections::BTreeSet<String> {
        let mut out = std::collections::BTreeSet::new();
        for stmt in block {
            self.collect_effects_stmt(&stmt.node, current_class, &mut out);
        }
        out
    }

    fn collect_effects_stmt(
        &self,
        stmt: &Stmt,
        current_class: Option<&str>,
        out: &mut std::collections::BTreeSet<String>,
    ) {
        match stmt {
            Stmt::Let { value, .. } => self.collect_effects_expr(&value.node, current_class, out),
            Stmt::Assign { target, value } => {
                self.collect_effects_expr(&target.node, current_class, out);
                self.collect_effects_expr(&value.node, current_class, out);
            }
            Stmt::Expr(expr) => self.collect_effects_expr(&expr.node, current_class, out),
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    self.collect_effects_expr(&expr.node, current_class, out);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.collect_effects_expr(&condition.node, current_class, out);
                for s in then_block {
                    self.collect_effects_stmt(&s.node, current_class, out);
                }
                if let Some(else_block) = else_block {
                    for s in else_block {
                        self.collect_effects_stmt(&s.node, current_class, out);
                    }
                }
            }
            Stmt::While { condition, body } => {
                self.collect_effects_expr(&condition.node, current_class, out);
                for s in body {
                    self.collect_effects_stmt(&s.node, current_class, out);
                }
            }
            Stmt::For { iterable, body, .. } => {
                self.collect_effects_expr(&iterable.node, current_class, out);
                for s in body {
                    self.collect_effects_stmt(&s.node, current_class, out);
                }
            }
            Stmt::Match { expr, arms } => {
                self.collect_effects_expr(&expr.node, current_class, out);
                for arm in arms {
                    for s in &arm.body {
                        self.collect_effects_stmt(&s.node, current_class, out);
                    }
                }
            }
            Stmt::Break | Stmt::Continue => {}
        }
    }

    fn collect_effects_expr(
        &self,
        expr: &Expr,
        current_class: Option<&str>,
        out: &mut std::collections::BTreeSet<String>,
    ) {
        match expr {
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident(name) = &callee.node {
                    let canonical = self
                        .resolve_import_alias_symbol(name)
                        .unwrap_or_else(|| name.clone());
                    if let Some(required) = Self::builtin_required_effect(&canonical) {
                        out.insert(required.to_string());
                    }
                    if let Some(sig) = self.functions.get(&canonical) {
                        for eff in &sig.effects {
                            out.insert(eff.clone());
                        }
                    }
                } else if let Expr::Field { object, field } = &callee.node {
                    if let Expr::Ident(name) = &object.node {
                        let builtin_name = self
                            .resolve_stdlib_alias_call_name(name, field)
                            .unwrap_or_else(|| format!("{}__{}", name, field));
                        if let Some(required) = Self::builtin_required_effect(&builtin_name) {
                            out.insert(required.to_string());
                        }
                        if let Some(sig) = self.functions.get(&builtin_name) {
                            for eff in &sig.effects {
                                out.insert(eff.clone());
                            }
                        }
                        // Instance-style method call; infer conservatively by method name across classes.
                        self.collect_class_method_name_effects(field, out);
                    } else if matches!(object.node, Expr::This) {
                        if let Some(class_name) = current_class {
                            if let Some(class_info) = self.classes.get(class_name) {
                                if let Some(sig) = class_info.methods.get(field) {
                                    for eff in &sig.effects {
                                        out.insert(eff.clone());
                                    }
                                }
                            }
                        }
                    } else {
                        self.collect_class_method_name_effects(field, out);
                    }
                }

                self.collect_effects_expr(&callee.node, current_class, out);
                for arg in args {
                    self.collect_effects_expr(&arg.node, current_class, out);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.collect_effects_expr(&left.node, current_class, out);
                self.collect_effects_expr(&right.node, current_class, out);
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => {
                self.collect_effects_expr(&expr.node, current_class, out);
            }
            Expr::Field { object, .. } => {
                self.collect_effects_expr(&object.node, current_class, out)
            }
            Expr::Index { object, index } => {
                self.collect_effects_expr(&object.node, current_class, out);
                self.collect_effects_expr(&index.node, current_class, out);
            }
            Expr::Construct { args, .. } => {
                for arg in args {
                    self.collect_effects_expr(&arg.node, current_class, out);
                }
            }
            Expr::Lambda { body, .. } => self.collect_effects_expr(&body.node, current_class, out),
            Expr::Match { expr, arms } => {
                self.collect_effects_expr(&expr.node, current_class, out);
                for arm in arms {
                    for s in &arm.body {
                        self.collect_effects_stmt(&s.node, current_class, out);
                    }
                }
            }
            Expr::StringInterp(parts) => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        self.collect_effects_expr(&e.node, current_class, out);
                    }
                }
            }
            Expr::AsyncBlock(body) | Expr::Block(body) => {
                for s in body {
                    self.collect_effects_stmt(&s.node, current_class, out);
                }
            }
            Expr::Require { condition, message } => {
                self.collect_effects_expr(&condition.node, current_class, out);
                if let Some(msg) = message {
                    self.collect_effects_expr(&msg.node, current_class, out);
                }
            }
            Expr::Range { start, end, .. } => {
                if let Some(s) = start {
                    self.collect_effects_expr(&s.node, current_class, out);
                }
                if let Some(e) = end {
                    self.collect_effects_expr(&e.node, current_class, out);
                }
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_effects_expr(&condition.node, current_class, out);
                for s in then_branch {
                    self.collect_effects_stmt(&s.node, current_class, out);
                }
                if let Some(else_branch) = else_branch {
                    for s in else_branch {
                        self.collect_effects_stmt(&s.node, current_class, out);
                    }
                }
            }
            Expr::Literal(_) | Expr::Ident(_) | Expr::This => {}
        }
    }

    fn collect_class_method_name_effects(
        &self,
        method_name: &str,
        out: &mut std::collections::BTreeSet<String>,
    ) {
        for class_info in self.classes.values() {
            if let Some(sig) = class_info.methods.get(method_name) {
                for eff in &sig.effects {
                    out.insert(eff.clone());
                }
            }
        }
    }

    fn populate_import_aliases(&mut self, program: &Program) {
        self.import_aliases.clear();
        for decl in &program.declarations {
            if let Decl::Import(import) = &decl.node {
                if let Some(alias) = &import.alias {
                    self.import_aliases
                        .insert(alias.clone(), import.path.clone());
                }
            }
        }
    }

    fn is_same_or_subclass_of(&self, class_name: &str, ancestor: &str) -> bool {
        if class_name == ancestor {
            return true;
        }

        let mut current = class_name;
        let mut depth = 0usize;
        while depth < 64 {
            let Some(info) = self.classes.get(current) else {
                return false;
            };
            let Some(parent) = &info.extends else {
                return false;
            };
            if parent == ancestor {
                return true;
            }
            current = parent;
            depth += 1;
        }
        false
    }

    fn class_implements_interface(&self, class_name: &str, interface_name: &str) -> bool {
        let mut current = class_name;
        let mut depth = 0usize;
        while depth < 64 {
            let Some(info) = self.classes.get(current) else {
                return false;
            };

            if info
                .implements
                .iter()
                .any(|i| i == interface_name || self.interface_extends(i, interface_name))
            {
                return true;
            }

            let Some(parent) = &info.extends else {
                return false;
            };
            current = parent;
            depth += 1;
        }
        false
    }

    fn interface_extends(&self, interface_name: &str, target: &str) -> bool {
        if interface_name == target {
            return true;
        }
        let mut stack = vec![interface_name.to_string()];
        let mut visited = std::collections::HashSet::new();
        while let Some(name) = stack.pop() {
            if !visited.insert(name.clone()) {
                continue;
            }
            if name == target {
                return true;
            }
            if let Some(info) = self.interfaces.get(&name) {
                for parent in &info.extends {
                    stack.push(parent.clone());
                }
            }
        }
        false
    }

    fn collect_interface_methods(
        &self,
        interface_name: &str,
        out: &mut HashMap<String, FuncSig>,
        visited: &mut std::collections::HashSet<String>,
    ) {
        if !visited.insert(interface_name.to_string()) {
            return;
        }
        let Some(info) = self.interfaces.get(interface_name) else {
            return;
        };
        for parent in &info.extends {
            self.collect_interface_methods(parent, out, visited);
        }
        for (name, sig) in &info.methods {
            out.insert(name.clone(), sig.clone());
        }
    }

    fn signatures_compatible(&self, expected: &FuncSig, actual: &FuncSig) -> bool {
        if expected.params.len() != actual.params.len() {
            return false;
        }
        for ((_, e), (_, a)) in expected.params.iter().zip(actual.params.iter()) {
            if !self.types_compatible(e, a) {
                return false;
            }
        }
        self.types_compatible(&expected.return_type, &actual.return_type)
    }

    fn class_base_name<'a>(&self, class_name: &'a str) -> &'a str {
        class_name.split('<').next().unwrap_or(class_name)
    }

    fn instantiated_class_substitutions(
        &self,
        class_name: &str,
    ) -> (String, HashMap<usize, ResolvedType>) {
        let base_name = self.class_base_name(class_name).to_string();
        let Some(class) = self.classes.get(&base_name) else {
            return (base_name, HashMap::new());
        };
        if class.generic_type_vars.is_empty()
            || !class_name.contains('<')
            || !class_name.ends_with('>')
        {
            return (base_name, HashMap::new());
        }

        let Some(open_bracket) = class_name.find('<') else {
            return (base_name, HashMap::new());
        };
        let inner = &class_name[open_bracket + 1..class_name.len() - 1];
        let parts = self.split_generic_args(inner);
        if parts.len() != class.generic_type_vars.len() {
            return (base_name, HashMap::new());
        }

        let substitutions = class
            .generic_type_vars
            .iter()
            .zip(parts.iter())
            .map(|(id, part)| (*id, self.parse_type_string(part)))
            .collect();
        (base_name, substitutions)
    }

    fn lookup_class_method(
        &self,
        class_name: &str,
        method_name: &str,
    ) -> Option<(String, FuncSig, Visibility)> {
        let mut current = self.class_base_name(class_name).to_string();
        let mut depth = 0usize;
        while depth < 64 {
            let class = self.classes.get(&current)?;
            if let Some(sig) = class.methods.get(method_name) {
                let vis = class
                    .method_visibilities
                    .get(method_name)
                    .copied()
                    .unwrap_or(Visibility::Private);
                return Some((current, sig.clone(), vis));
            }
            match &class.extends {
                Some(parent) => current = parent.clone(),
                None => break,
            }
            depth += 1;
        }
        None
    }

    fn lookup_class_field(
        &self,
        class_name: &str,
        field_name: &str,
    ) -> Option<(String, ResolvedType, bool, Visibility)> {
        let mut current = self.class_base_name(class_name).to_string();
        let mut depth = 0usize;
        while depth < 64 {
            let class = self.classes.get(&current)?;
            if let Some((ty, mutable, visibility)) = class.fields.get(field_name) {
                return Some((current, ty.clone(), *mutable, *visibility));
            }
            match &class.extends {
                Some(parent) => current = parent.clone(),
                None => break,
            }
            depth += 1;
        }
        None
    }

    fn check_member_visibility(
        &mut self,
        owner_class: &str,
        visibility: Visibility,
        member_kind: &str,
        member_name: &str,
        span: Span,
    ) {
        match visibility {
            Visibility::Public => {}
            Visibility::Private => {
                let allowed = self
                    .current_class
                    .as_ref()
                    .map(|c| c == owner_class)
                    .unwrap_or(false);
                if !allowed {
                    self.error(
                        format!(
                            "{} '{}' is private to class '{}'",
                            member_kind, member_name, owner_class
                        ),
                        span,
                    );
                }
            }
            Visibility::Protected => {
                let allowed = self
                    .current_class
                    .as_ref()
                    .map(|c| self.is_same_or_subclass_of(c, owner_class))
                    .unwrap_or(false);
                if !allowed {
                    self.error(
                        format!(
                            "{} '{}' is protected in class '{}'",
                            member_kind, member_name, owner_class
                        ),
                        span,
                    );
                }
            }
        }
    }

    fn check_class_visibility(&mut self, class_name: &str, span: Span) {
        let Some(class) = self.classes.get(class_name) else {
            return;
        };
        match class.visibility {
            Visibility::Public => {}
            Visibility::Private => {
                let allowed = self
                    .current_class
                    .as_ref()
                    .map(|c| c == class_name)
                    .unwrap_or(false);
                if !allowed {
                    self.error(format!("Class '{}' is private", class_name), span);
                }
            }
            Visibility::Protected => {
                let allowed = self
                    .current_class
                    .as_ref()
                    .map(|c| self.is_same_or_subclass_of(c, class_name))
                    .unwrap_or(false);
                if !allowed {
                    self.error(format!("Class '{}' is protected", class_name), span);
                }
            }
        }
    }

    fn check_type_visibility(&mut self, ty: &ResolvedType, span: Span) {
        match ty {
            ResolvedType::Class(name) => {
                self.check_class_visibility(self.class_base_name(name), span)
            }
            ResolvedType::Option(inner)
            | ResolvedType::List(inner)
            | ResolvedType::Set(inner)
            | ResolvedType::Ref(inner)
            | ResolvedType::MutRef(inner)
            | ResolvedType::Box(inner)
            | ResolvedType::Rc(inner)
            | ResolvedType::Arc(inner)
            | ResolvedType::Ptr(inner)
            | ResolvedType::Task(inner)
            | ResolvedType::Range(inner) => self.check_type_visibility(inner, span),
            ResolvedType::Result(ok, err) | ResolvedType::Map(ok, err) => {
                self.check_type_visibility(ok, span.clone());
                self.check_type_visibility(err, span);
            }
            ResolvedType::Function(params, ret) => {
                for p in params {
                    self.check_type_visibility(p, span.clone());
                }
                self.check_type_visibility(ret, span);
            }
            ResolvedType::Integer
            | ResolvedType::Float
            | ResolvedType::Boolean
            | ResolvedType::String
            | ResolvedType::Char
            | ResolvedType::None
            | ResolvedType::TypeVar(_)
            | ResolvedType::Unknown => {}
        }
    }

    /// Run type checking on a program
    pub fn check(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        self.populate_import_aliases(program);
        // First pass: collect all declarations
        self.collect_declarations(program);
        for (name, iface) in self.interfaces.clone() {
            for parent in iface.extends {
                if !self.interfaces.contains_key(&parent) {
                    self.error(
                        format!(
                            "Interface '{}' extends unknown interface '{}'",
                            name, parent
                        ),
                        iface.span.clone(),
                    );
                }
            }
        }
        // Infer effects for non-annotated call graph nodes
        self.infer_effects(program);

        // Second pass: check all function bodies
        for decl in &program.declarations {
            self.check_decl_with_prefix(&decl.node, decl.span.clone(), None);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    /// Collect all top-level declarations
    fn collect_declarations(&mut self, program: &Program) {
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

    fn insert_class_info(&mut self, class: &ClassDecl, key: &str, span: Span) {
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
                    self.resolve_type_with_bindings(&field.ty, &class_generic_bindings),
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
                        self.resolve_type_with_bindings(&p.ty, &generic_bindings),
                    )
                })
                .collect();

            let mut return_type =
                self.resolve_type_with_bindings(&method.return_type, &generic_bindings);
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
                        self.resolve_type_with_bindings(&p.ty, &class_generic_bindings),
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

    fn insert_interface_info(&mut self, interface: &InterfaceDecl, key: &str, span: Span) {
        let mut methods = HashMap::new();
        for method in &interface.methods {
            let params: Vec<(String, ResolvedType)> = method
                .params
                .iter()
                .map(|p| (p.name.clone(), self.resolve_type(&p.ty)))
                .collect();
            methods.insert(
                method.name.clone(),
                FuncSig {
                    params,
                    return_type: self.resolve_type(&method.return_type),
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
                extends: interface.extends.clone(),
                span,
            },
        );
    }

    fn insert_enum_info(&mut self, en: &EnumDecl, key: &str, span: Span) {
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

    fn collect_module_declarations(&mut self, module: &ModuleDecl, prefix: &str, span: Span) {
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

    fn insert_function_signature(
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
    }

    fn collect_module_function_signatures(&mut self, module: &ModuleDecl, prefix: &str) {
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

    fn enum_payload_supported_for_codegen(&self, ty: &ResolvedType) -> bool {
        matches!(
            ty,
            ResolvedType::Integer
                | ResolvedType::Boolean
                | ResolvedType::Char
                | ResolvedType::Float
                | ResolvedType::String
                | ResolvedType::Ref(_)
                | ResolvedType::MutRef(_)
                | ResolvedType::Ptr(_)
        ) || matches!(ty, ResolvedType::Class(name) if !self.enums.contains_key(name))
    }

    fn type_contains_borrowed_reference(ty: &ResolvedType) -> bool {
        match ty {
            ResolvedType::Ref(_) | ResolvedType::MutRef(_) => true,
            ResolvedType::Option(inner)
            | ResolvedType::List(inner)
            | ResolvedType::Set(inner)
            | ResolvedType::Box(inner)
            | ResolvedType::Rc(inner)
            | ResolvedType::Arc(inner)
            | ResolvedType::Ptr(inner)
            | ResolvedType::Task(inner)
            | ResolvedType::Range(inner) => Self::type_contains_borrowed_reference(inner),
            ResolvedType::Result(ok, err) | ResolvedType::Map(ok, err) => {
                Self::type_contains_borrowed_reference(ok)
                    || Self::type_contains_borrowed_reference(err)
            }
            ResolvedType::Function(params, ret) => {
                params.iter().any(Self::type_contains_borrowed_reference)
                    || Self::type_contains_borrowed_reference(ret)
            }
            _ => false,
        }
    }

    fn check_async_result_type(&mut self, ty: &ResolvedType, context: &str, span: Span) {
        if Self::type_contains_borrowed_reference(ty) {
            self.error(
                format!(
                    "{} cannot return a value containing borrowed references across an async boundary: {}",
                    context, ty
                ),
                span,
            );
        }
    }

    fn expr_mentions_ident(expr: &Expr, ident: &str) -> bool {
        match expr {
            Expr::Ident(name) => name == ident,
            Expr::Binary { left, right, .. } => {
                Self::expr_mentions_ident(&left.node, ident)
                    || Self::expr_mentions_ident(&right.node, ident)
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => Self::expr_mentions_ident(&expr.node, ident),
            Expr::Call { callee, args, .. } => {
                Self::expr_mentions_ident(&callee.node, ident)
                    || args
                        .iter()
                        .any(|arg| Self::expr_mentions_ident(&arg.node, ident))
            }
            Expr::Field { object, .. } => Self::expr_mentions_ident(&object.node, ident),
            Expr::Index { object, index } => {
                Self::expr_mentions_ident(&object.node, ident)
                    || Self::expr_mentions_ident(&index.node, ident)
            }
            Expr::Construct { args, .. } => args
                .iter()
                .any(|arg| Self::expr_mentions_ident(&arg.node, ident)),
            Expr::Lambda { body, .. } => Self::expr_mentions_ident(&body.node, ident),
            Expr::Match { expr, arms } => {
                Self::expr_mentions_ident(&expr.node, ident)
                    || arms.iter().any(|arm| {
                        arm.body
                            .iter()
                            .any(|stmt| Self::stmt_mentions_ident(&stmt.node, ident))
                    })
            }
            Expr::StringInterp(parts) => parts.iter().any(|part| match part {
                StringPart::Literal(_) => false,
                StringPart::Expr(expr) => Self::expr_mentions_ident(&expr.node, ident),
            }),
            Expr::AsyncBlock(body) | Expr::Block(body) => body
                .iter()
                .any(|stmt| Self::stmt_mentions_ident(&stmt.node, ident)),
            Expr::Require { condition, message } => {
                Self::expr_mentions_ident(&condition.node, ident)
                    || message
                        .as_ref()
                        .is_some_and(|msg| Self::expr_mentions_ident(&msg.node, ident))
            }
            Expr::Range { start, end, .. } => {
                start
                    .as_ref()
                    .is_some_and(|expr| Self::expr_mentions_ident(&expr.node, ident))
                    || end
                        .as_ref()
                        .is_some_and(|expr| Self::expr_mentions_ident(&expr.node, ident))
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expr_mentions_ident(&condition.node, ident)
                    || then_branch
                        .iter()
                        .any(|stmt| Self::stmt_mentions_ident(&stmt.node, ident))
                    || else_branch.as_ref().is_some_and(|stmts| {
                        stmts
                            .iter()
                            .any(|stmt| Self::stmt_mentions_ident(&stmt.node, ident))
                    })
            }
            Expr::Literal(_) | Expr::This => false,
        }
    }

    fn stmt_mentions_ident(stmt: &Stmt, ident: &str) -> bool {
        match stmt {
            Stmt::Let { value, .. } => Self::expr_mentions_ident(&value.node, ident),
            Stmt::Assign { target, value } => {
                Self::expr_mentions_ident(&target.node, ident)
                    || Self::expr_mentions_ident(&value.node, ident)
            }
            Stmt::Expr(expr) => Self::expr_mentions_ident(&expr.node, ident),
            Stmt::Return(expr) => expr
                .as_ref()
                .is_some_and(|expr| Self::expr_mentions_ident(&expr.node, ident)),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                Self::expr_mentions_ident(&condition.node, ident)
                    || then_block
                        .iter()
                        .any(|stmt| Self::stmt_mentions_ident(&stmt.node, ident))
                    || else_block.as_ref().is_some_and(|stmts| {
                        stmts
                            .iter()
                            .any(|stmt| Self::stmt_mentions_ident(&stmt.node, ident))
                    })
            }
            Stmt::While { condition, body } => {
                Self::expr_mentions_ident(&condition.node, ident)
                    || body
                        .iter()
                        .any(|stmt| Self::stmt_mentions_ident(&stmt.node, ident))
            }
            Stmt::For { iterable, body, .. } => {
                Self::expr_mentions_ident(&iterable.node, ident)
                    || body
                        .iter()
                        .any(|stmt| Self::stmt_mentions_ident(&stmt.node, ident))
            }
            Stmt::Match { expr, arms } => {
                Self::expr_mentions_ident(&expr.node, ident)
                    || arms.iter().any(|arm| {
                        arm.body
                            .iter()
                            .any(|stmt| Self::stmt_mentions_ident(&stmt.node, ident))
                    })
            }
            Stmt::Break | Stmt::Continue => false,
        }
    }

    /// Check a declaration
    fn check_decl_with_prefix(&mut self, decl: &Decl, span: Span, module_prefix: Option<&str>) {
        match decl {
            Decl::Function(func) => {
                let saved_module_prefix = self.current_module_prefix.clone();
                self.current_module_prefix = module_prefix.map(|p| p.to_string());
                let key = module_prefix.map(|p| format!("{}__{}", p, func.name));
                self.check_function(func, span, key.as_deref());
                self.current_module_prefix = saved_module_prefix;
            }
            Decl::Class(class) => {
                let saved_module_prefix = self.current_module_prefix.clone();
                self.current_module_prefix = module_prefix.map(|p| p.to_string());
                let key = module_prefix.map(|p| format!("{}__{}", p, class.name));
                self.check_class_named(class, span, key.as_deref().unwrap_or(&class.name));
                self.current_module_prefix = saved_module_prefix;
            }
            Decl::Enum(en) => {
                let saved_module_prefix = self.current_module_prefix.clone();
                self.current_module_prefix = module_prefix.map(|p| p.to_string());
                for variant in &en.variants {
                    for field in &variant.fields {
                        let ty = self.resolve_type(&field.ty);
                        if !self.enum_payload_supported_for_codegen(&ty) {
                            self.error(
                                format!(
                                    "Enum payload type '{}' is not supported yet; only primitive scalars, strings, class references, refs, mut refs, and raw pointers are supported in enum variants",
                                    ty
                                ),
                                span.clone(),
                            );
                        }
                    }
                }
                self.current_module_prefix = saved_module_prefix;
            }
            Decl::Interface(interface) => self.check_interface(interface, span),
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                for inner_decl in &module.declarations {
                    self.check_decl_with_prefix(
                        &inner_decl.node,
                        inner_decl.span.clone(),
                        Some(&next_prefix),
                    );
                }
            }
            _ => {}
        }
    }

    fn check_interface(&mut self, interface: &InterfaceDecl, span: Span) {
        for method in &interface.methods {
            let saved_generic_bindings = std::mem::take(&mut self.current_generic_type_bindings);
            for param in &method.params {
                let ty = self.resolve_type(&param.ty);
                self.check_type_visibility(&ty, span.clone());
            }
            let ret_ty = self.resolve_type(&method.return_type);
            self.check_type_visibility(&ret_ty, span.clone());
            self.current_generic_type_bindings = saved_generic_bindings;
        }

        for method in &interface.methods {
            let Some(body) = &method.default_impl else {
                continue;
            };
            let saved_generic_bindings = std::mem::take(&mut self.current_generic_type_bindings);
            self.enter_scope();
            let saved_effects = std::mem::take(&mut self.current_effects);
            let saved_pure = self.current_is_pure;
            let saved_any = self.current_allow_any;
            self.current_allow_any = true;
            self.current_is_pure = false;
            for param in &method.params {
                let ty = self.resolve_type(&param.ty);
                self.declare_variable(&param.name, ty, param.mutable, span.clone());
            }
            self.current_return_type = Some(self.resolve_type(&method.return_type));
            self.check_block(body);
            self.current_return_type = None;
            self.current_effects = saved_effects;
            self.current_is_pure = saved_pure;
            self.current_allow_any = saved_any;
            self.exit_scope();
            self.current_generic_type_bindings = saved_generic_bindings;
        }
    }

    /// Check a function
    fn check_function(&mut self, func: &FunctionDecl, span: Span, function_key: Option<&str>) {
        let saved_generic_bindings = std::mem::take(&mut self.current_generic_type_bindings);
        self.current_generic_type_bindings = self.make_generic_type_bindings(&func.generic_params);
        self.enter_scope();
        let saved_effects = std::mem::take(&mut self.current_effects);
        let saved_pure = self.current_is_pure;
        let saved_any = self.current_allow_any;
        let sig = function_key
            .and_then(|k| self.functions.get(k))
            .or_else(|| self.functions.get(&func.name));
        if let Some(sig) = sig {
            self.current_effects = sig.effects.clone();
            self.current_is_pure = sig.is_pure;
            self.current_allow_any = sig.allow_any;
        } else {
            // Fallback for unresolved keys; should be rare.
            let (effects, is_pure, allow_any, _) =
                self.parse_effects_from_attributes(&func.attributes);
            self.current_effects = effects;
            self.current_is_pure = is_pure;
            self.current_allow_any = allow_any;
        }

        // Add parameters to scope
        for param in &func.params {
            let ty = self.resolve_type(&param.ty);
            self.check_type_visibility(&ty, span.clone());
            if func.is_async && Self::type_contains_borrowed_reference(&ty) {
                self.error(
                    format!(
                        "Async function '{}' cannot accept a parameter containing borrowed references: {}",
                        func.name, ty
                    ),
                    span.clone(),
                );
            }
            self.declare_variable(&param.name, ty, param.mutable, span.clone());
        }

        // Set current return type
        let return_type = self.resolve_type(&func.return_type);
        self.check_type_visibility(&return_type, span.clone());
        let mut inner_return_type = return_type.clone();
        if func.is_async {
            if let ResolvedType::Task(inner) = &return_type {
                inner_return_type = (**inner).clone();
            }
            self.check_async_result_type(
                &inner_return_type,
                &format!("Async function '{}'", func.name),
                span.clone(),
            );
        }
        self.current_return_type = Some(inner_return_type);

        // Check body (extern declarations have no body)
        if !func.is_extern {
            self.check_block(&func.body);
        }

        self.current_return_type = None;
        self.current_effects = saved_effects;
        self.current_is_pure = saved_pure;
        self.current_allow_any = saved_any;
        self.exit_scope();
        self.current_generic_type_bindings = saved_generic_bindings;
    }

    /// Check a class
    fn check_class(&mut self, class: &ClassDecl, span: Span) {
        self.check_class_named(class, span, &class.name);
    }

    fn check_class_named(&mut self, class: &ClassDecl, span: Span, class_key: &str) {
        let saved_class = self.current_class.clone();
        let saved_generic_bindings = std::mem::take(&mut self.current_generic_type_bindings);
        self.current_generic_type_bindings = self.make_generic_type_bindings(&class.generic_params);
        self.current_class = Some(class_key.to_string());
        if let Some(parent) = &class.extends {
            if self.interfaces.contains_key(parent) {
                self.error(
                    format!("Class '{}' cannot extend interface '{}'", class_key, parent),
                    span.clone(),
                );
            } else if !self.classes.contains_key(parent) {
                self.error(
                    format!("Class '{}' extends unknown class '{}'", class_key, parent),
                    span.clone(),
                );
            } else if self.is_same_or_subclass_of(parent, class_key) {
                self.error(
                    format!(
                        "Inheritance cycle detected: '{}' cannot extend '{}'",
                        class_key, parent
                    ),
                    span.clone(),
                );
            } else {
                self.check_class_visibility(parent, span.clone());
            }
        }

        for field in &class.fields {
            let ty = self.resolve_type(&field.ty);
            self.check_type_visibility(&ty, span.clone());
        }

        for interface_name in &class.implements {
            if !self.interfaces.contains_key(interface_name) {
                self.error(
                    format!(
                        "Class '{}' implements unknown interface '{}'",
                        class_key, interface_name
                    ),
                    span.clone(),
                );
            }
        }

        let mut required_methods: HashMap<String, FuncSig> = HashMap::new();
        let mut visited = std::collections::HashSet::new();
        for interface_name in &class.implements {
            self.collect_interface_methods(interface_name, &mut required_methods, &mut visited);
        }
        for (method_name, required_sig) in required_methods {
            let Some((owner, actual_sig, _)) = self.lookup_class_method(class_key, &method_name)
            else {
                self.error(
                    format!(
                        "Class '{}' must implement interface method '{}'",
                        class_key, method_name
                    ),
                    span.clone(),
                );
                continue;
            };
            if !self.signatures_compatible(&required_sig, &actual_sig) {
                self.error(
                    format!(
                        "Method '{}.{}' does not match interface signature",
                        owner, method_name
                    ),
                    actual_sig.span.clone(),
                );
            }
        }

        // Check constructor
        if let Some(ctor) = &class.constructor {
            self.enter_scope();
            let saved_effects = std::mem::take(&mut self.current_effects);
            let saved_pure = self.current_is_pure;
            let saved_any = self.current_allow_any;
            let saved_class = self.current_class.clone();
            self.current_class = Some(class_key.to_string());
            self.current_effects = self
                .infer_effects_in_block(&ctor.body, Some(class_key))
                .into_iter()
                .collect();
            self.current_is_pure = false;
            self.current_allow_any = false;

            // Add 'this' binding
            self.declare_variable(
                "this",
                ResolvedType::Class(class_key.to_string()),
                true,
                span.clone(),
            );

            // Add parameters
            for param in &ctor.params {
                let ty = self.resolve_type(&param.ty);
                self.check_type_visibility(&ty, span.clone());
                self.declare_variable(&param.name, ty, param.mutable, span.clone());
            }

            self.check_block(&ctor.body);
            self.current_effects = saved_effects;
            self.current_is_pure = saved_pure;
            self.current_allow_any = saved_any;
            self.current_class = saved_class;
            self.exit_scope();
        }

        // Check destructor with inferred effects
        if let Some(dtor) = &class.destructor {
            self.enter_scope();
            let saved_effects = std::mem::take(&mut self.current_effects);
            let saved_pure = self.current_is_pure;
            let saved_any = self.current_allow_any;
            let saved_class = self.current_class.clone();
            self.current_class = Some(class_key.to_string());
            self.current_effects = self
                .infer_effects_in_block(&dtor.body, Some(class_key))
                .into_iter()
                .collect();
            self.current_is_pure = false;
            self.current_allow_any = false;
            self.declare_variable(
                "this",
                ResolvedType::Class(class_key.to_string()),
                true,
                span.clone(),
            );
            self.check_block(&dtor.body);
            self.current_effects = saved_effects;
            self.current_is_pure = saved_pure;
            self.current_allow_any = saved_any;
            self.current_class = saved_class;
            self.exit_scope();
        }

        // Check methods
        for method in &class.methods {
            let saved_generic_bindings = std::mem::take(&mut self.current_generic_type_bindings);
            let mut method_bindings = saved_generic_bindings.clone();
            method_bindings.extend(self.make_generic_type_bindings(&method.generic_params));
            self.current_generic_type_bindings = method_bindings;
            self.enter_scope();
            let saved_effects = std::mem::take(&mut self.current_effects);
            let saved_pure = self.current_is_pure;
            let saved_any = self.current_allow_any;
            let saved_class = self.current_class.clone();
            self.current_class = Some(class_key.to_string());
            if let Some(class_info) = self.classes.get(class_key) {
                if let Some(sig) = class_info.methods.get(&method.name) {
                    self.current_effects = sig.effects.clone();
                    self.current_is_pure = sig.is_pure;
                    self.current_allow_any = sig.allow_any;
                } else {
                    self.current_effects.clear();
                    self.current_is_pure = false;
                    self.current_allow_any = false;
                }
            } else {
                self.current_effects.clear();
                self.current_is_pure = false;
                self.current_allow_any = false;
            }

            // Add 'this' binding
            self.declare_variable(
                "this",
                ResolvedType::Class(class_key.to_string()),
                false,
                span.clone(),
            );

            // Add parameters
            for param in &method.params {
                let ty = self.resolve_type(&param.ty);
                self.check_type_visibility(&ty, span.clone());
                self.declare_variable(&param.name, ty, param.mutable, span.clone());
            }

            let return_type = self.resolve_type(&method.return_type);
            self.check_type_visibility(&return_type, span.clone());
            self.current_return_type = Some(return_type);

            self.check_block(&method.body);

            self.current_return_type = None;
            self.current_effects = saved_effects;
            self.current_is_pure = saved_pure;
            self.current_allow_any = saved_any;
            self.current_class = saved_class;
            self.exit_scope();
            self.current_generic_type_bindings = saved_generic_bindings;
        }
        self.current_class = saved_class;
        self.current_generic_type_bindings = saved_generic_bindings;
    }

    /// Check a block of statements
    fn check_block(&mut self, block: &Block) {
        for stmt in block {
            self.check_stmt(&stmt.node, stmt.span.clone());
        }
    }

    /// Check a statement
    fn check_stmt(&mut self, stmt: &Stmt, span: Span) {
        match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
            } => {
                let declared_type = self.resolve_type(ty);
                self.check_type_visibility(&declared_type, span.clone());
                let value_type = self.check_expr(&value.node, value.span.clone());

                // Check type compatibility
                if !self.types_compatible(&declared_type, &value_type) {
                    self.error(
                        format!(
                            "Type mismatch: cannot assign {} to variable of type {}",
                            value_type, declared_type
                        ),
                        value.span.clone(),
                    );
                }

                self.declare_variable(name, declared_type, *mutable, span);
            }

            Stmt::Assign { target, value } => {
                let target_type = self.check_expr(&target.node, target.span.clone());
                let value_type = self.check_expr(&value.node, value.span.clone());

                // Check if target is assignable (mutable)
                self.check_assignment_target_mutability(&target.node, target.span.clone());

                if !self.types_compatible(&target_type, &value_type) {
                    self.error(
                        format!(
                            "Type mismatch in assignment: expected {}, found {}",
                            target_type, value_type
                        ),
                        value.span.clone(),
                    );
                }
            }

            Stmt::Expr(expr) => {
                self.check_expr(&expr.node, expr.span.clone());
            }

            Stmt::Return(expr) => {
                let return_type = expr
                    .as_ref()
                    .map(|e| self.check_expr(&e.node, e.span.clone()))
                    .unwrap_or(ResolvedType::None);

                if let Some(expected) = &self.current_return_type {
                    if !self.types_compatible(expected, &return_type) {
                        self.error(
                            format!(
                                "Return type mismatch: expected {}, found {}",
                                expected, return_type
                            ),
                            span,
                        );
                    }
                }
            }

            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                let cond_type = self.check_expr(&condition.node, condition.span.clone());
                if !matches!(cond_type, ResolvedType::Boolean) {
                    self.error(
                        format!("Condition must be Boolean, found {}", cond_type),
                        condition.span.clone(),
                    );
                }

                self.enter_scope();
                self.check_block(then_block);
                self.exit_scope();

                if let Some(else_blk) = else_block {
                    self.enter_scope();
                    self.check_block(else_blk);
                    self.exit_scope();
                }
            }

            Stmt::While { condition, body } => {
                let cond_type = self.check_expr(&condition.node, condition.span.clone());
                if !matches!(cond_type, ResolvedType::Boolean) {
                    self.error(
                        format!("Condition must be Boolean, found {}", cond_type),
                        condition.span.clone(),
                    );
                }

                self.enter_scope();
                self.check_block(body);
                self.exit_scope();
            }

            Stmt::For {
                var,
                var_type,
                iterable,
                body,
            } => {
                let iter_type = self.check_expr(&iterable.node, iterable.span.clone());

                // Determine element type
                let elem_type = match &iter_type {
                    ResolvedType::List(inner) => (**inner).clone(),
                    ResolvedType::Range(inner) => (**inner).clone(),
                    ResolvedType::String => ResolvedType::Char,
                    _ => {
                        self.error(
                            format!("Cannot iterate over {}", iter_type),
                            iterable.span.clone(),
                        );
                        ResolvedType::Unknown
                    }
                };

                // Check declared type if provided
                if let Some(declared) = var_type {
                    let declared_type = self.resolve_type(declared);
                    if !self.types_compatible(&declared_type, &elem_type) {
                        self.error(
                            format!(
                                "Loop variable type mismatch: declared {}, but iterating over {}",
                                declared_type, iter_type
                            ),
                            iterable.span.clone(),
                        );
                    }
                }

                self.enter_scope();
                self.declare_variable(var, elem_type, false, span);
                self.check_block(body);
                self.exit_scope();
            }

            Stmt::Match { expr, arms } => {
                let match_type = self.check_expr(&expr.node, expr.span.clone());

                for arm in arms {
                    self.enter_scope();
                    self.check_pattern(&arm.pattern, &match_type, span.clone());
                    self.check_block(&arm.body);
                    self.exit_scope();
                }

                if !self.match_expression_exhaustive(&match_type, arms) {
                    self.error(
                        format!("Non-exhaustive match statement for type {}", match_type),
                        span,
                    );
                }
            }

            Stmt::Break | Stmt::Continue => {}
        }
    }

    fn check_assignment_target_mutability(&mut self, target: &Expr, span: Span) {
        match target {
            Expr::Ident(name) => {
                if let Some(var) = self.lookup_variable(name) {
                    if !var.mutable {
                        self.error_with_hint(
                            format!("Cannot assign to immutable variable '{}'", name),
                            span,
                            "Consider declaring with 'mut'".to_string(),
                        );
                    }
                }
            }
            Expr::Field { object, .. } | Expr::Index { object, .. } => {
                self.check_assignment_target_mutability(&object.node, span);
            }
            Expr::This | Expr::Deref(_) => {}
            _ => {}
        }
    }

    /// Check a pattern in match
    fn check_pattern(&mut self, pattern: &Pattern, expected_type: &ResolvedType, span: Span) {
        fn pattern_variant_leaf(name: &str) -> &str {
            name.rsplit('.').next().unwrap_or(name)
        }

        match pattern {
            Pattern::Wildcard => {}
            Pattern::Ident(name) => {
                self.declare_variable(name, expected_type.clone(), false, span);
            }
            Pattern::Literal(lit) => {
                let lit_type = self.literal_type(lit);
                if !self.types_compatible(expected_type, &lit_type) {
                    self.error(
                        format!(
                            "Pattern type mismatch: expected {}, found {}",
                            expected_type, lit_type
                        ),
                        span,
                    );
                }
            }
            Pattern::Variant(name, bindings) => {
                let imported_variant = (!name.contains('.'))
                    .then(|| self.resolve_import_alias_variant(name))
                    .flatten();
                let variant_name = imported_variant.as_ref().map_or_else(
                    || pattern_variant_leaf(name).to_string(),
                    |(_, variant)| variant.clone(),
                );
                match expected_type {
                    ResolvedType::Option(inner) => {
                        if variant_name == "Some" && bindings.len() == 1 {
                            self.declare_variable(&bindings[0], (**inner).clone(), false, span);
                        } else if variant_name == "None" && bindings.is_empty() {
                            // OK
                        } else {
                            self.error(format!("Invalid Option pattern: {}", name), span);
                        }
                    }
                    ResolvedType::Result(ok, err) => {
                        if variant_name == "Ok" && bindings.len() == 1 {
                            self.declare_variable(&bindings[0], (**ok).clone(), false, span);
                        } else if variant_name == "Error" && bindings.len() == 1 {
                            self.declare_variable(&bindings[0], (**err).clone(), false, span);
                        } else {
                            self.error(format!("Invalid Result pattern: {}", name), span);
                        }
                    }
                    ResolvedType::Class(enum_name) => {
                        if imported_variant
                            .as_ref()
                            .is_some_and(|(owner_enum, _)| owner_enum != enum_name)
                        {
                            self.error(
                                format!("Cannot match variant {} on type {}", name, expected_type),
                                span,
                            );
                            return;
                        }
                        let resolved_enum_name = self
                            .resolve_enum_name(enum_name)
                            .unwrap_or_else(|| enum_name.clone());
                        if let Some(enum_info) = self.enums.get(&resolved_enum_name).cloned() {
                            if let Some(field_tys) = enum_info.variants.get(&variant_name) {
                                if field_tys.len() != bindings.len() {
                                    self.error(
                                        format!(
                                            "Pattern '{}' expects {} binding(s), got {}",
                                            variant_name,
                                            field_tys.len(),
                                            bindings.len()
                                        ),
                                        span,
                                    );
                                } else {
                                    for (binding, ty) in bindings.iter().zip(field_tys.iter()) {
                                        self.declare_variable(
                                            binding,
                                            ty.clone(),
                                            false,
                                            span.clone(),
                                        );
                                    }
                                }
                            } else {
                                self.error(
                                    format!("Unknown variant '{}' for enum '{}'", name, enum_name),
                                    span,
                                );
                            }
                        } else {
                            self.error(
                                format!("Cannot match variant {} on type {}", name, expected_type),
                                span,
                            );
                        }
                    }
                    _ => {
                        self.error(
                            format!("Cannot match variant {} on type {}", name, expected_type),
                            span,
                        );
                    }
                }
            }
        }
    }

    fn infer_block_expression_type(&mut self, block: &Block) -> ResolvedType {
        let mut ty = ResolvedType::None;
        for stmt in block {
            if let Stmt::Expr(expr) = &stmt.node {
                ty = self.check_expr(&expr.node, expr.span.clone());
            }
        }
        ty
    }

    fn match_expression_exhaustive(&self, match_type: &ResolvedType, arms: &[MatchArm]) -> bool {
        let has_catch_all = arms
            .iter()
            .any(|arm| matches!(arm.pattern, Pattern::Wildcard | Pattern::Ident(_)));
        if has_catch_all {
            return true;
        }

        match match_type {
            ResolvedType::Boolean => {
                let has_true = arms
                    .iter()
                    .any(|arm| matches!(arm.pattern, Pattern::Literal(Literal::Boolean(true))));
                let has_false = arms
                    .iter()
                    .any(|arm| matches!(arm.pattern, Pattern::Literal(Literal::Boolean(false))));
                has_true && has_false
            }
            ResolvedType::Option(_) => {
                let has_some = arms.iter().any(|arm| {
                    matches!(&arm.pattern, Pattern::Variant(n, _) if n.rsplit('.').next().is_some_and(|leaf| leaf == "Some"))
                });
                let has_none = arms.iter().any(|arm| {
                    matches!(&arm.pattern, Pattern::Variant(n, _) if n.rsplit('.').next().is_some_and(|leaf| leaf == "None"))
                });
                has_some && has_none
            }
            ResolvedType::Result(_, _) => {
                let has_ok = arms.iter().any(|arm| {
                    matches!(&arm.pattern, Pattern::Variant(n, _) if n.rsplit('.').next().is_some_and(|leaf| leaf == "Ok"))
                });
                let has_err = arms.iter().any(|arm| {
                    matches!(&arm.pattern, Pattern::Variant(n, _) if n.rsplit('.').next().is_some_and(|leaf| leaf == "Error"))
                });
                has_ok && has_err
            }
            ResolvedType::Class(enum_name) => self
                .resolve_enum_name(enum_name)
                .and_then(|resolved_enum_name| self.enums.get(&resolved_enum_name))
                .is_some_and(|enum_info| {
                    enum_info.variants.keys().all(|variant_name| {
                        arms.iter().any(|arm| {
                            matches!(&arm.pattern, Pattern::Variant(name, _)
                                if (!name.contains('.')
                                    && self
                                        .resolve_import_alias_variant(name)
                                        .is_some_and(|(owner_enum, imported_variant)| {
                                            owner_enum == *enum_name && imported_variant == *variant_name
                                        }))
                                    || name
                                        .rsplit('.')
                                        .next()
                                        .is_some_and(|leaf| leaf == variant_name.as_str())
                            )
                        })
                    })
                }),
            _ => false,
        }
    }

    fn function_value_type_or_error(&mut self, function_name: &str, span: Span) -> ResolvedType {
        let sig = &self.functions[function_name];
        if sig.is_extern {
            self.error(
                format!(
                    "extern function '{}' cannot be used as a first-class value",
                    function_name
                ),
                span,
            );
            ResolvedType::Unknown
        } else {
            ResolvedType::Function(
                sig.params.iter().map(|(_, t)| t.clone()).collect(),
                Box::new(sig.return_type.clone()),
            )
        }
    }

    /// Check an expression and return its type
    fn check_expr(&mut self, expr: &Expr, span: Span) -> ResolvedType {
        match expr {
            Expr::Literal(lit) => self.literal_type(lit),

            Expr::Ident(name) => {
                if let Some(var) = self.lookup_variable(name) {
                    var.ty.clone()
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
                                    variant_name, enum_name
                                ),
                                span,
                            );
                            ResolvedType::Unknown
                        }
                    } else {
                        self.error(format!("Undefined variable: {}", name), span);
                        ResolvedType::Unknown
                    }
                } else if let Some(function_name) = self.resolve_function_value_name(name) {
                    let function_name = function_name.to_owned();
                    self.function_value_type_or_error(&function_name, span)
                } else {
                    self.error(format!("Undefined variable: {}", name), span);
                    ResolvedType::Unknown
                }
            }

            Expr::Binary { op, left, right } => {
                let left_type = self.check_expr(&left.node, left.span.clone());
                let right_type = self.check_expr(&right.node, right.span.clone());

                self.check_binary_op(*op, &left_type, &right_type, span)
            }

            Expr::Unary { op, expr: inner } => {
                let inner_type = self.check_expr(&inner.node, inner.span.clone());

                match op {
                    UnaryOp::Neg => {
                        if !inner_type.is_numeric() {
                            self.error(
                                format!("Cannot negate non-numeric type {}", inner_type),
                                span,
                            );
                        }
                        inner_type
                    }
                    UnaryOp::Not => {
                        if !matches!(inner_type, ResolvedType::Boolean) {
                            self.error(
                                format!("Cannot apply '!' to non-boolean type {}", inner_type),
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
            } => self.check_call(&callee.node, args, type_args, span),

            Expr::Field { object, field } => {
                if let Expr::Ident(owner_name) = &object.node {
                    let resolved_owner = self
                        .resolve_import_alias_symbol(owner_name)
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
                if let Some(path_parts) = Self::flatten_field_chain(expr) {
                    if path_parts.len() >= 2 {
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
                        }

                        let mangled = path_parts.join("__");
                        let resolved = self
                            .resolve_function_value_name(&mangled)
                            .unwrap_or(&mangled);
                        if self.functions.contains_key(resolved) {
                            let resolved = resolved.to_owned();
                            return self.function_value_type_or_error(&resolved, span.clone());
                        }
                    }
                }
                let obj_type = self.check_expr(&object.node, object.span.clone());
                self.check_field_access(&obj_type, field, span)
            }

            Expr::Index { object, index } => {
                let obj_type = self.check_expr(&object.node, object.span.clone());
                let idx_type = self.check_expr(&index.node, index.span.clone());

                match &obj_type {
                    ResolvedType::List(inner) => {
                        if !matches!(idx_type, ResolvedType::Integer) {
                            self.error(
                                format!("Index must be Integer, found {}", idx_type),
                                index.span.clone(),
                            );
                        }
                        (**inner).clone()
                    }
                    ResolvedType::String => {
                        if !matches!(idx_type, ResolvedType::Integer) {
                            self.error(
                                format!("Index must be Integer, found {}", idx_type),
                                index.span.clone(),
                            );
                        }
                        ResolvedType::Char
                    }
                    ResolvedType::Map(k, v) => {
                        if !self.types_compatible(k, &idx_type) {
                            self.error(
                                format!(
                                    "Map index type mismatch: expected {}, got {}",
                                    k, idx_type
                                ),
                                index.span.clone(),
                            );
                        }
                        (**v).clone()
                    }
                    _ => {
                        self.error(format!("Cannot index type {}", obj_type), span);
                        ResolvedType::Unknown
                    }
                }
            }

            Expr::Construct { ty, args } => {
                let scoped_ty = self
                    .module_scoped_type_name(ty)
                    .unwrap_or_else(|| ty.clone());

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
                                    let actual = self.check_expr(&arg.node, arg.span.clone());
                                    if !self.types_compatible(expected_ty, &actual) {
                                        self.error(
                                            format!(
                                                "Enum variant argument type mismatch: expected {}, got {}",
                                                expected_ty, actual
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
                    let resolved = self.parse_type_string(&scoped_ty);
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

                if self.interfaces.contains_key(&scoped_ty)
                    || self.interfaces.contains_key(&class_name)
                {
                    self.error(
                        format!("Cannot construct interface type '{}'", scoped_ty),
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
                                let arg_type = self.check_expr(&arg.node, arg.span.clone());
                                if !self.types_compatible(expected, &arg_type) {
                                    self.error(
                                        format!(
                                            "Constructor argument type mismatch: expected {}, got {}",
                                            expected, arg_type
                                        ),
                                        arg.span.clone(),
                                    );
                                }
                            }
                        }
                    }
                    self.parse_type_string(&scoped_ty)
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
                    self.error(format!("Unknown type: {}", scoped_ty), span);
                    ResolvedType::Unknown
                }
            }

            Expr::Lambda { params, body } => {
                self.enter_scope();

                let param_types: Vec<ResolvedType> = params
                    .iter()
                    .map(|p| {
                        let ty = self.resolve_type(&p.ty);
                        self.declare_variable(&p.name, ty.clone(), p.mutable, span.clone());
                        ty
                    })
                    .collect();

                let return_type = self.check_expr(&body.node, body.span.clone());

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
                        self.check_expr(&e.node, e.span.clone());
                    }
                }
                ResolvedType::String
            }

            Expr::Try(inner) => {
                let inner_type = self.check_expr(&inner.node, inner.span.clone());
                match inner_type {
                    ResolvedType::Option(inner) => *inner,
                    ResolvedType::Result(ok, _) => *ok,
                    _ => {
                        self.error(
                            format!(
                                "'?' operator can only be used on Option or Result, got {}",
                                inner_type
                            ),
                            span,
                        );
                        ResolvedType::Unknown
                    }
                }
            }

            Expr::Borrow(inner) => {
                let inner_type = self.check_expr(&inner.node, inner.span.clone());
                ResolvedType::Ref(Box::new(inner_type))
            }

            Expr::MutBorrow(inner) => {
                let inner_type = self.check_expr(&inner.node, inner.span.clone());

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
                    ResolvedType::Ref(inner) | ResolvedType::MutRef(inner) => *inner,
                    _ => {
                        self.error(
                            format!("Cannot dereference non-reference type {}", inner_type),
                            span,
                        );
                        ResolvedType::Unknown
                    }
                }
            }

            Expr::Match { expr, arms } => {
                let match_type = self.check_expr(&expr.node, expr.span.clone());
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
                        if !self.types_compatible(expected, &arm_type) {
                            self.error(
                                format!(
                                    "Match expression arm type mismatch: expected {}, got {}",
                                    expected, arm_type
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
                        format!("Non-exhaustive match expression for type {}", match_type),
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
                    _ => {
                        self.error(
                            format!("'await' can only be used on Task types, got {}", inner_type),
                            span,
                        );
                        ResolvedType::Unknown
                    }
                }
            }

            Expr::AsyncBlock(body) => {
                let captured_outer_scopes = self.scopes.clone();
                self.enter_scope();
                let mut return_type = ResolvedType::None;

                // For async blocks, we need to track return types specifically for this block
                let saved_return_type = self.current_return_type.clone();
                // Start with None, or if we want to support inference, a fresh type var
                self.current_return_type = Some(ResolvedType::None);

                for stmt in body {
                    if let Stmt::Return(Some(expr)) = &stmt.node {
                        let expr_type = self.check_expr(&expr.node, expr.span.clone());
                        if matches!(self.current_return_type, Some(ResolvedType::None)) {
                            self.current_return_type = Some(expr_type.clone());
                            return_type = expr_type;
                        } else if let Some(expected) = &self.current_return_type {
                            if !self.types_compatible(expected, &expr_type) {
                                self.error(
                                    format!(
                                        "Mismatching return types in async block: {} vs {}",
                                        expected, expr_type
                                    ),
                                    expr.span.clone(),
                                );
                            }
                        }
                    }
                    self.check_stmt(&stmt.node, stmt.span.clone());
                }

                for scope in &captured_outer_scopes {
                    for (name, var) in &scope.variables {
                        if Self::type_contains_borrowed_reference(&var.ty)
                            && body
                                .iter()
                                .any(|stmt| Self::stmt_mentions_ident(&stmt.node, name))
                        {
                            self.error(
                                format!(
                                    "Async block cannot capture '{}' because its type contains borrowed references: {}",
                                    name, var.ty
                                ),
                                span.clone(),
                            );
                        }
                    }
                }

                self.current_return_type = saved_return_type;
                self.exit_scope();
                self.check_async_result_type(&return_type, "Async block", span.clone());
                ResolvedType::Task(Box::new(return_type))
            }

            Expr::Require { condition, message } => {
                let cond_type = self.check_expr(&condition.node, condition.span.clone());
                if !matches!(cond_type, ResolvedType::Boolean) {
                    self.error(
                        format!("require() condition must be Boolean, got {}", cond_type),
                        condition.span.clone(),
                    );
                }
                if let Some(msg) = message {
                    let msg_type = self.check_expr(&msg.node, msg.span.clone());
                    if !matches!(msg_type, ResolvedType::String) {
                        self.error(
                            format!("require() message must be String, got {}", msg_type),
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
                    let start_type = self.check_expr(&s.node, s.span.clone());
                    if !matches!(start_type, ResolvedType::Integer) {
                        self.error(
                            format!("Range start must be Integer, got {}", start_type),
                            s.span.clone(),
                        );
                    }
                }
                if let Some(e) = end {
                    let end_type = self.check_expr(&e.node, e.span.clone());
                    if !matches!(end_type, ResolvedType::Integer) {
                        self.error(
                            format!("Range end must be Integer, got {}", end_type),
                            e.span.clone(),
                        );
                    }
                }
                ResolvedType::Range(Box::new(ResolvedType::Integer))
            }

            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_type = self.check_expr(&condition.node, condition.span.clone());
                if !matches!(cond_type, ResolvedType::Boolean) {
                    self.error(
                        format!("If condition must be Boolean, got {}", cond_type),
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

                    if !self.types_compatible(&then_type, &else_type) {
                        self.error(
                            format!(
                                "If expression branch type mismatch: then is {}, else is {}",
                                then_type, else_type
                            ),
                            condition.span.clone(),
                        );
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

    fn check_builtin_constructor_args(
        &mut self,
        ty_name: &str,
        resolved: &ResolvedType,
        args: &[Spanned<Expr>],
        span: Span,
    ) {
        let arg_types: Vec<ResolvedType> = args
            .iter()
            .map(|arg| self.check_expr(&arg.node, arg.span.clone()))
            .collect();

        // List<T>(N) supports optional integer preallocation size; otherwise List<T>().
        if ty_name == "List"
            || ty_name.starts_with("List<")
            || matches!(resolved, ResolvedType::List(_))
        {
            match arg_types.as_slice() {
                [] => {}
                [ResolvedType::Integer] => {}
                [other] => {
                    self.error(
                        format!(
                            "Constructor {} expects optional Integer capacity, got {}",
                            ty_name, other
                        ),
                        span,
                    );
                }
                _ => {
                    self.error(
                        format!(
                            "Constructor {} expects 0 or 1 arguments, got {}",
                            ty_name,
                            args.len()
                        ),
                        span,
                    );
                }
            }
            return;
        }

        // Map/Set/Option/Result constructors are default constructors with no value args.
        let requires_zero_args = ty_name == "Map"
            || ty_name.starts_with("Map<")
            || ty_name == "Set"
            || ty_name.starts_with("Set<")
            || ty_name == "Option"
            || ty_name.starts_with("Option<")
            || ty_name == "Result"
            || ty_name.starts_with("Result<")
            || matches!(
                resolved,
                ResolvedType::Map(_, _)
                    | ResolvedType::Set(_)
                    | ResolvedType::Option(_)
                    | ResolvedType::Result(_, _)
            );
        if requires_zero_args && !arg_types.is_empty() {
            self.error(
                format!(
                    "Constructor {} expects 0 arguments, got {}",
                    ty_name,
                    args.len()
                ),
                span,
            );
        }
    }

    fn validate_resolved_type_exists(&mut self, ty: &ResolvedType, span: Span) {
        match ty {
            ResolvedType::Class(name) => {
                let base_name = self.class_base_name(name);
                if !self.classes.contains_key(base_name)
                    && !self.interfaces.contains_key(base_name)
                    && !self.enums.contains_key(base_name)
                {
                    self.error(format!("Unknown type: {}", name), span);
                }
            }
            ResolvedType::Option(inner)
            | ResolvedType::List(inner)
            | ResolvedType::Set(inner)
            | ResolvedType::Ref(inner)
            | ResolvedType::MutRef(inner)
            | ResolvedType::Box(inner)
            | ResolvedType::Rc(inner)
            | ResolvedType::Arc(inner)
            | ResolvedType::Ptr(inner)
            | ResolvedType::Task(inner)
            | ResolvedType::Range(inner) => self.validate_resolved_type_exists(inner, span),
            ResolvedType::Result(ok, err) | ResolvedType::Map(ok, err) => {
                self.validate_resolved_type_exists(ok, span.clone());
                self.validate_resolved_type_exists(err, span);
            }
            ResolvedType::Function(params, ret) => {
                for p in params {
                    self.validate_resolved_type_exists(p, span.clone());
                }
                self.validate_resolved_type_exists(ret, span);
            }
            ResolvedType::Integer
            | ResolvedType::Float
            | ResolvedType::Boolean
            | ResolvedType::String
            | ResolvedType::Char
            | ResolvedType::None
            | ResolvedType::TypeVar(_)
            | ResolvedType::Unknown => {}
        }
    }

    fn substitute_type_vars(
        ty: &ResolvedType,
        substitutions: &HashMap<usize, ResolvedType>,
    ) -> ResolvedType {
        match ty {
            ResolvedType::TypeVar(id) => {
                substitutions.get(id).cloned().unwrap_or_else(|| ty.clone())
            }
            ResolvedType::Option(inner) => {
                ResolvedType::Option(Box::new(Self::substitute_type_vars(inner, substitutions)))
            }
            ResolvedType::Result(ok, err) => ResolvedType::Result(
                Box::new(Self::substitute_type_vars(ok, substitutions)),
                Box::new(Self::substitute_type_vars(err, substitutions)),
            ),
            ResolvedType::List(inner) => {
                ResolvedType::List(Box::new(Self::substitute_type_vars(inner, substitutions)))
            }
            ResolvedType::Map(k, v) => ResolvedType::Map(
                Box::new(Self::substitute_type_vars(k, substitutions)),
                Box::new(Self::substitute_type_vars(v, substitutions)),
            ),
            ResolvedType::Set(inner) => {
                ResolvedType::Set(Box::new(Self::substitute_type_vars(inner, substitutions)))
            }
            ResolvedType::Ref(inner) => {
                ResolvedType::Ref(Box::new(Self::substitute_type_vars(inner, substitutions)))
            }
            ResolvedType::MutRef(inner) => {
                ResolvedType::MutRef(Box::new(Self::substitute_type_vars(inner, substitutions)))
            }
            ResolvedType::Box(inner) => {
                ResolvedType::Box(Box::new(Self::substitute_type_vars(inner, substitutions)))
            }
            ResolvedType::Rc(inner) => {
                ResolvedType::Rc(Box::new(Self::substitute_type_vars(inner, substitutions)))
            }
            ResolvedType::Arc(inner) => {
                ResolvedType::Arc(Box::new(Self::substitute_type_vars(inner, substitutions)))
            }
            ResolvedType::Ptr(inner) => {
                ResolvedType::Ptr(Box::new(Self::substitute_type_vars(inner, substitutions)))
            }
            ResolvedType::Task(inner) => {
                ResolvedType::Task(Box::new(Self::substitute_type_vars(inner, substitutions)))
            }
            ResolvedType::Range(inner) => {
                ResolvedType::Range(Box::new(Self::substitute_type_vars(inner, substitutions)))
            }
            ResolvedType::Function(params, ret) => ResolvedType::Function(
                params
                    .iter()
                    .map(|p| Self::substitute_type_vars(p, substitutions))
                    .collect(),
                Box::new(Self::substitute_type_vars(ret, substitutions)),
            ),
            ResolvedType::Class(name) => {
                Self::substitute_type_vars_in_class_name(name, substitutions)
            }
            _ => ty.clone(),
        }
    }

    fn substitute_type_vars_in_class_name(
        name: &str,
        substitutions: &HashMap<usize, ResolvedType>,
    ) -> ResolvedType {
        if let Some(type_var_id) = name
            .strip_prefix("?T")
            .and_then(|id| id.parse::<usize>().ok())
        {
            return substitutions
                .get(&type_var_id)
                .cloned()
                .unwrap_or_else(|| ResolvedType::Class(name.to_string()));
        }

        if let Some(open_bracket) = name.find('<') {
            if name.ends_with('>') {
                let base = &name[..open_bracket];
                let inner = &name[open_bracket + 1..name.len() - 1];
                let resolved_args = Self::split_generic_args_static(inner)
                    .into_iter()
                    .map(|arg| {
                        Self::substitute_type_vars_in_class_name(&arg, substitutions).to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                return ResolvedType::Class(format!("{}<{}>", base, resolved_args));
            }
        }

        ResolvedType::Class(name.to_string())
    }

    fn instantiate_signature_for_call(
        &mut self,
        name: &str,
        sig: &FuncSig,
        type_args: &[Type],
        span: Span,
    ) -> (Vec<(String, ResolvedType)>, ResolvedType) {
        if type_args.is_empty() {
            return (sig.params.clone(), sig.return_type.clone());
        }

        if sig.generic_type_vars.is_empty() {
            self.error(
                format!(
                    "Function '{}' is not generic but called with explicit type arguments",
                    name
                ),
                span,
            );
            return (sig.params.clone(), sig.return_type.clone());
        }

        if type_args.len() != sig.generic_type_vars.len() {
            self.error(
                format!(
                    "Function '{}' expects {} type arguments, got {}",
                    name,
                    sig.generic_type_vars.len(),
                    type_args.len()
                ),
                span,
            );
            return (sig.params.clone(), sig.return_type.clone());
        }

        let mut substitutions: HashMap<usize, ResolvedType> = HashMap::new();
        for (type_var_id, arg) in sig.generic_type_vars.iter().zip(type_args.iter()) {
            let resolved = self.resolve_type(arg);
            self.validate_resolved_type_exists(&resolved, span.clone());
            substitutions.insert(*type_var_id, resolved);
        }

        let params = sig
            .params
            .iter()
            .map(|(name, ty)| (name.clone(), Self::substitute_type_vars(ty, &substitutions)))
            .collect::<Vec<_>>();
        let return_type = Self::substitute_type_vars(&sig.return_type, &substitutions);
        (params, return_type)
    }

    /// Check a function/method call
    fn check_call(
        &mut self,
        callee: &Expr,
        args: &[Spanned<Expr>],
        type_args: &[Type],
        span: Span,
    ) -> ResolvedType {
        let canonical_ident_call = match callee {
            Expr::Ident(name) => self.resolve_import_alias_symbol(name),
            _ => None,
        };
        let aliased_variant_call = match callee {
            Expr::Ident(name) => self.resolve_import_alias_variant(name),
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
                                let actual = self.check_expr(&arg.node, arg.span.clone());
                                if !self.types_compatible(expected_ty, &actual) {
                                    self.error(
                                        format!(
                                            "Enum variant argument type mismatch: expected {}, got {}",
                                            expected_ty, actual
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
            if let Some(path_parts) = Self::flatten_field_chain(callee) {
                if path_parts.len() >= 2 {
                    if let Some(candidate) =
                        self.resolve_import_alias_module_candidate(&path_parts[0], &path_parts[1..])
                    {
                        let resolved = self
                            .resolve_function_value_name(&candidate)
                            .unwrap_or(&candidate)
                            .to_string();
                        if let Some(sig) = self.functions.get(&resolved).cloned() {
                            self.enforce_call_effects(&sig, span.clone(), &resolved);
                            let (inst_params, inst_return_type) = self
                                .instantiate_signature_for_call(
                                    &resolved,
                                    &sig,
                                    type_args,
                                    span.clone(),
                                );
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
                                    let arg_type = self.check_expr(&arg.node, arg.span.clone());
                                    if !self.types_compatible(param_type, &arg_type) {
                                        self.error(
                                            format!(
                                                "Argument type mismatch: expected {}, got {}",
                                                param_type, arg_type
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
                    }

                    let mangled = path_parts.join("__");
                    if let Some(sig) = self.functions.get(&mangled).cloned() {
                        self.enforce_call_effects(&sig, span.clone(), &mangled);
                        let (inst_params, inst_return_type) = self.instantiate_signature_for_call(
                            &mangled,
                            &sig,
                            type_args,
                            span.clone(),
                        );
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
                                let arg_type = self.check_expr(&arg.node, arg.span.clone());
                                if !self.types_compatible(param_type, &arg_type) {
                                    self.error(
                                        format!(
                                            "Argument type mismatch: expected {}, got {}",
                                            param_type, arg_type
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
                                    self.check_expr(&arg.node, arg.span.clone())
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
                                    self.check_expr(&arg.node, arg.span.clone())
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
                                    self.check_expr(&arg.node, arg.span.clone())
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
                                let actual = self.check_expr(&arg.node, arg.span.clone());
                                if !self.types_compatible(expected_ty, &actual) {
                                    self.error(
                                        format!(
                                            "Enum variant argument type mismatch: expected {}, got {}",
                                            expected_ty, actual
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
                    let (inst_params, inst_return_type) = self.instantiate_signature_for_call(
                        &mangled,
                        &sig,
                        type_args,
                        span.clone(),
                    );
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
                            let arg_type = self.check_expr(&arg.node, arg.span.clone());
                            if !self.types_compatible(param_type, &arg_type) {
                                self.error(
                                    format!(
                                        "Argument type mismatch: expected {}, got {}",
                                        param_type, arg_type
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

            let obj_type = self.check_expr(&object.node, object.span.clone());
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
                                let arg_type = self.check_expr(&arg.node, arg.span.clone());
                                if !self.types_compatible(param_type, &arg_type) {
                                    self.error(
                                        format!(
                                            "Argument type mismatch: expected {}, got {}",
                                            param_type, arg_type
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
                let (inst_params, inst_return_type) = self.instantiate_signature_for_call(
                    resolved_name,
                    &sig,
                    type_args,
                    span.clone(),
                );
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
                        let arg_type = self.check_expr(&arg.node, arg.span.clone());
                        if !self.types_compatible(param_type, &arg_type) {
                            self.error(
                                format!(
                                    "Argument type mismatch: expected {}, got {}",
                                    param_type, arg_type
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
                    let arg_type = self.check_expr(&arg.node, arg.span.clone());
                    if !self.types_compatible(param_type, &arg_type) {
                        self.error(
                            format!(
                                "Argument type mismatch: expected {}, got {}",
                                param_type, arg_type
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
                format!("Cannot call non-function type {}", callee_type),
                span,
            );
        }
        ResolvedType::Unknown
    }

    fn check_variadic_ffi_tail_args(
        &mut self,
        callee: &str,
        args: &[Spanned<Expr>],
        fixed_count: usize,
    ) {
        for arg in args.iter().skip(fixed_count) {
            let t = self.check_expr(&arg.node, arg.span.clone());
            if !self.is_ffi_safe_type(&t) {
                self.error(
                    format!(
                        "Variadic extern call '{}' received non-FFI-safe variadic argument type {}",
                        callee, t
                    ),
                    arg.span.clone(),
                );
            }
        }
    }

    /// Check built-in function calls
    fn check_builtin_call(
        &mut self,
        name: &str,
        args: &[Spanned<Expr>],
        span: Span,
    ) -> Option<ResolvedType> {
        match name {
            "println" | "print" => {
                for arg in args {
                    self.check_expr(&arg.node, arg.span.clone());
                }
                Some(ResolvedType::None)
            }
            "Math__abs" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !t.is_numeric() {
                        self.error(format!("Math.abs() requires numeric type, got {}", t), span);
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
                    let t1 = self.check_expr(&args[0].node, args[0].span.clone());
                    let t2 = self.check_expr(&args[1].node, args[1].span.clone());
                    if !self.types_compatible(&t1, &t2) {
                        self.error(
                            format!(
                                "{}() arguments must have same type: {} vs {}",
                                func_name, t1, t2
                            ),
                            span,
                        );
                    }
                    Some(t1)
                } else {
                    Some(ResolvedType::Unknown)
                }
            }
            "Math__sqrt" | "Math__sin" | "Math__cos" | "Math__tan" | "Math__floor"
            | "Math__ceil" | "Math__round" | "Math__log" | "Math__log10" | "Math__exp" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !t.is_numeric() {
                        self.error(
                            format!(
                                "{}() requires numeric type, got {}",
                                name.replace("__", "."),
                                t
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
                    let t1 = self.check_expr(&args[0].node, args[0].span.clone());
                    let t2 = self.check_expr(&args[1].node, args[1].span.clone());
                    if !t1.is_numeric() || !t2.is_numeric() {
                        self.error("Math.pow() requires numeric types".to_string(), span);
                    }
                }
                Some(ResolvedType::Float)
            }
            "to_float" => {
                self.check_arg_count(name, args, 1, span);
                Some(ResolvedType::Float)
            }
            "to_int" => {
                self.check_arg_count(name, args, 1, span);
                Some(ResolvedType::Integer)
            }
            "to_string" => {
                self.check_arg_count(name, args, 1, span);
                Some(ResolvedType::String)
            }
            "Str__len" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::String) {
                        self.error(format!("Str.len() requires String, got {}", t), span);
                    }
                }
                Some(ResolvedType::Integer)
            }
            "Str__compare" => {
                self.check_arg_count(name, args, 2, span.clone());
                if args.len() >= 2 {
                    for arg in &args[..2] {
                        let t = self.check_expr(&arg.node, arg.span.clone());
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
                        let t = self.check_expr(&arg.node, arg.span.clone());
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
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::String) {
                        self.error("Str.upper() requires String".to_string(), span.clone());
                    }
                }
                Some(ResolvedType::String)
            }
            "Str__lower" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::String) {
                        self.error("Str.lower() requires String".to_string(), span.clone());
                    }
                }
                Some(ResolvedType::String)
            }
            "Str__trim" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::String) {
                        self.error("Str.trim() requires String".to_string(), span.clone());
                    }
                }
                Some(ResolvedType::String)
            }
            "Str__contains" => {
                self.check_arg_count(name, args, 2, span.clone());
                if args.len() >= 2 {
                    let t1 = self.check_expr(&args[0].node, args[0].span.clone());
                    let t2 = self.check_expr(&args[1].node, args[1].span.clone());
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
                    let t1 = self.check_expr(&args[0].node, args[0].span.clone());
                    let t2 = self.check_expr(&args[1].node, args[1].span.clone());
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
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Integer) {
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
                    let first_ty = self.check_expr(&first_arg.node, first_arg.span.clone());
                    if !matches!(first_ty, ResolvedType::Integer | ResolvedType::Float) {
                        self.error(
                            "range() arguments must be all Integer or all Float".to_string(),
                            span.clone(),
                        );
                    } else {
                        range_ty = first_ty.clone();
                    }
                    for arg in &args[1..] {
                        let arg_ty = self.check_expr(&arg.node, arg.span.clone());
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
                                    range_ty, arg_ty
                                ),
                                arg.span.clone(),
                            );
                        }
                    }
                }
                if let Some(step) = args.get(2) {
                    if matches!(step.node, Expr::Literal(Literal::Integer(0)))
                        || matches!(step.node, Expr::Literal(Literal::Float(f)) if f == 0.0)
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
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::String) {
                        self.error(format!("File.read() requires String path, got {}", t), span);
                    }
                }
                Some(ResolvedType::String)
            }
            "File__write" => {
                self.check_arg_count(name, args, 2, span.clone());
                if args.len() >= 2 {
                    let path_t = self.check_expr(&args[0].node, args[0].span.clone());
                    let content_t = self.check_expr(&args[1].node, args[1].span.clone());
                    if !matches!(path_t, ResolvedType::String) {
                        self.error(
                            "File.write() path must be String".to_string(),
                            args[0].span.clone(),
                        );
                    }
                    if !matches!(content_t, ResolvedType::String) {
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
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::String) {
                        self.error(
                            format!("File.exists() requires String path, got {}", t),
                            span,
                        );
                    }
                }
                Some(ResolvedType::Boolean)
            }
            "File__delete" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::String) {
                        self.error(
                            format!("File.delete() requires String path, got {}", t),
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
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::String) {
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
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Integer) {
                        self.error(
                            "Time.sleep() requires Integer milliseconds".to_string(),
                            span,
                        );
                    }
                }
                Some(ResolvedType::None)
            }
            // System Functions
            "System__getenv" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::String) {
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
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::String) {
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
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::String) {
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
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Integer) {
                        self.error(
                            "Args.get() requires Integer index".to_string(),
                            span.clone(),
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
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Boolean | ResolvedType::Integer) {
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
                    let t1 = self.check_expr(&args[0].node, args[0].span.clone());
                    let t2 = self.check_expr(&args[1].node, args[1].span.clone());
                    if !self.types_compatible(&t1, &t2) {
                        self.error(
                            format!(
                                "{}() arguments must have compatible types: {} vs {}",
                                name, t1, t2
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
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Boolean | ResolvedType::Integer) {
                        self.error("assert_true() requires boolean".to_string(), span.clone());
                    }
                }
                Some(ResolvedType::None)
            }
            "assert_false" => {
                // assert_false(condition: Boolean): None
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Boolean | ResolvedType::Integer) {
                        self.error("assert_false() requires boolean".to_string(), span.clone());
                    }
                }
                Some(ResolvedType::None)
            }
            "fail" => {
                // fail(message: String): None - unconditionally fails
                if !args.is_empty() {
                    self.check_arg_count(name, args, 1, span.clone());
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::String) {
                        self.error("fail() requires String message".to_string(), span.clone());
                    }
                }
                Some(ResolvedType::None)
            }
            _ => None,
        }
    }

    /// Check method call on object
    fn check_method_call(
        &mut self,
        obj_type: &ResolvedType,
        method: &str,
        args: &[Spanned<Expr>],
        type_args: &[Type],
        span: Span,
    ) -> ResolvedType {
        if !type_args.is_empty() && !matches!(obj_type, ResolvedType::Class(_)) {
            self.error(
                format!(
                    "Method '{}' on type '{}' does not accept explicit type arguments",
                    method, obj_type
                ),
                span.clone(),
            );
        }

        match obj_type {
            ResolvedType::List(inner) => match method {
                "push" => {
                    self.check_arg_count(method, args, 1, span.clone());
                    if !args.is_empty() {
                        let arg_type = self.check_expr(&args[0].node, args[0].span.clone());
                        if !self.types_compatible(inner, &arg_type) {
                            self.error(
                                format!(
                                    "List.push() type mismatch: expected {}, got {}",
                                    inner, arg_type
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
                        let idx_type = self.check_expr(&args[0].node, args[0].span.clone());
                        if !matches!(idx_type, ResolvedType::Integer) {
                            self.error(
                                format!("List.get() index must be Integer, got {}", idx_type),
                                args[0].span.clone(),
                            );
                        }
                    }
                    (**inner).clone()
                }
                "set" => {
                    self.check_arg_count(method, args, 2, span.clone());
                    if args.len() >= 2 {
                        let idx_type = self.check_expr(&args[0].node, args[0].span.clone());
                        let val_type = self.check_expr(&args[1].node, args[1].span.clone());
                        if !matches!(idx_type, ResolvedType::Integer) {
                            self.error(
                                "List.set() index must be Integer".to_string(),
                                args[0].span.clone(),
                            );
                        }
                        if !self.types_compatible(inner, &val_type) {
                            self.error(
                                format!(
                                    "List.set() value type mismatch: expected {}, got {}",
                                    inner, val_type
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
                        let k = self.check_expr(&args[0].node, args[0].span.clone());
                        let v = self.check_expr(&args[1].node, args[1].span.clone());
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
                        let k = self.check_expr(&args[0].node, args[0].span.clone());
                        if !self.types_compatible(key_type, &k) {
                            self.error("Map key type mismatch".to_string(), args[0].span.clone());
                        }
                    }
                    (**val_type).clone()
                }
                "contains" => {
                    self.check_arg_count(method, args, 1, span.clone());
                    if !args.is_empty() {
                        let k = self.check_expr(&args[0].node, args[0].span.clone());
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
                        let arg_type = self.check_expr(&args[0].node, args[0].span.clone());
                        if !self.types_compatible(inner, &arg_type) {
                            self.error(
                                format!(
                                    "Set.{}() type mismatch: expected {}, got {}",
                                    method, inner, arg_type
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
                if let Some(interface) = self.interfaces.get(&base_name).cloned() {
                    if let Some(sig) = interface.methods.get(method) {
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
                                let arg_type = self.check_expr(&arg.node, arg.span.clone());
                                if !self.types_compatible(param_type, &arg_type) {
                                    self.error(
                                        format!(
                                            "Argument type mismatch: expected {}, got {}",
                                            param_type, arg_type
                                        ),
                                        arg.span.clone(),
                                    );
                                }
                            }
                        }
                        sig.return_type.clone()
                    } else {
                        self.error(
                            format!("Unknown method '{}' on interface '{}'", method, base_name),
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
                    let (inst_params, inst_return_type) = self.instantiate_signature_for_call(
                        &method_name,
                        &sig,
                        type_args,
                        span.clone(),
                    );
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
                            let arg_type = self.check_expr(&arg.node, arg.span.clone());
                            if !self.types_compatible(param_type, &arg_type) {
                                self.error(
                                    format!(
                                        "Argument type mismatch: expected {}, got {}",
                                        param_type, arg_type
                                    ),
                                    arg.span.clone(),
                                );
                            }
                        }
                    }
                    inst_return_type
                } else {
                    self.error(format!("Unknown class: {}", name), span);
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
                        let t = self.check_expr(&arg.node, arg.span.clone());
                        if !matches!(t, ResolvedType::Integer) {
                            self.error(
                                format!(
                                    "Task.await_timeout() expects Integer milliseconds, got {}",
                                    t
                                ),
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
            _ => {
                self.error(format!("Cannot call method on type {}", obj_type), span);
                ResolvedType::Unknown
            }
        }
    }

    /// Check field access
    fn check_field_access(
        &mut self,
        obj_type: &ResolvedType,
        field: &str,
        span: Span,
    ) -> ResolvedType {
        match obj_type {
            ResolvedType::Class(name) => {
                let (base_name, class_substitutions) = self.instantiated_class_substitutions(name);
                if self.interfaces.contains_key(&base_name) {
                    self.error(
                        format!("Interfaces do not expose fields ('{}')", field),
                        span,
                    );
                    return ResolvedType::Unknown;
                }
                if let Some((owner, field_type, _, visibility)) =
                    self.lookup_class_field(&base_name, field)
                {
                    self.check_member_visibility(&owner, visibility, "Field", field, span.clone());
                    return Self::substitute_type_vars(&field_type, &class_substitutions);
                }
                self.error(
                    format!("Unknown field '{}' on class '{}'", field, name),
                    span,
                );
                ResolvedType::Unknown
            }
            _ => {
                self.error(format!("Cannot access field on type {}", obj_type), span);
                ResolvedType::Unknown
            }
        }
    }

    /// Check binary operator
    fn check_binary_op(
        &mut self,
        op: BinOp,
        left: &ResolvedType,
        right: &ResolvedType,
        span: Span,
    ) -> ResolvedType {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                if matches!(op, BinOp::Add)
                    && matches!(left, ResolvedType::String)
                    && matches!(right, ResolvedType::String)
                {
                    return ResolvedType::String;
                }

                if !left.is_numeric() || !right.is_numeric() {
                    self.error(
                        format!(
                            "Arithmetic operator requires numeric types, got {} and {}",
                            left, right
                        ),
                        span,
                    );
                    return ResolvedType::Unknown;
                }
                // Float if either is float
                if matches!(left, ResolvedType::Float) || matches!(right, ResolvedType::Float) {
                    ResolvedType::Float
                } else {
                    ResolvedType::Integer
                }
            }
            BinOp::Eq | BinOp::NotEq => {
                if !self.types_compatible(left, right) {
                    self.error(format!("Cannot compare {} and {}", left, right), span);
                }
                ResolvedType::Boolean
            }
            BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                if !left.is_numeric() || !right.is_numeric() {
                    self.error(
                        format!(
                            "Comparison requires numeric types, got {} and {}",
                            left, right
                        ),
                        span,
                    );
                }
                ResolvedType::Boolean
            }
            BinOp::And | BinOp::Or => {
                if !matches!(left, ResolvedType::Boolean) || !matches!(right, ResolvedType::Boolean)
                {
                    self.error(
                        format!(
                            "Logical operator requires Boolean types, got {} and {}",
                            left, right
                        ),
                        span,
                    );
                }
                ResolvedType::Boolean
            }
        }
    }

    /// Check argument count
    fn check_arg_count(&mut self, name: &str, args: &[Spanned<Expr>], expected: usize, span: Span) {
        if args.len() != expected {
            self.error(
                format!(
                    "{}() expects {} argument(s), got {}",
                    name,
                    expected,
                    args.len()
                ),
                span,
            );
        }
    }

    /// Resolve AST type to checked type
    #[allow(clippy::only_used_in_recursion)]
    fn resolve_type(&self, ty: &Type) -> ResolvedType {
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
                if let Some(module_scoped) = self.module_scoped_type_name(name) {
                    return ResolvedType::Class(module_scoped);
                }
                // Check for built-in types that might be parsed as Named
                match name.as_str() {
                    "Range" => ResolvedType::Class("Range".to_string()),
                    _ => ResolvedType::Class(name.clone()),
                }
            }
            Type::Option(inner) => {
                let inner = self.resolve_type(inner);
                self.module_scoped_generic_type("Option", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Option(Box::new(inner)))
            }
            Type::Result(ok, err) => {
                let ok = self.resolve_type(ok);
                let err = self.resolve_type(err);
                self.module_scoped_generic_type("Result", &[ok.clone(), err.clone()])
                    .unwrap_or_else(|| ResolvedType::Result(Box::new(ok), Box::new(err)))
            }
            Type::List(inner) => {
                let inner = self.resolve_type(inner);
                self.module_scoped_generic_type("List", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::List(Box::new(inner)))
            }
            Type::Map(k, v) => {
                let key = self.resolve_type(k);
                let value = self.resolve_type(v);
                self.module_scoped_generic_type("Map", &[key.clone(), value.clone()])
                    .unwrap_or_else(|| ResolvedType::Map(Box::new(key), Box::new(value)))
            }
            Type::Set(inner) => {
                let inner = self.resolve_type(inner);
                self.module_scoped_generic_type("Set", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Set(Box::new(inner)))
            }
            Type::Ref(inner) => ResolvedType::Ref(Box::new(self.resolve_type(inner))),
            Type::MutRef(inner) => ResolvedType::MutRef(Box::new(self.resolve_type(inner))),
            Type::Box(inner) => {
                let inner = self.resolve_type(inner);
                self.module_scoped_generic_type("Box", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Box(Box::new(inner)))
            }
            Type::Rc(inner) => {
                let inner = self.resolve_type(inner);
                self.module_scoped_generic_type("Rc", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Rc(Box::new(inner)))
            }
            Type::Arc(inner) => {
                let inner = self.resolve_type(inner);
                self.module_scoped_generic_type("Arc", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Arc(Box::new(inner)))
            }
            Type::Ptr(inner) => {
                let inner = self.resolve_type(inner);
                self.module_scoped_generic_type("Ptr", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Ptr(Box::new(inner)))
            }
            Type::Task(inner) => {
                let inner = self.resolve_type(inner);
                self.module_scoped_generic_type("Task", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Task(Box::new(inner)))
            }
            Type::Range(inner) => {
                let inner = self.resolve_type(inner);
                self.module_scoped_generic_type("Range", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| ResolvedType::Range(Box::new(inner)))
            }
            Type::Function(params, ret) => ResolvedType::Function(
                params.iter().map(|p| self.resolve_type(p)).collect(),
                Box::new(self.resolve_type(ret)),
            ),
            Type::Generic(name, args) => {
                // Handle generic types
                match name.as_str() {
                    _ if self.module_scoped_type_name(name).is_some() => {
                        let resolved_name = self
                            .module_scoped_type_name(name)
                            .unwrap_or_else(|| name.clone());
                        let args = args
                            .iter()
                            .map(|arg| self.resolve_type(arg).to_string())
                            .collect::<Vec<_>>()
                            .join(", ");
                        ResolvedType::Class(format!("{}<{}>", resolved_name, args))
                    }
                    "Option" if args.len() == 1 => {
                        ResolvedType::Option(Box::new(self.resolve_type(&args[0])))
                    }
                    "Result" if args.len() == 2 => ResolvedType::Result(
                        Box::new(self.resolve_type(&args[0])),
                        Box::new(self.resolve_type(&args[1])),
                    ),
                    "List" if args.len() == 1 => {
                        ResolvedType::List(Box::new(self.resolve_type(&args[0])))
                    }
                    "Map" if args.len() == 2 => ResolvedType::Map(
                        Box::new(self.resolve_type(&args[0])),
                        Box::new(self.resolve_type(&args[1])),
                    ),
                    "Set" if args.len() == 1 => {
                        ResolvedType::Set(Box::new(self.resolve_type(&args[0])))
                    }
                    "Box" if args.len() == 1 => {
                        ResolvedType::Box(Box::new(self.resolve_type(&args[0])))
                    }
                    "Rc" if args.len() == 1 => {
                        ResolvedType::Rc(Box::new(self.resolve_type(&args[0])))
                    }
                    "Arc" if args.len() == 1 => {
                        ResolvedType::Arc(Box::new(self.resolve_type(&args[0])))
                    }
                    "Ptr" if args.len() == 1 => {
                        ResolvedType::Ptr(Box::new(self.resolve_type(&args[0])))
                    }
                    "Task" if args.len() == 1 => {
                        ResolvedType::Task(Box::new(self.resolve_type(&args[0])))
                    }
                    "Range" if args.len() == 1 => {
                        ResolvedType::Range(Box::new(self.resolve_type(&args[0])))
                    }
                    _ => {
                        let args = args
                            .iter()
                            .map(|arg| self.resolve_type(arg).to_string())
                            .collect::<Vec<_>>()
                            .join(", ");
                        ResolvedType::Class(format!("{}<{}>", name, args))
                    }
                }
            }
        }
    }

    /// Get type of a literal
    fn literal_type(&self, lit: &Literal) -> ResolvedType {
        match lit {
            Literal::Integer(_) => ResolvedType::Integer,
            Literal::Float(_) => ResolvedType::Float,
            Literal::Boolean(_) => ResolvedType::Boolean,
            Literal::String(_) => ResolvedType::String,
            Literal::Char(_) => ResolvedType::Char,
            Literal::None => ResolvedType::None,
        }
    }

    /// Check if two types are compatible
    #[allow(clippy::only_used_in_recursion)]
    fn types_compatible(&self, expected: &ResolvedType, actual: &ResolvedType) -> bool {
        if expected == actual {
            return true;
        }

        // Handle type variables
        if matches!(expected, ResolvedType::TypeVar(_))
            || matches!(actual, ResolvedType::TypeVar(_))
        {
            return true; // Type inference will resolve
        }

        // Unknown is compatible with everything (error recovery)
        if matches!(expected, ResolvedType::Unknown) || matches!(actual, ResolvedType::Unknown) {
            return true;
        }

        // Integer can be promoted to Float
        if matches!(expected, ResolvedType::Float) && matches!(actual, ResolvedType::Integer) {
            return true;
        }

        // Class compatibility:
        // - exact class already handled above
        // - subclass is compatible with base class
        // - class implementing interface is compatible with interface type
        if let (ResolvedType::Class(expected_name), ResolvedType::Class(actual_name)) =
            (expected, actual)
        {
            let expected_base = self.class_base_name(expected_name);
            let actual_base = self.class_base_name(actual_name);
            if self.interfaces.contains_key(expected_base) {
                return self.class_implements_interface(actual_base, expected_base)
                    || self.interface_extends(actual_base, expected_base);
            }
            if self.classes.contains_key(expected_base) && self.classes.contains_key(actual_base) {
                return self.is_same_or_subclass_of(actual_base, expected_base);
            }
        }

        // Generic type compatibility
        match (expected, actual) {
            (ResolvedType::Ref(e), ResolvedType::Ref(a)) => self.types_compatible(e, a),
            (ResolvedType::MutRef(e), ResolvedType::MutRef(a)) => self.types_compatible(e, a),
            (ResolvedType::Ptr(e), ResolvedType::Ptr(a)) => self.types_compatible(e, a),
            (ResolvedType::Box(e), ResolvedType::Box(a)) => self.types_compatible(e, a),
            (ResolvedType::Rc(e), ResolvedType::Rc(a)) => self.types_compatible(e, a),
            (ResolvedType::Arc(e), ResolvedType::Arc(a)) => self.types_compatible(e, a),
            // Can use &mut T where &T is expected
            (ResolvedType::Ref(e), ResolvedType::MutRef(a)) => self.types_compatible(e, a),
            // List compatibility
            (ResolvedType::List(e), ResolvedType::List(a)) => self.types_compatible(e, a),
            (ResolvedType::Set(e), ResolvedType::Set(a)) => self.types_compatible(e, a),
            // Option compatibility
            (ResolvedType::Option(e), ResolvedType::Option(a)) => self.types_compatible(e, a),
            (ResolvedType::Task(e), ResolvedType::Task(a)) => self.types_compatible(e, a),
            (ResolvedType::Range(e), ResolvedType::Range(a)) => self.types_compatible(e, a),
            // Result compatibility
            (ResolvedType::Result(e_ok, e_err), ResolvedType::Result(a_ok, a_err)) => {
                self.types_compatible(e_ok, a_ok) && self.types_compatible(e_err, a_err)
            }
            // Map compatibility
            (ResolvedType::Map(ek, ev), ResolvedType::Map(ak, av)) => {
                self.types_compatible(ek, ak) && self.types_compatible(ev, av)
            }
            (ResolvedType::Function(e_params, e_ret), ResolvedType::Function(a_params, a_ret)) => {
                e_params.len() == a_params.len()
                    && e_params
                        .iter()
                        .zip(a_params.iter())
                        .all(|(e, a)| self.types_compatible(e, a))
                    && self.types_compatible(e_ret, a_ret)
            }
            _ => false,
        }
    }

    /// Fresh type variable for inference
    fn fresh_type_var(&mut self) -> ResolvedType {
        let id = self.type_var_counter;
        self.type_var_counter += 1;
        ResolvedType::TypeVar(id)
    }

    /// Enter a new scope
    fn enter_scope(&mut self) {
        let new_scope = Scope {
            variables: HashMap::new(),
            parent: Some(self.current_scope),
        };
        self.scopes.push(new_scope);
        self.current_scope = self.scopes.len() - 1;
    }

    /// Exit current scope
    fn exit_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }

    /// Declare a variable in current scope
    fn declare_variable(&mut self, name: &str, ty: ResolvedType, mutable: bool, span: Span) {
        let var = VarInfo {
            ty,
            mutable,
            initialized: true,
            span,
        };
        self.scopes[self.current_scope]
            .variables
            .insert(name.to_string(), var);
    }

    /// Look up a variable in scope chain
    fn lookup_variable(&self, name: &str) -> Option<&VarInfo> {
        let mut scope_idx = self.current_scope;
        loop {
            if let Some(var) = self.scopes[scope_idx].variables.get(name) {
                return Some(var);
            }
            if let Some(parent) = self.scopes[scope_idx].parent {
                scope_idx = parent;
            } else {
                break;
            }
        }
        None
    }

    /// Parse a type string like "Integer" or "List<Integer>"
    fn parse_type_string(&self, s: &str) -> ResolvedType {
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

                        match base {
                            "List" => {
                                ResolvedType::List(Box::new(self.parse_type_string(inner_str)))
                            }
                            "Set" => ResolvedType::Set(Box::new(self.parse_type_string(inner_str))),
                            "Option" => {
                                ResolvedType::Option(Box::new(self.parse_type_string(inner_str)))
                            }
                            "Task" => {
                                ResolvedType::Task(Box::new(self.parse_type_string(inner_str)))
                            }
                            "Box" => ResolvedType::Box(Box::new(self.parse_type_string(inner_str))),
                            "Rc" => ResolvedType::Rc(Box::new(self.parse_type_string(inner_str))),
                            "Arc" => ResolvedType::Arc(Box::new(self.parse_type_string(inner_str))),
                            "Ptr" => ResolvedType::Ptr(Box::new(self.parse_type_string(inner_str))),
                            "Map" => {
                                // Split by comma, respecting nested brackets
                                let parts = self.split_generic_args(inner_str);
                                if parts.len() == 2 {
                                    ResolvedType::Map(
                                        Box::new(self.parse_type_string(&parts[0])),
                                        Box::new(self.parse_type_string(&parts[1])),
                                    )
                                } else {
                                    ResolvedType::Unknown
                                }
                            }
                            "Result" => {
                                let parts = self.split_generic_args(inner_str);
                                if parts.len() == 2 {
                                    ResolvedType::Result(
                                        Box::new(self.parse_type_string(&parts[0])),
                                        Box::new(self.parse_type_string(&parts[1])),
                                    )
                                } else {
                                    ResolvedType::Unknown
                                }
                            }
                            _ => ResolvedType::Class(s.to_string()),
                        }
                    } else {
                        ResolvedType::Class(s.to_string())
                    }
                } else {
                    ResolvedType::Class(s.to_string())
                }
            }
        }
    }

    fn parse_function_type_string(&self, s: &str) -> Option<(Vec<String>, String)> {
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

    fn split_type_list(&self, s: &str) -> Vec<String> {
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

    fn split_generic_args_static(s: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut depth = 0;

        for c in s.chars() {
            match c {
                '<' => {
                    depth += 1;
                    current.push(c);
                }
                '>' => {
                    depth -= 1;
                    current.push(c);
                }
                ',' if depth == 0 => {
                    parts.push(current.trim().to_string());
                    current = String::new();
                }
                _ => current.push(c),
            }
        }
        parts.push(current.trim().to_string());
        parts
    }

    /// Split generic arguments by comma, respecting nested < >
    fn split_generic_args(&self, s: &str) -> Vec<String> {
        Self::split_generic_args_static(s)
    }

    /// Report an error
    fn error(&mut self, message: String, span: Span) {
        self.errors.push(TypeError::new(message, span));
    }

    /// Report an error with hint
    fn error_with_hint(&mut self, message: String, span: Span, hint: String) {
        self.errors
            .push(TypeError::new(message, span).with_hint(hint));
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::Parser;

    fn check_source(source: &str) -> Result<(), Vec<TypeError>> {
        let tokens = tokenize(source).expect("tokenize");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("parse");
        let mut checker = TypeChecker::new(source.to_string());
        checker.check(&program)
    }

    #[test]
    fn rejects_private_member_access_from_outside_class() {
        let src = r#"
            class Secret {
                private value: Integer;
                constructor(v: Integer) { this.value = v; }
                private function getV(): Integer { return this.value; }
            }
            function main(): Integer {
                s: Secret = Secret(1);
                x: Integer = s.value;
                y: Integer = s.getV();
                return x + y;
            }
        "#;
        let errors = check_source(src).expect_err("visibility violation should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("private"), "{joined}");
    }

    #[test]
    fn rejects_private_class_construction_from_outside() {
        let src = r#"
            private class Secret {
                constructor() {}
            }
            function main(): None {
                s: Secret = Secret();
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("private class use should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Class 'Secret' is private"), "{joined}");
    }

    #[test]
    fn rejects_private_class_in_function_signature() {
        let src = r#"
            private class Secret { constructor() {} }
            function take(s: Secret): None { return None; }
            function main(): None { return None; }
        "#;
        let errors = check_source(src).expect_err("private class in signature should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Class 'Secret' is private"), "{joined}");
    }

    #[test]
    fn rejects_extending_private_class_from_outside() {
        let src = r#"
            private class Base { constructor() {} }
            class Child extends Base { constructor() {} }
            function main(): None { return None; }
        "#;
        let errors = check_source(src).expect_err("extending private base should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Class 'Base' is private"), "{joined}");
    }

    #[test]
    fn rejects_private_class_in_interface_signature() {
        let src = r#"
            private class Secret { constructor() {} }
            interface I {
                function leak(s: Secret): None;
            }
            function main(): None { return None; }
        "#;
        let errors =
            check_source(src).expect_err("private class in interface signature should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Class 'Secret' is private"), "{joined}");
    }

    #[test]
    fn supports_inherited_method_lookup() {
        let src = r#"
            class Base {
                public function greet(): Integer { return 7; }
            }
            class Child extends Base {
                constructor() {}
            }
            function main(): Integer {
                c: Child = Child();
                return c.greet();
            }
        "#;
        check_source(src).expect("inherited method should typecheck");
    }

    #[test]
    fn enforces_interface_contracts() {
        let src = r#"
            interface Printable {
                function print_me(): None;
            }
            class Book implements Printable {
                constructor() {}
                function other(): None { return None; }
            }
            function main(): Integer { return 0; }
        "#;
        let errors = check_source(src).expect_err("missing interface method should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("must implement interface method"),
            "{joined}"
        );
    }

    #[test]
    fn allows_interface_typed_parameters() {
        let src = r#"
            interface Printable {
                function print_me(): None;
            }
            class Book implements Printable {
                constructor() {}
                function print_me(): None { return None; }
            }
            function show(item: Printable): None {
                item.print_me();
                return None;
            }
            function main(): Integer {
                b: Book = Book();
                show(b);
                return 0;
            }
        "#;
        check_source(src).expect("interface-typed calls should typecheck");
    }

    #[test]
    fn rejects_protected_member_access_from_non_subclass() {
        let src = r#"
            class Base {
                protected value: Integer;
                constructor(v: Integer) { this.value = v; }
            }
            class Other {
                function leak(b: Base): Integer {
                    return b.value;
                }
            }
            function main(): Integer { return 0; }
        "#;
        let errors = check_source(src).expect_err("protected visibility violation should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("protected"), "{joined}");
    }

    #[test]
    fn enforces_parent_interface_methods_when_implementing_child_interface() {
        let src = r#"
            interface Named {
                function name(): Integer;
            }
            interface Printable extends Named {
                function print_me(): None;
            }
            class Report implements Printable {
                constructor() {}
                function print_me(): None { return None; }
            }
            function main(): Integer { return 0; }
        "#;
        let errors = check_source(src).expect_err("missing parent-interface method should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("must implement interface method"),
            "{joined}"
        );
    }

    #[test]
    fn rejects_invalid_list_constructor_arguments() {
        let src = r#"
            function main(): None {
                xs: List<Integer> = List<Integer>("bad", true, 5);
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("invalid List constructor should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("expects 0 or 1 arguments"), "{joined}");
    }

    #[test]
    fn rejects_invalid_map_set_constructor_arguments() {
        let src = r#"
            function main(): None {
                m: Map<String, Integer> = Map<String, Integer>(1);
                s: Set<Integer> = Set<Integer>(1, 2);
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("invalid Map/Set constructors should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Constructor Map<String, Integer> expects 0 arguments"),
            "{joined}"
        );
        assert!(
            joined.contains("Constructor Set<Integer> expects 0 arguments"),
            "{joined}"
        );
    }

    #[test]
    fn accepts_valid_builtin_generic_constructors() {
        let src = r#"
            function main(): None {
                xs: List<Integer> = List<Integer>();
                ys: List<Integer> = List<Integer>(32);
                m: Map<String, Integer> = Map<String, Integer>();
                s: Set<Integer> = Set<Integer>();
                o: Option<Integer> = Option<Integer>();
                r: Result<Integer, String> = Result<Integer, String>();
                return None;
            }
        "#;
        check_source(src).expect("valid built-in generic constructors should typecheck");
    }

    #[test]
    fn accepts_map_indexing_with_non_integer_key_types() {
        let src = r#"
            function main(): None {
                m: Map<String, Integer> = Map<String, Integer>();
                m.set("x", 7);
                value: Integer = m["x"];
                return None;
            }
        "#;
        check_source(src).expect("Map indexing should accept key-typed indices");
    }

    #[test]
    fn rejects_map_indexing_with_wrong_key_type() {
        let src = r#"
            function main(): None {
                m: Map<String, Integer> = Map<String, Integer>();
                value: Integer = m[1];
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("wrong map key index type should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Map index type mismatch: expected String, got Integer"),
            "{joined}"
        );
    }

    #[test]
    fn accepts_user_defined_generic_class_construction_and_methods() {
        let src = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }
            function main(): None {
                b: Boxed<Integer> = Boxed<Integer>(1);
                out: Integer = if (true) { b.get(); } else { b.value; };
                return None;
            }
        "#;
        check_source(src).expect("generic class construction and member use should typecheck");
    }

    #[test]
    fn rejects_explicit_type_args_on_non_generic_function() {
        let src = r#"
            function f(x: Integer): Integer { return x; }
            function main(): None {
                y: Integer = f<String>(1);
                return None;
            }
        "#;
        let errors = check_source(src)
            .expect_err("non-generic function call with explicit type args should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("is not generic"), "{joined}");
    }

    #[test]
    fn rejects_explicit_type_arg_arity_mismatch() {
        let src = r#"
            function id<T>(x: T): T { return x; }
            function main(): None {
                y: Integer = id<Integer, String>(1);
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("generic arity mismatch should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("expects 1 type arguments"), "{joined}");
    }

    #[test]
    fn rejects_unknown_explicit_type_argument() {
        let src = r#"
            function id<T>(x: T): T { return x; }
            function main(): None {
                y: Integer = id<Nope>(1);
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("unknown explicit type arg should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Unknown type: Nope"), "{joined}");
    }

    #[test]
    fn explicit_generic_method_call_typechecks() {
        let src = r#"
            class C {
                constructor() {}
                function id<T>(x: T): T { return x; }
            }
            function main(): None {
                c: C = C();
                y: Integer = c.id<Integer>(1);
                return None;
            }
        "#;
        check_source(src).expect("explicit generic method call should typecheck");
    }

    #[test]
    fn explicit_generic_module_call_typechecks() {
        let src = r#"
            module M {
                function id<T>(x: T): T { return x; }
            }
            function main(): None {
                y: Integer = M.id<Integer>(1);
                return None;
            }
        "#;
        check_source(src).expect("explicit generic module call should typecheck");
    }

    #[test]
    fn explicit_generic_nested_module_mangled_call_typechecks() {
        let src = r#"
            module A {
                module X {
                    function id<T>(x: T): T { return x; }
                }
                module Y {
                    function add(a: Integer, b: Integer): Integer { return a + b; }
                }
            }
            function main(): None {
                y: Integer = A__X__id<Integer>(A__Y__add(1, 2));
                return None;
            }
        "#;
        check_source(src).expect("explicit generic nested module mangled call should typecheck");
    }

    #[test]
    fn nested_module_dot_call_typechecks() {
        let src = r#"
            module A {
                module Y {
                    function add(a: Integer, b: Integer): Integer { return a + b; }
                }
            }
            function main(): None {
                y: Integer = A.Y.add(1, 2);
                return None;
            }
        "#;
        check_source(src).expect("nested module dot call should typecheck");
    }

    #[test]
    fn explicit_generic_nested_module_dot_call_typechecks() {
        let src = r#"
            module A {
                module X {
                    function id<T>(x: T): T { return x; }
                }
                module Y {
                    function add(a: Integer, b: Integer): Integer { return a + b; }
                }
            }
            function main(): None {
                y: Integer = A.X.id<Integer>(A.Y.add(1, 2));
                return None;
            }
        "#;
        check_source(src).expect("explicit generic nested module dot call should typecheck");
    }

    #[test]
    fn list_of_function_types_typechecks() {
        let src = r#"
            function main(): None {
                fs: List<(Integer) -> Integer> = List<(Integer) -> Integer>();
                return None;
            }
        "#;
        check_source(src).expect("list of function types should typecheck");
    }

    #[test]
    fn option_some_static_constructor_typechecks() {
        let src = r#"
            function main(): None {
                maybe: Option<Integer> = Option.some(1);
                return None;
            }
        "#;
        check_source(src).expect("Option.some should typecheck");
    }

    #[test]
    fn option_of_function_type_typechecks() {
        let src = r#"
            function add1(x: Integer): Integer { return x + 1; }
            function main(): None {
                maybe: Option<(Integer) -> Integer> = Option.some(add1);
                return None;
            }
        "#;
        check_source(src).expect("Option of function type should typecheck");
    }

    #[test]
    fn function_valued_field_call_typechecks() {
        let src = r#"
            class C {
                f: (Integer) -> Integer;
                constructor() { this.f = (n: Integer) => n + 1; }
            }
            function main(): None {
                c: C = C();
                x: Integer = c.f(2);
                return None;
            }
        "#;
        check_source(src).expect("function-valued field calls should typecheck");
    }

    #[test]
    fn module_alias_function_values_typecheck() {
        let src = r#"
            module util {
                function add1(x: Integer): Integer { return x + 1; }
                function twice(f: (Integer) -> Integer, x: Integer): Integer { return f(f(x)); }
            }

            function main(): None {
                f: (Integer) -> Integer = util.add1;
                y: Integer = util.twice(f, 1);
                return None;
            }
        "#;
        check_source(src).expect("module alias-style function values should typecheck");
    }

    #[test]
    fn rejects_field_assignment_through_immutable_owner() {
        let src = r#"
            class C {
                mut v: Integer;
                constructor() { this.v = 1; }
            }
            function main(): None {
                c: C = C();
                c.v = 2;
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("immutable owner field assignment should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Cannot assign to immutable variable 'c'"),
            "{joined}"
        );
    }

    #[test]
    fn rejects_index_assignment_through_immutable_owner() {
        let src = r#"
            function main(): None {
                xs: List<Integer> = List<Integer>();
                xs.push(1);
                xs[0] = 2;
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("immutable owner index assignment should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Cannot assign to immutable variable 'xs'"),
            "{joined}"
        );
    }

    #[test]
    fn local_io_variable_does_not_act_as_stdlib_alias() {
        let src = r#"
            import std.io as io;
            function main(): None {
                io: Integer = 1;
                io.println("x");
                return None;
            }
        "#;
        let errors = check_source(src)
            .expect_err("local variable named io must not be treated as std.io alias");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Cannot call method on type Integer"),
            "{joined}"
        );
    }

    #[test]
    fn specific_stdlib_alias_import_resolves_ident_call() {
        let src = r#"
            import std.io.*;
            import std.math.Math__abs as abs_fn;
            function main(): None {
                x: Float = abs_fn(-2.5);
                println(to_string(x));
                return None;
            }
        "#;
        check_source(src).expect("specific stdlib alias import call should typecheck");
    }

    #[test]
    fn if_expression_branches_typecheck() {
        let src = r#"
            function main(): None {
                x: Integer = if (true) { 1; } else { 2; };
                return None;
            }
        "#;
        check_source(src).expect("if expression with matching branch types should typecheck");
    }

    #[test]
    fn if_expression_branch_type_mismatch_fails() {
        let src = r#"
            function main(): None {
                x: Integer = if (true) { 1; } else { "bad"; };
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("if expression branch mismatch should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("If expression branch type mismatch"),
            "{joined}"
        );
    }

    #[test]
    fn if_expression_without_else_is_none_typed() {
        let src = r#"
            function main(): None {
                x: Integer = if (true) { 1; };
                return None;
            }
        "#;
        let errors =
            check_source(src).expect_err("if expression without else should be None-typed");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Type mismatch: cannot assign None to variable of type Integer"),
            "{joined}"
        );
    }

    #[test]
    fn match_expression_branch_type_mismatch_fails() {
        let src = r#"
            function main(): None {
                x: Integer = match (1) {
                    1 => 1,
                    _ => "bad",
                };
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("match expression branch mismatch should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Match expression arm type mismatch")
                || joined.contains("Type mismatch: cannot assign"),
            "{joined}"
        );
    }

    #[test]
    fn match_expression_boolean_non_exhaustive_fails() {
        let src = r#"
            function main(): None {
                x: Integer = match (true) {
                    true => 1,
                };
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("non-exhaustive boolean match should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Non-exhaustive match expression"),
            "{joined}"
        );
    }

    #[test]
    fn match_statement_boolean_non_exhaustive_fails() {
        let src = r#"
            function main(): None {
                match (true) {
                    true => { }
                }
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("non-exhaustive boolean match stmt should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Non-exhaustive match statement"),
            "{joined}"
        );
    }

    #[test]
    fn empty_match_statement_fails() {
        let src = r#"
            function main(): None {
                match (1) {
                }
                return None;
            }
        "#;
        let tokens = tokenize(src).expect("tokenize");
        let mut parser = Parser::new(tokens);
        let err = parser
            .parse_program()
            .expect_err("empty match statement should now fail in parser");
        assert!(
            err.message
                .contains("match statements must contain at least one arm"),
            "{}",
            err.message
        );
    }

    #[test]
    fn integer_match_expression_requires_catch_all() {
        let src = r#"
            function main(): None {
                x: Integer = match (2) {
                    1 => 1,
                };
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("integer match expression without catch-all");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Non-exhaustive match expression"),
            "{joined}"
        );
    }

    #[test]
    fn empty_match_expression_fails() {
        let src = r#"
            function main(): None {
                n: None = match (1) {
                };
                return None;
            }
        "#;
        let tokens = tokenize(src).expect("tokenize");
        let mut parser = Parser::new(tokens);
        let err = parser
            .parse_program()
            .expect_err("empty match expression should now fail in parser");
        assert!(
            err.message
                .contains("match expressions must contain at least one arm"),
            "{}",
            err.message
        );
    }

    #[test]
    fn if_expression_reports_single_undefined_identifier_error() {
        let src = r#"
            function main(): None {
                x: Integer = if (true) { y; } else { 1; };
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("undefined variable should fail");
        let undef_count = errors
            .iter()
            .filter(|e| e.message.contains("Undefined variable: y"))
            .count();
        assert_eq!(undef_count, 1, "{:?}", errors);
    }

    #[test]
    fn match_expression_reports_single_undefined_identifier_error() {
        let src = r#"
            function main(): None {
                x: Integer = match (1) {
                    1 => { y; },
                    _ => { 0; }
                };
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("undefined variable should fail");
        let undef_count = errors
            .iter()
            .filter(|e| e.message.contains("Undefined variable: y"))
            .count();
        assert_eq!(undef_count, 1, "{:?}", errors);
    }

    #[test]
    fn qualified_enum_patterns_typecheck_against_leaf_variant_names() {
        let src = r#"
            enum E {
                A(Integer),
                B(Integer)
            }

            function main(): None {
                value: E = E.A(1);
                match (value) {
                    Enum.A(v) => { require(v == 1); }
                    util.E.B(w) => { require(w == 2); }
                    _ => { }
                }
                return None;
            }
        "#;
        check_source(src).expect("qualified enum patterns should typecheck");
    }

    #[test]
    fn enum_match_expression_is_exhaustive_without_wildcard() {
        let src = r#"
            enum E {
                A(Integer)
            }

            function main(): None {
                value: E = E.A(1);
                result: Integer = match (value) {
                    E.A(v) => v
                };
                require(result == 1);
                return None;
            }
        "#;
        check_source(src).expect("single-variant enum match should be exhaustive");
    }

    #[test]
    fn rejects_extern_function_values_during_typecheck() {
        let src = r#"
            extern(c, "puts") function puts(s: String): Integer;

            function main(): None {
                f: (String) -> Integer = puts;
                return None;
            }
        "#;
        let errors =
            check_source(src).expect_err("extern function value should fail during typecheck");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("extern function 'puts' cannot be used as a first-class value"),
            "{joined}"
        );
    }

    #[test]
    fn rejects_unsupported_enum_payload_types_during_typecheck() {
        let src = r#"
            class C {
                value: Integer;
                constructor(v: Integer) { this.value = v; }
            }

            enum EF {
                A((Integer) -> Integer)
            }

            enum EL {
                A(List<Integer>)
            }

            enum EO {
                A(Option<C>)
            }

            function inc(x: Integer): Integer { return x + 1; }

            function main(): None {
                ef: EF = EF.A(inc);
                el: EL = EL.A(List<Integer>());
                eo: EO = EO.A(Option.some(C(1)));
                return None;
            }
        "#;
        let errors =
            check_source(src).expect_err("unsupported enum payload types should fail early");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Enum payload type '(Integer) -> Integer' is not supported yet"));
        assert!(joined.contains("Enum payload type 'List<Integer>' is not supported yet"));
        assert!(joined.contains("Enum payload type 'Option<C>' is not supported yet"));
    }

    #[test]
    fn rejects_nested_enum_payload_types_during_typecheck() {
        let src = r#"
            enum Inner {
                X(Integer)
            }

            enum Outer {
                A(Inner)
            }

            function main(): Integer { return 0; }
        "#;
        let errors = check_source(src).expect_err("nested enum payload types should fail early");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Enum payload type 'Inner' is not supported yet"));
    }

    #[test]
    fn supports_unit_enum_variant_values() {
        let src = r#"
            enum E { A, B }

            function main(): None {
                e: E = E.A;
                match (e) {
                    E.A => { }
                    E.B => { }
                }
                return None;
            }
        "#;
        check_source(src).expect("unit enum variants should typecheck as values");
    }

    #[test]
    fn rejects_async_blocks_returning_borrowed_references() {
        let src = r#"
            function inc(x: Integer): Integer { return x + 1; }

            function main(): None {
                task: Task<&(Integer) -> Integer> = async {
                    return &inc;
                };
                return None;
            }
        "#;
        let errors =
            check_source(src).expect_err("async block returning borrowed reference should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Async block cannot return a value containing borrowed references"));
    }

    #[test]
    fn rejects_async_functions_returning_borrowed_references() {
        let src = r#"
            function inc(x: Integer): Integer { return x + 1; }

            async function make_ref(): Task<&(Integer) -> Integer> {
                return &inc;
            }
        "#;
        let errors =
            check_source(src).expect_err("async function returning borrowed reference should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains(
            "Async function 'make_ref' cannot return a value containing borrowed references"
        ));
    }

    #[test]
    fn rejects_async_functions_accepting_borrowed_reference_parameters() {
        let src = r#"
            async function read_ref(r: &Integer): Task<Integer> {
                return *r;
            }
        "#;
        let errors = check_source(src)
            .expect_err("async function accepting borrowed reference parameter should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains(
            "Async function 'read_ref' cannot accept a parameter containing borrowed references"
        ));
    }

    #[test]
    fn rejects_async_blocks_capturing_borrowed_reference_variables() {
        let src = r#"
            function main(): None {
                x: Integer = 1;
                r: &Integer = &x;
                task: Task<Integer> = async {
                    return *r;
                };
                return None;
            }
        "#;
        let errors = check_source(src)
            .expect_err("async block capturing borrowed reference variable should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains(
            "Async block cannot capture 'r' because its type contains borrowed references"
        ));
    }

    #[test]
    fn rejects_undocumented_task_result_type_method() {
        let src = r#"
            async function make(): Task<Integer> {
                return 1;
            }

            function main(): Integer {
                t: Task<Integer> = make();
                return t.result_type();
            }
        "#;
        let errors = check_source(src).expect_err("Task.result_type should fail during typecheck");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Unknown Task method: result_type"));
    }

    #[test]
    fn range_accepts_float_arguments() {
        let src = r#"
            function main(): None {
                r: Range<Float> = range(0.0, 3.0, 1.0);
                return None;
            }
        "#;
        check_source(src).expect("float range arguments should type check");
    }

    #[test]
    fn range_rejects_mixed_numeric_arguments() {
        let src = r#"
            function main(): None {
                r = range(0, 3.0, 1.0);
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("mixed numeric range arguments should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("range() arguments must use the same numeric type"),
            "{joined}"
        );
    }

    #[test]
    fn range_rejects_zero_literal_step() {
        let src = r#"
            function main(): None {
                r: Range<Integer> = range(0, 3, 0);
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("zero range step should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("range() step cannot be 0"), "{joined}");
    }

    #[test]
    fn range_rejects_zero_float_literal_step() {
        let src = r#"
            function main(): None {
                r: Range<Float> = range(0.0, 3.0, 0.0);
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("zero float range step should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("range() step cannot be 0"), "{joined}");
    }
}

/// Format type errors with source context
pub fn format_errors(errors: &[TypeError], source: &str, filename: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut output = String::new();

    for error in errors {
        // Find line number
        let mut line_num: usize = 1;
        let mut col: usize = 1;
        for (i, ch) in source.char_indices() {
            if i >= error.span.start {
                break;
            }
            if ch == '\n' {
                line_num += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        output.push_str(&format!("\x1b[1;31merror\x1b[0m: {}\n", error.message));
        output.push_str(&format!(
            "  \x1b[1;34m-->\x1b[0m {}:{}:{}\n",
            filename, line_num, col
        ));
        output.push_str("   \x1b[1;34m|\x1b[0m\n");

        if line_num <= lines.len() {
            output.push_str(&format!(
                "\x1b[1;34m{:3} |\x1b[0m {}\n",
                line_num,
                lines[line_num - 1]
            ));

            // Underline
            let underline_start = col.saturating_sub(1);
            let underline_len = (error.span.end - error.span.start).max(1);
            output.push_str(&format!(
                "   \x1b[1;34m|\x1b[0m {}\x1b[1;31m{}\x1b[0m\n",
                " ".repeat(underline_start),
                "^".repeat(underline_len.min(lines[line_num - 1].len() - underline_start))
            ));
        }

        if let Some(hint) = &error.hint {
            output.push_str(&format!("   \x1b[1;34m= help\x1b[0m: {}\n", hint));
        }

        output.push('\n');
    }

    output
}
