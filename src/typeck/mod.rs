//! Apex Type Checker - Semantic analysis with type inference
//!
//! This module provides:
//! - Type checking and inference
//! - Symbol table management
//! - Scope tracking
//! - Type error reporting with source locations

#![allow(dead_code)]

use crate::ast::*;
use crate::parse_type_source;
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

fn format_diagnostic_class_name(name: &str) -> String {
    if let Some(open_bracket) = name.find('<') {
        if name.ends_with('>') {
            let base = &name[..open_bracket];
            let inner = &name[open_bracket + 1..name.len() - 1];
            let formatted_args = split_generic_args_static(inner)
                .into_iter()
                .map(|arg| format_diagnostic_class_name(&arg))
                .collect::<Vec<_>>()
                .join(", ");
            return format!("{}<{}>", base.replace("__", "."), formatted_args);
        }
    }

    name.replace("__", ".")
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
    /// Import aliases (alias -> scoped import paths)
    import_aliases: HashMap<String, Vec<(Option<String>, String)>>,
    /// Current function declared effects
    current_effects: Vec<String>,
    /// Whether current function is declared pure
    current_is_pure: bool,
    /// Whether current function allows any effects
    current_allow_any: bool,
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
            current_generic_type_bindings: HashMap::new(),
            type_var_bounds: HashMap::new(),
            current_module_prefix: None,
        }
    }
    #[allow(clippy::only_used_in_recursion)]
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
            Expr::IfExpr {
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

    fn expr_mentions_ident(expr: &Expr, ident: &str) -> bool {
        Self::expr_mentions_ident_with_shadowing(expr, ident, &std::collections::HashSet::new())
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

    fn stmt_mentions_ident(stmt: &Stmt, ident: &str) -> bool {
        Self::stmt_mentions_ident_with_shadowing(stmt, ident, &mut std::collections::HashSet::new())
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
            self.current_allow_any = true;
            self.current_is_pure = false;
            for param in &method.params {
                let ty = self.resolve_type(&param.ty);
                self.validate_resolved_type_exists(&ty, span.clone());
                self.declare_variable(&param.name, ty, param.mutable, span.clone());
            }
            let return_type = self.resolve_type(&method.return_type);
            self.validate_resolved_type_exists(&return_type, span.clone());
            self.current_return_type = Some(return_type);
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
            self.validate_resolved_type_exists(&ty, span.clone());
            self.check_type_visibility(&ty, span.clone());
            if func.is_async && Self::type_contains_borrowed_reference(&ty) {
                self.error(
                    format!(
                        "Async function '{}' cannot accept a parameter containing borrowed references: {}",
                        func.name,
                        Self::format_resolved_type_for_diagnostic(&ty)
                    ),
                    span.clone(),
                );
            }
            self.declare_variable(&param.name, ty, param.mutable, span.clone());
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
                self.validate_resolved_type_exists(&ty, span.clone());
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
                self.validate_resolved_type_exists(&ty, span.clone());
                self.check_type_visibility(&ty, span.clone());
                self.declare_variable(&param.name, ty, param.mutable, span.clone());
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
                let suppress_assignment_mismatch = matches!(&value.node, Expr::IfExpr { .. })
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

                self.declare_variable(name, expected_type, *mutable, span);
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
            }

            Stmt::Expr(expr) => {
                self.check_expr(&expr.node, expr.span.clone());
            }

            Stmt::Return(expr) => {
                let expected_return_type = self.current_return_type.clone().or_else(|| {
                    self.current_async_return_type.clone().and_then(|ty| {
                        (!matches!(ty, ResolvedType::None)).then_some(ty)
                    })
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
            let (enum_name, variant_name) = this.resolve_import_alias_variant(name)?;
            let enum_info = this.enums.get(&enum_name)?;
            enum_info
                .variants
                .get(&variant_name)
                .is_some_and(|fields| fields.is_empty())
                .then_some((enum_name, variant_name))
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
                let lit_type = self.literal_type(lit);
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
    fn is_contextual_static_container_function_value(name: &str) -> bool {
        matches!(
            name,
            "Option__some" | "Option__none" | "Result__ok" | "Result__error"
        )
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
        let synthetic_function_type =
            ResolvedType::Function(vec![], Box::new(expected.clone()));
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
            Expr::IfExpr {
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

    fn builtin_argument_block_type_hint(
        &self,
        body: &[Spanned<Stmt>],
    ) -> Option<ResolvedType> {
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
        let actual_ty =
            ResolvedType::Function(field_types, Box::new(ResolvedType::Class(enum_name)));
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
            Expr::IfExpr {
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

    fn check_static_container_call_with_expected_type(
        &mut self,
        callee: &Expr,
        args: &[Spanned<Expr>],
        type_args: &[Type],
        span: Span,
        expected: &ResolvedType,
    ) -> Option<ResolvedType> {
        let Expr::Field { object, field } = callee else {
            return None;
        };
        let Expr::Ident(owner_name) = &object.node else {
            return None;
        };

        match (owner_name.as_str(), field.as_str(), expected) {
            ("Option", "some", ResolvedType::Option(inner)) => {
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
            ("Result", "ok", ResolvedType::Result(ok_ty, _)) => {
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
            ("Result", "error", ResolvedType::Result(_, err_ty)) => {
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
            _ => None,
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
        self.current_async_return_type = Some(
            expected_inner.cloned().unwrap_or(ResolvedType::None),
        );

        for stmt in body {
            match &stmt.node {
                Stmt::Expr(expr) => {
                    tail_expr_type = Some(
                        self.check_builtin_argument_expr(&expr.node, expr.span.clone()),
                    );
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
            } => self.check_call(&callee.node, args, type_args, span),

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

            Expr::IfExpr {
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

    /// Check a function/method call
    fn check_call(
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
                    let t1 =
                        self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    let t2 =
                        self.check_builtin_argument_expr(&args[1].node, args[1].span.clone());
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
                    let t1 =
                        self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    let t2 =
                        self.check_builtin_argument_expr(&args[1].node, args[1].span.clone());
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
                    let t1 =
                        self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    let t2 =
                        self.check_builtin_argument_expr(&args[1].node, args[1].span.clone());
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
                    let t1 =
                        self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    let t2 =
                        self.check_builtin_argument_expr(&args[1].node, args[1].span.clone());
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
                    let t1 =
                        self.check_builtin_argument_expr(&args[0].node, args[0].span.clone());
                    let t2 =
                        self.check_builtin_argument_expr(&args[1].node, args[1].span.clone());
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
                if !left.is_numeric() || !right.is_numeric() {
                    self.error(
                        format!(
                            "Comparison requires numeric types, got {} and {}",
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
    #[allow(clippy::only_used_in_recursion)]
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
mod resolve;
#[cfg(test)]
mod tests;

pub(crate) use display::format_errors;
