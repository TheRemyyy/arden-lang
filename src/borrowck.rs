//! Apex Borrow Checker - Ownership and lifetime analysis
//!
//! This module provides:
//! - Move semantics checking
//! - Borrow lifetime tracking
//! - Mutable borrow exclusivity
//! - Use-after-move detection

#![allow(dead_code)]

use crate::ast::*;
use std::collections::HashMap;

/// Borrow checking error
#[derive(Debug, Clone)]
pub struct BorrowError {
    pub message: String,
    pub span: Span,
    pub note: Option<(String, Span)>,
}

impl BorrowError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            note: None,
        }
    }

    pub fn with_note(mut self, message: impl Into<String>, span: Span) -> Self {
        self.note = Some((message.into(), span));
        self
    }
}

/// Ownership state of a value
#[derive(Debug, Clone, PartialEq)]
enum OwnershipState {
    /// Value is owned and valid
    Owned,
    /// Value has been moved
    Moved(Span),
    /// Value is borrowed immutably (count of borrows)
    Borrowed(usize),
    /// Value is borrowed mutably
    MutBorrowed(Span),
}

/// Information about a borrow
#[derive(Debug, Clone)]
struct BorrowInfo {
    /// Variable being borrowed
    borrowed_from: String,
    /// Is it a mutable borrow?
    mutable: bool,
    /// Span where borrow was created
    span: Span,
    /// Scope depth where borrow is valid
    scope_depth: usize,
}

/// Variable tracking for borrow checker
#[derive(Debug, Clone)]
struct VarState {
    /// Current ownership state
    state: OwnershipState,
    /// Is this variable mutable?
    mutable: bool,
    /// Where was this variable declared?
    declared_at: Span,
    /// Type of the variable (for drop checking)
    needs_drop: bool,
    /// Optional declared type (for method resolution in borrow checking)
    ty: Option<Type>,
}

/// Borrow checker state
pub struct BorrowChecker {
    /// Variable states by scope
    scopes: Vec<HashMap<String, VarState>>,
    /// Active borrows
    borrows: Vec<BorrowInfo>,
    /// Global function signatures
    functions: HashMap<String, Vec<ParamMode>>,
    /// Class method signatures
    classes: HashMap<String, ClassBorrowSigs>,
    /// Current scope depth
    scope_depth: usize,
    /// Collected errors
    errors: Vec<BorrowError>,
    /// Variables that need dropping at end of current scope
    drop_queue: Vec<Vec<String>>,
}

