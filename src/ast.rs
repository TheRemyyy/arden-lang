//! Apex AST - Abstract Syntax Tree definitions
//!
//! Production-ready AST with full language support including:
//! - Visibility modifiers (public, private, protected)
//! - Ownership system (owned, borrow, borrow mut)
//! - Generic type bounds
//! - Destructors
//! - Async/await
//! - Full pattern matching

#![allow(dead_code)]
#![allow(clippy::enum_variant_names)]

use std::ops::Range;

/// Source location span
pub type Span = Range<usize>;

/// AST node with span information
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

/// Visibility modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Private,
    Protected,
    Public,
}

/// Parameter passing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParamMode {
    #[default]
    Owned, // Takes ownership (default)
    Borrow,    // Immutable borrow
    BorrowMut, // Mutable borrow
}

/// Generic type parameter with optional bounds
#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<String>, // extends Interface1, Interface2
}

/// Function attribute (e.g., @Test, @Ignore)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attribute {
    Test,
    Ignore(Option<String>), // Optional reason
    Before,
    After,
    BeforeAll,
    AfterAll,
}

/// Complete program
#[derive(Debug, Clone)]
pub struct Program {
    /// Package/namespace for this file (e.g., "utils.math")
    pub package: Option<String>,
    pub declarations: Vec<Spanned<Decl>>,
}

/// Top-level declarations
#[derive(Debug, Clone)]
pub enum Decl {
    Function(FunctionDecl),
    Class(ClassDecl),
    Enum(EnumDecl),
    Interface(InterfaceDecl),
    Module(ModuleDecl),
    Import(ImportDecl),
}

/// Module declaration
#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub name: String,
    pub declarations: Vec<Spanned<Decl>>,
}

/// Import declaration
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: String,
}

/// Function declaration
#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub generic_params: Vec<GenericParam>, // <T extends Comparable>
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub body: Block,
    pub is_async: bool,
    pub visibility: Visibility,
    pub attributes: Vec<Attribute>, // @Test, @Ignore, etc.
}

/// Parameter with ownership mode
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub mode: ParamMode, // owned, borrow, borrow mut
}

/// Class declaration
#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: String,
    pub generic_params: Vec<GenericParam>, // class Box<T>
    pub extends: Option<String>,           // extends BaseClass
    pub implements: Vec<String>,           // implements Interface1, Interface2
    pub fields: Vec<Field>,
    pub constructor: Option<Constructor>,
    pub destructor: Option<Destructor>, // destructor() { ... }
    pub methods: Vec<FunctionDecl>,
    pub visibility: Visibility,
}

/// Class field with visibility
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub visibility: Visibility,
}

/// Constructor
#[derive(Debug, Clone)]
pub struct Constructor {
    pub params: Vec<Parameter>,
    pub body: Block,
}

/// Destructor (RAII cleanup)
#[derive(Debug, Clone)]
pub struct Destructor {
    pub body: Block,
}

/// Enum declaration
#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub generic_params: Vec<GenericParam>, // enum Result<T, E>
    pub variants: Vec<EnumVariant>,
    pub visibility: Visibility,
}

/// Enum variant
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<EnumField>, // Named or anonymous fields
}

/// Enum field (can be named or just type)
#[derive(Debug, Clone)]
pub struct EnumField {
    pub name: Option<String>, // None for positional, Some for named
    pub ty: Type,
}

/// Interface declaration (trait)
#[derive(Debug, Clone)]
pub struct InterfaceDecl {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub extends: Vec<String>, // extends other interfaces
    pub methods: Vec<InterfaceMethod>,
    pub visibility: Visibility,
}

/// Interface method signature with optional default implementation
#[derive(Debug, Clone)]
pub struct InterfaceMethod {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub default_impl: Option<Block>, // Default implementation
}

/// Types
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Primitive types
    Integer,
    Float,
    Boolean,
    String,
    Char,
    None,
    // User-defined types
    Named(String),
    Generic(String, Vec<Type>),
    // Function types
    Function(Vec<Type>, Box<Type>),
    // Built-in generic types
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Set(Box<Type>),
    // Reference types (ownership system)
    Ref(Box<Type>),    // &T - immutable borrow
    MutRef(Box<Type>), // &mut T - mutable borrow
    // Smart pointers
    Box(Box<Type>), // Box<T> - heap allocated
    Rc(Box<Type>),  // Rc<T> - reference counted
    Arc(Box<Type>), // Arc<T> - atomic reference counted
    // Async types
    Task(Box<Type>), // Task<T> - async task
    // Range type
    Range(Box<Type>), // Range<T> - for range(start, end)
}

