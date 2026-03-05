//! Apex Type Checker - Semantic analysis with type inference
//!
//! This module provides:
//! - Type checking and inference
//! - Symbol table management
//! - Scope tracking
//! - Type error reporting with source locations

#![allow(dead_code)]

use crate::ast::*;
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
    pub is_variadic: bool,
    pub is_extern: bool,
    pub effects: Vec<String>,
    pub is_pure: bool,
    pub allow_any: bool,
    pub has_explicit_effects: bool,
    pub span: Span,
}

/// Class information
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub fields: HashMap<String, (ResolvedType, bool)>, // (type, mutable)
    pub methods: HashMap<String, FuncSig>,
    pub constructor: Option<Vec<(String, ResolvedType)>>,
    pub span: Span,
}

/// Enum metadata used for type checking variant constructors and pattern matching
#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub variants: HashMap<String, Vec<ResolvedType>>,
    pub span: Span,
}

/// Scope for symbol table
#[derive(Debug)]
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
    /// Current function declared effects
    current_effects: Vec<String>,
    /// Whether current function is declared pure
    current_is_pure: bool,
    /// Whether current function allows any effects
    current_allow_any: bool,
    /// Source code for error messages
    source: String,
}

impl TypeChecker {
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
            enum_variant_to_enum: HashMap::new(),
            type_var_counter: 0,
            substitutions: HashMap::new(),
            errors: Vec::new(),
            current_return_type: None,
            current_effects: Vec::new(),
            current_is_pure: false,
            current_allow_any: false,
            source,
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
            Expr::Call { callee, args } => {
                if let Expr::Ident(name) = &callee.node {
                    if let Some(required) = Self::builtin_required_effect(name) {
                        out.insert(required.to_string());
                    }
                    if let Some(sig) = self.functions.get(name) {
                        for eff in &sig.effects {
                            out.insert(eff.clone());
                        }
                    }
                } else if let Expr::Field { object, field } = &callee.node {
                    if let Expr::Ident(name) = &object.node {
                        let builtin_name = format!("{}__{}", name, field);
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

    /// Run type checking on a program
    pub fn check(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        // First pass: collect all declarations
        self.collect_declarations(program);
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
                Decl::Function(func) => {
                    self.validate_effect_attributes(
                        &func.attributes,
                        decl.span.clone(),
                        &format!("Function '{}'", func.name),
                    );
                    self.validate_extern_signature(func, decl.span.clone());
                    let params: Vec<(String, ResolvedType)> = func
                        .params
                        .iter()
                        .map(|p| (p.name.clone(), self.resolve_type(&p.ty)))
                        .collect();
                    let mut return_type = self.resolve_type(&func.return_type);
                    if func.is_async && !matches!(return_type, ResolvedType::Task(_)) {
                        return_type = ResolvedType::Task(Box::new(return_type));
                    }
                    let (effects, is_pure, allow_any, has_explicit_effects) =
                        self.parse_effects_from_attributes(&func.attributes);

                    self.functions.insert(
                        func.name.clone(),
                        FuncSig {
                            params,
                            return_type,
                            is_variadic: func.is_variadic,
                            is_extern: func.is_extern,
                            effects,
                            is_pure,
                            allow_any,
                            has_explicit_effects,
                            span: decl.span.clone(),
                        },
                    );
                }
                Decl::Class(class) => {
                    let mut fields = HashMap::new();
                    for field in &class.fields {
                        fields.insert(
                            field.name.clone(),
                            (self.resolve_type(&field.ty), field.mutable),
                        );
                    }

                    let mut methods = HashMap::new();
                    for method in &class.methods {
                        self.validate_effect_attributes(
                            &method.attributes,
                            decl.span.clone(),
                            &format!("Method '{}.{}'", class.name, method.name),
                        );
                        let params: Vec<(String, ResolvedType)> = method
                            .params
                            .iter()
                            .map(|p| (p.name.clone(), self.resolve_type(&p.ty)))
                            .collect();

                        let mut return_type = self.resolve_type(&method.return_type);
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
                                is_variadic: method.is_variadic,
                                is_extern: method.is_extern,
                                effects,
                                is_pure,
                                allow_any,
                                has_explicit_effects,
                                span: decl.span.clone(),
                            },
                        );
                    }

                    let constructor = class.constructor.as_ref().map(|c| {
                        c.params
                            .iter()
                            .map(|p| (p.name.clone(), self.resolve_type(&p.ty)))
                            .collect()
                    });

                    self.classes.insert(
                        class.name.clone(),
                        ClassInfo {
                            fields,
                            methods,
                            constructor,
                            span: decl.span.clone(),
                        },
                    );
                }
                Decl::Enum(en) => {
                    let mut variants = HashMap::new();
                    for variant in &en.variants {
                        let fields = variant
                            .fields
                            .iter()
                            .map(|f| self.resolve_type(&f.ty))
                            .collect::<Vec<_>>();
                        variants.insert(variant.name.clone(), fields);
                        self.enum_variant_to_enum
                            .insert(variant.name.clone(), en.name.clone());
                    }
                    self.enums.insert(
                        en.name.clone(),
                        EnumInfo {
                            variants,
                            span: decl.span.clone(),
                        },
                    );
                }
                Decl::Module(module) => {
                    // Collect module functions with prefixed names
                    for inner_decl in &module.declarations {
                        if let Decl::Function(func) = &inner_decl.node {
                            self.validate_effect_attributes(
                                &func.attributes,
                                inner_decl.span.clone(),
                                &format!("Function '{}__{}'", module.name, func.name),
                            );
                            self.validate_extern_signature(func, inner_decl.span.clone());
                            let prefixed_name = format!("{}__{}", module.name, func.name);
                            let params: Vec<(String, ResolvedType)> = func
                                .params
                                .iter()
                                .map(|p| (p.name.clone(), self.resolve_type(&p.ty)))
                                .collect();
                            let mut return_type = self.resolve_type(&func.return_type);
                            if func.is_async && !matches!(return_type, ResolvedType::Task(_)) {
                                return_type = ResolvedType::Task(Box::new(return_type));
                            }
                            let (effects, is_pure, allow_any, has_explicit_effects) =
                                self.parse_effects_from_attributes(&func.attributes);

                            self.functions.insert(
                                prefixed_name,
                                FuncSig {
                                    params,
                                    return_type,
                                    is_variadic: func.is_variadic,
                                    is_extern: func.is_extern,
                                    effects,
                                    is_pure,
                                    allow_any,
                                    has_explicit_effects,
                                    span: inner_decl.span.clone(),
                                },
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Check a declaration
    fn check_decl_with_prefix(&mut self, decl: &Decl, span: Span, module_prefix: Option<&str>) {
        match decl {
            Decl::Function(func) => {
                let key = module_prefix.map(|p| format!("{}__{}", p, func.name));
                self.check_function(func, span, key.as_deref());
            }
            Decl::Class(class) => self.check_class(class, span),
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

    /// Check a function
    fn check_function(&mut self, func: &FunctionDecl, _span: Span, function_key: Option<&str>) {
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
            self.declare_variable(&param.name, ty, param.mutable, 0..0);
        }

        // Set current return type
        let return_type = self.resolve_type(&func.return_type);
        let mut inner_return_type = return_type.clone();
        if func.is_async {
            if let ResolvedType::Task(inner) = &return_type {
                inner_return_type = (**inner).clone();
            }
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
    }

    /// Check a class
    fn check_class(&mut self, class: &ClassDecl, _span: Span) {
        // Check constructor
        if let Some(ctor) = &class.constructor {
            self.enter_scope();
            let saved_effects = std::mem::take(&mut self.current_effects);
            let saved_pure = self.current_is_pure;
            let saved_any = self.current_allow_any;
            self.current_effects = self
                .infer_effects_in_block(&ctor.body, Some(&class.name))
                .into_iter()
                .collect();
            self.current_is_pure = false;
            self.current_allow_any = false;

            // Add 'this' binding
            self.declare_variable("this", ResolvedType::Class(class.name.clone()), true, 0..0);

            // Add parameters
            for param in &ctor.params {
                let ty = self.resolve_type(&param.ty);
                self.declare_variable(&param.name, ty, param.mutable, 0..0);
            }

            self.check_block(&ctor.body);
            self.current_effects = saved_effects;
            self.current_is_pure = saved_pure;
            self.current_allow_any = saved_any;
            self.exit_scope();
        }

        // Check destructor with inferred effects
        if let Some(dtor) = &class.destructor {
            self.enter_scope();
            let saved_effects = std::mem::take(&mut self.current_effects);
            let saved_pure = self.current_is_pure;
            let saved_any = self.current_allow_any;
            self.current_effects = self
                .infer_effects_in_block(&dtor.body, Some(&class.name))
                .into_iter()
                .collect();
            self.current_is_pure = false;
            self.current_allow_any = false;
            self.declare_variable("this", ResolvedType::Class(class.name.clone()), true, 0..0);
            self.check_block(&dtor.body);
            self.current_effects = saved_effects;
            self.current_is_pure = saved_pure;
            self.current_allow_any = saved_any;
            self.exit_scope();
        }

        // Check methods
        for method in &class.methods {
            self.enter_scope();
            let saved_effects = std::mem::take(&mut self.current_effects);
            let saved_pure = self.current_is_pure;
            let saved_any = self.current_allow_any;
            if let Some(class_info) = self.classes.get(&class.name) {
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
            self.declare_variable("this", ResolvedType::Class(class.name.clone()), false, 0..0);

            // Add parameters
            for param in &method.params {
                let ty = self.resolve_type(&param.ty);
                self.declare_variable(&param.name, ty, param.mutable, 0..0);
            }

            let return_type = self.resolve_type(&method.return_type);
            self.current_return_type = Some(return_type);

            self.check_block(&method.body);

            self.current_return_type = None;
            self.current_effects = saved_effects;
            self.current_is_pure = saved_pure;
            self.current_allow_any = saved_any;
            self.exit_scope();
        }
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
                if let Expr::Ident(name) = &target.node {
                    if let Some(var) = self.lookup_variable(name) {
                        if !var.mutable {
                            self.error_with_hint(
                                format!("Cannot assign to immutable variable '{}'", name),
                                target.span.clone(),
                                "Consider declaring with 'mut'".to_string(),
                            );
                        }
                    }
                }

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
            }

            Stmt::Break | Stmt::Continue => {}
        }
    }

    /// Check a pattern in match
    fn check_pattern(&mut self, pattern: &Pattern, expected_type: &ResolvedType, span: Span) {
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
                match expected_type {
                    ResolvedType::Option(inner) => {
                        if name == "Some" && bindings.len() == 1 {
                            self.declare_variable(&bindings[0], (**inner).clone(), false, span);
                        } else if name == "None" && bindings.is_empty() {
                            // OK
                        } else {
                            self.error(format!("Invalid Option pattern: {}", name), span);
                        }
                    }
                    ResolvedType::Result(ok, err) => {
                        if name == "Ok" && bindings.len() == 1 {
                            self.declare_variable(&bindings[0], (**ok).clone(), false, span);
                        } else if name == "Error" && bindings.len() == 1 {
                            self.declare_variable(&bindings[0], (**err).clone(), false, span);
                        } else {
                            self.error(format!("Invalid Result pattern: {}", name), span);
                        }
                    }
                    ResolvedType::Class(enum_name) => {
                        if let Some(enum_info) = self.enums.get(enum_name).cloned() {
                            if let Some(field_tys) = enum_info.variants.get(name) {
                                if field_tys.len() != bindings.len() {
                                    self.error(
                                        format!(
                                            "Pattern '{}' expects {} binding(s), got {}",
                                            name,
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

    /// Check an expression and return its type
    fn check_expr(&mut self, expr: &Expr, span: Span) -> ResolvedType {
        match expr {
            Expr::Literal(lit) => self.literal_type(lit),

            Expr::Ident(name) => {
                if let Some(var) = self.lookup_variable(name) {
                    var.ty.clone()
                } else if self.functions.contains_key(name) {
                    // Function reference
                    let sig = &self.functions[name];
                    ResolvedType::Function(
                        sig.params.iter().map(|(_, t)| t.clone()).collect(),
                        Box::new(sig.return_type.clone()),
                    )
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

            Expr::Call { callee, args } => self.check_call(&callee.node, args, span),

            Expr::Field { object, field } => {
                let obj_type = self.check_expr(&object.node, object.span.clone());
                self.check_field_access(&obj_type, field, span)
            }

            Expr::Index { object, index } => {
                let obj_type = self.check_expr(&object.node, object.span.clone());
                let idx_type = self.check_expr(&index.node, index.span.clone());

                if !matches!(idx_type, ResolvedType::Integer) {
                    self.error(
                        format!("Index must be Integer, found {}", idx_type),
                        index.span.clone(),
                    );
                }

                match &obj_type {
                    ResolvedType::List(inner) => (**inner).clone(),
                    ResolvedType::String => ResolvedType::Char,
                    ResolvedType::Map(_, v) => (**v).clone(),
                    _ => {
                        self.error(format!("Cannot index type {}", obj_type), span);
                        ResolvedType::Unknown
                    }
                }
            }

            Expr::Construct { ty, args } => {
                // Handle generic built-in types (e.g., List<Integer>, Set<String>)
                if ty.contains('<') && ty.ends_with('>') {
                    let resolved = self.parse_type_string(ty);
                    if !matches!(resolved, ResolvedType::Class(_))
                        && !matches!(resolved, ResolvedType::Unknown)
                    {
                        // TODO: Check arguments for specific generic constructors if needed
                        return resolved;
                    }
                }

                // Check if it's a class constructor
                if let Some(class) = self.classes.get(ty).cloned() {
                    if let Some(ctor_params) = &class.constructor {
                        if args.len() != ctor_params.len() {
                            self.error(
                                format!(
                                    "Constructor {} expects {} arguments, got {}",
                                    ty,
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
                    ResolvedType::Class(ty.clone())
                } else if ty == "List" || ty == "Map" || ty == "Option" || ty == "Result" {
                    // Non-parameterized version - needs inference
                    self.fresh_type_var()
                } else {
                    self.error(format!("Unknown type: {}", ty), span);
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

            Expr::Match { expr: _, arms: _ } => {
                // Match expressions need more complex analysis
                self.fresh_type_var()
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

                self.current_return_type = saved_return_type;
                self.exit_scope();
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
                    if let Stmt::Return(Some(expr)) = &stmt.node {
                        then_type = self.check_expr(&expr.node, expr.span.clone());
                    }
                    self.check_stmt(&stmt.node, stmt.span.clone());
                }
                self.exit_scope();

                if let Some(else_stmts) = else_branch {
                    self.enter_scope();
                    for stmt in else_stmts {
                        self.check_stmt(&stmt.node, stmt.span.clone());
                    }
                    self.exit_scope();
                }

                then_type
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

    /// Check a function/method call
    fn check_call(&mut self, callee: &Expr, args: &[Spanned<Expr>], span: Span) -> ResolvedType {
        // 1. Built-in functions (special handling for println, etc.)
        if let Expr::Ident(name) = callee {
            if let Some(required) = Self::builtin_required_effect(name) {
                self.enforce_required_effect(required, span.clone(), name);
            }
            if let Some(return_type) = self.check_builtin_call(name, args, span.clone()) {
                return return_type;
            }
        }

        // 2. Method call
        if let Expr::Field { object, field } = callee {
            // Special handling for static calls (e.g. File.read, Time.now)
            if let Expr::Ident(name) = &object.node {
                if matches!(
                    name.as_str(),
                    "File" | "Time" | "System" | "Math" | "Str" | "Args"
                ) {
                    let builtin_name = format!("{}__{}", name, field);
                    if let Some(required) = Self::builtin_required_effect(&builtin_name) {
                        self.enforce_required_effect(required, span.clone(), &builtin_name);
                    }
                    if let Some(ret) = self.check_builtin_call(&builtin_name, args, span.clone()) {
                        return ret;
                    }
                }

                // Enum variant constructor call: `Enum.Variant(...)`
                if let Some(enum_info) = self.enums.get(name).cloned() {
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
                        return ResolvedType::Class(name.clone());
                    }
                }

                // Module dot syntax: `Module.func(...)` -> `Module__func(...)`
                let mangled = format!("{}__{}", name, field);
                if let Some(sig) = self.functions.get(&mangled).cloned() {
                    self.enforce_call_effects(&sig, span.clone(), &mangled);
                    let expected = sig.params.len();
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
                        if sig.is_variadic && sig.is_extern {
                            self.check_variadic_ffi_tail_args(&mangled, args, expected);
                        }
                    }
                    return sig.return_type;
                }
            }

            let obj_type = self.check_expr(&object.node, object.span.clone());
            return self.check_method_call(&obj_type, field, args, span);
        }

        // 3. Evaluate callee to see if it's a function type (handles global functions and local variables/params)
        if let Expr::Ident(name) = callee {
            if let Some(sig) = self.functions.get(name).cloned() {
                self.enforce_call_effects(&sig, span.clone(), name);
                let expected = sig.params.len();
                let bad_arity = if sig.is_variadic {
                    args.len() < expected
                } else {
                    args.len() != expected
                };
                if bad_arity {
                    self.error(
                        format!(
                            "Function '{}' expects {} arguments, got {}",
                            name,
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
                    if sig.is_variadic && sig.is_extern {
                        self.check_variadic_ffi_tail_args(name, args, expected);
                    }
                }
                return sig.return_type;
            }
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
                        self.error(
                            format!(
                                "{}.{}() requires two String arguments",
                                name.split("__").next().unwrap(),
                                name.split("__").last().unwrap()
                            ),
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
                // range(start, end) -> Range<Integer> or range(start, end, step) -> Range<Integer>
                if args.len() < 2 || args.len() > 3 {
                    self.error("range() requires 2 or 3 arguments: range(start, end) or range(start, end, step)".to_string(), span.clone());
                }
                for arg in args {
                    let t = self.check_expr(&arg.node, arg.span.clone());
                    if !matches!(t, ResolvedType::Integer | ResolvedType::Float) {
                        self.error(
                            "range() arguments must be Integer or Float".to_string(),
                            span.clone(),
                        );
                    }
                }
                Some(ResolvedType::Range(Box::new(ResolvedType::Integer)))
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
        span: Span,
    ) -> ResolvedType {
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
                "insert" => {
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
                if let Some(class) = self.classes.get(name).cloned() {
                    if let Some(sig) = class.methods.get(method) {
                        self.enforce_call_effects(sig, span.clone(), method);
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
                            format!("Unknown method '{}' on class '{}'", method, name),
                            span,
                        );
                        ResolvedType::Unknown
                    }
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
                "result_type" => {
                    self.check_arg_count(method, args, 0, span);
                    (**inner).clone()
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
                if let Some(class) = self.classes.get(name) {
                    if let Some((field_type, _)) = class.fields.get(field) {
                        return field_type.clone();
                    }
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
                // Check for built-in types that might be parsed as Named
                match name.as_str() {
                    "Range" => ResolvedType::Class("Range".to_string()),
                    _ => ResolvedType::Class(name.clone()),
                }
            }
            Type::Option(inner) => ResolvedType::Option(Box::new(self.resolve_type(inner))),
            Type::Result(ok, err) => ResolvedType::Result(
                Box::new(self.resolve_type(ok)),
                Box::new(self.resolve_type(err)),
            ),
            Type::List(inner) => ResolvedType::List(Box::new(self.resolve_type(inner))),
            Type::Map(k, v) => ResolvedType::Map(
                Box::new(self.resolve_type(k)),
                Box::new(self.resolve_type(v)),
            ),
            Type::Set(inner) => ResolvedType::Set(Box::new(self.resolve_type(inner))),
            Type::Ref(inner) => ResolvedType::Ref(Box::new(self.resolve_type(inner))),
            Type::MutRef(inner) => ResolvedType::MutRef(Box::new(self.resolve_type(inner))),
            Type::Box(inner) => ResolvedType::Box(Box::new(self.resolve_type(inner))),
            Type::Rc(inner) => ResolvedType::Rc(Box::new(self.resolve_type(inner))),
            Type::Arc(inner) => ResolvedType::Arc(Box::new(self.resolve_type(inner))),
            Type::Ptr(inner) => ResolvedType::Ptr(Box::new(self.resolve_type(inner))),
            Type::Task(inner) => ResolvedType::Task(Box::new(self.resolve_type(inner))),
            Type::Range(inner) => ResolvedType::Range(Box::new(self.resolve_type(inner))),
            Type::Function(params, ret) => ResolvedType::Function(
                params.iter().map(|p| self.resolve_type(p)).collect(),
                Box::new(self.resolve_type(ret)),
            ),
            Type::Generic(name, args) => {
                // Handle generic types
                match name.as_str() {
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
                    _ => ResolvedType::Class(name.clone()),
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

        // Generic type compatibility
        match (expected, actual) {
            (ResolvedType::Ref(e), ResolvedType::Ref(a)) => self.types_compatible(e, a),
            (ResolvedType::MutRef(e), ResolvedType::MutRef(a)) => self.types_compatible(e, a),
            (ResolvedType::Ptr(e), ResolvedType::Ptr(a)) => self.types_compatible(e, a),
            // Can use &mut T where &T is expected
            (ResolvedType::Ref(e), ResolvedType::MutRef(a)) => self.types_compatible(e, a),
            // List compatibility
            (ResolvedType::List(e), ResolvedType::List(a)) => self.types_compatible(e, a),
            // Option compatibility
            (ResolvedType::Option(e), ResolvedType::Option(a)) => self.types_compatible(e, a),
            // Result compatibility
            (ResolvedType::Result(e_ok, e_err), ResolvedType::Result(a_ok, a_err)) => {
                self.types_compatible(e_ok, a_ok) && self.types_compatible(e_err, a_err)
            }
            // Map compatibility
            (ResolvedType::Map(ek, ev), ResolvedType::Map(ak, av)) => {
                self.types_compatible(ek, ak) && self.types_compatible(ev, av)
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

    /// Split generic arguments by comma, respecting nested < >
    fn split_generic_args(&self, s: &str) -> Vec<String> {
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