struct ClassBorrowSigs {
    methods: HashMap<String, Vec<ParamMode>>,
    constructor: Vec<ParamMode>,
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            borrows: Vec::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            scope_depth: 0,
            errors: Vec::new(),
            drop_queue: vec![Vec::new()],
        }
    }

    /// Run borrow checking on a program
    pub fn check(&mut self, program: &Program) -> Result<(), Vec<BorrowError>> {
        // First pass: collect signatures
        for decl in &program.declarations {
            self.collect_sig(&decl.node);
        }

        // Second pass: check function bodies
        for decl in &program.declarations {
            self.check_decl(&decl.node, decl.span.clone());
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    fn collect_sig(&mut self, decl: &Decl) {
        match decl {
            Decl::Function(func) => {
                self.functions.insert(
                    func.name.clone(),
                    func.params.iter().map(|p| p.mode).collect(),
                );
            }
            Decl::Class(class) => {
                let mut methods = HashMap::new();
                for method in &class.methods {
                    methods.insert(
                        method.name.clone(),
                        method.params.iter().map(|p| p.mode).collect(),
                    );
                }
                let constructor = class
                    .constructor
                    .as_ref()
                    .map(|c| c.params.iter().map(|p| p.mode).collect())
                    .unwrap_or_default();

                self.classes.insert(
                    class.name.clone(),
                    ClassBorrowSigs {
                        methods,
                        constructor,
                    },
                );
            }
            Decl::Module(module) => {
                for inner in &module.declarations {
                    match &inner.node {
                        Decl::Function(func) => {
                            self.functions.insert(
                                format!("{}__{}", module.name, func.name),
                                func.params.iter().map(|p| p.mode).collect(),
                            );
                            // Keep unprefixed for backward compatibility.
                            self.functions.insert(
                                func.name.clone(),
                                func.params.iter().map(|p| p.mode).collect(),
                            );
                        }
                        _ => self.collect_sig(&inner.node),
                    }
                }
            }
            _ => {}
        }
    }

    fn check_decl(&mut self, decl: &Decl, _span: Span) {
        match decl {
            Decl::Function(func) => self.check_function(func),
            Decl::Class(class) => self.check_class(class),
            Decl::Module(module) => {
                for inner in &module.declarations {
                    self.check_decl(&inner.node, inner.span.clone());
                }
            }
            _ => {}
        }
    }

    fn check_function(&mut self, func: &FunctionDecl) {
        self.enter_scope();

        // Add parameters with correct initial state
        for param in &func.params {
            self.declare_var(
                &param.name,
                param.mutable,
                0..0,
                self.needs_drop(&param.ty),
                Some(param.ty.clone()),
            );

            // If it's a borrow parameter, initialize it as borrowed
            match param.mode {
                ParamMode::Borrow => {
                    if let Some(var) = self.get_var_mut(&param.name) {
                        var.state = OwnershipState::Borrowed(1);
                    }
                }
                ParamMode::BorrowMut => {
                    if let Some(var) = self.get_var_mut(&param.name) {
                        var.state = OwnershipState::MutBorrowed(0..0);
                    }
                }
                ParamMode::Owned => {}
            }
        }

        self.check_block(&func.body);
        self.exit_scope();
    }

    fn check_class(&mut self, class: &ClassDecl) {
        // Check constructor
        if let Some(ctor) = &class.constructor {
            self.enter_scope();
            self.declare_var(
                "this",
                true,
                0..0,
                false,
                Some(Type::Named(class.name.clone())),
            );
            for param in &ctor.params {
                self.declare_var(
                    &param.name,
                    param.mutable,
                    0..0,
                    self.needs_drop(&param.ty),
                    Some(param.ty.clone()),
                );
            }
            self.check_block(&ctor.body);
            self.exit_scope();
        }

        // Check methods
        for method in &class.methods {
            self.enter_scope();
            self.declare_var(
                "this",
                false,
                0..0,
                false,
                Some(Type::Named(class.name.clone())),
            );
            for param in &method.params {
                self.declare_var(
                    &param.name,
                    param.mutable,
                    0..0,
                    self.needs_drop(&param.ty),
                    Some(param.ty.clone()),
                );

                // Initialize borrow state for parameters
                match param.mode {
                    ParamMode::Borrow => {
                        if let Some(var) = self.get_var_mut(&param.name) {
                            var.state = OwnershipState::Borrowed(1);
                        }
                    }
                    ParamMode::BorrowMut => {
                        if let Some(var) = self.get_var_mut(&param.name) {
                            var.state = OwnershipState::MutBorrowed(0..0);
                        }
                    }
                    ParamMode::Owned => {}
                }
            }
            self.check_block(&method.body);
            self.exit_scope();
        }
    }

    fn check_block(&mut self, block: &Block) {
        self.enter_scope();
        for stmt in block {
            self.check_stmt(&stmt.node, stmt.span.clone());
        }
        self.exit_scope();
    }

    fn check_stmt(&mut self, stmt: &Stmt, span: Span) {
        match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
            } => {
                // Check the value expression (may involve moves)
                self.check_expr(&value.node, value.span.clone(), false);

                // Declare the new variable
                self.declare_var(name, *mutable, span, self.needs_drop(ty), Some(ty.clone()));
            }

            Stmt::Assign { target, value } => {
                // Check value first
                self.check_expr(&value.node, value.span.clone(), false);

                // Check target is valid for assignment
                self.check_assign_target(&target.node, target.span.clone());
            }

            Stmt::Expr(expr) => {
                self.check_expr(&expr.node, expr.span.clone(), false);
            }

            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    self.check_expr(&e.node, e.span.clone(), false);
                }
            }

            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.check_expr(&condition.node, condition.span.clone(), false);
                self.check_block(then_block);
                if let Some(else_blk) = else_block {
                    self.check_block(else_blk);
                }
            }

            Stmt::While { condition, body } => {
                self.check_expr(&condition.node, condition.span.clone(), false);
                self.check_block(body);
            }

            Stmt::For {
                var,
                var_type,
                iterable,
                body,
            } => {
                self.check_expr(&iterable.node, iterable.span.clone(), false);
                self.enter_scope();
                let needs_drop = var_type
                    .as_ref()
                    .map(|t| self.needs_drop(t))
                    .unwrap_or(false);
                self.declare_var(var, false, span, needs_drop, var_type.clone());
                for stmt in body {
                    self.check_stmt(&stmt.node, stmt.span.clone());
                }
                self.exit_scope();
            }

            Stmt::Match { expr, arms } => {
                self.check_expr(&expr.node, expr.span.clone(), false);
                for arm in arms {
                    self.enter_scope();
                    self.bind_pattern(&arm.pattern, span.clone());
                    for stmt in &arm.body {
                        self.check_stmt(&stmt.node, stmt.span.clone());
                    }
                    self.exit_scope();
                }
            }

            Stmt::Break | Stmt::Continue => {}
        }
    }

    fn check_assign_target(&mut self, target: &Expr, span: Span) {
        match target {
            Expr::Ident(name) => {
                // Check mutability and borrow state
                let (mutable, state) = {
                    if let Some(var) = self.get_var(name) {
                        (var.mutable, var.state.clone())
                    } else {
                        self.errors.push(BorrowError::new(
                            format!("Cannot assign to undeclared variable '{}'", name),
                            span.clone(),
                        ));
                        return;
                    }
                };

                if !mutable {
                    self.errors.push(BorrowError::new(
                        format!("Cannot assign to immutable variable '{}'", name),
                        span.clone(),
                    ));
                }

                match state {
                    OwnershipState::MutBorrowed(borrow_span) => {
                        self.errors.push(
                            BorrowError::new(
                                format!("Cannot assign to '{}' while mutably borrowed", name),
                                span.clone(),
                            )
                            .with_note("Mutable borrow occurred here", borrow_span),
                        );
                    }
                    OwnershipState::Borrowed(count) if count > 0 => {
                        self.errors.push(BorrowError::new(
                            format!("Cannot assign to '{}' while borrowed", name),
                            span.clone(),
                        ));
                    }
                    _ => {}
                }

                // Reset ownership state (old value dropped)
                if let Some(var) = self.get_var_mut(name) {
                    var.state = OwnershipState::Owned;
                }
            }
            Expr::Field { object, field: _ } => {
                self.check_expr(&object.node, object.span.clone(), false);
            }
            Expr::Index { object, index } => {
                self.check_expr(&object.node, object.span.clone(), false);
                self.check_expr(&index.node, index.span.clone(), false);
            }
            Expr::Deref(inner) => {
                // Check that we're dereferencing a mutable reference
                self.check_expr(&inner.node, inner.span.clone(), true);
            }
            _ => {
                self.errors.push(BorrowError::new(
                    "Invalid assignment target".to_string(),
                    span,
                ));
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn check_expr(&mut self, expr: &Expr, span: Span, need_mut: bool) {
        match expr {
            Expr::Ident(name) => {
                // Using a variable - check if it's valid
                let state = self.get_var(name).map(|v| v.state.clone());
                if let Some(OwnershipState::Moved(move_span)) = state {
                    self.errors.push(
                        BorrowError::new(format!("Use of moved value '{}'", name), span.clone())
                            .with_note("Value moved here", move_span),
                    );
                }
            }

            Expr::Binary { left, right, .. } => {
                self.check_expr(&left.node, left.span.clone(), false);
                self.check_expr(&right.node, right.span.clone(), false);
            }

            Expr::Unary { expr: inner, .. } => {
                self.check_expr(&inner.node, inner.span.clone(), false);
            }

            Expr::Call { callee, args } => {
                self.check_expr(&callee.node, callee.span.clone(), false);

                let param_modes = self.resolve_call_param_modes(&callee.node, args.len());

                for (i, arg) in args.iter().enumerate() {
                    self.check_expr(&arg.node, arg.span.clone(), false);

                    let mode = param_modes.get(i).unwrap_or(&ParamMode::Owned);
                    match mode {
                        ParamMode::Owned => {
                            self.try_move(&arg.node, arg.span.clone());
                        }
                        ParamMode::Borrow => {
                            if let Expr::Ident(name) = &arg.node {
                                self.create_borrow(name, false, arg.span.clone());
                            }
                        }
                        ParamMode::BorrowMut => {
                            if let Expr::Ident(name) = &arg.node {
                                self.create_borrow(name, true, arg.span.clone());
                            }
                        }
                    }
                }
            }

            Expr::Field { object, field: _ } => {
                self.check_expr(&object.node, object.span.clone(), need_mut);
            }

            Expr::Index { object, index } => {
                self.check_expr(&object.node, object.span.clone(), need_mut);
                self.check_expr(&index.node, index.span.clone(), false);
            }

            Expr::Construct { ty, args } => {
                // Get constructor param modes
                let param_modes = self
                    .classes
                    .get(ty)
                    .map(|c| c.constructor.clone())
                    .unwrap_or_default();

                for (i, arg) in args.iter().enumerate() {
                    self.check_expr(&arg.node, arg.span.clone(), false);

                    let mode = param_modes.get(i).unwrap_or(&ParamMode::Owned);
                    match mode {
                        ParamMode::Owned => {
                            self.try_move(&arg.node, arg.span.clone());
                        }
                        ParamMode::Borrow => {
                            if let Expr::Ident(name) = &arg.node {
                                self.create_borrow(name, false, arg.span.clone());
                            }
                        }
                        ParamMode::BorrowMut => {
                            if let Expr::Ident(name) = &arg.node {
                                self.create_borrow(name, true, arg.span.clone());
                            }
                        }
                    }
                }
            }

            Expr::Lambda { params, body } => {
                // Lambda captures - free vars borrow or move from outer scope
                self.enter_scope();
                for param in params {
                    self.declare_var(
                        &param.name,
                        param.mutable,
                        span.clone(),
                        false,
                        Some(param.ty.clone()),
                    );
                }

                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                let free_idents = Self::collect_free_idents(&body.node, &param_names);
                for ident in free_idents {
                    if self.get_var(&ident).is_none() {
                        continue;
                    }
                    if self.expr_moves_ident(&body.node, &ident) {
                        self.try_move(&Expr::Ident(ident.clone()), span.clone());
                    } else {
                        self.create_borrow(&ident, false, span.clone());
                    }
                }

                self.check_expr(&body.node, body.span.clone(), false);
                self.exit_scope();
            }

            Expr::Borrow(inner) => {
                // Create immutable borrow
                if let Expr::Ident(name) = &inner.node {
                    self.create_borrow(name, false, span.clone());
                } else {
                    self.check_expr(&inner.node, inner.span.clone(), false);
                }
            }

            Expr::MutBorrow(inner) => {
                // Create mutable borrow
                if let Expr::Ident(name) = &inner.node {
                    self.create_borrow(name, true, span.clone());
                } else {
                    self.check_expr(&inner.node, inner.span.clone(), true);
                }
            }

            Expr::Deref(inner) => {
                self.check_expr(&inner.node, inner.span.clone(), need_mut);
            }

            Expr::Try(inner) => {
                self.check_expr(&inner.node, inner.span.clone(), false);
            }

            Expr::StringInterp(parts) => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        self.check_expr(&e.node, e.span.clone(), false);
                    }
                }
            }

            Expr::Match { expr: inner, arms } => {
                self.check_expr(&inner.node, inner.span.clone(), false);
                for arm in arms {
                    self.enter_scope();
                    self.bind_pattern(&arm.pattern, span.clone());
                    for stmt in &arm.body {
                        self.check_stmt(&stmt.node, stmt.span.clone());
                    }
                    self.exit_scope();
                }
            }

            Expr::Await(inner) => {
                self.check_expr(&inner.node, inner.span.clone(), false);
            }

            Expr::AsyncBlock(body) => {
                self.enter_scope();
                for stmt in body {
                    self.check_stmt(&stmt.node, stmt.span.clone());
                }
                self.exit_scope();
            }

            Expr::Require { condition, message } => {
                self.check_expr(&condition.node, condition.span.clone(), false);
                if let Some(msg) = message {
                    self.check_expr(&msg.node, msg.span.clone(), false);
                }
            }

            Expr::Range {
                start,
                end,
                inclusive: _,
            } => {
                if let Some(s) = start {
                    self.check_expr(&s.node, s.span.clone(), false);
                }
                if let Some(e) = end {
                    self.check_expr(&e.node, e.span.clone(), false);
                }
            }

            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                self.check_expr(&condition.node, condition.span.clone(), false);
                self.enter_scope();
                for stmt in then_branch {
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
            }

            Expr::Block(body) => {
                self.enter_scope();
                for stmt in body {
                    self.check_stmt(&stmt.node, stmt.span.clone());
                }
                self.exit_scope();
            }

            Expr::Literal(_) | Expr::This => {}
        }
    }

    /// Try to move a value
    fn try_move(&mut self, expr: &Expr, span: Span) {
        if let Expr::Ident(name) = expr {
            // Get info about the variable
            let (needs_drop, state) = {
                if let Some(var) = self.get_var(name) {
                    (var.needs_drop, var.state.clone())
                } else {
                    return;
                }
            };

            // Don't move Copy types
            if !needs_drop {
                return;
            }

            // Check for active borrows
            match state {
                OwnershipState::MutBorrowed(borrow_span) => {
                    self.errors.push(
                        BorrowError::new(
                            format!("Cannot move '{}' while mutably borrowed", name),
                            span.clone(),
                        )
                        .with_note("Mutable borrow occurred here", borrow_span),
                    );
                    return;
                }
                OwnershipState::Borrowed(count) if count > 0 => {
                    self.errors.push(BorrowError::new(
                        format!("Cannot move '{}' while borrowed", name),
                        span.clone(),
                    ));
                    return;
                }
                _ => {}
            }

            // Mark as moved
            if let Some(var) = self.get_var_mut(name) {
                var.state = OwnershipState::Moved(span);
            }
        }
    }

    /// Create a borrow
    fn create_borrow(&mut self, name: &str, mutable: bool, span: Span) {
        // Get current state
        let (var_mutable, state) = {
            if let Some(var) = self.get_var(name) {
                (var.mutable, var.state.clone())
            } else {
                return;
            }
        };

        // Check current state
        match state {
            OwnershipState::Moved(move_span) => {
                self.errors.push(
                    BorrowError::new(format!("Cannot borrow '{}' after move", name), span.clone())
                        .with_note("Value was moved here", move_span),
                );
                return;
            }
            OwnershipState::MutBorrowed(borrow_span) => {
                self.errors.push(
                    BorrowError::new(
                        format!("Cannot borrow '{}' while mutably borrowed", name),
                        span.clone(),
                    )
                    .with_note("Mutable borrow occurred here", borrow_span),
                );
                return;
            }
            OwnershipState::Borrowed(count) if mutable && count > 0 => {
                self.errors.push(BorrowError::new(
                    format!("Cannot mutably borrow '{}' while immutably borrowed", name),
                    span.clone(),
                ));
                return;
            }
            _ => {}
        }

        // Check mutability for mut borrow
        if mutable && !var_mutable {
            self.errors.push(BorrowError::new(
                format!("Cannot mutably borrow immutable variable '{}'", name),
                span.clone(),
            ));
            return;
        }

        // Update state
        if let Some(var) = self.get_var_mut(name) {
            if mutable {
                var.state = OwnershipState::MutBorrowed(span.clone());
            } else {
                match &mut var.state {
                    OwnershipState::Borrowed(count) => *count += 1,
                    OwnershipState::Owned => var.state = OwnershipState::Borrowed(1),
                    _ => {}
                }
            }
        }

        self.borrows.push(BorrowInfo {
            borrowed_from: name.to_string(),
            mutable,
            span,
            scope_depth: self.scope_depth,
        });
    }

    fn bind_pattern(&mut self, pattern: &Pattern, span: Span) {
        match pattern {
            Pattern::Ident(name) => {
                self.declare_var(name, false, span, false, None);
            }
            Pattern::Variant(_, bindings) => {
                for binding in bindings {
                    self.declare_var(binding, false, span.clone(), false, None);
                }
            }
            _ => {}
        }
    }

    fn is_borrowing_stdlib_call(name: &str) -> bool {
        matches!(
            name,
            "Str__len"
                | "Str__compare"
                | "Str__concat"
                | "Str__upper"
                | "Str__lower"
                | "Str__trim"
                | "Str__contains"
                | "print"
                | "println"
                | "File__exists"
                | "File__read"
                | "File__write"
                | "File__delete"
                | "Time__now"
                | "Time__sleep"
                | "System__getenv"
                | "System__shell"
                | "System__exec"
                | "System__exit"
                | "Math__abs"
                | "Math__min"
                | "Math__max"
                | "Math__sqrt"
                | "Math__pow"
                | "Math__sin"
                | "Math__cos"
                | "Math__tan"
                | "Math__floor"
                | "Math__ceil"
                | "Math__round"
                | "Math__log"
                | "Math__log10"
                | "Math__exp"
        )
    }

    fn infer_expr_class<'a>(&'a self, expr: &Expr) -> Option<&'a str> {
        match expr {
            Expr::Ident(name) => {
                let ty = self.get_var(name)?.ty.as_ref()?;
                match ty {
                    Type::Named(class_name) => Some(class_name.as_str()),
                    Type::Generic(class_name, _) => Some(class_name.as_str()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn resolve_call_param_modes(&self, callee: &Expr, arg_len: usize) -> Vec<ParamMode> {
        let mut param_modes = Vec::new();

        if let Expr::Ident(name) = callee {
            if Self::is_borrowing_stdlib_call(name) {
                return vec![ParamMode::Borrow; arg_len];
            }
            if let Some(modes) = self.functions.get(name) {
                return modes.clone();
            }
            return param_modes;
        }

        if let Expr::Field { object, field } = callee {
            if let Expr::Ident(name) = &object.node {
                if matches!(
                    name.as_str(),
                    "File" | "Time" | "System" | "Math" | "Str" | "Args"
                ) {
                    param_modes = vec![ParamMode::Borrow; arg_len];
                }

                let mangled = format!("{}__{}", name, field);
                if let Some(modes) = self.functions.get(&mangled) {
                    return modes.clone();
                }
            }

            if let Some(class_name) = self.infer_expr_class(&object.node) {
                if let Some(class_sig) = self.classes.get(class_name) {
                    if let Some(modes) = class_sig.methods.get(field) {
                        return modes.clone();
                    }
                }
            }
        }

        param_modes
    }

    fn collect_free_idents(expr: &Expr, params: &[String]) -> Vec<String> {
        let mut out = Vec::new();
        Self::collect_free_idents_inner(expr, params, &mut out);
        out.sort();
        out.dedup();
        out
    }

    fn collect_free_idents_inner(expr: &Expr, params: &[String], out: &mut Vec<String>) {
        match expr {
            Expr::Ident(name) => {
                if !params.iter().any(|p| p == name) {
                    out.push(name.clone());
                }
            }
            Expr::Call { callee, args } => {
                Self::collect_free_idents_inner(&callee.node, params, out);
                for arg in args {
                    Self::collect_free_idents_inner(&arg.node, params, out);
                }
            }
            Expr::Binary { left, right, .. } => {
                Self::collect_free_idents_inner(&left.node, params, out);
                Self::collect_free_idents_inner(&right.node, params, out);
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => Self::collect_free_idents_inner(&expr.node, params, out),
            Expr::Field { object, .. } => {
                Self::collect_free_idents_inner(&object.node, params, out)
            }
            Expr::Index { object, index } => {
                Self::collect_free_idents_inner(&object.node, params, out);
                Self::collect_free_idents_inner(&index.node, params, out);
            }
            Expr::Construct { args, .. } => {
                for arg in args {
                    Self::collect_free_idents_inner(&arg.node, params, out);
                }
            }
            Expr::Lambda {
                params: inner,
                body,
            } => {
                let mut nested_params: Vec<String> = params.to_vec();
                nested_params.extend(inner.iter().map(|p| p.name.clone()));
                Self::collect_free_idents_inner(&body.node, &nested_params, out);
            }
            Expr::Match { expr, arms } => {
                Self::collect_free_idents_inner(&expr.node, params, out);
                for arm in arms {
                    for stmt in &arm.body {
                        Self::collect_free_idents_stmt(&stmt.node, params, out);
                    }
                }
            }
            Expr::StringInterp(parts) => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        Self::collect_free_idents_inner(&e.node, params, out);
                    }
                }
            }
            Expr::AsyncBlock(stmts) | Expr::Block(stmts) => {
                for stmt in stmts {
                    Self::collect_free_idents_stmt(&stmt.node, params, out);
                }
            }
            Expr::Require { condition, message } => {
                Self::collect_free_idents_inner(&condition.node, params, out);
                if let Some(msg) = message {
                    Self::collect_free_idents_inner(&msg.node, params, out);
                }
            }
            Expr::Range { start, end, .. } => {
                if let Some(start) = start {
                    Self::collect_free_idents_inner(&start.node, params, out);
                }
                if let Some(end) = end {
                    Self::collect_free_idents_inner(&end.node, params, out);
                }
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::collect_free_idents_inner(&condition.node, params, out);
                for stmt in then_branch {
                    Self::collect_free_idents_stmt(&stmt.node, params, out);
                }
                if let Some(else_branch) = else_branch {
                    for stmt in else_branch {
                        Self::collect_free_idents_stmt(&stmt.node, params, out);
                    }
                }
            }
            Expr::Literal(_) | Expr::This => {}
        }
    }

    fn collect_free_idents_stmt(stmt: &Stmt, params: &[String], out: &mut Vec<String>) {
        match stmt {
            Stmt::Let { value, .. } => Self::collect_free_idents_inner(&value.node, params, out),
            Stmt::Assign { target, value } => {
                Self::collect_free_idents_inner(&target.node, params, out);
                Self::collect_free_idents_inner(&value.node, params, out);
            }
            Stmt::Expr(expr) => Self::collect_free_idents_inner(&expr.node, params, out),
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    Self::collect_free_idents_inner(&expr.node, params, out);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                Self::collect_free_idents_inner(&condition.node, params, out);
                for stmt in then_block {
                    Self::collect_free_idents_stmt(&stmt.node, params, out);
                }
                if let Some(else_block) = else_block {
                    for stmt in else_block {
                        Self::collect_free_idents_stmt(&stmt.node, params, out);
                    }
                }
            }
            Stmt::While { condition, body } => {
                Self::collect_free_idents_inner(&condition.node, params, out);
                for stmt in body {
                    Self::collect_free_idents_stmt(&stmt.node, params, out);
                }
            }
            Stmt::For { iterable, body, .. } => {
                Self::collect_free_idents_inner(&iterable.node, params, out);
                for stmt in body {
                    Self::collect_free_idents_stmt(&stmt.node, params, out);
                }
            }
            Stmt::Match { expr, arms } => {
                Self::collect_free_idents_inner(&expr.node, params, out);
                for arm in arms {
                    for stmt in &arm.body {
                        Self::collect_free_idents_stmt(&stmt.node, params, out);
                    }
                }
            }
            Stmt::Break | Stmt::Continue => {}
        }
    }

    fn expr_moves_ident(&self, expr: &Expr, ident: &str) -> bool {
        match expr {
            Expr::Call { callee, args } => {
                let param_modes = self.resolve_call_param_modes(&callee.node, args.len());
                for (i, arg) in args.iter().enumerate() {
                    if let Expr::Ident(name) = &arg.node {
                        let mode = param_modes.get(i).unwrap_or(&ParamMode::Owned);
                        if name == ident && *mode == ParamMode::Owned {
                            return true;
                        }
                    }
                    if self.expr_moves_ident(&arg.node, ident) {
                        return true;
                    }
                }
                self.expr_moves_ident(&callee.node, ident)
            }
            Expr::Binary { left, right, .. } => {
                self.expr_moves_ident(&left.node, ident)
                    || self.expr_moves_ident(&right.node, ident)
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => self.expr_moves_ident(&expr.node, ident),
            Expr::Field { object, .. } => self.expr_moves_ident(&object.node, ident),
            Expr::Index { object, index } => {
                self.expr_moves_ident(&object.node, ident)
                    || self.expr_moves_ident(&index.node, ident)
            }
            Expr::Construct { args, .. } => args
                .iter()
                .any(|arg| self.expr_moves_ident(&arg.node, ident)),
            Expr::Lambda { body, .. } => self.expr_moves_ident(&body.node, ident),
            Expr::Match { expr, arms } => {
                self.expr_moves_ident(&expr.node, ident)
                    || arms.iter().any(|arm| {
                        arm.body
                            .iter()
                            .any(|stmt| self.stmt_moves_ident(&stmt.node, ident))
                    })
            }
            Expr::StringInterp(parts) => parts.iter().any(|part| match part {
                StringPart::Expr(e) => self.expr_moves_ident(&e.node, ident),
                StringPart::Literal(_) => false,
            }),
            Expr::AsyncBlock(stmts) | Expr::Block(stmts) => stmts
                .iter()
                .any(|stmt| self.stmt_moves_ident(&stmt.node, ident)),
            Expr::Require { condition, message } => {
                self.expr_moves_ident(&condition.node, ident)
                    || message
                        .as_ref()
                        .map(|m| self.expr_moves_ident(&m.node, ident))
                        .unwrap_or(false)
            }
            Expr::Range { start, end, .. } => {
                start
                    .as_ref()
                    .map(|s| self.expr_moves_ident(&s.node, ident))
                    .unwrap_or(false)
                    || end
                        .as_ref()
                        .map(|e| self.expr_moves_ident(&e.node, ident))
                        .unwrap_or(false)
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expr_moves_ident(&condition.node, ident)
                    || then_branch
                        .iter()
                        .any(|stmt| self.stmt_moves_ident(&stmt.node, ident))
                    || else_branch
                        .as_ref()
                        .map(|stmts| {
                            stmts
                                .iter()
                                .any(|stmt| self.stmt_moves_ident(&stmt.node, ident))
                        })
                        .unwrap_or(false)
            }
            Expr::Ident(_) | Expr::Literal(_) | Expr::This => false,
        }
    }

    fn stmt_moves_ident(&self, stmt: &Stmt, ident: &str) -> bool {
        match stmt {
            Stmt::Let { value, .. } => self.expr_moves_ident(&value.node, ident),
            Stmt::Assign { target, value } => {
                self.expr_moves_ident(&target.node, ident)
                    || self.expr_moves_ident(&value.node, ident)
            }
            Stmt::Expr(expr) => self.expr_moves_ident(&expr.node, ident),
            Stmt::Return(expr) => expr
                .as_ref()
                .map(|e| self.expr_moves_ident(&e.node, ident))
                .unwrap_or(false),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.expr_moves_ident(&condition.node, ident)
                    || then_block
                        .iter()
                        .any(|stmt| self.stmt_moves_ident(&stmt.node, ident))
                    || else_block
                        .as_ref()
                        .map(|stmts| {
                            stmts
                                .iter()
                                .any(|stmt| self.stmt_moves_ident(&stmt.node, ident))
                        })
                        .unwrap_or(false)
            }
            Stmt::While { condition, body } => {
                self.expr_moves_ident(&condition.node, ident)
                    || body
                        .iter()
                        .any(|stmt| self.stmt_moves_ident(&stmt.node, ident))
            }
            Stmt::For { iterable, body, .. } => {
                self.expr_moves_ident(&iterable.node, ident)
                    || body
                        .iter()
                        .any(|stmt| self.stmt_moves_ident(&stmt.node, ident))
            }
            Stmt::Match { expr, arms } => {
                self.expr_moves_ident(&expr.node, ident)
                    || arms.iter().any(|arm| {
                        arm.body
                            .iter()
                            .any(|stmt| self.stmt_moves_ident(&stmt.node, ident))
                    })
            }
            Stmt::Break | Stmt::Continue => false,
        }
    }

    fn recount_borrows_for(&self, name: &str) -> (usize, Option<Span>) {
        let mut immutable_count = 0usize;
        let mut mutable_span = None;
        for borrow in &self.borrows {
            if borrow.borrowed_from == name {
                if borrow.mutable {
                    mutable_span = Some(borrow.span.clone());
                } else {
                    immutable_count += 1;
                }
            }
        }
        (immutable_count, mutable_span)
    }

    fn declare_var(
        &mut self,
        name: &str,
        mutable: bool,
        span: Span,
        needs_drop: bool,
        ty: Option<Type>,
    ) {
        let var = VarState {
            state: OwnershipState::Owned,
            mutable,
            declared_at: span,
            needs_drop,
            ty,
        };
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.to_string(), var);

        if needs_drop {
            self.drop_queue.last_mut().unwrap().push(name.to_string());
        }
    }

    fn get_var(&self, name: &str) -> Option<&VarState> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Some(var);
            }
        }
        None
    }

    fn get_var_mut(&mut self, name: &str) -> Option<&mut VarState> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.get_mut(name) {
                return Some(var);
            }
        }
        None
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.drop_queue.push(Vec::new());
        self.scope_depth += 1;
    }

    fn exit_scope(&mut self) {
        // Release borrows from this scope
        self.borrows.retain(|b| b.scope_depth < self.scope_depth);

        // Recompute ownership state based on still-active borrows per variable.
        let mut updates: Vec<(usize, String, OwnershipState)> = Vec::new();
        for (scope_idx, scope) in self.scopes.iter().enumerate() {
            for (name, var) in scope {
                let (immut_count, mut_span) = self.recount_borrows_for(name);
                if let Some(span) = mut_span {
                    updates.push((scope_idx, name.clone(), OwnershipState::MutBorrowed(span)));
                } else if immut_count > 0 {
                    updates.push((
                        scope_idx,
                        name.clone(),
                        OwnershipState::Borrowed(immut_count),
                    ));
                } else if matches!(
                    var.state,
                    OwnershipState::Borrowed(_) | OwnershipState::MutBorrowed(_)
                ) {
                    updates.push((scope_idx, name.clone(), OwnershipState::Owned));
                }
            }
        }
        for (scope_idx, name, new_state) in updates {
            if let Some(scope) = self.scopes.get_mut(scope_idx) {
                if let Some(var) = scope.get_mut(&name) {
                    var.state = new_state;
                }
            }
        }

        self.scopes.pop();
        self.drop_queue.pop();
        self.scope_depth -= 1;
    }

    /// Check if a type needs drop (not Copy)
    fn needs_drop(&self, ty: &Type) -> bool {
        match ty {
            Type::Integer | Type::Float | Type::Boolean | Type::Char | Type::None => false,
            Type::Ref(_) | Type::MutRef(_) => false, // References don't own data
            _ => true,                               // Strings, classes, collections need drop
        }
    }
}

