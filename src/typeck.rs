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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum NumericConst {
    Integer(i64),
    Float(f64),
}

impl NumericConst {
    fn is_zero(self) -> bool {
        match self {
            NumericConst::Integer(value) => value == 0,
            NumericConst::Float(value) => value == 0.0,
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
    /// Current async-block inferred return type while checking nested explicit returns
    current_async_return_type: Option<ResolvedType>,
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
    /// Interface bounds declared for generic type variables
    type_var_bounds: HashMap<usize, Vec<String>>,
    /// Current nested module prefix while collecting/checking module-scoped declarations
    current_module_prefix: Option<String>,
}

impl TypeChecker {
    fn supports_display_scalar(ty: &ResolvedType) -> bool {
        matches!(
            ty,
            ResolvedType::Integer
                | ResolvedType::Float
                | ResolvedType::Boolean
                | ResolvedType::String
                | ResolvedType::Char
                | ResolvedType::None
        )
    }

    fn peel_reference_type(ty: &ResolvedType) -> &ResolvedType {
        match ty {
            ResolvedType::Ref(inner) | ResolvedType::MutRef(inner) => {
                Self::peel_reference_type(inner)
            }
            _ => ty,
        }
    }

    pub(crate) fn eval_numeric_const_expr(expr: &Expr) -> Option<NumericConst> {
        match expr {
            Expr::Literal(Literal::Integer(value)) => Some(NumericConst::Integer(*value)),
            Expr::Literal(Literal::Float(value)) => Some(NumericConst::Float(*value)),
            Expr::Unary {
                op: UnaryOp::Neg,
                expr,
            } => match Self::eval_numeric_const_expr(&expr.node)? {
                NumericConst::Integer(value) => value.checked_neg().map(NumericConst::Integer),
                NumericConst::Float(value) => Some(NumericConst::Float(-value)),
            },
            Expr::Binary { op, left, right } => {
                let left = Self::eval_numeric_const_expr(&left.node)?;
                let right = Self::eval_numeric_const_expr(&right.node)?;
                match (left, right) {
                    (NumericConst::Integer(left), NumericConst::Integer(right)) => match op {
                        BinOp::Add => left.checked_add(right).map(NumericConst::Integer),
                        BinOp::Sub => left.checked_sub(right).map(NumericConst::Integer),
                        BinOp::Mul => left.checked_mul(right).map(NumericConst::Integer),
                        BinOp::Div => (right != 0).then(|| NumericConst::Integer(left / right)),
                        BinOp::Mod => (right != 0).then(|| NumericConst::Integer(left % right)),
                        _ => None,
                    },
                    (NumericConst::Float(left), NumericConst::Float(right)) => match op {
                        BinOp::Add => Some(NumericConst::Float(left + right)),
                        BinOp::Sub => Some(NumericConst::Float(left - right)),
                        BinOp::Mul => Some(NumericConst::Float(left * right)),
                        BinOp::Div => (right != 0.0).then(|| NumericConst::Float(left / right)),
                        BinOp::Mod => (right != 0.0).then(|| NumericConst::Float(left % right)),
                        _ => None,
                    },
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn check_non_negative_integer_const(&mut self, expr: &Expr, span: Span, message: &str) {
        if matches!(
            Self::eval_numeric_const_expr(expr),
            Some(NumericConst::Integer(value)) if value < 0
        ) {
            self.error(message.to_string(), span);
        }
    }

    fn eval_const_string_len(expr: &Expr) -> Option<usize> {
        match expr {
            Expr::Literal(Literal::String(text)) => Some(text.chars().count()),
            _ => None,
        }
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
        if let Some(canonical) = stdlib_registry().resolve_alias_call(&namespace, symbol) {
            return Some(canonical);
        }
        let full_mangled = path.replace('.', "__");
        if let Some(resolved) = self.resolve_function_value_name(&full_mangled) {
            return Some(resolved.to_string());
        }
        if let Some(resolved) = self.resolve_function_value_name(symbol) {
            return Some(resolved.to_string());
        }
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
            current_async_return_type: None,
            current_class: None,
            import_aliases: HashMap::new(),
            current_effects: Vec::new(),
            current_is_pure: false,
            current_allow_any: false,
            source,
            current_generic_type_bindings: HashMap::new(),
            type_var_bounds: HashMap::new(),
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

    fn known_type_exists(&self, name: &str) -> bool {
        self.classes.contains_key(name)
            || self.enums.contains_key(name)
            || self.interfaces.contains_key(name)
    }

    fn resolve_known_type_name(&self, name: &str) -> Option<String> {
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

    fn resolve_nominal_reference_name(&self, name: &str) -> Option<String> {
        if let Some(resolved) = self.resolve_known_type_name(name) {
            return Some(resolved);
        }

        if let Some(path) = self.import_aliases.get(name) {
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

    fn resolve_user_defined_generic_type(
        &self,
        name: &str,
        args: &[ResolvedType],
    ) -> Option<ResolvedType> {
        let resolved_name = self.resolve_known_type_name(name)?;
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
        self.normalize_inheritance_references();
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
            .map(|p| {
                let type_var = self.fresh_type_var();
                if let ResolvedType::TypeVar(id) = type_var {
                    self.type_var_bounds.insert(id, p.bounds.clone());
                }
                (p.name.clone(), type_var)
            })
            .collect()
    }

    fn validate_generic_param_bounds(
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

    fn type_satisfies_interface_bound(&self, actual: &ResolvedType, bound: &str) -> bool {
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

    fn type_var_satisfies_bounds(&self, type_var_id: usize, actual: &ResolvedType) -> bool {
        self.type_var_bounds.get(&type_var_id).is_none_or(|bounds| {
            bounds
                .iter()
                .all(|bound| self.type_satisfies_interface_bound(actual, bound))
        })
    }

    fn validate_class_type_argument_bounds(&mut self, class_name: &str, span: Span, context: &str) {
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
                    context, actual, bounds
                ),
                span.clone(),
            );
        }
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
                if let Some(resolved_name) = self.resolve_known_type_name(name) {
                    return ResolvedType::Class(resolved_name);
                }
                match name.as_str() {
                    "Range" => ResolvedType::Class("Range".to_string()),
                    _ => ResolvedType::Class(name.clone()),
                }
            }
            Type::Option(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.resolve_user_defined_generic_type("Option", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Option", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Option(Box::new(inner)))
            }
            Type::Result(ok, err) => {
                let ok = self.resolve_type_with_bindings(ok, bindings);
                let err = self.resolve_type_with_bindings(err, bindings);
                self.resolve_user_defined_generic_type("Result", &[ok.clone(), err.clone()])
                    .or_else(|| {
                        self.module_scoped_generic_type("Result", &[ok.clone(), err.clone()])
                    })
                    .unwrap_or_else(|| ResolvedType::Result(Box::new(ok), Box::new(err)))
            }
            Type::List(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.resolve_user_defined_generic_type("List", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("List", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::List(Box::new(inner)))
            }
            Type::Map(k, v) => {
                let key = self.resolve_type_with_bindings(k, bindings);
                let value = self.resolve_type_with_bindings(v, bindings);
                self.resolve_user_defined_generic_type("Map", &[key.clone(), value.clone()])
                    .or_else(|| {
                        self.module_scoped_generic_type("Map", &[key.clone(), value.clone()])
                    })
                    .unwrap_or_else(|| ResolvedType::Map(Box::new(key), Box::new(value)))
            }
            Type::Set(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.resolve_user_defined_generic_type("Set", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Set", std::slice::from_ref(&inner))
                    })
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
                self.resolve_user_defined_generic_type("Box", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Box", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Box(Box::new(inner)))
            }
            Type::Rc(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.resolve_user_defined_generic_type("Rc", std::slice::from_ref(&inner))
                    .or_else(|| self.module_scoped_generic_type("Rc", std::slice::from_ref(&inner)))
                    .unwrap_or_else(|| ResolvedType::Rc(Box::new(inner)))
            }
            Type::Arc(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.resolve_user_defined_generic_type("Arc", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Arc", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Arc(Box::new(inner)))
            }
            Type::Ptr(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.resolve_user_defined_generic_type("Ptr", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Ptr", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Ptr(Box::new(inner)))
            }
            Type::Task(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.resolve_user_defined_generic_type("Task", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Task", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Task(Box::new(inner)))
            }
            Type::Range(inner) => {
                let inner = self.resolve_type_with_bindings(inner, bindings);
                self.resolve_user_defined_generic_type("Range", std::slice::from_ref(&inner))
                    .or_else(|| {
                        self.module_scoped_generic_type("Range", std::slice::from_ref(&inner))
                    })
                    .unwrap_or_else(|| ResolvedType::Range(Box::new(inner)))
            }
            Type::Function(params, ret) => ResolvedType::Function(
                params
                    .iter()
                    .map(|p| self.resolve_type_with_bindings(p, bindings))
                    .collect(),
                Box::new(self.resolve_type_with_bindings(ret, bindings)),
            ),
            Type::Generic(name, args) => {
                let resolved_args = args
                    .iter()
                    .map(|arg| self.resolve_type_with_bindings(arg, bindings))
                    .collect::<Vec<_>>();
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

    fn signatures_mutually_compatible(&self, left: &FuncSig, right: &FuncSig) -> bool {
        self.signatures_compatible(left, right) && self.signatures_compatible(right, left)
    }

    fn validate_interface_inherited_method_conflicts(
        &mut self,
        interface: &InterfaceDecl,
        key: &str,
        span: Span,
    ) {
        let mut inherited_methods: HashMap<String, (String, FuncSig)> = HashMap::new();
        for parent in &interface.extends {
            let resolved_parent = self
                .resolve_nominal_reference_name(parent)
                .unwrap_or_else(|| parent.clone());
            let mut methods = HashMap::new();
            let mut visited = std::collections::HashSet::new();
            self.collect_interface_methods(&resolved_parent, &mut methods, &mut visited);
            for (method_name, sig) in methods {
                if let Some((existing_parent, existing_sig)) = inherited_methods.get(&method_name) {
                    if !self.signatures_mutually_compatible(existing_sig, &sig) {
                        self.error(
                            format!(
                                "Interface '{}' inherits incompatible signatures for method '{}' from '{}' and '{}'",
                                key, method_name, existing_parent, resolved_parent
                            ),
                            span.clone(),
                        );
                    }
                } else {
                    inherited_methods.insert(method_name, (resolved_parent.clone(), sig));
                }
            }
        }

        for method in &interface.methods {
            let params = method
                .params
                .iter()
                .map(|p| (p.name.clone(), self.resolve_type(&p.ty)))
                .collect::<Vec<_>>();
            let sig = FuncSig {
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
            };
            if let Some((parent_name, parent_sig)) = inherited_methods.get(&method.name) {
                if !self.signatures_mutually_compatible(parent_sig, &sig) {
                    self.error(
                        format!(
                            "Interface '{}.{}' overrides inherited method from '{}' with an incompatible signature",
                            key, method.name, parent_name
                        ),
                        span.clone(),
                    );
                }
            }
        }
    }

    fn lookup_type_var_bound_method(
        &self,
        type_var_id: usize,
        method_name: &str,
    ) -> std::result::Result<Option<FuncSig>, String> {
        let mut matches = Vec::new();
        let Some(bounds) = self.type_var_bounds.get(&type_var_id) else {
            return Ok(None);
        };
        for bound in bounds {
            let resolved_bound = self
                .resolve_nominal_reference_name(bound)
                .unwrap_or_else(|| bound.clone());
            let mut methods = HashMap::new();
            let mut visited = std::collections::HashSet::new();
            self.collect_interface_methods(&resolved_bound, &mut methods, &mut visited);
            if let Some(sig) = methods.get(method_name) {
                matches.push((resolved_bound, sig.clone()));
            }
        }
        if matches.is_empty() {
            return Ok(None);
        }
        let (_, first_sig) = &matches[0];
        for (bound_name, sig) in matches.iter().skip(1) {
            if !self.signatures_mutually_compatible(first_sig, sig) {
                return Err(format!(
                    "Generic bound method '{}.{}' has incompatible signatures across bounds",
                    bound_name, method_name
                ));
            }
        }
        Ok(Some(matches.swap_remove(0).1))
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
                self.validate_class_type_argument_bounds(name, span.clone(), "Type");
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
        self.normalize_inheritance_references();
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

    fn normalize_inheritance_references(&mut self) {
        let class_updates = self
            .classes
            .iter()
            .map(|(name, info)| {
                (
                    name.clone(),
                    info.extends.as_ref().map(|parent| {
                        self.resolve_nominal_reference_name(parent)
                            .unwrap_or_else(|| parent.clone())
                    }),
                    info.implements
                        .iter()
                        .map(|interface_name| {
                            self.resolve_nominal_reference_name(interface_name)
                                .unwrap_or_else(|| interface_name.clone())
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        for (name, extends, implements) in class_updates {
            if let Some(class) = self.classes.get_mut(&name) {
                class.extends = extends;
                class.implements = implements;
            }
        }

        let interface_updates = self
            .interfaces
            .iter()
            .map(|(name, info)| {
                (
                    name.clone(),
                    info.extends
                        .iter()
                        .map(|parent| {
                            self.resolve_nominal_reference_name(parent)
                                .unwrap_or_else(|| parent.clone())
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        for (name, extends) in interface_updates {
            if let Some(interface) = self.interfaces.get_mut(&name) {
                interface.extends = extends;
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
                self.validate_generic_param_bounds(
                    &en.generic_params,
                    span.clone(),
                    &format!("Enum '{}'", en.name),
                );
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
        self.validate_generic_param_bounds(
            &interface.generic_params,
            span.clone(),
            &format!("Interface '{}'", interface.name),
        );
        let interface_key = self
            .current_module_prefix
            .as_ref()
            .map(|prefix| format!("{}__{}", prefix, interface.name))
            .unwrap_or_else(|| interface.name.clone());
        self.validate_interface_inherited_method_conflicts(interface, &interface_key, span.clone());
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

    fn validate_main_signature(&mut self, func: &FunctionDecl, span: Span) {
        if !func.generic_params.is_empty() {
            self.error(
                "main() cannot declare generic parameters".to_string(),
                span.clone(),
            );
        }
        if !func.params.is_empty() {
            self.error("main() cannot declare parameters".to_string(), span.clone());
        }
        if func.is_async {
            self.error(
                "main() cannot be async; use a synchronous main() entrypoint".to_string(),
                span.clone(),
            );
        }
        if func.is_extern || func.extern_abi.is_some() {
            self.error("main() cannot be declared extern".to_string(), span.clone());
        }
        if func.is_variadic {
            self.error("main() cannot be variadic".to_string(), span.clone());
        }
        if !matches!(func.return_type, Type::None | Type::Integer) {
            self.error(
                "main() must return None or Integer".to_string(),
                span.clone(),
            );
        }
    }

    /// Check a function
    fn check_function(&mut self, func: &FunctionDecl, span: Span, function_key: Option<&str>) {
        let is_entry_main = matches!(function_key, None | Some("main")) && func.name == "main";
        if is_entry_main {
            self.validate_main_signature(func, span.clone());
        }
        let function_name = function_key.unwrap_or(&func.name);
        self.validate_generic_param_bounds(
            &func.generic_params,
            span.clone(),
            &format!("Function '{}'", function_name),
        );
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
        self.validate_generic_param_bounds(
            &class.generic_params,
            span.clone(),
            &format!("Class '{}'", class_key),
        );
        let saved_generic_bindings = std::mem::take(&mut self.current_generic_type_bindings);
        self.current_generic_type_bindings = self.make_generic_type_bindings(&class.generic_params);
        self.current_class = Some(class_key.to_string());
        if let Some(parent) = &class.extends {
            let resolved_parent = self
                .resolve_nominal_reference_name(parent)
                .unwrap_or_else(|| parent.clone());
            if self.interfaces.contains_key(&resolved_parent) {
                self.error(
                    format!("Class '{}' cannot extend interface '{}'", class_key, parent),
                    span.clone(),
                );
            } else if !self.classes.contains_key(&resolved_parent) {
                self.error(
                    format!("Class '{}' extends unknown class '{}'", class_key, parent),
                    span.clone(),
                );
            } else if self.is_same_or_subclass_of(&resolved_parent, class_key) {
                self.error(
                    format!(
                        "Inheritance cycle detected: '{}' cannot extend '{}'",
                        class_key, parent
                    ),
                    span.clone(),
                );
            } else {
                self.check_class_visibility(&resolved_parent, span.clone());
            }
        }

        for field in &class.fields {
            let ty = self.resolve_type(&field.ty);
            self.check_type_visibility(&ty, span.clone());
        }

        for interface_name in &class.implements {
            let resolved_interface = self
                .resolve_nominal_reference_name(interface_name)
                .unwrap_or_else(|| interface_name.clone());
            if !self.interfaces.contains_key(&resolved_interface) {
                self.error(
                    format!(
                        "Class '{}' implements unknown interface '{}'",
                        class_key, interface_name
                    ),
                    span.clone(),
                );
            }
        }

        let mut required_methods: HashMap<String, (String, FuncSig)> = HashMap::new();
        for interface_name in &class.implements {
            let resolved_interface = self
                .resolve_nominal_reference_name(interface_name)
                .unwrap_or_else(|| interface_name.clone());
            let mut methods = HashMap::new();
            let mut visited = std::collections::HashSet::new();
            self.collect_interface_methods(&resolved_interface, &mut methods, &mut visited);
            for (method_name, required_sig) in methods {
                if let Some((existing_interface, existing_sig)) = required_methods.get(&method_name)
                {
                    if !self.signatures_mutually_compatible(existing_sig, &required_sig) {
                        self.error(
                            format!(
                                "Class '{}' implements incompatible interface requirements for method '{}' from '{}' and '{}'",
                                class_key, method_name, existing_interface, resolved_interface
                            ),
                            span.clone(),
                        );
                    }
                } else {
                    required_methods
                        .insert(method_name, (resolved_interface.clone(), required_sig));
                }
            }
        }
        for (method_name, (_, required_sig)) in required_methods {
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
            if !self.signatures_mutually_compatible(&required_sig, &actual_sig) {
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
            self.validate_generic_param_bounds(
                &method.generic_params,
                span.clone(),
                &format!("Method '{}.{}'", class_key, method.name),
            );
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
                let value_type = self.check_expr_with_expected_type(
                    &value.node,
                    value.span.clone(),
                    Some(&declared_type),
                );

                // Check type compatibility. If the value is an if-expression that already
                // produced a branch mismatch diagnostic, avoid cascading a second local
                // assignment mismatch for the same root cause.
                let suppress_assignment_mismatch = matches!(&value.node, Expr::IfExpr { .. })
                    && !self.types_compatible(&declared_type, &value_type)
                    && self.errors.iter().any(|error| {
                        error.message.contains("If expression branch type mismatch")
                            && error.span == value.span
                    });

                if !self.types_compatible(&declared_type, &value_type)
                    && !suppress_assignment_mismatch
                {
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
                let value_type = self.check_expr_with_expected_type(
                    &value.node,
                    value.span.clone(),
                    Some(&target_type),
                );

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
                let expected_return_type = self.current_return_type.clone();
                let return_type = expr
                    .as_ref()
                    .map(|e| {
                        self.check_expr_with_expected_type(
                            &e.node,
                            e.span.clone(),
                            expected_return_type.as_ref(),
                        )
                    })
                    .unwrap_or(ResolvedType::None);

                if self.current_async_return_type.is_some() {
                    self.merge_async_return_type(&return_type, span.clone());
                    return;
                }

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
                let iter_item_type = Self::peel_reference_type(&iter_type);

                // Determine element type
                let elem_type = match iter_item_type {
                    ResolvedType::List(inner) => (**inner).clone(),
                    ResolvedType::Range(inner) => (**inner).clone(),
                    ResolvedType::String => ResolvedType::Char,
                    ResolvedType::Integer => ResolvedType::Integer,
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
                let loop_var_type = var_type
                    .as_ref()
                    .map(|declared| self.resolve_type(declared))
                    .unwrap_or(elem_type);
                self.declare_variable(var, loop_var_type, false, span);
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
                    match &var.ty {
                        ResolvedType::MutRef(_) => {}
                        ResolvedType::Ref(_) => {
                            self.error(
                                format!("Cannot assign through immutable reference '{}'", name),
                                span,
                            );
                        }
                        _ if !var.mutable => {
                            self.error_with_hint(
                                format!("Cannot assign to immutable variable '{}'", name),
                                span,
                                "Consider declaring with 'mut'".to_string(),
                            );
                        }
                        _ => {}
                    }
                }
            }
            Expr::Field { object, .. } | Expr::Index { object, .. } => {
                self.check_assignment_target_mutability(&object.node, span);
            }
            Expr::Deref(inner) => {
                let inner_type = self.check_expr(&inner.node, inner.span.clone());
                match inner_type {
                    ResolvedType::MutRef(_) => {}
                    ResolvedType::Ref(_) => {
                        let message = match &inner.node {
                            Expr::Ident(name) => {
                                format!("Cannot assign through immutable reference '{}'", name)
                            }
                            _ => "Cannot assign through immutable reference".to_string(),
                        };
                        self.error(message, span);
                    }
                    _ => {}
                }
            }
            Expr::This => {}
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
                if self
                    .common_compatible_type(expected_type, &lit_type)
                    .is_none()
                {
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
                let has_some = arms.iter().any(|arm| match &arm.pattern {
                    Pattern::Variant(name, _) => {
                        name.rsplit('.').next().is_some_and(|leaf| leaf == "Some")
                            || self
                                .resolve_import_alias_variant(name)
                                .is_some_and(|(owner_enum, variant)| owner_enum == "Option" && variant == "Some")
                    }
                    _ => false,
                });
                let has_none = arms.iter().any(|arm| match &arm.pattern {
                    Pattern::Variant(name, _) => {
                        name.rsplit('.').next().is_some_and(|leaf| leaf == "None")
                            || self
                                .resolve_import_alias_variant(name)
                                .is_some_and(|(owner_enum, variant)| owner_enum == "Option" && variant == "None")
                    }
                    _ => false,
                });
                has_some && has_none
            }
            ResolvedType::Result(_, _) => {
                let has_ok = arms.iter().any(|arm| match &arm.pattern {
                    Pattern::Variant(name, _) => {
                        name.rsplit('.').next().is_some_and(|leaf| leaf == "Ok")
                            || self
                                .resolve_import_alias_variant(name)
                                .is_some_and(|(owner_enum, variant)| owner_enum == "Result" && variant == "Ok")
                    }
                    _ => false,
                });
                let has_err = arms.iter().any(|arm| match &arm.pattern {
                    Pattern::Variant(name, _) => {
                        name.rsplit('.').next().is_some_and(|leaf| leaf == "Error")
                            || self
                                .resolve_import_alias_variant(name)
                                .is_some_and(|(owner_enum, variant)| owner_enum == "Result" && variant == "Error")
                    }
                    _ => false,
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
    fn check_expr_with_expected_type(
        &mut self,
        expr: &Expr,
        span: Span,
        expected: Option<&ResolvedType>,
    ) -> ResolvedType {
        if let (Expr::AsyncBlock(body), Some(ResolvedType::Task(expected_inner))) = (expr, expected)
        {
            return self.check_async_block_expr(body, span, Some(expected_inner.as_ref()));
        }
        self.check_expr(expr, span)
    }

    fn check_async_block_expr(
        &mut self,
        body: &Block,
        span: Span,
        expected_inner: Option<&ResolvedType>,
    ) -> ResolvedType {
        let captured_outer_scopes = self.scopes.clone();
        self.enter_scope();
        let mut tail_expr_type = None;

        let saved_return_type = self.current_return_type.clone();
        let saved_async_return_type = self.current_async_return_type.clone();
        self.current_return_type = None;
        self.current_async_return_type = Some(ResolvedType::None);

        for stmt in body {
            match &stmt.node {
                Stmt::Expr(expr) => {
                    tail_expr_type = Some(self.check_expr(&expr.node, expr.span.clone()));
                }
                _ => {
                    tail_expr_type = None;
                    self.check_stmt(&stmt.node, stmt.span.clone());
                }
            }
        }

        let mut return_type = self
            .current_async_return_type
            .clone()
            .unwrap_or(ResolvedType::None);
        if matches!(return_type, ResolvedType::None) {
            if let Some(tail_ty) = tail_expr_type {
                return_type = tail_ty;
            }
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
        self.current_async_return_type = saved_async_return_type;
        self.exit_scope();
        self.check_async_result_type(&return_type, "Async block", span.clone());

        if let Some(expected_inner) = expected_inner {
            if !self.types_compatible(expected_inner, &return_type) {
                self.error(
                    format!(
                        "Async block return type mismatch: expected {}, found {}",
                        expected_inner, return_type
                    ),
                    span,
                );
            }
            return ResolvedType::Task(Box::new(expected_inner.clone()));
        }

        ResolvedType::Task(Box::new(return_type))
    }

    fn check_expr(&mut self, expr: &Expr, span: Span) -> ResolvedType {
        match expr {
            Expr::Literal(lit) => self.literal_type(lit),

            Expr::Ident(name) => {
                if let Some(var) = self.lookup_variable(name) {
                    var.ty.clone()
                } else if let Some(canonical_name) = self
                    .resolve_import_alias_symbol(name)
                    .filter(|canonical_name| self.functions.contains_key(canonical_name))
                {
                    self.function_value_type_or_error(&canonical_name, span)
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

                if matches!(op, BinOp::Div | BinOp::Mod)
                    && matches!(left_type, ResolvedType::Integer)
                    && matches!(right_type, ResolvedType::Integer)
                    && matches!(
                        Self::eval_numeric_const_expr(&right.node),
                        Some(NumericConst::Integer(0))
                    )
                {
                    let message = match op {
                        BinOp::Div => "Integer division by zero",
                        BinOp::Mod => "Integer modulo by zero",
                        _ => unreachable!(),
                    };
                    self.error(message.to_string(), right.span.clone());
                }

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
                let indexed_type = Self::peel_reference_type(&obj_type);

                match indexed_type {
                    ResolvedType::List(inner) => {
                        if !matches!(idx_type, ResolvedType::Integer) {
                            self.error(
                                format!("Index must be Integer, found {}", idx_type),
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
                                format!("Index must be Integer, found {}", idx_type),
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
                self.validate_class_type_argument_bounds(&scoped_ty, span.clone(), "Constructor");

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

                let return_type = self.check_expr(&body.node, body.span.clone());

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
                        let ty = self.check_expr(&e.node, e.span.clone());
                        if !Self::supports_display_scalar(&ty) {
                            self.error(
                                format!(
                                    "String interpolation currently supports Integer, Float, Boolean, String, Char, and None, got {}",
                                    ty
                                ),
                                e.span.clone(),
                            );
                        }
                    }
                }
                ResolvedType::String
            }

            Expr::Try(inner) => {
                let inner_type = self.check_expr(&inner.node, inner.span.clone());
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
                                            err, outer_err
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
                        if let Some(common_type) = self.common_compatible_type(expected, &arm_type)
                        {
                            result_type = Some(common_type);
                        } else {
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

            Expr::AsyncBlock(body) => self.check_async_block_expr(body, span, None),

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

                    if let Some(common_type) = self.common_compatible_type(&then_type, &else_type) {
                        then_type = common_type;
                    } else {
                        self.error(
                            format!(
                                "If expression branch type mismatch: then is {}, else is {}",
                                then_type, else_type
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
                self.validate_class_type_argument_bounds(name, span.clone(), "Type");
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
    ) -> (Vec<(String, ResolvedType)>, ResolvedType, bool) {
        if type_args.is_empty() {
            return (sig.params.clone(), sig.return_type.clone(), true);
        }

        if sig.generic_type_vars.is_empty() {
            self.error(
                format!(
                    "Function '{}' is not generic but called with explicit type arguments",
                    name
                ),
                span,
            );
            return (sig.params.clone(), ResolvedType::Unknown, false);
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
            return (sig.params.clone(), ResolvedType::Unknown, false);
        }

        let mut substitutions: HashMap<usize, ResolvedType> = HashMap::new();
        for (type_var_id, arg) in sig.generic_type_vars.iter().zip(type_args.iter()) {
            let resolved = self.resolve_type(arg);
            self.validate_resolved_type_exists(&resolved, span.clone());
            if !self.type_var_satisfies_bounds(*type_var_id, &resolved) {
                let bounds = self
                    .type_var_bounds
                    .get(type_var_id)
                    .cloned()
                    .unwrap_or_default()
                    .join(", ");
                self.error(
                    format!(
                        "Function '{}' type argument {} does not satisfy bound(s) {}",
                        name, resolved, bounds
                    ),
                    span.clone(),
                );
            }
            substitutions.insert(*type_var_id, resolved);
        }

        let params = sig
            .params
            .iter()
            .map(|(name, ty)| (name.clone(), Self::substitute_type_vars(ty, &substitutions)))
            .collect::<Vec<_>>();
        let return_type = Self::substitute_type_vars(&sig.return_type, &substitutions);
        (params, return_type, true)
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
                    let ty = self.check_expr(&arg.node, arg.span.clone());
                    if !Self::supports_display_scalar(&ty) {
                        self.error(
                            format!(
                                "{}() currently supports Integer, Float, Boolean, String, Char, and None arguments, got {}",
                                name, ty
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
                    if let Some(common_type) = self.common_compatible_type(&t1, &t2) {
                        Some(common_type)
                    } else {
                        self.error(
                            format!(
                                "{}() arguments must have same type: {} vs {}",
                                func_name, t1, t2
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
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(t, ResolvedType::Integer | ResolvedType::Float) {
                        self.error(
                            format!("to_float() requires Integer or Float, got {}", t),
                            span,
                        );
                    }
                }
                Some(ResolvedType::Float)
            }
            "to_int" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !matches!(
                        t,
                        ResolvedType::Integer | ResolvedType::Float | ResolvedType::String
                    ) {
                        self.error(
                            format!("to_int() requires Integer, Float, or String, got {}", t),
                            span,
                        );
                    }
                }
                Some(ResolvedType::Integer)
            }
            "to_string" => {
                self.check_arg_count(name, args, 1, span.clone());
                if !args.is_empty() {
                    let t = self.check_expr(&args[0].node, args[0].span.clone());
                    if !Self::supports_display_scalar(&t) {
                        self.error(
                            format!(
                                "to_string() currently supports Integer, Float, Boolean, String, Char, and None, got {}",
                                t
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
                    if self.common_compatible_type(&t1, &t2).is_none() {
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
        let receiver_type = Self::peel_reference_type(obj_type);
        if !type_args.is_empty() && !matches!(receiver_type, ResolvedType::Class(_)) {
            self.error(
                format!(
                    "Method '{}' on type '{}' does not accept explicit type arguments",
                    method, obj_type
                ),
                span.clone(),
            );
        }

        match receiver_type {
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
                        let idx_type = self.check_expr(&args[0].node, args[0].span.clone());
                        let val_type = self.check_expr(&args[1].node, args[1].span.clone());
                        if !matches!(idx_type, ResolvedType::Integer) {
                            self.error(
                                "List.set() index must be Integer".to_string(),
                                args[0].span.clone(),
                            );
                        } else {
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
                    sig.return_type
                }
                Ok(None) => {
                    self.error(format!("Cannot call method on type {}", obj_type), span);
                    ResolvedType::Unknown
                }
                Err(message) => {
                    self.error(message, span);
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
        match Self::peel_reference_type(obj_type) {
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
            ResolvedType::Unknown => ResolvedType::Unknown,
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
                if self.common_compatible_type(left, right).is_none() {
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
                if let Some(resolved_name) = self.resolve_known_type_name(name) {
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
        if let ResolvedType::TypeVar(id) = expected {
            return self.type_var_satisfies_bounds(*id, actual);
        }
        if matches!(actual, ResolvedType::TypeVar(_)) {
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
            (ResolvedType::Ref(e), ResolvedType::Ref(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::MutRef(e), ResolvedType::MutRef(a)) => {
                self.types_compatible_invariant(e, a)
            }
            (ResolvedType::Ptr(e), ResolvedType::Ptr(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::Box(e), ResolvedType::Box(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::Rc(e), ResolvedType::Rc(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::Arc(e), ResolvedType::Arc(a)) => self.types_compatible_invariant(e, a),
            // Can use &mut T where &T is expected
            (ResolvedType::Ref(e), ResolvedType::MutRef(a)) => {
                self.types_compatible_invariant(e, a)
            }
            // List compatibility
            (ResolvedType::List(e), ResolvedType::List(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::Set(e), ResolvedType::Set(a)) => self.types_compatible_invariant(e, a),
            // Option compatibility
            (ResolvedType::Option(e), ResolvedType::Option(a)) => {
                self.types_compatible_invariant(e, a)
            }
            (ResolvedType::Task(e), ResolvedType::Task(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::Range(e), ResolvedType::Range(a)) => {
                self.types_compatible_invariant(e, a)
            }
            // Result compatibility
            (ResolvedType::Result(e_ok, e_err), ResolvedType::Result(a_ok, a_err)) => {
                self.types_compatible_invariant(e_ok, a_ok)
                    && self.types_compatible_invariant(e_err, a_err)
            }
            // Map compatibility
            (ResolvedType::Map(ek, ev), ResolvedType::Map(ak, av)) => {
                self.types_compatible_invariant(ek, ak) && self.types_compatible_invariant(ev, av)
            }
            (ResolvedType::Function(e_params, e_ret), ResolvedType::Function(a_params, a_ret)) => {
                e_params.len() == a_params.len()
                    && e_params
                        .iter()
                        .zip(a_params.iter())
                        .all(|(e, a)| self.types_compatible(a, e))
                    && self.types_compatible(e_ret, a_ret)
            }
            _ => false,
        }
    }

    fn types_compatible_invariant(&self, expected: &ResolvedType, actual: &ResolvedType) -> bool {
        if expected == actual {
            return true;
        }

        if let ResolvedType::TypeVar(id) = expected {
            return self.type_var_satisfies_bounds(*id, actual);
        }
        if matches!(actual, ResolvedType::TypeVar(_)) {
            return true;
        }
        if matches!(expected, ResolvedType::Unknown) || matches!(actual, ResolvedType::Unknown) {
            return true;
        }

        match (expected, actual) {
            (ResolvedType::Ref(e), ResolvedType::Ref(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::MutRef(e), ResolvedType::MutRef(a)) => {
                self.types_compatible_invariant(e, a)
            }
            (ResolvedType::Ptr(e), ResolvedType::Ptr(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::Box(e), ResolvedType::Box(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::Rc(e), ResolvedType::Rc(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::Arc(e), ResolvedType::Arc(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::Ref(e), ResolvedType::MutRef(a)) => {
                self.types_compatible_invariant(e, a)
            }
            (ResolvedType::List(e), ResolvedType::List(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::Set(e), ResolvedType::Set(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::Option(e), ResolvedType::Option(a)) => {
                self.types_compatible_invariant(e, a)
            }
            (ResolvedType::Task(e), ResolvedType::Task(a)) => self.types_compatible_invariant(e, a),
            (ResolvedType::Range(e), ResolvedType::Range(a)) => {
                self.types_compatible_invariant(e, a)
            }
            (ResolvedType::Result(e_ok, e_err), ResolvedType::Result(a_ok, a_err)) => {
                self.types_compatible_invariant(e_ok, a_ok)
                    && self.types_compatible_invariant(e_err, a_err)
            }
            (ResolvedType::Map(ek, ev), ResolvedType::Map(ak, av)) => {
                self.types_compatible_invariant(ek, ak) && self.types_compatible_invariant(ev, av)
            }
            (ResolvedType::Function(e_params, e_ret), ResolvedType::Function(a_params, a_ret)) => {
                e_params.len() == a_params.len()
                    && e_params
                        .iter()
                        .zip(a_params.iter())
                        .all(|(e, a)| self.types_compatible_invariant(e, a))
                    && self.types_compatible_invariant(e_ret, a_ret)
            }
            _ => false,
        }
    }

    fn common_compatible_type(
        &self,
        left: &ResolvedType,
        right: &ResolvedType,
    ) -> Option<ResolvedType> {
        if self.types_compatible(left, right) {
            return Some(left.clone());
        }
        if self.types_compatible(right, left) {
            return Some(right.clone());
        }
        if left.is_numeric() && right.is_numeric() {
            return Some(
                if matches!(left, ResolvedType::Float) || matches!(right, ResolvedType::Float) {
                    ResolvedType::Float
                } else {
                    ResolvedType::Integer
                },
            );
        }
        None
    }

    fn merge_async_return_type(&mut self, return_type: &ResolvedType, span: Span) {
        let Some(current) = self.current_async_return_type.clone() else {
            return;
        };

        if matches!(current, ResolvedType::None) {
            self.current_async_return_type = Some(return_type.clone());
            return;
        }

        if let Some(common_type) = self.common_compatible_type(&current, return_type) {
            self.current_async_return_type = Some(common_type);
        } else {
            self.error(
                format!(
                    "Mismatching return types in async block: {} vs {}",
                    current, return_type
                ),
                span,
            );
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
                            _ => ResolvedType::Class(s.to_string()),
                        }
                    } else {
                        self.resolve_known_type_name(s)
                            .map(ResolvedType::Class)
                            .unwrap_or_else(|| ResolvedType::Class(s.to_string()))
                    }
                } else {
                    self.resolve_known_type_name(s)
                        .map(ResolvedType::Class)
                        .unwrap_or_else(|| ResolvedType::Class(s.to_string()))
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
        let mut angle_depth = 0usize;
        let mut paren_depth = 0usize;

        for c in s.chars() {
            match c {
                '<' => {
                    angle_depth += 1;
                    current.push(c);
                }
                '>' => {
                    angle_depth = angle_depth.saturating_sub(1);
                    current.push(c);
                }
                '(' => {
                    paren_depth += 1;
                    current.push(c);
                }
                ')' => {
                    paren_depth = paren_depth.saturating_sub(1);
                    current.push(c);
                }
                ',' if angle_depth == 0 && paren_depth == 0 => {
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
    fn supports_extending_namespace_aliased_module_class() {
        let src = r#"
            import Lib as u;
            module Lib {
                class Base {
                    constructor() {}
                    public function greet(): Integer { return 7; }
                }
            }
            class Child extends u.Base {
                constructor() {}
            }
            function main(): Integer {
                c: Child = Child();
                return c.greet();
            }
        "#;
        check_source(src).expect("aliased base class should typecheck");
    }

    #[test]
    fn supports_extending_nested_namespace_aliased_module_class() {
        let src = r#"
            import Lib as u;
            module Lib {
                module Models {
                    class Base {
                        constructor() {}
                        public function greet(): Integer { return 7; }
                    }
                }
            }
            class Child extends u.Models.Base {
                constructor() {}
            }
            function main(): Integer {
                c: Child = Child();
                return c.greet();
            }
        "#;
        check_source(src).expect("nested aliased base class should typecheck");
    }

    #[test]
    fn supports_implementing_namespace_aliased_module_interface() {
        let src = r#"
            import Lib as u;
            module Lib {
                interface Printable {
                    function print_me(): Integer;
                }
            }
            class Book implements u.Printable {
                constructor() {}
                function print_me(): Integer { return 7; }
            }
            function main(): Integer {
                b: Book = Book();
                return b.print_me();
            }
        "#;
        check_source(src).expect("aliased interface should typecheck");
    }

    #[test]
    fn supports_implementing_nested_namespace_aliased_module_interface() {
        let src = r#"
            import Lib as u;
            module Lib {
                module Api {
                    interface Printable {
                        function print_me(): Integer;
                    }
                }
            }
            class Book implements u.Api.Printable {
                constructor() {}
                function print_me(): Integer { return 7; }
            }
            function main(): Integer {
                b: Book = Book();
                return b.print_me();
            }
        "#;
        check_source(src).expect("nested aliased interface should typecheck");
    }

    #[test]
    fn supports_implementing_multiple_namespace_aliased_interfaces() {
        let src = r#"
            import Lib as u;
            module Lib {
                interface Named {
                    function name(): Integer;
                }
                interface Printable {
                    function print_me(): Integer;
                }
            }
            class Book implements u.Named, u.Printable {
                constructor() {}
                function name(): Integer { return 1; }
                function print_me(): Integer { return 2; }
            }
            function main(): Integer {
                b: Book = Book();
                return b.name() + b.print_me();
            }
        "#;
        check_source(src).expect("multiple aliased interfaces should typecheck");
    }

    #[test]
    fn supports_implementing_multiple_nested_namespace_aliased_interfaces() {
        let src = r#"
            import Lib as u;
            module Lib {
                module Api {
                    interface Named {
                        function name(): Integer;
                    }
                    interface Printable {
                        function print_me(): Integer;
                    }
                }
            }
            class Book implements u.Api.Named, u.Api.Printable {
                constructor() {}
                function name(): Integer { return 1; }
                function print_me(): Integer { return 2; }
            }
            function main(): Integer {
                b: Book = Book();
                return b.name() + b.print_me();
            }
        "#;
        check_source(src).expect("multiple nested aliased interfaces should typecheck");
    }

    #[test]
    fn supports_interface_extending_namespace_aliased_interface() {
        let src = r#"
            import Lib as u;
            module Lib {
                interface Named {
                    function name(): Integer;
                }
            }
            interface Printable extends u.Named {
                function print_me(): Integer;
            }
            class Report implements Printable {
                constructor() {}
                function name(): Integer { return 1; }
                function print_me(): Integer { return 2; }
            }
            function main(): Integer {
                r: Report = Report();
                return r.name() + r.print_me();
            }
        "#;
        check_source(src).expect("aliased parent interface should typecheck");
    }

    #[test]
    fn supports_interface_extending_nested_namespace_aliased_interface() {
        let src = r#"
            import Lib as u;
            module Lib {
                module Api {
                    interface Named {
                        function name(): Integer;
                    }
                }
            }
            interface Printable extends u.Api.Named {
                function print_me(): Integer;
            }
            class Report implements Printable {
                constructor() {}
                function name(): Integer { return 1; }
                function print_me(): Integer { return 2; }
            }
            function main(): Integer {
                r: Report = Report();
                return r.name() + r.print_me();
            }
        "#;
        check_source(src).expect("nested aliased parent interface should typecheck");
    }

    #[test]
    fn supports_interface_extending_multiple_namespace_aliased_interfaces() {
        let src = r#"
            import Lib as u;
            module Lib {
                interface Named {
                    function name(): Integer;
                }
                interface Printable {
                    function print_me(): Integer;
                }
            }
            interface Reportable extends u.Named, u.Printable {}
            class Report implements Reportable {
                constructor() {}
                function name(): Integer { return 1; }
                function print_me(): Integer { return 2; }
            }
            function main(): Integer {
                r: Report = Report();
                return r.name() + r.print_me();
            }
        "#;
        check_source(src).expect("multiple aliased parent interfaces should typecheck");
    }

    #[test]
    fn supports_interface_extending_multiple_nested_namespace_aliased_interfaces() {
        let src = r#"
            import Lib as u;
            module Lib {
                module Api {
                    interface Named {
                        function name(): Integer;
                    }
                    interface Printable {
                        function print_me(): Integer;
                    }
                }
            }
            interface Reportable extends u.Api.Named, u.Api.Printable {}
            class Report implements Reportable {
                constructor() {}
                function name(): Integer { return 1; }
                function print_me(): Integer { return 2; }
            }
            function main(): Integer {
                r: Report = Report();
                return r.name() + r.print_me();
            }
        "#;
        check_source(src).expect("multiple nested aliased parent interfaces should typecheck");
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
    fn supports_module_local_interface_implements() {
        let src = r#"
            module M {
                interface Named {
                    function name(): Integer;
                }
                class Book implements Named {
                    constructor() {}
                    function name(): Integer { return 1; }
                }
            }
            function main(): Integer { return 0; }
        "#;
        check_source(src).expect("module-local interface implements should typecheck");
    }

    #[test]
    fn supports_module_local_nested_interface_implements() {
        let src = r#"
            module M {
                module Api {
                    interface Named {
                        function name(): Integer;
                    }
                }
                class Book implements Api.Named {
                    constructor() {}
                    function name(): Integer { return 1; }
                }
            }
            function main(): Integer { return 0; }
        "#;
        check_source(src).expect("module-local nested interface implements should typecheck");
    }

    #[test]
    fn supports_module_local_interface_extends() {
        let src = r#"
            module M {
                interface Named {
                    function name(): Integer;
                }
                interface Printable extends Named {
                    function print_me(): Integer;
                }
                class Report implements Printable {
                    constructor() {}
                    function name(): Integer { return 1; }
                    function print_me(): Integer { return 2; }
                }
            }
            function main(): Integer { return 0; }
        "#;
        check_source(src).expect("module-local interface extends should typecheck");
    }

    #[test]
    fn supports_module_local_nested_interface_extends() {
        let src = r#"
            module M {
                module Api {
                    interface Named {
                        function name(): Integer;
                    }
                }
                interface Printable extends Api.Named {
                    function print_me(): Integer;
                }
                class Report implements Printable {
                    constructor() {}
                    function name(): Integer { return 1; }
                    function print_me(): Integer { return 2; }
                }
            }
            function main(): Integer { return 0; }
        "#;
        check_source(src).expect("module-local nested interface extends should typecheck");
    }

    #[test]
    fn rejects_unknown_function_generic_bound() {
        let src = r#"
            function render<T extends Missing>(value: T): None {
                return None;
            }
            function main(): Integer { return 0; }
        "#;
        let errors = check_source(src).expect_err("unknown generic bound should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains(
                "Function 'render' generic parameter 'T' extends unknown interface 'Missing'"
            ),
            "{joined}"
        );
    }

    #[test]
    fn rejects_non_interface_function_generic_bound() {
        let src = r#"
            class Secret { constructor() {} }
            function render<T extends Secret>(value: T): None {
                return None;
            }
            function main(): Integer { return 0; }
        "#;
        let errors = check_source(src).expect_err("class generic bound should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Function 'render' generic parameter 'T' must use an interface bound, found 'Secret'"),
            "{joined}"
        );
    }

    #[test]
    fn rejects_unknown_class_generic_bound() {
        let src = r#"
            class Box<T extends Missing> {
                value: Integer;
            }
            function main(): Integer { return 0; }
        "#;
        let errors = check_source(src).expect_err("unknown class generic bound should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined
                .contains("Class 'Box' generic parameter 'T' extends unknown interface 'Missing'"),
            "{joined}"
        );
    }

    #[test]
    fn rejects_unknown_enum_generic_bound() {
        let src = r#"
            enum Maybe<T extends Missing> {
                Some(value: T),
                Empty
            }
            function main(): Integer { return 0; }
        "#;
        let errors = check_source(src).expect_err("unknown enum generic bound should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined
                .contains("Enum 'Maybe' generic parameter 'T' extends unknown interface 'Missing'"),
            "{joined}"
        );
    }

    #[test]
    fn rejects_unknown_interface_generic_bound() {
        let src = r#"
            interface Renderable<T extends Missing> {
                function render(value: T): None;
            }
            function main(): Integer { return 0; }
        "#;
        let errors = check_source(src).expect_err("unknown interface generic bound should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains(
                "Interface 'Renderable' generic parameter 'T' extends unknown interface 'Missing'"
            ),
            "{joined}"
        );
    }

    #[test]
    fn rejects_explicit_function_type_arg_that_violates_interface_bound() {
        let src = r#"
            interface Named { function name(): Integer; }
            class Plain { constructor() {} }
            function render<T extends Named>(value: T): Integer {
                return 1;
            }
            function main(): Integer {
                return render<Plain>(Plain());
            }
        "#;
        let errors =
            check_source(src).expect_err("explicit generic arg violating bound should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined
                .contains("Function 'render' type argument Plain does not satisfy bound(s) Named"),
            "{joined}"
        );
    }

    #[test]
    fn rejects_inferred_function_arg_that_violates_interface_bound() {
        let src = r#"
            interface Named { function name(): Integer; }
            class Plain { constructor() {} }
            function render<T extends Named>(value: T): Integer {
                return 1;
            }
            function main(): Integer {
                return render(Plain());
            }
        "#;
        let errors =
            check_source(src).expect_err("inferred generic arg violating bound should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Argument type mismatch: expected ?T"),
            "{joined}"
        );
    }

    #[test]
    fn allows_method_calls_through_generic_interface_bound() {
        let src = r#"
            interface Named { function name(): Integer; }
            class Person implements Named {
                constructor() {}
                function name(): Integer { return 7; }
            }
            function read_name<T extends Named>(value: T): Integer {
                return value.name();
            }
            function main(): Integer {
                return read_name(Person());
            }
        "#;
        check_source(src).expect("bounded generic interface method calls should typecheck");
    }

    #[test]
    fn rejects_ambiguous_bounded_generic_method_signatures() {
        let src = r#"
            interface A { function render(): Integer; }
            interface B { function render(): String; }
            function read<T extends A, B>(value: T): Integer {
                return value.render();
            }
            function main(): Integer { return 0; }
        "#;
        let errors = check_source(src)
            .expect_err("conflicting bounded generic method signatures should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains(
                "Generic bound method 'B.render' has incompatible signatures across bounds"
            ),
            "{joined}"
        );
    }

    #[test]
    fn rejects_interface_inheriting_conflicting_parent_method_signatures() {
        let src = r#"
            interface A { function render(): Integer; }
            interface B { function render(): String; }
            interface C extends A, B {}
            function main(): Integer { return 0; }
        "#;
        let errors =
            check_source(src).expect_err("conflicting parent interface methods should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains(
                "Interface 'C' inherits incompatible signatures for method 'render' from 'A' and 'B'"
            ),
            "{joined}"
        );
    }

    #[test]
    fn rejects_interface_overriding_parent_method_with_incompatible_signature() {
        let src = r#"
            interface A { function render(): Integer; }
            interface C extends A {
                function render(): String;
            }
            function main(): Integer { return 0; }
        "#;
        let errors = check_source(src).expect_err("incompatible interface override should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains(
                "Interface 'C.render' overrides inherited method from 'A' with an incompatible signature"
            ),
            "{joined}"
        );
    }

    #[test]
    fn rejects_class_implementing_conflicting_interface_method_requirements() {
        let src = r#"
            interface A { function render(): Integer; }
            interface B { function render(): String; }
            class Both implements A, B {
                constructor() {}
                function render(): Integer { return 1; }
            }
            function main(): Integer { return 0; }
        "#;
        let errors =
            check_source(src).expect_err("conflicting implemented interface methods should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains(
                "Class 'Both' implements incompatible interface requirements for method 'render' from 'A' and 'B'"
            ),
            "{joined}"
        );
    }

    #[test]
    fn rejects_interface_implementation_with_narrower_parameter_type() {
        let src = r#"
            class Animal { constructor() {} }
            class Dog extends Animal { constructor() {} }
            interface Feeder { function feed(animal: Animal): Integer; }
            class Kennel implements Feeder {
                constructor() {}
                function feed(animal: Dog): Integer { return 1; }
            }
            function main(): Integer { return 0; }
        "#;
        let errors =
            check_source(src).expect_err("narrower interface implementation parameter should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Method 'Kennel.feed' does not match interface signature"),
            "{joined}"
        );
    }

    #[test]
    fn rejects_constructor_type_arg_that_violates_interface_bound() {
        let src = r#"
            interface Named { function name(): Integer; }
            class Plain { constructor() {} }
            class Box<T extends Named> {
                value: Integer;
                constructor() { this.value = 1; }
            }
            function main(): Integer {
                bad: Box<Plain> = Box<Plain>();
                return bad.value;
            }
        "#;
        let errors =
            check_source(src).expect_err("constructor generic arg violating bound should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Constructor type argument Plain does not satisfy bound(s) Named"),
            "{joined}"
        );
    }

    #[test]
    fn rejects_annotation_only_generic_type_arg_that_violates_interface_bound() {
        let src = r#"
            interface Named { function name(): Integer; }
            class Plain { constructor() {} }
            class Box<T extends Named> {
                value: Integer;
                constructor() { this.value = 1; }
            }
            function main(): Integer {
                bad: Box<Plain> = Box<Plain>();
                return 0;
            }
        "#;
        let errors =
            check_source(src).expect_err("annotation-only generic arg violating bound should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Type type argument Plain does not satisfy bound(s) Named"),
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
    fn borrowed_read_accesses_typecheck() {
        let src = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function get(): Integer { return this.value; }
            }

            function main(): None {
                s: String = "ab";
                xs: List<Integer> = List<Integer>();
                xs.push(40);
                m: Map<String, Integer> = Map<String, Integer>();
                m.set("k", 41);
                b: Boxed = Boxed(42);

                rs: &String = &s;
                rxs: &List<Integer> = &xs;
                rm: &Map<String, Integer> = &m;
                rb: &Boxed = &b;

                a: Integer = rb.value;
                c: Integer = rb.get();
                d: Char = rs[1];
                e: Integer = rxs[0];
                f: Integer = rxs.get(0);
                g: Integer = rxs.length();
                h: Integer = rm["k"];
                i: Integer = rm.get("k");
                j: Boolean = rm.contains("k");
                return None;
            }
        "#;
        check_source(src).expect("borrowed read accesses should typecheck");
    }

    #[test]
    fn borrowed_mutating_accesses_typecheck() {
        let src = r#"
            class Bag {
                mut xs: List<Integer>;
                mut m: Map<String, Integer>;
                mut s: Set<Integer>;
                mut r: Range<Integer>;

                constructor() {
                    this.xs = List<Integer>();
                    this.m = Map<String, Integer>();
                    this.s = Set<Integer>();
                    this.r = range(0, 3);
                }
            }

            function main(): None {
                mut xs: List<Integer> = List<Integer>();
                mut m: Map<String, Integer> = Map<String, Integer>();
                mut s: Set<Integer> = Set<Integer>();
                mut r: Range<Integer> = range(0, 2);
                mut bag: Bag = Bag();

                rxs: &mut List<Integer> = &mut xs;
                rm: &mut Map<String, Integer> = &mut m;
                rs: &mut Set<Integer> = &mut s;
                rr: &mut Range<Integer> = &mut r;
                rb: &mut Bag = &mut bag;

                rxs.push(1);
                rxs.set(0, 2);
                value: Integer = rxs.pop();
                rm.set("k", value);
                rs.add(value);
                rs.remove(value);
                x: Integer = rr.next();
                require(x == 0);

                rb.xs.push(3);
                rb.m.set("k2", 4);
                rb.s.add(5);
                y: Integer = rb.r.next();
                require(y == 0);
                return None;
            }
        "#;
        check_source(src).expect("borrowed mutating accesses should typecheck");
    }

    #[test]
    fn borrowed_mutating_index_assignments_typecheck() {
        let src = r#"
            class Bag {
                mut xs: List<Integer>;
                mut m: Map<String, Integer>;

                constructor() {
                    this.xs = List<Integer>();
                    this.m = Map<String, Integer>();
                }
            }

            function main(): None {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                mut m: Map<String, Integer> = Map<String, Integer>();
                mut bag: Bag = Bag();

                rxs: &mut List<Integer> = &mut xs;
                rm: &mut Map<String, Integer> = &mut m;
                rb: &mut Bag = &mut bag;

                rxs[0] = 2;
                rm["k"] = 7;
                rb.xs.push(1);
                rb.xs[0] = 3;
                rb.m["k2"] = 4;
                return None;
            }
        "#;
        check_source(src).expect("borrowed mutating index assignments should typecheck");
    }

    #[test]
    fn immutable_reference_index_assignment_rejected() {
        let src = r#"
            function main(): None {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                mut m: Map<String, Integer> = Map<String, Integer>();

                rxs: &List<Integer> = &xs;
                rm: &Map<String, Integer> = &m;

                rxs[0] = 2;
                rm["k"] = 7;
                return None;
            }
        "#;
        let errors = check_source(src)
            .expect_err("immutable reference index assignments should be rejected");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Cannot assign through immutable reference 'rxs'"),
            "{joined}"
        );
        assert!(
            joined.contains("Cannot assign through immutable reference 'rm'"),
            "{joined}"
        );
    }

    #[test]
    fn immutable_reference_deref_assignment_rejected() {
        let src = r#"
            function main(): None {
                mut x: Integer = 1;
                r: &Integer = &x;
                *r = 2;
                return None;
            }
        "#;
        let errors =
            check_source(src).expect_err("immutable reference deref assignment should be rejected");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Cannot assign through immutable reference 'r'"),
            "{joined}"
        );
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
    fn qualified_module_type_paths_typecheck_against_mangled_symbols() {
        let src = r#"
            module util {
                class Item {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                    function get(): Integer { return this.value; }
                }
                function mk(): Item { return Item(7); }
            }

            function main(): Integer {
                item: util.Item = util.mk();
                return item.get();
            }
        "#;

        check_source(src).expect("qualified module type paths should resolve to mangled symbols");
    }

    #[test]
    fn user_defined_generic_classes_named_like_builtins_typecheck() {
        let src = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function mk(value: Integer): Box<Integer> {
                return Box<Integer>(value);
            }

            function main(): Integer {
                return mk(42).get();
            }
        "#;

        check_source(src)
            .expect("user-defined generic classes named like built-ins should typecheck");
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
    fn accepts_async_block_tail_expression_type() {
        let src = r#"
            function main(): None {
                task: Task<Integer> = async { 7 };
                value: Integer = await(task);
                return None;
            }
        "#;
        check_source(src).expect("async block tail expression should infer Task<Integer>");
    }

    #[test]
    fn accepts_async_block_unary_tail_expression_type() {
        let src = r#"
            function main(): None {
                task: Task<Integer> = async { -7 };
                flag: Task<Boolean> = async { !false };
                a: Integer = await(task);
                b: Boolean = await(flag);
                return None;
            }
        "#;
        check_source(src).expect("async block unary tail expressions should infer correct Task<T>");
    }

    #[test]
    fn accepts_async_block_binary_tail_expression_type() {
        let src = r#"
            function main(): None {
                sum_task: Task<Integer> = async { 2 + 5 };
                cmp_task: Task<Boolean> = async { 2 + 5 == 7 };
                a: Integer = await(sum_task);
                b: Boolean = await(cmp_task);
                return None;
            }
        "#;
        check_source(src)
            .expect("async block binary tail expressions should infer correct Task<T>");
    }

    #[test]
    fn accepts_async_block_function_value_tail_expression_type() {
        let src = r#"
            function inc(x: Integer): Integer { return x + 1; }

            function main(): None {
                task: Task<(Integer) -> Integer> = async { inc };
                f: (Integer) -> Integer = await(task);
                value: Integer = f(1);
                return None;
            }
        "#;
        check_source(src)
            .expect("async block function-value tail expressions should infer correct Task<T>");
    }

    #[test]
    fn accepts_async_block_unit_enum_value_tail_expression_type() {
        let src = r#"
            enum E { A, B }

            function main(): None {
                task: Task<E> = async { E.A };
                value: E = await(task);
                return None;
            }
        "#;
        check_source(src)
            .expect("async block unit-enum tail expressions should infer correct Task<T>");
    }

    #[test]
    fn accepts_builtin_and_reference_async_block_tail_expression_types() {
        let src = r#"
            import std.string.*;
            import std.io.println;

            function main(): None {
                some_task: Task<Option<Integer>> = async { Option.some(7) };
                none_task: Task<Option<Integer>> = async { Option.none() };
                ok_task: Task<Result<Integer, String>> = async { Result.ok(7) };
                err_task: Task<Result<Integer, String>> = async { Result.error("boom") };
                len_task: Task<Integer> = async { Str.len("abc") };
                compare_task: Task<Integer> = async { Str.compare("a", "a") };
                concat_task: Task<String> = async { Str.concat("a", "b") };
                upper_task: Task<String> = async { Str.upper("ab") };
                lower_task: Task<String> = async { Str.lower("AB") };
                trim_task: Task<String> = async { Str.trim("  ok  ") };
                contains_task: Task<Boolean> = async { Str.contains("abc", "b") };
                starts_task: Task<Boolean> = async { Str.startsWith("abc", "a") };
                ends_task: Task<Boolean> = async { Str.endsWith("abc", "c") };
                string_task: Task<String> = async { to_string(7) };
                print_task: Task<None> = async { println("hi") };
                require_task: Task<None> = async { require(true) };
                range_task: Task<Range<Integer>> = async { range(0, 3) };
                lambda_task: Task<(Integer) -> Integer> = async { |x: Integer| x + 1 };
                if_task: Task<Integer> = async { if (true) { Str.len("abc") } else { Str.len("ab") } };
                match_task: Task<String> = async {
                    match (1) {
                        1 => { to_string(7) }
                        _ => { to_string(8) }
                    }
                };

                require(await(some_task).unwrap() == 7);
                require(await(none_task).is_none());
                require(await(ok_task).unwrap() == 7);
                require(await(err_task).is_error());
                require(await(len_task) == 3);
                require(await(compare_task) == 0);
                require(await(concat_task) == "ab");
                require(await(upper_task) == "AB");
                require(await(lower_task) == "ab");
                require(await(trim_task) == "ok");
                require(await(contains_task));
                require(await(starts_task));
                require(await(ends_task));
                require(await(string_task) == "7");
                await(print_task);
                await(require_task);
                require(await(range_task).has_next());
                require((await(lambda_task))(1) == 2);
                require(await(if_task) == 3);
                require(await(match_task) == "7");
                return None;
            }
        "#;
        check_source(src)
            .expect("builtin and reference async block tails should infer correct Task<T>");
    }

    #[test]
    fn accepts_function_types_inside_generic_class_arguments() {
        let src = r#"
            class Holder<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function add(x: Integer, y: Integer): Integer { return x + y; }

            function main(): None {
                holder: Holder<(Integer, Integer) -> Integer> = Holder<(Integer, Integer) -> Integer>(add);
                f: (Integer, Integer) -> Integer = holder.get();
                return None;
            }
        "#;
        check_source(src).expect("generic classes should preserve function-type arguments");
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

    #[test]
    fn range_rejects_constant_integer_zero_step_expression() {
        let src = r#"
            function main(): None {
                r: Range<Integer> = range(0, 3, 1 - 1);
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("constant integer zero step should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("range() step cannot be 0"), "{joined}");
    }

    #[test]
    fn range_rejects_constant_float_zero_step_expression() {
        let src = r#"
            function main(): None {
                r: Range<Float> = range(0.0, 3.0, 0.5 - 0.5);
                return None;
            }
        "#;
        let errors = check_source(src).expect_err("constant float zero step should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("range() step cannot be 0"), "{joined}");
    }

    #[test]
    fn integer_division_rejects_constant_zero_divisor() {
        let src = r#"
            function main(): Integer {
                return 6 / (2 - 2);
            }
        "#;
        let errors = check_source(src).expect_err("constant integer zero divisor should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Integer division by zero"), "{joined}");
    }

    #[test]
    fn integer_modulo_rejects_constant_zero_divisor() {
        let src = r#"
            function main(): Integer {
                return 6 % (2 - 2);
            }
        "#;
        let errors =
            check_source(src).expect_err("constant integer zero modulo divisor should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Integer modulo by zero"), "{joined}");
    }

    #[test]
    fn await_timeout_rejects_negative_constant_literal() {
        let src = r#"
            async function work(): Integer {
                return 1;
            }

            function main(): Integer {
                maybe: Option<Integer> = work().await_timeout(-1);
                if (maybe.is_some()) { return 1; }
                return 0;
            }
        "#;
        let errors = check_source(src).expect_err("negative await_timeout literal should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Task.await_timeout() timeout must be non-negative"),
            "{joined}"
        );
    }

    #[test]
    fn await_timeout_rejects_negative_constant_expression() {
        let src = r#"
            async function work(): Integer {
                return 1;
            }

            function main(): Integer {
                maybe: Option<Integer> = work().await_timeout(1 - 2);
                if (maybe.is_some()) { return 1; }
                return 0;
            }
        "#;
        let errors = check_source(src).expect_err("negative await_timeout expression should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Task.await_timeout() timeout must be non-negative"),
            "{joined}"
        );
    }

    #[test]
    fn time_sleep_rejects_negative_constant_literal() {
        let src = r#"
            import std.time.*;

            function main(): Integer {
                Time.sleep(-1);
                return 0;
            }
        "#;
        let errors = check_source(src).expect_err("negative Time.sleep literal should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Time.sleep() milliseconds must be non-negative"),
            "{joined}"
        );
    }

    #[test]
    fn time_sleep_rejects_negative_constant_expression() {
        let src = r#"
            import std.time.*;

            function main(): Integer {
                Time.sleep(1 - 2);
                return 0;
            }
        "#;
        let errors = check_source(src).expect_err("negative Time.sleep expression should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Time.sleep() milliseconds must be non-negative"),
            "{joined}"
        );
    }

    #[test]
    fn args_get_rejects_negative_constant_literal() {
        let src = r#"
            import std.args.*;

            function main(): Integer {
                value: String = Args.get(-1);
                return 0;
            }
        "#;
        let errors = check_source(src).expect_err("negative Args.get literal should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Args.get() index cannot be negative"),
            "{joined}"
        );
    }

    #[test]
    fn read_line_imported_from_std_io_typechecks() {
        let src = r#"
            import std.io.*;

            function main(): Integer {
                line: String = read_line();
                return 0;
            }
        "#;
        check_source(src).expect("read_line should typecheck from std.io wildcard import");
    }

    #[test]
    fn list_get_rejects_negative_constant_index() {
        let src = r#"
            function main(): Integer {
                values: List<Integer> = List<Integer>();
                values.push(1);
                return values.get(-1);
            }
        "#;
        let errors = check_source(src).expect_err("negative list.get index should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("List.get() index cannot be negative"),
            "{joined}"
        );
    }

    #[test]
    fn list_index_rejects_negative_constant_index() {
        let src = r#"
            function main(): Integer {
                values: List<Integer> = List<Integer>();
                values.push(1);
                return values[-1];
            }
        "#;
        let errors = check_source(src).expect_err("negative list index should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("List index cannot be negative"), "{joined}");
    }

    #[test]
    fn list_set_rejects_negative_constant_index() {
        let src = r#"
            function main(): Integer {
                values: List<Integer> = List<Integer>();
                values.push(1);
                values.set(-1, 2);
                return 0;
            }
        "#;
        let errors = check_source(src).expect_err("negative list.set index should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("List.set() index cannot be negative"),
            "{joined}"
        );
    }

    #[test]
    fn string_index_rejects_negative_constant_index() {
        let src = r#"
            function main(): Char {
                return "abc"[-1];
            }
        "#;
        let errors = check_source(src).expect_err("negative string index should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("String index cannot be negative"),
            "{joined}"
        );
    }

    #[test]
    fn string_index_rejects_constant_out_of_bounds_literal_index() {
        let src = r#"
            function bad(): Char {
                return "abc"[5];
            }

            function main(): Integer {
                c: Char = bad();
                return 0;
            }
        "#;
        let errors =
            check_source(src).expect_err("constant out-of-bounds string index should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("String index out of bounds"), "{joined}");
    }

    #[test]
    fn string_index_rejects_unicode_literal_index_past_char_len() {
        let src = r#"
            function bad(): Char {
                return "🚀"[1];
            }

            function main(): Integer {
                c: Char = bad();
                return 0;
            }
        "#;
        let errors =
            check_source(src).expect_err("unicode string literal char index past len should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("String index out of bounds"), "{joined}");
    }

    #[test]
    fn main_rejects_non_integer_or_none_return_type() {
        let src = r#"
            function main(): String {
                return "oops";
            }
        "#;
        let errors = check_source(src).expect_err("main string return type should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("main() must return None or Integer"),
            "{joined}"
        );
    }

    #[test]
    fn main_rejects_parameters() {
        let src = r#"
            function main(x: Integer): Integer {
                return x;
            }
        "#;
        let errors = check_source(src).expect_err("main parameters should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("main() cannot declare parameters"),
            "{joined}"
        );
    }

    #[test]
    fn main_rejects_async_entrypoint() {
        let src = r#"
            async function main(): Task<Integer> {
                return 1;
            }
        "#;
        let errors = check_source(src).expect_err("async main should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("main() cannot be async; use a synchronous main() entrypoint"),
            "{joined}"
        );
    }

    #[test]
    fn try_on_result_requires_result_return_context() {
        let src = r#"
            function choose(): Result<Integer, String> { return Result.ok(1); }
            function helper(): Integer {
                value: Integer = choose()?;
                return value;
            }
            function main(): Integer {
                return helper();
            }
        "#;
        let errors =
            check_source(src).expect_err("try on Result outside Result return context should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("'?' on Result requires the enclosing function to return Result"),
            "{joined}"
        );
    }

    #[test]
    fn try_on_option_requires_option_return_context() {
        let src = r#"
            function choose(): Option<Integer> { return Option.some(1); }
            function helper(): Result<Integer, String> {
                value: Integer = choose()?;
                return Result.ok(value);
            }
            function main(): Integer {
                return helper().unwrap();
            }
        "#;
        let errors = check_source(src)
            .expect_err("try on Option inside Result-returning function should fail");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("'?' on Option requires the enclosing function to return Option"),
            "{joined}"
        );
    }

    #[test]
    fn try_inside_lambda_does_not_inherit_outer_result_context() {
        let src = r#"
            function choose(): Result<Integer, String> { return Result.ok(1); }
            function wrap(): Result<Integer, String> {
                f: () -> Integer = () => choose()?;
                return Result.ok(f());
            }
            function main(): Integer {
                return wrap().unwrap();
            }
        "#;
        let errors = check_source(src)
            .expect_err("try inside lambda should not inherit outer Result return context");
        let joined = errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("'?' on Result requires the enclosing function to return Result"),
            "{joined}"
        );
    }

    #[test]
    fn seeded_check_supports_interface_extending_namespace_aliased_interface() {
        let src = r#"
            import Lib as u;
            module Lib {
                interface Named {
                    function name(): Integer;
                }
            }
            interface Printable extends u.Named {
                function print_me(): Integer;
            }
            class Report implements Printable {
                constructor() {}
                function name(): Integer { return 1; }
                function print_me(): Integer { return 2; }
            }
            function main(): Integer {
                r: Report = Report();
                return r.name() + r.print_me();
            }
        "#;
        let tokens = tokenize(src).expect("tokenize seeded alias interface source");
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse_program()
            .expect("parse seeded alias interface source");
        let mut type_checker = TypeChecker::new(src.to_string());
        type_checker
            .check_with_effect_seeds(&program, &HashMap::new(), &HashMap::new())
            .expect("seeded check should support aliased parent interface");
    }

    #[test]
    fn seeded_check_supports_interface_extending_nested_namespace_aliased_interface() {
        let src = r#"
            import Lib as u;
            module Lib {
                module Api {
                    interface Named {
                        function name(): Integer;
                    }
                }
            }
            interface Printable extends u.Api.Named {
                function print_me(): Integer;
            }
            class Report implements Printable {
                constructor() {}
                function name(): Integer { return 1; }
                function print_me(): Integer { return 2; }
            }
            function main(): Integer {
                r: Report = Report();
                return r.name() + r.print_me();
            }
        "#;
        let tokens = tokenize(src).expect("tokenize seeded nested alias source");
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse_program()
            .expect("parse seeded nested alias source");
        let mut type_checker = TypeChecker::new(src.to_string());
        type_checker
            .check_with_effect_seeds(&program, &HashMap::new(), &HashMap::new())
            .expect("seeded check should support nested aliased parent interface");
    }

    #[test]
    fn seeded_check_supports_interface_extending_multiple_namespace_aliased_interfaces() {
        let src = r#"
            import Lib as u;
            module Lib {
                interface Named {
                    function name(): Integer;
                }
                interface Printable {
                    function print_me(): Integer;
                }
            }
            interface Reportable extends u.Named, u.Printable {}
            class Report implements Reportable {
                constructor() {}
                function name(): Integer { return 1; }
                function print_me(): Integer { return 2; }
            }
            function main(): Integer {
                r: Report = Report();
                return r.name() + r.print_me();
            }
        "#;
        let tokens = tokenize(src).expect("tokenize seeded multi alias interface source");
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse_program()
            .expect("parse seeded multi alias interface source");
        let mut type_checker = TypeChecker::new(src.to_string());
        type_checker
            .check_with_effect_seeds(&program, &HashMap::new(), &HashMap::new())
            .expect("seeded check should support multiple aliased parent interfaces");
    }

    #[test]
    fn seeded_check_supports_interface_extending_multiple_nested_namespace_aliased_interfaces() {
        let src = r#"
            import Lib as u;
            module Lib {
                module Api {
                    interface Named {
                        function name(): Integer;
                    }
                    interface Printable {
                        function print_me(): Integer;
                    }
                }
            }
            interface Reportable extends u.Api.Named, u.Api.Printable {}
            class Report implements Reportable {
                constructor() {}
                function name(): Integer { return 1; }
                function print_me(): Integer { return 2; }
            }
            function main(): Integer {
                r: Report = Report();
                return r.name() + r.print_me();
            }
        "#;
        let tokens = tokenize(src).expect("tokenize seeded multi nested alias interface source");
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse_program()
            .expect("parse seeded multi nested alias interface source");
        let mut type_checker = TypeChecker::new(src.to_string());
        type_checker
            .check_with_effect_seeds(&program, &HashMap::new(), &HashMap::new())
            .expect("seeded check should support multiple nested aliased parent interfaces");
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
            let underline_len = error.span.end.saturating_sub(error.span.start).max(1);
            let available = lines[line_num - 1].len().saturating_sub(underline_start);
            output.push_str(&format!(
                "   \x1b[1;34m|\x1b[0m {}\x1b[1;31m{}\x1b[0m\n",
                " ".repeat(underline_start),
                "^".repeat(underline_len.min(available).max(1))
            ));
        }

        if let Some(hint) = &error.hint {
            output.push_str(&format!("   \x1b[1;34m= help\x1b[0m: {}\n", hint));
        }

        output.push('\n');
    }

    output
}