impl Type {
    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Integer | Type::Float)
    }

    pub fn is_copy(&self) -> bool {
        matches!(
            self,
            Type::Integer | Type::Float | Type::Boolean | Type::Char | Type::None
        )
    }

    pub fn is_reference(&self) -> bool {
        matches!(self, Type::Ref(_) | Type::MutRef(_))
    }
}

/// Block of statements
pub type Block = Vec<Spanned<Stmt>>;

/// Statements
#[derive(Debug, Clone)]
pub enum Stmt {
    /// Variable declaration: name: Type = expr;
    Let {
        name: String,
        ty: Type,
        value: Spanned<Expr>,
        mutable: bool,
    },
    /// Assignment: name = expr;
    Assign {
        target: Spanned<Expr>,
        value: Spanned<Expr>,
    },
    /// Expression statement
    Expr(Spanned<Expr>),
    /// Return statement
    Return(Option<Spanned<Expr>>),
    /// If statement
    If {
        condition: Spanned<Expr>,
        then_block: Block,
        else_block: Option<Block>,
    },
    /// While loop
    While {
        condition: Spanned<Expr>,
        body: Block,
    },
    /// For loop: for (item: Type in collection)
    For {
        var: String,
        var_type: Option<Type>,
        iterable: Spanned<Expr>,
        body: Block,
    },
    /// Break
    Break,
    /// Continue
    Continue,
    /// Match statement
    Match {
        expr: Spanned<Expr>,
        arms: Vec<MatchArm>,
    },
}

/// Match arm
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Block,
}

/// Pattern for matching
#[derive(Debug, Clone)]
pub enum Pattern {
    /// Wildcard _
    Wildcard,
    /// Literal value
    Literal(Literal),
    /// Identifier binding
    Ident(String),
    /// Enum variant: Variant(bindings...)
    Variant(String, Vec<String>),
}

/// Expressions
#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal values
    Literal(Literal),
    /// Identifier
    Ident(String),
    /// Binary operation
    Binary {
        op: BinOp,
        left: Box<Spanned<Expr>>,
        right: Box<Spanned<Expr>>,
    },
    /// Unary operation
    Unary {
        op: UnaryOp,
        expr: Box<Spanned<Expr>>,
    },
    /// Function/method call
    Call {
        callee: Box<Spanned<Expr>>,
        args: Vec<Spanned<Expr>>,
    },
    /// Field access: expr.field
    Field {
        object: Box<Spanned<Expr>>,
        field: String,
    },
    /// Index access: expr[index]
    Index {
        object: Box<Spanned<Expr>>,
        index: Box<Spanned<Expr>>,
    },
    /// Object construction: Type(args)
    Construct {
        ty: String,
        args: Vec<Spanned<Expr>>,
    },
    /// Lambda: (params) => body
    Lambda {
        params: Vec<Parameter>,
        body: Box<Spanned<Expr>>,
    },
    /// this
    This,
    /// Match expression
    Match {
        expr: Box<Spanned<Expr>>,
        arms: Vec<MatchArm>,
    },
    /// String interpolation parts
    StringInterp(Vec<StringPart>),
    /// Try operator: expr? (unwrap or propagate error)
    Try(Box<Spanned<Expr>>),
    /// Borrow expression: &expr
    Borrow(Box<Spanned<Expr>>),
    /// Mutable borrow expression: &mut expr
    MutBorrow(Box<Spanned<Expr>>),
    /// Dereference expression: *expr
    Deref(Box<Spanned<Expr>>),
    /// Await expression: await expr
    Await(Box<Spanned<Expr>>),
    /// Async block: async { ... }
    AsyncBlock(Block),
    /// Require assertion: require(condition)
    Require {
        condition: Box<Spanned<Expr>>,
        message: Option<Box<Spanned<Expr>>>,
    },
    /// Range expression: start..end or start..=end
    Range {
        start: Option<Box<Spanned<Expr>>>,
        end: Option<Box<Spanned<Expr>>>,
        inclusive: bool,
    },
    /// If expression (returns value)
    IfExpr {
        condition: Box<Spanned<Expr>>,
        then_branch: Block,
        else_branch: Option<Block>,
    },
    /// Block expression
    Block(Block),
}

/// Parts of interpolated string
#[derive(Debug, Clone)]
pub enum StringPart {
    Literal(String),
    Expr(Spanned<Expr>),
}

/// Literal values
#[derive(Debug, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Char(char),
    None,
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

impl BinOp {
    pub fn precedence(&self) -> u8 {
        match self {
            BinOp::Or => 1,
            BinOp::And => 2,
            BinOp::Eq | BinOp::NotEq => 3,
            BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => 4,
            BinOp::Add | BinOp::Sub => 5,
            BinOp::Mul | BinOp::Div | BinOp::Mod => 6,
        }
    }
}

/// Unary operators
#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg,
    Not,
}