/// Format borrow errors with source context
pub fn format_borrow_errors(errors: &[BorrowError], source: &str, filename: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut output = String::new();

    for error in errors {
        let (line_num, col) = span_to_location(&error.span, source);

        output.push_str(&format!(
            "\x1b[1;31merror[E0505]\x1b[0m: {}\n",
            error.message
        ));
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

            let underline_start = col.saturating_sub(1);
            let underline_len = (error.span.end - error.span.start).max(1);
            output.push_str(&format!(
                "   \x1b[1;34m|\x1b[0m {}\x1b[1;31m{}\x1b[0m\n",
                " ".repeat(underline_start),
                "^".repeat(underline_len.min(50))
            ));
        }

        if let Some((note_msg, note_span)) = &error.note {
            let (note_line, _) = span_to_location(note_span, source);
            output.push_str("   \x1b[1;34m|\x1b[0m\n");
            output.push_str(&format!(
                "   \x1b[1;34m= note\x1b[0m: {} (at line {})\n",
                note_msg, note_line
            ));
        }

        output.push('\n');
    }

    output
}

fn span_to_location(span: &Span, source: &str) -> (usize, usize) {
    let mut line_num: usize = 1;
    let mut col: usize = 1;

    for (i, ch) in source.char_indices() {
        if i >= span.start {
            break;
        }
        if ch == '\n' {
            line_num += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line_num, col)
}

#[cfg(test)]
mod tests {
    use super::BorrowChecker;
    use crate::parser::Parser;
    use crate::{ast::Program, lexer};

    fn parse_program(source: &str) -> Program {
        let tokens = lexer::tokenize(source).expect("tokenization should succeed");
        let mut parser = Parser::new(tokens);
        parser.parse_program().expect("parse should succeed")
    }

    fn borrow_errors(source: &str) -> Vec<String> {
        let program = parse_program(source);
        let mut checker = BorrowChecker::new();
        checker
            .check(&program)
            .expect_err("borrow check should fail")
            .into_iter()
            .map(|e| e.message)
            .collect()
    }

    fn borrow_ok(source: &str) {
        let program = parse_program(source);
        let mut checker = BorrowChecker::new();
        checker.check(&program).expect("borrow check should pass");
    }

    #[test]
    fn detects_use_after_move() {
        let source = r#"
            import std.io.*;
            function consume(owned s: String): None { return None; }
            function main(): None {
                s: String = "hello";
                consume(s);
                println(s);
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors.iter().any(|m| m.contains("Use of moved value 's'")));
    }

    #[test]
    fn detects_move_while_borrowed() {
        let source = r#"
            function consume(owned s: String): None { return None; }
            function main(): None {
                s: String = "hello";
                r: &String = &s;
                consume(s);
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors
            .iter()
            .any(|m| m.contains("Cannot move 's' while borrowed")));
    }

    #[test]
    fn detects_double_mutable_borrow() {
        let source = r#"
            function main(): None {
                mut x: Integer = 1;
                a: &mut Integer = &mut x;
                b: &mut Integer = &mut x;
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors
            .iter()
            .any(|m| m.contains("Cannot borrow 'x' while mutably borrowed")));
    }

    #[test]
    fn immutable_borrow_released_after_scope() {
        let source = r#"
            function consume(owned s: String): None { return None; }
            function main(): None {
                s: String = "hello";
                if (true) {
                    r: &String = &s;
                }
                consume(s);
                return None;
            }
        "#;
        borrow_ok(source);
    }

    #[test]
    fn lambda_capture_marks_move() {
        let source = r#"
            import std.io.*;
            function consume(owned s: String): None { return None; }
            function main(): None {
                s: String = "hello";
                f: () -> None = () => consume(s);
                println(s);
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors.iter().any(|m| m.contains("Use of moved value 's'")));
    }

    #[test]
    fn compound_assign_on_mut_borrowed_variable_is_rejected() {
        let source = r#"
            function main(): None {
                mut x: Integer = 10;
                r: &mut Integer = &mut x;
                x += 1;
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors
            .iter()
            .any(|m| m.contains("Cannot assign to 'x' while mutably borrowed")));
    }
}
