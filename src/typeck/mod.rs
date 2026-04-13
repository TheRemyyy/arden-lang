//! Arden Type Checker - Semantic analysis with type inference
//!
//! This module provides:
//! - Type checking and inference
//! - Symbol table management
//! - Scope tracking
//! - Type error reporting with source locations

use crate::ast::*;
use crate::parser::parse_type_source;
use crate::shared::type_name::{format_diagnostic_class_name, split_generic_args_static};
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

    pub fn supports_ordered_comparison_with(&self, other: &ResolvedType) -> bool {
        (self.is_numeric() && other.is_numeric())
            || matches!((self, other), (ResolvedType::Char, ResolvedType::Char))
    }

    pub fn contains_function_type(&self) -> bool {
        match self {
            ResolvedType::Function(_, _) => true,
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
            | ResolvedType::Range(inner) => inner.contains_function_type(),
            ResolvedType::Result(ok, err) | ResolvedType::Map(ok, err) => {
                ok.contains_function_type() || err.contains_function_type()
            }
            _ => false,
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
pub enum FunctionEffectContract {
    Effects(Vec<String>),
    Any,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct VarInfo {
    pub ty: ResolvedType,
    pub mutable: bool,
    pub callable_effects: Option<FunctionEffectContract>,
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
}

/// Enum metadata used for type checking variant constructors and pattern matching
#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub variants: HashMap<String, Vec<ResolvedType>>,
    pub generic_type_vars: Vec<usize>,
}

/// Interface metadata
#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub methods: HashMap<String, FuncSig>,
    pub generic_param_names: Vec<String>,
    pub generic_type_vars: Vec<usize>,
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
    /// Reverse lookup: unqualified function name -> unique canonical name, or None if ambiguous
    function_leaf_names: HashMap<String, Option<String>>,
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
    /// Collected errors
    errors: Vec<TypeError>,
    /// Current function return type (for checking returns)
    current_return_type: Option<ResolvedType>,
    /// Current async-block inferred return type while checking nested explicit returns
    current_async_return_type: Option<ResolvedType>,
    /// Current class context (for visibility checks)
    current_class: Option<String>,
    /// Import aliases (alias -> scoped import paths)
    import_aliases: HashMap<String, Vec<(Option<String>, String)>>,
    /// Current function declared effects
    current_effects: Vec<String>,
    /// Whether current function is declared pure
    current_is_pure: bool,
    /// Whether current function allows any effects
    current_allow_any: bool,
    /// Whether the current function/method declared explicit effect policy attributes.
    current_has_explicit_effects: bool,
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
        ) || matches!(ty, ResolvedType::Option(inner) if Self::supports_display_scalar(inner))
            || matches!(
                ty,
                ResolvedType::Result(ok, err)
                    if Self::supports_display_scalar(ok) && Self::supports_display_scalar(err)
            )
    }

    fn supports_display_expr(&self, expr: &Expr, ty: &ResolvedType) -> bool {
        if Self::supports_display_scalar(ty) {
            return true;
        }

        let Expr::Call { callee, args, .. } = expr else {
            return false;
        };
        let Expr::Field { object, field } = &callee.node else {
            return false;
        };
        let Expr::Ident(owner_name) = &object.node else {
            return false;
        };
        let owner_name = self.resolve_builtin_module_alias(owner_name);

        match (owner_name.as_str(), field.as_str(), ty) {
            ("Option", "none", ResolvedType::Option(_)) => true,
            ("Option", "some", ResolvedType::Option(inner)) => args
                .first()
                .is_some_and(|arg| self.supports_display_expr(&arg.node, inner.as_ref())),
            ("Result", "ok", ResolvedType::Result(ok, _)) => args
                .first()
                .is_some_and(|arg| self.supports_display_expr(&arg.node, ok.as_ref())),
            ("Result", "error", ResolvedType::Result(_, err)) => args
                .first()
                .is_some_and(|arg| self.supports_display_expr(&arg.node, err.as_ref())),
            _ => false,
        }
    }
    fn resolved_type_contains_unknown(ty: &ResolvedType) -> bool {
        match ty {
            ResolvedType::Unknown => true,
            ResolvedType::Option(inner)
            | ResolvedType::Set(inner)
            | ResolvedType::Ref(inner)
            | ResolvedType::MutRef(inner)
            | ResolvedType::Box(inner)
            | ResolvedType::Rc(inner)
            | ResolvedType::Arc(inner)
            | ResolvedType::Ptr(inner)
            | ResolvedType::Task(inner)
            | ResolvedType::Range(inner) => Self::resolved_type_contains_unknown(inner),
            ResolvedType::Result(ok, err) | ResolvedType::Map(ok, err) => {
                Self::resolved_type_contains_unknown(ok)
                    || Self::resolved_type_contains_unknown(err)
            }
            ResolvedType::List(inner) => Self::resolved_type_contains_unknown(inner),
            ResolvedType::Function(params, ret) => {
                params.iter().any(Self::resolved_type_contains_unknown)
                    || Self::resolved_type_contains_unknown(ret)
            }
            ResolvedType::Integer
            | ResolvedType::Float
            | ResolvedType::Boolean
            | ResolvedType::String
            | ResolvedType::Char
            | ResolvedType::None
            | ResolvedType::Class(_)
            | ResolvedType::TypeVar(_) => false,
        }
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
    pub fn new() -> Self {
        let global_scope = Scope {
            variables: HashMap::new(),
            parent: None,
        };

        Self {
            scopes: vec![global_scope],
            current_scope: 0,
            functions: HashMap::new(),
            function_leaf_names: HashMap::new(),
            classes: HashMap::new(),
            enums: HashMap::new(),
            interfaces: HashMap::new(),
            enum_variant_to_enum: HashMap::new(),
            type_var_counter: 0,
            errors: Vec::new(),
            current_return_type: None,
            current_async_return_type: None,
            current_class: None,
            import_aliases: HashMap::new(),
            current_effects: Vec::new(),
            current_is_pure: false,
            current_allow_any: false,
            current_has_explicit_effects: false,
            current_generic_type_bindings: HashMap::new(),
            type_var_bounds: HashMap::new(),
            current_module_prefix: None,
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
            self.validate_resolved_type_exists(&resolved, span.clone());
            if !self.is_ffi_safe_type(&resolved) {
                self.error(
                    format!(
                        "Extern function '{}' has non-FFI-safe parameter '{}: {}'",
                        func.name,
                        param.name,
                        Self::format_resolved_type_for_diagnostic(&resolved)
                    ),
                    span.clone(),
                );
            }
        }
        let ret = self.resolve_type(&func.return_type);
        self.validate_resolved_type_exists(&ret, span.clone());
        if !self.is_ffi_safe_type(&ret) {
            self.error(
                format!(
                    "Extern function '{}' has non-FFI-safe return type '{}'",
                    func.name,
                    Self::format_resolved_type_for_diagnostic(&ret)
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
            let suggested_attr = match effect {
                "io" => "Io".to_string(),
                "net" => "Net".to_string(),
                "alloc" => "Alloc".to_string(),
                "unsafe" => "Unsafe".to_string(),
                "thread" => "Thread".to_string(),
                _ => {
                    let mut chars = effect.chars();
                    match chars.next() {
                        Some(first) => {
                            let mut value = first.to_ascii_uppercase().to_string();
                            value.push_str(chars.as_str());
                            value
                        }
                        None => "Any".to_string(),
                    }
                }
            };
            self.error(
                format!(
                    "Missing effect '{}' for call to '{}'. Add @{} (or @Any) on the caller function",
                    effect,
                    callee,
                    suggested_attr
                ),
                span,
            );
        }
    }

    fn enforce_call_effects(&mut self, sig: &FuncSig, span: Span, callee: &str) {
        if sig.is_pure {
            return;
        }
        if sig.allow_any {
            if self.current_is_pure {
                self.error(
                    format!(
                        "Pure function cannot call @Any function '{}'; @Any may perform effects",
                        callee
                    ),
                    span,
                );
                return;
            }
            if self.current_has_explicit_effects && !self.current_allow_any {
                self.error(
                    format!(
                        "Call to @Any function '{}' requires @Any on the caller function",
                        callee
                    ),
                    span,
                );
            }
            return;
        }
        for eff in &sig.effects {
            self.enforce_required_effect(eff, span.clone(), callee);
        }
    }

    fn enforce_function_value_effect_contract(
        &mut self,
        contract: &FunctionEffectContract,
        span: Span,
        callee: &str,
    ) {
        match contract {
            FunctionEffectContract::Any => {
                if self.current_is_pure {
                    self.error(
                        format!(
                            "Pure function cannot call @Any function value '{}'; @Any may perform effects",
                            callee
                        ),
                        span,
                    );
                    return;
                }
                if self.current_has_explicit_effects && !self.current_allow_any {
                    self.error(
                        format!(
                            "Call to @Any function value '{}' requires @Any on the caller function",
                            callee
                        ),
                        span,
                    );
                }
            }
            FunctionEffectContract::Effects(effects) => {
                if self.current_is_pure {
                    for eff in effects {
                        self.enforce_required_effect(eff, span.clone(), callee);
                    }
                    return;
                }
                if !self.current_has_explicit_effects {
                    return;
                }
                for eff in effects {
                    self.enforce_required_effect(eff, span.clone(), callee);
                }
            }
            FunctionEffectContract::Unknown => {
                if self.current_is_pure {
                    self.error(
                        format!(
                            "Pure function cannot call function value '{}' with unknown effect contract",
                            callee
                        ),
                        span,
                    );
                    return;
                }
                if self.current_has_explicit_effects && !self.current_allow_any {
                    self.error(
                        format!(
                            "Call to function value '{}' with unknown effect contract requires @Any on the caller function",
                            callee
                        ),
                        span,
                    );
                }
            }
        }
    }
    fn populate_import_aliases(&mut self, program: &Program) {
        fn collect_import_aliases(
            import_aliases: &mut HashMap<String, Vec<(Option<String>, String)>>,
            declarations: &[Spanned<Decl>],
            module_prefix: Option<&str>,
        ) {
            for decl in declarations {
                match &decl.node {
                    Decl::Import(import) => {
                        if let Some(alias) = &import.alias {
                            import_aliases
                                .entry(alias.clone())
                                .or_default()
                                .push((module_prefix.map(str::to_string), import.path.clone()));
                        } else if import.path.ends_with(".*") {
                            import_aliases
                                .entry(import.path.clone())
                                .or_default()
                                .push((module_prefix.map(str::to_string), import.path.clone()));
                        }
                    }
                    Decl::Module(module) => {
                        let next_prefix = if let Some(prefix) = module_prefix {
                            format!("{}__{}", prefix, module.name)
                        } else {
                            module.name.clone()
                        };
                        collect_import_aliases(
                            import_aliases,
                            &module.declarations,
                            Some(&next_prefix),
                        );
                    }
                    _ => {}
                }
            }
        }

        self.import_aliases.clear();
        collect_import_aliases(&mut self.import_aliases, &program.declarations, None);
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
        let target_base = self.interface_base_name(interface_name);
        let mut current = class_name;
        let mut depth = 0usize;
        while depth < 64 {
            let Some(info) = self.classes.get(current) else {
                return false;
            };

            if info.implements.iter().any(|i| {
                self.interface_base_name(i) == target_base
                    || self.interface_extends(i, interface_name)
            }) {
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
        let target_base = self.interface_base_name(target).to_string();
        if self.interface_base_name(interface_name) == target_base {
            return true;
        }
        let mut stack = vec![interface_name.to_string()];
        let mut visited = std::collections::HashSet::new();
        while let Some(name) = stack.pop() {
            if !visited.insert(name.clone()) {
                continue;
            }
            if self.interface_base_name(&name) == target_base {
                return true;
            }
            let (base_name, _, name_substitutions) =
                self.instantiated_interface_substitutions(&name);
            if let Some(info) = self.interfaces.get(&base_name) {
                for parent in &info.extends {
                    stack.push(self.substitute_interface_reference(parent, &name_substitutions));
                }
            }
        }
        false
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
        let interface_generic_bindings = self.make_generic_type_bindings(&interface.generic_params);
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
                                format_diagnostic_class_name(key),
                                method_name,
                                format_diagnostic_class_name(existing_parent),
                                format_diagnostic_class_name(&resolved_parent)
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
                .map(|p| {
                    (
                        p.name.clone(),
                        self.resolve_type_with_bindings(&p.ty, &interface_generic_bindings),
                    )
                })
                .collect::<Vec<_>>();
            let sig = FuncSig {
                params,
                return_type: self
                    .resolve_type_with_bindings(&method.return_type, &interface_generic_bindings),
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
                            format_diagnostic_class_name(key),
                            method.name,
                            format_diagnostic_class_name(parent_name)
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

    fn instantiated_enum_substitutions(
        &self,
        enum_name: &str,
    ) -> (String, HashMap<usize, ResolvedType>) {
        let base_name = self.class_base_name(enum_name).to_string();
        let Some(en) = self.enums.get(&base_name) else {
            return (base_name, HashMap::new());
        };
        if en.generic_type_vars.is_empty() || !enum_name.contains('<') || !enum_name.ends_with('>')
        {
            return (base_name, HashMap::new());
        }

        let Some(open_bracket) = enum_name.find('<') else {
            return (base_name, HashMap::new());
        };
        let inner = &enum_name[open_bracket + 1..enum_name.len() - 1];
        let parts = self.split_generic_args(inner);
        if parts.len() != en.generic_type_vars.len() {
            return (base_name, HashMap::new());
        }
        let substitutions = en
            .generic_type_vars
            .iter()
            .zip(parts.iter())
            .map(|(id, part)| (*id, self.parse_type_string(part)))
            .collect();
        (base_name, substitutions)
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

    fn interface_base_name<'a>(&self, interface_name: &'a str) -> &'a str {
        interface_name.split('<').next().unwrap_or(interface_name)
    }

    fn instantiated_interface_substitutions(
        &self,
        interface_name: &str,
    ) -> (
        String,
        HashMap<usize, ResolvedType>,
        HashMap<String, ResolvedType>,
    ) {
        let base_name = self.interface_base_name(interface_name).to_string();
        let Some(interface) = self.interfaces.get(&base_name) else {
            return (base_name, HashMap::new(), HashMap::new());
        };
        if interface.generic_type_vars.is_empty()
            || !interface_name.contains('<')
            || !interface_name.ends_with('>')
        {
            return (base_name, HashMap::new(), HashMap::new());
        }

        let Some(open_bracket) = interface_name.find('<') else {
            return (base_name, HashMap::new(), HashMap::new());
        };
        let inner = &interface_name[open_bracket + 1..interface_name.len() - 1];
        let parts = self.split_generic_args(inner);
        if parts.len() != interface.generic_type_vars.len()
            || parts.len() != interface.generic_param_names.len()
        {
            return (base_name, HashMap::new(), HashMap::new());
        }

        let positional = interface
            .generic_type_vars
            .iter()
            .zip(parts.iter())
            .map(|(id, part)| (*id, self.parse_type_string(part)))
            .collect::<HashMap<_, _>>();
        let named = interface
            .generic_param_names
            .iter()
            .zip(parts.iter())
            .map(|(name, part)| (name.clone(), self.parse_type_string(part)))
            .collect::<HashMap<_, _>>();
        (base_name, positional, named)
    }

    fn substitute_interface_reference(
        &self,
        interface_name: &str,
        substitutions: &HashMap<String, ResolvedType>,
    ) -> String {
        parse_type_source(interface_name)
            .ok()
            .map(|ty| {
                self.resolve_type_with_bindings(&ty, substitutions)
                    .to_string()
            })
            .unwrap_or_else(|| interface_name.to_string())
    }

    fn instantiate_interface_signature(&self, interface_name: &str, sig: &FuncSig) -> FuncSig {
        let (_, substitutions, _) = self.instantiated_interface_substitutions(interface_name);
        if substitutions.is_empty() {
            return sig.clone();
        }
        FuncSig {
            params: sig
                .params
                .iter()
                .map(|(name, ty)| (name.clone(), Self::substitute_type_vars(ty, &substitutions)))
                .collect(),
            return_type: Self::substitute_type_vars(&sig.return_type, &substitutions),
            ..sig.clone()
        }
    }

    fn lookup_interface_method(&self, interface_name: &str, method_name: &str) -> Option<FuncSig> {
        let mut methods = HashMap::new();
        let mut visited = std::collections::HashSet::new();
        self.collect_interface_methods(interface_name, &mut methods, &mut visited);
        methods.get(method_name).cloned()
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
    /// Collect all top-level declarations
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
                | ResolvedType::TypeVar(_)
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

    fn add_pattern_bindings(
        pattern: &Pattern,
        local_names: &mut std::collections::HashSet<String>,
    ) {
        match pattern {
            Pattern::Ident(name) => {
                local_names.insert(name.clone());
            }
            Pattern::Variant(_, bindings) => {
                for binding in bindings {
                    local_names.insert(binding.clone());
                }
            }
            Pattern::Wildcard | Pattern::Literal(_) => {}
        }
    }

    fn block_mentions_ident_with_shadowing(
        block: &[Spanned<Stmt>],
        ident: &str,
        local_names: &mut std::collections::HashSet<String>,
    ) -> bool {
        block
            .iter()
            .any(|stmt| Self::stmt_mentions_ident_with_shadowing(&stmt.node, ident, local_names))
    }

    fn expr_mentions_ident_with_shadowing(
        expr: &Expr,
        ident: &str,
        local_names: &std::collections::HashSet<String>,
    ) -> bool {
        match expr {
            Expr::Ident(name) => name == ident && !local_names.contains(name),
            Expr::Binary { left, right, .. } => {
                Self::expr_mentions_ident_with_shadowing(&left.node, ident, local_names)
                    || Self::expr_mentions_ident_with_shadowing(&right.node, ident, local_names)
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => {
                Self::expr_mentions_ident_with_shadowing(&expr.node, ident, local_names)
            }
            Expr::Call { callee, args, .. } => {
                Self::expr_mentions_ident_with_shadowing(&callee.node, ident, local_names)
                    || args.iter().any(|arg| {
                        Self::expr_mentions_ident_with_shadowing(&arg.node, ident, local_names)
                    })
            }
            Expr::GenericFunctionValue { callee, .. } => {
                Self::expr_mentions_ident_with_shadowing(&callee.node, ident, local_names)
            }
            Expr::Field { object, .. } => {
                Self::expr_mentions_ident_with_shadowing(&object.node, ident, local_names)
            }
            Expr::Index { object, index } => {
                Self::expr_mentions_ident_with_shadowing(&object.node, ident, local_names)
                    || Self::expr_mentions_ident_with_shadowing(&index.node, ident, local_names)
            }
            Expr::Construct { args, .. } => args
                .iter()
                .any(|arg| Self::expr_mentions_ident_with_shadowing(&arg.node, ident, local_names)),
            Expr::Lambda { params, body } => {
                let mut nested_locals = local_names.clone();
                for param in params {
                    nested_locals.insert(param.name.clone());
                }
                Self::expr_mentions_ident_with_shadowing(&body.node, ident, &nested_locals)
            }
            Expr::Match { expr, arms } => {
                Self::expr_mentions_ident_with_shadowing(&expr.node, ident, local_names)
                    || arms.iter().any(|arm| {
                        let mut arm_locals = local_names.clone();
                        Self::add_pattern_bindings(&arm.pattern, &mut arm_locals);
                        Self::block_mentions_ident_with_shadowing(&arm.body, ident, &mut arm_locals)
                    })
            }
            Expr::StringInterp(parts) => parts.iter().any(|part| match part {
                StringPart::Literal(_) => false,
                StringPart::Expr(expr) => {
                    Self::expr_mentions_ident_with_shadowing(&expr.node, ident, local_names)
                }
            }),
            Expr::AsyncBlock(body) | Expr::Block(body) => {
                let mut block_locals = local_names.clone();
                Self::block_mentions_ident_with_shadowing(body, ident, &mut block_locals)
            }
            Expr::Require { condition, message } => {
                Self::expr_mentions_ident_with_shadowing(&condition.node, ident, local_names)
                    || message.as_ref().is_some_and(|msg| {
                        Self::expr_mentions_ident_with_shadowing(&msg.node, ident, local_names)
                    })
            }
            Expr::Range { start, end, .. } => {
                start.as_ref().is_some_and(|expr| {
                    Self::expr_mentions_ident_with_shadowing(&expr.node, ident, local_names)
                }) || end.as_ref().is_some_and(|expr| {
                    Self::expr_mentions_ident_with_shadowing(&expr.node, ident, local_names)
                })
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expr_mentions_ident_with_shadowing(&condition.node, ident, local_names)
                    || {
                        let mut then_locals = local_names.clone();
                        Self::block_mentions_ident_with_shadowing(
                            then_branch,
                            ident,
                            &mut then_locals,
                        )
                    }
                    || else_branch.as_ref().is_some_and(|stmts| {
                        let mut else_locals = local_names.clone();
                        Self::block_mentions_ident_with_shadowing(stmts, ident, &mut else_locals)
                    })
            }
            Expr::Literal(_) | Expr::This => false,
        }
    }

    fn stmt_mentions_ident_with_shadowing(
        stmt: &Stmt,
        ident: &str,
        local_names: &mut std::collections::HashSet<String>,
    ) -> bool {
        match stmt {
            Stmt::Let { name, value, .. } => {
                let mentions =
                    Self::expr_mentions_ident_with_shadowing(&value.node, ident, local_names);
                local_names.insert(name.clone());
                mentions
            }
            Stmt::Assign { target, value } => {
                Self::expr_mentions_ident_with_shadowing(&target.node, ident, local_names)
                    || Self::expr_mentions_ident_with_shadowing(&value.node, ident, local_names)
            }
            Stmt::Expr(expr) => {
                Self::expr_mentions_ident_with_shadowing(&expr.node, ident, local_names)
            }
            Stmt::Return(expr) => expr.as_ref().is_some_and(|expr| {
                Self::expr_mentions_ident_with_shadowing(&expr.node, ident, local_names)
            }),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                Self::expr_mentions_ident_with_shadowing(&condition.node, ident, local_names)
                    || {
                        let mut then_locals = local_names.clone();
                        Self::block_mentions_ident_with_shadowing(
                            then_block,
                            ident,
                            &mut then_locals,
                        )
                    }
                    || else_block.as_ref().is_some_and(|stmts| {
                        let mut else_locals = local_names.clone();
                        Self::block_mentions_ident_with_shadowing(stmts, ident, &mut else_locals)
                    })
            }
            Stmt::While { condition, body } => {
                Self::expr_mentions_ident_with_shadowing(&condition.node, ident, local_names) || {
                    let mut body_locals = local_names.clone();
                    Self::block_mentions_ident_with_shadowing(body, ident, &mut body_locals)
                }
            }
            Stmt::For {
                var,
                iterable,
                body,
                ..
            } => {
                Self::expr_mentions_ident_with_shadowing(&iterable.node, ident, local_names) || {
                    let mut body_locals = local_names.clone();
                    body_locals.insert(var.clone());
                    Self::block_mentions_ident_with_shadowing(body, ident, &mut body_locals)
                }
            }
            Stmt::Match { expr, arms } => {
                Self::expr_mentions_ident_with_shadowing(&expr.node, ident, local_names)
                    || arms.iter().any(|arm| {
                        let mut arm_locals = local_names.clone();
                        Self::add_pattern_bindings(&arm.pattern, &mut arm_locals);
                        Self::block_mentions_ident_with_shadowing(&arm.body, ident, &mut arm_locals)
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
                if !en.generic_params.is_empty() {
                    self.error(
                        format!(
                            "Enum '{}' uses generic parameters, but user-defined generic enums are not supported yet",
                            en.name
                        ),
                        span.clone(),
                    );
                    self.current_module_prefix = saved_module_prefix;
                    return;
                }
                let enum_generic_bindings = self.make_generic_type_bindings(&en.generic_params);
                for variant in &en.variants {
                    for field in &variant.fields {
                        let ty = self.resolve_type_with_bindings(&field.ty, &enum_generic_bindings);
                        self.validate_resolved_type_exists(&ty, span.clone());
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
        let interface_generic_bindings = self.make_generic_type_bindings(&interface.generic_params);
        let interface_key = self
            .current_module_prefix
            .as_ref()
            .map(|prefix| format!("{}__{}", prefix, interface.name))
            .unwrap_or_else(|| interface.name.clone());
        self.validate_interface_inherited_method_conflicts(interface, &interface_key, span.clone());
        for method in &interface.methods {
            let saved_generic_bindings = std::mem::take(&mut self.current_generic_type_bindings);
            self.current_generic_type_bindings = interface_generic_bindings.clone();
            for param in &method.params {
                let ty = self.resolve_type(&param.ty);
                self.validate_resolved_type_exists(&ty, span.clone());
                self.check_type_visibility(&ty, span.clone());
            }
            let ret_ty = self.resolve_type(&method.return_type);
            self.validate_resolved_type_exists(&ret_ty, span.clone());
            self.check_type_visibility(&ret_ty, span.clone());
            self.current_generic_type_bindings = saved_generic_bindings;
        }

        for method in &interface.methods {
            let Some(body) = &method.default_impl else {
                continue;
            };
            let saved_generic_bindings = std::mem::take(&mut self.current_generic_type_bindings);
            self.current_generic_type_bindings = interface_generic_bindings.clone();
            self.enter_scope();
            let saved_effects = std::mem::take(&mut self.current_effects);
            let saved_pure = self.current_is_pure;
            let saved_any = self.current_allow_any;
            let saved_explicit = self.current_has_explicit_effects;
            self.current_allow_any = true;
            self.current_is_pure = false;
            self.current_has_explicit_effects = false;
            for param in &method.params {
                let ty = self.resolve_type(&param.ty);
                self.validate_resolved_type_exists(&ty, span.clone());
                self.declare_variable(
                    &param.name,
                    ty,
                    param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                    span.clone(),
                );
            }
            let return_type = self.resolve_type(&method.return_type);
            self.validate_resolved_type_exists(&return_type, span.clone());
            self.current_return_type = Some(return_type);
            self.check_block(body);
            self.current_return_type = None;
            self.current_effects = saved_effects;
            self.current_is_pure = saved_pure;
            self.current_allow_any = saved_any;
            self.current_has_explicit_effects = saved_explicit;
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
        let saved_explicit = self.current_has_explicit_effects;
        let sig = function_key
            .and_then(|k| self.functions.get(k))
            .or_else(|| self.functions.get(&func.name));
        if let Some(sig) = sig {
            self.current_effects = sig.effects.clone();
            self.current_is_pure = sig.is_pure;
            self.current_allow_any = sig.allow_any;
            self.current_has_explicit_effects = sig.has_explicit_effects || sig.is_pure;
        } else {
            // Fallback for unresolved keys; should be rare.
            let (effects, is_pure, allow_any, has_explicit_effects) =
                self.parse_effects_from_attributes(&func.attributes);
            self.current_effects = effects;
            self.current_is_pure = is_pure;
            self.current_allow_any = allow_any;
            self.current_has_explicit_effects = has_explicit_effects || is_pure;
        }

        // Add parameters to scope
        for param in &func.params {
            let ty = self.resolve_type(&param.ty);
            self.validate_resolved_type_exists(&ty, span.clone());
            self.check_type_visibility(&ty, span.clone());
            if func.is_async
                && (Self::type_contains_borrowed_reference(&ty)
                    || !matches!(param.mode, ParamMode::Owned))
            {
                self.error(
                    format!(
                        "Async function '{}' cannot accept a parameter containing borrowed references or borrow-mode parameters: {}",
                        func.name,
                        Self::format_resolved_type_for_diagnostic(&ty)
                    ),
                    span.clone(),
                );
            }
            self.declare_variable(
                &param.name,
                ty,
                param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                span.clone(),
            );
        }

        // Set current return type
        let return_type = self.resolve_type(&func.return_type);
        self.validate_resolved_type_exists(&return_type, span.clone());
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
        self.current_has_explicit_effects = saved_explicit;
        self.exit_scope();
        self.current_generic_type_bindings = saved_generic_bindings;
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
                .classes
                .get(class_key)
                .and_then(|info| info.extends.clone())
                .unwrap_or_else(|| {
                    self.resolve_nominal_reference_name(parent)
                        .unwrap_or_else(|| parent.clone())
                });
            let resolved_parent_base = self.class_base_name(&resolved_parent);
            if self.interfaces.contains_key(resolved_parent_base) {
                self.error(
                    format!(
                        "Class '{}' cannot extend interface '{}'",
                        format_diagnostic_class_name(class_key),
                        format_diagnostic_class_name(parent)
                    ),
                    span.clone(),
                );
            } else if !self.classes.contains_key(resolved_parent_base) {
                self.error(
                    format!(
                        "Class '{}' extends unknown class '{}'",
                        format_diagnostic_class_name(class_key),
                        format_diagnostic_class_name(parent)
                    ),
                    span.clone(),
                );
            } else if self.is_same_or_subclass_of(&resolved_parent, class_key) {
                self.error(
                    format!(
                        "Inheritance cycle detected: '{}' cannot extend '{}'",
                        format_diagnostic_class_name(class_key),
                        format_diagnostic_class_name(parent)
                    ),
                    span.clone(),
                );
            } else {
                self.check_class_visibility(&resolved_parent, span.clone());
            }
        }

        for field in &class.fields {
            let ty = self.resolve_type(&field.ty);
            self.validate_resolved_type_exists(&ty, span.clone());
            self.check_type_visibility(&ty, span.clone());
        }

        for interface_name in &class.implements {
            let resolved_interface = self
                .resolve_nominal_reference_name(interface_name)
                .unwrap_or_else(|| interface_name.clone());
            if !self
                .interfaces
                .contains_key(self.interface_base_name(&resolved_interface))
            {
                self.error(
                    format!(
                        "Class '{}' implements unknown interface '{}'",
                        format_diagnostic_class_name(class_key),
                        format_diagnostic_class_name(interface_name)
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
                                format_diagnostic_class_name(class_key),
                                method_name,
                                format_diagnostic_class_name(existing_interface),
                                format_diagnostic_class_name(&resolved_interface)
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
                        format_diagnostic_class_name(class_key),
                        method_name
                    ),
                    span.clone(),
                );
                continue;
            };
            if !self.signatures_mutually_compatible(&required_sig, &actual_sig) {
                self.error(
                    format!(
                        "Method '{}.{}' does not match interface signature",
                        format_diagnostic_class_name(&owner),
                        method_name
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
            let saved_explicit = self.current_has_explicit_effects;
            let saved_class = self.current_class.clone();
            self.current_class = Some(class_key.to_string());
            self.current_effects = self
                .infer_effects_in_block(&ctor.body, Some(class_key))
                .into_iter()
                .collect();
            self.current_is_pure = false;
            self.current_allow_any = false;
            self.current_has_explicit_effects = false;

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
                self.validate_resolved_type_exists(&ty, span.clone());
                self.check_type_visibility(&ty, span.clone());
                self.declare_variable(
                    &param.name,
                    ty,
                    param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                    span.clone(),
                );
            }

            self.check_block(&ctor.body);
            self.current_effects = saved_effects;
            self.current_is_pure = saved_pure;
            self.current_allow_any = saved_any;
            self.current_has_explicit_effects = saved_explicit;
            self.current_class = saved_class;
            self.exit_scope();
        }

        // Check destructor with inferred effects
        if let Some(dtor) = &class.destructor {
            self.enter_scope();
            let saved_effects = std::mem::take(&mut self.current_effects);
            let saved_pure = self.current_is_pure;
            let saved_any = self.current_allow_any;
            let saved_explicit = self.current_has_explicit_effects;
            let saved_class = self.current_class.clone();
            self.current_class = Some(class_key.to_string());
            self.current_effects = self
                .infer_effects_in_block(&dtor.body, Some(class_key))
                .into_iter()
                .collect();
            self.current_is_pure = false;
            self.current_allow_any = false;
            self.current_has_explicit_effects = false;
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
            self.current_has_explicit_effects = saved_explicit;
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
            let saved_explicit = self.current_has_explicit_effects;
            let saved_class = self.current_class.clone();
            self.current_class = Some(class_key.to_string());
            if let Some(class_info) = self.classes.get(class_key) {
                if let Some(sig) = class_info.methods.get(&method.name) {
                    self.current_effects = sig.effects.clone();
                    self.current_is_pure = sig.is_pure;
                    self.current_allow_any = sig.allow_any;
                    self.current_has_explicit_effects = sig.has_explicit_effects || sig.is_pure;
                } else {
                    self.current_effects.clear();
                    self.current_is_pure = false;
                    self.current_allow_any = false;
                    self.current_has_explicit_effects = false;
                }
            } else {
                self.current_effects.clear();
                self.current_is_pure = false;
                self.current_allow_any = false;
                self.current_has_explicit_effects = false;
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
                self.validate_resolved_type_exists(&ty, span.clone());
                self.check_type_visibility(&ty, span.clone());
                if method.is_async
                    && (Self::type_contains_borrowed_reference(&ty)
                        || !matches!(param.mode, ParamMode::Owned))
                {
                    self.error(
                        format!(
                            "Async method '{}.{}' cannot accept a parameter containing borrowed references or borrow-mode parameters: {}",
                            format_diagnostic_class_name(class_key),
                            method.name,
                            Self::format_resolved_type_for_diagnostic(&ty)
                        ),
                        span.clone(),
                    );
                }
                self.declare_variable(
                    &param.name,
                    ty,
                    param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                    span.clone(),
                );
            }

            let return_type = self.resolve_type(&method.return_type);
            self.validate_resolved_type_exists(&return_type, span.clone());
            self.check_type_visibility(&return_type, span.clone());
            self.current_return_type = Some(return_type);

            self.check_block(&method.body);

            self.current_return_type = None;
            self.current_effects = saved_effects;
            self.current_is_pure = saved_pure;
            self.current_allow_any = saved_any;
            self.current_has_explicit_effects = saved_explicit;
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
                let declared_type_exists = self.resolved_type_exists(&declared_type);
                if declared_type_exists {
                    self.check_type_visibility(&declared_type, span.clone());
                } else {
                    self.validate_resolved_type_exists(&declared_type, span.clone());
                }
                let expected_type = if declared_type_exists {
                    declared_type.clone()
                } else {
                    ResolvedType::Unknown
                };
                let value_type = self.check_expr_with_expected_type(
                    &value.node,
                    value.span.clone(),
                    Some(&expected_type),
                );

                // Check type compatibility. If the value is an if-expression that already
                // produced a branch mismatch diagnostic, avoid cascading a second local
                // assignment mismatch for the same root cause.
                let suppress_assignment_mismatch = matches!(&value.node, Expr::If { .. })
                    && !self.types_compatible(&expected_type, &value_type)
                    && self.errors.iter().any(|error| {
                        error.message.contains("If expression branch type mismatch")
                            && error.span == value.span
                    });

                if !self.types_compatible(&expected_type, &value_type)
                    && !suppress_assignment_mismatch
                {
                    self.error(
                        format!(
                            "Type mismatch: cannot assign {} to variable of type {}",
                            Self::format_resolved_type_for_diagnostic(&value_type),
                            Self::format_resolved_type_for_diagnostic(&expected_type)
                        ),
                        value.span.clone(),
                    );
                }

                let callable_effects =
                    self.infer_function_value_effect_contract(&value.node, &expected_type);
                self.declare_variable_with_contract(
                    name,
                    expected_type,
                    *mutable,
                    span,
                    callable_effects,
                );
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
                            Self::format_resolved_type_for_diagnostic(&target_type),
                            Self::format_resolved_type_for_diagnostic(&value_type)
                        ),
                        value.span.clone(),
                    );
                }
                if let Expr::Ident(name) = &target.node {
                    let callable_effects =
                        self.infer_function_value_effect_contract(&value.node, &target_type);
                    self.update_variable_callable_effects(name, callable_effects);
                }
            }

            Stmt::Expr(expr) => {
                self.check_expr(&expr.node, expr.span.clone());
            }

            Stmt::Return(expr) => {
                let expected_return_type = self.current_return_type.clone().or_else(|| {
                    self.current_async_return_type
                        .clone()
                        .and_then(|ty| (!matches!(ty, ResolvedType::None)).then_some(ty))
                });
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
                                Self::format_resolved_type_for_diagnostic(expected),
                                Self::format_resolved_type_for_diagnostic(&return_type)
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
                        format!(
                            "Condition must be Boolean, found {}",
                            Self::format_resolved_type_for_diagnostic(&cond_type)
                        ),
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
                        format!(
                            "Condition must be Boolean, found {}",
                            Self::format_resolved_type_for_diagnostic(&cond_type)
                        ),
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
                let iter_type =
                    self.check_builtin_argument_expr(&iterable.node, iterable.span.clone());
                let iter_item_type = Self::peel_reference_type(&iter_type);

                // Determine element type
                let elem_type = match iter_item_type {
                    ResolvedType::List(inner) => (**inner).clone(),
                    ResolvedType::Range(inner) => (**inner).clone(),
                    ResolvedType::String => ResolvedType::Char,
                    ResolvedType::Integer => ResolvedType::Integer,
                    _ => {
                        self.error(
                            format!(
                                "Cannot iterate over {}",
                                Self::format_resolved_type_for_diagnostic(&iter_type)
                            ),
                            iterable.span.clone(),
                        );
                        ResolvedType::Unknown
                    }
                };

                // Check declared type if provided
                if let Some(declared) = var_type {
                    let declared_type = self.resolve_type(declared);
                    if !self.resolved_type_exists(&declared_type) {
                        self.validate_resolved_type_exists(&declared_type, iterable.span.clone());
                    } else if !self.types_compatible(&declared_type, &elem_type) {
                        self.error(
                            format!(
                                "Loop variable type mismatch: declared {}, but iterating over {}",
                                Self::format_resolved_type_for_diagnostic(&declared_type),
                                Self::format_resolved_type_for_diagnostic(&iter_type)
                            ),
                            iterable.span.clone(),
                        );
                    }
                }

                self.enter_scope();
                let loop_var_type = var_type
                    .as_ref()
                    .map(|declared| {
                        let declared_type = self.resolve_type(declared);
                        if self.resolved_type_exists(&declared_type) {
                            declared_type
                        } else {
                            ResolvedType::Unknown
                        }
                    })
                    .unwrap_or(elem_type);
                self.declare_variable(var, loop_var_type, false, span);
                self.check_block(body);
                self.exit_scope();
            }

            Stmt::Match { expr, arms } => {
                let match_type = self.check_builtin_argument_expr(&expr.node, expr.span.clone());

                for arm in arms {
                    self.enter_scope();
                    self.check_pattern(&arm.pattern, &match_type, span.clone());
                    self.check_block(&arm.body);
                    self.exit_scope();
                }

                if !self.match_expression_exhaustive(&match_type, arms) {
                    self.error(
                        format!(
                            "Non-exhaustive match statement for type {}",
                            Self::format_resolved_type_for_diagnostic(&match_type)
                        ),
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

        let imported_unit_variant = |this: &Self, name: &str| -> Option<(String, String)> {
            let (enum_name, variant_name, is_unit) = this.resolve_pattern_variant_alias(name)?;
            is_unit.then_some((enum_name, variant_name))
        };

        match pattern {
            Pattern::Wildcard => {}
            Pattern::Ident(name) => {
                if let Some((enum_name, variant_name)) = imported_unit_variant(self, name) {
                    match expected_type {
                        ResolvedType::Option(_) | ResolvedType::Result(_, _) => {
                            self.check_pattern(
                                &Pattern::Variant(variant_name, vec![]),
                                expected_type,
                                span,
                            );
                        }
                        ResolvedType::Class(expected_enum) if expected_enum == &enum_name => {
                            self.check_pattern(
                                &Pattern::Variant(name.clone(), vec![]),
                                expected_type,
                                span,
                            );
                        }
                        ResolvedType::Class(expected_enum) => {
                            self.error(
                                format!("Cannot match variant {} on type {}", name, expected_enum),
                                span,
                            );
                        }
                        _ => {
                            self.error(
                                format!("Cannot match variant {} on type {}", name, expected_type),
                                span,
                            );
                        }
                    }
                } else {
                    self.declare_variable(name, expected_type.clone(), false, span);
                }
            }
            Pattern::Literal(lit) => {
                let lit_type = Self::literal_type(lit);
                if self
                    .common_compatible_type(expected_type, &lit_type)
                    .is_none()
                {
                    self.error(
                        format!(
                            "Pattern type mismatch: expected {}, found {}",
                            Self::format_resolved_type_for_diagnostic(expected_type),
                            Self::format_resolved_type_for_diagnostic(&lit_type)
                        ),
                        span,
                    );
                }
            }
            Pattern::Variant(name, bindings) => {
                let imported_variant = (!name.contains('.'))
                    .then(|| self.resolve_pattern_variant_alias(name))
                    .flatten();
                let variant_name = imported_variant.as_ref().map_or_else(
                    || pattern_variant_leaf(name).to_string(),
                    |(_, variant, _)| variant.clone(),
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
                            .is_some_and(|(owner_enum, _, _)| owner_enum != enum_name)
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
                        let (_enum_base, enum_substitutions) =
                            self.instantiated_enum_substitutions(enum_name);
                        if let Some(enum_info) = self.enums.get(&resolved_enum_name).cloned() {
                            if let Some(field_tys) = enum_info.variants.get(&variant_name) {
                                let expected_field_tys = field_tys
                                    .iter()
                                    .map(|ty| Self::substitute_type_vars(ty, &enum_substitutions))
                                    .collect::<Vec<_>>();
                                if field_tys.len() != bindings.len() {
                                    self.error(
                                        format!(
                                            "Pattern '{}' expects {} binding(s), got {}",
                                            variant_name,
                                            expected_field_tys.len(),
                                            bindings.len()
                                        ),
                                        span,
                                    );
                                } else {
                                    for (binding, ty) in
                                        bindings.iter().zip(expected_field_tys.iter())
                                    {
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
                                    format!(
                                        "Unknown variant '{}' for enum '{}'",
                                        name,
                                        format_diagnostic_class_name(enum_name)
                                    ),
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

    fn match_expression_exhaustive(&self, match_type: &ResolvedType, arms: &[MatchArm]) -> bool {
        let imported_unit_variant = |name: &str| -> Option<(String, String)> {
            let (enum_name, variant_name) = self.resolve_import_alias_variant(name)?;
            self.enums
                .get(&enum_name)
                .and_then(|enum_info| enum_info.variants.get(&variant_name))
                .is_some_and(|fields| fields.is_empty())
                .then_some((enum_name, variant_name))
        };
        let has_catch_all = arms.iter().any(|arm| match &arm.pattern {
            Pattern::Wildcard => true,
            Pattern::Ident(name) => imported_unit_variant(name).is_none(),
            _ => false,
        });
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
                    Pattern::Ident(name) => imported_unit_variant(name)
                        .is_some_and(|(owner_enum, variant)| owner_enum == "Option" && variant == "Some"),
                    Pattern::Variant(name, _) => {
                        name.rsplit('.').next().is_some_and(|leaf| leaf == "Some")
                            || self
                                .resolve_import_alias_variant(name)
                                .is_some_and(|(owner_enum, variant)| owner_enum == "Option" && variant == "Some")
                    }
                    _ => false,
                });
                let has_none = arms.iter().any(|arm| match &arm.pattern {
                    Pattern::Ident(name) => imported_unit_variant(name)
                        .is_some_and(|(owner_enum, variant)| owner_enum == "Option" && variant == "None"),
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
                    Pattern::Ident(name) => imported_unit_variant(name)
                        .is_some_and(|(owner_enum, variant)| owner_enum == "Result" && variant == "Ok"),
                    Pattern::Variant(name, _) => {
                        name.rsplit('.').next().is_some_and(|leaf| leaf == "Ok")
                            || self
                                .resolve_import_alias_variant(name)
                                .is_some_and(|(owner_enum, variant)| owner_enum == "Result" && variant == "Ok")
                    }
                    _ => false,
                });
                let has_err = arms.iter().any(|arm| match &arm.pattern {
                    Pattern::Ident(name) => imported_unit_variant(name)
                        .is_some_and(|(owner_enum, variant)| owner_enum == "Result" && variant == "Error"),
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
                            matches!(&arm.pattern, Pattern::Ident(name)
                                if imported_unit_variant(name).is_some_and(|(owner_enum, imported_variant)| {
                                    owner_enum == *enum_name && imported_variant == *variant_name
                                }))
                                ||
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

    fn instantiate_function_value_type(
        &mut self,
        function_name: &str,
        sig: &FuncSig,
        type_args: &[Type],
        span: Span,
    ) -> ResolvedType {
        if sig.is_extern {
            self.error(
                format!(
                    "extern function '{}' cannot be used as a first-class value",
                    function_name
                ),
                span,
            );
            return ResolvedType::Unknown;
        }

        let (inst_params, inst_return_type, valid_explicit_type_args) =
            self.instantiate_signature_for_call(function_name, sig, type_args, span.clone());
        if !valid_explicit_type_args {
            return ResolvedType::Unknown;
        }

        ResolvedType::Function(
            inst_params.into_iter().map(|(_, ty)| ty).collect(),
            Box::new(inst_return_type),
        )
    }
    fn nominal_function_value_type_source(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Ident(name) => Some(name.clone()),
            Expr::Field { .. } => Some(flatten_field_chain(expr)?.join(".")),
            _ => None,
        }
    }
    fn builtin_matches_expected_function_type(name: &str, expected: &ResolvedType) -> bool {
        let ResolvedType::Function(params, ret) = expected else {
            return matches!(
                name,
                "read_line"
                    | "File__read"
                    | "File__write"
                    | "File__exists"
                    | "File__delete"
                    | "System__getenv"
                    | "System__shell"
                    | "System__exec"
                    | "System__cwd"
                    | "System__os"
                    | "System__exit"
                    | "Time__now"
                    | "Time__unix"
                    | "Time__sleep"
                    | "Args__count"
                    | "Args__get"
                    | "Math__abs"
                    | "Math__min"
                    | "Math__max"
                    | "Math__sqrt"
                    | "Math__sin"
                    | "Math__cos"
                    | "Math__tan"
                    | "Math__pow"
                    | "Math__floor"
                    | "Math__ceil"
                    | "Math__round"
                    | "Math__log"
                    | "Math__log10"
                    | "Math__exp"
                    | "Math__pi"
                    | "Math__e"
                    | "Math__random"
                    | "Str__len"
                    | "Str__compare"
                    | "Str__concat"
                    | "Str__upper"
                    | "Str__lower"
                    | "Str__trim"
                    | "Str__contains"
                    | "Str__startsWith"
                    | "Str__endsWith"
                    | "to_float"
                    | "to_int"
                    | "to_string"
                    | "assert"
                    | "assert_eq"
                    | "assert_ne"
                    | "assert_true"
                    | "assert_false"
                    | "fail"
                    | "exit"
                    | "range"
                    | "Option__some"
                    | "Option__none"
                    | "Result__ok"
                    | "Result__error"
            );
        };

        match name {
            "Option__some" => {
                params.len() == 1
                    && matches!(ret.as_ref(), ResolvedType::Option(inner) if params[0] == inner.as_ref().clone())
            }
            "Option__none" => params.is_empty() && matches!(ret.as_ref(), ResolvedType::Option(_)),
            "Result__ok" => {
                params.len() == 1
                    && matches!(ret.as_ref(), ResolvedType::Result(ok, _) if params[0] == ok.as_ref().clone())
            }
            "Result__error" => {
                params.len() == 1
                    && matches!(ret.as_ref(), ResolvedType::Result(_, err) if params[0] == err.as_ref().clone())
            }
            "read_line" | "System__cwd" | "System__os" => {
                params.is_empty() && matches!(ret.as_ref(), ResolvedType::String)
            }
            "File__read" | "System__getenv" | "System__shell" | "System__exec" | "Time__now" => {
                params.len() == 1
                    && matches!(params[0], ResolvedType::String)
                    && matches!(ret.as_ref(), ResolvedType::String | ResolvedType::Integer)
                    && match name {
                        "System__shell" => matches!(ret.as_ref(), ResolvedType::Integer),
                        _ => matches!(ret.as_ref(), ResolvedType::String),
                    }
            }
            "File__write" => {
                params.len() == 2
                    && matches!(params[0], ResolvedType::String)
                    && matches!(params[1], ResolvedType::String)
                    && matches!(ret.as_ref(), ResolvedType::Boolean)
            }
            "File__exists" | "File__delete" => {
                params.len() == 1
                    && matches!(params[0], ResolvedType::String)
                    && matches!(ret.as_ref(), ResolvedType::Boolean)
            }
            "System__exit" | "exit" | "Time__sleep" => {
                params.len() == 1
                    && matches!(params[0], ResolvedType::Integer)
                    && matches!(ret.as_ref(), ResolvedType::None)
            }
            "Time__unix" | "Args__count" => {
                params.is_empty() && matches!(ret.as_ref(), ResolvedType::Integer)
            }
            "Args__get" => {
                params.len() == 1
                    && matches!(params[0], ResolvedType::Integer)
                    && matches!(ret.as_ref(), ResolvedType::String)
            }
            "Math__abs" => {
                params.len() == 1
                    && params[0] == ret.as_ref().clone()
                    && matches!(params[0], ResolvedType::Integer | ResolvedType::Float)
            }
            "Math__min" | "Math__max" => {
                (params.len() == 2
                    && params[0] == params[1]
                    && params[0] == ret.as_ref().clone()
                    && matches!(params[0], ResolvedType::Integer | ResolvedType::Float))
                    || (params.len() == 2
                        && matches!(params[0], ResolvedType::Integer | ResolvedType::Float)
                        && matches!(params[1], ResolvedType::Integer | ResolvedType::Float)
                        && params[0] != params[1]
                        && matches!(ret.as_ref(), ResolvedType::Float))
            }
            "Math__pow" => {
                params.len() == 2
                    && matches!(params[0], ResolvedType::Integer | ResolvedType::Float)
                    && matches!(params[1], ResolvedType::Integer | ResolvedType::Float)
                    && matches!(ret.as_ref(), ResolvedType::Float)
            }
            "Math__sqrt" | "Math__sin" | "Math__cos" | "Math__tan" | "Math__floor"
            | "Math__ceil" | "Math__round" | "Math__log" | "Math__log10" | "Math__exp" => {
                params.len() == 1
                    && matches!(params[0], ResolvedType::Integer | ResolvedType::Float)
                    && matches!(ret.as_ref(), ResolvedType::Float)
            }
            "Math__pi" | "Math__e" | "Math__random" => {
                params.is_empty() && matches!(ret.as_ref(), ResolvedType::Float)
            }
            "Str__len" => {
                params.len() == 1
                    && matches!(params[0], ResolvedType::String)
                    && matches!(ret.as_ref(), ResolvedType::Integer)
            }
            "Str__compare" => {
                params.len() == 2
                    && matches!(params[0], ResolvedType::String)
                    && matches!(params[1], ResolvedType::String)
                    && matches!(ret.as_ref(), ResolvedType::Integer)
            }
            "Str__concat" => {
                params.len() == 2
                    && matches!(params[0], ResolvedType::String)
                    && matches!(params[1], ResolvedType::String)
                    && matches!(ret.as_ref(), ResolvedType::String)
            }
            "Str__upper" | "Str__lower" | "Str__trim" => {
                params.len() == 1
                    && matches!(params[0], ResolvedType::String)
                    && matches!(ret.as_ref(), ResolvedType::String)
            }
            "Str__contains" | "Str__startsWith" | "Str__endsWith" => {
                params.len() == 2
                    && matches!(params[0], ResolvedType::String)
                    && matches!(params[1], ResolvedType::String)
                    && matches!(ret.as_ref(), ResolvedType::Boolean)
            }
            "to_float" => {
                params.len() == 1
                    && matches!(params[0], ResolvedType::Integer | ResolvedType::Float)
                    && matches!(ret.as_ref(), ResolvedType::Float)
            }
            "to_int" => {
                params.len() == 1
                    && matches!(
                        params[0],
                        ResolvedType::Integer | ResolvedType::Float | ResolvedType::String
                    )
                    && matches!(ret.as_ref(), ResolvedType::Integer)
            }
            "to_string" => {
                params.len() == 1
                    && Self::supports_display_scalar(&params[0])
                    && matches!(ret.as_ref(), ResolvedType::String)
            }
            "assert" | "assert_true" | "assert_false" => {
                params.len() == 1
                    && matches!(params[0], ResolvedType::Boolean)
                    && matches!(ret.as_ref(), ResolvedType::None)
            }
            "fail" => {
                (params.is_empty()
                    || (params.len() == 1 && matches!(params[0], ResolvedType::String)))
                    && matches!(ret.as_ref(), ResolvedType::None)
            }
            "assert_eq" | "assert_ne" => {
                params.len() == 2
                    && (params[0] == params[1]
                        || (matches!(params[0], ResolvedType::Integer)
                            && matches!(params[1], ResolvedType::Float))
                        || (matches!(params[0], ResolvedType::Float)
                            && matches!(params[1], ResolvedType::Integer)))
                    && matches!(ret.as_ref(), ResolvedType::None)
            }
            "range" => {
                (params.len() == 2 || params.len() == 3)
                    && params
                        .iter()
                        .all(|param| matches!(param, ResolvedType::Integer))
                    && matches!(
                        ret.as_ref(),
                        ResolvedType::Range(inner) if matches!(inner.as_ref(), ResolvedType::Integer)
                    )
                    || (params.len() == 2 || params.len() == 3)
                        && params
                            .iter()
                            .all(|param| matches!(param, ResolvedType::Float))
                        && matches!(
                            ret.as_ref(),
                            ResolvedType::Range(inner) if matches!(inner.as_ref(), ResolvedType::Float)
                        )
            }
            _ => false,
        }
    }

    fn builtin_function_value_type(name: &str) -> Option<ResolvedType> {
        match name {
            "Option__some" => Some(ResolvedType::Function(
                vec![ResolvedType::Unknown],
                Box::new(ResolvedType::Option(Box::new(ResolvedType::Unknown))),
            )),
            "Option__none" => Some(ResolvedType::Function(
                vec![],
                Box::new(ResolvedType::Option(Box::new(ResolvedType::Unknown))),
            )),
            "Result__ok" => Some(ResolvedType::Function(
                vec![ResolvedType::Unknown],
                Box::new(ResolvedType::Result(
                    Box::new(ResolvedType::Unknown),
                    Box::new(ResolvedType::Unknown),
                )),
            )),
            "Result__error" => Some(ResolvedType::Function(
                vec![ResolvedType::Unknown],
                Box::new(ResolvedType::Result(
                    Box::new(ResolvedType::Unknown),
                    Box::new(ResolvedType::Unknown),
                )),
            )),
            "read_line" | "System__cwd" | "System__os" => Some(ResolvedType::Function(
                vec![],
                Box::new(ResolvedType::String),
            )),
            "File__read" | "System__getenv" | "System__exec" | "Time__now" => Some(
                ResolvedType::Function(vec![ResolvedType::String], Box::new(ResolvedType::String)),
            ),
            "System__shell" => Some(ResolvedType::Function(
                vec![ResolvedType::String],
                Box::new(ResolvedType::Integer),
            )),
            "File__write" => Some(ResolvedType::Function(
                vec![ResolvedType::String, ResolvedType::String],
                Box::new(ResolvedType::Boolean),
            )),
            "File__exists" | "File__delete" => Some(ResolvedType::Function(
                vec![ResolvedType::String],
                Box::new(ResolvedType::Boolean),
            )),
            "System__exit" | "exit" | "Time__sleep" => Some(ResolvedType::Function(
                vec![ResolvedType::Integer],
                Box::new(ResolvedType::None),
            )),
            "Time__unix" | "Args__count" => Some(ResolvedType::Function(
                vec![],
                Box::new(ResolvedType::Integer),
            )),
            "Args__get" => Some(ResolvedType::Function(
                vec![ResolvedType::Integer],
                Box::new(ResolvedType::String),
            )),
            "Math__abs" => Some(ResolvedType::Function(
                vec![ResolvedType::Unknown],
                Box::new(ResolvedType::Unknown),
            )),
            "Math__min" | "Math__max" => Some(ResolvedType::Function(
                vec![ResolvedType::Unknown, ResolvedType::Unknown],
                Box::new(ResolvedType::Unknown),
            )),
            "Math__pow" => Some(ResolvedType::Function(
                vec![ResolvedType::Unknown, ResolvedType::Unknown],
                Box::new(ResolvedType::Float),
            )),
            "Math__sqrt" | "Math__sin" | "Math__cos" | "Math__tan" | "Math__floor"
            | "Math__ceil" | "Math__round" | "Math__log" | "Math__log10" | "Math__exp" => Some(
                ResolvedType::Function(vec![ResolvedType::Unknown], Box::new(ResolvedType::Float)),
            ),
            "Math__pi" | "Math__e" | "Math__random" => Some(ResolvedType::Function(
                vec![],
                Box::new(ResolvedType::Float),
            )),
            "Str__len" => Some(ResolvedType::Function(
                vec![ResolvedType::String],
                Box::new(ResolvedType::Integer),
            )),
            "Str__compare" => Some(ResolvedType::Function(
                vec![ResolvedType::String, ResolvedType::String],
                Box::new(ResolvedType::Integer),
            )),
            "Str__concat" => Some(ResolvedType::Function(
                vec![ResolvedType::String, ResolvedType::String],
                Box::new(ResolvedType::String),
            )),
            "Str__upper" | "Str__lower" | "Str__trim" => Some(ResolvedType::Function(
                vec![ResolvedType::String],
                Box::new(ResolvedType::String),
            )),
            "Str__contains" | "Str__startsWith" | "Str__endsWith" => Some(ResolvedType::Function(
                vec![ResolvedType::String, ResolvedType::String],
                Box::new(ResolvedType::Boolean),
            )),
            "to_float" => Some(ResolvedType::Function(
                vec![ResolvedType::Unknown],
                Box::new(ResolvedType::Float),
            )),
            "to_int" => Some(ResolvedType::Function(
                vec![ResolvedType::Unknown],
                Box::new(ResolvedType::Integer),
            )),
            "to_string" => Some(ResolvedType::Function(
                vec![ResolvedType::Unknown],
                Box::new(ResolvedType::String),
            )),
            "assert" | "assert_true" | "assert_false" | "fail" => Some(ResolvedType::Function(
                vec![ResolvedType::Unknown],
                Box::new(ResolvedType::None),
            )),
            "assert_eq" | "assert_ne" => Some(ResolvedType::Function(
                vec![ResolvedType::Unknown, ResolvedType::Unknown],
                Box::new(ResolvedType::None),
            )),
            "range" => Some(ResolvedType::Function(
                vec![ResolvedType::Unknown, ResolvedType::Unknown],
                Box::new(ResolvedType::Unknown),
            )),
            _ => None,
        }
    }

    fn builtin_function_value_concrete_type_for_expected(
        name: &str,
        expected: &ResolvedType,
    ) -> Option<ResolvedType> {
        match (name, expected) {
            ("Option__some", ResolvedType::Function(params, ret))
                if params.len() == 1
                    && matches!(ret.as_ref(), ResolvedType::Option(inner) if params[0] == inner.as_ref().clone()) =>
            {
                Some(expected.clone())
            }
            ("Option__none", ResolvedType::Function(params, ret))
                if params.is_empty() && matches!(ret.as_ref(), ResolvedType::Option(_)) =>
            {
                Some(expected.clone())
            }
            ("Result__ok", ResolvedType::Function(params, ret))
                if params.len() == 1
                    && matches!(ret.as_ref(), ResolvedType::Result(ok, _) if params[0] == ok.as_ref().clone()) =>
            {
                Some(expected.clone())
            }
            ("Result__error", ResolvedType::Function(params, ret))
                if params.len() == 1
                    && matches!(ret.as_ref(), ResolvedType::Result(_, err) if params[0] == err.as_ref().clone()) =>
            {
                Some(expected.clone())
            }
            ("Math__abs", ResolvedType::Function(params, ret))
                if params.len() == 1
                    && matches!(params[0], ResolvedType::Integer)
                    && matches!(ret.as_ref(), ResolvedType::Float) =>
            {
                Some(ResolvedType::Function(
                    vec![ResolvedType::Integer],
                    Box::new(ResolvedType::Integer),
                ))
            }
            ("Math__min" | "Math__max", ResolvedType::Function(params, ret))
                if params.len() == 2
                    && matches!(params[0], ResolvedType::Integer)
                    && matches!(params[1], ResolvedType::Integer)
                    && matches!(ret.as_ref(), ResolvedType::Float) =>
            {
                Some(ResolvedType::Function(
                    vec![ResolvedType::Integer, ResolvedType::Integer],
                    Box::new(ResolvedType::Integer),
                ))
            }
            ("Math__pow", ResolvedType::Function(params, ret))
                if params.len() == 2
                    && params.iter().all(|param| {
                        matches!(param, ResolvedType::Integer | ResolvedType::Float)
                    })
                    && matches!(ret.as_ref(), ResolvedType::Float) =>
            {
                Some(ResolvedType::Function(
                    vec![ResolvedType::Float, ResolvedType::Float],
                    Box::new(ResolvedType::Float),
                ))
            }
            ("Math__abs", ResolvedType::Function(_, _))
            | ("Math__min" | "Math__max", ResolvedType::Function(_, _))
            | ("Math__pow", ResolvedType::Function(_, _)) => None,
            _ => Self::builtin_function_value_type(name),
        }
    }

    fn builtin_zero_arg_value_type_for_expected(
        &self,
        name: &str,
        expected: &ResolvedType,
    ) -> Option<ResolvedType> {
        let synthetic_function_type = ResolvedType::Function(vec![], Box::new(expected.clone()));
        if Self::builtin_matches_expected_function_type(name, &synthetic_function_type) {
            Some(expected.clone())
        } else {
            None
        }
    }

    fn concrete_zero_arg_builtin_value_type(name: &str) -> Option<ResolvedType> {
        match name {
            "read_line" | "System__cwd" | "System__os" => Some(ResolvedType::String),
            "Time__unix" | "Args__count" => Some(ResolvedType::Integer),
            "Math__pi" | "Math__e" | "Math__random" => Some(ResolvedType::Float),
            _ => None,
        }
    }

    fn builtin_argument_expr_type_hint(&self, expr: &Expr) -> Option<ResolvedType> {
        match expr {
            Expr::Ident(_) | Expr::Field { .. } => self
                .resolve_contextual_function_value_name(expr)
                .and_then(|name| Self::concrete_zero_arg_builtin_value_type(&name)),
            Expr::Literal(lit) => Some(match lit {
                Literal::Integer(_) => ResolvedType::Integer,
                Literal::Float(_) => ResolvedType::Float,
                Literal::Boolean(_) => ResolvedType::Boolean,
                Literal::String(_) => ResolvedType::String,
                Literal::Char(_) => ResolvedType::Char,
                Literal::None => ResolvedType::None,
            }),
            Expr::StringInterp(_) => Some(ResolvedType::String),
            Expr::Block(body) => self.builtin_argument_block_type_hint(body),
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_ty = self.builtin_argument_block_type_hint(then_branch)?;
                let else_ty = self.builtin_argument_block_type_hint(else_branch.as_ref()?)?;
                self.common_compatible_type(&then_ty, &else_ty)
            }
            Expr::Match { arms, .. } => {
                let mut arm_types = arms
                    .iter()
                    .filter_map(|arm| self.builtin_argument_block_type_hint(&arm.body));
                let first = arm_types.next()?;
                arm_types.try_fold(first, |acc, ty| self.common_compatible_type(&acc, &ty))
            }
            _ => None,
        }
    }

    fn builtin_argument_block_type_hint(&self, body: &[Spanned<Stmt>]) -> Option<ResolvedType> {
        body.iter().rev().find_map(|stmt| match &stmt.node {
            Stmt::Expr(expr) => self.builtin_argument_expr_type_hint(&expr.node),
            _ => None,
        })
    }

    fn check_builtin_argument_expr(&mut self, expr: &Expr, span: Span) -> ResolvedType {
        if let Some(expected) = self.builtin_argument_expr_type_hint(expr) {
            return self.check_expr_with_expected_type(expr, span, Some(&expected));
        }
        self.check_expr(expr, span)
    }
    fn check_enum_variant_function_value_with_expected_type(
        &mut self,
        expr: &Expr,
        expected: &ResolvedType,
        span: &Span,
    ) -> Option<ResolvedType> {
        let ResolvedType::Function(_, _) = expected else {
            return None;
        };
        let (enum_name, field_types) = self.resolve_enum_variant_function_value(expr)?;
        let expected_return_enum = match expected {
            ResolvedType::Function(_, ret) => match ret.as_ref() {
                ResolvedType::Class(name) => Some(name.clone()),
                _ => None,
            },
            _ => None,
        };
        let actual_ty = if let Some(expected_enum_name) = expected_return_enum {
            let (expected_base, enum_substitutions) =
                self.instantiated_enum_substitutions(&expected_enum_name);
            if expected_base == enum_name {
                let params = field_types
                    .iter()
                    .map(|ty| Self::substitute_type_vars(ty, &enum_substitutions))
                    .collect::<Vec<_>>();
                ResolvedType::Function(params, Box::new(ResolvedType::Class(expected_enum_name)))
            } else {
                ResolvedType::Function(field_types, Box::new(ResolvedType::Class(enum_name)))
            }
        } else {
            ResolvedType::Function(field_types, Box::new(ResolvedType::Class(enum_name)))
        };
        if self.types_compatible(expected, &actual_ty) {
            return Some(actual_ty);
        }
        self.error(
            format!(
                "Type mismatch: expected {}, got {}",
                Self::format_resolved_type_for_diagnostic(expected),
                Self::format_resolved_type_for_diagnostic(&actual_ty)
            ),
            span.clone(),
        );
        Some(ResolvedType::Unknown)
    }

    fn check_class_constructor_call_with_expected_type(
        &mut self,
        callee: &Expr,
        args: &[Spanned<Expr>],
        type_args: &[Type],
        span: Span,
        expected: &ResolvedType,
    ) -> Option<ResolvedType> {
        let ResolvedType::Class(expected_class_name) = expected else {
            return None;
        };

        let mut type_source = Self::nominal_function_value_type_source(callee)?;
        if !type_args.is_empty() {
            type_source = format!(
                "{}<{}>",
                type_source,
                type_args
                    .iter()
                    .map(Self::format_ast_type_source)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        } else {
            let resolved_base = self.resolve_nominal_reference_name(&type_source)?;
            if self.class_base_name(expected_class_name) != self.class_base_name(&resolved_base) {
                return None;
            }
            type_source = expected_class_name.clone();
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
                return Some(ResolvedType::Unknown);
            }
            for (arg, (_, expected_ty)) in args.iter().zip(ctor_params.iter()) {
                let actual = self.check_expr_with_expected_type(
                    &arg.node,
                    arg.span.clone(),
                    Some(expected_ty),
                );
                if !self.types_compatible(expected_ty, &actual) {
                    self.error(
                        format!(
                            "Constructor argument type mismatch: expected {}, got {}",
                            Self::format_resolved_type_for_diagnostic(expected_ty),
                            Self::format_resolved_type_for_diagnostic(&actual)
                        ),
                        arg.span.clone(),
                    );
                }
            }
        } else if !args.is_empty() {
            self.error(
                format!(
                    "Constructor {} expects 0 arguments, got {}",
                    scoped_ty,
                    args.len()
                ),
                span,
            );
            return Some(ResolvedType::Unknown);
        }

        Some(resolved_ctor_type.unwrap_or_else(|| self.parse_type_string(&scoped_ty)))
    }

    /// Check an expression and return its type
    fn check_expr_with_expected_type(
        &mut self,
        expr: &Expr,
        span: Span,
        expected: Option<&ResolvedType>,
    ) -> ResolvedType {
        if let Some(expected_ty) = expected {
            if let Some(actual_ty) =
                self.check_enum_variant_function_value_with_expected_type(expr, expected_ty, &span)
            {
                return actual_ty;
            }
        }
        if let (
            Expr::Call {
                callee,
                args,
                type_args,
            },
            Some(expected_ty),
        ) = (expr, expected)
        {
            if let Some(enum_ty) = self.check_enum_variant_call_with_expected_type(
                &callee.node,
                args,
                type_args,
                span.clone(),
                expected_ty,
            ) {
                return enum_ty;
            }
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
            if let Some(container_ty) = self.check_static_container_call_with_expected_type(
                &callee.node,
                args,
                type_args,
                span.clone(),
                expected_ty,
            ) {
                return container_ty;
            }
            if let Some(class_ty) = self.check_class_constructor_call_with_expected_type(
                &callee.node,
                args,
                type_args,
                span.clone(),
                expected_ty,
            ) {
                return class_ty;
            }
        }
        if let (Expr::Block(body), Some(expected_ty)) = (expr, expected) {
            return self.check_block_expr_with_expected_type(body, expected_ty);
        }
        if let (
            Expr::If {
                condition,
                then_branch,
                else_branch,
            },
            Some(expected_ty),
        ) = (expr, expected)
        {
            return self.check_if_expr_with_expected_type(
                &condition.node,
                condition.span.clone(),
                then_branch,
                else_branch.as_ref(),
                span,
                expected_ty,
            );
        }
        if let (Expr::Match { expr, arms }, Some(expected_ty)) = (expr, expected) {
            return self.check_match_expr_with_expected_type(
                &expr.node,
                expr.span.clone(),
                arms,
                span,
                expected_ty,
            );
        }
        if let (Expr::Lambda { params, body }, Some(expected_ty)) = (expr, expected) {
            if let Some(lambda_ty) =
                self.check_lambda_expr_with_expected_type(params, body, span.clone(), expected_ty)
            {
                return lambda_ty;
            }
        }
        if let Some(expected_ty) = expected {
            if !matches!(expected_ty, ResolvedType::Function(_, _)) {
                if let Some(name) = self.resolve_contextual_function_value_name(expr) {
                    if let Some(actual_ty) =
                        self.builtin_zero_arg_value_type_for_expected(&name, expected_ty)
                    {
                        return actual_ty;
                    }
                }
            }
            if matches!(expected_ty, ResolvedType::Function(_, _)) {
                if let Expr::GenericFunctionValue { callee, type_args } = expr {
                    if let Some(actual_ty) = self.resolve_class_constructor_function_value_type(
                        &callee.node,
                        Some(type_args),
                        Some(expected_ty),
                        span.clone(),
                    ) {
                        return actual_ty;
                    }
                } else if let Some(actual_ty) = self.resolve_class_constructor_function_value_type(
                    expr,
                    None,
                    Some(expected_ty),
                    span.clone(),
                ) {
                    return actual_ty;
                }
                if let Some(name) = self.resolve_contextual_function_value_name(expr) {
                    if self.functions.contains_key(&name) {
                        return self.function_value_type_or_error(&name, span);
                    }
                    if Self::builtin_matches_expected_function_type(&name, expected_ty) {
                        return expected_ty.clone();
                    }
                    let actual_ty =
                        Self::builtin_function_value_concrete_type_for_expected(&name, expected_ty)
                            .or_else(|| Self::builtin_function_value_type(&name))
                            .unwrap_or(ResolvedType::Unknown);
                    if Self::resolved_type_contains_unknown(&actual_ty) {
                        let actual_ty = Self::builtin_function_value_type(&name)
                            .unwrap_or(ResolvedType::Unknown);
                        self.error(
                            format!(
                                "Type mismatch: expected {}, got {}",
                                Self::format_resolved_type_for_diagnostic(expected_ty),
                                Self::format_resolved_type_for_diagnostic(&actual_ty)
                            ),
                            span,
                        );
                        return ResolvedType::Unknown;
                    }
                    if self.types_compatible(expected_ty, &actual_ty) {
                        return actual_ty;
                    }
                }
            }
        }
        if let (Expr::AsyncBlock(body), Some(ResolvedType::Task(expected_inner))) = (expr, expected)
        {
            return self.check_async_block_expr(body, span, Some(expected_inner.as_ref()));
        }
        self.check_expr(expr, span)
    }

    fn check_enum_variant_call_with_expected_type(
        &mut self,
        callee: &Expr,
        args: &[Spanned<Expr>],
        type_args: &[Type],
        span: Span,
        expected: &ResolvedType,
    ) -> Option<ResolvedType> {
        let ResolvedType::Class(expected_enum_name) = expected else {
            return None;
        };
        let (resolved_expected_enum, enum_substitutions) =
            self.instantiated_enum_substitutions(expected_enum_name);
        let (resolved_callee_enum, variant_name) = self.resolve_enum_variant_owner(callee)?;
        if resolved_expected_enum != resolved_callee_enum {
            return None;
        }
        let enum_info = self.enums.get(&resolved_callee_enum)?;
        let variant_fields = enum_info.variants.get(&variant_name).cloned()?;
        if !type_args.is_empty() {
            self.error(
                format!(
                    "Enum variant '{}.{}' does not accept type arguments",
                    resolved_callee_enum, variant_name
                ),
                span.clone(),
            );
        }
        let expected_fields = variant_fields
            .iter()
            .map(|ty| Self::substitute_type_vars(ty, &enum_substitutions))
            .collect::<Vec<_>>();
        if args.len() != expected_fields.len() {
            self.error(
                format!(
                    "Enum variant '{}.{}' expects {} argument(s), got {}",
                    resolved_callee_enum,
                    variant_name,
                    expected_fields.len(),
                    args.len()
                ),
                span,
            );
            return Some(expected.clone());
        }
        for (arg, expected_ty) in args.iter().zip(expected_fields.iter()) {
            let actual =
                self.check_expr_with_expected_type(&arg.node, arg.span.clone(), Some(expected_ty));
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
        Some(expected.clone())
    }

    fn resolve_enum_variant_owner(&self, callee: &Expr) -> Option<(String, String)> {
        if let Expr::Ident(name) = callee {
            if let Some((enum_name, variant_name)) = self.resolve_import_alias_variant(name) {
                return Some((enum_name, variant_name));
            }
        }
        let Expr::Field { object, field } = callee else {
            return None;
        };
        if let Some(path_parts) = flatten_field_chain(callee) {
            if path_parts.len() >= 2 {
                let owner_source = path_parts[..path_parts.len() - 1].join(".");
                if let Some(resolved_owner) = self.resolve_nominal_reference_name(&owner_source) {
                    if self.enums.contains_key(&resolved_owner) {
                        return Some((resolved_owner, field.clone()));
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
        self.enums
            .contains_key(&resolved_owner)
            .then_some((resolved_owner, field.clone()))
    }

    fn check_static_container_call_with_expected_type(
        &mut self,
        callee: &Expr,
        args: &[Spanned<Expr>],
        type_args: &[Type],
        span: Span,
        expected: &ResolvedType,
    ) -> Option<ResolvedType> {
        let path_parts = flatten_field_chain(callee)?;
        if path_parts.len() < 2 {
            return None;
        }
        let owner_name = path_parts[..path_parts.len() - 1].join(".");
        let field = path_parts.last()?.as_str();
        let resolved_builtin = if let Some((alias, member_parts)) = path_parts.split_first() {
            self.resolve_import_alias_module_candidate(alias, member_parts)
                .or_else(|| {
                    crate::ast::builtin_exact_import_alias_canonical(&path_parts.join("."))
                        .map(str::to_string)
                })
        } else {
            None
        };

        match (resolved_builtin.as_deref(), expected) {
            (Some("Option__some"), ResolvedType::Option(inner)) => {
                if !inner.contains_function_type() {
                    return None;
                }
                if !type_args.is_empty() {
                    self.error(
                        "Option static methods do not accept explicit type arguments".to_string(),
                        span.clone(),
                    );
                }
                self.check_arg_count("Option.some", args, 1, span.clone());
                if let Some(arg) = args.first() {
                    let actual = self.check_expr_with_expected_type(
                        &arg.node,
                        arg.span.clone(),
                        Some(inner),
                    );
                    if !self.types_compatible(inner, &actual) {
                        self.error(
                            format!(
                                "Option.some argument type mismatch: expected {}, got {}",
                                Self::format_resolved_type_for_diagnostic(inner),
                                Self::format_resolved_type_for_diagnostic(&actual)
                            ),
                            arg.span.clone(),
                        );
                    }
                }
                Some(expected.clone())
            }
            (Some("Result__ok"), ResolvedType::Result(ok_ty, _)) => {
                if !ok_ty.contains_function_type() {
                    return None;
                }
                if !type_args.is_empty() {
                    self.error(
                        "Result static methods do not accept explicit type arguments".to_string(),
                        span.clone(),
                    );
                }
                self.check_arg_count("Result.ok", args, 1, span.clone());
                if let Some(arg) = args.first() {
                    let actual = self.check_expr_with_expected_type(
                        &arg.node,
                        arg.span.clone(),
                        Some(ok_ty),
                    );
                    if !self.types_compatible(ok_ty, &actual) {
                        self.error(
                            format!(
                                "Result.ok argument type mismatch: expected {}, got {}",
                                Self::format_resolved_type_for_diagnostic(ok_ty),
                                Self::format_resolved_type_for_diagnostic(&actual)
                            ),
                            arg.span.clone(),
                        );
                    }
                }
                Some(expected.clone())
            }
            (Some("Result__error"), ResolvedType::Result(_, err_ty)) => {
                if !err_ty.contains_function_type() {
                    return None;
                }
                if !type_args.is_empty() {
                    self.error(
                        "Result static methods do not accept explicit type arguments".to_string(),
                        span.clone(),
                    );
                }
                self.check_arg_count("Result.error", args, 1, span.clone());
                if let Some(arg) = args.first() {
                    let actual = self.check_expr_with_expected_type(
                        &arg.node,
                        arg.span.clone(),
                        Some(err_ty),
                    );
                    if !self.types_compatible(err_ty, &actual) {
                        self.error(
                            format!(
                                "Result.error argument type mismatch: expected {}, got {}",
                                Self::format_resolved_type_for_diagnostic(err_ty),
                                Self::format_resolved_type_for_diagnostic(&actual)
                            ),
                            arg.span.clone(),
                        );
                    }
                }
                Some(expected.clone())
            }
            _ => match (owner_name.as_str(), field, expected) {
                ("Option", "some", ResolvedType::Option(inner)) => {
                    if !inner.contains_function_type() {
                        return None;
                    }
                    if !type_args.is_empty() {
                        self.error(
                            "Option static methods do not accept explicit type arguments"
                                .to_string(),
                            span.clone(),
                        );
                    }
                    self.check_arg_count("Option.some", args, 1, span.clone());
                    if let Some(arg) = args.first() {
                        let actual = self.check_expr_with_expected_type(
                            &arg.node,
                            arg.span.clone(),
                            Some(inner),
                        );
                        if !self.types_compatible(inner, &actual) {
                            self.error(
                                format!(
                                    "Option.some argument type mismatch: expected {}, got {}",
                                    Self::format_resolved_type_for_diagnostic(inner),
                                    Self::format_resolved_type_for_diagnostic(&actual)
                                ),
                                arg.span.clone(),
                            );
                        }
                    }
                    Some(expected.clone())
                }
                ("Result", "ok", ResolvedType::Result(ok_ty, _)) => {
                    if !ok_ty.contains_function_type() {
                        return None;
                    }
                    if !type_args.is_empty() {
                        self.error(
                            "Result static methods do not accept explicit type arguments"
                                .to_string(),
                            span.clone(),
                        );
                    }
                    self.check_arg_count("Result.ok", args, 1, span.clone());
                    if let Some(arg) = args.first() {
                        let actual = self.check_expr_with_expected_type(
                            &arg.node,
                            arg.span.clone(),
                            Some(ok_ty),
                        );
                        if !self.types_compatible(ok_ty, &actual) {
                            self.error(
                                format!(
                                    "Result.ok argument type mismatch: expected {}, got {}",
                                    Self::format_resolved_type_for_diagnostic(ok_ty),
                                    Self::format_resolved_type_for_diagnostic(&actual)
                                ),
                                arg.span.clone(),
                            );
                        }
                    }
                    Some(expected.clone())
                }
                ("Result", "error", ResolvedType::Result(_, err_ty)) => {
                    if !err_ty.contains_function_type() {
                        return None;
                    }
                    if !type_args.is_empty() {
                        self.error(
                            "Result static methods do not accept explicit type arguments"
                                .to_string(),
                            span.clone(),
                        );
                    }
                    self.check_arg_count("Result.error", args, 1, span.clone());
                    if let Some(arg) = args.first() {
                        let actual = self.check_expr_with_expected_type(
                            &arg.node,
                            arg.span.clone(),
                            Some(err_ty),
                        );
                        if !self.types_compatible(err_ty, &actual) {
                            self.error(
                                format!(
                                    "Result.error argument type mismatch: expected {}, got {}",
                                    Self::format_resolved_type_for_diagnostic(err_ty),
                                    Self::format_resolved_type_for_diagnostic(&actual)
                                ),
                                arg.span.clone(),
                            );
                        }
                    }
                    Some(expected.clone())
                }
                _ => None,
            },
        }
    }

    fn check_block_expr_with_expected_type(
        &mut self,
        body: &[Spanned<Stmt>],
        expected: &ResolvedType,
    ) -> ResolvedType {
        self.enter_scope();
        let mut result_type = ResolvedType::None;
        for stmt in body {
            match &stmt.node {
                Stmt::Expr(expr) => {
                    result_type = self.check_expr_with_expected_type(
                        &expr.node,
                        expr.span.clone(),
                        Some(expected),
                    );
                }
                _ => self.check_stmt(&stmt.node, stmt.span.clone()),
            }
        }
        self.exit_scope();
        result_type
    }

    fn check_if_expr_with_expected_type(
        &mut self,
        condition: &Expr,
        condition_span: Span,
        then_branch: &[Spanned<Stmt>],
        else_branch: Option<&Vec<Spanned<Stmt>>>,
        span: Span,
        expected: &ResolvedType,
    ) -> ResolvedType {
        let cond_type = self.check_expr(condition, condition_span.clone());
        if !matches!(cond_type, ResolvedType::Unknown)
            && !matches!(cond_type, ResolvedType::Boolean)
        {
            self.error(
                format!(
                    "If condition must be Boolean, got {}",
                    Self::format_resolved_type_for_diagnostic(&cond_type)
                ),
                condition_span,
            );
        }

        let then_type = self.check_block_expr_with_expected_type(then_branch, expected);
        let else_type =
            else_branch.map(|branch| self.check_block_expr_with_expected_type(branch, expected));

        match else_type {
            Some(else_type) => self
                .common_compatible_type(&then_type, &else_type)
                .unwrap_or_else(|| {
                    self.error(
                        format!(
                            "If expression branch type mismatch: then is {}, else is {}",
                            Self::format_resolved_type_for_diagnostic(&then_type),
                            Self::format_resolved_type_for_diagnostic(&else_type)
                        ),
                        span,
                    );
                    ResolvedType::Unknown
                }),
            None => ResolvedType::None,
        }
    }

    fn check_match_expr_with_expected_type(
        &mut self,
        expr: &Expr,
        expr_span: Span,
        arms: &[MatchArm],
        span: Span,
        expected: &ResolvedType,
    ) -> ResolvedType {
        let match_type = self.check_builtin_argument_expr(expr, expr_span);
        let mut result_type: Option<ResolvedType> = None;

        for arm in arms {
            self.enter_scope();
            self.check_pattern(&arm.pattern, &match_type, span.clone());
            let arm_type = self.check_block_expr_with_expected_type(&arm.body, expected);
            self.exit_scope();

            if let Some(current) = &result_type {
                if let Some(common_type) = self.common_compatible_type(current, &arm_type) {
                    result_type = Some(common_type);
                } else {
                    self.error(
                        format!(
                            "Match expression arm type mismatch: expected {}, got {}",
                            Self::format_resolved_type_for_diagnostic(current),
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

    fn check_lambda_expr_with_expected_type(
        &mut self,
        params: &[Parameter],
        body: &Spanned<Expr>,
        span: Span,
        expected: &ResolvedType,
    ) -> Option<ResolvedType> {
        let ResolvedType::Function(expected_params, expected_return) = expected else {
            return None;
        };

        if params.len() != expected_params.len() {
            self.error(
                format!(
                    "Lambda parameter count mismatch: expected {}, got {}",
                    expected_params.len(),
                    params.len()
                ),
                span,
            );
            return Some(ResolvedType::Unknown);
        }

        self.enter_scope();
        let saved_return_type = self.current_return_type.clone();
        self.current_return_type = None;

        let param_types = params
            .iter()
            .zip(expected_params.iter())
            .map(|(param, expected_param)| {
                let resolved_param_type = if matches!(param.ty, Type::None) {
                    expected_param.clone()
                } else {
                    self.resolve_type(&param.ty)
                };
                self.validate_resolved_type_exists(&resolved_param_type, span.clone());
                self.check_type_visibility(&resolved_param_type, span.clone());
                if !matches!(param.ty, Type::None)
                    && !self.types_compatible(expected_param, &resolved_param_type)
                {
                    self.error(
                        format!(
                            "Lambda parameter type mismatch: expected {}, got {}",
                            Self::format_resolved_type_for_diagnostic(expected_param),
                            Self::format_resolved_type_for_diagnostic(&resolved_param_type)
                        ),
                        span.clone(),
                    );
                }
                self.declare_variable(
                    &param.name,
                    resolved_param_type.clone(),
                    param.mutable,
                    span.clone(),
                );
                resolved_param_type
            })
            .collect::<Vec<_>>();

        let return_type = self.check_expr_with_expected_type(
            &body.node,
            body.span.clone(),
            Some(expected_return.as_ref()),
        );

        self.current_return_type = saved_return_type;
        self.exit_scope();

        Some(ResolvedType::Function(param_types, Box::new(return_type)))
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
        self.current_async_return_type =
            Some(expected_inner.cloned().unwrap_or(ResolvedType::None));

        for stmt in body {
            match &stmt.node {
                Stmt::Expr(expr) => {
                    tail_expr_type = Some(if let Some(expected_inner) = expected_inner {
                        self.check_expr_with_expected_type(
                            &expr.node,
                            expr.span.clone(),
                            Some(expected_inner),
                        )
                    } else {
                        self.check_builtin_argument_expr(&expr.node, expr.span.clone())
                    });
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

        let mut shadowed_outer_names = std::collections::HashSet::new();
        for scope in captured_outer_scopes.iter().rev() {
            for (name, var) in &scope.variables {
                if shadowed_outer_names.contains(name) {
                    continue;
                }
                if Self::type_contains_borrowed_reference(&var.ty)
                    && Self::block_mentions_ident_with_shadowing(
                        body,
                        name,
                        &mut shadowed_outer_names.clone(),
                    )
                {
                    self.error(
                        format!(
                            "Async block cannot capture '{}' because its type contains borrowed references: {}",
                            name,
                            Self::format_resolved_type_for_diagnostic(&var.ty)
                        ),
                        span.clone(),
                    );
                }
            }
            shadowed_outer_names.extend(scope.variables.keys().cloned());
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
                        Self::format_resolved_type_for_diagnostic(expected_inner),
                        Self::format_resolved_type_for_diagnostic(&return_type)
                    ),
                    span,
                );
            }
            return ResolvedType::Task(Box::new(expected_inner.clone()));
        }

        ResolvedType::Task(Box::new(return_type))
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
            .map(|arg| self.check_builtin_argument_expr(&arg.node, arg.span.clone()))
            .collect();
        let has_unknown_arg = arg_types
            .iter()
            .any(|arg_type| matches!(arg_type, ResolvedType::Unknown));

        // List<T>(N) supports optional integer preallocation size; otherwise List<T>().
        if ty_name == "List"
            || ty_name.starts_with("List<")
            || matches!(resolved, ResolvedType::List(_))
        {
            match arg_types.as_slice() {
                [] => {}
                [ResolvedType::Integer] => {
                    self.check_non_negative_integer_const(
                        &args[0].node,
                        args[0].span.clone(),
                        "List constructor capacity cannot be negative",
                    );
                }
                [ResolvedType::Unknown] => {}
                [other] => {
                    self.error(
                        format!(
                            "Constructor {} expects optional Integer capacity, got {}",
                            format_diagnostic_class_name(ty_name),
                            Self::format_resolved_type_for_diagnostic(other)
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

        let allows_optional_single_value_arg = ty_name == "Box"
            || ty_name.starts_with("Box<")
            || ty_name == "Rc"
            || ty_name.starts_with("Rc<")
            || ty_name == "Arc"
            || ty_name.starts_with("Arc<")
            || matches!(
                resolved,
                ResolvedType::Box(_) | ResolvedType::Rc(_) | ResolvedType::Arc(_)
            );
        if allows_optional_single_value_arg {
            if args.len() > 1 {
                self.error(
                    format!(
                        "Constructor {} expects 0 or 1 arguments, got {}",
                        ty_name,
                        args.len()
                    ),
                    span,
                );
            }
            return;
        }

        let non_constructible_builtin = ty_name == "Ptr"
            || ty_name.starts_with("Ptr<")
            || ty_name == "Task"
            || ty_name.starts_with("Task<")
            || ty_name == "Range"
            || ty_name.starts_with("Range<")
            || matches!(
                resolved,
                ResolvedType::Ptr(_) | ResolvedType::Task(_) | ResolvedType::Range(_)
            );
        if non_constructible_builtin {
            self.error(
                format!(
                    "Cannot construct built-in type '{}'",
                    format_diagnostic_class_name(ty_name)
                ),
                span,
            );
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
            if has_unknown_arg {
                return;
            }
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
                    self.error(
                        format!("Unknown type: {}", format_diagnostic_class_name(name)),
                        span,
                    );
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

    fn resolved_type_exists(&self, ty: &ResolvedType) -> bool {
        match ty {
            ResolvedType::Class(name) => {
                let base_name = self.class_base_name(name);
                self.classes.contains_key(base_name)
                    || self.interfaces.contains_key(base_name)
                    || self.enums.contains_key(base_name)
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
            | ResolvedType::Range(inner) => self.resolved_type_exists(inner),
            ResolvedType::Result(ok, err) | ResolvedType::Map(ok, err) => {
                self.resolved_type_exists(ok) && self.resolved_type_exists(err)
            }
            ResolvedType::Function(params, ret) => {
                params.iter().all(|param| self.resolved_type_exists(param))
                    && self.resolved_type_exists(ret)
            }
            ResolvedType::Integer
            | ResolvedType::Float
            | ResolvedType::Boolean
            | ResolvedType::String
            | ResolvedType::Char
            | ResolvedType::None
            | ResolvedType::TypeVar(_)
            | ResolvedType::Unknown => true,
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
                let resolved_args = split_generic_args_static(inner)
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
                        name,
                        Self::format_resolved_type_for_diagnostic(&resolved),
                        bounds.replace("__", ".")
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

    fn check_variadic_ffi_tail_args(
        &mut self,
        callee: &str,
        args: &[Spanned<Expr>],
        fixed_count: usize,
    ) {
        for arg in args.iter().skip(fixed_count) {
            let t = self.check_builtin_argument_expr(&arg.node, arg.span.clone());
            if !self.is_ffi_safe_type(&t) {
                self.error(
                    format!(
                        "Variadic extern call '{}' received non-FFI-safe variadic argument type {}",
                        callee,
                        Self::format_resolved_type_for_diagnostic(&t)
                    ),
                    arg.span.clone(),
                );
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
                    if let Some(sig) = self.lookup_interface_method(name, field) {
                        let params = sig
                            .params
                            .iter()
                            .map(|(_, ty)| ty.clone())
                            .collect::<Vec<_>>();
                        return ResolvedType::Function(params, Box::new(sig.return_type.clone()));
                    }
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
                if let Some((owner, method_sig, visibility)) =
                    self.lookup_class_method(&base_name, field)
                {
                    self.check_member_visibility(&owner, visibility, "Method", field, span.clone());
                    let params = method_sig
                        .params
                        .iter()
                        .map(|(_, ty)| Self::substitute_type_vars(ty, &class_substitutions))
                        .collect::<Vec<_>>();
                    let ret =
                        Self::substitute_type_vars(&method_sig.return_type, &class_substitutions);
                    return ResolvedType::Function(params, Box::new(ret));
                }
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
            ResolvedType::Unknown => ResolvedType::Unknown,
            _ => {
                self.error(
                    format!(
                        "Cannot access field on type {}",
                        Self::format_resolved_type_for_diagnostic(obj_type)
                    ),
                    span,
                );
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
        if matches!(left, ResolvedType::Unknown) || matches!(right, ResolvedType::Unknown) {
            return match op {
                BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                    ResolvedType::Boolean
                }
                _ => ResolvedType::Unknown,
            };
        }

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
                            Self::format_resolved_type_for_diagnostic(left),
                            Self::format_resolved_type_for_diagnostic(right)
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
                    self.error(
                        format!(
                            "Cannot compare {} and {}",
                            Self::format_resolved_type_for_diagnostic(left),
                            Self::format_resolved_type_for_diagnostic(right)
                        ),
                        span,
                    );
                }
                ResolvedType::Boolean
            }
            BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                if !left.supports_ordered_comparison_with(right) {
                    self.error(
                        format!(
                            "Comparison requires ordered types, got {} and {}",
                            Self::format_resolved_type_for_diagnostic(left),
                            Self::format_resolved_type_for_diagnostic(right)
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
                            Self::format_resolved_type_for_diagnostic(left),
                            Self::format_resolved_type_for_diagnostic(right)
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
    /// Get type of a literal
    fn literal_type(lit: &Literal) -> ResolvedType {
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
        self.declare_variable_with_contract(name, ty, mutable, span, None);
    }

    fn declare_variable_with_contract(
        &mut self,
        name: &str,
        ty: ResolvedType,
        mutable: bool,
        span: Span,
        callable_effects: Option<FunctionEffectContract>,
    ) {
        let _ = span;
        let var = VarInfo {
            ty,
            mutable,
            callable_effects,
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

    fn lookup_variable_mut(&mut self, name: &str) -> Option<&mut VarInfo> {
        let mut scope_idx = self.current_scope;
        loop {
            if self.scopes[scope_idx].variables.contains_key(name) {
                return self.scopes[scope_idx].variables.get_mut(name);
            }
            if let Some(parent) = self.scopes[scope_idx].parent {
                scope_idx = parent;
            } else {
                break;
            }
        }
        None
    }

    fn update_variable_callable_effects(
        &mut self,
        name: &str,
        callable_effects: Option<FunctionEffectContract>,
    ) {
        if let Some(var) = self.lookup_variable_mut(name) {
            var.callable_effects = callable_effects;
        }
    }

    fn infer_function_value_effect_contract(
        &self,
        value_expr: &Expr,
        expected_type: &ResolvedType,
    ) -> Option<FunctionEffectContract> {
        if !matches!(expected_type, ResolvedType::Function(_, _)) {
            return None;
        }
        self.infer_function_value_effect_contract_from_expr(value_expr)
    }

    fn infer_function_value_effect_contract_from_block(
        &self,
        block: &Block,
    ) -> Option<FunctionEffectContract> {
        let tail_expr = block.iter().rev().find_map(|stmt| match &stmt.node {
            Stmt::Expr(expr) => Some(&expr.node),
            _ => None,
        })?;
        self.infer_function_value_effect_contract_from_expr(tail_expr)
    }

    fn merge_function_effect_contracts(
        left: Option<FunctionEffectContract>,
        right: Option<FunctionEffectContract>,
    ) -> Option<FunctionEffectContract> {
        match (left, right) {
            (None, None) => None,
            (Some(contract), None) | (None, Some(contract)) => Some(contract),
            (Some(FunctionEffectContract::Any), _) | (_, Some(FunctionEffectContract::Any)) => {
                Some(FunctionEffectContract::Any)
            }
            (Some(FunctionEffectContract::Unknown), _)
            | (_, Some(FunctionEffectContract::Unknown)) => Some(FunctionEffectContract::Unknown),
            (
                Some(FunctionEffectContract::Effects(mut a)),
                Some(FunctionEffectContract::Effects(b)),
            ) => {
                a.extend(b);
                a.sort();
                a.dedup();
                Some(FunctionEffectContract::Effects(a))
            }
        }
    }

    fn infer_function_value_effect_contract_from_expr(
        &self,
        value_expr: &Expr,
    ) -> Option<FunctionEffectContract> {
        if let Expr::Ident(name) = value_expr {
            if let Some(contract) = self
                .lookup_variable(name)
                .and_then(|var| var.callable_effects.clone())
            {
                return Some(contract);
            }
        }
        match value_expr {
            Expr::Call { .. } | Expr::Await(_) => {
                return Some(FunctionEffectContract::Unknown);
            }
            Expr::GenericFunctionValue { callee, .. } => {
                return self.infer_function_value_effect_contract_from_expr(&callee.node);
            }
            Expr::Field { object, field } => {
                if let Some(contract) =
                    self.infer_bound_method_value_effect_contract(&object.node, field)
                {
                    return Some(contract);
                }
            }
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_contract =
                    self.infer_function_value_effect_contract_from_block(then_branch);
                let else_contract = else_branch
                    .as_ref()
                    .and_then(|block| self.infer_function_value_effect_contract_from_block(block));
                return Self::merge_function_effect_contracts(then_contract, else_contract);
            }
            Expr::Match { arms, .. } => {
                let mut merged: Option<FunctionEffectContract> = None;
                for arm in arms {
                    let contract = self.infer_function_value_effect_contract_from_block(&arm.body);
                    merged = Self::merge_function_effect_contracts(merged, contract);
                }
                return merged;
            }
            Expr::Block(body) => {
                return self.infer_function_value_effect_contract_from_block(body);
            }
            _ => {}
        }

        let name = self.resolve_contextual_function_value_name(value_expr)?;
        if let Some(sig) = self.functions.get(&name) {
            if sig.allow_any {
                return Some(FunctionEffectContract::Any);
            }
            if !sig.effects.is_empty() {
                return Some(FunctionEffectContract::Effects(sig.effects.clone()));
            }
            return None;
        }
        Self::builtin_required_effect(&name)
            .map(|effect| FunctionEffectContract::Effects(vec![effect.to_string()]))
    }

    fn infer_bound_method_value_effect_contract(
        &self,
        object_expr: &Expr,
        method_name: &str,
    ) -> Option<FunctionEffectContract> {
        let receiver_class = self.infer_receiver_class_name_for_method_value(object_expr)?;
        let receiver_base = self.class_base_name(&receiver_class);
        if self.interfaces.contains_key(receiver_base) {
            let mut methods = HashMap::new();
            let mut visited = std::collections::HashSet::new();
            self.collect_interface_methods(&receiver_class, &mut methods, &mut visited);
            if methods.contains_key(method_name) {
                return Some(FunctionEffectContract::Unknown);
            }
        }
        let (base_name, _) = self.instantiated_class_substitutions(&receiver_class);
        let (_owner, sig, _visibility) = self.lookup_class_method(&base_name, method_name)?;
        if sig.allow_any {
            return Some(FunctionEffectContract::Any);
        }
        if !sig.effects.is_empty() {
            return Some(FunctionEffectContract::Effects(sig.effects.clone()));
        }
        None
    }

    fn infer_receiver_class_name_for_method_value(&self, object_expr: &Expr) -> Option<String> {
        match object_expr {
            Expr::Ident(name) => {
                let receiver_ty = self.lookup_variable(name).map(|var| &var.ty)?;
                match Self::peel_reference_type(receiver_ty) {
                    ResolvedType::Class(name) => Some(name.clone()),
                    _ => None,
                }
            }
            Expr::This => self.current_class.clone(),
            Expr::Construct { ty, .. } => self
                .resolve_nominal_reference_name(ty)
                .or_else(|| Some(ty.clone()))
                .and_then(|resolved| {
                    let base = self.class_base_name(&resolved).to_string();
                    self.classes.contains_key(&base).then_some(resolved)
                }),
            Expr::Call { callee, .. } => self.infer_callable_return_class_name(&callee.node),
            Expr::GenericFunctionValue { callee, .. } => {
                self.infer_callable_return_class_name(&callee.node)
            }
            Expr::Block(body) => {
                let tail_expr = body.iter().rev().find_map(|stmt| match &stmt.node {
                    Stmt::Expr(expr) => Some(&expr.node),
                    _ => None,
                })?;
                self.infer_receiver_class_name_for_method_value(tail_expr)
            }
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_expr = then_branch.iter().rev().find_map(|stmt| match &stmt.node {
                    Stmt::Expr(expr) => Some(&expr.node),
                    _ => None,
                });
                let else_expr = else_branch.as_ref().and_then(|branch| {
                    branch.iter().rev().find_map(|stmt| match &stmt.node {
                        Stmt::Expr(expr) => Some(&expr.node),
                        _ => None,
                    })
                });
                let then_class = then_expr
                    .and_then(|expr| self.infer_receiver_class_name_for_method_value(expr));
                let else_class = else_expr
                    .and_then(|expr| self.infer_receiver_class_name_for_method_value(expr));
                Self::merge_inferred_receiver_classes(then_class, else_class)
            }
            Expr::Match { arms, .. } => {
                let mut merged: Option<String> = None;
                for arm in arms {
                    let arm_expr = arm.body.iter().rev().find_map(|stmt| match &stmt.node {
                        Stmt::Expr(expr) => Some(&expr.node),
                        _ => None,
                    });
                    let arm_class = arm_expr
                        .and_then(|expr| self.infer_receiver_class_name_for_method_value(expr));
                    merged = Self::merge_inferred_receiver_classes(merged, arm_class);
                }
                merged
            }
            _ => None,
        }
    }

    fn infer_callable_return_class_name(&self, callee_expr: &Expr) -> Option<String> {
        if let Some(callee_name) = self.resolve_contextual_function_value_name(callee_expr) {
            if let Some(sig) = self.functions.get(&callee_name) {
                if let ResolvedType::Class(name) = Self::peel_reference_type(&sig.return_type) {
                    return Some(name.clone());
                }
            }
        }
        if let Expr::Field { object, field } = callee_expr {
            let receiver = self.infer_receiver_class_name_for_method_value(&object.node)?;
            let (base_name, _) = self.instantiated_class_substitutions(&receiver);
            let (_owner, sig, _visibility) = self.lookup_class_method(&base_name, field)?;
            if let ResolvedType::Class(name) = Self::peel_reference_type(&sig.return_type) {
                return Some(name.clone());
            }
        }
        None
    }

    fn merge_inferred_receiver_classes(
        left: Option<String>,
        right: Option<String>,
    ) -> Option<String> {
        match (left, right) {
            (None, None) => None,
            (Some(name), None) | (None, Some(name)) => Some(name),
            (Some(a), Some(b)) => {
                let a_base = a.split('<').next().unwrap_or(&a);
                let b_base = b.split('<').next().unwrap_or(&b);
                if a_base == b_base {
                    Some(a)
                } else {
                    None
                }
            }
        }
    }

    fn enforce_function_argument_effect_contract(
        &mut self,
        param_type: &ResolvedType,
        arg_expr: &Expr,
        arg_span: Span,
    ) {
        if !matches!(param_type, ResolvedType::Function(_, _)) {
            return;
        }
        let Some(contract) = self.infer_function_value_effect_contract_from_expr(arg_expr) else {
            return;
        };
        self.enforce_function_value_effect_contract(&contract, arg_span, "function argument");
    }

    /// Parse a type string like "Integer" or "List<Integer>"
    /// Split generic arguments by comma, respecting nested < >
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

mod check;
mod collect;
mod display;
mod effects;
mod expr_calls;
mod resolve;
#[cfg(test)]
#[path = "../tests/typeck_cases/mod.rs"]
mod tests;

pub(crate) use display::format_errors;
