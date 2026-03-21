//! Apex Borrow Checker - Ownership and lifetime analysis
//!
//! This module provides:
//! - Move semantics checking
//! - Borrow lifetime tracking
//! - Mutable borrow exclusivity
//! - Use-after-move detection

#![allow(dead_code)]

use crate::ast::*;
use crate::stdlib::stdlib_registry;
use std::collections::{HashMap, HashSet};

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
    /// Stdlib function names for default borrow-mode fallback
    stdlib_functions: std::collections::HashSet<String>,
    /// Import aliases (alias -> namespace path), e.g. io -> std.io
    import_aliases: HashMap<String, String>,
    /// Current scope depth
    scope_depth: usize,
    /// Collected errors
    errors: Vec<BorrowError>,
    /// Variables that need dropping at end of current scope
    drop_queue: Vec<Vec<String>>,
}

struct ClassBorrowSigs {
    methods: HashMap<String, MethodBorrowSig>,
    constructor: Vec<ParamMode>,
    field_types: HashMap<String, Type>,
}

struct MethodBorrowSig {
    receiver_mode: ParamMode,
    params: Vec<ParamMode>,
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            borrows: Vec::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            stdlib_functions: stdlib_registry().get_functions().keys().cloned().collect(),
            import_aliases: HashMap::new(),
            scope_depth: 0,
            errors: Vec::new(),
            drop_queue: vec![Vec::new()],
        }
    }

    fn apply_mutating_method_seeds(
        &mut self,
        class_mutating_methods: &HashMap<String, HashSet<String>>,
    ) {
        for (class_name, methods) in class_mutating_methods {
            if let Some(class) = self.classes.get_mut(class_name) {
                for (method_name, sig) in &mut class.methods {
                    sig.receiver_mode = if methods.contains(method_name) {
                        ParamMode::BorrowMut
                    } else {
                        ParamMode::Borrow
                    };
                }
            }
        }
    }

    pub fn export_class_mutating_method_summary(&self) -> HashMap<String, HashSet<String>> {
        self.classes
            .iter()
            .map(|(class_name, class)| {
                (
                    class_name.clone(),
                    class
                        .methods
                        .iter()
                        .filter_map(|(method_name, sig)| {
                            (sig.receiver_mode == ParamMode::BorrowMut)
                                .then_some(method_name.clone())
                        })
                        .collect(),
                )
            })
            .collect()
    }

    pub fn check_with_mutating_method_seeds(
        &mut self,
        program: &Program,
        class_mutating_methods: &HashMap<String, HashSet<String>>,
    ) -> Result<(), Vec<BorrowError>> {
        for decl in &program.declarations {
            self.collect_sig(&decl.node);
        }
        self.apply_mutating_method_seeds(class_mutating_methods);

        for decl in &program.declarations {
            self.check_decl(&decl.node, decl.span.clone());
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
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
        self.collect_sig_with_prefix(decl, None);
    }

    fn collect_sig_with_prefix(&mut self, decl: &Decl, module_prefix: Option<&str>) {
        match decl {
            Decl::Function(func) => {
                let qualified_name = module_prefix
                    .map(|prefix| format!("{}__{}", prefix, func.name))
                    .unwrap_or_else(|| func.name.clone());
                self.functions
                    .insert(qualified_name, func.params.iter().map(|p| p.mode).collect());
                if module_prefix.is_none() {
                    self.functions.insert(
                        func.name.clone(),
                        func.params.iter().map(|p| p.mode).collect(),
                    );
                }
            }
            Decl::Class(class) => {
                let mut methods = HashMap::new();
                let mutating_methods = Self::class_mutating_methods(class);
                for method in &class.methods {
                    methods.insert(
                        method.name.clone(),
                        MethodBorrowSig {
                            receiver_mode: if mutating_methods.contains(&method.name) {
                                ParamMode::BorrowMut
                            } else {
                                ParamMode::Borrow
                            },
                            params: method.params.iter().map(|p| p.mode).collect(),
                        },
                    );
                }
                let constructor = class
                    .constructor
                    .as_ref()
                    .map(|c| c.params.iter().map(|p| p.mode).collect())
                    .unwrap_or_default();
                let field_types = class
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), f.ty.clone()))
                    .collect();

                self.classes.insert(
                    class.name.clone(),
                    ClassBorrowSigs {
                        methods,
                        constructor,
                        field_types,
                    },
                );
            }
            Decl::Module(module) => {
                let next_prefix = module_prefix
                    .map(|prefix| format!("{}__{}", prefix, module.name))
                    .unwrap_or_else(|| module.name.clone());
                for inner in &module.declarations {
                    self.collect_sig_with_prefix(&inner.node, Some(&next_prefix));
                }
            }
            Decl::Import(import) => {
                if let Some(alias) = &import.alias {
                    self.import_aliases
                        .insert(alias.clone(), import.path.clone());
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
            let param_mutable = param.mutable || matches!(param.mode, ParamMode::BorrowMut);
            self.declare_var(
                &param.name,
                param_mutable,
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
                let param_mutable = param.mutable || matches!(param.mode, ParamMode::BorrowMut);
                self.declare_var(
                    &param.name,
                    param_mutable,
                    0..0,
                    self.needs_drop(&param.ty),
                    Some(param.ty.clone()),
                );
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
                let param_mutable = param.mutable || matches!(param.mode, ParamMode::BorrowMut);
                self.declare_var(
                    &param.name,
                    param_mutable,
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
            if Self::stmt_always_terminates(&stmt.node) {
                break;
            }
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

                // If a reference is initialized from a call that borrows an argument,
                // keep that borrow alive for the lifetime of this scope.
                if matches!(ty, Type::Ref(_) | Type::MutRef(_)) {
                    self.bind_reference_origin_borrow(&value.node, value.span.clone());
                }

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
                match Self::literal_bool(&condition.node) {
                    Some(true) => self.check_block(then_block),
                    Some(false) => {
                        if let Some(else_blk) = else_block {
                            self.check_block(else_blk);
                        }
                    }
                    None => {
                        self.check_block(then_block);
                        if let Some(else_blk) = else_block {
                            self.check_block(else_blk);
                        }
                    }
                }
            }

            Stmt::While { condition, body } => {
                self.check_expr(&condition.node, condition.span.clone(), false);
                if !matches!(Self::literal_bool(&condition.node), Some(false)) {
                    self.check_block(body);
                }
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
                    if Self::stmt_terminates_control_flow(&stmt.node) {
                        break;
                    }
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
                        if Self::stmt_terminates_control_flow(&stmt.node) {
                            break;
                        }
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

                let mut assignment_valid = true;
                if !mutable {
                    self.errors.push(BorrowError::new(
                        format!("Cannot assign to immutable variable '{}'", name),
                        span.clone(),
                    ));
                    assignment_valid = false;
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
                        assignment_valid = false;
                    }
                    OwnershipState::Borrowed(count) if count > 0 => {
                        self.errors.push(BorrowError::new(
                            format!("Cannot assign to '{}' while borrowed", name),
                            span.clone(),
                        ));
                        assignment_valid = false;
                    }
                    _ => {}
                }

                // Reset ownership state (old value dropped)
                if assignment_valid {
                    if let Some(var) = self.get_var_mut(name) {
                        var.state = OwnershipState::Owned;
                    }
                }
            }
            Expr::Field { object, field: _ } => {
                self.check_owner_mutability_for_assignment(&object.node, span.clone());
                self.check_owner_borrow_state_for_assignment(&object.node, span.clone());
                self.check_expr(&object.node, object.span.clone(), false);
            }
            Expr::Index { object, index } => {
                self.check_owner_mutability_for_assignment(&object.node, span.clone());
                self.check_owner_borrow_state_for_assignment(&object.node, span.clone());
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

    fn check_owner_borrow_state_for_assignment(&mut self, expr: &Expr, span: Span) {
        match expr {
            Expr::Ident(name) => {
                if let Some(var) = self.get_var(name) {
                    match &var.state {
                        OwnershipState::MutBorrowed(borrow_span) => {
                            self.errors.push(
                                BorrowError::new(
                                    format!(
                                        "Cannot assign through '{}' while mutably borrowed",
                                        name
                                    ),
                                    span.clone(),
                                )
                                .with_note("Mutable borrow occurred here", borrow_span.clone()),
                            );
                        }
                        OwnershipState::Borrowed(count) if *count > 0 => {
                            self.errors.push(BorrowError::new(
                                format!("Cannot assign through '{}' while borrowed", name),
                                span.clone(),
                            ));
                        }
                        _ => {}
                    }
                }
            }
            Expr::Field { object, .. } | Expr::Index { object, .. } => {
                self.check_owner_borrow_state_for_assignment(&object.node, span);
            }
            _ => {}
        }
    }

    fn check_owner_mutability_for_assignment(&mut self, expr: &Expr, span: Span) {
        match expr {
            Expr::Ident(name) => {
                if let Some(var) = self.get_var(name) {
                    if !var.mutable {
                        self.errors.push(BorrowError::new(
                            format!("Cannot assign through immutable variable '{}'", name),
                            span,
                        ));
                    }
                }
            }
            Expr::Field { object, .. } | Expr::Index { object, .. } => {
                self.check_owner_mutability_for_assignment(&object.node, span);
            }
            Expr::This | Expr::Deref(_) => {}
            _ => {}
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

            Expr::Binary { left, right, op } => {
                self.check_expr(&left.node, left.span.clone(), false);
                let should_check_right = !matches!(
                    (op, Self::literal_bool(&left.node)),
                    (BinOp::Or, Some(true)) | (BinOp::And, Some(false))
                );
                if should_check_right {
                    self.check_expr(&right.node, right.span.clone(), false);
                }
            }

            Expr::Unary { expr: inner, .. } => {
                self.check_expr(&inner.node, inner.span.clone(), false);
            }

            Expr::Call { callee, args, .. } => {
                self.check_expr(&callee.node, callee.span.clone(), false);

                // Borrows created to satisfy receiver/argument modes are
                // temporary for this call expression.
                self.enter_scope();
                if let Some(mode) = self.resolve_call_receiver_mode(&callee.node) {
                    if let Expr::Field { object, .. } = &callee.node {
                        self.apply_receiver_mode(&object.node, mode, callee.span.clone());
                    }
                }

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
                self.exit_scope();
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
                // Lambda captures - free vars borrow or move from outer scope.
                // Capture effects apply at lambda creation site (outer scope).
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                let free_idents = Self::collect_free_idents(&body.node, &param_names);
                let mut moved_captures = Vec::new();
                for ident in free_idents {
                    if self.get_var(&ident).is_none() {
                        continue;
                    }
                    if self.expr_moves_ident(&body.node, &ident) {
                        moved_captures.push(ident);
                    } else {
                        self.create_borrow(&ident, false, span.clone());
                    }
                }

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

                self.check_expr(&body.node, body.span.clone(), false);
                // Mark owned captures as moved after body analysis to avoid false
                // use-after-move reports inside the lambda expression itself.
                for ident in moved_captures {
                    self.try_move(&Expr::Ident(ident), span.clone());
                }
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
                // Async blocks capture from outer scope. Non-moved captures must
                // keep a borrow active after async block creation.
                let local_declared = Self::collect_declared_names_in_block(body);
                let mut free_idents = Vec::new();
                for stmt in body {
                    Self::collect_free_idents_stmt(&stmt.node, &local_declared, &mut free_idents);
                }
                let mut seen = HashSet::new();
                free_idents.retain(|name| seen.insert(name.clone()));

                let capture_moves: HashMap<String, bool> = free_idents
                    .iter()
                    .map(|ident| {
                        let moved = body
                            .iter()
                            .any(|stmt| self.stmt_moves_ident(&stmt.node, ident));
                        (ident.clone(), moved)
                    })
                    .collect();
                let capture_mut_borrows: HashMap<String, bool> = free_idents
                    .iter()
                    .map(|ident| {
                        let mut_borrowed = body
                            .iter()
                            .any(|stmt| self.stmt_mutably_borrows_ident(&stmt.node, ident));
                        (ident.clone(), mut_borrowed)
                    })
                    .collect();

                self.enter_scope();
                for stmt in body {
                    self.check_stmt(&stmt.node, stmt.span.clone());
                }
                self.exit_scope();

                for ident in free_idents {
                    if self.get_var(&ident).is_none() {
                        continue;
                    }
                    if capture_moves.get(&ident).copied().unwrap_or(false) {
                        continue;
                    }
                    if capture_mut_borrows.get(&ident).copied().unwrap_or(false) {
                        self.create_borrow(&ident, true, span.clone());
                    } else {
                        self.create_borrow(&ident, false, span.clone());
                    }
                }
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

    fn is_borrowing_stdlib_call(&self, name: &str) -> bool {
        self.stdlib_functions.contains(name)
    }

    fn resolve_stdlib_alias_call_name(&self, alias_ident: &str, member: &str) -> Option<String> {
        // Local bindings must shadow import aliases.
        if self.get_var(alias_ident).is_some() {
            return None;
        }
        let namespace_path = self.import_aliases.get(alias_ident)?;
        stdlib_registry().resolve_alias_call(namespace_path, member)
    }

    fn infer_expr_class(&self, expr: &Expr) -> Option<String> {
        let ty = self.infer_expr_type(expr)?;
        match ty {
            Type::Named(class_name) => Some(class_name),
            Type::Generic(class_name, _) => Some(class_name),
            _ => None,
        }
    }

    fn infer_expr_type(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Ident(name) => self.get_var(name)?.ty.clone(),
            Expr::This => self.get_var("this")?.ty.clone(),
            Expr::Field { object, field } => {
                let owner_class = self.infer_expr_class(&object.node)?;
                self.classes
                    .get(&owner_class)?
                    .field_types
                    .get(field)
                    .cloned()
            }
            _ => None,
        }
    }

    fn resolve_call_param_modes(&self, callee: &Expr, arg_len: usize) -> Vec<ParamMode> {
        let param_modes = Vec::new();

        if let Expr::Ident(name) = callee {
            if self.is_borrowing_stdlib_call(name) {
                return vec![ParamMode::Borrow; arg_len];
            }
            if let Some(modes) = self.functions.get(name) {
                return modes.clone();
            }
            return param_modes;
        }

        if let Expr::Field { object, field } = callee {
            // Prefer type-directed method resolution first when receiver type is known.
            if let Some(class_name) = self.infer_expr_class(&object.node) {
                if let Some(class_sig) = self.classes.get(&class_name) {
                    if let Some(sig) = class_sig.methods.get(field) {
                        return sig.params.clone();
                    }
                }
            }

            if let Expr::Ident(name) = &object.node {
                if let Some(canonical) = self.resolve_stdlib_alias_call_name(name, field) {
                    if self.is_borrowing_stdlib_call(&canonical) {
                        return vec![ParamMode::Borrow; arg_len];
                    }
                    if let Some(modes) = self.functions.get(&canonical) {
                        return modes.clone();
                    }
                }

                let mangled = format!("{}__{}", name, field);
                if let Some(modes) = self.functions.get(&mangled) {
                    return modes.clone();
                }
                if self.is_borrowing_stdlib_call(&mangled) {
                    return vec![ParamMode::Borrow; arg_len];
                }
            }

            if let Some(path_parts) = Self::flatten_field_chain(callee) {
                let mangled = path_parts.join("__");
                if let Some(modes) = self.functions.get(&mangled) {
                    return modes.clone();
                }
                if self.is_borrowing_stdlib_call(&mangled) {
                    return vec![ParamMode::Borrow; arg_len];
                }
            }

            // Unknown method receiver type: stay conservative and avoid
            // implicit move-default on arguments for member-call syntax.
            return vec![ParamMode::Borrow; arg_len];
        }

        param_modes
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

    fn resolve_call_receiver_mode(&self, callee: &Expr) -> Option<ParamMode> {
        let Expr::Field { object, field } = callee else {
            return None;
        };
        let class_name = self.infer_expr_class(&object.node)?;
        let class_sig = self.classes.get(&class_name)?;
        class_sig.methods.get(field).map(|sig| sig.receiver_mode)
    }

    fn apply_receiver_mode(&mut self, receiver: &Expr, mode: ParamMode, span: Span) {
        match receiver {
            Expr::Ident(name) => match mode {
                ParamMode::Borrow => self.create_borrow(name, false, span),
                ParamMode::BorrowMut => self.create_receiver_borrow(name, true, span),
                ParamMode::Owned => self.try_move(receiver, span),
            },
            Expr::Field { object, .. } | Expr::Index { object, .. } => {
                self.apply_receiver_mode(&object.node, mode, span)
            }
            Expr::This => {}
            _ => {}
        }
    }

    fn create_receiver_borrow(&mut self, name: &str, mutable: bool, span: Span) {
        let (state, is_mutable) = {
            if let Some(var) = self.get_var(name) {
                (var.state.clone(), var.mutable)
            } else {
                return;
            }
        };

        if mutable && !is_mutable {
            self.errors.push(BorrowError::new(
                format!("Cannot mutably borrow immutable variable '{}'", name),
                span,
            ));
            return;
        }

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

    fn class_mutating_methods(class: &ClassDecl) -> std::collections::HashSet<String> {
        let mut mutating: std::collections::HashSet<String> = class
            .methods
            .iter()
            .filter(|m| Self::block_mutates_this(&m.body))
            .map(|m| m.name.clone())
            .collect();

        let mut changed = true;
        while changed {
            changed = false;
            for method in &class.methods {
                if mutating.contains(&method.name) {
                    continue;
                }
                if Self::block_calls_this_method_in_set(&method.body, &mutating) {
                    mutating.insert(method.name.clone());
                    changed = true;
                }
            }
        }
        mutating
    }

    fn block_mutates_this(block: &Block) -> bool {
        block.iter().any(|stmt| Self::stmt_mutates_this(&stmt.node))
    }

    fn stmt_mutates_this(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Assign { target, .. } => Self::expr_root_is_this(&target.node),
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                Self::block_mutates_this(then_block)
                    || else_block.as_ref().is_some_and(Self::block_mutates_this)
            }
            Stmt::While { body, .. } => Self::block_mutates_this(body),
            Stmt::For { body, .. } => Self::block_mutates_this(body),
            Stmt::Match { arms, .. } => arms.iter().any(|arm| Self::block_mutates_this(&arm.body)),
            Stmt::Expr(_) | Stmt::Let { .. } | Stmt::Return(_) | Stmt::Break | Stmt::Continue => {
                false
            }
        }
    }

    fn expr_root_is_this(expr: &Expr) -> bool {
        match expr {
            Expr::This => true,
            Expr::Field { object, .. } | Expr::Index { object, .. } => {
                Self::expr_root_is_this(&object.node)
            }
            Expr::Deref(inner) => Self::expr_root_is_this(&inner.node),
            _ => false,
        }
    }

    fn block_calls_this_method_in_set(
        block: &Block,
        methods: &std::collections::HashSet<String>,
    ) -> bool {
        block
            .iter()
            .any(|stmt| Self::stmt_calls_this_method_in_set(&stmt.node, methods))
    }

    fn stmt_calls_this_method_in_set(
        stmt: &Stmt,
        methods: &std::collections::HashSet<String>,
    ) -> bool {
        match stmt {
            Stmt::Let { value, .. } => Self::expr_calls_this_method_in_set(&value.node, methods),
            Stmt::Assign { target, value } => {
                Self::expr_calls_this_method_in_set(&target.node, methods)
                    || Self::expr_calls_this_method_in_set(&value.node, methods)
            }
            Stmt::Expr(expr) => Self::expr_calls_this_method_in_set(&expr.node, methods),
            Stmt::Return(expr) => expr
                .as_ref()
                .is_some_and(|e| Self::expr_calls_this_method_in_set(&e.node, methods)),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                if Self::expr_calls_this_method_in_set(&condition.node, methods) {
                    return true;
                }
                match Self::literal_bool(&condition.node) {
                    Some(true) => Self::block_calls_this_method_in_set(then_block, methods),
                    Some(false) => else_block
                        .as_ref()
                        .is_some_and(|b| Self::block_calls_this_method_in_set(b, methods)),
                    None => {
                        Self::block_calls_this_method_in_set(then_block, methods)
                            || else_block
                                .as_ref()
                                .is_some_and(|b| Self::block_calls_this_method_in_set(b, methods))
                    }
                }
            }
            Stmt::While { condition, body } => {
                Self::expr_calls_this_method_in_set(&condition.node, methods)
                    || Self::block_calls_this_method_in_set(body, methods)
            }
            Stmt::For { iterable, body, .. } => {
                Self::expr_calls_this_method_in_set(&iterable.node, methods)
                    || Self::block_calls_this_method_in_set(body, methods)
            }
            Stmt::Match { expr, arms } => {
                Self::expr_calls_this_method_in_set(&expr.node, methods)
                    || arms
                        .iter()
                        .any(|arm| Self::block_calls_this_method_in_set(&arm.body, methods))
            }
            Stmt::Break | Stmt::Continue => false,
        }
    }

    fn expr_calls_this_method_in_set(
        expr: &Expr,
        methods: &std::collections::HashSet<String>,
    ) -> bool {
        match expr {
            Expr::Call { callee, args, .. } => {
                let calls_mutating = matches!(
                    &callee.node,
                    Expr::Field { object, field }
                        if matches!(&object.node, Expr::This) && methods.contains(field)
                );
                calls_mutating
                    || Self::expr_calls_this_method_in_set(&callee.node, methods)
                    || args
                        .iter()
                        .any(|a| Self::expr_calls_this_method_in_set(&a.node, methods))
            }
            Expr::Binary { left, right, op } => {
                if Self::expr_calls_this_method_in_set(&left.node, methods) {
                    return true;
                }
                let should_check_right = !matches!(
                    (op, Self::literal_bool(&left.node)),
                    (BinOp::Or, Some(true)) | (BinOp::And, Some(false))
                );
                should_check_right && Self::expr_calls_this_method_in_set(&right.node, methods)
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => Self::expr_calls_this_method_in_set(&expr.node, methods),
            Expr::Field { object, .. } => {
                Self::expr_calls_this_method_in_set(&object.node, methods)
            }
            Expr::Index { object, index } => {
                Self::expr_calls_this_method_in_set(&object.node, methods)
                    || Self::expr_calls_this_method_in_set(&index.node, methods)
            }
            Expr::Construct { args, .. } => args
                .iter()
                .any(|a| Self::expr_calls_this_method_in_set(&a.node, methods)),
            Expr::Lambda { body, .. } => Self::expr_calls_this_method_in_set(&body.node, methods),
            Expr::Match { expr, arms } => {
                Self::expr_calls_this_method_in_set(&expr.node, methods)
                    || arms.iter().any(|arm| {
                        arm.body
                            .iter()
                            .any(|s| Self::stmt_calls_this_method_in_set(&s.node, methods))
                    })
            }
            Expr::StringInterp(parts) => parts.iter().any(|p| match p {
                StringPart::Expr(e) => Self::expr_calls_this_method_in_set(&e.node, methods),
                StringPart::Literal(_) => false,
            }),
            Expr::AsyncBlock(stmts) | Expr::Block(stmts) => stmts
                .iter()
                .any(|s| Self::stmt_calls_this_method_in_set(&s.node, methods)),
            Expr::Require { condition, message } => {
                Self::expr_calls_this_method_in_set(&condition.node, methods)
                    || message
                        .as_ref()
                        .is_some_and(|m| Self::expr_calls_this_method_in_set(&m.node, methods))
            }
            Expr::Range { start, end, .. } => {
                start
                    .as_ref()
                    .is_some_and(|s| Self::expr_calls_this_method_in_set(&s.node, methods))
                    || end
                        .as_ref()
                        .is_some_and(|e| Self::expr_calls_this_method_in_set(&e.node, methods))
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expr_calls_this_method_in_set(&condition.node, methods)
                    || then_branch
                        .iter()
                        .any(|s| Self::stmt_calls_this_method_in_set(&s.node, methods))
                    || else_branch.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| Self::stmt_calls_this_method_in_set(&s.node, methods))
                    })
            }
            Expr::Ident(_) | Expr::Literal(_) | Expr::This => false,
        }
    }

    fn bind_reference_origin_borrow(&mut self, value: &Expr, span: Span) {
        let Expr::Call { callee, args, .. } = value else {
            return;
        };
        let param_modes = self.resolve_call_param_modes(&callee.node, args.len());
        for (i, arg) in args.iter().enumerate() {
            let mode = param_modes.get(i).copied().unwrap_or(ParamMode::Owned);
            let Expr::Ident(name) = &arg.node else {
                continue;
            };
            match mode {
                ParamMode::Borrow => self.create_borrow(name, false, span.clone()),
                ParamMode::BorrowMut => self.create_borrow(name, true, span.clone()),
                ParamMode::Owned => {}
            }
        }
    }

    fn collect_declared_names_in_block(block: &[Spanned<Stmt>]) -> Vec<String> {
        let mut names = HashSet::new();
        for stmt in block {
            Self::collect_declared_names_stmt(&stmt.node, &mut names);
        }
        names.into_iter().collect()
    }

    fn collect_declared_names_stmt(stmt: &Stmt, out: &mut HashSet<String>) {
        match stmt {
            Stmt::Let { name, .. } => {
                out.insert(name.clone());
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                for stmt in then_block {
                    Self::collect_declared_names_stmt(&stmt.node, out);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        Self::collect_declared_names_stmt(&stmt.node, out);
                    }
                }
            }
            Stmt::While { body, .. } => {
                for stmt in body {
                    Self::collect_declared_names_stmt(&stmt.node, out);
                }
            }
            Stmt::For { var, body, .. } => {
                out.insert(var.clone());
                for stmt in body {
                    Self::collect_declared_names_stmt(&stmt.node, out);
                }
            }
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    Self::collect_pattern_bindings(&arm.pattern, out);
                    for stmt in &arm.body {
                        Self::collect_declared_names_stmt(&stmt.node, out);
                    }
                }
            }
            Stmt::Expr(_)
            | Stmt::Return(_)
            | Stmt::Break
            | Stmt::Continue
            | Stmt::Assign { .. } => {}
        }
    }

    fn collect_pattern_bindings(pattern: &Pattern, out: &mut HashSet<String>) {
        match pattern {
            Pattern::Ident(name) => {
                out.insert(name.clone());
            }
            Pattern::Variant(_, bindings) => {
                for binding in bindings {
                    out.insert(binding.clone());
                }
            }
            Pattern::Wildcard | Pattern::Literal(_) => {}
        }
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
            Expr::Call { callee, args, .. } => {
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
            Expr::Call { callee, args, .. } => {
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
            Expr::Binary { left, right, op } => {
                if self.expr_moves_ident(&left.node, ident) {
                    return true;
                }
                let should_check_right = !matches!(
                    (op, Self::literal_bool(&left.node)),
                    (BinOp::Or, Some(true)) | (BinOp::And, Some(false))
                );
                should_check_right && self.expr_moves_ident(&right.node, ident)
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
                if self.expr_moves_ident(&condition.node, ident) {
                    return true;
                }
                match Self::literal_bool(&condition.node) {
                    Some(true) => then_block
                        .iter()
                        .any(|stmt| self.stmt_moves_ident(&stmt.node, ident)),
                    Some(false) => else_block
                        .as_ref()
                        .map(|stmts| {
                            stmts
                                .iter()
                                .any(|stmt| self.stmt_moves_ident(&stmt.node, ident))
                        })
                        .unwrap_or(false),
                    None => {
                        then_block
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
                }
            }
            Stmt::While { condition, body } => {
                self.expr_moves_ident(&condition.node, ident)
                    || (!matches!(Self::literal_bool(&condition.node), Some(false))
                        && body
                            .iter()
                            .any(|stmt| self.stmt_moves_ident(&stmt.node, ident)))
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

    fn expr_mutably_borrows_ident(&self, expr: &Expr, ident: &str) -> bool {
        match expr {
            Expr::MutBorrow(inner) => {
                matches!(&inner.node, Expr::Ident(name) if name == ident)
                    || self.expr_mutably_borrows_ident(&inner.node, ident)
            }
            Expr::Call { callee, args, .. } => {
                let param_modes = self.resolve_call_param_modes(&callee.node, args.len());
                if args.iter().enumerate().any(|(i, arg)| {
                    matches!(param_modes.get(i), Some(ParamMode::BorrowMut))
                        && matches!(&arg.node, Expr::Ident(name) if name == ident)
                }) {
                    return true;
                }
                if let Some(mode) = self.resolve_call_receiver_mode(&callee.node) {
                    if mode == ParamMode::BorrowMut
                        && matches!(&callee.node, Expr::Field { object, .. } if matches!(&object.node, Expr::Ident(name) if name == ident))
                    {
                        return true;
                    }
                }
                self.expr_mutably_borrows_ident(&callee.node, ident)
                    || args
                        .iter()
                        .any(|arg| self.expr_mutably_borrows_ident(&arg.node, ident))
            }
            Expr::Binary { left, right, op } => {
                if self.expr_mutably_borrows_ident(&left.node, ident) {
                    return true;
                }
                let should_check_right = !matches!(
                    (op, Self::literal_bool(&left.node)),
                    (BinOp::Or, Some(true)) | (BinOp::And, Some(false))
                );
                should_check_right && self.expr_mutably_borrows_ident(&right.node, ident)
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => self.expr_mutably_borrows_ident(&expr.node, ident),
            Expr::Field { object, .. } => self.expr_mutably_borrows_ident(&object.node, ident),
            Expr::Index { object, index } => {
                self.expr_mutably_borrows_ident(&object.node, ident)
                    || self.expr_mutably_borrows_ident(&index.node, ident)
            }
            Expr::Construct { args, .. } => args
                .iter()
                .any(|arg| self.expr_mutably_borrows_ident(&arg.node, ident)),
            Expr::Lambda { body, .. } => self.expr_mutably_borrows_ident(&body.node, ident),
            Expr::Match { expr, arms } => {
                self.expr_mutably_borrows_ident(&expr.node, ident)
                    || arms.iter().any(|arm| {
                        arm.body
                            .iter()
                            .any(|stmt| self.stmt_mutably_borrows_ident(&stmt.node, ident))
                    })
            }
            Expr::StringInterp(parts) => parts.iter().any(|part| match part {
                StringPart::Expr(e) => self.expr_mutably_borrows_ident(&e.node, ident),
                StringPart::Literal(_) => false,
            }),
            Expr::AsyncBlock(stmts) | Expr::Block(stmts) => stmts
                .iter()
                .any(|stmt| self.stmt_mutably_borrows_ident(&stmt.node, ident)),
            Expr::Require { condition, message } => {
                self.expr_mutably_borrows_ident(&condition.node, ident)
                    || message
                        .as_ref()
                        .map(|m| self.expr_mutably_borrows_ident(&m.node, ident))
                        .unwrap_or(false)
            }
            Expr::Range { start, end, .. } => {
                start
                    .as_ref()
                    .map(|s| self.expr_mutably_borrows_ident(&s.node, ident))
                    .unwrap_or(false)
                    || end
                        .as_ref()
                        .map(|e| self.expr_mutably_borrows_ident(&e.node, ident))
                        .unwrap_or(false)
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expr_mutably_borrows_ident(&condition.node, ident)
                    || then_branch
                        .iter()
                        .any(|stmt| self.stmt_mutably_borrows_ident(&stmt.node, ident))
                    || else_branch
                        .as_ref()
                        .map(|stmts| {
                            stmts
                                .iter()
                                .any(|stmt| self.stmt_mutably_borrows_ident(&stmt.node, ident))
                        })
                        .unwrap_or(false)
            }
            Expr::Ident(_) | Expr::Literal(_) | Expr::This => false,
        }
    }

    fn stmt_mutably_borrows_ident(&self, stmt: &Stmt, ident: &str) -> bool {
        match stmt {
            Stmt::Let { value, .. } => self.expr_mutably_borrows_ident(&value.node, ident),
            Stmt::Assign { target, value } => {
                self.expr_mutably_borrows_ident(&target.node, ident)
                    || self.expr_mutably_borrows_ident(&value.node, ident)
            }
            Stmt::Expr(expr) => self.expr_mutably_borrows_ident(&expr.node, ident),
            Stmt::Return(expr) => expr
                .as_ref()
                .map(|e| self.expr_mutably_borrows_ident(&e.node, ident))
                .unwrap_or(false),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.expr_mutably_borrows_ident(&condition.node, ident)
                    || then_block
                        .iter()
                        .any(|stmt| self.stmt_mutably_borrows_ident(&stmt.node, ident))
                    || else_block
                        .as_ref()
                        .map(|stmts| {
                            stmts
                                .iter()
                                .any(|stmt| self.stmt_mutably_borrows_ident(&stmt.node, ident))
                        })
                        .unwrap_or(false)
            }
            Stmt::While { condition, body } => {
                self.expr_mutably_borrows_ident(&condition.node, ident)
                    || body
                        .iter()
                        .any(|stmt| self.stmt_mutably_borrows_ident(&stmt.node, ident))
            }
            Stmt::For { iterable, body, .. } => {
                self.expr_mutably_borrows_ident(&iterable.node, ident)
                    || body
                        .iter()
                        .any(|stmt| self.stmt_mutably_borrows_ident(&stmt.node, ident))
            }
            Stmt::Match { expr, arms } => {
                self.expr_mutably_borrows_ident(&expr.node, ident)
                    || arms.iter().any(|arm| {
                        arm.body
                            .iter()
                            .any(|stmt| self.stmt_mutably_borrows_ident(&stmt.node, ident))
                    })
            }
            Stmt::Break | Stmt::Continue => false,
        }
    }

    fn stmt_terminates_control_flow(stmt: &Stmt) -> bool {
        matches!(stmt, Stmt::Return(_) | Stmt::Break | Stmt::Continue)
    }

    fn stmt_always_terminates(stmt: &Stmt) -> bool {
        if Self::stmt_terminates_control_flow(stmt) {
            return true;
        }
        match stmt {
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => match Self::literal_bool(&condition.node) {
                Some(true) => Self::block_always_terminates(then_block),
                Some(false) => else_block
                    .as_ref()
                    .is_some_and(Self::block_always_terminates),
                None => false,
            },
            _ => false,
        }
    }

    fn block_always_terminates(block: &Block) -> bool {
        for stmt in block {
            if Self::stmt_always_terminates(&stmt.node) {
                return true;
            }
        }
        false
    }

    fn literal_bool(expr: &Expr) -> Option<bool> {
        if let Expr::Literal(Literal::Boolean(v)) = expr {
            Some(*v)
        } else {
            None
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
    fn lambda_owned_capture_does_not_fail_inside_lambda() {
        let source = r#"
            function consume(owned s: String): None { return None; }
            function main(): None {
                s: String = "hello";
                f: () -> None = () => consume(s);
                return None;
            }
        "#;
        borrow_ok(source);
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

    #[test]
    fn field_assign_on_borrowed_owner_is_rejected() {
        let source = r#"
            class C {
                mut value: Integer;
                constructor(v: Integer) { this.value = v; }
            }
            function main(): None {
                mut c: C = C(1);
                r: &C = &c;
                c.value += 1;
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors
            .iter()
            .any(|m| m.contains("Cannot assign through 'c' while borrowed")));
    }

    #[test]
    fn this_method_uses_declared_param_modes() {
        let source = r#"
            import std.io.*;
            class A {
                function take(borrow s: String): None { return None; }
                function run(): None {
                    s: String = "x";
                    this.take(s);
                    println(s);
                    return None;
                }
            }
            function main(): None {
                a: A = A();
                a.run();
                return None;
            }
        "#;
        borrow_ok(source);
    }

    #[test]
    fn invalid_assign_does_not_clear_borrow_state() {
        let source = r#"
            function consume(owned s: String): None { return None; }
            function main(): None {
                mut s: String = "a";
                r: &String = &s;
                s = "b";
                consume(s);
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors
            .iter()
            .any(|m| m.contains("Cannot assign to 's' while borrowed")));
        assert!(errors
            .iter()
            .any(|m| m.contains("Cannot move 's' while borrowed")));
    }

    #[test]
    fn stdlib_alias_call_borrows_instead_of_moves() {
        let source = r#"
            import std.io as io;
            function consume(owned s: String): None { return None; }
            function main(): None {
                s: String = "x";
                io.println(s);
                consume(s);
                return None;
            }
        "#;
        borrow_ok(source);
    }

    #[test]
    fn reference_return_from_borrow_keeps_source_borrowed() {
        let source = r#"
            function id_borrow(borrow s: String): &String { return &s; }
            function consume(owned s: String): None { return None; }
            function main(): None {
                s: String = "x";
                r: &String = id_borrow(s);
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
    fn lambda_borrow_capture_blocks_move_after_creation() {
        let source = r#"
            function take_borrow(borrow s: String): None { return None; }
            function consume(owned s: String): None { return None; }
            function main(): None {
                s: String = "x";
                f: () -> None = () => take_borrow(s);
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
    fn immutable_borrow_blocks_mutating_method_call() {
        let source = r#"
            class C {
                mut v: Integer;
                constructor(v: Integer) { this.v = v; }
                function touch(): None { this.v += 1; return None; }
            }
            function main(): None {
                mut c: C = C(1);
                r: &C = &c;
                c.touch();
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors
            .iter()
            .any(|m| m.contains("Cannot mutably borrow 'c' while immutably borrowed")));
    }

    #[test]
    fn immutable_borrow_allows_read_only_method_call() {
        let source = r#"
            class C {
                mut v: Integer;
                constructor(v: Integer) { this.v = v; }
                function get(): Integer { return this.v; }
            }
            function main(): None {
                mut c: C = C(1);
                r: &C = &c;
                x: Integer = c.get();
                return None;
            }
        "#;
        borrow_ok(source);
    }

    #[test]
    fn immutable_borrow_blocks_transitively_mutating_method_call() {
        let source = r#"
            class C {
                mut v: Integer;
                constructor(v: Integer) { this.v = v; }
                function touch2(): None { this.v += 1; return None; }
                function wrapper(): None { this.touch2(); return None; }
            }
            function main(): None {
                mut c: C = C(1);
                r: &C = &c;
                c.wrapper();
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors
            .iter()
            .any(|m| m.contains("Cannot mutably borrow 'c' while immutably borrowed")));
    }

    #[test]
    fn mutating_method_receiver_borrow_is_temporary() {
        let source = r#"
            class C {
                mut v: Integer;
                constructor(v: Integer) { this.v = v; }
                function touch(): None { this.v += 1; return None; }
            }
            function main(): None {
                mut c: C = C(1);
                c.touch();
                c.touch();
                return None;
            }
        "#;
        borrow_ok(source);
    }

    #[test]
    fn method_call_with_expression_receiver_does_not_force_owned_args() {
        let source = r#"
            import std.io.*;
            class C {
                function use(borrow s: String): None { println(s); return None; }
            }
            function mk(): C { return C(); }
            function main(): None {
                s: String = "x";
                mk().use(s);
                println(s);
                return None;
            }
        "#;
        borrow_ok(source);
    }

    #[test]
    fn immutable_receiver_cannot_call_mutating_method() {
        let source = r#"
            class C {
                mut v: Integer;
                constructor(v: Integer) { this.v = v; }
                function touch(): None { this.v += 1; return None; }
            }
            function main(): None {
                c: C = C(1);
                c.touch();
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors
            .iter()
            .any(|m| m.contains("Cannot mutably borrow immutable variable 'c'")));
    }

    #[test]
    fn mutating_method_inference_respects_short_circuit_literals() {
        let source = r#"
            class C {
                mut v: Integer;
                constructor(v: Integer) { this.v = v; }
                function touch_flag(): Boolean { this.v += 1; return true; }
                function maybe_touch(): None {
                    if (true || this.touch_flag()) {
                    }
                    return None;
                }
            }
            function main(): None {
                mut c: C = C(1);
                r: &C = &c;
                c.maybe_touch();
                return None;
            }
        "#;
        borrow_ok(source);
    }

    #[test]
    fn async_borrow_capture_blocks_move_after_creation() {
        let source = r#"
            function take_borrow(borrow s: String): None { return None; }
            function consume(owned s: String): None { return None; }
            function main(): None {
                s: String = "x";
                t: Task<None> = async { take_borrow(s); return None; };
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
    fn async_mut_borrow_capture_blocks_assignment_after_creation() {
        let source = r#"
            function main(): None {
                mut x: Integer = 1;
                t: Task<None> = async {
                    r: &mut Integer = &mut x;
                    return None;
                };
                x += 1;
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors.iter().any(|m| {
            m.contains("Cannot assign to 'x' while borrowed")
                || m.contains("Cannot assign to 'x' while mutably borrowed")
        }));
    }

    #[test]
    fn async_mut_borrow_capture_blocks_later_immutable_borrow() {
        let source = r#"
            function main(): None {
                mut x: Integer = 1;
                t: Task<None> = async {
                    r: &mut Integer = &mut x;
                    return None;
                };
                y: &Integer = &x;
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors
            .iter()
            .any(|m| m.contains("Cannot borrow 'x' while mutably borrowed")));
    }

    #[test]
    fn async_mut_borrow_capture_blocks_later_mutable_borrow_with_correct_reason() {
        let source = r#"
            function main(): None {
                mut x: Integer = 1;
                t: Task<None> = async {
                    r: &mut Integer = &mut x;
                    return None;
                };
                y: &mut Integer = &mut x;
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors
            .iter()
            .any(|m| m.contains("Cannot borrow 'x' while mutably borrowed")));
    }

    #[test]
    fn immutable_borrow_blocks_mutating_nested_receiver_call() {
        let source = r#"
            class B {
                mut v: Integer;
                constructor(v: Integer) { this.v = v; }
                function touch(): None { this.v += 1; return None; }
            }
            class A {
                mut b: B;
                constructor(v: Integer) { this.b = B(v); }
            }
            function main(): None {
                mut a: A = A(1);
                r: &A = &a;
                a.b.touch();
                return None;
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors
            .iter()
            .any(|m| m.contains("Cannot mutably borrow 'a' while immutably borrowed")));
    }

    #[test]
    fn short_circuit_or_with_true_literal_does_not_move_rhs() {
        let source = r#"
            function takes(owned s: String): Boolean { return true; }
            function consume(owned s: String): None { return None; }
            function main(): None {
                s: String = "x";
                if (true || takes(s)) {
                }
                consume(s);
                return None;
            }
        "#;
        borrow_ok(source);
    }

    #[test]
    fn short_circuit_and_with_false_literal_does_not_move_rhs() {
        let source = r#"
            function takes(owned s: String): Boolean { return true; }
            function consume(owned s: String): None { return None; }
            function main(): None {
                s: String = "x";
                if (false && takes(s)) {
                }
                consume(s);
                return None;
            }
        "#;
        borrow_ok(source);
    }

    #[test]
    fn constant_if_with_early_return_does_not_move_unreachable_path() {
        let source = r#"
            function consume(owned s: String): None { return None; }
            function main(): None {
                s: String = "x";
                if (true) {
                    consume(s);
                    return None;
                }
                consume(s);
                return None;
            }
        "#;
        borrow_ok(source);
    }

    #[test]
    fn constructor_borrow_params_cannot_be_moved() {
        let source = r#"
            function consume(owned s: String): None { return None; }
            class Boxed {
                constructor(borrow s: String) {
                    consume(s);
                    return None;
                }
            }
        "#;
        let errors = borrow_errors(source);
        assert!(errors
            .iter()
            .any(|m| m.contains("Cannot move 's' while borrowed")));
    }

    #[test]
    fn nested_module_borrow_calls_keep_argument_usable() {
        let source = r#"
            module Outer {
                module Inner {
                    function keep(borrow s: String): None { return None; }
                }
            }
            function consume(owned s: String): None { return None; }
            function main(): None {
                s: String = "x";
                Outer.Inner.keep(s);
                consume(s);
                return None;
            }
        "#;
        borrow_ok(source);
    }
}
