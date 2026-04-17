//! Arden Code Generator - LLVM IR generation

mod async_codegen;
mod call_codegen;
mod expr_codegen;
mod loop_codegen;
mod stdlib_calls;
mod string_display_codegen;
mod utf8_runtime;

use crate::cache::elapsed_nanos_u64;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};

use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, FunctionValue, GlobalValue, InstructionValue, IntValue,
    MetadataValue, PointerValue, ValueKind,
};
use inkwell::{AddressSpace, AtomicOrdering, FloatPredicate, IntPredicate};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use crate::ast::*;
use crate::parser::parse_type_source;
use crate::shared::type_name::split_generic_args_static;
use crate::stdlib::stdlib_registry;
use crate::typeck::{NumericConst, TypeChecker};

/// Codegen error
#[derive(Debug)]
pub struct CodegenError {
    pub message: String,
}

impl CodegenError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, CodegenError>;

#[derive(Debug, Clone, Default)]
pub struct CodegenPhaseTimingSnapshot {
    pub program_has_generic_classes_ns: u64,
    pub specialize_generic_classes_initial_ns: u64,
    pub program_has_explicit_generic_calls_ns: u64,
    pub specialize_explicit_generic_calls_ns: u64,
    pub specialize_generic_classes_final_ns: u64,
    pub collect_generated_spec_symbols_ns: u64,
    pub specialized_active_symbols_ns: u64,
    pub import_alias_collection_ns: u64,
    pub enum_declare_pass_ns: u64,
    pub enum_declare_decl_filter_ns: u64,
    pub enum_declare_work_ns: u64,
    pub decl_pass_ns: u64,
    pub decl_pass_decl_filter_ns: u64,
    pub decl_pass_class_work_ns: u64,
    pub decl_pass_function_work_ns: u64,
    pub decl_pass_module_work_ns: u64,
    pub body_pass_ns: u64,
    pub body_pass_decl_filter_ns: u64,
    pub body_pass_function_work_ns: u64,
    pub body_function_setup_ns: u64,
    pub body_function_param_alloc_ns: u64,
    pub body_function_stmt_loop_ns: u64,
    pub body_stmt_let_ns: u64,
    pub body_stmt_assign_ns: u64,
    pub body_stmt_expr_ns: u64,
    pub body_stmt_return_ns: u64,
    pub body_function_implicit_return_ns: u64,
    pub expr_literal_ns: u64,
    pub expr_ident_ns: u64,
    pub expr_binary_ns: u64,
    pub expr_call_ns: u64,
    pub body_pass_class_work_ns: u64,
    pub body_pass_module_work_ns: u64,
    pub total_decls_count: usize,
    pub import_alias_count: usize,
    pub active_symbols_count: usize,
    pub declaration_symbols_count: usize,
    pub generated_spec_owners_count: usize,
    pub declared_enum_count: usize,
    pub declared_class_count: usize,
    pub declared_function_count: usize,
    pub declared_module_count: usize,
    pub compiled_function_count: usize,
    pub compiled_class_count: usize,
    pub compiled_module_count: usize,
}

struct CodegenPhaseTimingTotals {
    program_has_generic_classes_ns: AtomicU64,
    specialize_generic_classes_initial_ns: AtomicU64,
    program_has_explicit_generic_calls_ns: AtomicU64,
    specialize_explicit_generic_calls_ns: AtomicU64,
    specialize_generic_classes_final_ns: AtomicU64,
    collect_generated_spec_symbols_ns: AtomicU64,
    specialized_active_symbols_ns: AtomicU64,
    import_alias_collection_ns: AtomicU64,
    enum_declare_pass_ns: AtomicU64,
    enum_declare_decl_filter_ns: AtomicU64,
    enum_declare_work_ns: AtomicU64,
    decl_pass_ns: AtomicU64,
    decl_pass_decl_filter_ns: AtomicU64,
    decl_pass_class_work_ns: AtomicU64,
    decl_pass_function_work_ns: AtomicU64,
    decl_pass_module_work_ns: AtomicU64,
    body_pass_ns: AtomicU64,
    body_pass_decl_filter_ns: AtomicU64,
    body_pass_function_work_ns: AtomicU64,
    body_function_setup_ns: AtomicU64,
    body_function_param_alloc_ns: AtomicU64,
    body_function_stmt_loop_ns: AtomicU64,
    body_stmt_let_ns: AtomicU64,
    body_stmt_assign_ns: AtomicU64,
    body_stmt_expr_ns: AtomicU64,
    body_stmt_return_ns: AtomicU64,
    body_function_implicit_return_ns: AtomicU64,
    expr_literal_ns: AtomicU64,
    expr_ident_ns: AtomicU64,
    expr_binary_ns: AtomicU64,
    expr_call_ns: AtomicU64,
    body_pass_class_work_ns: AtomicU64,
    body_pass_module_work_ns: AtomicU64,
    total_decls_count: AtomicUsize,
    import_alias_count: AtomicUsize,
    active_symbols_count: AtomicUsize,
    declaration_symbols_count: AtomicUsize,
    generated_spec_owners_count: AtomicUsize,
    declared_enum_count: AtomicUsize,
    declared_class_count: AtomicUsize,
    declared_function_count: AtomicUsize,
    declared_module_count: AtomicUsize,
    compiled_function_count: AtomicUsize,
    compiled_class_count: AtomicUsize,
    compiled_module_count: AtomicUsize,
}

static CODEGEN_PHASE_TIMING_TOTALS: CodegenPhaseTimingTotals = CodegenPhaseTimingTotals {
    program_has_generic_classes_ns: AtomicU64::new(0),
    specialize_generic_classes_initial_ns: AtomicU64::new(0),
    program_has_explicit_generic_calls_ns: AtomicU64::new(0),
    specialize_explicit_generic_calls_ns: AtomicU64::new(0),
    specialize_generic_classes_final_ns: AtomicU64::new(0),
    collect_generated_spec_symbols_ns: AtomicU64::new(0),
    specialized_active_symbols_ns: AtomicU64::new(0),
    import_alias_collection_ns: AtomicU64::new(0),
    enum_declare_pass_ns: AtomicU64::new(0),
    enum_declare_decl_filter_ns: AtomicU64::new(0),
    enum_declare_work_ns: AtomicU64::new(0),
    decl_pass_ns: AtomicU64::new(0),
    decl_pass_decl_filter_ns: AtomicU64::new(0),
    decl_pass_class_work_ns: AtomicU64::new(0),
    decl_pass_function_work_ns: AtomicU64::new(0),
    decl_pass_module_work_ns: AtomicU64::new(0),
    body_pass_ns: AtomicU64::new(0),
    body_pass_decl_filter_ns: AtomicU64::new(0),
    body_pass_function_work_ns: AtomicU64::new(0),
    body_function_setup_ns: AtomicU64::new(0),
    body_function_param_alloc_ns: AtomicU64::new(0),
    body_function_stmt_loop_ns: AtomicU64::new(0),
    body_stmt_let_ns: AtomicU64::new(0),
    body_stmt_assign_ns: AtomicU64::new(0),
    body_stmt_expr_ns: AtomicU64::new(0),
    body_stmt_return_ns: AtomicU64::new(0),
    body_function_implicit_return_ns: AtomicU64::new(0),
    expr_literal_ns: AtomicU64::new(0),
    expr_ident_ns: AtomicU64::new(0),
    expr_binary_ns: AtomicU64::new(0),
    expr_call_ns: AtomicU64::new(0),
    body_pass_class_work_ns: AtomicU64::new(0),
    body_pass_module_work_ns: AtomicU64::new(0),
    total_decls_count: AtomicUsize::new(0),
    import_alias_count: AtomicUsize::new(0),
    active_symbols_count: AtomicUsize::new(0),
    declaration_symbols_count: AtomicUsize::new(0),
    generated_spec_owners_count: AtomicUsize::new(0),
    declared_enum_count: AtomicUsize::new(0),
    declared_class_count: AtomicUsize::new(0),
    declared_function_count: AtomicUsize::new(0),
    declared_module_count: AtomicUsize::new(0),
    compiled_function_count: AtomicUsize::new(0),
    compiled_class_count: AtomicUsize::new(0),
    compiled_module_count: AtomicUsize::new(0),
};

pub fn reset_codegen_phase_timings() {
    CODEGEN_PHASE_TIMING_TOTALS
        .program_has_generic_classes_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .specialize_generic_classes_initial_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .program_has_explicit_generic_calls_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .specialize_explicit_generic_calls_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .specialize_generic_classes_final_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .collect_generated_spec_symbols_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .specialized_active_symbols_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .import_alias_collection_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .enum_declare_pass_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .enum_declare_decl_filter_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .enum_declare_work_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .decl_pass_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .decl_pass_decl_filter_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .decl_pass_class_work_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .decl_pass_function_work_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .decl_pass_module_work_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .body_pass_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .body_pass_decl_filter_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .body_pass_function_work_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .body_function_setup_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .body_function_param_alloc_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .body_function_stmt_loop_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .body_stmt_let_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .body_stmt_assign_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .body_stmt_expr_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .body_stmt_return_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .body_function_implicit_return_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .expr_literal_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .expr_ident_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .expr_binary_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .expr_call_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .body_pass_class_work_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .body_pass_module_work_ns
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .total_decls_count
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .import_alias_count
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .active_symbols_count
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .declaration_symbols_count
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .generated_spec_owners_count
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .declared_enum_count
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .declared_class_count
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .declared_function_count
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .declared_module_count
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .compiled_function_count
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .compiled_class_count
        .store(0, Ordering::Relaxed);
    CODEGEN_PHASE_TIMING_TOTALS
        .compiled_module_count
        .store(0, Ordering::Relaxed);
}

pub fn snapshot_codegen_phase_timings() -> CodegenPhaseTimingSnapshot {
    CodegenPhaseTimingSnapshot {
        program_has_generic_classes_ns: CODEGEN_PHASE_TIMING_TOTALS
            .program_has_generic_classes_ns
            .load(Ordering::Relaxed),
        specialize_generic_classes_initial_ns: CODEGEN_PHASE_TIMING_TOTALS
            .specialize_generic_classes_initial_ns
            .load(Ordering::Relaxed),
        program_has_explicit_generic_calls_ns: CODEGEN_PHASE_TIMING_TOTALS
            .program_has_explicit_generic_calls_ns
            .load(Ordering::Relaxed),
        specialize_explicit_generic_calls_ns: CODEGEN_PHASE_TIMING_TOTALS
            .specialize_explicit_generic_calls_ns
            .load(Ordering::Relaxed),
        specialize_generic_classes_final_ns: CODEGEN_PHASE_TIMING_TOTALS
            .specialize_generic_classes_final_ns
            .load(Ordering::Relaxed),
        collect_generated_spec_symbols_ns: CODEGEN_PHASE_TIMING_TOTALS
            .collect_generated_spec_symbols_ns
            .load(Ordering::Relaxed),
        specialized_active_symbols_ns: CODEGEN_PHASE_TIMING_TOTALS
            .specialized_active_symbols_ns
            .load(Ordering::Relaxed),
        import_alias_collection_ns: CODEGEN_PHASE_TIMING_TOTALS
            .import_alias_collection_ns
            .load(Ordering::Relaxed),
        enum_declare_pass_ns: CODEGEN_PHASE_TIMING_TOTALS
            .enum_declare_pass_ns
            .load(Ordering::Relaxed),
        enum_declare_decl_filter_ns: CODEGEN_PHASE_TIMING_TOTALS
            .enum_declare_decl_filter_ns
            .load(Ordering::Relaxed),
        enum_declare_work_ns: CODEGEN_PHASE_TIMING_TOTALS
            .enum_declare_work_ns
            .load(Ordering::Relaxed),
        decl_pass_ns: CODEGEN_PHASE_TIMING_TOTALS
            .decl_pass_ns
            .load(Ordering::Relaxed),
        decl_pass_decl_filter_ns: CODEGEN_PHASE_TIMING_TOTALS
            .decl_pass_decl_filter_ns
            .load(Ordering::Relaxed),
        decl_pass_class_work_ns: CODEGEN_PHASE_TIMING_TOTALS
            .decl_pass_class_work_ns
            .load(Ordering::Relaxed),
        decl_pass_function_work_ns: CODEGEN_PHASE_TIMING_TOTALS
            .decl_pass_function_work_ns
            .load(Ordering::Relaxed),
        decl_pass_module_work_ns: CODEGEN_PHASE_TIMING_TOTALS
            .decl_pass_module_work_ns
            .load(Ordering::Relaxed),
        body_pass_ns: CODEGEN_PHASE_TIMING_TOTALS
            .body_pass_ns
            .load(Ordering::Relaxed),
        body_pass_decl_filter_ns: CODEGEN_PHASE_TIMING_TOTALS
            .body_pass_decl_filter_ns
            .load(Ordering::Relaxed),
        body_pass_function_work_ns: CODEGEN_PHASE_TIMING_TOTALS
            .body_pass_function_work_ns
            .load(Ordering::Relaxed),
        body_function_setup_ns: CODEGEN_PHASE_TIMING_TOTALS
            .body_function_setup_ns
            .load(Ordering::Relaxed),
        body_function_param_alloc_ns: CODEGEN_PHASE_TIMING_TOTALS
            .body_function_param_alloc_ns
            .load(Ordering::Relaxed),
        body_function_stmt_loop_ns: CODEGEN_PHASE_TIMING_TOTALS
            .body_function_stmt_loop_ns
            .load(Ordering::Relaxed),
        body_stmt_let_ns: CODEGEN_PHASE_TIMING_TOTALS
            .body_stmt_let_ns
            .load(Ordering::Relaxed),
        body_stmt_assign_ns: CODEGEN_PHASE_TIMING_TOTALS
            .body_stmt_assign_ns
            .load(Ordering::Relaxed),
        body_stmt_expr_ns: CODEGEN_PHASE_TIMING_TOTALS
            .body_stmt_expr_ns
            .load(Ordering::Relaxed),
        body_stmt_return_ns: CODEGEN_PHASE_TIMING_TOTALS
            .body_stmt_return_ns
            .load(Ordering::Relaxed),
        body_function_implicit_return_ns: CODEGEN_PHASE_TIMING_TOTALS
            .body_function_implicit_return_ns
            .load(Ordering::Relaxed),
        expr_literal_ns: CODEGEN_PHASE_TIMING_TOTALS
            .expr_literal_ns
            .load(Ordering::Relaxed),
        expr_ident_ns: CODEGEN_PHASE_TIMING_TOTALS
            .expr_ident_ns
            .load(Ordering::Relaxed),
        expr_binary_ns: CODEGEN_PHASE_TIMING_TOTALS
            .expr_binary_ns
            .load(Ordering::Relaxed),
        expr_call_ns: CODEGEN_PHASE_TIMING_TOTALS
            .expr_call_ns
            .load(Ordering::Relaxed),
        body_pass_class_work_ns: CODEGEN_PHASE_TIMING_TOTALS
            .body_pass_class_work_ns
            .load(Ordering::Relaxed),
        body_pass_module_work_ns: CODEGEN_PHASE_TIMING_TOTALS
            .body_pass_module_work_ns
            .load(Ordering::Relaxed),
        total_decls_count: CODEGEN_PHASE_TIMING_TOTALS
            .total_decls_count
            .load(Ordering::Relaxed),
        import_alias_count: CODEGEN_PHASE_TIMING_TOTALS
            .import_alias_count
            .load(Ordering::Relaxed),
        active_symbols_count: CODEGEN_PHASE_TIMING_TOTALS
            .active_symbols_count
            .load(Ordering::Relaxed),
        declaration_symbols_count: CODEGEN_PHASE_TIMING_TOTALS
            .declaration_symbols_count
            .load(Ordering::Relaxed),
        generated_spec_owners_count: CODEGEN_PHASE_TIMING_TOTALS
            .generated_spec_owners_count
            .load(Ordering::Relaxed),
        declared_enum_count: CODEGEN_PHASE_TIMING_TOTALS
            .declared_enum_count
            .load(Ordering::Relaxed),
        declared_class_count: CODEGEN_PHASE_TIMING_TOTALS
            .declared_class_count
            .load(Ordering::Relaxed),
        declared_function_count: CODEGEN_PHASE_TIMING_TOTALS
            .declared_function_count
            .load(Ordering::Relaxed),
        declared_module_count: CODEGEN_PHASE_TIMING_TOTALS
            .declared_module_count
            .load(Ordering::Relaxed),
        compiled_function_count: CODEGEN_PHASE_TIMING_TOTALS
            .compiled_function_count
            .load(Ordering::Relaxed),
        compiled_class_count: CODEGEN_PHASE_TIMING_TOTALS
            .compiled_class_count
            .load(Ordering::Relaxed),
        compiled_module_count: CODEGEN_PHASE_TIMING_TOTALS
            .compiled_module_count
            .load(Ordering::Relaxed),
    }
}

/// Variable in codegen
#[derive(Debug, Clone)]
pub struct Variable<'ctx> {
    pub ptr: PointerValue<'ctx>,
    pub ty: Type,
    pub mutable: bool,
}

/// Class info
pub struct ClassInfo<'ctx> {
    pub struct_type: StructType<'ctx>,
    pub field_indices: HashMap<String, u32>,
    pub field_types: HashMap<String, Type>,
    pub generic_params: Vec<String>,
    pub extends: Option<String>,
}

/// Enum variant metadata
#[derive(Debug, Clone)]
pub struct EnumVariantInfo {
    pub tag: u8,
    pub fields: Vec<Type>,
}

/// Enum metadata used by codegen and pattern matching
pub struct EnumInfo<'ctx> {
    pub struct_type: StructType<'ctx>,
    pub payload_slots: usize,
    pub variants: HashMap<String, EnumVariantInfo>,
}

/// Loop context for break/continue
pub struct LoopContext<'ctx> {
    pub loop_block: BasicBlock<'ctx>,
    pub after_block: BasicBlock<'ctx>,
}

struct AsyncFunctionPlan<'ctx> {
    wrapper: FunctionValue<'ctx>,
    body: FunctionValue<'ctx>,
    thunk: FunctionValue<'ctx>,
    env_type: StructType<'ctx>,
    inner_return_type: Type,
}

#[derive(Clone)]
struct GenericTemplate {
    func: FunctionDecl,
    span: Span,
    owner_class: Option<String>,
}

#[derive(Clone)]
struct GenericClassTemplate {
    class: ClassDecl,
    span: Span,
}

struct GenericRewriteTemplates<'a> {
    function_templates: &'a HashMap<String, GenericTemplate>,
    method_templates: &'a HashMap<String, Vec<GenericTemplate>>,
    class_templates: &'a HashMap<String, GenericClassTemplate>,
    import_aliases: &'a HashMap<String, String>,
}

struct GenericRewriteOutputs<'a> {
    emitted: &'a mut HashSet<String>,
    generated_functions: &'a mut Vec<Spanned<Decl>>,
    generated_methods: &'a mut HashMap<String, Vec<FunctionDecl>>,
}

type CountedPushLoopInfo = (String, i64, HashSet<String>, HashMap<String, i64>);

#[derive(Clone, Copy, Default)]
struct BinaryCodegenOptions {
    skip_nonzero_divisor_guard: bool,
    skip_signed_division_overflow_guard: bool,
}

/// Code generator
pub struct Codegen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub variables: HashMap<String, Variable<'ctx>>,
    pub(crate) non_negative_locals: HashSet<String>,
    pub(crate) non_zero_locals: HashSet<String>,
    pub(crate) exact_integer_locals: HashMap<String, i64>,
    pub(crate) upper_bound_locals: HashMap<String, i64>,
    pub(crate) exact_list_lengths: HashMap<String, i64>,
    pub(crate) exact_list_capacities: HashMap<String, i64>,
    pub(crate) list_element_upper_bounds: HashMap<String, i64>,
    pub(crate) distinct_list_alloc_ids: HashMap<String, u64>,
    next_distinct_list_alloc_id: u64,
    pub functions: HashMap<String, (FunctionValue<'ctx>, Type)>,
    pub(crate) non_negative_functions: HashSet<String>,
    pub function_param_modes: HashMap<String, Vec<ParamMode>>,
    pub classes: HashMap<String, ClassInfo<'ctx>>,
    pub enums: HashMap<String, EnumInfo<'ctx>>,
    pub interfaces: HashMap<String, HashSet<String>>,
    pub interface_implementors: HashMap<String, HashSet<String>>,
    pub enum_variant_to_enum: HashMap<String, String>,
    pub current_function: Option<FunctionValue<'ctx>>,
    pub current_return_type: Option<Type>,
    pub loop_stack: Vec<LoopContext<'ctx>>,
    pub str_counter: u32,
    pub lambda_counter: u32,
    async_counter: u32,
    async_functions: HashMap<String, AsyncFunctionPlan<'ctx>>,
    extern_functions: HashSet<String>,
    import_aliases: HashMap<String, Vec<(Option<String>, String)>>,
    current_package: String,
    current_module_prefix: Option<String>,
    pub(crate) current_generic_bounds: HashMap<String, Vec<String>>,
}

impl<'ctx> Codegen<'ctx> {
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

    fn format_diagnostic_name(name: &str) -> String {
        name.replace("__", ".")
    }

    pub(crate) fn format_diagnostic_type(ty: &Type) -> String {
        match ty {
            Type::Named(name) => Self::format_specialized_class_diagnostic_name(name)
                .unwrap_or_else(|| Self::format_diagnostic_name(name)),
            Type::Generic(name, args) => format!(
                "{}<{}>",
                Self::format_specialized_class_diagnostic_name(name)
                    .unwrap_or_else(|| Self::format_diagnostic_name(name)),
                args.iter()
                    .map(Self::format_diagnostic_type)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Type::Function(params, ret) => format!(
                "({}) -> {}",
                params
                    .iter()
                    .map(Self::format_diagnostic_type)
                    .collect::<Vec<_>>()
                    .join(", "),
                Self::format_diagnostic_type(ret)
            ),
            Type::Option(inner) => format!("Option<{}>", Self::format_diagnostic_type(inner)),
            Type::Result(ok, err) => format!(
                "Result<{}, {}>",
                Self::format_diagnostic_type(ok),
                Self::format_diagnostic_type(err)
            ),
            Type::List(inner) => format!("List<{}>", Self::format_diagnostic_type(inner)),
            Type::Map(key, value) => format!(
                "Map<{}, {}>",
                Self::format_diagnostic_type(key),
                Self::format_diagnostic_type(value)
            ),
            Type::Set(inner) => format!("Set<{}>", Self::format_diagnostic_type(inner)),
            Type::Ref(inner) => format!("&{}", Self::format_diagnostic_type(inner)),
            Type::MutRef(inner) => format!("&mut {}", Self::format_diagnostic_type(inner)),
            Type::Box(inner) => format!("Box<{}>", Self::format_diagnostic_type(inner)),
            Type::Rc(inner) => format!("Rc<{}>", Self::format_diagnostic_type(inner)),
            Type::Arc(inner) => format!("Arc<{}>", Self::format_diagnostic_type(inner)),
            Type::Ptr(inner) => format!("*{}", Self::format_diagnostic_type(inner)),
            Type::Task(inner) => format!("Task<{}>", Self::format_diagnostic_type(inner)),
            Type::Range(inner) => format!("Range<{}>", Self::format_diagnostic_type(inner)),
            _ => Self::format_diagnostic_name(&Self::format_type_string(ty)),
        }
    }

    fn format_specialized_class_diagnostic_name(name: &str) -> Option<String> {
        let (base_name, suffixes) = name.split_once("__spec__")?;
        let args = if let Some(decoded) = Self::decode_specialization_suffix_token(suffixes) {
            vec![decoded]
        } else {
            Self::decode_generic_specialization_args(suffixes)
                .into_iter()
                .find_map(|(args, consumed)| (consumed == suffixes.len()).then_some(args))?
        };
        Some(format!(
            "{}<{}>",
            Self::format_diagnostic_name(base_name),
            args.join(", ")
        ))
    }

    fn decode_specialization_suffix_token(token: &str) -> Option<String> {
        if let Some((base_name, arg_slice, _)) = Self::decode_length_prefixed_generic_name(token) {
            for (args, args_consumed) in Self::decode_generic_specialization_args(arg_slice) {
                if args_consumed == arg_slice.len() {
                    return Some(format!(
                        "{}<{}>",
                        Self::format_diagnostic_name(base_name),
                        args.join(", ")
                    ));
                }
            }
        }

        if let Some(rest) = token.strip_prefix('G') {
            let mut best_match: Option<(String, usize, bool, usize)> = None;
            for base_end in 1..rest.len() {
                let base_name = &rest[..base_end];
                let arg_slice = &rest[base_end..];
                for (args, args_consumed) in Self::decode_generic_specialization_args(arg_slice) {
                    if args_consumed == arg_slice.len() {
                        let decoded = format!(
                            "{}<{}>",
                            Self::format_diagnostic_name(base_name),
                            args.join(", ")
                        );
                        let candidate = (
                            decoded,
                            args.len(),
                            base_name.ends_with('_') || base_name.ends_with('.'),
                            base_name.len(),
                        );
                        if best_match.as_ref().is_none_or(
                            |(_, best_arg_count, best_trailing_sep, best_base_len)| {
                                candidate.1 > *best_arg_count
                                    || (candidate.1 == *best_arg_count
                                        && candidate.2 != *best_trailing_sep
                                        && !candidate.2)
                                    || (candidate.1 == *best_arg_count
                                        && candidate.2 == *best_trailing_sep
                                        && candidate.3 < *best_base_len)
                            },
                        ) {
                            best_match = Some(candidate);
                        }
                    }
                }
            }
            if let Some((decoded, _, _, _)) = best_match {
                return Some(decoded);
            }
        }
        Self::decode_specialization_suffix_token_prefix(token)
            .into_iter()
            .find_map(|(decoded, consumed)| (consumed == token.len()).then_some(decoded))
    }

    fn decode_specialization_suffix_token_prefix(token: &str) -> Vec<(String, usize)> {
        let mut results = Vec::new();

        if let Some((name, consumed)) = Self::decode_length_prefixed_named_name(token) {
            results.push((Self::format_diagnostic_name(name), consumed));
        }

        if let Some((base_name, arg_slice, prefix_consumed)) =
            Self::decode_length_prefixed_generic_name(token)
        {
            for (args, args_consumed) in Self::decode_generic_specialization_args(arg_slice) {
                results.push((
                    format!(
                        "{}<{}>",
                        Self::format_diagnostic_name(base_name),
                        args.join(", ")
                    ),
                    prefix_consumed + args_consumed,
                ));
            }
        }

        match token {
            _ if token.starts_with("I64") => results.push(("Integer".to_string(), 3)),
            _ if token.starts_with("F64") => results.push(("Float".to_string(), 3)),
            _ if token.starts_with("Bool") => results.push(("Boolean".to_string(), 4)),
            _ if token.starts_with("Str") => results.push(("String".to_string(), 3)),
            _ if token.starts_with("Char") => results.push(("Char".to_string(), 4)),
            _ if token.starts_with("None") => results.push(("None".to_string(), 4)),
            _ => {}
        }

        for (prefix, display_name) in [
            ("MutRef", "&mut "),
            ("Ref", "&"),
            ("Option", "Option<"),
            ("List", "List<"),
            ("Set", "Set<"),
            ("Box", "Box<"),
            ("Rc", "Rc<"),
            ("Arc", "Arc<"),
            ("Ptr", "Ptr<"),
            ("Task", "Task<"),
            ("Range", "Range<"),
            ("Opt", "Option<"),
        ] {
            if let Some(rest) = token.strip_prefix(prefix) {
                for (decoded_inner, consumed_inner) in
                    Self::decode_specialization_suffix_token_prefix(rest)
                {
                    let decoded = match prefix {
                        "Ref" | "MutRef" => format!("{display_name}{decoded_inner}"),
                        _ => format!("{display_name}{decoded_inner}>"),
                    };
                    results.push((decoded, prefix.len() + consumed_inner));
                }
            }
        }

        for prefix in ["Result", "Res", "Map"] {
            if let Some(rest) = token.strip_prefix(prefix) {
                for (left, left_consumed) in Self::decode_specialization_suffix_token_prefix(rest) {
                    let Some(right_rest) = rest[left_consumed..].strip_prefix('_') else {
                        continue;
                    };
                    for (right, right_consumed) in
                        Self::decode_specialization_suffix_token_prefix(right_rest)
                    {
                        let display_name = if prefix == "Map" { "Map" } else { "Result" };
                        results.push((
                            format!("{display_name}<{left}, {right}>"),
                            prefix.len() + left_consumed + 1 + right_consumed,
                        ));
                    }
                }
            }
        }

        if let Some(rest) = token.strip_prefix("Fn") {
            for (params, params_consumed) in Self::decode_function_specialization_params(rest) {
                let Some(ret_rest) = rest[params_consumed..].strip_prefix("To") else {
                    continue;
                };
                for (ret, ret_consumed) in Self::decode_specialization_suffix_token_prefix(ret_rest)
                {
                    results.push((
                        format!("({}) -> {}", params.join(", "), ret),
                        2 + params_consumed + 2 + ret_consumed,
                    ));
                }
            }
        }

        if let Some(rest) = token.strip_prefix('G') {
            let mut generic_results: Vec<(String, usize, bool, usize)> = Vec::new();
            for base_end in 1..rest.len() {
                let base_name = &rest[..base_end];
                let arg_slice = &rest[base_end..];
                for (args, args_consumed) in Self::decode_generic_specialization_args(arg_slice) {
                    generic_results.push((
                        format!(
                            "{}<{}>",
                            Self::format_diagnostic_name(base_name),
                            args.join(", ")
                        ),
                        1 + base_end + args_consumed,
                        base_name.ends_with('_') || base_name.ends_with('.'),
                        base_name.len(),
                    ));
                }
            }
            generic_results.sort_by(|a, b| {
                a.2.cmp(&b.2)
                    .then_with(|| a.3.cmp(&b.3))
                    .then_with(|| b.1.cmp(&a.1))
            });
            results.extend(
                generic_results
                    .into_iter()
                    .map(|(decoded, consumed, _, _)| (decoded, consumed)),
            );
        }

        if let Some(name) = token.strip_prefix('N') {
            results.push((Self::format_diagnostic_name(name), token.len()));
        }

        results
    }

    fn decode_length_prefixed_named_name(token: &str) -> Option<(&str, usize)> {
        let rest = token.strip_prefix('N')?;
        let (len_str, remainder) = rest.split_once('_')?;
        let name_len: usize = len_str.parse().ok()?;
        if remainder.len() < name_len {
            return None;
        }
        let (name, _) = remainder.split_at(name_len);
        Some((name, 1 + len_str.len() + 1 + name_len))
    }

    fn decode_length_prefixed_generic_name(token: &str) -> Option<(&str, &str, usize)> {
        let rest = token.strip_prefix('G')?;
        let (len_str, remainder) = rest.split_once('_')?;
        let name_len: usize = len_str.parse().ok()?;
        if remainder.len() < name_len {
            return None;
        }
        let (name, args) = remainder.split_at(name_len);
        Some((name, args, 1 + len_str.len() + 1 + name_len))
    }

    fn decode_generic_specialization_args(token: &str) -> Vec<(Vec<String>, usize)> {
        let mut results = Vec::new();

        for (first, first_consumed) in Self::decode_specialization_suffix_token_prefix(token) {
            results.push((vec![first.clone()], first_consumed));

            let Some(rest) = token[first_consumed..].strip_prefix('_') else {
                continue;
            };
            for (remaining, remaining_consumed) in Self::decode_generic_specialization_args(rest) {
                let mut args = vec![first.clone()];
                args.extend(remaining);
                results.push((args, first_consumed + 1 + remaining_consumed));
            }
        }

        results
    }

    fn decode_function_specialization_params(token: &str) -> Vec<(Vec<String>, usize)> {
        if let Some(rest) = token.strip_prefix("To") {
            return vec![(Vec::new(), token.len() - rest.len())];
        }

        let mut results = Vec::new();
        for (first, first_consumed) in Self::decode_specialization_suffix_token_prefix(token) {
            results.push((vec![first.clone()], first_consumed));

            let Some(rest) = token[first_consumed..].strip_prefix('_') else {
                continue;
            };
            for (remaining, remaining_consumed) in Self::decode_function_specialization_params(rest)
            {
                let mut params = vec![first.clone()];
                params.extend(remaining);
                results.push((params, first_consumed + 1 + remaining_consumed));
            }
        }

        results
    }

    pub(crate) fn undefined_variable_error(name: &str) -> CodegenError {
        CodegenError::new(format!("Undefined variable: {}", name))
    }

    pub(crate) fn undefined_function_error(name: &str) -> CodegenError {
        CodegenError::new(format!("Undefined function: {}", name))
    }

    pub(crate) fn member_root_undefined_variable(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Ident(name) => {
                if self.variables.contains_key(name) {
                    return None;
                }
                if self.classes.contains_key(name) {
                    return Some(name.clone());
                }
                if self.resolve_contextual_function_value_name(expr).is_none() {
                    return Some(name.clone());
                }
                None
            }
            Expr::Field { object, .. } | Expr::Index { object, .. } => {
                self.member_root_undefined_variable(&object.node)
            }
            Expr::GenericFunctionValue { callee, .. } => {
                self.member_root_undefined_variable(&callee.node)
            }
            _ => None,
        }
    }

    fn non_function_call_error(ty: &Type) -> CodegenError {
        CodegenError::new(format!(
            "Cannot call non-function type {}",
            Self::format_diagnostic_type(ty)
        ))
    }

    fn function_call_arity_error(function_ty: &Type, got: usize) -> CodegenError {
        let expected = match function_ty {
            Type::Function(params, _) => params.len(),
            _ => 0,
        };
        CodegenError::new(format!(
            "Function value {} expects {} argument(s), got {}",
            Self::format_diagnostic_type(function_ty),
            expected,
            got
        ))
    }

    fn constructor_call_arity_error(ty: &Type, expected: usize, got: usize) -> CodegenError {
        CodegenError::new(format!(
            "Constructor {} expects {} argument(s), got {}",
            Self::format_diagnostic_type(ty),
            expected,
            got
        ))
    }

    fn method_call_arity_error(
        receiver_ty: &Type,
        method: &str,
        expected: usize,
        got: usize,
    ) -> CodegenError {
        CodegenError::new(format!(
            "{}.{}() expects {} argument(s), got {}",
            Self::format_diagnostic_type(receiver_ty),
            method,
            expected,
            got
        ))
    }

    pub(crate) fn call_expr_arity_error(&self, expr: &Expr) -> Option<CodegenError> {
        let Expr::Call { callee, args, .. } = expr else {
            return None;
        };
        let callee_ty = self.infer_expr_type(&callee.node, &[]);
        let Type::Function(_, _) = callee_ty else {
            return None;
        };
        let expected = match &callee_ty {
            Type::Function(params, _) => params.len(),
            _ => return None,
        };
        (args.len() != expected).then(|| Self::function_call_arity_error(&callee_ty, args.len()))
    }

    pub(crate) fn unknown_field_error(field: &str, ty: &Type) -> CodegenError {
        CodegenError::new(format!(
            "Unknown field '{}' on class '{}'",
            field,
            Self::format_diagnostic_type(ty)
        ))
    }

    fn unknown_method_error(method: &str, ty: &Type) -> CodegenError {
        CodegenError::new(format!(
            "Unknown method '{}' for class '{}'",
            method,
            Self::format_diagnostic_type(ty)
        ))
    }

    fn unknown_interface_method_error(method: &str, interface_name: &str) -> CodegenError {
        CodegenError::new(format!(
            "Unknown method '{}' for interface '{}'",
            method,
            Self::format_diagnostic_name(interface_name)
        ))
    }

    fn explicit_generic_field_access_error(
        &self,
        callee: &Expr,
        treat_as_method: bool,
    ) -> Option<CodegenError> {
        let Expr::Field { object, field } = callee else {
            return None;
        };
        let object_ty = self.infer_object_type(&object.node);
        let display_ty = self.deref_codegen_type(&object_ty.clone()?).clone();
        let class_name = self.type_to_class_name(&display_ty)?;
        let class_info = self.classes.get(&class_name)?;
        if !class_info.field_types.contains_key(field) {
            return None;
        }

        Some(if treat_as_method {
            Self::unknown_method_error(field, &display_ty)
        } else {
            Self::unknown_field_error(field, &display_ty)
        })
    }

    fn function_value_signature_mismatch_error(
        actual_ty: &Type,
        expected_ty: &Type,
    ) -> CodegenError {
        CodegenError::new(format!(
            "Cannot use function value {} as {}",
            Self::format_diagnostic_type(actual_ty),
            Self::format_diagnostic_type(expected_ty)
        ))
    }

    pub(crate) fn type_mismatch_error(expected_ty: &Type, actual_ty: &Type) -> CodegenError {
        CodegenError::new(format!(
            "Type mismatch: expected {}, got {}",
            Self::format_diagnostic_type(expected_ty),
            Self::format_diagnostic_type(actual_ty)
        ))
    }

    fn has_known_codegen_type(&self, name: &str) -> bool {
        self.classes.contains_key(name)
            || self.enums.contains_key(name)
            || self.interfaces.contains_key(name)
    }

    fn interface_base_name_from_ref(raw: &str) -> Option<String> {
        match parse_type_source(raw).ok()? {
            Type::Named(name) | Type::Generic(name, _) => Some(name),
            _ => None,
        }
    }

    pub(crate) fn resolve_interface_name_for_lookup(
        &self,
        raw_name: &str,
        module_prefix: Option<&str>,
    ) -> Option<String> {
        let base_name = Self::interface_base_name_from_ref(raw_name)?;

        if let Some(prefix) = module_prefix {
            let scoped = format!("{}__{}", prefix, base_name);
            if self.interfaces.contains_key(&scoped) {
                return Some(scoped);
            }
        }

        if self.interfaces.contains_key(&base_name) {
            return Some(base_name);
        }

        let mangled = base_name.replace('.', "__");
        if self.interfaces.contains_key(&mangled) {
            return Some(mangled);
        }

        if let Some(resolved) = self.resolve_alias_qualified_codegen_type_name(&base_name) {
            if self.interfaces.contains_key(&resolved) {
                return Some(resolved);
            }
        }

        self.canonical_codegen_type_name(&base_name)
            .filter(|name| self.interfaces.contains_key(name))
    }

    fn collect_interface_methods_from_decl(
        decl: &Spanned<Decl>,
        module_prefix: Option<&str>,
        interfaces: &mut HashMap<String, HashSet<String>>,
        interface_extends: &mut HashMap<String, Vec<(Option<String>, String)>>,
    ) {
        match &decl.node {
            Decl::Interface(interface) => {
                let interface_name = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, interface.name)
                } else {
                    interface.name.clone()
                };
                interfaces.insert(
                    interface_name.clone(),
                    interface
                        .methods
                        .iter()
                        .map(|method| method.name.clone())
                        .collect(),
                );
                interface_extends.insert(
                    interface_name,
                    interface
                        .extends
                        .iter()
                        .cloned()
                        .map(|parent| (module_prefix.map(str::to_string), parent))
                        .collect(),
                );
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                for nested in &module.declarations {
                    Self::collect_interface_methods_from_decl(
                        nested,
                        Some(&next_prefix),
                        interfaces,
                        interface_extends,
                    );
                }
            }
            Decl::Function(_) | Decl::Class(_) | Decl::Enum(_) | Decl::Import(_) => {}
        }
    }

    fn collect_interface_methods_with_inheritance(
        &self,
        interface_name: &str,
        own_methods: &HashMap<String, HashSet<String>>,
        interface_extends: &HashMap<String, Vec<(Option<String>, String)>>,
        cache: &mut HashMap<String, HashSet<String>>,
        visiting: &mut HashSet<String>,
    ) -> HashSet<String> {
        if let Some(methods) = cache.get(interface_name) {
            return methods.clone();
        }
        if !visiting.insert(interface_name.to_string()) {
            return own_methods.get(interface_name).cloned().unwrap_or_default();
        }

        let mut methods = own_methods.get(interface_name).cloned().unwrap_or_default();
        if let Some(parents) = interface_extends.get(interface_name) {
            for (module_prefix, parent) in parents {
                if let Some(parent_name) =
                    self.resolve_interface_name_for_lookup(parent, module_prefix.as_deref())
                {
                    methods.extend(self.collect_interface_methods_with_inheritance(
                        &parent_name,
                        own_methods,
                        interface_extends,
                        cache,
                        visiting,
                    ));
                }
            }
        }

        visiting.remove(interface_name);
        cache.insert(interface_name.to_string(), methods.clone());
        methods
    }

    fn collect_interface_ancestors(
        &self,
        interface_name: &str,
        interface_extends: &HashMap<String, Vec<(Option<String>, String)>>,
        cache: &mut HashMap<String, HashSet<String>>,
        visiting: &mut HashSet<String>,
    ) -> HashSet<String> {
        if let Some(ancestors) = cache.get(interface_name) {
            return ancestors.clone();
        }
        if !visiting.insert(interface_name.to_string()) {
            return HashSet::from([interface_name.to_string()]);
        }

        let mut ancestors = HashSet::from([interface_name.to_string()]);
        if let Some(parents) = interface_extends.get(interface_name) {
            for (module_prefix, parent) in parents {
                if let Some(parent_name) =
                    self.resolve_interface_name_for_lookup(parent, module_prefix.as_deref())
                {
                    ancestors.extend(self.collect_interface_ancestors(
                        &parent_name,
                        interface_extends,
                        cache,
                        visiting,
                    ));
                }
            }
        }

        visiting.remove(interface_name);
        cache.insert(interface_name.to_string(), ancestors.clone());
        ancestors
    }

    fn collect_class_interface_impls_from_decl(
        decl: &Spanned<Decl>,
        module_prefix: Option<&str>,
        class_impls: &mut Vec<(String, Option<String>, String)>,
    ) {
        match &decl.node {
            Decl::Class(class) => {
                let class_name = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, class.name)
                } else {
                    class.name.clone()
                };
                for interface in &class.implements {
                    class_impls.push((
                        class_name.clone(),
                        module_prefix.map(str::to_string),
                        interface.clone(),
                    ));
                }
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                for nested in &module.declarations {
                    Self::collect_class_interface_impls_from_decl(
                        nested,
                        Some(&next_prefix),
                        class_impls,
                    );
                }
            }
            Decl::Function(_) | Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
        }
    }

    fn reset_current_generic_bounds(&mut self) {
        self.current_generic_bounds.clear();
    }

    fn extend_current_generic_bounds(&mut self, generic_params: &[GenericParam]) {
        for param in generic_params {
            if !param.bounds.is_empty() {
                self.current_generic_bounds
                    .insert(param.name.clone(), param.bounds.clone());
            }
        }
    }

    pub(crate) fn resolved_generic_bound_interfaces(&self, ty: &Type) -> Vec<String> {
        match self.normalize_codegen_type(ty) {
            Type::Named(name) | Type::Generic(name, _) => self
                .current_generic_bounds
                .get(&name)
                .into_iter()
                .flatten()
                .filter_map(|bound| self.resolve_interface_name_for_lookup(bound, None))
                .collect(),
            Type::Ref(inner)
            | Type::MutRef(inner)
            | Type::Box(inner)
            | Type::Rc(inner)
            | Type::Arc(inner) => self.resolved_generic_bound_interfaces(&inner),
            _ => Vec::new(),
        }
    }

    pub(crate) fn type_contains_active_generic_placeholder(&self, ty: &Type) -> bool {
        match self.normalize_codegen_type(ty) {
            Type::Named(name) => {
                self.current_generic_bounds.contains_key(&name)
                    || (!self.classes.contains_key(&name)
                        && !self.enums.contains_key(&name)
                        && !self.interfaces.contains_key(&name))
            }
            Type::Generic(_, args) => args
                .iter()
                .any(|arg| self.type_contains_active_generic_placeholder(arg)),
            Type::Function(params, ret) => {
                params
                    .iter()
                    .any(|param| self.type_contains_active_generic_placeholder(param))
                    || self.type_contains_active_generic_placeholder(&ret)
            }
            Type::Option(inner)
            | Type::List(inner)
            | Type::Set(inner)
            | Type::Ref(inner)
            | Type::MutRef(inner)
            | Type::Box(inner)
            | Type::Rc(inner)
            | Type::Arc(inner)
            | Type::Ptr(inner)
            | Type::Task(inner)
            | Type::Range(inner) => self.type_contains_active_generic_placeholder(&inner),
            Type::Result(ok, err) | Type::Map(ok, err) => {
                self.type_contains_active_generic_placeholder(&ok)
                    || self.type_contains_active_generic_placeholder(&err)
            }
            Type::Integer
            | Type::Float
            | Type::Boolean
            | Type::String
            | Type::Char
            | Type::None => false,
        }
    }

    fn matching_interface_implementors(
        &self,
        interface_names: &[String],
        method: &str,
    ) -> Result<HashSet<String>> {
        let matching_interfaces = interface_names
            .iter()
            .filter(|interface_name| {
                self.interfaces
                    .get(*interface_name)
                    .is_some_and(|methods| methods.contains(method))
            })
            .cloned()
            .collect::<Vec<_>>();
        if matching_interfaces.is_empty() {
            let display_interface = interface_names
                .first()
                .map(String::as_str)
                .unwrap_or("unknown");
            return Err(Self::unknown_interface_method_error(
                method,
                display_interface,
            ));
        }

        let mut implementors = HashSet::new();
        for interface_name in matching_interfaces {
            if let Some(owners) = self.interface_implementors.get(&interface_name) {
                implementors.extend(owners.iter().cloned());
            }
        }
        Ok(implementors)
    }

    fn infer_bound_field_function_type(&self, object: &Expr, field: &str) -> Option<Type> {
        let obj_ty = self
            .infer_object_type(object)
            .or_else(|| Some(self.infer_builtin_argument_type(object)))?;
        let (class_name, generic_args) = self.unwrap_class_like_type(&obj_ty)?;

        if let Some(class_info) = self.classes.get(&class_name) {
            let method_name = self.resolve_method_function_name(&class_name, field)?;
            let (_, func_ty) = self.functions.get(&method_name)?;
            if let Some(args) = generic_args.as_ref() {
                if class_info.generic_params.len() == args.len() {
                    let bindings = class_info
                        .generic_params
                        .iter()
                        .cloned()
                        .zip(args.iter().cloned())
                        .collect::<HashMap<_, _>>();
                    return Some(Self::substitute_type(func_ty, &bindings));
                }
            }
            return Some(func_ty.clone());
        }

        if self.enums.contains_key(&class_name) {
            return None;
        }

        let generic_bound_interfaces = self.resolved_generic_bound_interfaces(&obj_ty);
        let receiver_interfaces = if !generic_bound_interfaces.is_empty() {
            generic_bound_interfaces
        } else if self.interfaces.contains_key(&class_name) {
            vec![class_name.clone()]
        } else {
            Vec::new()
        };
        if receiver_interfaces.is_empty() {
            return None;
        }

        let implementors = self
            .matching_interface_implementors(&receiver_interfaces, field)
            .ok()?;
        let suffix = format!("__{}", field);
        let mut candidates = self
            .functions
            .iter()
            .filter_map(|(name, (_, ty))| {
                let owner = name.strip_suffix(&suffix)?;
                implementors.contains(owner).then_some(ty.clone())
            })
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return None;
        }

        candidates.pop()
    }

    pub(crate) fn canonical_codegen_type_name(&self, name: &str) -> Option<String> {
        if self.has_known_codegen_type(name) {
            return Some(name.to_string());
        }

        if name.contains('.') {
            let mangled = name.replace('.', "__");
            if self.has_known_codegen_type(&mangled) {
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

    pub(crate) fn normalize_user_defined_generic_type(
        &self,
        name: &str,
        args: &[Type],
    ) -> Option<Type> {
        let canonical_name = self.canonical_codegen_type_name(name)?;
        let spec_name = Self::generic_class_spec_name(&canonical_name, args);
        if self.classes.contains_key(&spec_name) {
            Some(Type::Named(spec_name))
        } else {
            Some(Type::Generic(canonical_name, args.to_vec()))
        }
    }

    pub(crate) fn resolve_alias_qualified_codegen_type_name(&self, name: &str) -> Option<String> {
        if let Some(canonical) = self.canonical_or_current_package_type_name(name) {
            return Some(canonical);
        }

        if !name.contains('.') {
            let mut wildcard_matches = self
                .visible_wildcard_import_paths()
                .into_iter()
                .filter_map(|path| path.strip_suffix(".*"))
                .filter_map(|module_path| {
                    self.canonical_or_current_package_type_name(&format!(
                        "{}.{}",
                        module_path, name
                    ))
                })
                .collect::<Vec<_>>();
            wildcard_matches.sort_unstable();
            wildcard_matches.dedup();
            if wildcard_matches.len() == 1 {
                return Some(wildcard_matches[0].clone());
            }
        }

        if let Some((alias, rest)) = name.split_once('.') {
            if let Some(path) = self.lookup_import_alias_path(alias) {
                let candidate = format!("{}.{}", path, rest);
                if let Some(canonical) = self.canonical_or_current_package_type_name(&candidate) {
                    return Some(canonical);
                }
            }
        }

        let path = self.lookup_import_alias_path(name)?;
        if path.ends_with(".*") {
            return None;
        }
        self.canonical_or_current_package_type_name(path)
    }

    fn current_package_relative_path<'a>(&self, path: &'a str) -> Option<&'a str> {
        if self.current_package.is_empty() {
            return None;
        }
        path.strip_prefix(&self.current_package)
            .and_then(|rest| rest.strip_prefix('.'))
    }

    fn canonical_or_current_package_type_name(&self, name: &str) -> Option<String> {
        self.canonical_codegen_type_name(name).or_else(|| {
            self.current_package_relative_path(name)
                .and_then(|relative| self.canonical_codegen_type_name(relative))
        })
    }

    pub(crate) fn type_specialization_suffix(ty: &Type) -> String {
        match ty {
            Type::Integer => "I64".to_string(),
            Type::Float => "F64".to_string(),
            Type::Boolean => "Bool".to_string(),
            Type::String => "Str".to_string(),
            Type::Char => "Char".to_string(),
            Type::None => "None".to_string(),
            Type::Named(name) => format!("N{}_{}", name.len(), name),
            Type::Generic(name, args) => format!(
                "G{}_{}{}",
                name.len(),
                name,
                args.iter()
                    .map(Self::type_specialization_suffix)
                    .collect::<Vec<_>>()
                    .join("_")
            ),
            Type::Function(params, ret) => format!(
                "Fn{}To{}",
                params
                    .iter()
                    .map(Self::type_specialization_suffix)
                    .collect::<Vec<_>>()
                    .join("_"),
                Self::type_specialization_suffix(ret)
            ),
            Type::Option(inner) => format!("Opt{}", Self::type_specialization_suffix(inner)),
            Type::Result(ok, err) => format!(
                "Res{}_{}",
                Self::type_specialization_suffix(ok),
                Self::type_specialization_suffix(err)
            ),
            Type::List(inner) => format!("List{}", Self::type_specialization_suffix(inner)),
            Type::Map(k, v) => format!(
                "Map{}_{}",
                Self::type_specialization_suffix(k),
                Self::type_specialization_suffix(v)
            ),
            Type::Set(inner) => format!("Set{}", Self::type_specialization_suffix(inner)),
            Type::Ref(inner) => format!("Ref{}", Self::type_specialization_suffix(inner)),
            Type::MutRef(inner) => format!("MutRef{}", Self::type_specialization_suffix(inner)),
            Type::Box(inner) => format!("Box{}", Self::type_specialization_suffix(inner)),
            Type::Rc(inner) => format!("Rc{}", Self::type_specialization_suffix(inner)),
            Type::Arc(inner) => format!("Arc{}", Self::type_specialization_suffix(inner)),
            Type::Ptr(inner) => format!("Ptr{}", Self::type_specialization_suffix(inner)),
            Type::Task(inner) => format!("Task{}", Self::type_specialization_suffix(inner)),
            Type::Range(inner) => format!("Range{}", Self::type_specialization_suffix(inner)),
        }
    }

    pub(crate) fn format_type_string(ty: &Type) -> String {
        match ty {
            Type::Integer => "Integer".to_string(),
            Type::Float => "Float".to_string(),
            Type::Boolean => "Boolean".to_string(),
            Type::String => "String".to_string(),
            Type::Char => "Char".to_string(),
            Type::None => "None".to_string(),
            Type::Named(name) => name.clone(),
            Type::Generic(name, args) => format!(
                "{}<{}>",
                name,
                args.iter()
                    .map(Self::format_type_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Type::Function(params, ret) => format!(
                "({}) -> {}",
                params
                    .iter()
                    .map(Self::format_type_string)
                    .collect::<Vec<_>>()
                    .join(", "),
                Self::format_type_string(ret)
            ),
            Type::Option(inner) => format!("Option<{}>", Self::format_type_string(inner)),
            Type::Result(ok, err) => format!(
                "Result<{}, {}>",
                Self::format_type_string(ok),
                Self::format_type_string(err)
            ),
            Type::List(inner) => format!("List<{}>", Self::format_type_string(inner)),
            Type::Map(k, v) => format!(
                "Map<{}, {}>",
                Self::format_type_string(k),
                Self::format_type_string(v)
            ),
            Type::Set(inner) => format!("Set<{}>", Self::format_type_string(inner)),
            Type::Ref(inner) => format!("&{}", Self::format_type_string(inner)),
            Type::MutRef(inner) => format!("&mut {}", Self::format_type_string(inner)),
            Type::Box(inner) => format!("Box<{}>", Self::format_type_string(inner)),
            Type::Rc(inner) => format!("Rc<{}>", Self::format_type_string(inner)),
            Type::Arc(inner) => format!("Arc<{}>", Self::format_type_string(inner)),
            Type::Ptr(inner) => format!("Ptr<{}>", Self::format_type_string(inner)),
            Type::Task(inner) => format!("Task<{}>", Self::format_type_string(inner)),
            Type::Range(inner) => format!("Range<{}>", Self::format_type_string(inner)),
        }
    }

    fn supports_display_scalar(ty: &Type) -> bool {
        matches!(
            ty,
            Type::Integer | Type::Float | Type::Boolean | Type::String | Type::Char | Type::None
        ) || matches!(ty, Type::Option(inner) if Self::supports_display_scalar(inner))
            || matches!(
                ty,
                Type::Result(ok, err)
                    if Self::supports_display_scalar(ok) && Self::supports_display_scalar(err)
            )
    }

    fn supports_display_expr(expr: &Expr, ty: &Type) -> bool {
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

        match (owner_name.as_str(), field.as_str(), ty) {
            ("Option", "none", Type::Option(_)) => true,
            ("Option", "some", Type::Option(inner)) => args
                .first()
                .is_some_and(|arg| Self::supports_display_expr(&arg.node, inner)),
            ("Result", "ok", Type::Result(ok, _)) => args
                .first()
                .is_some_and(|arg| Self::supports_display_expr(&arg.node, ok)),
            ("Result", "error", Type::Result(_, err)) => args
                .first()
                .is_some_and(|arg| Self::supports_display_expr(&arg.node, err)),
            _ => false,
        }
    }

    pub(crate) fn substitute_type(ty: &Type, bindings: &HashMap<String, Type>) -> Type {
        match ty {
            Type::Named(name) => bindings.get(name).cloned().unwrap_or_else(|| ty.clone()),
            Type::Generic(name, args) => Type::Generic(
                name.clone(),
                args.iter()
                    .map(|arg| Self::substitute_type(arg, bindings))
                    .collect(),
            ),
            Type::Function(params, ret) => Type::Function(
                params
                    .iter()
                    .map(|p| Self::substitute_type(p, bindings))
                    .collect(),
                Box::new(Self::substitute_type(ret, bindings)),
            ),
            Type::Option(inner) => Type::Option(Box::new(Self::substitute_type(inner, bindings))),
            Type::Result(ok, err) => Type::Result(
                Box::new(Self::substitute_type(ok, bindings)),
                Box::new(Self::substitute_type(err, bindings)),
            ),
            Type::List(inner) => Type::List(Box::new(Self::substitute_type(inner, bindings))),
            Type::Map(k, v) => Type::Map(
                Box::new(Self::substitute_type(k, bindings)),
                Box::new(Self::substitute_type(v, bindings)),
            ),
            Type::Set(inner) => Type::Set(Box::new(Self::substitute_type(inner, bindings))),
            Type::Ref(inner) => Type::Ref(Box::new(Self::substitute_type(inner, bindings))),
            Type::MutRef(inner) => Type::MutRef(Box::new(Self::substitute_type(inner, bindings))),
            Type::Box(inner) => Type::Box(Box::new(Self::substitute_type(inner, bindings))),
            Type::Rc(inner) => Type::Rc(Box::new(Self::substitute_type(inner, bindings))),
            Type::Arc(inner) => Type::Arc(Box::new(Self::substitute_type(inner, bindings))),
            Type::Ptr(inner) => Type::Ptr(Box::new(Self::substitute_type(inner, bindings))),
            Type::Task(inner) => Type::Task(Box::new(Self::substitute_type(inner, bindings))),
            Type::Range(inner) => Type::Range(Box::new(Self::substitute_type(inner, bindings))),
            _ => ty.clone(),
        }
    }

    fn owner_class_type_args_from_type(ty: &Type, owner_class: &str) -> Option<Vec<Type>> {
        match ty {
            Type::Generic(name, args) if name == owner_class => Some(args.clone()),
            Type::Ref(inner) | Type::MutRef(inner) => {
                Self::owner_class_type_args_from_type(inner, owner_class)
            }
            Type::Option(inner) if owner_class == "Option" => Some(vec![(**inner).clone()]),
            Type::Result(ok, err) if owner_class == "Result" => {
                Some(vec![(**ok).clone(), (**err).clone()])
            }
            Type::List(inner) if owner_class == "List" => Some(vec![(**inner).clone()]),
            Type::Map(key, value) if owner_class == "Map" => {
                Some(vec![(**key).clone(), (**value).clone()])
            }
            Type::Set(inner) if owner_class == "Set" => Some(vec![(**inner).clone()]),
            Type::Box(inner) if owner_class == "Box" => Some(vec![(**inner).clone()]),
            Type::Rc(inner) if owner_class == "Rc" => Some(vec![(**inner).clone()]),
            Type::Arc(inner) if owner_class == "Arc" => Some(vec![(**inner).clone()]),
            Type::Ptr(inner) if owner_class == "Ptr" => Some(vec![(**inner).clone()]),
            Type::Task(inner) if owner_class == "Task" => Some(vec![(**inner).clone()]),
            Type::Range(inner) if owner_class == "Range" => Some(vec![(**inner).clone()]),
            _ => None,
        }
    }

    fn infer_explicit_receiver_type(expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Construct { ty, .. } => parse_type_source(ty).ok(),
            Expr::Borrow(inner) => Some(Type::Ref(Box::new(Self::infer_explicit_receiver_type(
                &inner.node,
            )?))),
            Expr::MutBorrow(inner) => Some(Type::MutRef(Box::new(
                Self::infer_explicit_receiver_type(&inner.node)?,
            ))),
            Expr::Deref(inner) => match Self::infer_explicit_receiver_type(&inner.node)? {
                Type::Ref(inner) | Type::MutRef(inner) | Type::Ptr(inner) => Some(*inner),
                _ => None,
            },
            Expr::Try(inner) => match Self::infer_explicit_receiver_type(&inner.node)? {
                Type::Result(ok, _) | Type::Option(ok) => Some(*ok),
                _ => None,
            },
            Expr::Await(inner) => match Self::infer_explicit_receiver_type(&inner.node)? {
                Type::Task(inner) => Some(*inner),
                _ => None,
            },
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_ty = Self::infer_explicit_receiver_block_tail_type(then_branch)?;
                let else_ty = else_branch
                    .as_ref()
                    .and_then(|block| Self::infer_explicit_receiver_block_tail_type(block))?;
                (then_ty == else_ty).then_some(then_ty)
            }
            Expr::Match { arms, .. } => {
                let mut arm_types = arms
                    .iter()
                    .filter_map(|arm| Self::infer_explicit_receiver_block_tail_type(&arm.body));
                let first = arm_types.next()?;
                arm_types.all(|ty| ty == first).then_some(first)
            }
            Expr::Block(block) | Expr::AsyncBlock(block) => {
                Self::infer_explicit_receiver_block_tail_type(block)
            }
            _ => None,
        }
    }

    fn infer_explicit_receiver_block_tail_type(block: &[Spanned<Stmt>]) -> Option<Type> {
        let last = block.last()?;
        match &last.node {
            Stmt::Expr(expr) => Self::infer_explicit_receiver_type(&expr.node),
            _ => None,
        }
    }

    fn explicit_receiver_class_bindings(
        object: &Expr,
        owner_class: &str,
        class_templates: &HashMap<String, GenericClassTemplate>,
    ) -> Option<(String, HashMap<String, Type>)> {
        let inferred_ty = Self::infer_explicit_receiver_type(object)?;
        let type_args = Self::owner_class_type_args_from_type(&inferred_ty, owner_class)?;
        let template = class_templates.get(owner_class)?;
        if template.class.generic_params.len() != type_args.len() {
            return None;
        }

        let owner_key = Self::generic_class_spec_name(owner_class, &type_args);
        let bindings = template
            .class
            .generic_params
            .iter()
            .map(|param| param.name.clone())
            .zip(type_args)
            .collect::<HashMap<_, _>>();
        Some((owner_key, bindings))
    }

    fn specialize_constructor_param_types(
        &self,
        source_ty: Option<&Type>,
        normalized_ty: &Type,
        ctor_params: &[Type],
    ) -> Vec<Type> {
        let generic_binding_source = match source_ty {
            Some(Type::Generic(name, args)) => Some((
                self.canonical_codegen_type_name(name)
                    .unwrap_or_else(|| name.clone()),
                args.iter()
                    .map(|arg| self.normalize_codegen_type(arg))
                    .collect::<Vec<_>>(),
            )),
            _ => match normalized_ty {
                Type::Generic(name, args) => Some((name.clone(), args.clone())),
                _ => None,
            },
        };

        let Some((base_name, type_args)) = generic_binding_source else {
            return ctor_params.to_vec();
        };
        let Some(class_info) = self.classes.get(&base_name) else {
            return ctor_params.to_vec();
        };
        if class_info.generic_params.len() != type_args.len() {
            return ctor_params.to_vec();
        }

        let bindings = class_info
            .generic_params
            .iter()
            .cloned()
            .zip(type_args)
            .collect::<HashMap<_, _>>();
        ctor_params
            .iter()
            .map(|param| Self::substitute_type(param, &bindings))
            .collect()
    }

    fn specialize_method_signature_for_receiver(
        &self,
        receiver_ty: Option<&Type>,
        class_name: &str,
        func_ty: &Type,
    ) -> Type {
        let Some(receiver_ty) = receiver_ty else {
            return func_ty.clone();
        };
        let Some((receiver_name, receiver_args)) = self.unwrap_class_like_type(receiver_ty) else {
            return func_ty.clone();
        };
        if receiver_name != class_name {
            return func_ty.clone();
        }
        let Some(type_args) = receiver_args else {
            return func_ty.clone();
        };
        let Some(class_info) = self.classes.get(class_name) else {
            return func_ty.clone();
        };
        if class_info.generic_params.len() != type_args.len() {
            return func_ty.clone();
        }

        let bindings = class_info
            .generic_params
            .iter()
            .cloned()
            .zip(type_args)
            .collect::<HashMap<_, _>>();
        Self::substitute_type(func_ty, &bindings)
    }

    fn local_module_class_name(
        module_prefix: &str,
        name: &str,
        class_templates: &HashMap<String, GenericClassTemplate>,
    ) -> Option<String> {
        let candidate = format!("{}__{}", module_prefix, name);
        class_templates
            .contains_key(&candidate)
            .then_some(candidate)
    }

    fn rewrite_type_for_local_module_classes(
        ty: &Type,
        module_prefix: &str,
        class_templates: &HashMap<String, GenericClassTemplate>,
    ) -> Type {
        match ty {
            Type::Named(name) => {
                Self::local_module_class_name(module_prefix, name, class_templates)
                    .map(Type::Named)
                    .unwrap_or_else(|| ty.clone())
            }
            Type::Generic(name, args) => {
                let rewritten_args = args
                    .iter()
                    .map(|arg| {
                        Self::rewrite_type_for_local_module_classes(
                            arg,
                            module_prefix,
                            class_templates,
                        )
                    })
                    .collect::<Vec<_>>();
                if let Some(class_name) =
                    Self::local_module_class_name(module_prefix, name, class_templates)
                {
                    Type::Generic(class_name, rewritten_args)
                } else {
                    Type::Generic(name.clone(), rewritten_args)
                }
            }
            Type::Function(params, ret) => Type::Function(
                params
                    .iter()
                    .map(|param| {
                        Self::rewrite_type_for_local_module_classes(
                            param,
                            module_prefix,
                            class_templates,
                        )
                    })
                    .collect(),
                Box::new(Self::rewrite_type_for_local_module_classes(
                    ret,
                    module_prefix,
                    class_templates,
                )),
            ),
            Type::Option(inner) => {
                Self::local_module_class_name(module_prefix, "Option", class_templates)
                    .map(|name| {
                        Type::Generic(
                            name,
                            vec![Self::rewrite_type_for_local_module_classes(
                                inner,
                                module_prefix,
                                class_templates,
                            )],
                        )
                    })
                    .unwrap_or_else(|| {
                        Type::Option(Box::new(Self::rewrite_type_for_local_module_classes(
                            inner,
                            module_prefix,
                            class_templates,
                        )))
                    })
            }
            Type::Result(ok, err) => {
                Self::local_module_class_name(module_prefix, "Result", class_templates)
                    .map(|name| {
                        Type::Generic(
                            name,
                            vec![
                                Self::rewrite_type_for_local_module_classes(
                                    ok,
                                    module_prefix,
                                    class_templates,
                                ),
                                Self::rewrite_type_for_local_module_classes(
                                    err,
                                    module_prefix,
                                    class_templates,
                                ),
                            ],
                        )
                    })
                    .unwrap_or_else(|| {
                        Type::Result(
                            Box::new(Self::rewrite_type_for_local_module_classes(
                                ok,
                                module_prefix,
                                class_templates,
                            )),
                            Box::new(Self::rewrite_type_for_local_module_classes(
                                err,
                                module_prefix,
                                class_templates,
                            )),
                        )
                    })
            }
            Type::List(inner) => {
                Self::local_module_class_name(module_prefix, "List", class_templates)
                    .map(|name| {
                        Type::Generic(
                            name,
                            vec![Self::rewrite_type_for_local_module_classes(
                                inner,
                                module_prefix,
                                class_templates,
                            )],
                        )
                    })
                    .unwrap_or_else(|| {
                        Type::List(Box::new(Self::rewrite_type_for_local_module_classes(
                            inner,
                            module_prefix,
                            class_templates,
                        )))
                    })
            }
            Type::Map(key, value) => {
                Self::local_module_class_name(module_prefix, "Map", class_templates)
                    .map(|name| {
                        Type::Generic(
                            name,
                            vec![
                                Self::rewrite_type_for_local_module_classes(
                                    key,
                                    module_prefix,
                                    class_templates,
                                ),
                                Self::rewrite_type_for_local_module_classes(
                                    value,
                                    module_prefix,
                                    class_templates,
                                ),
                            ],
                        )
                    })
                    .unwrap_or_else(|| {
                        Type::Map(
                            Box::new(Self::rewrite_type_for_local_module_classes(
                                key,
                                module_prefix,
                                class_templates,
                            )),
                            Box::new(Self::rewrite_type_for_local_module_classes(
                                value,
                                module_prefix,
                                class_templates,
                            )),
                        )
                    })
            }
            Type::Set(inner) => {
                Self::local_module_class_name(module_prefix, "Set", class_templates)
                    .map(|name| {
                        Type::Generic(
                            name,
                            vec![Self::rewrite_type_for_local_module_classes(
                                inner,
                                module_prefix,
                                class_templates,
                            )],
                        )
                    })
                    .unwrap_or_else(|| {
                        Type::Set(Box::new(Self::rewrite_type_for_local_module_classes(
                            inner,
                            module_prefix,
                            class_templates,
                        )))
                    })
            }
            Type::Box(inner) => {
                Self::local_module_class_name(module_prefix, "Box", class_templates)
                    .map(|name| {
                        Type::Generic(
                            name,
                            vec![Self::rewrite_type_for_local_module_classes(
                                inner,
                                module_prefix,
                                class_templates,
                            )],
                        )
                    })
                    .unwrap_or_else(|| {
                        Type::Box(Box::new(Self::rewrite_type_for_local_module_classes(
                            inner,
                            module_prefix,
                            class_templates,
                        )))
                    })
            }
            Type::Rc(inner) => Self::local_module_class_name(module_prefix, "Rc", class_templates)
                .map(|name| {
                    Type::Generic(
                        name,
                        vec![Self::rewrite_type_for_local_module_classes(
                            inner,
                            module_prefix,
                            class_templates,
                        )],
                    )
                })
                .unwrap_or_else(|| {
                    Type::Rc(Box::new(Self::rewrite_type_for_local_module_classes(
                        inner,
                        module_prefix,
                        class_templates,
                    )))
                }),
            Type::Arc(inner) => {
                Self::local_module_class_name(module_prefix, "Arc", class_templates)
                    .map(|name| {
                        Type::Generic(
                            name,
                            vec![Self::rewrite_type_for_local_module_classes(
                                inner,
                                module_prefix,
                                class_templates,
                            )],
                        )
                    })
                    .unwrap_or_else(|| {
                        Type::Arc(Box::new(Self::rewrite_type_for_local_module_classes(
                            inner,
                            module_prefix,
                            class_templates,
                        )))
                    })
            }
            Type::Ptr(inner) => {
                Self::local_module_class_name(module_prefix, "Ptr", class_templates)
                    .map(|name| {
                        Type::Generic(
                            name,
                            vec![Self::rewrite_type_for_local_module_classes(
                                inner,
                                module_prefix,
                                class_templates,
                            )],
                        )
                    })
                    .unwrap_or_else(|| {
                        Type::Ptr(Box::new(Self::rewrite_type_for_local_module_classes(
                            inner,
                            module_prefix,
                            class_templates,
                        )))
                    })
            }
            Type::Task(inner) => {
                Self::local_module_class_name(module_prefix, "Task", class_templates)
                    .map(|name| {
                        Type::Generic(
                            name,
                            vec![Self::rewrite_type_for_local_module_classes(
                                inner,
                                module_prefix,
                                class_templates,
                            )],
                        )
                    })
                    .unwrap_or_else(|| {
                        Type::Task(Box::new(Self::rewrite_type_for_local_module_classes(
                            inner,
                            module_prefix,
                            class_templates,
                        )))
                    })
            }
            Type::Range(inner) => {
                Self::local_module_class_name(module_prefix, "Range", class_templates)
                    .map(|name| {
                        Type::Generic(
                            name,
                            vec![Self::rewrite_type_for_local_module_classes(
                                inner,
                                module_prefix,
                                class_templates,
                            )],
                        )
                    })
                    .unwrap_or_else(|| {
                        Type::Range(Box::new(Self::rewrite_type_for_local_module_classes(
                            inner,
                            module_prefix,
                            class_templates,
                        )))
                    })
            }
            Type::Ref(inner) => Type::Ref(Box::new(Self::rewrite_type_for_local_module_classes(
                inner,
                module_prefix,
                class_templates,
            ))),
            Type::MutRef(inner) => Type::MutRef(Box::new(
                Self::rewrite_type_for_local_module_classes(inner, module_prefix, class_templates),
            )),
            _ => ty.clone(),
        }
    }

    fn module_prefix_for_owner_class(owner_class: &str) -> Option<&str> {
        let base = owner_class
            .split_once("__spec__")
            .map_or(owner_class, |(base, _)| base);
        base.rsplit_once("__").map(|(prefix, _)| prefix)
    }

    fn collect_generic_templates_from_decl(
        decl: &Spanned<Decl>,
        module_prefix: Option<&str>,
        function_templates: &mut HashMap<String, GenericTemplate>,
        method_templates: &mut HashMap<String, Vec<GenericTemplate>>,
    ) {
        match &decl.node {
            Decl::Function(func) => {
                if func.generic_params.is_empty() {
                    return;
                }
                let key = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, func.name)
                } else {
                    func.name.clone()
                };
                function_templates.insert(
                    key,
                    GenericTemplate {
                        func: func.clone(),
                        span: decl.span.clone(),
                        owner_class: None,
                    },
                );
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                for inner in &module.declarations {
                    Self::collect_generic_templates_from_decl(
                        inner,
                        Some(&next_prefix),
                        function_templates,
                        method_templates,
                    );
                }
            }
            Decl::Class(class) => {
                let class_name = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, class.name)
                } else {
                    class.name.clone()
                };
                for method in &class.methods {
                    if method.generic_params.is_empty() {
                        continue;
                    }
                    method_templates
                        .entry(method.name.clone())
                        .or_default()
                        .push(GenericTemplate {
                            func: method.clone(),
                            span: decl.span.clone(),
                            owner_class: Some(class_name.clone()),
                        });
                }
            }
            _ => {}
        }
    }

    fn collect_generic_class_templates_from_decl(
        decl: &Spanned<Decl>,
        module_prefix: Option<&str>,
        class_templates: &mut HashMap<String, GenericClassTemplate>,
    ) {
        match &decl.node {
            Decl::Class(class) => {
                if class.generic_params.is_empty() {
                    return;
                }
                let key = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, class.name)
                } else {
                    class.name.clone()
                };
                let mut class = class.clone();
                class.name = key.clone();
                class_templates.insert(
                    key,
                    GenericClassTemplate {
                        class,
                        span: decl.span.clone(),
                    },
                );
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                for inner in &module.declarations {
                    Self::collect_generic_class_templates_from_decl(
                        inner,
                        Some(&next_prefix),
                        class_templates,
                    );
                }
            }
            _ => {}
        }
    }

    pub(crate) fn generic_class_spec_name(base: &str, args: &[Type]) -> String {
        format!(
            "{}__spec__{}",
            base,
            args.iter()
                .map(Self::type_specialization_suffix)
                .collect::<Vec<_>>()
                .join("_")
        )
    }

    fn template_key_for_callee(callee: &Expr) -> Option<String> {
        match callee {
            Expr::Ident(name) => Some(name.clone()),
            _ => flatten_field_chain(callee).and_then(|parts| {
                if parts.len() >= 2 {
                    Some(parts.join("__"))
                } else {
                    None
                }
            }),
        }
    }

    fn collect_function_template_key_candidates(
        callee: &Expr,
        import_aliases: &HashMap<String, String>,
    ) -> Vec<String> {
        let mut candidates = Vec::new();
        let mut push_candidate = |candidate: String| {
            if !candidate.is_empty() && !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        };

        if let Some(direct_key) = Self::template_key_for_callee(callee) {
            push_candidate(direct_key);
        }

        match callee {
            Expr::Ident(name) => {
                if let Some(path) = import_aliases.get(name) {
                    if !path.ends_with(".*") {
                        push_candidate(path.replace('.', "__"));
                        if let Some(symbol) = path.rsplit('.').next() {
                            push_candidate(symbol.to_string());
                        }
                    }
                }
            }
            _ => {
                if let Some(path_parts) = flatten_field_chain(callee) {
                    if path_parts.len() >= 2 {
                        if let Some(path) = import_aliases.get(&path_parts[0]) {
                            if !path.ends_with(".*") {
                                push_candidate(
                                    format!("{}.{}", path, path_parts[1..].join("."))
                                        .replace('.', "__"),
                                );
                                if let Some(leaf) = path_parts.last() {
                                    push_candidate(leaf.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        candidates
    }

    fn resolve_function_template_key(
        function_templates: &HashMap<String, GenericTemplate>,
        import_aliases: &HashMap<String, String>,
        callee: &Expr,
    ) -> Option<String> {
        let candidates = Self::collect_function_template_key_candidates(callee, import_aliases);
        for candidate in &candidates {
            if function_templates.contains_key(candidate) {
                return Some(candidate.clone());
            }
        }

        let mut matches = candidates
            .iter()
            .flat_map(|candidate| {
                let leaf_name = candidate.rsplit("__").next().unwrap_or(candidate);
                let suffix = format!("__{}", leaf_name);
                function_templates
                    .keys()
                    .filter(move |template_key| {
                        *template_key == leaf_name || template_key.ends_with(&suffix)
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        matches.sort_unstable();
        matches.dedup();
        (matches.len() == 1).then(|| matches[0].clone())
    }

    fn resolve_class_template_key(
        class_templates: &HashMap<String, GenericClassTemplate>,
        import_aliases: &HashMap<String, String>,
        callee: &Expr,
    ) -> Option<String> {
        let mut candidates = Vec::new();
        let mut push_candidate = |candidate: String| {
            if !candidate.is_empty() && !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        };

        if let Some(direct_key) = Self::template_key_for_callee(callee) {
            push_candidate(direct_key);
        }

        match callee {
            Expr::Ident(name) => {
                if let Some(path) = import_aliases.get(name) {
                    if !path.ends_with(".*") {
                        push_candidate(path.replace('.', "__"));
                        if let Some(symbol) = path.rsplit('.').next() {
                            push_candidate(symbol.to_string());
                        }
                    }
                }
            }
            _ => {
                if let Some(path_parts) = flatten_field_chain(callee) {
                    if path_parts.len() >= 2 {
                        if let Some(path) = import_aliases.get(&path_parts[0]) {
                            if !path.ends_with(".*") {
                                push_candidate(
                                    format!("{}.{}", path, path_parts[1..].join("."))
                                        .replace('.', "__"),
                                );
                                if let Some(leaf) = path_parts.last() {
                                    push_candidate(leaf.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        for candidate in &candidates {
            if class_templates.contains_key(candidate) {
                return Some(candidate.clone());
            }
        }

        let mut matches = candidates
            .iter()
            .flat_map(|candidate| {
                let leaf_name = candidate.rsplit("__").next().unwrap_or(candidate);
                let suffix = format!("__{}", leaf_name);
                class_templates
                    .keys()
                    .filter(move |template_key| {
                        *template_key == leaf_name || template_key.ends_with(&suffix)
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        matches.sort_unstable();
        matches.dedup();
        (matches.len() == 1).then(|| matches[0].clone())
    }

    fn resolve_class_template_name(
        class_templates: &HashMap<String, GenericClassTemplate>,
        import_aliases: &HashMap<String, String>,
        name: &str,
    ) -> Option<String> {
        if !name.contains('.') {
            let mut wildcard_matches = import_aliases
                .values()
                .filter_map(|path| path.strip_suffix(".*"))
                .filter_map(|module_path| {
                    let dotted = format!("{}.{}", module_path, name);
                    if class_templates.contains_key(&dotted) {
                        return Some(dotted);
                    }
                    let mangled = dotted.replace('.', "__");
                    class_templates.contains_key(&mangled).then_some(mangled)
                })
                .collect::<Vec<_>>();
            wildcard_matches.sort_unstable();
            wildcard_matches.dedup();
            if wildcard_matches.len() == 1 {
                return Some(wildcard_matches[0].clone());
            }
        }

        if class_templates.contains_key(name) {
            return Some(name.to_string());
        }
        if name.contains('.') {
            let mangled = name.replace('.', "__");
            if class_templates.contains_key(&mangled) {
                return Some(mangled);
            }
        }
        if let Some((alias, rest)) = name.split_once('.') {
            if let Some(path) = import_aliases.get(alias) {
                let dotted = format!("{}.{}", path, rest);
                if class_templates.contains_key(&dotted) {
                    return Some(dotted);
                }
                let mangled = dotted.replace('.', "__");
                if class_templates.contains_key(&mangled) {
                    return Some(mangled);
                }
            }
        }
        if let Some(path) = import_aliases.get(name) {
            if class_templates.contains_key(path) {
                return Some(path.clone());
            }
            let mangled = path.replace('.', "__");
            if class_templates.contains_key(&mangled) {
                return Some(mangled);
            }
        }

        let leaf_name = name.rsplit('.').next().unwrap_or(name);
        let suffix = format!("__{}", leaf_name);
        let mut matches = class_templates
            .keys()
            .filter(|template_key| *template_key == leaf_name || template_key.ends_with(&suffix))
            .cloned()
            .collect::<Vec<_>>();
        matches.sort_unstable();
        matches.dedup();
        (matches.len() == 1).then(|| matches[0].clone())
    }

    fn rewrite_stmt_generic_calls(
        stmt: &Stmt,
        templates: &GenericRewriteTemplates<'_>,
        outputs: &mut GenericRewriteOutputs<'_>,
    ) -> Result<Stmt> {
        if !Self::stmt_needs_generic_call_rewrite(stmt) {
            return Ok(stmt.clone());
        }

        Ok(match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
            } => Stmt::Let {
                name: name.clone(),
                ty: ty.clone(),
                value: Spanned::new(
                    Self::rewrite_expr_generic_calls(&value.node, templates, outputs)?,
                    value.span.clone(),
                ),
                mutable: *mutable,
            },
            Stmt::Assign { target, value } => Stmt::Assign {
                target: Spanned::new(
                    Self::rewrite_expr_generic_calls(&target.node, templates, outputs)?,
                    target.span.clone(),
                ),
                value: Spanned::new(
                    Self::rewrite_expr_generic_calls(&value.node, templates, outputs)?,
                    value.span.clone(),
                ),
            },
            Stmt::Expr(expr) => Stmt::Expr(Spanned::new(
                Self::rewrite_expr_generic_calls(&expr.node, templates, outputs)?,
                expr.span.clone(),
            )),
            Stmt::Return(expr) => Stmt::Return(
                expr.as_ref()
                    .map(|e| {
                        Ok(Spanned::new(
                            Self::rewrite_expr_generic_calls(&e.node, templates, outputs)?,
                            e.span.clone(),
                        ))
                    })
                    .transpose()?,
            ),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => Stmt::If {
                condition: Spanned::new(
                    Self::rewrite_expr_generic_calls(&condition.node, templates, outputs)?,
                    condition.span.clone(),
                ),
                then_block: then_block
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(&s.node, templates, outputs)?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
                else_block: else_block
                    .as_ref()
                    .map(|blk| {
                        blk.iter()
                            .map(|s| {
                                Ok(Spanned::new(
                                    Self::rewrite_stmt_generic_calls(&s.node, templates, outputs)?,
                                    s.span.clone(),
                                ))
                            })
                            .collect::<Result<Vec<_>>>()
                    })
                    .transpose()?,
            },
            Stmt::While { condition, body } => Stmt::While {
                condition: Spanned::new(
                    Self::rewrite_expr_generic_calls(&condition.node, templates, outputs)?,
                    condition.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(&s.node, templates, outputs)?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
            Stmt::For {
                var,
                var_type,
                iterable,
                body,
            } => Stmt::For {
                var: var.clone(),
                var_type: var_type.clone(),
                iterable: Spanned::new(
                    Self::rewrite_expr_generic_calls(&iterable.node, templates, outputs)?,
                    iterable.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(&s.node, templates, outputs)?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
            Stmt::Match { expr, arms } => Stmt::Match {
                expr: Spanned::new(
                    Self::rewrite_expr_generic_calls(&expr.node, templates, outputs)?,
                    expr.span.clone(),
                ),
                arms: arms
                    .iter()
                    .map(|arm| {
                        Ok(MatchArm {
                            pattern: arm.pattern.clone(),
                            body: arm
                                .body
                                .iter()
                                .map(|s| {
                                    Ok(Spanned::new(
                                        Self::rewrite_stmt_generic_calls(
                                            &s.node, templates, outputs,
                                        )?,
                                        s.span.clone(),
                                    ))
                                })
                                .collect::<Result<Vec<_>>>()?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
            Stmt::Break => Stmt::Break,
            Stmt::Continue => Stmt::Continue,
        })
    }

    fn stmt_needs_generic_call_rewrite(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Let { value, .. } => Self::expr_needs_generic_call_rewrite(&value.node),
            Stmt::Assign { target, value } => {
                Self::expr_needs_generic_call_rewrite(&target.node)
                    || Self::expr_needs_generic_call_rewrite(&value.node)
            }
            Stmt::Expr(expr) => Self::expr_needs_generic_call_rewrite(&expr.node),
            Stmt::Return(expr) => expr
                .as_ref()
                .is_some_and(|expr| Self::expr_needs_generic_call_rewrite(&expr.node)),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                Self::expr_needs_generic_call_rewrite(&condition.node)
                    || then_block
                        .iter()
                        .any(|stmt| Self::stmt_needs_generic_call_rewrite(&stmt.node))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|stmt| Self::stmt_needs_generic_call_rewrite(&stmt.node))
                    })
            }
            Stmt::While { condition, body } => {
                Self::expr_needs_generic_call_rewrite(&condition.node)
                    || body
                        .iter()
                        .any(|stmt| Self::stmt_needs_generic_call_rewrite(&stmt.node))
            }
            Stmt::For { iterable, body, .. } => {
                Self::expr_needs_generic_call_rewrite(&iterable.node)
                    || body
                        .iter()
                        .any(|stmt| Self::stmt_needs_generic_call_rewrite(&stmt.node))
            }
            Stmt::Match { expr, arms } => {
                Self::expr_needs_generic_call_rewrite(&expr.node)
                    || arms.iter().any(|arm| {
                        arm.body
                            .iter()
                            .any(|stmt| Self::stmt_needs_generic_call_rewrite(&stmt.node))
                    })
            }
            Stmt::Break | Stmt::Continue => false,
        }
    }

    fn expr_needs_generic_call_rewrite(expr: &Expr) -> bool {
        match expr {
            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                !type_args.is_empty()
                    || Self::expr_needs_generic_call_rewrite(&callee.node)
                    || args
                        .iter()
                        .any(|arg| Self::expr_needs_generic_call_rewrite(&arg.node))
            }
            Expr::GenericFunctionValue { callee, type_args } => {
                !type_args.is_empty() || Self::expr_needs_generic_call_rewrite(&callee.node)
            }
            Expr::Construct { args, .. } => args
                .iter()
                .any(|arg| Self::expr_needs_generic_call_rewrite(&arg.node)),
            Expr::Binary { left, right, .. } => {
                Self::expr_needs_generic_call_rewrite(&left.node)
                    || Self::expr_needs_generic_call_rewrite(&right.node)
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr)
            | Expr::Field { object: expr, .. } => Self::expr_needs_generic_call_rewrite(&expr.node),
            Expr::Index { object, index } => {
                Self::expr_needs_generic_call_rewrite(&object.node)
                    || Self::expr_needs_generic_call_rewrite(&index.node)
            }
            Expr::Lambda { body, .. } => Self::expr_needs_generic_call_rewrite(&body.node),
            Expr::Match { expr, arms } => {
                Self::expr_needs_generic_call_rewrite(&expr.node)
                    || arms.iter().any(|arm| {
                        arm.body
                            .iter()
                            .any(|stmt| Self::stmt_needs_generic_call_rewrite(&stmt.node))
                    })
            }
            Expr::StringInterp(parts) => parts.iter().any(|part| match part {
                StringPart::Literal(_) => false,
                StringPart::Expr(expr) => Self::expr_needs_generic_call_rewrite(&expr.node),
            }),
            Expr::AsyncBlock(block) | Expr::Block(block) => block
                .iter()
                .any(|stmt| Self::stmt_needs_generic_call_rewrite(&stmt.node)),
            Expr::Require { condition, message } => {
                Self::expr_needs_generic_call_rewrite(&condition.node)
                    || message
                        .as_ref()
                        .is_some_and(|message| Self::expr_needs_generic_call_rewrite(&message.node))
            }
            Expr::Range { start, end, .. } => {
                start
                    .as_ref()
                    .is_some_and(|start| Self::expr_needs_generic_call_rewrite(&start.node))
                    || end
                        .as_ref()
                        .is_some_and(|end| Self::expr_needs_generic_call_rewrite(&end.node))
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expr_needs_generic_call_rewrite(&condition.node)
                    || then_branch
                        .iter()
                        .any(|stmt| Self::stmt_needs_generic_call_rewrite(&stmt.node))
                    || else_branch.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|stmt| Self::stmt_needs_generic_call_rewrite(&stmt.node))
                    })
            }
            Expr::Literal(_) | Expr::Ident(_) | Expr::This => false,
        }
    }

    fn substitute_expr_types(expr: &Expr, bindings: &HashMap<String, Type>) -> Expr {
        match expr {
            Expr::Call {
                callee,
                args,
                type_args,
            } => Expr::Call {
                callee: Box::new(Spanned::new(
                    Self::substitute_expr_types(&callee.node, bindings),
                    callee.span.clone(),
                )),
                args: args
                    .iter()
                    .map(|a| {
                        Spanned::new(
                            Self::substitute_expr_types(&a.node, bindings),
                            a.span.clone(),
                        )
                    })
                    .collect(),
                type_args: type_args
                    .iter()
                    .map(|t| Self::substitute_type(t, bindings))
                    .collect(),
            },
            Expr::Construct { ty, args } => {
                let rewritten_ty = parse_type_source(ty)
                    .ok()
                    .map(|parsed| Self::substitute_type(&parsed, bindings))
                    .map(|rewritten| Self::format_type_string(&rewritten))
                    .unwrap_or_else(|| ty.clone());
                Expr::Construct {
                    ty: rewritten_ty,
                    args: args
                        .iter()
                        .map(|arg| {
                            Spanned::new(
                                Self::substitute_expr_types(&arg.node, bindings),
                                arg.span.clone(),
                            )
                        })
                        .collect(),
                }
            }
            Expr::Binary { op, left, right } => Expr::Binary {
                op: *op,
                left: Box::new(Spanned::new(
                    Self::substitute_expr_types(&left.node, bindings),
                    left.span.clone(),
                )),
                right: Box::new(Spanned::new(
                    Self::substitute_expr_types(&right.node, bindings),
                    right.span.clone(),
                )),
            },
            Expr::Unary { op, expr } => Expr::Unary {
                op: *op,
                expr: Box::new(Spanned::new(
                    Self::substitute_expr_types(&expr.node, bindings),
                    expr.span.clone(),
                )),
            },
            Expr::Field { object, field } => Expr::Field {
                object: Box::new(Spanned::new(
                    Self::substitute_expr_types(&object.node, bindings),
                    object.span.clone(),
                )),
                field: field.clone(),
            },
            Expr::Index { object, index } => Expr::Index {
                object: Box::new(Spanned::new(
                    Self::substitute_expr_types(&object.node, bindings),
                    object.span.clone(),
                )),
                index: Box::new(Spanned::new(
                    Self::substitute_expr_types(&index.node, bindings),
                    index.span.clone(),
                )),
            },
            Expr::Lambda { params, body } => Expr::Lambda {
                params: params
                    .iter()
                    .map(|p| Parameter {
                        name: p.name.clone(),
                        ty: Self::substitute_type(&p.ty, bindings),
                        mutable: p.mutable,
                        mode: p.mode,
                    })
                    .collect(),
                body: Box::new(Spanned::new(
                    Self::substitute_expr_types(&body.node, bindings),
                    body.span.clone(),
                )),
            },
            Expr::StringInterp(parts) => Expr::StringInterp(
                parts
                    .iter()
                    .map(|p| match p {
                        StringPart::Literal(s) => StringPart::Literal(s.clone()),
                        StringPart::Expr(e) => StringPart::Expr(Spanned::new(
                            Self::substitute_expr_types(&e.node, bindings),
                            e.span.clone(),
                        )),
                    })
                    .collect(),
            ),
            Expr::Try(inner) => Expr::Try(Box::new(Spanned::new(
                Self::substitute_expr_types(&inner.node, bindings),
                inner.span.clone(),
            ))),
            Expr::Borrow(inner) => Expr::Borrow(Box::new(Spanned::new(
                Self::substitute_expr_types(&inner.node, bindings),
                inner.span.clone(),
            ))),
            Expr::MutBorrow(inner) => Expr::MutBorrow(Box::new(Spanned::new(
                Self::substitute_expr_types(&inner.node, bindings),
                inner.span.clone(),
            ))),
            Expr::Deref(inner) => Expr::Deref(Box::new(Spanned::new(
                Self::substitute_expr_types(&inner.node, bindings),
                inner.span.clone(),
            ))),
            Expr::Await(inner) => Expr::Await(Box::new(Spanned::new(
                Self::substitute_expr_types(&inner.node, bindings),
                inner.span.clone(),
            ))),
            Expr::Require { condition, message } => Expr::Require {
                condition: Box::new(Spanned::new(
                    Self::substitute_expr_types(&condition.node, bindings),
                    condition.span.clone(),
                )),
                message: message.as_ref().map(|m| {
                    Box::new(Spanned::new(
                        Self::substitute_expr_types(&m.node, bindings),
                        m.span.clone(),
                    ))
                }),
            },
            Expr::Range {
                start,
                end,
                inclusive,
            } => Expr::Range {
                start: start.as_ref().map(|s| {
                    Box::new(Spanned::new(
                        Self::substitute_expr_types(&s.node, bindings),
                        s.span.clone(),
                    ))
                }),
                end: end.as_ref().map(|e| {
                    Box::new(Spanned::new(
                        Self::substitute_expr_types(&e.node, bindings),
                        e.span.clone(),
                    ))
                }),
                inclusive: *inclusive,
            },
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => Expr::If {
                condition: Box::new(Spanned::new(
                    Self::substitute_expr_types(&condition.node, bindings),
                    condition.span.clone(),
                )),
                then_branch: then_branch
                    .iter()
                    .map(|s| {
                        Spanned::new(
                            Self::substitute_stmt_types(&s.node, bindings),
                            s.span.clone(),
                        )
                    })
                    .collect(),
                else_branch: else_branch.as_ref().map(|blk| {
                    blk.iter()
                        .map(|s| {
                            Spanned::new(
                                Self::substitute_stmt_types(&s.node, bindings),
                                s.span.clone(),
                            )
                        })
                        .collect()
                }),
            },
            Expr::Block(block) => Expr::Block(
                block
                    .iter()
                    .map(|s| {
                        Spanned::new(
                            Self::substitute_stmt_types(&s.node, bindings),
                            s.span.clone(),
                        )
                    })
                    .collect(),
            ),
            Expr::AsyncBlock(block) => Expr::AsyncBlock(
                block
                    .iter()
                    .map(|s| {
                        Spanned::new(
                            Self::substitute_stmt_types(&s.node, bindings),
                            s.span.clone(),
                        )
                    })
                    .collect(),
            ),
            Expr::Match { expr, arms } => Expr::Match {
                expr: Box::new(Spanned::new(
                    Self::substitute_expr_types(&expr.node, bindings),
                    expr.span.clone(),
                )),
                arms: arms
                    .iter()
                    .map(|arm| MatchArm {
                        pattern: arm.pattern.clone(),
                        body: arm
                            .body
                            .iter()
                            .map(|s| {
                                Spanned::new(
                                    Self::substitute_stmt_types(&s.node, bindings),
                                    s.span.clone(),
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            },
            _ => expr.clone(),
        }
    }

    fn substitute_stmt_types(stmt: &Stmt, bindings: &HashMap<String, Type>) -> Stmt {
        match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
            } => Stmt::Let {
                name: name.clone(),
                ty: Self::substitute_type(ty, bindings),
                value: Spanned::new(
                    Self::substitute_expr_types(&value.node, bindings),
                    value.span.clone(),
                ),
                mutable: *mutable,
            },
            Stmt::Assign { target, value } => Stmt::Assign {
                target: Spanned::new(
                    Self::substitute_expr_types(&target.node, bindings),
                    target.span.clone(),
                ),
                value: Spanned::new(
                    Self::substitute_expr_types(&value.node, bindings),
                    value.span.clone(),
                ),
            },
            Stmt::Expr(expr) => Stmt::Expr(Spanned::new(
                Self::substitute_expr_types(&expr.node, bindings),
                expr.span.clone(),
            )),
            Stmt::Return(expr) => Stmt::Return(expr.as_ref().map(|e| {
                Spanned::new(
                    Self::substitute_expr_types(&e.node, bindings),
                    e.span.clone(),
                )
            })),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => Stmt::If {
                condition: Spanned::new(
                    Self::substitute_expr_types(&condition.node, bindings),
                    condition.span.clone(),
                ),
                then_block: then_block
                    .iter()
                    .map(|s| {
                        Spanned::new(
                            Self::substitute_stmt_types(&s.node, bindings),
                            s.span.clone(),
                        )
                    })
                    .collect(),
                else_block: else_block.as_ref().map(|blk| {
                    blk.iter()
                        .map(|s| {
                            Spanned::new(
                                Self::substitute_stmt_types(&s.node, bindings),
                                s.span.clone(),
                            )
                        })
                        .collect()
                }),
            },
            Stmt::While { condition, body } => Stmt::While {
                condition: Spanned::new(
                    Self::substitute_expr_types(&condition.node, bindings),
                    condition.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|s| {
                        Spanned::new(
                            Self::substitute_stmt_types(&s.node, bindings),
                            s.span.clone(),
                        )
                    })
                    .collect(),
            },
            Stmt::For {
                var,
                var_type,
                iterable,
                body,
            } => Stmt::For {
                var: var.clone(),
                var_type: var_type
                    .as_ref()
                    .map(|t| Self::substitute_type(t, bindings)),
                iterable: Spanned::new(
                    Self::substitute_expr_types(&iterable.node, bindings),
                    iterable.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|s| {
                        Spanned::new(
                            Self::substitute_stmt_types(&s.node, bindings),
                            s.span.clone(),
                        )
                    })
                    .collect(),
            },
            Stmt::Match { expr, arms } => Stmt::Match {
                expr: Spanned::new(
                    Self::substitute_expr_types(&expr.node, bindings),
                    expr.span.clone(),
                ),
                arms: arms
                    .iter()
                    .map(|arm| MatchArm {
                        pattern: arm.pattern.clone(),
                        body: arm
                            .body
                            .iter()
                            .map(|s| {
                                Spanned::new(
                                    Self::substitute_stmt_types(&s.node, bindings),
                                    s.span.clone(),
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            },
            Stmt::Break => Stmt::Break,
            Stmt::Continue => Stmt::Continue,
        }
    }

    fn rewrite_expr_for_local_module_classes(
        expr: &Expr,
        module_prefix: &str,
        class_templates: &HashMap<String, GenericClassTemplate>,
    ) -> Expr {
        match expr {
            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                let rewritten_callee = Box::new(Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &callee.node,
                        module_prefix,
                        class_templates,
                    ),
                    callee.span.clone(),
                ));
                let rewritten_args = args
                    .iter()
                    .map(|arg| {
                        Spanned::new(
                            Self::rewrite_expr_for_local_module_classes(
                                &arg.node,
                                module_prefix,
                                class_templates,
                            ),
                            arg.span.clone(),
                        )
                    })
                    .collect::<Vec<_>>();
                let rewritten_type_args = type_args
                    .iter()
                    .map(|arg| {
                        Self::rewrite_type_for_local_module_classes(
                            arg,
                            module_prefix,
                            class_templates,
                        )
                    })
                    .collect::<Vec<_>>();

                if !rewritten_type_args.is_empty() {
                    if let Expr::Ident(name) = &callee.node {
                        if let Some(class_name) =
                            Self::local_module_class_name(module_prefix, name, class_templates)
                        {
                            return Expr::Construct {
                                ty: Self::format_type_string(&Type::Generic(
                                    class_name,
                                    rewritten_type_args,
                                )),
                                args: rewritten_args,
                            };
                        }
                    }
                }

                Expr::Call {
                    callee: rewritten_callee,
                    args: rewritten_args,
                    type_args: rewritten_type_args,
                }
            }
            Expr::GenericFunctionValue { callee, type_args } => Expr::GenericFunctionValue {
                callee: Box::new(Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &callee.node,
                        module_prefix,
                        class_templates,
                    ),
                    callee.span.clone(),
                )),
                type_args: type_args
                    .iter()
                    .map(|arg| {
                        Self::rewrite_type_for_local_module_classes(
                            arg,
                            module_prefix,
                            class_templates,
                        )
                    })
                    .collect(),
            },
            Expr::Construct { ty, args } => {
                let rewritten_ty = parse_type_source(ty)
                    .ok()
                    .map(|parsed| {
                        Self::rewrite_type_for_local_module_classes(
                            &parsed,
                            module_prefix,
                            class_templates,
                        )
                    })
                    .map(|rewritten| Self::format_type_string(&rewritten))
                    .unwrap_or_else(|| ty.clone());
                Expr::Construct {
                    ty: rewritten_ty,
                    args: args
                        .iter()
                        .map(|arg| {
                            Spanned::new(
                                Self::rewrite_expr_for_local_module_classes(
                                    &arg.node,
                                    module_prefix,
                                    class_templates,
                                ),
                                arg.span.clone(),
                            )
                        })
                        .collect(),
                }
            }
            Expr::Binary { op, left, right } => Expr::Binary {
                op: *op,
                left: Box::new(Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &left.node,
                        module_prefix,
                        class_templates,
                    ),
                    left.span.clone(),
                )),
                right: Box::new(Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &right.node,
                        module_prefix,
                        class_templates,
                    ),
                    right.span.clone(),
                )),
            },
            Expr::Unary { op, expr } => Expr::Unary {
                op: *op,
                expr: Box::new(Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &expr.node,
                        module_prefix,
                        class_templates,
                    ),
                    expr.span.clone(),
                )),
            },
            Expr::Field { object, field } => Expr::Field {
                object: Box::new(Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &object.node,
                        module_prefix,
                        class_templates,
                    ),
                    object.span.clone(),
                )),
                field: field.clone(),
            },
            Expr::Index { object, index } => Expr::Index {
                object: Box::new(Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &object.node,
                        module_prefix,
                        class_templates,
                    ),
                    object.span.clone(),
                )),
                index: Box::new(Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &index.node,
                        module_prefix,
                        class_templates,
                    ),
                    index.span.clone(),
                )),
            },
            Expr::Lambda { params, body } => Expr::Lambda {
                params: params
                    .iter()
                    .map(|param| Parameter {
                        name: param.name.clone(),
                        ty: Self::rewrite_type_for_local_module_classes(
                            &param.ty,
                            module_prefix,
                            class_templates,
                        ),
                        mutable: param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                        mode: param.mode,
                    })
                    .collect(),
                body: Box::new(Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &body.node,
                        module_prefix,
                        class_templates,
                    ),
                    body.span.clone(),
                )),
            },
            Expr::StringInterp(parts) => Expr::StringInterp(
                parts
                    .iter()
                    .map(|part| match part {
                        StringPart::Literal(s) => StringPart::Literal(s.clone()),
                        StringPart::Expr(expr) => StringPart::Expr(Spanned::new(
                            Self::rewrite_expr_for_local_module_classes(
                                &expr.node,
                                module_prefix,
                                class_templates,
                            ),
                            expr.span.clone(),
                        )),
                    })
                    .collect(),
            ),
            Expr::Try(inner) => Expr::Try(Box::new(Spanned::new(
                Self::rewrite_expr_for_local_module_classes(
                    &inner.node,
                    module_prefix,
                    class_templates,
                ),
                inner.span.clone(),
            ))),
            Expr::Borrow(inner) => Expr::Borrow(Box::new(Spanned::new(
                Self::rewrite_expr_for_local_module_classes(
                    &inner.node,
                    module_prefix,
                    class_templates,
                ),
                inner.span.clone(),
            ))),
            Expr::MutBorrow(inner) => Expr::MutBorrow(Box::new(Spanned::new(
                Self::rewrite_expr_for_local_module_classes(
                    &inner.node,
                    module_prefix,
                    class_templates,
                ),
                inner.span.clone(),
            ))),
            Expr::Deref(inner) => Expr::Deref(Box::new(Spanned::new(
                Self::rewrite_expr_for_local_module_classes(
                    &inner.node,
                    module_prefix,
                    class_templates,
                ),
                inner.span.clone(),
            ))),
            Expr::Await(inner) => Expr::Await(Box::new(Spanned::new(
                Self::rewrite_expr_for_local_module_classes(
                    &inner.node,
                    module_prefix,
                    class_templates,
                ),
                inner.span.clone(),
            ))),
            Expr::Require { condition, message } => Expr::Require {
                condition: Box::new(Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &condition.node,
                        module_prefix,
                        class_templates,
                    ),
                    condition.span.clone(),
                )),
                message: message.as_ref().map(|message| {
                    Box::new(Spanned::new(
                        Self::rewrite_expr_for_local_module_classes(
                            &message.node,
                            module_prefix,
                            class_templates,
                        ),
                        message.span.clone(),
                    ))
                }),
            },
            Expr::Range {
                start,
                end,
                inclusive,
            } => Expr::Range {
                start: start.as_ref().map(|start| {
                    Box::new(Spanned::new(
                        Self::rewrite_expr_for_local_module_classes(
                            &start.node,
                            module_prefix,
                            class_templates,
                        ),
                        start.span.clone(),
                    ))
                }),
                end: end.as_ref().map(|end| {
                    Box::new(Spanned::new(
                        Self::rewrite_expr_for_local_module_classes(
                            &end.node,
                            module_prefix,
                            class_templates,
                        ),
                        end.span.clone(),
                    ))
                }),
                inclusive: *inclusive,
            },
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => Expr::If {
                condition: Box::new(Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &condition.node,
                        module_prefix,
                        class_templates,
                    ),
                    condition.span.clone(),
                )),
                then_branch: then_branch
                    .iter()
                    .map(|stmt| {
                        Spanned::new(
                            Self::rewrite_stmt_for_local_module_classes(
                                &stmt.node,
                                module_prefix,
                                class_templates,
                            ),
                            stmt.span.clone(),
                        )
                    })
                    .collect(),
                else_branch: else_branch.as_ref().map(|block| {
                    block
                        .iter()
                        .map(|stmt| {
                            Spanned::new(
                                Self::rewrite_stmt_for_local_module_classes(
                                    &stmt.node,
                                    module_prefix,
                                    class_templates,
                                ),
                                stmt.span.clone(),
                            )
                        })
                        .collect()
                }),
            },
            Expr::Block(block) => Expr::Block(
                block
                    .iter()
                    .map(|stmt| {
                        Spanned::new(
                            Self::rewrite_stmt_for_local_module_classes(
                                &stmt.node,
                                module_prefix,
                                class_templates,
                            ),
                            stmt.span.clone(),
                        )
                    })
                    .collect(),
            ),
            Expr::AsyncBlock(block) => Expr::AsyncBlock(
                block
                    .iter()
                    .map(|stmt| {
                        Spanned::new(
                            Self::rewrite_stmt_for_local_module_classes(
                                &stmt.node,
                                module_prefix,
                                class_templates,
                            ),
                            stmt.span.clone(),
                        )
                    })
                    .collect(),
            ),
            Expr::Match { expr, arms } => Expr::Match {
                expr: Box::new(Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &expr.node,
                        module_prefix,
                        class_templates,
                    ),
                    expr.span.clone(),
                )),
                arms: arms
                    .iter()
                    .map(|arm| MatchArm {
                        pattern: arm.pattern.clone(),
                        body: arm
                            .body
                            .iter()
                            .map(|stmt| {
                                Spanned::new(
                                    Self::rewrite_stmt_for_local_module_classes(
                                        &stmt.node,
                                        module_prefix,
                                        class_templates,
                                    ),
                                    stmt.span.clone(),
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            },
            Expr::Literal(_) | Expr::Ident(_) | Expr::This => expr.clone(),
        }
    }

    fn rewrite_stmt_for_local_module_classes(
        stmt: &Stmt,
        module_prefix: &str,
        class_templates: &HashMap<String, GenericClassTemplate>,
    ) -> Stmt {
        match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
            } => Stmt::Let {
                name: name.clone(),
                ty: Self::rewrite_type_for_local_module_classes(ty, module_prefix, class_templates),
                value: Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &value.node,
                        module_prefix,
                        class_templates,
                    ),
                    value.span.clone(),
                ),
                mutable: *mutable,
            },
            Stmt::Assign { target, value } => Stmt::Assign {
                target: Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &target.node,
                        module_prefix,
                        class_templates,
                    ),
                    target.span.clone(),
                ),
                value: Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &value.node,
                        module_prefix,
                        class_templates,
                    ),
                    value.span.clone(),
                ),
            },
            Stmt::Expr(expr) => Stmt::Expr(Spanned::new(
                Self::rewrite_expr_for_local_module_classes(
                    &expr.node,
                    module_prefix,
                    class_templates,
                ),
                expr.span.clone(),
            )),
            Stmt::Return(expr) => Stmt::Return(expr.as_ref().map(|expr| {
                Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &expr.node,
                        module_prefix,
                        class_templates,
                    ),
                    expr.span.clone(),
                )
            })),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => Stmt::If {
                condition: Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &condition.node,
                        module_prefix,
                        class_templates,
                    ),
                    condition.span.clone(),
                ),
                then_block: then_block
                    .iter()
                    .map(|stmt| {
                        Spanned::new(
                            Self::rewrite_stmt_for_local_module_classes(
                                &stmt.node,
                                module_prefix,
                                class_templates,
                            ),
                            stmt.span.clone(),
                        )
                    })
                    .collect(),
                else_block: else_block.as_ref().map(|block| {
                    block
                        .iter()
                        .map(|stmt| {
                            Spanned::new(
                                Self::rewrite_stmt_for_local_module_classes(
                                    &stmt.node,
                                    module_prefix,
                                    class_templates,
                                ),
                                stmt.span.clone(),
                            )
                        })
                        .collect()
                }),
            },
            Stmt::While { condition, body } => Stmt::While {
                condition: Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &condition.node,
                        module_prefix,
                        class_templates,
                    ),
                    condition.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|stmt| {
                        Spanned::new(
                            Self::rewrite_stmt_for_local_module_classes(
                                &stmt.node,
                                module_prefix,
                                class_templates,
                            ),
                            stmt.span.clone(),
                        )
                    })
                    .collect(),
            },
            Stmt::For {
                var,
                var_type,
                iterable,
                body,
            } => Stmt::For {
                var: var.clone(),
                var_type: var_type.as_ref().map(|ty| {
                    Self::rewrite_type_for_local_module_classes(ty, module_prefix, class_templates)
                }),
                iterable: Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &iterable.node,
                        module_prefix,
                        class_templates,
                    ),
                    iterable.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|stmt| {
                        Spanned::new(
                            Self::rewrite_stmt_for_local_module_classes(
                                &stmt.node,
                                module_prefix,
                                class_templates,
                            ),
                            stmt.span.clone(),
                        )
                    })
                    .collect(),
            },
            Stmt::Match { expr, arms } => Stmt::Match {
                expr: Spanned::new(
                    Self::rewrite_expr_for_local_module_classes(
                        &expr.node,
                        module_prefix,
                        class_templates,
                    ),
                    expr.span.clone(),
                ),
                arms: arms
                    .iter()
                    .map(|arm| MatchArm {
                        pattern: arm.pattern.clone(),
                        body: arm
                            .body
                            .iter()
                            .map(|stmt| {
                                Spanned::new(
                                    Self::rewrite_stmt_for_local_module_classes(
                                        &stmt.node,
                                        module_prefix,
                                        class_templates,
                                    ),
                                    stmt.span.clone(),
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            },
            Stmt::Break => Stmt::Break,
            Stmt::Continue => Stmt::Continue,
        }
    }

    fn type_contains_generic_names(ty: &Type, generic_names: &HashSet<String>) -> bool {
        match ty {
            Type::Named(name) => generic_names.contains(name),
            Type::Generic(name, args) => {
                generic_names.contains(name)
                    || args
                        .iter()
                        .any(|arg| Self::type_contains_generic_names(arg, generic_names))
            }
            Type::Function(params, ret) => {
                params
                    .iter()
                    .any(|p| Self::type_contains_generic_names(p, generic_names))
                    || Self::type_contains_generic_names(ret, generic_names)
            }
            Type::Option(inner)
            | Type::List(inner)
            | Type::Set(inner)
            | Type::Ref(inner)
            | Type::MutRef(inner)
            | Type::Box(inner)
            | Type::Rc(inner)
            | Type::Arc(inner)
            | Type::Ptr(inner)
            | Type::Task(inner)
            | Type::Range(inner) => Self::type_contains_generic_names(inner, generic_names),
            Type::Result(ok, err) | Type::Map(ok, err) => {
                Self::type_contains_generic_names(ok, generic_names)
                    || Self::type_contains_generic_names(err, generic_names)
            }
            _ => false,
        }
    }

    fn collect_generic_class_instantiation_from_type(
        ty: &Type,
        class_templates: &HashMap<String, GenericClassTemplate>,
        import_aliases: &HashMap<String, String>,
        in_scope_generics: &HashSet<String>,
        instantiations: &mut HashMap<String, Vec<Type>>,
    ) {
        let maybe_record_builtin_like =
            |name: &str, args: &[Type], instantiations: &mut HashMap<String, Vec<Type>>| {
                if let Some(resolved_name) =
                    Self::resolve_class_template_name(class_templates, import_aliases, name)
                {
                    if args
                        .iter()
                        .any(|arg| Self::type_contains_generic_names(arg, in_scope_generics))
                    {
                        return;
                    }
                    instantiations
                        .entry(Self::generic_class_spec_name(&resolved_name, args))
                        .or_insert_with(|| args.to_vec());
                }
            };
        match ty {
            Type::Generic(name, args) => {
                for arg in args {
                    Self::collect_generic_class_instantiation_from_type(
                        arg,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
                if let Some(resolved_name) =
                    Self::resolve_class_template_name(class_templates, import_aliases, name)
                {
                    if !args
                        .iter()
                        .any(|arg| Self::type_contains_generic_names(arg, in_scope_generics))
                    {
                        instantiations
                            .entry(Self::generic_class_spec_name(&resolved_name, args))
                            .or_insert_with(|| args.clone());
                    }
                }
            }
            Type::Option(inner) => {
                Self::collect_generic_class_instantiation_from_type(
                    inner,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                maybe_record_builtin_like(
                    "Option",
                    std::slice::from_ref(inner.as_ref()),
                    instantiations,
                );
            }
            Type::List(inner) => {
                Self::collect_generic_class_instantiation_from_type(
                    inner,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                maybe_record_builtin_like(
                    "List",
                    std::slice::from_ref(inner.as_ref()),
                    instantiations,
                );
            }
            Type::Set(inner) => {
                Self::collect_generic_class_instantiation_from_type(
                    inner,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                maybe_record_builtin_like(
                    "Set",
                    std::slice::from_ref(inner.as_ref()),
                    instantiations,
                );
            }
            Type::Box(inner) => {
                Self::collect_generic_class_instantiation_from_type(
                    inner,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                maybe_record_builtin_like(
                    "Box",
                    std::slice::from_ref(inner.as_ref()),
                    instantiations,
                );
            }
            Type::Rc(inner) => {
                Self::collect_generic_class_instantiation_from_type(
                    inner,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                maybe_record_builtin_like(
                    "Rc",
                    std::slice::from_ref(inner.as_ref()),
                    instantiations,
                );
            }
            Type::Arc(inner) => {
                Self::collect_generic_class_instantiation_from_type(
                    inner,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                maybe_record_builtin_like(
                    "Arc",
                    std::slice::from_ref(inner.as_ref()),
                    instantiations,
                );
            }
            Type::Ptr(inner) => {
                Self::collect_generic_class_instantiation_from_type(
                    inner,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                maybe_record_builtin_like(
                    "Ptr",
                    std::slice::from_ref(inner.as_ref()),
                    instantiations,
                );
            }
            Type::Task(inner) => {
                Self::collect_generic_class_instantiation_from_type(
                    inner,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                maybe_record_builtin_like(
                    "Task",
                    std::slice::from_ref(inner.as_ref()),
                    instantiations,
                );
            }
            Type::Range(inner) => {
                Self::collect_generic_class_instantiation_from_type(
                    inner,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                maybe_record_builtin_like(
                    "Range",
                    std::slice::from_ref(inner.as_ref()),
                    instantiations,
                );
            }
            Type::Function(params, ret) => {
                for param in params {
                    Self::collect_generic_class_instantiation_from_type(
                        param,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
                Self::collect_generic_class_instantiation_from_type(
                    ret,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
            }
            Type::Ref(inner) | Type::MutRef(inner) => {
                Self::collect_generic_class_instantiation_from_type(
                    inner,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                )
            }
            Type::Result(ok, err) => {
                Self::collect_generic_class_instantiation_from_type(
                    ok,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                Self::collect_generic_class_instantiation_from_type(
                    err,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                maybe_record_builtin_like(
                    "Result",
                    &[ok.as_ref().clone(), err.as_ref().clone()],
                    instantiations,
                );
            }
            Type::Map(ok, err) => {
                Self::collect_generic_class_instantiation_from_type(
                    ok,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                Self::collect_generic_class_instantiation_from_type(
                    err,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                maybe_record_builtin_like(
                    "Map",
                    &[ok.as_ref().clone(), err.as_ref().clone()],
                    instantiations,
                );
            }
            _ => {}
        }
    }

    fn collect_generic_class_instantiation_from_expr(
        expr: &Expr,
        class_templates: &HashMap<String, GenericClassTemplate>,
        import_aliases: &HashMap<String, String>,
        in_scope_generics: &HashSet<String>,
        instantiations: &mut HashMap<String, Vec<Type>>,
    ) {
        match expr {
            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                Self::collect_generic_class_instantiation_from_expr(
                    &callee.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                for arg in args {
                    Self::collect_generic_class_instantiation_from_expr(
                        &arg.node,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
                for ty in type_args {
                    Self::collect_generic_class_instantiation_from_type(
                        ty,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
                if !type_args.is_empty() {
                    if let Some(path_parts) = flatten_field_chain(&callee.node) {
                        let full_path = path_parts.join(".");
                        if let Some(resolved_name) = Self::resolve_class_template_name(
                            class_templates,
                            import_aliases,
                            &full_path,
                        ) {
                            if !type_args.iter().any(|arg| {
                                Self::type_contains_generic_names(arg, in_scope_generics)
                            }) {
                                instantiations
                                    .entry(Self::generic_class_spec_name(&resolved_name, type_args))
                                    .or_insert_with(|| type_args.clone());
                            }
                        }
                    }
                }
            }
            Expr::GenericFunctionValue { callee, type_args } => {
                Self::collect_generic_class_instantiation_from_expr(
                    &callee.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                for ty in type_args {
                    Self::collect_generic_class_instantiation_from_type(
                        ty,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
                if let Some(template_key) = match &callee.node {
                    Expr::Ident(name) => {
                        Self::resolve_class_template_name(class_templates, import_aliases, name)
                    }
                    _ => flatten_field_chain(&callee.node)
                        .map(|parts| parts.join("."))
                        .and_then(|full_path| {
                            Self::resolve_class_template_name(
                                class_templates,
                                import_aliases,
                                &full_path,
                            )
                        }),
                } {
                    if !type_args
                        .iter()
                        .any(|arg| Self::type_contains_generic_names(arg, in_scope_generics))
                    {
                        instantiations
                            .entry(Self::generic_class_spec_name(&template_key, type_args))
                            .or_insert_with(|| type_args.clone());
                    }
                }
            }
            Expr::Construct { ty, args } => {
                if let Some((name, type_args)) = Self::parse_construct_nominal_type_source(ty) {
                    let resolved_name =
                        Self::resolve_class_template_name(class_templates, import_aliases, &name);
                    if let Some(resolved_name) = resolved_name {
                        if !type_args
                            .iter()
                            .any(|arg| Self::type_contains_generic_names(arg, in_scope_generics))
                        {
                            instantiations
                                .entry(Self::generic_class_spec_name(&resolved_name, &type_args))
                                .or_insert(type_args);
                        }
                    }
                }
                for arg in args {
                    Self::collect_generic_class_instantiation_from_expr(
                        &arg.node,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
            }
            Expr::Binary { left, right, .. } => {
                Self::collect_generic_class_instantiation_from_expr(
                    &left.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                Self::collect_generic_class_instantiation_from_expr(
                    &right.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr)
            | Expr::Field { object: expr, .. } => {
                Self::collect_generic_class_instantiation_from_expr(
                    &expr.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
            }
            Expr::Index { object, index } => {
                Self::collect_generic_class_instantiation_from_expr(
                    &object.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                Self::collect_generic_class_instantiation_from_expr(
                    &index.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
            }
            Expr::Lambda { params, body } => {
                for param in params {
                    Self::collect_generic_class_instantiation_from_type(
                        &param.ty,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
                Self::collect_generic_class_instantiation_from_expr(
                    &body.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
            }
            Expr::Match { expr, arms } => {
                Self::collect_generic_class_instantiation_from_expr(
                    &expr.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                for arm in arms {
                    for stmt in &arm.body {
                        Self::collect_generic_class_instantiation_from_stmt(
                            &stmt.node,
                            class_templates,
                            import_aliases,
                            in_scope_generics,
                            instantiations,
                        );
                    }
                }
            }
            Expr::StringInterp(parts) => {
                for part in parts {
                    if let StringPart::Expr(expr) = part {
                        Self::collect_generic_class_instantiation_from_expr(
                            &expr.node,
                            class_templates,
                            import_aliases,
                            in_scope_generics,
                            instantiations,
                        );
                    }
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::collect_generic_class_instantiation_from_expr(
                    &condition.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                for stmt in then_branch {
                    Self::collect_generic_class_instantiation_from_stmt(
                        &stmt.node,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
                if let Some(block) = else_branch {
                    for stmt in block {
                        Self::collect_generic_class_instantiation_from_stmt(
                            &stmt.node,
                            class_templates,
                            import_aliases,
                            in_scope_generics,
                            instantiations,
                        );
                    }
                }
            }
            Expr::Block(block) | Expr::AsyncBlock(block) => {
                for stmt in block {
                    Self::collect_generic_class_instantiation_from_stmt(
                        &stmt.node,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
            }
            Expr::Require { condition, message } => {
                Self::collect_generic_class_instantiation_from_expr(
                    &condition.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                if let Some(message) = message {
                    Self::collect_generic_class_instantiation_from_expr(
                        &message.node,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
            }
            Expr::Range { start, end, .. } => {
                if let Some(start) = start {
                    Self::collect_generic_class_instantiation_from_expr(
                        &start.node,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
                if let Some(end) = end {
                    Self::collect_generic_class_instantiation_from_expr(
                        &end.node,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
            }
            Expr::Literal(_) | Expr::Ident(_) | Expr::This => {}
        }
    }

    fn collect_generic_class_instantiation_from_stmt(
        stmt: &Stmt,
        class_templates: &HashMap<String, GenericClassTemplate>,
        import_aliases: &HashMap<String, String>,
        in_scope_generics: &HashSet<String>,
        instantiations: &mut HashMap<String, Vec<Type>>,
    ) {
        match stmt {
            Stmt::Let { ty, value, .. } => {
                Self::collect_generic_class_instantiation_from_type(
                    ty,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                Self::collect_generic_class_instantiation_from_expr(
                    &value.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
            }
            Stmt::Assign { target, value } => {
                Self::collect_generic_class_instantiation_from_expr(
                    &target.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                Self::collect_generic_class_instantiation_from_expr(
                    &value.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
            }
            Stmt::Expr(expr) => Self::collect_generic_class_instantiation_from_expr(
                &expr.node,
                class_templates,
                import_aliases,
                in_scope_generics,
                instantiations,
            ),
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    Self::collect_generic_class_instantiation_from_expr(
                        &expr.node,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                Self::collect_generic_class_instantiation_from_expr(
                    &condition.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                for stmt in then_block {
                    Self::collect_generic_class_instantiation_from_stmt(
                        &stmt.node,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
                if let Some(block) = else_block {
                    for stmt in block {
                        Self::collect_generic_class_instantiation_from_stmt(
                            &stmt.node,
                            class_templates,
                            import_aliases,
                            in_scope_generics,
                            instantiations,
                        );
                    }
                }
            }
            Stmt::While { condition, body } => {
                Self::collect_generic_class_instantiation_from_expr(
                    &condition.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                for stmt in body {
                    Self::collect_generic_class_instantiation_from_stmt(
                        &stmt.node,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
            }
            Stmt::For {
                var_type,
                iterable,
                body,
                ..
            } => {
                if let Some(var_type) = var_type {
                    Self::collect_generic_class_instantiation_from_type(
                        var_type,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
                Self::collect_generic_class_instantiation_from_expr(
                    &iterable.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                for stmt in body {
                    Self::collect_generic_class_instantiation_from_stmt(
                        &stmt.node,
                        class_templates,
                        import_aliases,
                        in_scope_generics,
                        instantiations,
                    );
                }
            }
            Stmt::Match { expr, arms } => {
                Self::collect_generic_class_instantiation_from_expr(
                    &expr.node,
                    class_templates,
                    import_aliases,
                    in_scope_generics,
                    instantiations,
                );
                for arm in arms {
                    for stmt in &arm.body {
                        Self::collect_generic_class_instantiation_from_stmt(
                            &stmt.node,
                            class_templates,
                            import_aliases,
                            in_scope_generics,
                            instantiations,
                        );
                    }
                }
            }
            Stmt::Break | Stmt::Continue => {}
        }
    }

    fn rewrite_expr_generic_calls(
        expr: &Expr,
        templates: &GenericRewriteTemplates<'_>,
        outputs: &mut GenericRewriteOutputs<'_>,
    ) -> Result<Expr> {
        if !Self::expr_needs_generic_call_rewrite(expr) {
            return Ok(expr.clone());
        }
        let function_templates = templates.function_templates;
        let method_templates = templates.method_templates;
        let class_templates = templates.class_templates;
        let import_aliases = templates.import_aliases;

        Ok(match expr {
            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                let rewritten_callee = Spanned::new(
                    Self::rewrite_expr_generic_calls(&callee.node, templates, outputs)?,
                    callee.span.clone(),
                );
                let rewritten_args = args
                    .iter()
                    .map(|arg| {
                        Ok(Spanned::new(
                            Self::rewrite_expr_generic_calls(&arg.node, templates, outputs)?,
                            arg.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;

                if !type_args.is_empty() {
                    if let Expr::Field { object, field } = &callee.node {
                        if let Expr::Ident(owner_name) = &object.node {
                            match (owner_name.as_str(), field.as_str()) {
                                ("Option", "some" | "none") => {
                                    return Err(CodegenError::new(
                                        "Option static methods do not accept explicit type arguments",
                                    ));
                                }
                                ("Result", "ok" | "error") => {
                                    return Err(CodegenError::new(
                                        "Result static methods do not accept explicit type arguments",
                                    ));
                                }
                                _ => {}
                            }
                        }
                    }
                    if let Expr::Field { field, .. } = &callee.node {
                        if let Some(candidates) = method_templates.get(field) {
                            let eligible_templates: Vec<_> = candidates
                                .iter()
                                .filter(|template| {
                                    template.func.generic_params.len() == type_args.len()
                                })
                                .collect();
                            if !eligible_templates.is_empty() {
                                let suffix = type_args
                                    .iter()
                                    .map(Self::type_specialization_suffix)
                                    .collect::<Vec<_>>()
                                    .join("_");
                                for template in &eligible_templates {
                                    let default_owner = template
                                        .owner_class
                                        .clone()
                                        .unwrap_or_else(|| field.clone());
                                    let (owner_key, mut bindings) = template
                                        .owner_class
                                        .as_deref()
                                        .and_then(|owner_class| {
                                            if let Expr::Field { object, .. } = &callee.node {
                                                Self::explicit_receiver_class_bindings(
                                                    &object.node,
                                                    owner_class,
                                                    class_templates,
                                                )
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or_else(|| (default_owner.clone(), HashMap::new()));
                                    let spec_name = if owner_key == default_owner {
                                        format!("{}__spec__{}", field, suffix)
                                    } else {
                                        format!(
                                            "{}__recv__{}__spec__{}",
                                            field,
                                            owner_key.replace("__", "_"),
                                            suffix
                                        )
                                    };
                                    let emitted_key = format!("{}::{}", owner_key, spec_name);
                                    if !outputs.emitted.insert(emitted_key) {
                                        continue;
                                    }

                                    for (param, ty) in
                                        template.func.generic_params.iter().zip(type_args.iter())
                                    {
                                        bindings.insert(param.name.clone(), ty.clone());
                                    }

                                    let mut spec_func = template.func.clone();
                                    spec_func.name = spec_name.clone();
                                    spec_func.generic_params.clear();
                                    for param in &mut spec_func.params {
                                        param.ty = Self::substitute_type(&param.ty, &bindings);
                                    }
                                    spec_func.return_type =
                                        Self::substitute_type(&spec_func.return_type, &bindings);
                                    spec_func.body = spec_func
                                        .body
                                        .iter()
                                        .map(|s| {
                                            Spanned::new(
                                                Self::substitute_stmt_types(&s.node, &bindings),
                                                s.span.clone(),
                                            )
                                        })
                                        .collect();
                                    if let Some(owner_class) = &template.owner_class {
                                        if let Some(module_prefix) =
                                            Self::module_prefix_for_owner_class(owner_class)
                                        {
                                            for param in &mut spec_func.params {
                                                param.ty =
                                                    Self::rewrite_type_for_local_module_classes(
                                                        &param.ty,
                                                        module_prefix,
                                                        class_templates,
                                                    );
                                            }
                                            spec_func.return_type =
                                                Self::rewrite_type_for_local_module_classes(
                                                    &spec_func.return_type,
                                                    module_prefix,
                                                    class_templates,
                                                );
                                            spec_func.body = spec_func
                                                .body
                                                .iter()
                                                .map(|stmt| {
                                                    Spanned::new(
                                                        Self::rewrite_stmt_for_local_module_classes(
                                                            &stmt.node,
                                                            module_prefix,
                                                            class_templates,
                                                        ),
                                                        stmt.span.clone(),
                                                    )
                                                })
                                                .collect();
                                        }
                                    }

                                    let rewritten_body = spec_func
                                        .body
                                        .iter()
                                        .map(|s| {
                                            Ok(Spanned::new(
                                                Self::rewrite_stmt_generic_calls(
                                                    &s.node, templates, outputs,
                                                )?,
                                                s.span.clone(),
                                            ))
                                        })
                                        .collect::<Result<Vec<_>>>()?;
                                    spec_func.body = rewritten_body;
                                    if template.owner_class.is_some() {
                                        outputs
                                            .generated_methods
                                            .entry(owner_key.clone())
                                            .or_default()
                                            .push(spec_func.clone());
                                        if owner_key != default_owner {
                                            outputs
                                                .generated_methods
                                                .entry(default_owner.clone())
                                                .or_default()
                                                .push(spec_func);
                                        }
                                    }
                                }

                                if let Expr::Field { object, .. } = &rewritten_callee.node {
                                    let default_owner = eligible_templates[0]
                                        .owner_class
                                        .clone()
                                        .unwrap_or_else(|| field.clone());
                                    let owner_key = eligible_templates[0]
                                        .owner_class
                                        .as_deref()
                                        .and_then(|owner_class| {
                                            Self::explicit_receiver_class_bindings(
                                                &object.node,
                                                owner_class,
                                                class_templates,
                                            )
                                            .map(|(owner_key, _)| owner_key)
                                        })
                                        .unwrap_or(default_owner.clone());
                                    let spec_name = if owner_key == default_owner {
                                        format!("{}__spec__{}", field, suffix)
                                    } else {
                                        format!(
                                            "{}__recv__{}__spec__{}",
                                            field,
                                            owner_key.replace("__", "_"),
                                            suffix
                                        )
                                    };
                                    return Ok(Expr::Call {
                                        callee: Box::new(Spanned::new(
                                            Expr::Field {
                                                object: object.clone(),
                                                field: spec_name,
                                            },
                                            rewritten_callee.span,
                                        )),
                                        args: rewritten_args,
                                        type_args: Vec::new(),
                                    });
                                }
                            }
                        }
                    }

                    if let Some(template_key) = Self::resolve_function_template_key(
                        function_templates,
                        import_aliases,
                        &callee.node,
                    ) {
                        if let Some(template) = function_templates.get(&template_key) {
                            if template.func.generic_params.len() != type_args.len() {
                                return Ok(Expr::Call {
                                    callee: Box::new(rewritten_callee),
                                    args: rewritten_args,
                                    type_args: type_args.clone(),
                                });
                            }
                            let suffix = type_args
                                .iter()
                                .map(Self::type_specialization_suffix)
                                .collect::<Vec<_>>()
                                .join("_");
                            let spec_name = format!("{}__spec__{}", template_key, suffix);

                            if outputs.emitted.insert(spec_name.clone()) {
                                let mut bindings: HashMap<String, Type> = HashMap::new();
                                for (param, ty) in
                                    template.func.generic_params.iter().zip(type_args.iter())
                                {
                                    bindings.insert(param.name.clone(), ty.clone());
                                }

                                let mut spec_func = template.func.clone();
                                spec_func.name = spec_name.clone();
                                spec_func.generic_params.clear();
                                for param in &mut spec_func.params {
                                    param.ty = Self::substitute_type(&param.ty, &bindings);
                                }
                                spec_func.return_type =
                                    Self::substitute_type(&spec_func.return_type, &bindings);
                                spec_func.body = spec_func
                                    .body
                                    .iter()
                                    .map(|s| {
                                        Spanned::new(
                                            Self::substitute_stmt_types(&s.node, &bindings),
                                            s.span.clone(),
                                        )
                                    })
                                    .collect();
                                if let Some((module_prefix, _)) = template_key.rsplit_once("__") {
                                    for param in &mut spec_func.params {
                                        param.ty = Self::rewrite_type_for_local_module_classes(
                                            &param.ty,
                                            module_prefix,
                                            class_templates,
                                        );
                                    }
                                    spec_func.return_type =
                                        Self::rewrite_type_for_local_module_classes(
                                            &spec_func.return_type,
                                            module_prefix,
                                            class_templates,
                                        );
                                    spec_func.body = spec_func
                                        .body
                                        .iter()
                                        .map(|stmt| {
                                            Spanned::new(
                                                Self::rewrite_stmt_for_local_module_classes(
                                                    &stmt.node,
                                                    module_prefix,
                                                    class_templates,
                                                ),
                                                stmt.span.clone(),
                                            )
                                        })
                                        .collect();
                                }

                                let rewritten_body = spec_func
                                    .body
                                    .iter()
                                    .map(|s| {
                                        Ok(Spanned::new(
                                            Self::rewrite_stmt_generic_calls(
                                                &s.node, templates, outputs,
                                            )?,
                                            s.span.clone(),
                                        ))
                                    })
                                    .collect::<Result<Vec<_>>>()?;
                                spec_func.body = rewritten_body;
                                outputs.generated_functions.push(Spanned::new(
                                    Decl::Function(spec_func),
                                    template.span.clone(),
                                ));
                            }

                            Expr::Call {
                                callee: Box::new(Spanned::new(
                                    Expr::Ident(spec_name),
                                    rewritten_callee.span,
                                )),
                                args: rewritten_args,
                                type_args: Vec::new(),
                            }
                        } else {
                            Expr::Call {
                                callee: Box::new(rewritten_callee),
                                args: rewritten_args,
                                type_args: type_args.clone(),
                            }
                        }
                    } else {
                        Expr::Call {
                            callee: Box::new(rewritten_callee),
                            args: rewritten_args,
                            type_args: type_args.clone(),
                        }
                    }
                } else {
                    Expr::Call {
                        callee: Box::new(rewritten_callee),
                        args: rewritten_args,
                        type_args: type_args.clone(),
                    }
                }
            }
            Expr::GenericFunctionValue { callee, type_args } => {
                let rewritten_callee = Spanned::new(
                    Self::rewrite_expr_generic_calls(&callee.node, templates, outputs)?,
                    callee.span.clone(),
                );

                if let Some(template_key) = Self::resolve_function_template_key(
                    function_templates,
                    import_aliases,
                    &callee.node,
                ) {
                    if let Some(template) = function_templates.get(&template_key) {
                        if template.func.generic_params.len() != type_args.len() {
                            Expr::GenericFunctionValue {
                                callee: Box::new(rewritten_callee),
                                type_args: type_args.clone(),
                            }
                        } else {
                            let suffix = type_args
                                .iter()
                                .map(Self::type_specialization_suffix)
                                .collect::<Vec<_>>()
                                .join("_");
                            let spec_name = format!("{}__spec__{}", template_key, suffix);

                            if outputs.emitted.insert(spec_name.clone()) {
                                let mut bindings: HashMap<String, Type> = HashMap::new();
                                for (param, ty) in
                                    template.func.generic_params.iter().zip(type_args.iter())
                                {
                                    bindings.insert(param.name.clone(), ty.clone());
                                }

                                let mut spec_func = template.func.clone();
                                spec_func.name = spec_name.clone();
                                spec_func.generic_params.clear();
                                for param in &mut spec_func.params {
                                    param.ty = Self::substitute_type(&param.ty, &bindings);
                                }
                                spec_func.return_type =
                                    Self::substitute_type(&spec_func.return_type, &bindings);
                                spec_func.body = spec_func
                                    .body
                                    .iter()
                                    .map(|s| {
                                        Spanned::new(
                                            Self::substitute_stmt_types(&s.node, &bindings),
                                            s.span.clone(),
                                        )
                                    })
                                    .collect();
                                if let Some((module_prefix, _)) = template_key.rsplit_once("__") {
                                    for param in &mut spec_func.params {
                                        param.ty = Self::rewrite_type_for_local_module_classes(
                                            &param.ty,
                                            module_prefix,
                                            class_templates,
                                        );
                                    }
                                    spec_func.return_type =
                                        Self::rewrite_type_for_local_module_classes(
                                            &spec_func.return_type,
                                            module_prefix,
                                            class_templates,
                                        );
                                    spec_func.body = spec_func
                                        .body
                                        .iter()
                                        .map(|stmt| {
                                            Spanned::new(
                                                Self::rewrite_stmt_for_local_module_classes(
                                                    &stmt.node,
                                                    module_prefix,
                                                    class_templates,
                                                ),
                                                stmt.span.clone(),
                                            )
                                        })
                                        .collect();
                                }

                                let rewritten_body = spec_func
                                    .body
                                    .iter()
                                    .map(|s| {
                                        Ok(Spanned::new(
                                            Self::rewrite_stmt_generic_calls(
                                                &s.node, templates, outputs,
                                            )?,
                                            s.span.clone(),
                                        ))
                                    })
                                    .collect::<Result<Vec<_>>>()?;
                                spec_func.body = rewritten_body;
                                outputs.generated_functions.push(Spanned::new(
                                    Decl::Function(spec_func),
                                    template.span.clone(),
                                ));
                            }

                            Expr::Ident(spec_name)
                        }
                    } else {
                        Expr::GenericFunctionValue {
                            callee: Box::new(rewritten_callee),
                            type_args: type_args.clone(),
                        }
                    }
                } else if let Some(template_key) =
                    Self::resolve_class_template_key(class_templates, import_aliases, &callee.node)
                {
                    Expr::GenericFunctionValue {
                        callee: Box::new(Spanned::new(
                            Expr::Ident(template_key),
                            rewritten_callee.span,
                        )),
                        type_args: type_args.clone(),
                    }
                } else {
                    Expr::GenericFunctionValue {
                        callee: Box::new(rewritten_callee),
                        type_args: type_args.clone(),
                    }
                }
            }
            Expr::Binary { op, left, right } => Expr::Binary {
                op: *op,
                left: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&left.node, templates, outputs)?,
                    left.span.clone(),
                )),
                right: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&right.node, templates, outputs)?,
                    right.span.clone(),
                )),
            },
            Expr::Unary { op, expr } => Expr::Unary {
                op: *op,
                expr: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&expr.node, templates, outputs)?,
                    expr.span.clone(),
                )),
            },
            Expr::Field { object, field } => Expr::Field {
                object: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&object.node, templates, outputs)?,
                    object.span.clone(),
                )),
                field: field.clone(),
            },
            Expr::Index { object, index } => Expr::Index {
                object: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&object.node, templates, outputs)?,
                    object.span.clone(),
                )),
                index: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&index.node, templates, outputs)?,
                    index.span.clone(),
                )),
            },
            Expr::Lambda { params, body } => Expr::Lambda {
                params: params.clone(),
                body: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&body.node, templates, outputs)?,
                    body.span.clone(),
                )),
            },
            Expr::Match { expr, arms } => Expr::Match {
                expr: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&expr.node, templates, outputs)?,
                    expr.span.clone(),
                )),
                arms: arms
                    .iter()
                    .map(|arm| {
                        Ok(MatchArm {
                            pattern: arm.pattern.clone(),
                            body: arm
                                .body
                                .iter()
                                .map(|s| {
                                    Ok(Spanned::new(
                                        Self::rewrite_stmt_generic_calls(
                                            &s.node, templates, outputs,
                                        )?,
                                        s.span.clone(),
                                    ))
                                })
                                .collect::<Result<Vec<_>>>()?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
            Expr::StringInterp(parts) => Expr::StringInterp(
                parts
                    .iter()
                    .map(|p| match p {
                        StringPart::Literal(s) => Ok(StringPart::Literal(s.clone())),
                        StringPart::Expr(e) => Ok(StringPart::Expr(Spanned::new(
                            Self::rewrite_expr_generic_calls(&e.node, templates, outputs)?,
                            e.span.clone(),
                        ))),
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            Expr::Try(inner) => Expr::Try(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(&inner.node, templates, outputs)?,
                inner.span.clone(),
            ))),
            Expr::Borrow(inner) => Expr::Borrow(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(&inner.node, templates, outputs)?,
                inner.span.clone(),
            ))),
            Expr::MutBorrow(inner) => Expr::MutBorrow(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(&inner.node, templates, outputs)?,
                inner.span.clone(),
            ))),
            Expr::Deref(inner) => Expr::Deref(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(&inner.node, templates, outputs)?,
                inner.span.clone(),
            ))),
            Expr::Await(inner) => Expr::Await(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(&inner.node, templates, outputs)?,
                inner.span.clone(),
            ))),
            Expr::AsyncBlock(block) => Expr::AsyncBlock(
                block
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(&s.node, templates, outputs)?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            Expr::Require { condition, message } => Expr::Require {
                condition: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&condition.node, templates, outputs)?,
                    condition.span.clone(),
                )),
                message: message
                    .as_ref()
                    .map(|m| {
                        Ok(Box::new(Spanned::new(
                            Self::rewrite_expr_generic_calls(&m.node, templates, outputs)?,
                            m.span.clone(),
                        )))
                    })
                    .transpose()?,
            },
            Expr::Range {
                start,
                end,
                inclusive,
            } => Expr::Range {
                start: start
                    .as_ref()
                    .map(|s| {
                        Ok(Box::new(Spanned::new(
                            Self::rewrite_expr_generic_calls(&s.node, templates, outputs)?,
                            s.span.clone(),
                        )))
                    })
                    .transpose()?,
                end: end
                    .as_ref()
                    .map(|e| {
                        Ok(Box::new(Spanned::new(
                            Self::rewrite_expr_generic_calls(&e.node, templates, outputs)?,
                            e.span.clone(),
                        )))
                    })
                    .transpose()?,
                inclusive: *inclusive,
            },
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => Expr::If {
                condition: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(&condition.node, templates, outputs)?,
                    condition.span.clone(),
                )),
                then_branch: then_branch
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(&s.node, templates, outputs)?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
                else_branch: else_branch
                    .as_ref()
                    .map(|blk| {
                        blk.iter()
                            .map(|s| {
                                Ok(Spanned::new(
                                    Self::rewrite_stmt_generic_calls(&s.node, templates, outputs)?,
                                    s.span.clone(),
                                ))
                            })
                            .collect::<Result<Vec<_>>>()
                    })
                    .transpose()?,
            },
            Expr::Block(block) => Expr::Block(
                block
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(&s.node, templates, outputs)?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            _ => expr.clone(),
        })
    }

    fn rewrite_specialized_class_type(ty: &Type, emitted_classes: &HashSet<String>) -> Type {
        match ty {
            Type::Generic(name, args) => {
                let rewritten_args = args
                    .iter()
                    .map(|arg| Self::rewrite_specialized_class_type(arg, emitted_classes))
                    .collect::<Vec<_>>();
                let spec_name = Self::generic_class_spec_name(name, &rewritten_args);
                if emitted_classes.contains(&spec_name) {
                    Type::Named(spec_name)
                } else {
                    Type::Generic(name.clone(), rewritten_args)
                }
            }
            Type::Function(params, ret) => Type::Function(
                params
                    .iter()
                    .map(|param| Self::rewrite_specialized_class_type(param, emitted_classes))
                    .collect(),
                Box::new(Self::rewrite_specialized_class_type(ret, emitted_classes)),
            ),
            Type::Option(inner) => {
                let rewritten = Self::rewrite_specialized_class_type(inner, emitted_classes);
                let spec_name =
                    Self::generic_class_spec_name("Option", std::slice::from_ref(&rewritten));
                if emitted_classes.contains(&spec_name) {
                    Type::Named(spec_name)
                } else {
                    Type::Option(Box::new(rewritten))
                }
            }
            Type::Result(ok, err) => {
                let ok = Self::rewrite_specialized_class_type(ok, emitted_classes);
                let err = Self::rewrite_specialized_class_type(err, emitted_classes);
                let spec_name = Self::generic_class_spec_name("Result", &[ok.clone(), err.clone()]);
                if emitted_classes.contains(&spec_name) {
                    Type::Named(spec_name)
                } else {
                    Type::Result(Box::new(ok), Box::new(err))
                }
            }
            Type::List(inner) => {
                let rewritten = Self::rewrite_specialized_class_type(inner, emitted_classes);
                let spec_name =
                    Self::generic_class_spec_name("List", std::slice::from_ref(&rewritten));
                if emitted_classes.contains(&spec_name) {
                    Type::Named(spec_name)
                } else {
                    Type::List(Box::new(rewritten))
                }
            }
            Type::Map(k, v) => {
                let k = Self::rewrite_specialized_class_type(k, emitted_classes);
                let v = Self::rewrite_specialized_class_type(v, emitted_classes);
                let spec_name = Self::generic_class_spec_name("Map", &[k.clone(), v.clone()]);
                if emitted_classes.contains(&spec_name) {
                    Type::Named(spec_name)
                } else {
                    Type::Map(Box::new(k), Box::new(v))
                }
            }
            Type::Set(inner) => {
                let rewritten = Self::rewrite_specialized_class_type(inner, emitted_classes);
                let spec_name =
                    Self::generic_class_spec_name("Set", std::slice::from_ref(&rewritten));
                if emitted_classes.contains(&spec_name) {
                    Type::Named(spec_name)
                } else {
                    Type::Set(Box::new(rewritten))
                }
            }
            Type::Ref(inner) => Type::Ref(Box::new(Self::rewrite_specialized_class_type(
                inner,
                emitted_classes,
            ))),
            Type::MutRef(inner) => Type::MutRef(Box::new(Self::rewrite_specialized_class_type(
                inner,
                emitted_classes,
            ))),
            Type::Box(inner) => {
                let rewritten = Self::rewrite_specialized_class_type(inner, emitted_classes);
                let spec_name =
                    Self::generic_class_spec_name("Box", std::slice::from_ref(&rewritten));
                if emitted_classes.contains(&spec_name) {
                    Type::Named(spec_name)
                } else {
                    Type::Box(Box::new(rewritten))
                }
            }
            Type::Rc(inner) => {
                let rewritten = Self::rewrite_specialized_class_type(inner, emitted_classes);
                let spec_name =
                    Self::generic_class_spec_name("Rc", std::slice::from_ref(&rewritten));
                if emitted_classes.contains(&spec_name) {
                    Type::Named(spec_name)
                } else {
                    Type::Rc(Box::new(rewritten))
                }
            }
            Type::Arc(inner) => {
                let rewritten = Self::rewrite_specialized_class_type(inner, emitted_classes);
                let spec_name =
                    Self::generic_class_spec_name("Arc", std::slice::from_ref(&rewritten));
                if emitted_classes.contains(&spec_name) {
                    Type::Named(spec_name)
                } else {
                    Type::Arc(Box::new(rewritten))
                }
            }
            Type::Ptr(inner) => {
                let rewritten = Self::rewrite_specialized_class_type(inner, emitted_classes);
                let spec_name =
                    Self::generic_class_spec_name("Ptr", std::slice::from_ref(&rewritten));
                if emitted_classes.contains(&spec_name) {
                    Type::Named(spec_name)
                } else {
                    Type::Ptr(Box::new(rewritten))
                }
            }
            Type::Task(inner) => {
                let rewritten = Self::rewrite_specialized_class_type(inner, emitted_classes);
                let spec_name =
                    Self::generic_class_spec_name("Task", std::slice::from_ref(&rewritten));
                if emitted_classes.contains(&spec_name) {
                    Type::Named(spec_name)
                } else {
                    Type::Task(Box::new(rewritten))
                }
            }
            Type::Range(inner) => {
                let rewritten = Self::rewrite_specialized_class_type(inner, emitted_classes);
                let spec_name =
                    Self::generic_class_spec_name("Range", std::slice::from_ref(&rewritten));
                if emitted_classes.contains(&spec_name) {
                    Type::Named(spec_name)
                } else {
                    Type::Range(Box::new(rewritten))
                }
            }
            _ => ty.clone(),
        }
    }

    fn rewrite_specialized_class_expr(expr: &Expr, emitted_classes: &HashSet<String>) -> Expr {
        match expr {
            Expr::Call {
                callee,
                args,
                type_args,
            } => Expr::Call {
                callee: Box::new(Spanned::new(
                    Self::rewrite_specialized_class_expr(&callee.node, emitted_classes),
                    callee.span.clone(),
                )),
                args: args
                    .iter()
                    .map(|arg| {
                        Spanned::new(
                            Self::rewrite_specialized_class_expr(&arg.node, emitted_classes),
                            arg.span.clone(),
                        )
                    })
                    .collect(),
                type_args: type_args
                    .iter()
                    .map(|ty| Self::rewrite_specialized_class_type(ty, emitted_classes))
                    .collect(),
            },
            Expr::GenericFunctionValue { callee, type_args } => Expr::GenericFunctionValue {
                callee: Box::new(Spanned::new(
                    Self::rewrite_specialized_class_expr(&callee.node, emitted_classes),
                    callee.span.clone(),
                )),
                type_args: type_args
                    .iter()
                    .map(|ty| Self::rewrite_specialized_class_type(ty, emitted_classes))
                    .collect(),
            },
            Expr::Construct { ty, args } => {
                let rewritten_ty = parse_type_source(ty)
                    .ok()
                    .map(|parsed| Self::rewrite_specialized_class_type(&parsed, emitted_classes))
                    .and_then(|rewritten| match rewritten {
                        Type::Named(name) => Some(name),
                        Type::Generic(name, args) => {
                            let spec_name = Self::generic_class_spec_name(&name, &args);
                            emitted_classes.contains(&spec_name).then_some(spec_name)
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| ty.clone());
                Expr::Construct {
                    ty: rewritten_ty,
                    args: args
                        .iter()
                        .map(|arg| {
                            Spanned::new(
                                Self::rewrite_specialized_class_expr(&arg.node, emitted_classes),
                                arg.span.clone(),
                            )
                        })
                        .collect(),
                }
            }
            Expr::Binary { op, left, right } => Expr::Binary {
                op: *op,
                left: Box::new(Spanned::new(
                    Self::rewrite_specialized_class_expr(&left.node, emitted_classes),
                    left.span.clone(),
                )),
                right: Box::new(Spanned::new(
                    Self::rewrite_specialized_class_expr(&right.node, emitted_classes),
                    right.span.clone(),
                )),
            },
            Expr::Unary { op, expr } => Expr::Unary {
                op: *op,
                expr: Box::new(Spanned::new(
                    Self::rewrite_specialized_class_expr(&expr.node, emitted_classes),
                    expr.span.clone(),
                )),
            },
            Expr::Field { object, field } => Expr::Field {
                object: Box::new(Spanned::new(
                    Self::rewrite_specialized_class_expr(&object.node, emitted_classes),
                    object.span.clone(),
                )),
                field: field.clone(),
            },
            Expr::Index { object, index } => Expr::Index {
                object: Box::new(Spanned::new(
                    Self::rewrite_specialized_class_expr(&object.node, emitted_classes),
                    object.span.clone(),
                )),
                index: Box::new(Spanned::new(
                    Self::rewrite_specialized_class_expr(&index.node, emitted_classes),
                    index.span.clone(),
                )),
            },
            Expr::Lambda { params, body } => Expr::Lambda {
                params: params
                    .iter()
                    .map(|param| Parameter {
                        name: param.name.clone(),
                        ty: Self::rewrite_specialized_class_type(&param.ty, emitted_classes),
                        mutable: param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                        mode: param.mode,
                    })
                    .collect(),
                body: Box::new(Spanned::new(
                    Self::rewrite_specialized_class_expr(&body.node, emitted_classes),
                    body.span.clone(),
                )),
            },
            Expr::StringInterp(parts) => Expr::StringInterp(
                parts
                    .iter()
                    .map(|part| match part {
                        StringPart::Literal(text) => StringPart::Literal(text.clone()),
                        StringPart::Expr(expr) => StringPart::Expr(Spanned::new(
                            Self::rewrite_specialized_class_expr(&expr.node, emitted_classes),
                            expr.span.clone(),
                        )),
                    })
                    .collect(),
            ),
            Expr::Try(inner) => Expr::Try(Box::new(Spanned::new(
                Self::rewrite_specialized_class_expr(&inner.node, emitted_classes),
                inner.span.clone(),
            ))),
            Expr::Borrow(inner) => Expr::Borrow(Box::new(Spanned::new(
                Self::rewrite_specialized_class_expr(&inner.node, emitted_classes),
                inner.span.clone(),
            ))),
            Expr::MutBorrow(inner) => Expr::MutBorrow(Box::new(Spanned::new(
                Self::rewrite_specialized_class_expr(&inner.node, emitted_classes),
                inner.span.clone(),
            ))),
            Expr::Deref(inner) => Expr::Deref(Box::new(Spanned::new(
                Self::rewrite_specialized_class_expr(&inner.node, emitted_classes),
                inner.span.clone(),
            ))),
            Expr::Await(inner) => Expr::Await(Box::new(Spanned::new(
                Self::rewrite_specialized_class_expr(&inner.node, emitted_classes),
                inner.span.clone(),
            ))),
            Expr::Require { condition, message } => Expr::Require {
                condition: Box::new(Spanned::new(
                    Self::rewrite_specialized_class_expr(&condition.node, emitted_classes),
                    condition.span.clone(),
                )),
                message: message.as_ref().map(|msg| {
                    Box::new(Spanned::new(
                        Self::rewrite_specialized_class_expr(&msg.node, emitted_classes),
                        msg.span.clone(),
                    ))
                }),
            },
            Expr::Range {
                start,
                end,
                inclusive,
            } => Expr::Range {
                start: start.as_ref().map(|expr| {
                    Box::new(Spanned::new(
                        Self::rewrite_specialized_class_expr(&expr.node, emitted_classes),
                        expr.span.clone(),
                    ))
                }),
                end: end.as_ref().map(|expr| {
                    Box::new(Spanned::new(
                        Self::rewrite_specialized_class_expr(&expr.node, emitted_classes),
                        expr.span.clone(),
                    ))
                }),
                inclusive: *inclusive,
            },
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => Expr::If {
                condition: Box::new(Spanned::new(
                    Self::rewrite_specialized_class_expr(&condition.node, emitted_classes),
                    condition.span.clone(),
                )),
                then_branch: then_branch
                    .iter()
                    .map(|stmt| {
                        Spanned::new(
                            Self::rewrite_specialized_class_stmt(&stmt.node, emitted_classes),
                            stmt.span.clone(),
                        )
                    })
                    .collect(),
                else_branch: else_branch.as_ref().map(|block| {
                    block
                        .iter()
                        .map(|stmt| {
                            Spanned::new(
                                Self::rewrite_specialized_class_stmt(&stmt.node, emitted_classes),
                                stmt.span.clone(),
                            )
                        })
                        .collect()
                }),
            },
            Expr::Block(block) => Expr::Block(
                block
                    .iter()
                    .map(|stmt| {
                        Spanned::new(
                            Self::rewrite_specialized_class_stmt(&stmt.node, emitted_classes),
                            stmt.span.clone(),
                        )
                    })
                    .collect(),
            ),
            Expr::AsyncBlock(block) => Expr::AsyncBlock(
                block
                    .iter()
                    .map(|stmt| {
                        Spanned::new(
                            Self::rewrite_specialized_class_stmt(&stmt.node, emitted_classes),
                            stmt.span.clone(),
                        )
                    })
                    .collect(),
            ),
            Expr::Match { expr, arms } => Expr::Match {
                expr: Box::new(Spanned::new(
                    Self::rewrite_specialized_class_expr(&expr.node, emitted_classes),
                    expr.span.clone(),
                )),
                arms: arms
                    .iter()
                    .map(|arm| MatchArm {
                        pattern: arm.pattern.clone(),
                        body: arm
                            .body
                            .iter()
                            .map(|stmt| {
                                Spanned::new(
                                    Self::rewrite_specialized_class_stmt(
                                        &stmt.node,
                                        emitted_classes,
                                    ),
                                    stmt.span.clone(),
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            },
            Expr::Literal(_) | Expr::Ident(_) | Expr::This => expr.clone(),
        }
    }

    fn rewrite_specialized_class_stmt(stmt: &Stmt, emitted_classes: &HashSet<String>) -> Stmt {
        match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
            } => Stmt::Let {
                name: name.clone(),
                ty: Self::rewrite_specialized_class_type(ty, emitted_classes),
                value: Spanned::new(
                    Self::rewrite_specialized_class_expr(&value.node, emitted_classes),
                    value.span.clone(),
                ),
                mutable: *mutable,
            },
            Stmt::Assign { target, value } => Stmt::Assign {
                target: Spanned::new(
                    Self::rewrite_specialized_class_expr(&target.node, emitted_classes),
                    target.span.clone(),
                ),
                value: Spanned::new(
                    Self::rewrite_specialized_class_expr(&value.node, emitted_classes),
                    value.span.clone(),
                ),
            },
            Stmt::Expr(expr) => Stmt::Expr(Spanned::new(
                Self::rewrite_specialized_class_expr(&expr.node, emitted_classes),
                expr.span.clone(),
            )),
            Stmt::Return(expr) => Stmt::Return(expr.as_ref().map(|expr| {
                Spanned::new(
                    Self::rewrite_specialized_class_expr(&expr.node, emitted_classes),
                    expr.span.clone(),
                )
            })),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => Stmt::If {
                condition: Spanned::new(
                    Self::rewrite_specialized_class_expr(&condition.node, emitted_classes),
                    condition.span.clone(),
                ),
                then_block: then_block
                    .iter()
                    .map(|stmt| {
                        Spanned::new(
                            Self::rewrite_specialized_class_stmt(&stmt.node, emitted_classes),
                            stmt.span.clone(),
                        )
                    })
                    .collect(),
                else_block: else_block.as_ref().map(|block| {
                    block
                        .iter()
                        .map(|stmt| {
                            Spanned::new(
                                Self::rewrite_specialized_class_stmt(&stmt.node, emitted_classes),
                                stmt.span.clone(),
                            )
                        })
                        .collect()
                }),
            },
            Stmt::While { condition, body } => Stmt::While {
                condition: Spanned::new(
                    Self::rewrite_specialized_class_expr(&condition.node, emitted_classes),
                    condition.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|stmt| {
                        Spanned::new(
                            Self::rewrite_specialized_class_stmt(&stmt.node, emitted_classes),
                            stmt.span.clone(),
                        )
                    })
                    .collect(),
            },
            Stmt::For {
                var,
                var_type,
                iterable,
                body,
            } => Stmt::For {
                var: var.clone(),
                var_type: var_type
                    .as_ref()
                    .map(|ty| Self::rewrite_specialized_class_type(ty, emitted_classes)),
                iterable: Spanned::new(
                    Self::rewrite_specialized_class_expr(&iterable.node, emitted_classes),
                    iterable.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|stmt| {
                        Spanned::new(
                            Self::rewrite_specialized_class_stmt(&stmt.node, emitted_classes),
                            stmt.span.clone(),
                        )
                    })
                    .collect(),
            },
            Stmt::Match { expr, arms } => Stmt::Match {
                expr: Spanned::new(
                    Self::rewrite_specialized_class_expr(&expr.node, emitted_classes),
                    expr.span.clone(),
                ),
                arms: arms
                    .iter()
                    .map(|arm| MatchArm {
                        pattern: arm.pattern.clone(),
                        body: arm
                            .body
                            .iter()
                            .map(|stmt| {
                                Spanned::new(
                                    Self::rewrite_specialized_class_stmt(
                                        &stmt.node,
                                        emitted_classes,
                                    ),
                                    stmt.span.clone(),
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            },
            Stmt::Break => Stmt::Break,
            Stmt::Continue => Stmt::Continue,
        }
    }

    fn collect_generic_class_instantiations_from_decl_with_templates(
        decl: &Spanned<Decl>,
        class_templates: &HashMap<String, GenericClassTemplate>,
        import_aliases: &HashMap<String, String>,
        discovered: &mut HashMap<String, Vec<Type>>,
    ) {
        match &decl.node {
            Decl::Function(func) => {
                let generic_names = func
                    .generic_params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect::<HashSet<_>>();
                for param in &func.params {
                    Self::collect_generic_class_instantiation_from_type(
                        &param.ty,
                        class_templates,
                        import_aliases,
                        &generic_names,
                        discovered,
                    );
                }
                Self::collect_generic_class_instantiation_from_type(
                    &func.return_type,
                    class_templates,
                    import_aliases,
                    &generic_names,
                    discovered,
                );
                for stmt in &func.body {
                    Self::collect_generic_class_instantiation_from_stmt(
                        &stmt.node,
                        class_templates,
                        import_aliases,
                        &generic_names,
                        discovered,
                    );
                }
            }
            Decl::Class(class) => {
                let generic_names = class
                    .generic_params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect::<HashSet<_>>();
                if let Some(parent) = &class.extends {
                    if let Ok(parsed_parent) = parse_type_source(parent) {
                        Self::collect_generic_class_instantiation_from_type(
                            &parsed_parent,
                            class_templates,
                            import_aliases,
                            &generic_names,
                            discovered,
                        );
                    }
                }
                for field in &class.fields {
                    Self::collect_generic_class_instantiation_from_type(
                        &field.ty,
                        class_templates,
                        import_aliases,
                        &generic_names,
                        discovered,
                    );
                }
                if let Some(ctor) = &class.constructor {
                    for param in &ctor.params {
                        Self::collect_generic_class_instantiation_from_type(
                            &param.ty,
                            class_templates,
                            import_aliases,
                            &generic_names,
                            discovered,
                        );
                    }
                    for stmt in &ctor.body {
                        Self::collect_generic_class_instantiation_from_stmt(
                            &stmt.node,
                            class_templates,
                            import_aliases,
                            &generic_names,
                            discovered,
                        );
                    }
                }
                for method in &class.methods {
                    let mut method_generics = generic_names.clone();
                    method_generics
                        .extend(method.generic_params.iter().map(|param| param.name.clone()));
                    for param in &method.params {
                        Self::collect_generic_class_instantiation_from_type(
                            &param.ty,
                            class_templates,
                            import_aliases,
                            &method_generics,
                            discovered,
                        );
                    }
                    Self::collect_generic_class_instantiation_from_type(
                        &method.return_type,
                        class_templates,
                        import_aliases,
                        &method_generics,
                        discovered,
                    );
                    for stmt in &method.body {
                        Self::collect_generic_class_instantiation_from_stmt(
                            &stmt.node,
                            class_templates,
                            import_aliases,
                            &method_generics,
                            discovered,
                        );
                    }
                }
            }
            Decl::Module(module) => {
                for inner in &module.declarations {
                    Self::collect_generic_class_instantiations_from_decl_with_templates(
                        inner,
                        class_templates,
                        import_aliases,
                        discovered,
                    );
                }
            }
            Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
        }
    }

    fn rewrite_specialized_class_decl(
        decl: &Spanned<Decl>,
        module_prefix: Option<&str>,
        class_templates: &HashMap<String, GenericClassTemplate>,
        emitted_classes: &HashSet<String>,
    ) -> Spanned<Decl> {
        let node = match &decl.node {
            Decl::Function(func) => {
                let mut func = func.clone();
                if let Some(module_prefix) = module_prefix {
                    func.params = func
                        .params
                        .iter()
                        .map(|param| Parameter {
                            name: param.name.clone(),
                            ty: Self::rewrite_type_for_local_module_classes(
                                &param.ty,
                                module_prefix,
                                class_templates,
                            ),
                            mutable: param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                            mode: param.mode,
                        })
                        .collect();
                    func.return_type = Self::rewrite_type_for_local_module_classes(
                        &func.return_type,
                        module_prefix,
                        class_templates,
                    );
                    func.body = func
                        .body
                        .iter()
                        .map(|stmt| {
                            Spanned::new(
                                Self::rewrite_stmt_for_local_module_classes(
                                    &stmt.node,
                                    module_prefix,
                                    class_templates,
                                ),
                                stmt.span.clone(),
                            )
                        })
                        .collect();
                }
                func.params = func
                    .params
                    .iter()
                    .map(|param| Parameter {
                        name: param.name.clone(),
                        ty: Self::rewrite_specialized_class_type(&param.ty, emitted_classes),
                        mutable: param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                        mode: param.mode,
                    })
                    .collect();
                func.return_type =
                    Self::rewrite_specialized_class_type(&func.return_type, emitted_classes);
                func.body = func
                    .body
                    .iter()
                    .map(|stmt| {
                        Spanned::new(
                            Self::rewrite_specialized_class_stmt(&stmt.node, emitted_classes),
                            stmt.span.clone(),
                        )
                    })
                    .collect();
                Decl::Function(func)
            }
            Decl::Class(class) => {
                let mut class = class.clone();
                if let Some(module_prefix) = module_prefix {
                    class.extends = class.extends.as_ref().and_then(|parent| {
                        parse_type_source(parent).ok().map(|parsed| {
                            Self::format_type_string(&Self::rewrite_type_for_local_module_classes(
                                &parsed,
                                module_prefix,
                                class_templates,
                            ))
                        })
                    });
                    class.fields = class
                        .fields
                        .iter()
                        .map(|field| Field {
                            name: field.name.clone(),
                            ty: Self::rewrite_type_for_local_module_classes(
                                &field.ty,
                                module_prefix,
                                class_templates,
                            ),
                            mutable: field.mutable,
                            visibility: field.visibility,
                        })
                        .collect();
                    if let Some(constructor) = &class.constructor {
                        let mut ctor = constructor.clone();
                        ctor.params = ctor
                            .params
                            .iter()
                            .map(|param| Parameter {
                                name: param.name.clone(),
                                ty: Self::rewrite_type_for_local_module_classes(
                                    &param.ty,
                                    module_prefix,
                                    class_templates,
                                ),
                                mutable: param.mutable
                                    || matches!(param.mode, ParamMode::BorrowMut),
                                mode: param.mode,
                            })
                            .collect();
                        ctor.body = ctor
                            .body
                            .iter()
                            .map(|stmt| {
                                Spanned::new(
                                    Self::rewrite_stmt_for_local_module_classes(
                                        &stmt.node,
                                        module_prefix,
                                        class_templates,
                                    ),
                                    stmt.span.clone(),
                                )
                            })
                            .collect();
                        class.constructor = Some(ctor);
                    }
                    class.methods = class
                        .methods
                        .iter()
                        .map(|method| {
                            let mut method = method.clone();
                            method.params = method
                                .params
                                .iter()
                                .map(|param| Parameter {
                                    name: param.name.clone(),
                                    ty: Self::rewrite_type_for_local_module_classes(
                                        &param.ty,
                                        module_prefix,
                                        class_templates,
                                    ),
                                    mutable: param.mutable
                                        || matches!(param.mode, ParamMode::BorrowMut),
                                    mode: param.mode,
                                })
                                .collect();
                            method.return_type = Self::rewrite_type_for_local_module_classes(
                                &method.return_type,
                                module_prefix,
                                class_templates,
                            );
                            method.body = method
                                .body
                                .iter()
                                .map(|stmt| {
                                    Spanned::new(
                                        Self::rewrite_stmt_for_local_module_classes(
                                            &stmt.node,
                                            module_prefix,
                                            class_templates,
                                        ),
                                        stmt.span.clone(),
                                    )
                                })
                                .collect();
                            method
                        })
                        .collect();
                }
                class.extends = class.extends.as_ref().and_then(|parent| {
                    parse_type_source(parent).ok().map(|parsed| {
                        Self::format_type_string(&Self::rewrite_specialized_class_type(
                            &parsed,
                            emitted_classes,
                        ))
                    })
                });
                class.fields = class
                    .fields
                    .iter()
                    .map(|field| Field {
                        name: field.name.clone(),
                        ty: Self::rewrite_specialized_class_type(&field.ty, emitted_classes),
                        mutable: field.mutable,
                        visibility: field.visibility,
                    })
                    .collect();
                if let Some(constructor) = &class.constructor {
                    let mut ctor = constructor.clone();
                    ctor.params = ctor
                        .params
                        .iter()
                        .map(|param| Parameter {
                            name: param.name.clone(),
                            ty: Self::rewrite_specialized_class_type(&param.ty, emitted_classes),
                            mutable: param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                            mode: param.mode,
                        })
                        .collect();
                    ctor.body = ctor
                        .body
                        .iter()
                        .map(|stmt| {
                            Spanned::new(
                                Self::rewrite_specialized_class_stmt(&stmt.node, emitted_classes),
                                stmt.span.clone(),
                            )
                        })
                        .collect();
                    class.constructor = Some(ctor);
                }
                class.methods = class
                    .methods
                    .iter()
                    .map(|method| {
                        let mut method = method.clone();
                        method.params = method
                            .params
                            .iter()
                            .map(|param| Parameter {
                                name: param.name.clone(),
                                ty: Self::rewrite_specialized_class_type(
                                    &param.ty,
                                    emitted_classes,
                                ),
                                mutable: param.mutable
                                    || matches!(param.mode, ParamMode::BorrowMut),
                                mode: param.mode,
                            })
                            .collect();
                        method.return_type = Self::rewrite_specialized_class_type(
                            &method.return_type,
                            emitted_classes,
                        );
                        method.body = method
                            .body
                            .iter()
                            .map(|stmt| {
                                Spanned::new(
                                    Self::rewrite_specialized_class_stmt(
                                        &stmt.node,
                                        emitted_classes,
                                    ),
                                    stmt.span.clone(),
                                )
                            })
                            .collect();
                        method
                    })
                    .collect();
                Decl::Class(class)
            }
            Decl::Module(module) => {
                let mut module = module.clone();
                let next_prefix = module_prefix
                    .map(|prefix| format!("{}__{}", prefix, module.name))
                    .unwrap_or_else(|| module.name.clone());
                module.declarations = module
                    .declarations
                    .iter()
                    .map(|inner| {
                        Self::rewrite_specialized_class_decl(
                            inner,
                            Some(&next_prefix),
                            class_templates,
                            emitted_classes,
                        )
                    })
                    .collect();
                Decl::Module(module)
            }
            _ => decl.node.clone(),
        };
        Spanned::new(node, decl.span.clone())
    }

    fn specialize_generic_classes(program: &Program) -> Result<Program> {
        let mut class_templates = HashMap::new();
        for decl in &program.declarations {
            Self::collect_generic_class_templates_from_decl(decl, None, &mut class_templates);
        }
        if class_templates.is_empty() {
            return Ok(program.clone());
        }

        let mut emitted_classes: HashSet<String> = HashSet::new();
        for decl in &program.declarations {
            Self::collect_specialized_class_names_from_decl(decl, &mut emitted_classes);
        }
        let mut generated_classes: Vec<Spanned<Decl>> = Vec::new();
        let mut pending_decls = program.declarations.clone();
        let import_aliases = Self::collect_import_resolution_paths(program);

        loop {
            let mut discovered = HashMap::new();
            for decl in &pending_decls {
                Self::collect_generic_class_instantiations_from_decl_with_templates(
                    decl,
                    &class_templates,
                    &import_aliases,
                    &mut discovered,
                );
            }

            let mut added = false;
            let mut new_generated = Vec::new();
            for (spec_name, args) in discovered {
                if emitted_classes.contains(&spec_name) {
                    continue;
                }
                let Some((base_name, _)) = spec_name.split_once("__spec__") else {
                    continue;
                };
                let Some(template) = class_templates.get(base_name) else {
                    continue;
                };
                if template.class.generic_params.len() != args.len() {
                    continue;
                }

                let bindings = template
                    .class
                    .generic_params
                    .iter()
                    .map(|param| param.name.clone())
                    .zip(args.iter().cloned())
                    .collect::<HashMap<_, _>>();

                let mut spec_class = template.class.clone();
                spec_class.name = spec_name.clone();
                spec_class.generic_params.clear();
                spec_class.extends = spec_class.extends.as_ref().and_then(|parent| {
                    parse_type_source(parent).ok().map(|parsed| {
                        Self::format_type_string(&Self::substitute_type(&parsed, &bindings))
                    })
                });
                spec_class.fields = spec_class
                    .fields
                    .iter()
                    .map(|field| Field {
                        name: field.name.clone(),
                        ty: Self::substitute_type(&field.ty, &bindings),
                        mutable: field.mutable,
                        visibility: field.visibility,
                    })
                    .collect();
                if let Some(constructor) = &spec_class.constructor {
                    let mut new_constructor = constructor.clone();
                    new_constructor.params = new_constructor
                        .params
                        .iter()
                        .map(|param| Parameter {
                            name: param.name.clone(),
                            ty: Self::substitute_type(&param.ty, &bindings),
                            mutable: param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                            mode: param.mode,
                        })
                        .collect();
                    new_constructor.body = new_constructor
                        .body
                        .iter()
                        .map(|stmt| {
                            Spanned::new(
                                Self::substitute_stmt_types(&stmt.node, &bindings),
                                stmt.span.clone(),
                            )
                        })
                        .collect();
                    spec_class.constructor = Some(new_constructor);
                }
                spec_class.methods = spec_class
                    .methods
                    .iter()
                    .map(|method| {
                        let mut method = method.clone();
                        method.params = method
                            .params
                            .iter()
                            .map(|param| Parameter {
                                name: param.name.clone(),
                                ty: Self::substitute_type(&param.ty, &bindings),
                                mutable: param.mutable
                                    || matches!(param.mode, ParamMode::BorrowMut),
                                mode: param.mode,
                            })
                            .collect();
                        method.return_type = Self::substitute_type(&method.return_type, &bindings);
                        method.body = method
                            .body
                            .iter()
                            .map(|stmt| {
                                Spanned::new(
                                    Self::substitute_stmt_types(&stmt.node, &bindings),
                                    stmt.span.clone(),
                                )
                            })
                            .collect();
                        method
                    })
                    .collect();

                emitted_classes.insert(spec_name.clone());
                let decl = Spanned::new(Decl::Class(spec_class), template.span.clone());
                new_generated.push(decl.clone());
                generated_classes.push(decl);
                added = true;
            }

            if !added {
                break;
            }
            pending_decls.extend(new_generated);
        }

        if emitted_classes.is_empty() {
            return Ok(program.clone());
        }

        let mut all_decls = program.declarations.clone();
        all_decls.extend(generated_classes);
        let rewritten_decls = all_decls
            .iter()
            .map(|decl| {
                Self::rewrite_specialized_class_decl(decl, None, &class_templates, &emitted_classes)
            })
            .collect();

        Ok(Program {
            package: program.package.clone(),
            declarations: rewritten_decls,
        })
    }

    fn rewrite_decl_generic_calls(
        decl: &Spanned<Decl>,
        templates: &GenericRewriteTemplates<'_>,
        outputs: &mut GenericRewriteOutputs<'_>,
    ) -> Result<Spanned<Decl>> {
        Ok(match &decl.node {
            Decl::Function(func) => {
                let mut f = func.clone();
                f.body = f
                    .body
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(&s.node, templates, outputs)?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;
                Spanned::new(Decl::Function(f), decl.span.clone())
            }
            Decl::Module(module) => {
                let mut m = module.clone();
                m.declarations = m
                    .declarations
                    .iter()
                    .map(|inner| Self::rewrite_decl_generic_calls(inner, templates, outputs))
                    .collect::<Result<Vec<_>>>()?;
                Spanned::new(Decl::Module(m), decl.span.clone())
            }
            Decl::Class(class) => {
                let mut c = class.clone();
                if let Some(ctor) = &mut c.constructor {
                    ctor.body = ctor
                        .body
                        .iter()
                        .map(|s| {
                            Ok(Spanned::new(
                                Self::rewrite_stmt_generic_calls(&s.node, templates, outputs)?,
                                s.span.clone(),
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?;
                }
                if let Some(dtor) = &mut c.destructor {
                    dtor.body = dtor
                        .body
                        .iter()
                        .map(|s| {
                            Ok(Spanned::new(
                                Self::rewrite_stmt_generic_calls(&s.node, templates, outputs)?,
                                s.span.clone(),
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?;
                }
                c.methods = c
                    .methods
                    .iter()
                    .map(|method| {
                        let mut nm = method.clone();
                        nm.body = nm
                            .body
                            .iter()
                            .map(|s| {
                                Ok(Spanned::new(
                                    Self::rewrite_stmt_generic_calls(&s.node, templates, outputs)?,
                                    s.span.clone(),
                                ))
                            })
                            .collect::<Result<Vec<_>>>()?;
                        Ok(nm)
                    })
                    .collect::<Result<Vec<_>>>()?;
                Spanned::new(Decl::Class(c), decl.span.clone())
            }
            _ => decl.clone(),
        })
    }

    fn append_generated_methods_to_decl(
        decl: &Spanned<Decl>,
        module_prefix: Option<&str>,
        generated_methods: &HashMap<String, Vec<FunctionDecl>>,
    ) -> Spanned<Decl> {
        match &decl.node {
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                let mut rewritten_module = module.clone();
                rewritten_module.declarations = module
                    .declarations
                    .iter()
                    .map(|inner| {
                        Self::append_generated_methods_to_decl(
                            inner,
                            Some(&next_prefix),
                            generated_methods,
                        )
                    })
                    .collect();
                Spanned::new(Decl::Module(rewritten_module), decl.span.clone())
            }
            Decl::Class(class) => {
                let class_key = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, class.name)
                } else {
                    class.name.clone()
                };
                let mut rewritten_class = class.clone();
                if let Some(extra_methods) = generated_methods.get(&class_key) {
                    rewritten_class.methods.extend(extra_methods.clone());
                }
                Spanned::new(Decl::Class(rewritten_class), decl.span.clone())
            }
            _ => decl.clone(),
        }
    }

    fn expr_has_explicit_generic_calls(expr: &Expr) -> bool {
        match expr {
            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                !type_args.is_empty()
                    || Self::expr_has_explicit_generic_calls(&callee.node)
                    || args
                        .iter()
                        .any(|arg| Self::expr_has_explicit_generic_calls(&arg.node))
            }
            Expr::GenericFunctionValue { callee, type_args } => {
                !type_args.is_empty() || Self::expr_has_explicit_generic_calls(&callee.node)
            }
            Expr::Binary { left, right, .. } => {
                Self::expr_has_explicit_generic_calls(&left.node)
                    || Self::expr_has_explicit_generic_calls(&right.node)
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => Self::expr_has_explicit_generic_calls(&expr.node),
            Expr::Field { object, .. } => Self::expr_has_explicit_generic_calls(&object.node),
            Expr::Index { object, index } => {
                Self::expr_has_explicit_generic_calls(&object.node)
                    || Self::expr_has_explicit_generic_calls(&index.node)
            }
            Expr::Construct { args, .. } => args
                .iter()
                .any(|arg| Self::expr_has_explicit_generic_calls(&arg.node)),
            Expr::Lambda { body, .. } => Self::expr_has_explicit_generic_calls(&body.node),
            Expr::Match { expr, arms } => {
                Self::expr_has_explicit_generic_calls(&expr.node)
                    || arms.iter().any(|arm| {
                        arm.body
                            .iter()
                            .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                    })
            }
            Expr::StringInterp(parts) => parts.iter().any(|part| match part {
                StringPart::Literal(_) => false,
                StringPart::Expr(expr) => Self::expr_has_explicit_generic_calls(&expr.node),
            }),
            Expr::AsyncBlock(block) | Expr::Block(block) => block
                .iter()
                .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node)),
            Expr::Require { condition, message } => {
                Self::expr_has_explicit_generic_calls(&condition.node)
                    || message
                        .as_ref()
                        .is_some_and(|msg| Self::expr_has_explicit_generic_calls(&msg.node))
            }
            Expr::Range { start, end, .. } => {
                start
                    .as_ref()
                    .is_some_and(|expr| Self::expr_has_explicit_generic_calls(&expr.node))
                    || end
                        .as_ref()
                        .is_some_and(|expr| Self::expr_has_explicit_generic_calls(&expr.node))
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                Self::expr_has_explicit_generic_calls(&condition.node)
                    || then_branch
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                    || else_branch.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                    })
            }
            Expr::Literal(_) | Expr::Ident(_) | Expr::This => false,
        }
    }

    fn stmt_has_explicit_generic_calls(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Let { value, .. } => Self::expr_has_explicit_generic_calls(&value.node),
            Stmt::Assign { target, value } => {
                Self::expr_has_explicit_generic_calls(&target.node)
                    || Self::expr_has_explicit_generic_calls(&value.node)
            }
            Stmt::Expr(expr) => Self::expr_has_explicit_generic_calls(&expr.node),
            Stmt::Return(expr) => expr
                .as_ref()
                .is_some_and(|expr| Self::expr_has_explicit_generic_calls(&expr.node)),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                Self::expr_has_explicit_generic_calls(&condition.node)
                    || then_block
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                    })
            }
            Stmt::While { condition, body } => {
                Self::expr_has_explicit_generic_calls(&condition.node)
                    || body
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
            }
            Stmt::For { iterable, body, .. } => {
                Self::expr_has_explicit_generic_calls(&iterable.node)
                    || body
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
            }
            Stmt::Match { expr, arms } => {
                Self::expr_has_explicit_generic_calls(&expr.node)
                    || arms.iter().any(|arm| {
                        arm.body
                            .iter()
                            .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                    })
            }
            Stmt::Break | Stmt::Continue => false,
        }
    }

    fn decl_has_explicit_generic_calls(decl: &Decl) -> bool {
        match decl {
            Decl::Function(func) => func
                .body
                .iter()
                .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node)),
            Decl::Class(class) => {
                class.constructor.as_ref().is_some_and(|ctor| {
                    ctor.body
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                }) || class.destructor.as_ref().is_some_and(|dtor| {
                    dtor.body
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                }) || class.methods.iter().any(|method| {
                    method
                        .body
                        .iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                })
            }
            Decl::Module(module) => module
                .declarations
                .iter()
                .any(|decl| Self::decl_has_explicit_generic_calls(&decl.node)),
            Decl::Interface(interface) => interface.methods.iter().any(|method| {
                method.default_impl.as_ref().is_some_and(|body| {
                    body.iter()
                        .any(|stmt| Self::stmt_has_explicit_generic_calls(&stmt.node))
                })
            }),
            Decl::Enum(_) | Decl::Import(_) => false,
        }
    }

    fn program_has_explicit_generic_calls(program: &Program) -> bool {
        program
            .declarations
            .iter()
            .any(|decl| Self::decl_has_explicit_generic_calls(&decl.node))
    }

    fn decl_has_generic_classes(decl: &Decl) -> bool {
        match decl {
            Decl::Class(class) => !class.generic_params.is_empty(),
            Decl::Module(module) => module
                .declarations
                .iter()
                .any(|decl| Self::decl_has_generic_classes(&decl.node)),
            Decl::Function(_) | Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => false,
        }
    }

    fn program_has_generic_classes(program: &Program) -> bool {
        program
            .declarations
            .iter()
            .any(|decl| Self::decl_has_generic_classes(&decl.node))
    }

    fn first_user_generic_enum_name(
        declarations: &[Spanned<Decl>],
        module_prefix: Option<&str>,
    ) -> Option<String> {
        for decl in declarations {
            match &decl.node {
                Decl::Enum(en) if !en.generic_params.is_empty() => {
                    let enum_name = if let Some(prefix) = module_prefix {
                        format!("{prefix}__{}", en.name)
                    } else {
                        en.name.clone()
                    };
                    return Some(enum_name);
                }
                Decl::Module(module) => {
                    let nested_prefix = if let Some(prefix) = module_prefix {
                        format!("{prefix}__{}", module.name)
                    } else {
                        module.name.clone()
                    };
                    if let Some(found) = Self::first_user_generic_enum_name(
                        &module.declarations,
                        Some(&nested_prefix),
                    ) {
                        return Some(found);
                    }
                }
                Decl::Function(_) | Decl::Class(_) | Decl::Interface(_) | Decl::Import(_) => {}
                Decl::Enum(_) => {}
            }
        }
        None
    }

    fn collect_specialized_class_names_from_decl(decl: &Spanned<Decl>, out: &mut HashSet<String>) {
        match &decl.node {
            Decl::Class(class) => {
                if class.name.contains("__spec__") {
                    out.insert(class.name.clone());
                }
            }
            Decl::Module(module) => {
                for inner in &module.declarations {
                    Self::collect_specialized_class_names_from_decl(inner, out);
                }
            }
            Decl::Function(_) | Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
        }
    }

    fn normalize_import_resolution_path(current_package: Option<&str>, path: &str) -> String {
        current_package
            .and_then(|package| {
                path.strip_prefix(package)
                    .and_then(|rest| rest.strip_prefix('.'))
            })
            .map(str::to_string)
            .unwrap_or_else(|| path.to_string())
    }

    fn collect_import_resolution_paths(program: &Program) -> HashMap<String, String> {
        fn collect_from_decls(
            declarations: &[Spanned<Decl>],
            current_package: Option<&str>,
            normalize_import_path: fn(Option<&str>, &str) -> String,
            import_paths: &mut HashMap<String, String>,
        ) {
            for decl in declarations {
                match &decl.node {
                    Decl::Import(import) => {
                        if let Some(alias) = &import.alias {
                            import_paths.insert(
                                alias.clone(),
                                normalize_import_path(current_package, &import.path),
                            );
                        } else if import.path.ends_with(".*") {
                            import_paths.insert(
                                import.path.clone(),
                                normalize_import_path(current_package, &import.path),
                            );
                        }
                    }
                    Decl::Module(module) => {
                        collect_from_decls(
                            &module.declarations,
                            current_package,
                            normalize_import_path,
                            import_paths,
                        );
                    }
                    _ => {}
                }
            }
        }

        let mut import_paths = HashMap::new();
        let current_package = program.package.as_deref();
        collect_from_decls(
            &program.declarations,
            current_package,
            Self::normalize_import_resolution_path,
            &mut import_paths,
        );
        import_paths
    }

    fn specialize_explicit_generic_calls(program: &Program) -> Result<Program> {
        let mut function_templates: HashMap<String, GenericTemplate> = HashMap::new();
        let mut method_templates: HashMap<String, Vec<GenericTemplate>> = HashMap::new();
        let mut class_templates: HashMap<String, GenericClassTemplate> = HashMap::new();
        let import_aliases = Self::collect_import_resolution_paths(program);
        for decl in &program.declarations {
            Self::collect_generic_templates_from_decl(
                decl,
                None,
                &mut function_templates,
                &mut method_templates,
            );
            Self::collect_generic_class_templates_from_decl(decl, None, &mut class_templates);
        }
        if function_templates.is_empty()
            && method_templates.is_empty()
            && class_templates.is_empty()
        {
            return Ok(program.clone());
        }

        let mut emitted_specs: HashSet<String> = HashSet::new();
        let mut generated_functions: Vec<Spanned<Decl>> = Vec::new();
        let mut generated_methods: HashMap<String, Vec<FunctionDecl>> = HashMap::new();
        let mut emitted_class_specs: HashSet<String> = HashSet::new();
        for decl in &program.declarations {
            Self::collect_specialized_class_names_from_decl(decl, &mut emitted_class_specs);
        }
        let templates = GenericRewriteTemplates {
            function_templates: &function_templates,
            method_templates: &method_templates,
            class_templates: &class_templates,
            import_aliases: &import_aliases,
        };
        let mut outputs = GenericRewriteOutputs {
            emitted: &mut emitted_specs,
            generated_functions: &mut generated_functions,
            generated_methods: &mut generated_methods,
        };
        let rewritten = program
            .declarations
            .iter()
            .map(|decl| Self::rewrite_decl_generic_calls(decl, &templates, &mut outputs))
            .collect::<Result<Vec<_>>>()?;

        let mut all_decls = rewritten
            .iter()
            .map(|decl| Self::append_generated_methods_to_decl(decl, None, &generated_methods))
            .collect::<Vec<_>>();
        all_decls.extend(generated_functions);
        let rewritten_all = all_decls
            .iter()
            .map(|decl| {
                Self::rewrite_specialized_class_decl(
                    decl,
                    None,
                    &class_templates,
                    &emitted_class_specs,
                )
            })
            .collect();
        Ok(Program {
            package: program.package.clone(),
            declarations: rewritten_all,
        })
    }

    pub fn new(context: &'ctx Context, name: &str) -> Self {
        let module = context.create_module(name);
        let builder = context.create_builder();

        Self {
            context,
            module,
            builder,
            variables: HashMap::new(),
            non_negative_locals: HashSet::new(),
            non_zero_locals: HashSet::new(),
            exact_integer_locals: HashMap::new(),
            upper_bound_locals: HashMap::new(),
            exact_list_lengths: HashMap::new(),
            exact_list_capacities: HashMap::new(),
            list_element_upper_bounds: HashMap::new(),
            distinct_list_alloc_ids: HashMap::new(),
            next_distinct_list_alloc_id: 1,
            functions: HashMap::new(),
            non_negative_functions: HashSet::new(),
            function_param_modes: HashMap::new(),
            classes: HashMap::new(),
            enums: HashMap::new(),
            interfaces: HashMap::new(),
            interface_implementors: HashMap::new(),
            enum_variant_to_enum: HashMap::new(),
            current_function: None,
            current_return_type: None,
            loop_stack: Vec::new(),
            str_counter: 0,
            lambda_counter: 0,
            async_counter: 0,
            async_functions: HashMap::new(),
            extern_functions: HashSet::new(),
            import_aliases: HashMap::new(),
            current_package: String::new(),
            current_module_prefix: None,
            current_generic_bounds: HashMap::new(),
        }
    }

    /// Check if a function name is a stdlib function
    pub fn is_stdlib_function(name: &str) -> bool {
        matches!(
            name,
            // Math functions
            "Math__abs" | "Math__min" | "Math__max" | "Math__sqrt" | "Math__pow" |
            "Math__sin" | "Math__cos" | "Math__tan" | "Math__floor" | "Math__ceil" |
            "Math__round" | "Math__log" | "Math__log10" | "Math__exp" | "Math__pi" |
            "Math__e" | "Math__random" |
            // Type conversions
            "to_int" | "to_float" | "to_string" |
            // String functions
            "Str__len" | "Str__compare" | "Str__concat" | "Str__upper" | "Str__lower" |
            "Str__trim" | "Str__contains" | "Str__startsWith" | "Str__endsWith" |
            // I/O functions
            "read_line" |
            // System functions
            "System__exit" | "exit" | "System__cwd" | "System__os" | "System__shell" |
            "System__exec" | "System__getenv" |
            // Time functions
            "Time__now" | "Time__unix" | "Time__sleep" |
            // File functions
            "File__read" | "File__write" | "File__exists" | "File__delete" |
            // Args functions
            "Args__get" | "Args__count" |
            // Assertion functions
            "assert" | "assert_eq" | "assert_ne" | "assert_true" | "assert_false" | "fail" |
            // Range function
            "range"
        )
    }

    pub(crate) fn param_mode_for_function(&self, function_name: &str, index: usize) -> ParamMode {
        self.function_param_modes
            .get(function_name)
            .and_then(|modes| modes.get(index).copied())
            .unwrap_or(ParamMode::Owned)
    }

    pub(crate) fn compile_argument_for_param(
        &mut self,
        function_name: &str,
        index: usize,
        arg: &Spanned<Expr>,
        expected_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        match self.param_mode_for_function(function_name, index) {
            ParamMode::Owned => {
                self.compile_expr_for_concrete_class_payload(&arg.node, expected_ty)
            }
            ParamMode::Borrow => self.compile_borrow(&arg.node),
            ParamMode::BorrowMut => self.compile_mut_borrow(&arg.node),
        }
    }

    fn validate_stdlib_arg_count(&self, name: &str, args: &[Spanned<Expr>]) -> Result<()> {
        let expected = match name {
            "read_line" | "Time__unix" | "System__cwd" | "System__os" | "Math__random"
            | "Math__pi" | "Math__e" | "Args__count" => Some((0usize, 0usize)),
            "Math__abs" | "Math__sqrt" | "Math__sin" | "Math__cos" | "Math__tan"
            | "Math__floor" | "Math__ceil" | "Math__round" | "Math__log" | "Math__log10"
            | "Math__exp" | "to_float" | "to_int" | "to_string" | "Str__len" | "Str__upper"
            | "Str__lower" | "Str__trim" | "System__exit" | "exit" | "File__read"
            | "File__exists" | "File__delete" | "Time__now" | "Time__sleep" | "System__getenv"
            | "System__shell" | "System__exec" | "Args__get" | "assert" | "assert_true"
            | "assert_false" => Some((1, 1)),
            "Math__min" | "Math__max" | "Math__pow" | "Str__compare" | "Str__concat"
            | "Str__contains" | "Str__startsWith" | "Str__endsWith" | "File__write"
            | "assert_eq" | "assert_ne" => Some((2, 2)),
            "range" => Some((2, 3)),
            "fail" => Some((0, 1)),
            _ => None,
        };

        if let Some((min_args, max_args)) = expected {
            if args.len() < min_args || args.len() > max_args {
                return if min_args == max_args {
                    Err(CodegenError::new(format!(
                        "{}() expects {} argument(s), got {}",
                        name,
                        min_args,
                        args.len()
                    )))
                } else if name == "range" {
                    Err(CodegenError::new(
                        "range() requires 2 or 3 arguments: range(start, end) or range(start, end, step)",
                    ))
                } else {
                    Err(CodegenError::new(format!(
                        "{}() expects {} or {} argument(s), got {}",
                        name,
                        min_args,
                        max_args,
                        args.len()
                    )))
                };
            }
        }

        Ok(())
    }

    fn validate_numeric_stdlib_arg(&self, name: &str, expr: &Expr) -> Result<Type> {
        let arg_ty = self.infer_expr_type(expr, &[]);
        if !matches!(arg_ty, Type::Integer | Type::Float) {
            return Err(CodegenError::new(format!(
                "{}() requires numeric type, got {}",
                name,
                Self::format_diagnostic_type(&arg_ty)
            )));
        }
        Ok(arg_ty)
    }

    fn validate_numeric_stdlib_pair(
        &self,
        name: &str,
        left: &Expr,
        right: &Expr,
    ) -> Result<(Type, Type)> {
        let left_ty = self.infer_expr_type(left, &[]);
        let right_ty = self.infer_expr_type(right, &[]);
        if !matches!(left_ty, Type::Integer | Type::Float)
            || !matches!(right_ty, Type::Integer | Type::Float)
        {
            return Err(CodegenError::new(format!(
                "{}() arguments must be numeric types, got {} and {}",
                name,
                Self::format_diagnostic_type(&left_ty),
                Self::format_diagnostic_type(&right_ty)
            )));
        }
        Ok((left_ty, right_ty))
    }

    /// Compile full program.
    pub fn compile(&mut self, program: &Program) -> Result<()> {
        self.compile_internal(program, None, None)
    }

    pub fn compile_filtered_with_decl_symbols(
        &mut self,
        program: &Program,
        active_symbols: &HashSet<String>,
        declaration_symbols: &HashSet<String>,
    ) -> Result<()> {
        self.compile_internal(program, Some(active_symbols), Some(declaration_symbols))
    }

    fn compile_internal(
        &mut self,
        program: &Program,
        active_symbols: Option<&HashSet<String>>,
        declaration_symbols: Option<&HashSet<String>>,
    ) -> Result<()> {
        self.current_package = program.package.clone().unwrap_or_default();
        if let Some(enum_name) = Self::first_user_generic_enum_name(&program.declarations, None) {
            return Err(CodegenError::new(format!(
                "Enum '{}' uses generic parameters, but user-defined generic enums are not supported yet",
                enum_name
            )));
        }
        fn collect_generated_spec_symbols(program: &Program) -> HashMap<String, HashSet<String>> {
            fn collect_decl_symbols(
                decl: &Spanned<Decl>,
                module_prefix: Option<&str>,
                symbols_by_owner: &mut HashMap<String, HashSet<String>>,
            ) {
                match &decl.node {
                    Decl::Function(func) => {
                        let name = if let Some(prefix) = module_prefix {
                            format!("{}__{}", prefix, func.name)
                        } else {
                            func.name.clone()
                        };
                        if let Some((owner, _)) = name.split_once("__spec__") {
                            symbols_by_owner
                                .entry(owner.to_string())
                                .or_default()
                                .insert(name);
                        }
                    }
                    Decl::Class(class) => {
                        let class_name = if let Some(prefix) = module_prefix {
                            format!("{}__{}", prefix, class.name)
                        } else {
                            class.name.clone()
                        };
                        let class_owner = class_name
                            .split_once("__spec__")
                            .map(|(owner, _)| owner.to_string());
                        if let Some(owner) = class_owner.as_ref() {
                            symbols_by_owner
                                .entry(owner.clone())
                                .or_default()
                                .insert(class_name.clone());
                            symbols_by_owner
                                .entry(owner.clone())
                                .or_default()
                                .insert(format!("{}__new", class_name));
                        }
                        for method in &class.methods {
                            let method_name = format!("{}__{}", class_name, method.name);
                            if method_name.contains("__spec__") || class_owner.is_some() {
                                if let Some(owner) = class_owner.as_ref() {
                                    symbols_by_owner
                                        .entry(owner.clone())
                                        .or_default()
                                        .insert(method_name);
                                }
                            }
                        }
                    }
                    Decl::Enum(en) => {
                        let name = if let Some(prefix) = module_prefix {
                            format!("{}__{}", prefix, en.name)
                        } else {
                            en.name.clone()
                        };
                        if let Some((owner, _)) = name.split_once("__spec__") {
                            symbols_by_owner
                                .entry(owner.to_string())
                                .or_default()
                                .insert(name);
                        }
                    }
                    Decl::Module(module) => {
                        let module_name = if let Some(prefix) = module_prefix {
                            format!("{}__{}", prefix, module.name)
                        } else {
                            module.name.clone()
                        };
                        for inner in &module.declarations {
                            collect_decl_symbols(inner, Some(&module_name), symbols_by_owner);
                        }
                    }
                    Decl::Interface(_) | Decl::Import(_) => {}
                }
            }

            let mut symbols_by_owner = HashMap::new();
            for decl in &program.declarations {
                collect_decl_symbols(decl, None, &mut symbols_by_owner);
            }
            symbols_by_owner
        }

        let generic_class_check_started_at = Instant::now();
        let has_generic_classes = Self::program_has_generic_classes(program);
        CODEGEN_PHASE_TIMING_TOTALS
            .program_has_generic_classes_ns
            .fetch_add(
                elapsed_nanos_u64(generic_class_check_started_at),
                Ordering::Relaxed,
            );
        let class_specialized_program;
        let explicit_specialized_program;
        let final_specialized_program;
        let program = if has_generic_classes {
            let specialize_generic_classes_started_at = Instant::now();
            class_specialized_program = Self::specialize_generic_classes(program)?;
            CODEGEN_PHASE_TIMING_TOTALS
                .specialize_generic_classes_initial_ns
                .fetch_add(
                    elapsed_nanos_u64(specialize_generic_classes_started_at),
                    Ordering::Relaxed,
                );
            let explicit_generic_check_started_at = Instant::now();
            let has_explicit_generic_calls =
                Self::program_has_explicit_generic_calls(&class_specialized_program);
            CODEGEN_PHASE_TIMING_TOTALS
                .program_has_explicit_generic_calls_ns
                .fetch_add(
                    elapsed_nanos_u64(explicit_generic_check_started_at),
                    Ordering::Relaxed,
                );
            if has_explicit_generic_calls {
                let specialize_explicit_started_at = Instant::now();
                explicit_specialized_program =
                    Self::specialize_explicit_generic_calls(&class_specialized_program)?;
                CODEGEN_PHASE_TIMING_TOTALS
                    .specialize_explicit_generic_calls_ns
                    .fetch_add(
                        elapsed_nanos_u64(specialize_explicit_started_at),
                        Ordering::Relaxed,
                    );
                let specialize_generic_classes_final_started_at = Instant::now();
                final_specialized_program =
                    Self::specialize_generic_classes(&explicit_specialized_program)?;
                CODEGEN_PHASE_TIMING_TOTALS
                    .specialize_generic_classes_final_ns
                    .fetch_add(
                        elapsed_nanos_u64(specialize_generic_classes_final_started_at),
                        Ordering::Relaxed,
                    );
                &final_specialized_program
            } else {
                &class_specialized_program
            }
        } else {
            let explicit_generic_check_started_at = Instant::now();
            let has_explicit_generic_calls = Self::program_has_explicit_generic_calls(program);
            CODEGEN_PHASE_TIMING_TOTALS
                .program_has_explicit_generic_calls_ns
                .fetch_add(
                    elapsed_nanos_u64(explicit_generic_check_started_at),
                    Ordering::Relaxed,
                );
            if has_explicit_generic_calls {
                let specialize_explicit_started_at = Instant::now();
                explicit_specialized_program = Self::specialize_explicit_generic_calls(program)?;
                CODEGEN_PHASE_TIMING_TOTALS
                    .specialize_explicit_generic_calls_ns
                    .fetch_add(
                        elapsed_nanos_u64(specialize_explicit_started_at),
                        Ordering::Relaxed,
                    );
                &explicit_specialized_program
            } else {
                program
            }
        };
        CODEGEN_PHASE_TIMING_TOTALS
            .total_decls_count
            .fetch_add(program.declarations.len(), Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .active_symbols_count
            .fetch_add(active_symbols.map_or(0, HashSet::len), Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .declaration_symbols_count
            .fetch_add(
                declaration_symbols.map_or(0, HashSet::len),
                Ordering::Relaxed,
            );
        let collect_generated_spec_symbols_started_at = Instant::now();
        let generated_spec_symbols_by_owner = collect_generated_spec_symbols(program);
        CODEGEN_PHASE_TIMING_TOTALS
            .collect_generated_spec_symbols_ns
            .fetch_add(
                elapsed_nanos_u64(collect_generated_spec_symbols_started_at),
                Ordering::Relaxed,
            );
        CODEGEN_PHASE_TIMING_TOTALS
            .generated_spec_owners_count
            .fetch_add(generated_spec_symbols_by_owner.len(), Ordering::Relaxed);
        let specialized_active_symbols_started_at = Instant::now();
        let specialized_declaration_symbols = declaration_symbols.map(|symbols| {
            let mut combined = symbols.clone();
            for owner in symbols {
                if let Some(generated_symbols) = generated_spec_symbols_by_owner.get(owner) {
                    combined.extend(generated_symbols.iter().cloned());
                }
            }
            for generated_symbols in generated_spec_symbols_by_owner.values() {
                combined.extend(generated_symbols.iter().cloned());
            }
            combined
        });
        let specialized_active_symbols = active_symbols.map(|symbols| {
            let mut combined = symbols.clone();
            let owner_symbols = symbols.iter().chain(
                specialized_declaration_symbols
                    .iter()
                    .flat_map(|set| set.iter()),
            );
            for owner in owner_symbols {
                if let Some(generated_symbols) = generated_spec_symbols_by_owner.get(owner) {
                    combined.extend(generated_symbols.iter().cloned());
                }
            }
            for generated_symbols in generated_spec_symbols_by_owner.values() {
                combined.extend(generated_symbols.iter().cloned());
            }
            combined
        });
        CODEGEN_PHASE_TIMING_TOTALS
            .specialized_active_symbols_ns
            .fetch_add(
                elapsed_nanos_u64(specialized_active_symbols_started_at),
                Ordering::Relaxed,
            );

        if Self::program_is_top_level_function_only(program) {
            self.import_aliases.clear();
            self.interfaces.clear();
            self.interface_implementors.clear();

            let import_alias_collection_started_at = Instant::now();
            let import_alias_count =
                self.collect_top_level_codegen_import_aliases(&program.declarations);
            CODEGEN_PHASE_TIMING_TOTALS
                .import_alias_collection_ns
                .fetch_add(
                    elapsed_nanos_u64(import_alias_collection_started_at),
                    Ordering::Relaxed,
                );
            CODEGEN_PHASE_TIMING_TOTALS
                .import_alias_count
                .fetch_add(import_alias_count, Ordering::Relaxed);

            return self.compile_top_level_function_only_program(
                program,
                specialized_active_symbols.as_ref(),
                specialized_declaration_symbols.as_ref(),
            );
        }

        let import_alias_collection_started_at = Instant::now();
        let mut import_alias_count = 0_usize;
        let mut interface_extends: HashMap<String, Vec<(Option<String>, String)>> = HashMap::new();
        let mut class_interface_impls = Vec::new();
        self.import_aliases.clear();
        self.interfaces.clear();
        self.interface_implementors.clear();
        fn collect_codegen_import_aliases(
            declarations: &[Spanned<Decl>],
            import_aliases: &mut HashMap<String, Vec<(Option<String>, String)>>,
            import_alias_count: &mut usize,
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
                            *import_alias_count += 1;
                        } else if import.path.ends_with(".*") {
                            import_aliases
                                .entry(import.path.clone())
                                .or_default()
                                .push((module_prefix.map(str::to_string), import.path.clone()));
                            *import_alias_count += 1;
                        }
                    }
                    Decl::Module(module) => {
                        let next_prefix = if let Some(prefix) = module_prefix {
                            format!("{}__{}", prefix, module.name)
                        } else {
                            module.name.clone()
                        };
                        collect_codegen_import_aliases(
                            &module.declarations,
                            import_aliases,
                            import_alias_count,
                            Some(&next_prefix),
                        );
                    }
                    _ => {}
                }
            }
        }
        collect_codegen_import_aliases(
            &program.declarations,
            &mut self.import_aliases,
            &mut import_alias_count,
            None,
        );
        for decl in &program.declarations {
            Self::collect_interface_methods_from_decl(
                decl,
                None,
                &mut self.interfaces,
                &mut interface_extends,
            );
            Self::collect_class_interface_impls_from_decl(decl, None, &mut class_interface_impls);
        }
        let own_interface_methods = self.interfaces.clone();
        let mut expanded_interface_methods = HashMap::new();
        let mut visiting = HashSet::new();
        for interface_name in own_interface_methods.keys() {
            let methods = self.collect_interface_methods_with_inheritance(
                interface_name,
                &own_interface_methods,
                &interface_extends,
                &mut expanded_interface_methods,
                &mut visiting,
            );
            expanded_interface_methods.insert(interface_name.clone(), methods);
        }
        self.interfaces = expanded_interface_methods;
        let mut ancestor_cache = HashMap::new();
        let mut ancestor_visiting = HashSet::new();
        for (class_name, module_prefix, interface_ref) in class_interface_impls {
            if let Some(interface_name) =
                self.resolve_interface_name_for_lookup(&interface_ref, module_prefix.as_deref())
            {
                for ancestor in self.collect_interface_ancestors(
                    &interface_name,
                    &interface_extends,
                    &mut ancestor_cache,
                    &mut ancestor_visiting,
                ) {
                    self.interface_implementors
                        .entry(ancestor)
                        .or_default()
                        .insert(class_name.clone());
                }
            }
        }
        CODEGEN_PHASE_TIMING_TOTALS
            .import_alias_collection_ns
            .fetch_add(
                elapsed_nanos_u64(import_alias_collection_started_at),
                Ordering::Relaxed,
            );
        CODEGEN_PHASE_TIMING_TOTALS
            .import_alias_count
            .fetch_add(import_alias_count, Ordering::Relaxed);

        let top_level_decl_filter_started_at = Instant::now();
        let precomputed_declare_flags = specialized_declaration_symbols.as_ref().map(|symbols| {
            program
                .declarations
                .iter()
                .map(|decl| self.should_compile_decl(&decl.node, symbols))
                .collect::<Vec<_>>()
        });
        let precomputed_body_flags = specialized_active_symbols.as_ref().map(|symbols| {
            program
                .declarations
                .iter()
                .map(|decl| self.should_emit_decl_body(&decl.node, symbols))
                .collect::<Vec<_>>()
        });
        let top_level_decl_filter_ns = elapsed_nanos_u64(top_level_decl_filter_started_at);

        // First pass (0): declare all enums first so Named(Enum) resolves correctly.
        let enum_declare_pass_started_at = Instant::now();
        let enum_declare_decl_filter_ns = top_level_decl_filter_ns;
        let mut enum_declare_work_ns = 0_u64;
        let mut declared_enum_count = 0_usize;
        for (decl_index, decl) in program.declarations.iter().enumerate() {
            let should_declare = precomputed_declare_flags
                .as_ref()
                .map(|flags| flags[decl_index])
                .unwrap_or(true);
            if !should_declare {
                continue;
            }
            if let Decl::Enum(en) = &decl.node {
                let declare_enum_started_at = Instant::now();
                self.declare_enum(en)?;
                enum_declare_work_ns += elapsed_nanos_u64(declare_enum_started_at);
                declared_enum_count += 1;
            }
        }
        CODEGEN_PHASE_TIMING_TOTALS.enum_declare_pass_ns.fetch_add(
            elapsed_nanos_u64(enum_declare_pass_started_at),
            Ordering::Relaxed,
        );
        CODEGEN_PHASE_TIMING_TOTALS
            .enum_declare_decl_filter_ns
            .fetch_add(enum_declare_decl_filter_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .enum_declare_work_ns
            .fetch_add(enum_declare_work_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .declared_enum_count
            .fetch_add(declared_enum_count, Ordering::Relaxed);

        // First pass: declare all classes and functions
        let decl_pass_started_at = Instant::now();
        let decl_pass_decl_filter_ns = top_level_decl_filter_ns;
        let mut decl_pass_class_work_ns = 0_u64;
        let mut decl_pass_function_work_ns = 0_u64;
        let mut decl_pass_module_work_ns = 0_u64;
        let mut declared_class_count = 0_usize;
        let mut declared_function_count = 0_usize;
        let mut declared_module_count = 0_usize;
        let mut pending_classes = Vec::new();
        for (decl_index, decl) in program.declarations.iter().enumerate() {
            if let Decl::Class(class) = &decl.node {
                pending_classes.push(class.clone());
                continue;
            }
            let should_declare = precomputed_declare_flags
                .as_ref()
                .map(|flags| flags[decl_index])
                .unwrap_or(true);
            if !should_declare {
                continue;
            }
            match &decl.node {
                Decl::Function(func) => {
                    let declare_started_at = Instant::now();
                    self.declare_function(func)?;
                    decl_pass_function_work_ns += elapsed_nanos_u64(declare_started_at);
                    declared_function_count += 1;
                }
                Decl::Class(_) => {}
                Decl::Enum(_) => {}
                Decl::Interface(_) => {} // Interfaces don't generate code
                Decl::Module(_) => {}
                Decl::Import(_) => {} // Handled at file level
            }
        }
        let mut module_enums = Vec::new();
        let mut module_classes = Vec::new();
        for decl in &program.declarations {
            if let Decl::Module(module) = &decl.node {
                Self::collect_module_enums_with_prefix(module, &module.name, &mut module_enums);
                Self::collect_module_classes_with_prefix(module, &module.name, &mut module_classes);
            }
        }
        for en in &module_enums {
            self.declare_enum(en)?;
        }
        pending_classes.extend(module_classes);
        while !pending_classes.is_empty() {
            let mut progress = false;
            let mut next_pending = Vec::new();
            for class in pending_classes {
                let declare_started_at = Instant::now();
                let parent_ready = class.extends.as_ref().is_none_or(|parent| {
                    self.classes.contains_key(parent)
                        || self.materialize_generic_parent_class_info(parent).is_some()
                });
                if parent_ready {
                    self.declare_class(&class)?;
                    decl_pass_class_work_ns += elapsed_nanos_u64(declare_started_at);
                    declared_class_count += 1;
                    progress = true;
                } else {
                    next_pending.push(class);
                }
            }
            if !progress {
                for class in next_pending {
                    let declare_started_at = Instant::now();
                    self.declare_class(&class)?;
                    decl_pass_class_work_ns += elapsed_nanos_u64(declare_started_at);
                    declared_class_count += 1;
                }
                break;
            }
            pending_classes = next_pending;
        }
        for (decl_index, decl) in program.declarations.iter().enumerate() {
            let should_declare = precomputed_declare_flags
                .as_ref()
                .map(|flags| flags[decl_index])
                .unwrap_or(true);
            if !should_declare {
                continue;
            }
            if let Decl::Module(module) = &decl.node {
                let declare_started_at = Instant::now();
                self.declare_module_functions_with_prefix(module, &module.name)?;
                decl_pass_module_work_ns += elapsed_nanos_u64(declare_started_at);
                declared_module_count += 1;
            }
        }
        CODEGEN_PHASE_TIMING_TOTALS
            .decl_pass_ns
            .fetch_add(elapsed_nanos_u64(decl_pass_started_at), Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .decl_pass_decl_filter_ns
            .fetch_add(decl_pass_decl_filter_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .decl_pass_class_work_ns
            .fetch_add(decl_pass_class_work_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .decl_pass_function_work_ns
            .fetch_add(decl_pass_function_work_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .decl_pass_module_work_ns
            .fetch_add(decl_pass_module_work_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .declared_class_count
            .fetch_add(declared_class_count, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .declared_function_count
            .fetch_add(declared_function_count, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .declared_module_count
            .fetch_add(declared_module_count, Ordering::Relaxed);

        // Second pass: compile function bodies
        let body_pass_started_at = Instant::now();
        let body_pass_decl_filter_ns = top_level_decl_filter_ns;
        let mut body_pass_function_work_ns = 0_u64;
        let mut body_pass_class_work_ns = 0_u64;
        let mut body_pass_module_work_ns = 0_u64;
        let mut compiled_function_count = 0_usize;
        let mut compiled_class_count = 0_usize;
        let mut compiled_module_count = 0_usize;
        for (decl_index, decl) in program.declarations.iter().enumerate() {
            let should_compile = precomputed_body_flags
                .as_ref()
                .map(|flags| flags[decl_index])
                .unwrap_or(true);
            if !should_compile {
                continue;
            }
            match &decl.node {
                Decl::Function(func) => {
                    let compile_started_at = Instant::now();
                    self.compile_function(func)?;
                    body_pass_function_work_ns += elapsed_nanos_u64(compile_started_at);
                    compiled_function_count += 1;
                }
                Decl::Class(class) => {
                    let compile_started_at = Instant::now();
                    if let Some(symbols) = specialized_active_symbols.as_ref() {
                        self.compile_class_filtered(class, symbols)?;
                    } else {
                        self.compile_class(class)?;
                    }
                    body_pass_class_work_ns += elapsed_nanos_u64(compile_started_at);
                    compiled_class_count += 1;
                }
                Decl::Enum(_) => {}
                Decl::Interface(_) => {} // Interfaces don't generate code
                Decl::Module(_) => {}
                Decl::Import(_) => {} // Handled at file level
            }
        }
        for (decl_index, decl) in program.declarations.iter().enumerate() {
            let should_compile = precomputed_body_flags
                .as_ref()
                .map(|flags| flags[decl_index])
                .unwrap_or(true);
            if !should_compile {
                continue;
            }
            if let Decl::Module(module) = &decl.node {
                let compile_started_at = Instant::now();
                if let Some(symbols) = specialized_active_symbols.as_ref() {
                    self.compile_module_filtered(module, symbols)?;
                } else {
                    self.compile_module(module)?;
                }
                body_pass_module_work_ns += elapsed_nanos_u64(compile_started_at);
                compiled_module_count += 1;
            }
        }
        CODEGEN_PHASE_TIMING_TOTALS
            .body_pass_ns
            .fetch_add(elapsed_nanos_u64(body_pass_started_at), Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .body_pass_decl_filter_ns
            .fetch_add(body_pass_decl_filter_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .body_pass_function_work_ns
            .fetch_add(body_pass_function_work_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .body_pass_class_work_ns
            .fetch_add(body_pass_class_work_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .body_pass_module_work_ns
            .fetch_add(body_pass_module_work_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .compiled_function_count
            .fetch_add(compiled_function_count, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .compiled_class_count
            .fetch_add(compiled_class_count, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .compiled_module_count
            .fetch_add(compiled_module_count, Ordering::Relaxed);

        Ok(())
    }

    fn program_is_top_level_function_only(program: &Program) -> bool {
        program
            .declarations
            .iter()
            .all(|decl| matches!(decl.node, Decl::Function(_) | Decl::Import(_)))
    }

    fn collect_top_level_codegen_import_aliases(
        &mut self,
        declarations: &[Spanned<Decl>],
    ) -> usize {
        let mut import_alias_count = 0_usize;
        for decl in declarations {
            if let Decl::Import(import) = &decl.node {
                if let Some(alias) = &import.alias {
                    self.import_aliases
                        .entry(alias.clone())
                        .or_default()
                        .push((None, import.path.clone()));
                    import_alias_count += 1;
                } else if import.path.ends_with(".*") {
                    self.import_aliases
                        .entry(import.path.clone())
                        .or_default()
                        .push((None, import.path.clone()));
                    import_alias_count += 1;
                }
            }
        }
        import_alias_count
    }

    fn compile_top_level_function_only_program(
        &mut self,
        program: &Program,
        specialized_active_symbols: Option<&HashSet<String>>,
        specialized_declaration_symbols: Option<&HashSet<String>>,
    ) -> Result<()> {
        let decl_pass_started_at = Instant::now();
        let mut decl_pass_decl_filter_ns = 0_u64;
        let mut decl_pass_function_work_ns = 0_u64;
        let mut declared_function_count = 0_usize;
        for decl in &program.declarations {
            let Decl::Function(func) = &decl.node else {
                continue;
            };
            let decl_filter_started_at = Instant::now();
            let should_declare = specialized_declaration_symbols
                .as_ref()
                .map(|symbols| symbols.contains(&func.name) || func.name.contains("__spec__"))
                .unwrap_or(true);
            decl_pass_decl_filter_ns += elapsed_nanos_u64(decl_filter_started_at);
            if !should_declare {
                continue;
            }
            let declare_started_at = Instant::now();
            self.declare_function(func)?;
            decl_pass_function_work_ns += elapsed_nanos_u64(declare_started_at);
            declared_function_count += 1;
        }
        CODEGEN_PHASE_TIMING_TOTALS
            .decl_pass_ns
            .fetch_add(elapsed_nanos_u64(decl_pass_started_at), Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .decl_pass_decl_filter_ns
            .fetch_add(decl_pass_decl_filter_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .decl_pass_function_work_ns
            .fetch_add(decl_pass_function_work_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .declared_function_count
            .fetch_add(declared_function_count, Ordering::Relaxed);

        let body_pass_started_at = Instant::now();
        let mut body_pass_decl_filter_ns = 0_u64;
        let mut body_pass_function_work_ns = 0_u64;
        let mut compiled_function_count = 0_usize;
        for decl in &program.declarations {
            let Decl::Function(func) = &decl.node else {
                continue;
            };
            let decl_filter_started_at = Instant::now();
            let should_compile = specialized_active_symbols
                .as_ref()
                .map(|symbols| symbols.contains(&func.name) || func.name.contains("__spec__"))
                .unwrap_or(true);
            body_pass_decl_filter_ns += elapsed_nanos_u64(decl_filter_started_at);
            if !should_compile {
                continue;
            }
            let compile_started_at = Instant::now();
            self.compile_function(func)?;
            body_pass_function_work_ns += elapsed_nanos_u64(compile_started_at);
            compiled_function_count += 1;
        }
        CODEGEN_PHASE_TIMING_TOTALS
            .body_pass_ns
            .fetch_add(elapsed_nanos_u64(body_pass_started_at), Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .body_pass_decl_filter_ns
            .fetch_add(body_pass_decl_filter_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .body_pass_function_work_ns
            .fetch_add(body_pass_function_work_ns, Ordering::Relaxed);
        CODEGEN_PHASE_TIMING_TOTALS
            .compiled_function_count
            .fetch_add(compiled_function_count, Ordering::Relaxed);

        Ok(())
    }

    fn should_compile_decl(&self, decl: &Decl, active_symbols: &HashSet<String>) -> bool {
        fn class_has_active_method_symbol(
            class_name: &str,
            active_symbols: &HashSet<String>,
        ) -> bool {
            let method_prefix = format!("{}__", class_name);
            active_symbols
                .iter()
                .any(|symbol| symbol.starts_with(&method_prefix))
        }

        fn module_has_active_symbol(
            module: &ModuleDecl,
            prefix: &str,
            active_symbols: &HashSet<String>,
        ) -> bool {
            if active_symbols.contains(prefix) {
                return true;
            }
            for inner in &module.declarations {
                match &inner.node {
                    Decl::Function(func) => {
                        let prefixed = format!("{}__{}", prefix, func.name);
                        if active_symbols.contains(&prefixed) || prefixed.contains("__spec__") {
                            return true;
                        }
                    }
                    Decl::Class(class) => {
                        let class_name = format!("{}__{}", prefix, class.name);
                        if active_symbols.contains(&class_name)
                            || class_name.contains("__spec__")
                            || class_has_active_method_symbol(&class_name, active_symbols)
                        {
                            return true;
                        }
                    }
                    Decl::Enum(en) => {
                        if active_symbols.contains(&format!("{}__{}", prefix, en.name)) {
                            return true;
                        }
                    }
                    Decl::Module(nested) => {
                        let nested_prefix = format!("{}__{}", prefix, nested.name);
                        if module_has_active_symbol(nested, &nested_prefix, active_symbols) {
                            return true;
                        }
                    }
                    Decl::Interface(_) | Decl::Import(_) => {}
                }
            }
            false
        }

        match decl {
            Decl::Function(func) => {
                active_symbols.contains(&func.name) || func.name.contains("__spec__")
            }
            Decl::Class(class) => {
                active_symbols.contains(&class.name)
                    || class.name.contains("__spec__")
                    || class_has_active_method_symbol(&class.name, active_symbols)
            }
            Decl::Module(module) => module_has_active_symbol(module, &module.name, active_symbols),
            Decl::Enum(en) => active_symbols.contains(&en.name),
            Decl::Interface(_) | Decl::Import(_) => false,
        }
    }

    fn should_emit_decl_body(&self, decl: &Decl, active_symbols: &HashSet<String>) -> bool {
        fn class_has_active_method_symbol(
            class_name: &str,
            active_symbols: &HashSet<String>,
        ) -> bool {
            let method_prefix = format!("{}__", class_name);
            active_symbols
                .iter()
                .any(|symbol| symbol.starts_with(&method_prefix))
        }

        fn module_has_active_symbol(
            module: &ModuleDecl,
            prefix: &str,
            active_symbols: &HashSet<String>,
        ) -> bool {
            if active_symbols.contains(prefix) {
                return true;
            }
            for inner in &module.declarations {
                match &inner.node {
                    Decl::Function(func) => {
                        if active_symbols.contains(&format!("{}__{}", prefix, func.name)) {
                            return true;
                        }
                    }
                    Decl::Class(class) => {
                        let class_name = format!("{}__{}", prefix, class.name);
                        if active_symbols.contains(&class_name)
                            || class_has_active_method_symbol(&class_name, active_symbols)
                        {
                            return true;
                        }
                    }
                    Decl::Enum(en) => {
                        if active_symbols.contains(&format!("{}__{}", prefix, en.name)) {
                            return true;
                        }
                    }
                    Decl::Module(nested) => {
                        let nested_prefix = format!("{}__{}", prefix, nested.name);
                        if module_has_active_symbol(nested, &nested_prefix, active_symbols) {
                            return true;
                        }
                    }
                    Decl::Interface(_) | Decl::Import(_) => {}
                }
            }
            false
        }

        match decl {
            Decl::Function(func) => {
                active_symbols.contains(&func.name) || func.name.contains("__spec__")
            }
            Decl::Class(class) => {
                active_symbols.contains(&class.name)
                    || class.name.contains("__spec__")
                    || class_has_active_method_symbol(&class.name, active_symbols)
            }
            Decl::Module(module) => module_has_active_symbol(module, &module.name, active_symbols),
            Decl::Enum(en) => active_symbols.contains(&en.name),
            Decl::Interface(_) | Decl::Import(_) => false,
        }
    }

    pub(crate) fn resolve_module_alias(&self, name: &str) -> String {
        if let Some(path) = self.lookup_import_alias_path(name) {
            if !path.ends_with(".*") {
                let exact_module = self
                    .current_package_relative_path(path)
                    .unwrap_or(path)
                    .replace('.', "__");
                let module_prefix = format!("{}__", exact_module);
                if self
                    .functions
                    .keys()
                    .any(|candidate| candidate.starts_with(&module_prefix))
                    || self
                        .classes
                        .keys()
                        .any(|candidate| candidate.starts_with(&module_prefix))
                    || self
                        .enums
                        .keys()
                        .any(|candidate| candidate.starts_with(&module_prefix))
                {
                    return exact_module;
                }
            }
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
            if let Some(owner) = owner {
                return owner;
            }
        }
        name.to_string()
    }

    pub(crate) fn resolve_function_alias(&self, name: &str) -> String {
        let Some(path) = self.lookup_import_alias_path(name) else {
            return self
                .resolve_wildcard_import_symbol(name)
                .unwrap_or_else(|| name.to_string());
        };
        if let Some(canonical) = builtin_exact_import_alias_canonical(path) {
            return canonical.to_string();
        }
        if path.ends_with(".*") {
            return name.to_string();
        }
        let mut parts = path.split('.').collect::<Vec<_>>();
        let Some(symbol) = parts.pop() else {
            return name.to_string();
        };
        let namespace = parts.join(".");

        if let Some(canonical) = stdlib_registry().resolve_alias_call(&namespace, symbol) {
            return canonical;
        }

        let full_mangled = self
            .current_package_relative_path(path)
            .unwrap_or(path)
            .replace('.', "__");
        if self.functions.contains_key(&full_mangled) {
            return full_mangled;
        }
        if stdlib_registry()
            .get_namespace(symbol)
            .is_some_and(|owner| owner == &namespace)
        {
            return symbol.to_string();
        }

        name.to_string()
    }

    fn resolve_wildcard_import_symbol(&self, ident: &str) -> Option<String> {
        if self.variables.contains_key(ident) {
            return None;
        }
        let mut matches = self
            .visible_wildcard_import_paths()
            .into_iter()
            .filter_map(|path| path.strip_suffix(".*"))
            .filter_map(|namespace| {
                stdlib_registry()
                    .resolve_alias_call(namespace, ident)
                    .or_else(|| {
                        let candidate = format!("{}__{}", namespace.replace('.', "__"), ident);
                        self.functions.contains_key(&candidate).then_some(candidate)
                    })
            })
            .collect::<Vec<_>>();
        matches.sort_unstable();
        matches.dedup();
        (matches.len() == 1).then(|| matches[0].clone())
    }

    pub(crate) fn resolve_import_alias_variant(
        &self,
        alias_ident: &str,
    ) -> Option<(String, String)> {
        if self.variables.contains_key(alias_ident) {
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
        if matches.len() == 1 {
            return Some((matches[0].clone(), variant_name.to_string()));
        }
        matches!(enum_name.as_str(), "Option" | "Result")
            .then(|| (enum_name, variant_name.to_string()))
    }

    pub(crate) fn resolve_pattern_variant_alias(
        &self,
        alias_ident: &str,
    ) -> Option<(String, String, bool)> {
        if let Some((enum_name, variant_name)) = self.resolve_import_alias_variant(alias_ident) {
            if matches!(enum_name.as_str(), "Option" | "Result") {
                return Some((
                    enum_name,
                    variant_name.clone(),
                    matches!(variant_name.as_str(), "None"),
                ));
            }
            let variant_info = self.enums.get(&enum_name)?.variants.get(&variant_name)?;
            return Some((enum_name, variant_name, variant_info.fields.is_empty()));
        }

        match self.resolve_function_alias(alias_ident).as_str() {
            "Option__some" => Some(("Option".to_string(), "Some".to_string(), false)),
            "Option__none" => Some(("Option".to_string(), "None".to_string(), true)),
            "Result__ok" => Some(("Result".to_string(), "Ok".to_string(), false)),
            "Result__error" => Some(("Result".to_string(), "Error".to_string(), false)),
            _ => None,
        }
    }

    fn resolve_wildcard_import_module_function_candidate(
        &self,
        module_name: &str,
        member_parts: &[String],
    ) -> Option<String> {
        if member_parts.is_empty() || self.variables.contains_key(module_name) {
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

    fn resolve_import_alias_module_function_candidate(
        &self,
        alias_ident: &str,
        member_parts: &[String],
    ) -> Option<String> {
        if member_parts.is_empty() || self.variables.contains_key(alias_ident) {
            return None;
        }
        let path = self.lookup_import_alias_path(alias_ident)?;
        if path.ends_with(".*") {
            return None;
        }
        let full_path = format!("{}.{}", path, member_parts.join("."));
        if let Some(canonical) = builtin_exact_import_alias_canonical(&full_path) {
            return Some(canonical.to_string());
        }
        Some(format!(
            "{}__{}",
            path.replace('.', "__"),
            member_parts.join("__")
        ))
    }

    pub(crate) fn parse_construct_nominal_type_source(ty: &str) -> Option<(String, Vec<Type>)> {
        let trimmed = ty.trim();
        if let Some(start) = trimmed.find('<') {
            let end = trimmed.rfind('>')?;
            if end <= start {
                return None;
            }
            let name = trimmed[..start].trim();
            if name.is_empty() {
                return None;
            }
            let args = split_generic_args_static(&trimmed[start + 1..end])
                .into_iter()
                .map(|arg| parse_type_source(&arg).ok())
                .collect::<Option<Vec<_>>>()?;
            Some((name.to_string(), args))
        } else {
            match parse_type_source(trimmed).ok()? {
                Type::Named(name) => Some((name, Vec::new())),
                Type::Generic(name, args) => Some((name, args)),
                _ => Some((trimmed.to_string(), Vec::new())),
            }
        }
    }

    pub(crate) fn resolve_contextual_function_value_name(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Ident(name) => {
                let resolved = self.resolve_function_alias(name);
                if resolved != *name {
                    Some(resolved)
                } else if self.functions.contains_key(name)
                    || Self::is_supported_builtin_function_name(name)
                {
                    Some(name.clone())
                } else {
                    None
                }
            }
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
                            let full_path = format!("{}.{}", path, path_parts[1..].join("."));
                            if let Some(canonical) =
                                builtin_exact_import_alias_canonical(&full_path)
                            {
                                return Some(canonical.to_string());
                            }
                            let candidate = format!(
                                "{}__{}",
                                path.replace('.', "__"),
                                path_parts[1..].join("__")
                            );
                            if self.functions.contains_key(&candidate) {
                                return Some(candidate);
                            }
                        }
                        if let Some(candidate) = self
                            .resolve_wildcard_import_module_function_candidate(
                                &path_parts[0],
                                &path_parts[1..],
                            )
                        {
                            return Some(candidate);
                        }

                        if path_parts.len() == 2 {
                            let owner = self.resolve_module_alias(&path_parts[0]);
                            if let Some(static_container_name) =
                                builtin_exact_import_alias_canonical(&format!(
                                    "{}.{}",
                                    owner, field
                                ))
                            {
                                return Some(static_container_name.to_string());
                            }
                            let builtin_name = format!("{}__{}", owner, field);
                            if Self::is_supported_builtin_function_name(&builtin_name) {
                                return Some(builtin_name);
                            }
                        }

                        let mangled = path_parts.join("__");
                        if self.functions.contains_key(&mangled) {
                            return Some(mangled);
                        }
                    }
                }
                if let Expr::Ident(owner_name) = &object.node {
                    let owner = self.resolve_module_alias(owner_name);
                    if let Some(static_container_name) =
                        builtin_exact_import_alias_canonical(&format!("{}.{}", owner, field))
                    {
                        return Some(static_container_name.to_string());
                    }
                    let builtin_name = format!("{}__{}", owner, field);
                    if Self::is_supported_builtin_function_name(&builtin_name) {
                        return Some(builtin_name);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn resolve_class_constructor_function_value(
        &self,
        expr: &Expr,
        explicit_type_args: Option<&[Type]>,
        expected_ty: &Type,
    ) -> Option<(String, Type)> {
        fn class_template_family(name: &str) -> &str {
            name.split("__spec__")
                .next()
                .unwrap_or(name)
                .split('<')
                .next()
                .unwrap_or(name)
        }
        let mut type_source = match expr {
            Expr::Ident(name) => {
                if let Some(path) = self.lookup_import_alias_path(name) {
                    if !path.ends_with(".*") && self.canonical_codegen_type_name(path).is_some() {
                        path.to_string()
                    } else {
                        self.resolve_alias_qualified_codegen_type_name(name)
                            .unwrap_or_else(|| name.clone())
                    }
                } else {
                    self.resolve_alias_qualified_codegen_type_name(name)
                        .unwrap_or_else(|| name.clone())
                }
            }
            Expr::Field { .. } => flatten_field_chain(expr)?.join("."),
            _ => return None,
        };
        if let Some(type_args) = explicit_type_args {
            type_source = format!(
                "{}<{}>",
                type_source,
                type_args
                    .iter()
                    .map(Self::format_type_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        } else if let Type::Function(_, ret) = expected_ty {
            if let Type::Named(expected_name) | Type::Generic(expected_name, _) = ret.as_ref() {
                let resolved_base = self.resolve_alias_qualified_codegen_type_name(&type_source)?;
                let expected_base = class_template_family(expected_name);
                let resolved_base_name = class_template_family(&resolved_base);
                if expected_base == resolved_base_name {
                    type_source = Self::format_type_string(ret.as_ref());
                }
            }
        }

        let parsed_ty = parse_type_source(&type_source).ok()?;
        let normalized_ty = self.normalize_codegen_type(&parsed_ty);
        let (class_name, class_args) = match &normalized_ty {
            Type::Named(name) => {
                if let Type::Function(_, ret) = expected_ty {
                    if let Type::Generic(expected_name, expected_args) = ret.as_ref() {
                        if expected_name == name {
                            (name.clone(), expected_args.clone())
                        } else {
                            (name.clone(), Vec::new())
                        }
                    } else {
                        (name.clone(), Vec::new())
                    }
                } else {
                    (name.clone(), Vec::new())
                }
            }
            Type::Generic(name, args) => (name.clone(), args.clone()),
            _ => return None,
        };
        let func_name = format!("{}__new", class_name);
        let (func_name, unresolved_ty) = if let Some((_, ty)) = self.functions.get(&func_name) {
            (func_name, ty.clone())
        } else if let Some((base_name, _)) = class_name.split_once("__spec__") {
            let base_func_name = format!("{}__new", base_name);
            let (_, ty) = self.functions.get(&base_func_name)?;
            (base_func_name, ty.clone())
        } else {
            return None;
        };

        let mut bindings = HashMap::new();
        if let Some(class_info) = self.classes.get(&class_name).or_else(|| {
            class_name
                .split_once("__spec__")
                .and_then(|(base_name, _)| self.classes.get(base_name))
        }) {
            for (generic_name, arg) in class_info.generic_params.iter().zip(class_args.iter()) {
                bindings.insert(generic_name.clone(), arg.clone());
            }
        }

        let actual_ty = if bindings.is_empty() {
            unresolved_ty
        } else {
            let substituted = Self::substitute_type(&unresolved_ty, &bindings);
            match substituted {
                Type::Function(params, _) => Type::Function(params, Box::new(normalized_ty)),
                _ => substituted,
            }
        };

        Some((func_name, actual_ty))
    }

    fn compile_class_constructor_function_value_with_expected_type(
        &mut self,
        expr: &Expr,
        explicit_type_args: Option<&[Type]>,
        expected_ty: &Type,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let Some((name, actual_ty)) =
            self.resolve_class_constructor_function_value(expr, explicit_type_args, expected_ty)
        else {
            return Ok(None);
        };
        let Some((func, _)) = self.functions.get(&name).cloned() else {
            return Ok(None);
        };
        let struct_ty = self.llvm_type(&actual_ty).into_struct_type();
        let mut closure = struct_ty.get_undef();
        let fn_ptr = func.as_global_value().as_pointer_value();
        let null_env = self.context.ptr_type(AddressSpace::default()).const_null();

        closure = self
            .builder
            .build_insert_value(closure, fn_ptr, 0, "ctor_fn")
            .map_err(|e| CodegenError::new(format!("failed to build constructor closure fn: {e}")))?
            .into_struct_value();
        closure = self
            .builder
            .build_insert_value(closure, null_env, 1, "ctor_env")
            .map_err(|e| {
                CodegenError::new(format!("failed to build constructor closure env: {e}"))
            })?
            .into_struct_value();

        if &actual_ty == expected_ty {
            return Ok(Some(closure.into()));
        }

        if let Some(adapted) = self.compile_function_value_adapter_from_closure(
            closure.into(),
            &actual_ty,
            expected_ty,
        )? {
            return Ok(Some(adapted));
        }

        Err(Self::function_value_signature_mismatch_error(
            &actual_ty,
            expected_ty,
        ))
    }

    fn is_supported_builtin_function_name(name: &str) -> bool {
        Self::builtin_matches_expected_function_type(
            name,
            &Type::Function(vec![], Box::new(Type::None)),
        ) || matches!(
            name,
            "read_line"
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
                | "File__read"
                | "File__write"
                | "File__exists"
                | "File__delete"
                | "System__getenv"
                | "System__shell"
                | "System__exec"
                | "System__cwd"
                | "System__os"
                | "Time__unix"
                | "Time__sleep"
                | "Time__now"
                | "Args__count"
                | "Args__get"
                | "Str__len"
                | "Str__compare"
                | "Str__concat"
                | "Str__upper"
                | "Str__lower"
                | "Str__trim"
                | "Str__contains"
                | "Str__startsWith"
                | "Str__endsWith"
                | "Option__some"
                | "Option__none"
                | "Result__ok"
                | "Result__error"
        )
    }

    fn builtin_matches_expected_function_type(name: &str, expected: &Type) -> bool {
        let Type::Function(params, ret) = expected else {
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
                    && matches!(ret.as_ref(), Type::Option(inner) if params[0] == inner.as_ref().clone())
            }
            "Option__none" => params.is_empty() && matches!(ret.as_ref(), Type::Option(_)),
            "Result__ok" => {
                params.len() == 1
                    && matches!(ret.as_ref(), Type::Result(ok, _) if params[0] == ok.as_ref().clone())
            }
            "Result__error" => {
                params.len() == 1
                    && matches!(ret.as_ref(), Type::Result(_, err) if params[0] == err.as_ref().clone())
            }
            "read_line" | "System__cwd" | "System__os" => {
                params.is_empty() && matches!(ret.as_ref(), Type::String)
            }
            "File__read" | "System__getenv" | "Time__now" => {
                params.len() == 1
                    && matches!(params[0], Type::String)
                    && matches!(ret.as_ref(), Type::String)
            }
            "System__shell" => {
                params.len() == 1
                    && matches!(params[0], Type::String)
                    && matches!(ret.as_ref(), Type::Integer)
            }
            "System__exec" => {
                params.len() == 1
                    && matches!(params[0], Type::String)
                    && matches!(ret.as_ref(), Type::String)
            }
            "File__write" => {
                params.len() == 2
                    && matches!(params[0], Type::String)
                    && matches!(params[1], Type::String)
                    && matches!(ret.as_ref(), Type::Boolean)
            }
            "File__exists" | "File__delete" => {
                params.len() == 1
                    && matches!(params[0], Type::String)
                    && matches!(ret.as_ref(), Type::Boolean)
            }
            "System__exit" | "exit" | "Time__sleep" => {
                params.len() == 1
                    && matches!(params[0], Type::Integer)
                    && matches!(ret.as_ref(), Type::None)
            }
            "Time__unix" | "Args__count" => {
                params.is_empty() && matches!(ret.as_ref(), Type::Integer)
            }
            "Args__get" => {
                params.len() == 1
                    && matches!(params[0], Type::Integer)
                    && matches!(ret.as_ref(), Type::String)
            }
            "Math__abs" => {
                params.len() == 1
                    && params[0] == ret.as_ref().clone()
                    && matches!(params[0], Type::Integer | Type::Float)
            }
            "Math__min" | "Math__max" => {
                (params.len() == 2
                    && params[0] == params[1]
                    && params[0] == ret.as_ref().clone()
                    && matches!(params[0], Type::Integer | Type::Float))
                    || (params.len() == 2
                        && matches!(params[0], Type::Integer | Type::Float)
                        && matches!(params[1], Type::Integer | Type::Float)
                        && params[0] != params[1]
                        && matches!(ret.as_ref(), Type::Float))
            }
            "Math__pow" => {
                params.len() == 2
                    && matches!(params[0], Type::Integer | Type::Float)
                    && matches!(params[1], Type::Integer | Type::Float)
                    && matches!(ret.as_ref(), Type::Float)
            }
            "Math__sqrt" | "Math__sin" | "Math__cos" | "Math__tan" | "Math__floor"
            | "Math__ceil" | "Math__round" | "Math__log" | "Math__log10" | "Math__exp" => {
                params.len() == 1
                    && matches!(params[0], Type::Integer | Type::Float)
                    && matches!(ret.as_ref(), Type::Float)
            }
            "Math__pi" | "Math__e" | "Math__random" => {
                params.is_empty() && matches!(ret.as_ref(), Type::Float)
            }
            "Str__len" => {
                params.len() == 1
                    && matches!(params[0], Type::String)
                    && matches!(ret.as_ref(), Type::Integer)
            }
            "Str__compare" => {
                params.len() == 2
                    && matches!(params[0], Type::String)
                    && matches!(params[1], Type::String)
                    && matches!(ret.as_ref(), Type::Integer)
            }
            "Str__concat" => {
                params.len() == 2
                    && matches!(params[0], Type::String)
                    && matches!(params[1], Type::String)
                    && matches!(ret.as_ref(), Type::String)
            }
            "Str__upper" | "Str__lower" | "Str__trim" => {
                params.len() == 1
                    && matches!(params[0], Type::String)
                    && matches!(ret.as_ref(), Type::String)
            }
            "Str__contains" | "Str__startsWith" | "Str__endsWith" => {
                params.len() == 2
                    && matches!(params[0], Type::String)
                    && matches!(params[1], Type::String)
                    && matches!(ret.as_ref(), Type::Boolean)
            }
            "to_float" => {
                params.len() == 1
                    && matches!(params[0], Type::Integer | Type::Float)
                    && matches!(ret.as_ref(), Type::Float)
            }
            "to_int" => {
                params.len() == 1
                    && matches!(params[0], Type::Integer | Type::Float | Type::String)
                    && matches!(ret.as_ref(), Type::Integer)
            }
            "to_string" => {
                params.len() == 1
                    && matches!(
                        params[0],
                        Type::Integer
                            | Type::Float
                            | Type::Boolean
                            | Type::String
                            | Type::Char
                            | Type::None
                    )
                    && matches!(ret.as_ref(), Type::String)
            }
            "assert" | "assert_true" | "assert_false" => {
                params.len() == 1
                    && matches!(params[0], Type::Boolean)
                    && matches!(ret.as_ref(), Type::None)
            }
            "fail" => {
                (params.is_empty() || (params.len() == 1 && matches!(params[0], Type::String)))
                    && matches!(ret.as_ref(), Type::None)
            }
            "assert_eq" | "assert_ne" => {
                params.len() == 2
                    && (params[0] == params[1]
                        || (matches!(params[0], Type::Integer) && matches!(params[1], Type::Float))
                        || (matches!(params[0], Type::Float) && matches!(params[1], Type::Integer)))
                    && matches!(ret.as_ref(), Type::None)
            }
            "range" => {
                ((params.len() == 2 || params.len() == 3)
                    && params.iter().all(|param| matches!(param, Type::Integer))
                    && matches!(
                        ret.as_ref(),
                        Type::Range(inner) if matches!(inner.as_ref(), Type::Integer)
                    ))
                    || ((params.len() == 2 || params.len() == 3)
                        && params.iter().all(|param| matches!(param, Type::Float))
                        && matches!(
                            ret.as_ref(),
                            Type::Range(inner) if matches!(inner.as_ref(), Type::Float)
                        ))
            }
            _ => false,
        }
    }

    pub(crate) fn resolve_method_function_name(
        &self,
        class_name: &str,
        method: &str,
    ) -> Option<String> {
        let mut current = class_name.to_string();
        let mut depth = 0usize;
        while depth < 64 {
            let candidate = format!("{}__{}", current, method);
            if self.functions.contains_key(&candidate) {
                return Some(candidate);
            }
            let next = self.classes.get(&current)?.extends.clone();
            match next {
                Some(parent) => current = parent,
                None => break,
            }
            depth += 1;
        }
        None
    }

    pub(crate) fn resolve_unit_enum_variant_owner(&self, expr: &Expr) -> Option<String> {
        let path_parts = flatten_field_chain(expr)?;
        if path_parts.len() < 2 {
            return None;
        }

        let owner_path = if let Some(alias_path) = self.lookup_import_alias_path(&path_parts[0]) {
            if path_parts.len() == 2 {
                alias_path.to_string()
            } else {
                format!(
                    "{}.{}",
                    alias_path,
                    path_parts[1..path_parts.len() - 1].join(".")
                )
            }
        } else if path_parts.len() == 2 {
            self.resolve_module_alias(&path_parts[0])
        } else {
            path_parts[..path_parts.len() - 1].join(".")
        };
        let variant_name = path_parts.last()?;
        let canonical_owner = self.canonical_codegen_type_name(&owner_path)?;
        self.enums
            .get(&canonical_owner)
            .and_then(|enum_info| enum_info.variants.get(variant_name))
            .filter(|variant_info| variant_info.fields.is_empty())
            .map(|_| canonical_owner)
    }

    pub fn declare_enum(&mut self, en: &EnumDecl) -> Result<()> {
        if !en.generic_params.is_empty() {
            let params = en
                .generic_params
                .iter()
                .map(|param| param.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(CodegenError::new(format!(
                "Enum '{}' uses generic parameters ({params}), but user-defined generic enums are not supported yet",
                en.name
            )));
        }

        let payload_slots = en
            .variants
            .iter()
            .map(|v| v.fields.len())
            .max()
            .unwrap_or(0);

        // Runtime representation:
        // { tag: i8, payload_0: i64, payload_1: i64, ... }
        let mut fields: Vec<BasicTypeEnum<'ctx>> = vec![self.context.i8_type().into()];
        for _ in 0..payload_slots {
            fields.push(self.context.i64_type().into());
        }
        let struct_type = self.context.struct_type(&fields, false);

        let mut variants = HashMap::new();
        for (i, variant) in en.variants.iter().enumerate() {
            variants.insert(
                variant.name.clone(),
                EnumVariantInfo {
                    tag: i as u8,
                    fields: variant
                        .fields
                        .iter()
                        .map(|f| self.normalize_codegen_type(&f.ty))
                        .collect(),
                },
            );
            self.enum_variant_to_enum
                .insert(variant.name.clone(), en.name.clone());
        }

        self.enums.insert(
            en.name.clone(),
            EnumInfo {
                struct_type,
                payload_slots,
                variants,
            },
        );

        Ok(())
    }

    pub fn compile_module(&mut self, module: &ModuleDecl) -> Result<()> {
        self.compile_module_with_prefix(module, &module.name)
    }

    fn compile_module_filtered(
        &mut self,
        module: &ModuleDecl,
        active_symbols: &HashSet<String>,
    ) -> Result<()> {
        self.compile_module_filtered_with_prefix(module, &module.name, active_symbols)
    }

    fn collect_module_enums_with_prefix(
        module: &ModuleDecl,
        prefix: &str,
        out: &mut Vec<EnumDecl>,
    ) {
        for decl in &module.declarations {
            match &decl.node {
                Decl::Enum(en) => {
                    let mut prefixed_enum = en.clone();
                    prefixed_enum.name = format!("{}__{}", prefix, en.name);
                    out.push(prefixed_enum);
                }
                Decl::Module(nested) => {
                    let nested_prefix = format!("{}__{}", prefix, nested.name);
                    Self::collect_module_enums_with_prefix(nested, &nested_prefix, out);
                }
                _ => {}
            }
        }
    }

    fn collect_module_classes_with_prefix(
        module: &ModuleDecl,
        prefix: &str,
        out: &mut Vec<ClassDecl>,
    ) {
        for decl in &module.declarations {
            match &decl.node {
                Decl::Class(class) => {
                    let mut prefixed_class = class.clone();
                    prefixed_class.name = format!("{}__{}", prefix, class.name);
                    out.push(prefixed_class);
                }
                Decl::Module(nested) => {
                    let nested_prefix = format!("{}__{}", prefix, nested.name);
                    Self::collect_module_classes_with_prefix(nested, &nested_prefix, out);
                }
                _ => {}
            }
        }
    }

    fn declare_module_functions_with_prefix(
        &mut self,
        module: &ModuleDecl,
        prefix: &str,
    ) -> Result<()> {
        for decl in &module.declarations {
            match &decl.node {
                Decl::Function(func) => {
                    let mut prefixed_func = func.clone();
                    prefixed_func.name = format!("{}__{}", prefix, func.name);
                    self.declare_function(&prefixed_func)?;
                }
                Decl::Module(nested) => {
                    let nested_prefix = format!("{}__{}", prefix, nested.name);
                    self.declare_module_functions_with_prefix(nested, &nested_prefix)?;
                }
                Decl::Class(_) | Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
            }
        }
        Ok(())
    }

    fn compile_module_with_prefix(&mut self, module: &ModuleDecl, prefix: &str) -> Result<()> {
        let saved_module_prefix = self.current_module_prefix.clone();
        self.current_module_prefix = Some(prefix.to_string());
        for decl in &module.declarations {
            match &decl.node {
                Decl::Function(func) => {
                    let mut prefixed_func = func.clone();
                    prefixed_func.name = format!("{}__{}", prefix, func.name);
                    if let Err(err) = self.compile_function(&prefixed_func) {
                        self.current_module_prefix = saved_module_prefix;
                        return Err(err);
                    }
                }
                Decl::Class(class) => {
                    let mut prefixed_class = class.clone();
                    prefixed_class.name = format!("{}__{}", prefix, class.name);
                    if let Err(err) = self.compile_class(&prefixed_class) {
                        self.current_module_prefix = saved_module_prefix;
                        return Err(err);
                    }
                }
                Decl::Enum(_) => {}
                Decl::Module(nested) => {
                    let nested_prefix = format!("{}__{}", prefix, nested.name);
                    if let Err(err) = self.compile_module_with_prefix(nested, &nested_prefix) {
                        self.current_module_prefix = saved_module_prefix;
                        return Err(err);
                    }
                }
                Decl::Interface(_) | Decl::Import(_) => {}
            }
        }
        self.current_module_prefix = saved_module_prefix;
        Ok(())
    }

    fn compile_module_filtered_with_prefix(
        &mut self,
        module: &ModuleDecl,
        prefix: &str,
        active_symbols: &HashSet<String>,
    ) -> Result<()> {
        fn class_has_active_method_symbol(
            class_name: &str,
            active_symbols: &HashSet<String>,
        ) -> bool {
            let method_prefix = format!("{}__", class_name);
            active_symbols
                .iter()
                .any(|symbol| symbol.starts_with(&method_prefix))
        }

        if active_symbols.contains(prefix) {
            return self.compile_module_with_prefix(module, prefix);
        }

        let saved_module_prefix = self.current_module_prefix.clone();
        self.current_module_prefix = Some(prefix.to_string());
        for decl in &module.declarations {
            match &decl.node {
                Decl::Function(func) => {
                    let prefixed = format!("{}__{}", prefix, func.name);
                    if active_symbols.contains(&prefixed) || prefixed.contains("__spec__") {
                        let mut prefixed_func = func.clone();
                        prefixed_func.name = prefixed;
                        if let Err(err) = self.compile_function(&prefixed_func) {
                            self.current_module_prefix = saved_module_prefix;
                            return Err(err);
                        }
                    }
                }
                Decl::Class(class) => {
                    let prefixed = format!("{}__{}", prefix, class.name);
                    if active_symbols.contains(&prefixed)
                        || prefixed.contains("__spec__")
                        || class_has_active_method_symbol(&prefixed, active_symbols)
                    {
                        let mut prefixed_class = class.clone();
                        prefixed_class.name = prefixed;
                        if let Err(err) =
                            self.compile_class_filtered(&prefixed_class, active_symbols)
                        {
                            self.current_module_prefix = saved_module_prefix;
                            return Err(err);
                        }
                    }
                }
                Decl::Enum(_) => {}
                Decl::Module(nested) => {
                    let nested_prefix = format!("{}__{}", prefix, nested.name);
                    if let Err(err) = self.compile_module_filtered_with_prefix(
                        nested,
                        &nested_prefix,
                        active_symbols,
                    ) {
                        self.current_module_prefix = saved_module_prefix;
                        return Err(err);
                    }
                }
                Decl::Interface(_) | Decl::Import(_) => {}
            }
        }
        self.current_module_prefix = saved_module_prefix;
        Ok(())
    }

    // === Type System ===

    pub fn llvm_type(&self, ty: &Type) -> BasicTypeEnum<'ctx> {
        match ty {
            Type::Integer => self.context.i64_type().into(),
            Type::Float => self.context.f64_type().into(),
            Type::Boolean => self.context.bool_type().into(),
            Type::String => self.context.ptr_type(AddressSpace::default()).into(),
            Type::Char => self.context.i32_type().into(),
            Type::None => self.context.i8_type().into(),
            Type::Named(name) => {
                if let Some(enum_info) = self.enums.get(name) {
                    enum_info.struct_type.into()
                } else {
                    self.context.ptr_type(AddressSpace::default()).into()
                }
            }
            Type::Generic(_, _) => self.context.ptr_type(AddressSpace::default()).into(),
            Type::Function(_, _) => self
                .context
                .struct_type(
                    &[
                        self.context.ptr_type(AddressSpace::default()).into(), // function pointer
                        self.context.ptr_type(AddressSpace::default()).into(), // environment pointer
                    ],
                    false,
                )
                .into(),
            // Option<T> is represented as a struct { is_some: i8, value: T }
            Type::Option(inner) => {
                let inner_ty = self.llvm_type(inner);
                self.context
                    .struct_type(
                        &[
                            self.context.i8_type().into(), // tag: 0=None, 1=Some
                            inner_ty,                      // value
                        ],
                        false,
                    )
                    .into()
            }
            // Result<T, E> is represented as struct { is_ok: i8, ok_value: T, err_value: E }
            Type::Result(ok_ty, err_ty) => {
                let ok_llvm = self.llvm_type(ok_ty);
                let err_llvm = self.llvm_type(err_ty);
                self.context
                    .struct_type(
                        &[
                            self.context.i8_type().into(), // tag: 1=Ok, 0=Error
                            ok_llvm,                       // ok value
                            err_llvm,                      // error value
                        ],
                        false,
                    )
                    .into()
            }
            // List<T> is represented as struct { capacity: i64, length: i64, data: ptr }
            Type::List(_) => {
                self.context
                    .struct_type(
                        &[
                            self.context.i64_type().into(),                        // capacity
                            self.context.i64_type().into(),                        // length
                            self.context.ptr_type(AddressSpace::default()).into(), // data pointer
                        ],
                        false,
                    )
                    .into()
            }
            // Map<K, V> - for now just a pointer (will need proper implementation)
            Type::Map(_, _) => {
                self.context
                    .struct_type(
                        &[
                            self.context.i64_type().into(),                        // capacity
                            self.context.i64_type().into(),                        // length
                            self.context.ptr_type(AddressSpace::default()).into(), // keys pointer
                            self.context.ptr_type(AddressSpace::default()).into(), // values pointer
                        ],
                        false,
                    )
                    .into()
            }
            // Set<T> - similar to List
            Type::Set(_) => {
                self.context
                    .struct_type(
                        &[
                            self.context.i64_type().into(),                        // capacity
                            self.context.i64_type().into(),                        // length
                            self.context.ptr_type(AddressSpace::default()).into(), // data pointer
                        ],
                        false,
                    )
                    .into()
            }
            // Reference types - represented as pointers
            Type::Ref(_) | Type::MutRef(_) => self.context.ptr_type(AddressSpace::default()).into(),
            // Smart pointers - all represented as pointers
            Type::Box(_) | Type::Rc(_) | Type::Arc(_) => {
                self.context.ptr_type(AddressSpace::default()).into()
            }
            Type::Ptr(_) => self.context.ptr_type(AddressSpace::default()).into(),
            // Task<T> - runtime task handle pointer
            Type::Task(_) => self.context.ptr_type(AddressSpace::default()).into(),
            // Range<T> - represented as a struct { start, end, step }
            Type::Range(_) => self.context.ptr_type(AddressSpace::default()).into(),
        }
    }

    pub(crate) fn normalize_codegen_type(&self, ty: &Type) -> Type {
        let normalize_builtin_named_generic = |builtin_name: &str, args: &[Type]| -> Option<Type> {
            let resolved_name = self
                .resolve_alias_qualified_codegen_type_name(builtin_name)
                .unwrap_or_else(|| builtin_name.to_string());
            self.normalize_user_defined_generic_type(&resolved_name, args)
        };
        match ty {
            Type::Named(name) => self
                .resolve_alias_qualified_codegen_type_name(name)
                .map(Type::Named)
                .unwrap_or_else(|| ty.clone()),
            Type::Generic(name, args) => {
                let normalized_args = args
                    .iter()
                    .map(|arg| self.normalize_codegen_type(arg))
                    .collect::<Vec<_>>();
                let resolved_name = self
                    .resolve_alias_qualified_codegen_type_name(name)
                    .unwrap_or_else(|| name.clone());
                self.normalize_user_defined_generic_type(&resolved_name, &normalized_args)
                    .unwrap_or(Type::Generic(resolved_name, normalized_args))
            }
            Type::Function(params, ret) => Type::Function(
                params
                    .iter()
                    .map(|param| self.normalize_codegen_type(param))
                    .collect(),
                Box::new(self.normalize_codegen_type(ret)),
            ),
            Type::Option(inner) => {
                let inner = self.normalize_codegen_type(inner);
                normalize_builtin_named_generic("Option", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| Type::Option(Box::new(inner)))
            }
            Type::Result(ok, err) => {
                let ok = self.normalize_codegen_type(ok);
                let err = self.normalize_codegen_type(err);
                normalize_builtin_named_generic("Result", &[ok.clone(), err.clone()])
                    .unwrap_or_else(|| Type::Result(Box::new(ok), Box::new(err)))
            }
            Type::List(inner) => {
                let inner = self.normalize_codegen_type(inner);
                normalize_builtin_named_generic("List", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| Type::List(Box::new(inner)))
            }
            Type::Map(k, v) => {
                let k = self.normalize_codegen_type(k);
                let v = self.normalize_codegen_type(v);
                normalize_builtin_named_generic("Map", &[k.clone(), v.clone()])
                    .unwrap_or_else(|| Type::Map(Box::new(k), Box::new(v)))
            }
            Type::Set(inner) => {
                let inner = self.normalize_codegen_type(inner);
                normalize_builtin_named_generic("Set", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| Type::Set(Box::new(inner)))
            }
            Type::Ref(inner) => Type::Ref(Box::new(self.normalize_codegen_type(inner))),
            Type::MutRef(inner) => Type::MutRef(Box::new(self.normalize_codegen_type(inner))),
            Type::Box(inner) => {
                let inner = self.normalize_codegen_type(inner);
                normalize_builtin_named_generic("Box", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| Type::Box(Box::new(inner)))
            }
            Type::Rc(inner) => {
                let inner = self.normalize_codegen_type(inner);
                normalize_builtin_named_generic("Rc", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| Type::Rc(Box::new(inner)))
            }
            Type::Arc(inner) => {
                let inner = self.normalize_codegen_type(inner);
                normalize_builtin_named_generic("Arc", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| Type::Arc(Box::new(inner)))
            }
            Type::Ptr(inner) => {
                let inner = self.normalize_codegen_type(inner);
                normalize_builtin_named_generic("Ptr", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| Type::Ptr(Box::new(inner)))
            }
            Type::Task(inner) => {
                let inner = self.normalize_codegen_type(inner);
                normalize_builtin_named_generic("Task", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| Type::Task(Box::new(inner)))
            }
            Type::Range(inner) => {
                let inner = self.normalize_codegen_type(inner);
                normalize_builtin_named_generic("Range", std::slice::from_ref(&inner))
                    .unwrap_or_else(|| Type::Range(Box::new(inner)))
            }
            _ => ty.clone(),
        }
    }

    fn materialize_generic_parent_class_info(&mut self, parent: &str) -> Option<()> {
        if self.classes.contains_key(parent) {
            return Some(());
        }

        let (base_name, args) =
            if let Ok(Type::Generic(base_name, args)) = parse_type_source(parent) {
                (base_name, args)
            } else if let Some((base_name, suffixes)) = parent.split_once("__spec__") {
                let (decoded_args, consumed) = Self::decode_generic_specialization_args(suffixes)
                    .into_iter()
                    .find(|(_, consumed)| *consumed == suffixes.len())?;
                if consumed != suffixes.len() {
                    return None;
                }
                let parsed_args = decoded_args
                    .into_iter()
                    .map(|arg| parse_type_source(&arg).ok())
                    .collect::<Option<Vec<_>>>()?;
                (base_name.to_string(), parsed_args)
            } else {
                return None;
            };

        let base_info = self.classes.get(&base_name)?;
        if base_info.generic_params.len() != args.len() {
            return None;
        }

        let bindings = base_info
            .generic_params
            .iter()
            .cloned()
            .zip(args)
            .collect::<HashMap<_, _>>();
        let field_indices = base_info.field_indices.clone();
        let field_types = base_info
            .field_types
            .iter()
            .map(|(name, ty)| (name.clone(), Self::substitute_type(ty, &bindings)))
            .collect::<HashMap<_, _>>();
        let mut ordered_fields = field_indices
            .iter()
            .map(|(name, idx)| (name.clone(), *idx))
            .collect::<Vec<_>>();
        ordered_fields.sort_by_key(|(_, idx)| *idx);
        let field_llvm_types = ordered_fields
            .iter()
            .filter_map(|(name, _)| field_types.get(name).map(|ty| self.llvm_type(ty)))
            .collect::<Vec<_>>();
        let extends = base_info.extends.as_ref().map(|extends| {
            parse_type_source(extends)
                .ok()
                .map(|parsed| Self::format_type_string(&Self::substitute_type(&parsed, &bindings)))
                .unwrap_or_else(|| extends.clone())
        });

        self.classes.insert(
            parent.to_string(),
            ClassInfo {
                struct_type: self.context.struct_type(&field_llvm_types, false),
                field_indices,
                field_types,
                generic_params: Vec::new(),
                extends,
            },
        );
        Some(())
    }

    // === Classes ===

    pub fn declare_class(&mut self, class: &ClassDecl) -> Result<()> {
        let mut field_llvm_types: Vec<BasicTypeEnum<'ctx>> = Vec::new();
        let mut field_indices: HashMap<String, u32> = HashMap::new();
        let mut field_types_map: HashMap<String, Type> = HashMap::new();

        let mut next_index = 0u32;
        if let Some(parent) = &class.extends {
            self.materialize_generic_parent_class_info(parent);
            let parent_info = self
                .classes
                .get(parent)
                .ok_or_else(|| CodegenError::new(format!("Unknown base class: {}", parent)))?;
            let mut parent_fields = parent_info
                .field_indices
                .iter()
                .map(|(name, idx)| (name.clone(), *idx))
                .collect::<Vec<_>>();
            parent_fields.sort_by_key(|(_, idx)| *idx);

            for (name, idx) in parent_fields {
                let ty = parent_info
                    .field_types
                    .get(&name)
                    .ok_or_else(|| CodegenError::new("Missing inherited field type"))?
                    .clone();
                field_llvm_types.push(self.llvm_type(&ty));
                field_indices.insert(name.clone(), idx);
                field_types_map.insert(name, ty);
                next_index = next_index.max(idx + 1);
            }
        }

        for field in &class.fields {
            if field_indices.contains_key(&field.name) {
                return Err(CodegenError::new(format!(
                    "Field '{}' already exists in base class",
                    field.name
                )));
            }
            let i = next_index;
            let normalized_field_ty = self.normalize_codegen_type(&field.ty);
            field_llvm_types.push(self.llvm_type(&normalized_field_ty));
            field_indices.insert(field.name.clone(), i);
            field_types_map.insert(field.name.clone(), normalized_field_ty);
            next_index += 1;
        }

        let struct_type = self.context.struct_type(&field_llvm_types, false);
        self.classes.insert(
            class.name.clone(),
            ClassInfo {
                struct_type,
                field_indices,
                field_types: field_types_map,
                generic_params: class
                    .generic_params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect(),
                extends: class.extends.clone(),
            },
        );

        // Declare constructor (implicit default constructor when omitted)
        self.declare_class_constructor(class)?;

        // Declare methods
        for method in &class.methods {
            self.declare_class_method(class, method)?;
        }

        Ok(())
    }

    pub fn declare_class_constructor(&mut self, class: &ClassDecl) -> Result<()> {
        let ctor_params = class
            .constructor
            .as_ref()
            .map(|c| c.params.as_slice())
            .unwrap_or(&[]);
        let normalized_ctor_params = ctor_params
            .iter()
            .map(|p| self.normalize_codegen_type(&p.ty))
            .collect::<Vec<_>>();
        let ctor_param_modes = ctor_params.iter().map(|p| p.mode).collect::<Vec<_>>();
        let ctor_signature_params = normalized_ctor_params
            .iter()
            .zip(ctor_param_modes.iter())
            .map(|(ty, mode)| match mode {
                ParamMode::Owned => ty.clone(),
                ParamMode::Borrow => Type::Ref(Box::new(ty.clone())),
                ParamMode::BorrowMut => Type::MutRef(Box::new(ty.clone())),
            })
            .collect::<Vec<_>>();
        let param_types: Vec<BasicMetadataTypeEnum> = normalized_ctor_params
            .iter()
            .zip(ctor_param_modes.iter())
            .map(|(ty, mode)| match mode {
                ParamMode::Owned => self.llvm_type(ty).into(),
                ParamMode::Borrow | ParamMode::BorrowMut => {
                    self.context.ptr_type(AddressSpace::default()).into()
                }
            })
            .collect();

        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![
            self.context.ptr_type(AddressSpace::default()).into(), // env_ptr
        ];
        llvm_params.extend(param_types);

        let ret_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ret_type.fn_type(&llvm_params, false);

        let name = format!("{}__new", class.name);
        let func = self.module.add_function(&name, fn_type, None);
        if name.contains("__spec__") {
            func.set_linkage(Linkage::Internal);
        }
        self.functions.insert(
            name,
            (
                func,
                Type::Function(
                    ctor_signature_params,
                    Box::new(self.normalize_codegen_type(&Type::Named(class.name.clone()))),
                ),
            ),
        );
        self.function_param_modes
            .insert(format!("{}__new", class.name), ctor_param_modes);

        Ok(())
    }

    pub fn declare_class_method(&mut self, class: &ClassDecl, method: &FunctionDecl) -> Result<()> {
        let self_type = self.context.ptr_type(AddressSpace::default());

        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![
            self.context.ptr_type(AddressSpace::default()).into(), // env_ptr
            self_type.into(),                                      // this
        ];
        let normalized_params = method
            .params
            .iter()
            .map(|p| self.normalize_codegen_type(&p.ty))
            .collect::<Vec<_>>();
        let method_param_modes = method.params.iter().map(|p| p.mode).collect::<Vec<_>>();
        let method_signature_params = normalized_params
            .iter()
            .zip(method_param_modes.iter())
            .map(|(ty, mode)| match mode {
                ParamMode::Owned => ty.clone(),
                ParamMode::Borrow => Type::Ref(Box::new(ty.clone())),
                ParamMode::BorrowMut => Type::MutRef(Box::new(ty.clone())),
            })
            .collect::<Vec<_>>();
        for (param_ty, mode) in normalized_params.iter().zip(method_param_modes.iter()) {
            llvm_params.push(match mode {
                ParamMode::Owned => self.llvm_type(param_ty).into(),
                ParamMode::Borrow | ParamMode::BorrowMut => {
                    self.context.ptr_type(AddressSpace::default()).into()
                }
            });
        }

        let normalized_return = self.normalize_codegen_type(&method.return_type);
        let fn_type = match &normalized_return {
            Type::None => self.context.void_type().fn_type(&llvm_params, false),
            ty => self.llvm_type(ty).fn_type(&llvm_params, false),
        };

        let name = format!("{}__{}", class.name, method.name);
        let func = self.module.add_function(&name, fn_type, None);
        if name.contains("__spec__") {
            func.set_linkage(Linkage::Internal);
        }
        self.functions.insert(
            name,
            (
                func,
                Type::Function(method_signature_params, Box::new(normalized_return)),
            ),
        );
        self.function_param_modes.insert(
            format!("{}__{}", class.name, method.name),
            method_param_modes,
        );

        Ok(())
    }

    pub fn compile_class(&mut self, class: &ClassDecl) -> Result<()> {
        let implicit_constructor = Constructor {
            params: vec![],
            body: vec![],
        };
        self.compile_constructor(
            class,
            class.constructor.as_ref().unwrap_or(&implicit_constructor),
        )?;

        for method in &class.methods {
            self.compile_method(class, method)?;
        }

        Ok(())
    }

    fn compile_class_filtered(
        &mut self,
        class: &ClassDecl,
        active_symbols: &HashSet<String>,
    ) -> Result<()> {
        let ctor_name = format!("{}__new", class.name);
        let class_is_specialized = class.name.contains("__spec__");
        let compile_entire_specialized_class =
            class_is_specialized && active_symbols.contains(&class.name);
        if compile_entire_specialized_class
            || active_symbols.contains(&class.name)
            || active_symbols.contains(&ctor_name)
        {
            let implicit_constructor = Constructor {
                params: vec![],
                body: vec![],
            };
            self.compile_constructor(
                class,
                class.constructor.as_ref().unwrap_or(&implicit_constructor),
            )?;
        }

        for method in &class.methods {
            let method_name = format!("{}__{}", class.name, method.name);
            if compile_entire_specialized_class
                || active_symbols.contains(&class.name)
                || active_symbols.contains(&method_name)
            {
                self.compile_method(class, method)?;
            }
        }

        Ok(())
    }

    pub fn compile_constructor(
        &mut self,
        class: &ClassDecl,
        constructor: &Constructor,
    ) -> Result<()> {
        let name = format!("{}__new", class.name);
        let (func, _) =
            self.functions.get(&name).cloned().ok_or_else(|| {
                CodegenError::new(format!("Missing declared constructor: {}", name))
            })?;

        self.current_function = Some(func);
        self.current_return_type =
            Some(self.normalize_codegen_type(&Type::Named(class.name.clone())));
        let entry = self.context.append_basic_block(func, "entry");
        self.builder.position_at_end(entry);
        self.variables.clear();
        self.non_negative_locals.clear();
        self.non_zero_locals.clear();
        self.exact_integer_locals.clear();
        self.upper_bound_locals.clear();
        self.exact_list_lengths.clear();
        self.exact_list_capacities.clear();
        self.list_element_upper_bounds.clear();
        self.distinct_list_alloc_ids.clear();
        self.reset_current_generic_bounds();
        self.extend_current_generic_bounds(&class.generic_params);

        // Allocate parameters
        // Param 0 is env_ptr, constructor params start at 1
        for (i, param) in constructor.params.iter().enumerate() {
            let normalized_param_ty = self.normalize_codegen_type(&param.ty);
            let llvm_param = func.get_nth_param((i + 1) as u32).ok_or_else(|| {
                CodegenError::new(format!(
                    "Missing constructor parameter {} for {}",
                    i + 1,
                    class.name
                ))
            })?;
            let ptr = match param.mode {
                ParamMode::Owned => {
                    let alloca = self
                        .builder
                        .build_alloca(self.llvm_type(&normalized_param_ty), &param.name)
                        .map_err(|e| {
                            CodegenError::new(format!("alloca failed for '{}': {}", param.name, e))
                        })?;
                    self.builder.build_store(alloca, llvm_param).map_err(|e| {
                        CodegenError::new(format!("store failed for '{}': {}", param.name, e))
                    })?;
                    alloca
                }
                ParamMode::Borrow | ParamMode::BorrowMut => llvm_param.into_pointer_value(),
            };
            self.variables.insert(
                param.name.clone(),
                Variable {
                    ptr,
                    ty: normalized_param_ty,
                    mutable: param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                },
            );
        }

        // Allocate instance
        let class_info = self
            .classes
            .get(&class.name)
            .ok_or_else(|| CodegenError::new(format!("Unknown class: {}", class.name)))?;
        let struct_type = class_info.struct_type;
        let size = struct_type
            .size_of()
            .ok_or_else(|| CodegenError::new("Failed to compute class struct size"))?;
        let ptr =
            self.build_malloc_call(size, "instance", "malloc call failed for class instance")?;
        let instance =
            self.extract_call_pointer_value(ptr, "malloc call did not produce a pointer result")?;

        // Store 'this'
        let this_alloca = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "this")
            .map_err(|e| CodegenError::new(format!("alloca failed for this: {}", e)))?;
        self.builder
            .build_store(this_alloca, instance)
            .map_err(|e| CodegenError::new(format!("store failed for this: {}", e)))?;
        self.variables.insert(
            "this".to_string(),
            Variable {
                ptr: this_alloca,
                ty: Type::Named(class.name.clone()),
                mutable: false,
            },
        );

        // Compile body
        for stmt in &constructor.body {
            self.compile_stmt(&stmt.node)?;
        }

        // Return instance
        let this_val = self
            .builder
            .build_load(
                self.context.ptr_type(AddressSpace::default()),
                this_alloca,
                "this",
            )
            .map_err(|e| CodegenError::new(format!("load failed for this: {}", e)))?;
        self.builder
            .build_return(Some(&this_val))
            .map_err(|e| CodegenError::new(format!("return failed in constructor: {}", e)))?;

        self.current_function = None;
        self.current_return_type = None;
        self.reset_current_generic_bounds();
        Ok(())
    }

    pub fn compile_method(&mut self, class: &ClassDecl, method: &FunctionDecl) -> Result<()> {
        let name = format!("{}__{}", class.name, method.name);
        let (func, _) = self
            .functions
            .get(&name)
            .cloned()
            .ok_or_else(|| CodegenError::new(format!("Missing declared method: {}", name)))?;

        self.current_function = Some(func);
        self.current_return_type = Some(self.normalize_codegen_type(&method.return_type));
        let entry = self.context.append_basic_block(func, "entry");
        self.builder.position_at_end(entry);
        self.variables.clear();
        self.non_negative_locals.clear();
        self.non_zero_locals.clear();
        self.exact_integer_locals.clear();
        self.upper_bound_locals.clear();
        self.exact_list_lengths.clear();
        self.exact_list_capacities.clear();
        self.list_element_upper_bounds.clear();
        self.distinct_list_alloc_ids.clear();
        self.reset_current_generic_bounds();
        self.extend_current_generic_bounds(&class.generic_params);
        self.extend_current_generic_bounds(&method.generic_params);

        // Param 0 is env_ptr
        // Store 'this' (Param 1)
        let this_param = func.get_nth_param(1).ok_or_else(|| {
            CodegenError::new(format!("Missing 'this' param for method {}", name))
        })?;
        let class_info = self
            .classes
            .get(&class.name)
            .ok_or_else(|| CodegenError::new(format!("Unknown class: {}", class.name)))?;
        let _struct_type = class_info.struct_type;
        let this_alloca = self
            .builder
            .build_alloca(self.context.ptr_type(AddressSpace::default()), "this")
            .map_err(|e| CodegenError::new(format!("alloca failed for this: {}", e)))?;
        self.builder
            .build_store(this_alloca, this_param)
            .map_err(|e| CodegenError::new(format!("store failed for this: {}", e)))?;
        self.variables.insert(
            "this".to_string(),
            Variable {
                ptr: this_alloca,
                ty: Type::Named(class.name.clone()),
                mutable: false,
            },
        );

        // Store parameters
        // Start from index 2 because 0=env_ptr, 1=this
        for (i, param) in method.params.iter().enumerate() {
            let normalized_param_ty = self.normalize_codegen_type(&param.ty);
            let llvm_param = func.get_nth_param((i + 2) as u32).ok_or_else(|| {
                CodegenError::new(format!("Missing method parameter {} for {}", i + 2, name))
            })?;
            let ptr = match param.mode {
                ParamMode::Owned => {
                    let alloca = self
                        .builder
                        .build_alloca(self.llvm_type(&normalized_param_ty), &param.name)
                        .map_err(|e| {
                            CodegenError::new(format!("alloca failed for '{}': {}", param.name, e))
                        })?;
                    self.builder.build_store(alloca, llvm_param).map_err(|e| {
                        CodegenError::new(format!("store failed for '{}': {}", param.name, e))
                    })?;
                    alloca
                }
                ParamMode::Borrow | ParamMode::BorrowMut => llvm_param.into_pointer_value(),
            };
            self.variables.insert(
                param.name.clone(),
                Variable {
                    ptr,
                    ty: normalized_param_ty,
                    mutable: param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                },
            );
        }

        // Compile body
        for stmt in &method.body {
            self.compile_stmt(&stmt.node)?;
        }

        // Add implicit return
        if self.needs_terminator() {
            match &method.return_type {
                Type::None => {
                    self.builder.build_return(None).map_err(|e| {
                        CodegenError::new(format!("return failed in method {}: {}", name, e))
                    })?;
                }
                _ => {
                    self.builder.build_unreachable().map_err(|e| {
                        CodegenError::new(format!(
                            "unreachable emit failed in method {}: {}",
                            name, e
                        ))
                    })?;
                }
            }
        }

        self.current_function = None;
        self.current_return_type = None;
        self.reset_current_generic_bounds();
        Ok(())
    }

    // === Functions ===

    pub fn declare_function(&mut self, func: &FunctionDecl) -> Result<FunctionValue<'ctx>> {
        if func.is_extern {
            return self.declare_extern_function(func);
        }

        if func.is_async {
            if func.name == "main" {
                return Err(CodegenError::new(
                    "async main is not supported; use a sync main() and await tasks inside async functions",
                ));
            }
            return self.declare_async_function(func);
        }

        let normalized_params: Vec<Type> = func
            .params
            .iter()
            .map(|p| self.normalize_codegen_type(&p.ty))
            .collect();
        let param_modes: Vec<ParamMode> = func.params.iter().map(|p| p.mode).collect();
        let signature_params: Vec<Type> = normalized_params
            .iter()
            .zip(param_modes.iter())
            .map(|(ty, mode)| match mode {
                ParamMode::Owned => ty.clone(),
                ParamMode::Borrow => Type::Ref(Box::new(ty.clone())),
                ParamMode::BorrowMut => Type::MutRef(Box::new(ty.clone())),
            })
            .collect();
        let normalized_return = self.normalize_codegen_type(&func.return_type);

        let param_types: Vec<BasicMetadataTypeEnum> = normalized_params
            .iter()
            .zip(param_modes.iter())
            .map(|(ty, mode)| match mode {
                ParamMode::Owned => self.llvm_type(ty).into(),
                ParamMode::Borrow | ParamMode::BorrowMut => {
                    self.context.ptr_type(AddressSpace::default()).into()
                }
            })
            .collect();

        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![
            self.context.ptr_type(AddressSpace::default()).into(), // env_ptr
        ];
        llvm_params.extend(param_types);

        // Main function always returns i32 for C runtime compatibility
        let fn_type = if func.name == "main" {
            // main(argc: i32, argv: i8**)
            let main_params: Vec<BasicMetadataTypeEnum> = vec![
                self.context.i32_type().into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ];
            self.context.i32_type().fn_type(&main_params, false)
        } else {
            match &normalized_return {
                Type::None => self.context.void_type().fn_type(&llvm_params, false),
                ty => self.llvm_type(ty).fn_type(&llvm_params, false),
            }
        };

        let function = self.module.add_function(&func.name, fn_type, None);
        if func.name.contains("__spec__") {
            function.set_linkage(Linkage::Internal);
        }
        if Self::function_returns_provably_non_negative(func) {
            self.non_negative_functions.insert(func.name.clone());
        }

        // Add optimization attributes
        // Always inline small functions
        if func.params.len() <= 3 && !func.name.starts_with("main") {
            let always_inline = self
                .context
                .create_enum_attribute(Attribute::get_named_enum_kind_id("alwaysinline"), 0);
            function.add_attribute(AttributeLoc::Function, always_inline);
        }

        // Function doesn't unwind (no exceptions)
        let no_unwind = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("nounwind"), 0);
        function.add_attribute(AttributeLoc::Function, no_unwind);

        let must_progress = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("mustprogress"), 0);
        function.add_attribute(AttributeLoc::Function, must_progress);

        // Function will return (no infinite loops in analyzed functions)
        let will_return = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("willreturn"), 0);
        function.add_attribute(AttributeLoc::Function, will_return);

        self.functions.insert(
            func.name.clone(),
            (
                function,
                Type::Function(signature_params, Box::new(normalized_return)),
            ),
        );
        self.function_param_modes
            .insert(func.name.clone(), param_modes);
        Ok(function)
    }

    fn declare_extern_function(&mut self, func: &FunctionDecl) -> Result<FunctionValue<'ctx>> {
        let param_types: Vec<BasicMetadataTypeEnum> = func
            .params
            .iter()
            .map(|p| match p.mode {
                ParamMode::Owned => self.llvm_type(&p.ty).into(),
                ParamMode::Borrow | ParamMode::BorrowMut => {
                    self.context.ptr_type(AddressSpace::default()).into()
                }
            })
            .collect();

        let fn_type = match &func.return_type {
            Type::None => self
                .context
                .void_type()
                .fn_type(&param_types, func.is_variadic),
            ty => self.llvm_type(ty).fn_type(&param_types, func.is_variadic),
        };

        let symbol_name = func.extern_link_name.as_deref().unwrap_or(&func.name);
        let function = self.module.add_function(symbol_name, fn_type, None);
        match func.extern_abi.as_deref().unwrap_or("c") {
            // On current targets, C/system are both emitted as default C calling convention.
            "c" | "system" => {
                function.set_call_conventions(0);
            }
            _ => {}
        }
        self.extern_functions.insert(func.name.clone());
        self.functions.insert(
            func.name.clone(),
            (
                function,
                Type::Function(
                    func.params
                        .iter()
                        .map(|p| match p.mode {
                            ParamMode::Owned => p.ty.clone(),
                            ParamMode::Borrow => Type::Ref(Box::new(p.ty.clone())),
                            ParamMode::BorrowMut => Type::MutRef(Box::new(p.ty.clone())),
                        })
                        .collect(),
                    Box::new(func.return_type.clone()),
                ),
            ),
        );
        self.function_param_modes.insert(
            func.name.clone(),
            func.params.iter().map(|p| p.mode).collect(),
        );
        Ok(function)
    }

    fn task_struct_type(&self) -> StructType<'ctx> {
        let ptr = self.context.ptr_type(AddressSpace::default());
        self.context.struct_type(
            &[
                self.context.i64_type().into(),
                ptr.into(),
                self.context.i8_type().into(),
                self.context.i8_type().into(),
            ],
            false,
        )
    }

    fn async_inner_return_type(&self, ty: &Type) -> Type {
        if let Type::Task(inner) = ty {
            (**inner).clone()
        } else {
            ty.clone()
        }
    }

    fn build_atomic_bool_load(
        &mut self,
        ptr: PointerValue<'ctx>,
        name: &str,
        ordering: AtomicOrdering,
    ) -> Result<IntValue<'ctx>> {
        let raw = self
            .builder
            .build_load(self.context.i8_type(), ptr, name)
            .map_err(|e| CodegenError::new(format!("failed to build atomic bool load: {e}")))?
            .into_int_value();
        let block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::new("atomic load used outside basic block"))?;
        let inst = block
            .get_last_instruction()
            .ok_or_else(|| CodegenError::new("failed to capture atomic load instruction"))?;
        inst.set_atomic_ordering(ordering)
            .map_err(|e| CodegenError::new(format!("failed to set atomic load ordering: {e}")))?;
        self.builder
            .build_int_compare(
                IntPredicate::NE,
                raw,
                self.context.i8_type().const_zero(),
                &format!("{name}_bool"),
            )
            .map_err(|e| CodegenError::new(format!("failed to compare atomic bool load: {e}")))
    }

    fn build_atomic_bool_store(
        &mut self,
        ptr: PointerValue<'ctx>,
        value: IntValue<'ctx>,
        ordering: AtomicOrdering,
    ) -> Result<()> {
        let byte_value = self
            .builder
            .build_int_cast(value, self.context.i8_type(), "atomic_flag_store")
            .map_err(|e| {
                CodegenError::new(format!("failed to cast atomic bool store value: {e}"))
            })?;
        let inst = self
            .builder
            .build_store(ptr, byte_value)
            .map_err(|e| CodegenError::new(format!("failed to build atomic bool store: {e}")))?;
        inst.set_atomic_ordering(ordering)
            .map_err(|e| CodegenError::new(format!("failed to set atomic store ordering: {e}")))?;
        Ok(())
    }

    fn declare_async_function(&mut self, func: &FunctionDecl) -> Result<FunctionValue<'ctx>> {
        let inner_return = self.async_inner_return_type(&func.return_type);
        let task_return = Type::Task(Box::new(inner_return.clone()));
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let param_modes = func.params.iter().map(|p| p.mode).collect::<Vec<_>>();

        let mut wrapper_params: Vec<BasicMetadataTypeEnum> = vec![ptr_type.into()];
        let mut env_fields: Vec<BasicTypeEnum<'ctx>> = Vec::new();
        for param in &func.params {
            let llvm = self.llvm_type(&param.ty);
            match param.mode {
                ParamMode::Owned => {
                    wrapper_params.push(llvm.into());
                    env_fields.push(llvm);
                }
                ParamMode::Borrow | ParamMode::BorrowMut => {
                    wrapper_params.push(ptr_type.into());
                    env_fields.push(ptr_type.into());
                }
            }
        }
        env_fields.push(ptr_type.into());
        let env_type = self.context.struct_type(&env_fields, false);

        let wrapper_fn_type = ptr_type.fn_type(&wrapper_params, false);
        let wrapper = self.module.add_function(&func.name, wrapper_fn_type, None);

        let body_name = format!("__arden_async_body__{}", func.name);
        let body_fn_type = match &inner_return {
            Type::None => self.context.void_type().fn_type(&[ptr_type.into()], false),
            ty => self.llvm_type(ty).fn_type(&[ptr_type.into()], false),
        };
        let body = self.module.add_function(&body_name, body_fn_type, None);

        let thunk_name = format!("__arden_async_thunk__{}", func.name);
        #[cfg(windows)]
        let thunk_fn_type = self.context.i32_type().fn_type(&[ptr_type.into()], false);
        #[cfg(not(windows))]
        let thunk_fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        let thunk = self.module.add_function(&thunk_name, thunk_fn_type, None);

        self.functions.insert(
            func.name.clone(),
            (
                wrapper,
                Type::Function(
                    func.params
                        .iter()
                        .map(|p| match p.mode {
                            ParamMode::Owned => p.ty.clone(),
                            ParamMode::Borrow => Type::Ref(Box::new(p.ty.clone())),
                            ParamMode::BorrowMut => Type::MutRef(Box::new(p.ty.clone())),
                        })
                        .collect(),
                    Box::new(task_return),
                ),
            ),
        );
        self.function_param_modes
            .insert(func.name.clone(), param_modes);

        self.async_functions.insert(
            func.name.clone(),
            AsyncFunctionPlan {
                wrapper,
                body,
                thunk,
                env_type,
                inner_return_type: inner_return,
            },
        );

        Ok(wrapper)
    }

    fn create_task(
        &mut self,
        runner_fn: PointerValue<'ctx>,
        env_ptr: PointerValue<'ctx>,
        env_task_slot_ptr: PointerValue<'ctx>,
    ) -> Result<PointerValue<'ctx>> {
        let task_ty = self.task_struct_type();
        let size = task_ty
            .size_of()
            .ok_or_else(|| CodegenError::new("failed to compute Task runtime size"))?;

        let raw = self.build_malloc_call(size, "task_alloc", "failed to call malloc for Task")?;
        let task_raw =
            self.extract_call_pointer_value(raw, "malloc should return pointer for Task")?;

        let task_ptr = self
            .builder
            .build_pointer_cast(
                task_raw,
                self.context.ptr_type(AddressSpace::default()),
                "task_ptr",
            )
            .map_err(|e| {
                CodegenError::new(format!("failed to cast Task allocation pointer: {e}"))
            })?;

        let i32_ty = self.context.i32_type();
        let zero = i32_ty.const_int(0, false);
        let thread_idx = i32_ty.const_int(0, false);
        let result_idx = i32_ty.const_int(1, false);
        let done_idx = i32_ty.const_int(2, false);
        let completed_idx = i32_ty.const_int(3, false);

        let thread_field = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, thread_idx], "task_thread_field")
                .map_err(|e| CodegenError::new(format!("failed to get Task thread field: {e}")))?
        };
        self.builder
            .build_store(thread_field, self.context.i64_type().const_int(0, false))
            .map_err(|e| {
                CodegenError::new(format!("failed to initialize Task thread field: {e}"))
            })?;

        let result_field = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, result_idx], "task_result_ptr")
                .map_err(|e| CodegenError::new(format!("failed to get Task result field: {e}")))?
        };
        self.builder
            .build_store(
                result_field,
                self.context.ptr_type(AddressSpace::default()).const_null(),
            )
            .map_err(|e| {
                CodegenError::new(format!("failed to initialize Task result field: {e}"))
            })?;

        let done_field = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done")
                .map_err(|e| CodegenError::new(format!("failed to get Task done field: {e}")))?
        };
        self.builder
            .build_store(done_field, self.context.i8_type().const_int(0, false))
            .map_err(|e| CodegenError::new(format!("failed to initialize Task done field: {e}")))?;
        let completed_field = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, completed_idx], "task_completed")
                .map_err(|e| {
                    CodegenError::new(format!("failed to get Task completed field: {e}"))
                })?
        };
        self.builder
            .build_store(completed_field, self.context.i8_type().const_int(0, false))
            .map_err(|e| {
                CodegenError::new(format!("failed to initialize Task completed field: {e}"))
            })?;
        self.builder
            .build_store(env_task_slot_ptr, task_ptr)
            .map_err(|e| CodegenError::new(format!("failed to store Task handle in env: {e}")))?;

        let thread_val = {
            let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
            let start_fn = self
                .builder
                .build_pointer_cast(
                    runner_fn,
                    self.context.ptr_type(AddressSpace::default()),
                    "task_start_fn",
                )
                .map_err(|e| {
                    CodegenError::new(format!("failed to cast Task runner function: {e}"))
                })?;
            #[cfg(windows)]
            {
                let create_thread = self.get_or_declare_create_thread_win();
                let raw_handle = self
                    .builder
                    .build_call(
                        create_thread,
                        &[
                            null_ptr.into(),
                            self.context.i64_type().const_zero().into(),
                            start_fn.into(),
                            env_ptr.into(),
                            self.context.i32_type().const_zero().into(),
                            null_ptr.into(),
                        ],
                        "task_spawn",
                    )
                    .map_err(|e| {
                        CodegenError::new(format!("failed to spawn Windows task thread: {e}"))
                    })?;
                let handle = self
                    .extract_call_pointer_value(raw_handle, "CreateThread should return handle")?;
                self.builder
                    .build_ptr_to_int(handle, self.context.i64_type(), "task_thread")
                    .map_err(|e| {
                        CodegenError::new(format!("failed to convert Windows task handle: {e}"))
                    })?
            }
            #[cfg(not(windows))]
            {
                let pthread_create = self.get_or_declare_pthread_create();
                let pthread_t_ty = self.libc_ulong_type();
                let thread_tmp = self
                    .builder
                    .build_alloca(pthread_t_ty, "task_thread_tmp")
                    .map_err(|e| {
                        CodegenError::new(format!("failed to allocate task thread temp: {e}"))
                    })?;
                self.builder
                    .build_store(thread_tmp, pthread_t_ty.const_zero())
                    .map_err(|e| {
                        CodegenError::new(format!("failed to initialize task thread temp: {e}"))
                    })?;
                let _spawn_status = self
                    .builder
                    .build_call(
                        pthread_create,
                        &[
                            thread_tmp.into(),
                            null_ptr.into(),
                            start_fn.into(),
                            env_ptr.into(),
                        ],
                        "task_spawn",
                    )
                    .map_err(|e| CodegenError::new(format!("failed to spawn pthread task: {e}")))?;

                let pthread_id = self
                    .builder
                    .build_load(pthread_t_ty, thread_tmp, "task_thread")
                    .map_err(|e| {
                        CodegenError::new(format!("failed to load pthread task handle: {e}"))
                    })?
                    .into_int_value();
                self.builder
                    .build_int_cast(pthread_id, self.context.i64_type(), "task_thread_i64")
                    .map_err(|e| {
                        CodegenError::new(format!(
                            "failed to cast pthread task handle for task storage: {e}"
                        ))
                    })?
            }
        };
        self.builder
            .build_store(thread_field, thread_val)
            .map_err(|e| CodegenError::new(format!("failed to store Task thread handle: {e}")))?;

        self.builder
            .build_pointer_cast(
                task_ptr,
                self.context.ptr_type(AddressSpace::default()),
                "task_raw",
            )
            .map_err(|e| CodegenError::new(format!("failed to cast Task pointer for return: {e}")))
    }

    fn await_task(
        &mut self,
        task_raw: PointerValue<'ctx>,
        inner_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let task_ty = self.task_struct_type();
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let task_ptr = self
            .builder
            .build_pointer_cast(
                task_raw,
                self.context.ptr_type(AddressSpace::default()),
                "task_cast",
            )
            .map_err(|_| CodegenError::new("failed to cast awaited task pointer"))?;

        let i32_ty = self.context.i32_type();
        let zero = i32_ty.const_int(0, false);
        let thread_idx = i32_ty.const_int(0, false);
        let result_idx = i32_ty.const_int(1, false);
        let done_idx = i32_ty.const_int(2, false);

        let done_field = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done_ptr")
                .map_err(|_| CodegenError::new("failed to compute task done field pointer"))?
        };
        let done_val = self
            .builder
            .build_load(self.context.i8_type(), done_field, "task_done")
            .map_err(|_| CodegenError::new("failed to load task done flag"))?
            .into_int_value();
        let done_ready = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                done_val,
                self.context.i8_type().const_zero(),
                "task_done_ready",
            )
            .map_err(|_| CodegenError::new("failed to compare task done flag"))?;

        let result_field = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, result_idx], "task_result_field")
                .map_err(|_| CodegenError::new("failed to compute task result field pointer"))?
        };
        let existing_result = self
            .builder
            .build_load(ptr_ty, result_field, "task_result_existing")
            .map_err(|_| CodegenError::new("failed to load existing task result pointer"))?
            .into_pointer_value();

        let current_bb = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::new("await used outside of basic block"))?;
        let current_fn = self
            .current_function
            .ok_or_else(|| CodegenError::new("await used outside of function"))?;
        let join_bb = self.context.append_basic_block(current_fn, "task_join");
        let cont_bb = self.context.append_basic_block(current_fn, "task_cont");

        self.builder
            .build_conditional_branch(done_ready, cont_bb, join_bb)
            .map_err(|_| CodegenError::new("failed to branch in task await"))?;

        self.builder.position_at_end(join_bb);
        let thread_field = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, thread_idx], "task_thread_ptr")
                .map_err(|_| CodegenError::new("failed to compute task thread field pointer"))?
        };
        let thread_id = self
            .builder
            .build_load(self.context.i64_type(), thread_field, "task_thread_id")
            .map_err(|_| CodegenError::new("failed to load task thread id"))?
            .into_int_value();
        #[cfg(windows)]
        let new_result = {
            let wait_fn = self.get_or_declare_wait_for_single_object_win();
            let close_fn = self.get_or_declare_close_handle_win();
            let handle = self
                .builder
                .build_int_to_ptr(thread_id, ptr_ty, "task_thread_handle")
                .map_err(|_| CodegenError::new("failed to cast task handle on Windows"))?;
            self.builder
                .build_call(
                    wait_fn,
                    &[
                        handle.into(),
                        self.context.i32_type().const_all_ones().into(),
                    ],
                    "task_join_call",
                )
                .map_err(|_| CodegenError::new("failed to emit WaitForSingleObject call"))?;
            self.builder
                .build_call(close_fn, &[handle.into()], "")
                .map_err(|_| CodegenError::new("failed to emit CloseHandle call"))?;
            self.builder
                .build_store(thread_field, self.context.i64_type().const_zero())
                .map_err(|_| CodegenError::new("failed to clear Windows task handle"))?;
            self.builder
                .build_load(ptr_ty, result_field, "task_joined_result")
                .map_err(|_| CodegenError::new("failed to load joined task result on Windows"))?
                .into_pointer_value()
        };
        #[cfg(not(windows))]
        let new_result = {
            let pthread_join = self.get_or_declare_pthread_join();
            let pthread_t_ty = self.libc_ulong_type();
            let pthread_thread_id = self
                .builder
                .build_int_cast(thread_id, pthread_t_ty, "task_thread_pthread_t")
                .map_err(|_| CodegenError::new("failed to cast task thread id to pthread_t"))?;
            let join_result_ptr = self
                .builder
                .build_alloca(ptr_ty, "task_join_result")
                .map_err(|_| CodegenError::new("failed to allocate pthread join result slot"))?;
            self.builder
                .build_store(join_result_ptr, ptr_ty.const_null())
                .map_err(|_| CodegenError::new("failed to initialize pthread join result slot"))?;
            self.builder
                .build_call(
                    pthread_join,
                    &[pthread_thread_id.into(), join_result_ptr.into()],
                    "task_join_call",
                )
                .map_err(|_| CodegenError::new("failed to emit pthread_join call"))?;
            self.builder
                .build_load(ptr_ty, join_result_ptr, "task_joined_result")
                .map_err(|_| CodegenError::new("failed to load pthread join result"))?
                .into_pointer_value()
        };
        self.builder
            .build_store(result_field, new_result)
            .map_err(|_| CodegenError::new("failed to store joined task result"))?;
        self.builder
            .build_store(done_field, self.context.i8_type().const_int(1, false))
            .map_err(|_| CodegenError::new("failed to store completed task flag"))?;
        self.builder
            .build_unconditional_branch(cont_bb)
            .map_err(|_| CodegenError::new("failed to continue after task join"))?;

        self.builder.position_at_end(cont_bb);
        let phi = self
            .builder
            .build_phi(ptr_ty, "task_result_phi")
            .map_err(|_| CodegenError::new("failed to create task result phi"))?;
        phi.add_incoming(&[(&existing_result, current_bb), (&new_result, join_bb)]);
        let result_ptr = phi.as_basic_value().into_pointer_value();

        if matches!(inner_ty, Type::None) {
            return Ok(self.context.i8_type().const_int(0, false).into());
        }

        let result_ty = self.llvm_type(inner_ty);
        let typed_ptr = self
            .builder
            .build_pointer_cast(
                result_ptr,
                self.context.ptr_type(AddressSpace::default()),
                "task_result_typed",
            )
            .map_err(|_| CodegenError::new("failed to cast awaited task result pointer"))?;
        self.builder
            .build_load(result_ty, typed_ptr, "task_result")
            .map_err(|_| CodegenError::new("failed to load awaited task result"))
    }

    pub fn compile_function(&mut self, func: &FunctionDecl) -> Result<()> {
        if func.is_extern {
            return Ok(());
        }

        if func.is_async {
            return self.compile_async_function(func);
        }

        let (function, _) =
            self.functions.get(&func.name).cloned().ok_or_else(|| {
                CodegenError::new(format!("Missing compiled function {}", func.name))
            })?;

        let setup_started_at = Instant::now();
        self.current_function = Some(function);
        self.current_return_type = Some(self.normalize_codegen_type(&func.return_type));
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);
        self.variables.clear();
        self.non_negative_locals.clear();
        self.non_zero_locals.clear();
        self.exact_integer_locals.clear();
        self.upper_bound_locals.clear();
        self.exact_list_lengths.clear();
        self.exact_list_capacities.clear();
        self.list_element_upper_bounds.clear();
        self.distinct_list_alloc_ids.clear();
        self.loop_stack.clear();
        self.reset_current_generic_bounds();
        self.extend_current_generic_bounds(&func.generic_params);

        // Special handling for main: store argc/argv in globals
        if func.name == "main" {
            let argc = function
                .get_nth_param(0)
                .ok_or_else(|| CodegenError::new("compiled main missing argc parameter"))?
                .into_int_value();
            let argv = function
                .get_nth_param(1)
                .ok_or_else(|| CodegenError::new("compiled main missing argv parameter"))?
                .into_pointer_value();

            let argc_global = match self.module.get_global("_arden_argc") {
                Some(g) => g,
                None => {
                    let g = self
                        .module
                        .add_global(self.context.i32_type(), None, "_arden_argc");
                    g.set_initializer(&self.context.i32_type().const_int(0, false));
                    g
                }
            };
            self.builder
                .build_store(argc_global.as_pointer_value(), argc)
                .map_err(|_| CodegenError::new("failed to store argc global"))?;

            let argv_global = match self.module.get_global("_arden_argv") {
                Some(g) => g,
                None => {
                    let g = self.module.add_global(
                        self.context.ptr_type(AddressSpace::default()),
                        None,
                        "_arden_argv",
                    );
                    g.set_initializer(&self.context.ptr_type(AddressSpace::default()).const_null());
                    g
                }
            };
            self.builder
                .build_store(argv_global.as_pointer_value(), argv)
                .map_err(|_| CodegenError::new("failed to store argv global"))?;
        }
        CODEGEN_PHASE_TIMING_TOTALS
            .body_function_setup_ns
            .fetch_add(elapsed_nanos_u64(setup_started_at), Ordering::Relaxed);

        // Allocate parameters
        // Param 0 is argc for main, but for other functions 0 is env_ptr
        // We skip argc/argv for main in the regular parameter allocation loop
        // because main() in Arden is usually main(): None
        let param_alloc_started_at = Instant::now();
        let start_idx = if func.name == "main" { 2 } else { 1 };
        for (i, param) in func.params.iter().enumerate() {
            let normalized_param_ty = self.normalize_codegen_type(&param.ty);
            let llvm_param = function
                .get_nth_param((i + start_idx) as u32)
                .ok_or_else(|| {
                    CodegenError::new(format!(
                        "compiled function '{}' missing parameter '{}'",
                        func.name, param.name
                    ))
                })?;
            let ptr = match param.mode {
                ParamMode::Owned => {
                    let alloca = self
                        .builder
                        .build_alloca(self.llvm_type(&normalized_param_ty), &param.name)
                        .map_err(|_| {
                            CodegenError::new(format!(
                                "failed to allocate parameter '{}'",
                                param.name
                            ))
                        })?;
                    self.builder.build_store(alloca, llvm_param).map_err(|_| {
                        CodegenError::new(format!("failed to store parameter '{}'", param.name))
                    })?;
                    alloca
                }
                ParamMode::Borrow | ParamMode::BorrowMut => llvm_param.into_pointer_value(),
            };
            self.variables.insert(
                param.name.clone(),
                Variable {
                    ptr,
                    ty: normalized_param_ty,
                    mutable: param.mutable || matches!(param.mode, ParamMode::BorrowMut),
                },
            );
        }
        CODEGEN_PHASE_TIMING_TOTALS
            .body_function_param_alloc_ns
            .fetch_add(elapsed_nanos_u64(param_alloc_started_at), Ordering::Relaxed);

        // Compile body
        let stmt_loop_started_at = Instant::now();
        for stmt in &func.body {
            self.compile_stmt(&stmt.node)?;
        }
        CODEGEN_PHASE_TIMING_TOTALS
            .body_function_stmt_loop_ns
            .fetch_add(elapsed_nanos_u64(stmt_loop_started_at), Ordering::Relaxed);

        // Add implicit return
        let implicit_return_started_at = Instant::now();
        if self.needs_terminator() {
            if func.name == "main" {
                // Main returns 0 for success
                let zero = self.context.i32_type().const_int(0, false);
                self.builder
                    .build_return(Some(&zero))
                    .map_err(|_| CodegenError::new("failed to emit implicit main return"))?;
            } else {
                match &func.return_type {
                    Type::None => {
                        self.builder
                            .build_return(None)
                            .map_err(|_| CodegenError::new("failed to emit implicit return"))?;
                    }
                    _ => {
                        self.builder.build_unreachable().map_err(|_| {
                            CodegenError::new("failed to terminate missing return path")
                        })?;
                    }
                }
            }
        }
        CODEGEN_PHASE_TIMING_TOTALS
            .body_function_implicit_return_ns
            .fetch_add(
                elapsed_nanos_u64(implicit_return_started_at),
                Ordering::Relaxed,
            );

        self.current_function = None;
        self.reset_current_generic_bounds();
        Ok(())
    }

    // === Statements ===

    fn build_entry_alloca(
        &self,
        ty: BasicTypeEnum<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>> {
        let function = self
            .current_function
            .ok_or_else(|| CodegenError::new("attempted to allocate outside of a function"))?;
        let entry_block = function
            .get_first_basic_block()
            .ok_or_else(|| CodegenError::new("function is missing an entry basic block"))?;

        let alloc_builder = self.context.create_builder();
        if let Some(first_instruction) = entry_block.get_first_instruction() {
            alloc_builder.position_before(&first_instruction);
        } else {
            alloc_builder.position_at_end(entry_block);
        }

        alloc_builder
            .build_alloca(ty, name)
            .map_err(|_| CodegenError::new(format!("failed to allocate local variable '{}'", name)))
    }

    pub fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
            } => {
                let stmt_started_at = Instant::now();
                let normalized_ty = self.normalize_codegen_type(ty);
                let val = self.compile_expr_with_expected_type(&value.node, &normalized_ty)?;
                let actual_ty = self.infer_expr_type(&value.node, &[]);
                self.reject_incompatible_expected_type_value(&normalized_ty, &actual_ty, val)?;
                let alloca = self.build_entry_alloca(self.llvm_type(&normalized_ty), name)?;
                self.builder.build_store(alloca, val).map_err(|_| {
                    CodegenError::new(format!("failed to store local variable '{}'", name))
                })?;
                self.variables.insert(
                    name.clone(),
                    Variable {
                        ptr: alloca,
                        ty: normalized_ty,
                        mutable: *mutable,
                    },
                );
                self.update_binding_non_negative_fact(name, ty, &value.node);
                self.update_binding_list_alias_fact(name, ty, &value.node);
                if self.expr_creates_empty_list(&value.node, ty) {
                    self.exact_list_lengths.insert(name.clone(), 0);
                    self.exact_list_capacities.remove(name);
                    self.list_element_upper_bounds.remove(name);
                } else if let Expr::Ident(source_name) = &value.node {
                    if matches!(self.deref_codegen_type(ty), Type::List(_)) {
                        if let Some(upper_bound) =
                            self.list_element_upper_bounds.get(source_name).copied()
                        {
                            self.list_element_upper_bounds
                                .insert(name.clone(), upper_bound);
                        } else {
                            self.list_element_upper_bounds.remove(name);
                        }
                    } else {
                        self.list_element_upper_bounds.remove(name);
                    }
                } else {
                    self.exact_list_lengths.remove(name);
                    self.exact_list_capacities.remove(name);
                    self.list_element_upper_bounds.remove(name);
                }
                CODEGEN_PHASE_TIMING_TOTALS
                    .body_stmt_let_ns
                    .fetch_add(elapsed_nanos_u64(stmt_started_at), Ordering::Relaxed);
            }

            Stmt::Assign { target, value } => {
                let stmt_started_at = Instant::now();
                self.ensure_assignment_target_mutable(&target.node)?;
                if let Some((op, rhs)) =
                    Self::match_compound_assign_target(&target.node, &value.node)
                {
                    let is_map_index_target = matches!(
                        &target.node,
                        Expr::Index { object, .. }
                            if matches!(
                                self.infer_object_type(&object.node)
                                    .or_else(|| Some(self.infer_expr_type(&object.node, &[])))
                                    .map(|ty| self.deref_codegen_type(&ty).clone()),
                                Some(Type::Map(_, _))
                            )
                    );
                    if !is_map_index_target {
                        let target_ty = self.infer_expr_type(&target.node, &[]);
                        let rhs_ty = self.infer_builtin_argument_type(&rhs.node);
                        let ptr = self.compile_lvalue(&target.node)?;
                        let current = self
                            .builder
                            .build_load(self.llvm_type(&target_ty), ptr, "compound_current")
                            .map_err(|_| {
                                CodegenError::new(
                                    "failed to load current compound assignment value",
                                )
                            })?;
                        let rhs_value =
                            self.compile_expr_with_expected_type(&rhs.node, &target_ty)?;
                        let result = self
                            .compile_binary_values(op, current, rhs_value, &target_ty, &rhs_ty)?;
                        self.builder.build_store(ptr, result).map_err(|_| {
                            CodegenError::new("failed to store compound assignment result")
                        })?;
                        if let Expr::Ident(name) = &target.node {
                            self.update_binding_non_negative_fact(name, &target_ty, &value.node);
                            self.update_binding_list_alias_fact(name, &target_ty, &value.node);
                            self.exact_list_lengths.remove(name);
                            self.exact_list_capacities.remove(name);
                            self.list_element_upper_bounds.remove(name);
                        }
                        CODEGEN_PHASE_TIMING_TOTALS
                            .body_stmt_assign_ns
                            .fetch_add(elapsed_nanos_u64(stmt_started_at), Ordering::Relaxed);
                        return Ok(());
                    }
                }

                if let Expr::Index { object, index } = &target.node {
                    let object_ty = self
                        .infer_object_type(&object.node)
                        .or_else(|| Some(self.infer_expr_type(&object.node, &[])));
                    let deref_object_ty = object_ty
                        .clone()
                        .map(|ty| self.deref_codegen_type(&ty).clone());
                    if let Some(map_ty @ Type::Map(_, _)) = deref_object_ty {
                        let map_value =
                            if matches!(object_ty, Some(Type::Ref(_)) | Some(Type::MutRef(_))) {
                                self.compile_expr(&object.node)?
                            } else if let Ok(map_ptr) = self.compile_lvalue(&object.node) {
                                map_ptr.into()
                            } else {
                                self.compile_expr(&object.node)?
                            };
                        if let Some((op, rhs)) =
                            Self::match_compound_assign_target(&target.node, &value.node)
                        {
                            let Type::Map(key_ty, val_ty) = &map_ty else {
                                return Err(CodegenError::new(
                                    "internal error: expected map type for map compound assignment",
                                ));
                            };
                            let (key_ty, val_ty) = ((*key_ty.clone()), (*val_ty.clone()));
                            let key = self.compile_expr_with_expected_type(&index.node, &key_ty)?;
                            // Materialize the evaluated key once and reload it for get/set.
                            // This keeps side-effect semantics intact and avoids reusing a
                            // complex aggregate SSA value across two large map helper expansions.
                            let key_slot = self
                                .build_entry_alloca(self.llvm_type(&key_ty), "map_compound_key")?;
                            self.builder.build_store(key_slot, key).map_err(|_| {
                                CodegenError::new(
                                    "failed to store map compound-assignment key temporary",
                                )
                            })?;
                            let key_for_get = self
                                .builder
                                .build_load(self.llvm_type(&key_ty), key_slot, "map_key_get")
                                .map_err(|_| {
                                    CodegenError::new(
                                        "failed to reload map compound-assignment key for get",
                                    )
                                })?;
                            let current = self.compile_map_get_on_value_with_compiled_key(
                                map_value,
                                &map_ty,
                                key_for_get,
                            )?;
                            let rhs_value =
                                self.compile_expr_with_expected_type(&rhs.node, &val_ty)?;
                            let result = self
                                .compile_binary_values(op, current, rhs_value, &val_ty, &val_ty)?;
                            let key_for_set = self
                                .builder
                                .build_load(self.llvm_type(&key_ty), key_slot, "map_key_set")
                                .map_err(|_| {
                                    CodegenError::new(
                                        "failed to reload map compound-assignment key for set",
                                    )
                                })?;
                            self.compile_map_set_on_value_with_compiled_key_value(
                                map_value,
                                &map_ty,
                                key_for_set,
                                result,
                            )?;
                            CODEGEN_PHASE_TIMING_TOTALS
                                .body_stmt_assign_ns
                                .fetch_add(elapsed_nanos_u64(stmt_started_at), Ordering::Relaxed);
                            return Ok(());
                        }
                        let args = [
                            Spanned::new(index.node.clone(), index.span.clone()),
                            Spanned::new(value.node.clone(), value.span.clone()),
                        ];
                        self.compile_map_method_on_value(map_value, &map_ty, "set", &args)?;
                        CODEGEN_PHASE_TIMING_TOTALS
                            .body_stmt_assign_ns
                            .fetch_add(elapsed_nanos_u64(stmt_started_at), Ordering::Relaxed);
                        return Ok(());
                    }
                }

                let target_ty = self.infer_expr_type(&target.node, &[]);
                let ptr = self.compile_lvalue(&target.node)?;
                let val = self.compile_expr_with_expected_type(&value.node, &target_ty)?;
                let target_is_plain_scalar = matches!(
                    target_ty,
                    Type::Integer | Type::Float | Type::Boolean | Type::Char
                );
                if !target_is_plain_scalar || val.get_type() != self.llvm_type(&target_ty) {
                    let actual_ty = self.infer_expr_type(&value.node, &[]);
                    self.reject_incompatible_expected_type_value(&target_ty, &actual_ty, val)?;
                }
                self.builder
                    .build_store(ptr, val)
                    .map_err(|_| CodegenError::new("failed to store assignment value"))?;
                if let Expr::Ident(name) = &target.node {
                    self.update_binding_non_negative_fact(name, &target_ty, &value.node);
                    self.update_binding_list_alias_fact(name, &target_ty, &value.node);
                    if self.expr_creates_empty_list(&value.node, &target_ty) {
                        self.exact_list_lengths.insert(name.clone(), 0);
                        self.exact_list_capacities.remove(name);
                        self.list_element_upper_bounds.remove(name);
                    } else if let Expr::Ident(source_name) = &value.node {
                        if matches!(self.deref_codegen_type(&target_ty), Type::List(_)) {
                            if let Some(upper_bound) =
                                self.list_element_upper_bounds.get(source_name).copied()
                            {
                                self.list_element_upper_bounds
                                    .insert(name.clone(), upper_bound);
                            } else {
                                self.list_element_upper_bounds.remove(name);
                            }
                        } else {
                            self.list_element_upper_bounds.remove(name);
                        }
                    } else {
                        self.exact_list_lengths.remove(name);
                        self.exact_list_capacities.remove(name);
                        self.list_element_upper_bounds.remove(name);
                    }
                }
                CODEGEN_PHASE_TIMING_TOTALS
                    .body_stmt_assign_ns
                    .fetch_add(elapsed_nanos_u64(stmt_started_at), Ordering::Relaxed);
            }

            Stmt::Expr(expr) => {
                let stmt_started_at = Instant::now();
                self.compile_expr(&expr.node)?;
                CODEGEN_PHASE_TIMING_TOTALS
                    .body_stmt_expr_ns
                    .fetch_add(elapsed_nanos_u64(stmt_started_at), Ordering::Relaxed);
            }

            Stmt::Return(value) => {
                let stmt_started_at = Instant::now();
                // Check if we're in main function (returns i32)
                let is_main = self
                    .current_function
                    .map(|f| f.get_name().to_str().unwrap_or("") == "main")
                    .unwrap_or(false);

                match value {
                    Some(expr) => {
                        // Check if returning None literal
                        if matches!(&expr.node, Expr::Literal(Literal::None)) {
                            if is_main {
                                let zero = self.context.i32_type().const_int(0, false);
                                self.builder.build_return(Some(&zero)).map_err(|_| {
                                    CodegenError::new("failed to emit main return for None")
                                })?;
                            } else {
                                self.builder.build_return(None).map_err(|_| {
                                    CodegenError::new("failed to emit return for None")
                                })?;
                            }
                        } else {
                            let val = if let Some(ret_ty) = self.current_return_type.clone() {
                                let compiled =
                                    self.compile_expr_with_expected_type(&expr.node, &ret_ty)?;
                                let ret_is_plain_scalar = matches!(
                                    ret_ty,
                                    Type::Integer | Type::Float | Type::Boolean | Type::Char
                                );
                                if !ret_is_plain_scalar
                                    || compiled.get_type() != self.llvm_type(&ret_ty)
                                {
                                    let inferred_expr_ty = self.infer_expr_type(&expr.node, &[]);
                                    self.reject_incompatible_expected_type_value(
                                        &ret_ty,
                                        &inferred_expr_ty,
                                        compiled,
                                    )?;
                                }
                                compiled
                            } else {
                                self.compile_expr(&expr.node)?
                            };
                            // Main function must return i32 for C compatibility
                            let ret_val = if is_main && val.is_int_value() {
                                let int_val = val.into_int_value();
                                if int_val.get_type().get_bit_width() != 32 {
                                    self.builder
                                        .build_int_truncate(
                                            int_val,
                                            self.context.i32_type(),
                                            "ret_cast",
                                        )
                                        .map_err(|_| {
                                            CodegenError::new(
                                                "failed to truncate main return value",
                                            )
                                        })?
                                        .into()
                                } else {
                                    val
                                }
                            } else {
                                val
                            };
                            self.builder
                                .build_return(Some(&ret_val))
                                .map_err(|_| CodegenError::new("failed to emit return value"))?;
                        }
                    }
                    None => {
                        if is_main {
                            let zero = self.context.i32_type().const_int(0, false);
                            self.builder.build_return(Some(&zero)).map_err(|_| {
                                CodegenError::new("failed to emit implicit main return")
                            })?;
                        } else {
                            self.builder
                                .build_return(None)
                                .map_err(|_| CodegenError::new("failed to emit empty return"))?;
                        }
                    }
                }
                CODEGEN_PHASE_TIMING_TOTALS
                    .body_stmt_return_ns
                    .fetch_add(elapsed_nanos_u64(stmt_started_at), Ordering::Relaxed);
            }

            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.compile_if(condition, then_block, else_block.as_ref())?;
            }

            Stmt::While { condition, body } => {
                self.compile_while(condition, body)?;
            }

            Stmt::For {
                var,
                var_type,
                iterable,
                body,
            } => {
                self.compile_for(var, var_type.as_ref(), iterable, body)?;
            }

            Stmt::Break => {
                if let Some(loop_ctx) = self.loop_stack.last() {
                    self.builder
                        .build_unconditional_branch(loop_ctx.after_block)
                        .map_err(|_| CodegenError::new("failed to emit break branch"))?;
                }
            }

            Stmt::Continue => {
                if let Some(loop_ctx) = self.loop_stack.last() {
                    self.builder
                        .build_unconditional_branch(loop_ctx.loop_block)
                        .map_err(|_| CodegenError::new("failed to emit continue branch"))?;
                }
            }

            Stmt::Match { expr, arms } => {
                self.compile_match_stmt(expr, arms)?;
            }
        }

        Ok(())
    }

    pub fn compile_if(
        &mut self,
        cond: &Spanned<Expr>,
        then_block: &Block,
        else_block: Option<&Block>,
    ) -> Result<()> {
        let cond_val = self.compile_condition_expr(&cond.node)?;
        let func = self
            .current_function
            .ok_or_else(|| CodegenError::new("if statement used outside function"))?;

        let then_bb = self.context.append_basic_block(func, "then");
        let else_bb = self.context.append_basic_block(func, "else");
        let merge_bb = self.context.append_basic_block(func, "merge");

        self.builder
            .build_conditional_branch(cond_val, then_bb, else_bb)
            .map_err(|_| CodegenError::new("failed to branch for if statement"))?;

        // Then
        self.builder.position_at_end(then_bb);
        self.with_variable_scope(|this| {
            for stmt in then_block {
                this.compile_stmt(&stmt.node)?;
            }
            Ok(())
        })?;
        if self.needs_terminator() {
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|_| CodegenError::new("failed to branch from if-then to merge"))?;
        }

        // Else
        self.builder.position_at_end(else_bb);
        if let Some(else_stmts) = else_block {
            self.with_variable_scope(|this| {
                for stmt in else_stmts {
                    this.compile_stmt(&stmt.node)?;
                }
                Ok(())
            })?;
        }
        if self.needs_terminator() {
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|_| CodegenError::new("failed to branch from if-else to merge"))?;
        }

        self.builder.position_at_end(merge_bb);
        Ok(())
    }

    pub fn compile_while(&mut self, cond: &Spanned<Expr>, body: &Block) -> Result<()> {
        let func = self
            .current_function
            .ok_or_else(|| CodegenError::new("while loop used outside function"))?;

        let counted_push_loop_info = self.analyze_counted_push_only_while_loop(&cond.node, body);
        self.reserve_capacity_for_push_only_while_loop(&cond.node, body)?;

        // LOOP ROTATION OPTIMIZATION:
        // Instead of: while (cond) { body }
        // We generate: if (cond) { do { body } while (cond) }
        // This eliminates one branch per iteration!

        let entry_bb = self.context.append_basic_block(func, "while.entry");
        let body_bb = self.context.append_basic_block(func, "while.body");
        let cond_bb = self.context.append_basic_block(func, "while.cond");
        let after_bb = self.context.append_basic_block(func, "while.after");

        // First, check condition (entry test)
        self.builder
            .build_unconditional_branch(entry_bb)
            .map_err(|_| CodegenError::new("failed to enter while loop"))?;
        self.builder.position_at_end(entry_bb);
        let entry_cond = self.compile_condition_expr(&cond.node)?;
        self.builder
            .build_conditional_branch(entry_cond, body_bb, after_bb)
            .map_err(|_| CodegenError::new("failed to branch on while entry condition"))?;

        // Body (executed at least once if we get here)
        self.builder.position_at_end(body_bb);
        self.loop_stack.push(LoopContext {
            loop_block: cond_bb,
            after_block: after_bb,
        });
        self.with_condition_non_negative_facts(&cond.node, |this| {
            this.with_variable_scope(|this| {
                for stmt in body {
                    this.compile_stmt(&stmt.node)?;
                }
                Ok(())
            })
        })?;
        self.loop_stack.pop();
        if self.needs_terminator() {
            self.builder
                .build_unconditional_branch(cond_bb)
                .map_err(|_| CodegenError::new("failed to continue while loop"))?;
        }

        // Loop condition check at end (loop rotation)
        self.builder.position_at_end(cond_bb);
        let loop_cond = self.compile_condition_expr(&cond.node)?;

        self.builder
            .build_conditional_branch(loop_cond, body_bb, after_bb)
            .map_err(|_| CodegenError::new("failed to branch on while loop condition"))?;

        self.builder.position_at_end(after_bb);
        if let Some((counter_name, exact_bound, pushed_lists, list_element_upper_bounds)) =
            counted_push_loop_info
        {
            self.exact_integer_locals.insert(counter_name, exact_bound);
            for list_name in pushed_lists {
                self.exact_list_lengths.insert(list_name, exact_bound);
            }
            for (list_name, upper_bound) in list_element_upper_bounds {
                self.list_element_upper_bounds
                    .insert(list_name, upper_bound);
            }
        }
        Ok(())
    }

    fn reserve_capacity_for_push_only_while_loop(
        &mut self,
        condition: &Expr,
        body: &Block,
    ) -> Result<()> {
        let Some(bound_expr) = Self::simple_while_loop_upper_bound_expr(condition) else {
            return Ok(());
        };
        if !matches!(self.infer_builtin_argument_type(bound_expr), Type::Integer) {
            return Ok(());
        }
        if !matches!(
            bound_expr,
            Expr::Ident(_) | Expr::Literal(Literal::Integer(_))
        ) {
            return Ok(());
        }
        if !self.expr_is_provably_non_negative(bound_expr) {
            return Ok(());
        }

        let pushed_lists = Self::collect_direct_local_list_push_targets(body);
        if pushed_lists.is_empty() {
            return Ok(());
        }
        let exact_capacity = self.exact_integer_value(bound_expr);

        let requested_capacity = self
            .compile_expr_with_expected_type(bound_expr, &Type::Integer)?
            .into_int_value();
        for list_name in pushed_lists {
            let Some(variable) = self.variables.get(&list_name).cloned() else {
                continue;
            };
            if !matches!(self.deref_codegen_type(&variable.ty), Type::List(_)) {
                continue;
            }
            self.ensure_list_capacity_ptr(variable.ptr, &variable.ty, requested_capacity)?;
            if let Some(exact_capacity) = exact_capacity {
                self.exact_list_capacities
                    .insert(list_name.clone(), exact_capacity);
            } else {
                self.exact_list_capacities.remove(&list_name);
            }
        }
        Ok(())
    }

    fn simple_while_loop_upper_bound_expr(condition: &Expr) -> Option<&Expr> {
        match condition {
            Expr::Binary {
                op: BinOp::Lt,
                left,
                right,
            } if matches!(left.node, Expr::Ident(_)) => Some(&right.node),
            _ => None,
        }
    }

    fn simple_while_loop_counter_name(condition: &Expr) -> Option<&str> {
        match condition {
            Expr::Binary {
                op: BinOp::Lt,
                left,
                ..
            } => match &left.node {
                Expr::Ident(name) => Some(name.as_str()),
                _ => None,
            },
            _ => None,
        }
    }

    fn collect_direct_local_list_push_targets(body: &Block) -> HashSet<String> {
        let mut pushed_lists = HashSet::new();
        for stmt in body {
            let Stmt::Expr(expr) = &stmt.node else {
                continue;
            };
            let Expr::Call { callee, .. } = &expr.node else {
                continue;
            };
            let Expr::Field { object, field } = &callee.node else {
                continue;
            };
            if field != "push" {
                continue;
            }
            let Expr::Ident(name) = &object.node else {
                continue;
            };
            pushed_lists.insert(name.clone());
        }
        pushed_lists
    }

    fn analyze_counted_push_only_while_loop(
        &self,
        condition: &Expr,
        body: &Block,
    ) -> Option<CountedPushLoopInfo> {
        let counter_name = Self::simple_while_loop_counter_name(condition)?.to_string();
        let bound_expr = Self::simple_while_loop_upper_bound_expr(condition)?;
        let exact_bound = self.exact_integer_value(bound_expr)?;
        if exact_bound < 0 {
            return None;
        }
        if self.exact_integer_locals.get(&counter_name).copied()? != 0 {
            return None;
        }

        let mut pushed_lists = HashSet::new();
        let mut list_element_upper_bounds = HashMap::new();
        let mut saw_counter_increment = false;
        for stmt in body {
            match &stmt.node {
                Stmt::Expr(expr) => {
                    let Expr::Call { callee, args, .. } = &expr.node else {
                        return None;
                    };
                    let Expr::Field { object, field } = &callee.node else {
                        return None;
                    };
                    if field != "push" {
                        return None;
                    }
                    let Expr::Ident(name) = &object.node else {
                        return None;
                    };
                    if self.exact_list_lengths.get(name).copied()? != 0 {
                        return None;
                    }
                    let push_value = args.first()?;
                    if self.expr_is_provably_non_negative(&push_value.node) {
                        let upper_bound = self.expr_upper_bound_exclusive(&push_value.node)?;
                        list_element_upper_bounds.insert(name.clone(), upper_bound);
                    }
                    pushed_lists.insert(name.clone());
                }
                Stmt::Assign { target, value } => {
                    let Expr::Ident(target_name) = &target.node else {
                        return None;
                    };
                    if target_name != &counter_name {
                        return None;
                    }
                    let Expr::Binary {
                        op: BinOp::Add,
                        left,
                        right,
                    } = &value.node
                    else {
                        return None;
                    };
                    if !matches!(&left.node, Expr::Ident(name) if name == &counter_name)
                        || !matches!(
                            TypeChecker::eval_numeric_const_expr(&right.node),
                            Some(NumericConst::Integer(1))
                        )
                    {
                        return None;
                    }
                    saw_counter_increment = true;
                }
                _ => return None,
            }
        }
        (!pushed_lists.is_empty() && saw_counter_increment).then_some((
            counter_name,
            exact_bound,
            pushed_lists,
            list_element_upper_bounds,
        ))
    }

    fn adapt_for_loop_binding_value(
        &self,
        value: BasicValueEnum<'ctx>,
        source_ty: &Type,
        target_ty: &Type,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>> {
        if source_ty == target_ty {
            return Ok(value);
        }

        match (source_ty, target_ty) {
            (Type::Integer, Type::Float) if value.is_int_value() => {
                return Ok(self
                    .builder
                    .build_signed_int_to_float(
                        value.into_int_value(),
                        self.context.f64_type(),
                        name,
                    )
                    .map_err(|_| CodegenError::new("failed to cast for-loop binding to float"))?
                    .into())
            }
            _ => {}
        }

        if self.is_supported_function_adapter_assignment(target_ty, source_ty) {
            return Ok(value);
        }

        Err(CodegenError::new(format!(
            "unsupported for-loop binding conversion: {:?} -> {:?}",
            source_ty, target_ty
        )))
    }

    fn expr_creates_empty_list(&self, expr: &Expr, ty: &Type) -> bool {
        matches!(self.deref_codegen_type(ty), Type::List(_))
            && matches!(expr, Expr::Construct { args, .. } if args.is_empty())
    }

    fn encode_enum_payload(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<IntValue<'ctx>> {
        let i64_type = self.context.i64_type();
        let normalized_ty = self.normalize_codegen_type(ty);
        let encoded = match &normalized_ty {
            Type::Integer => match value {
                BasicValueEnum::IntValue(v) => v,
                _ => {
                    return Err(CodegenError::new(format!(
                        "enum payload type mismatch: expected Integer-compatible value for {:?}",
                        normalized_ty
                    )));
                }
            },
            Type::Boolean => self
                .builder
                .build_int_z_extend(
                    match value {
                        BasicValueEnum::IntValue(v) => v,
                        _ => {
                            return Err(CodegenError::new(
                                "enum payload type mismatch: expected Boolean-compatible value",
                            ));
                        }
                    },
                    i64_type,
                    "bool_to_i64",
                )
                .map_err(|_| CodegenError::new("failed to encode boolean enum payload"))?,
            Type::Char => self
                .builder
                .build_int_z_extend(
                    match value {
                        BasicValueEnum::IntValue(v) => v,
                        _ => {
                            return Err(CodegenError::new(
                                "enum payload type mismatch: expected Char-compatible value",
                            ));
                        }
                    },
                    i64_type,
                    "char_to_i64",
                )
                .map_err(|_| CodegenError::new("failed to encode char enum payload"))?,
            Type::Float => self
                .builder
                .build_bit_cast(
                    match value {
                        BasicValueEnum::FloatValue(v) => v,
                        _ => {
                            return Err(CodegenError::new(
                                "enum payload type mismatch: expected Float-compatible value",
                            ));
                        }
                    },
                    i64_type,
                    "float_bits",
                )
                .map_err(|_| CodegenError::new("failed to encode float enum payload"))?
                .into_int_value(),
            Type::String | Type::Named(_) | Type::Ref(_) | Type::MutRef(_) | Type::Ptr(_) => self
                .builder
                .build_ptr_to_int(
                    match value {
                        BasicValueEnum::PointerValue(v) => v,
                        _ => {
                            return Err(CodegenError::new(format!(
                                "enum payload type mismatch: expected pointer-compatible value for {:?}",
                                normalized_ty
                            )));
                        }
                    },
                    i64_type,
                    "ptr_to_i64",
                )
                .map_err(|_| CodegenError::new("failed to encode pointer enum payload"))?,
            Type::Generic(name, _)
                if self
                    .canonical_codegen_type_name(name)
                    .is_some_and(|canonical| self.classes.contains_key(&canonical)) =>
            {
                self.builder
                    .build_ptr_to_int(
                        match value {
                            BasicValueEnum::PointerValue(v) => v,
                            _ => {
                                return Err(CodegenError::new(
                                    "enum payload type mismatch: expected pointer-compatible generic payload",
                                ));
                            }
                        },
                        i64_type,
                        "ptr_to_i64",
                    )
                    .map_err(|_| CodegenError::new("failed to encode generic enum payload"))?
            }
            _ => {
                return Err(CodegenError::new(
                    "Unsupported enum payload type for codegen",
                ));
            }
        };
        Ok(encoded)
    }

    pub(crate) fn decode_enum_payload(
        &self,
        raw: IntValue<'ctx>,
        ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let i64_type = self.context.i64_type();
        let normalized_ty = self.normalize_codegen_type(ty);
        let decoded = match &normalized_ty {
            Type::Integer => raw.into(),
            Type::Boolean => self
                .builder
                .build_int_truncate(raw, self.context.bool_type(), "i64_to_bool")
                .map_err(|_| CodegenError::new("failed to decode boolean enum payload"))?
                .into(),
            Type::Char => self
                .builder
                .build_int_truncate(raw, self.context.i32_type(), "i64_to_char")
                .map_err(|_| CodegenError::new("failed to decode char enum payload"))?
                .into(),
            Type::Float => self
                .builder
                .build_bit_cast(raw, self.context.f64_type(), "bits_to_float")
                .map_err(|_| CodegenError::new("failed to decode float enum payload"))?,
            Type::String | Type::Named(_) | Type::Ref(_) | Type::MutRef(_) | Type::Ptr(_) => self
                .builder
                .build_int_to_ptr(
                    raw,
                    self.context.ptr_type(AddressSpace::default()),
                    "i64_to_ptr",
                )
                .map_err(|_| CodegenError::new("failed to decode pointer enum payload"))?
                .into(),
            Type::Generic(name, _)
                if self
                    .canonical_codegen_type_name(name)
                    .is_some_and(|canonical| self.classes.contains_key(&canonical)) =>
            {
                self.builder
                    .build_int_to_ptr(
                        raw,
                        self.context.ptr_type(AddressSpace::default()),
                        "i64_to_ptr",
                    )
                    .map_err(|_| CodegenError::new("failed to decode generic enum payload"))?
                    .into()
            }
            _ => {
                return Err(CodegenError::new(
                    "Unsupported enum payload type for codegen",
                ));
            }
        };
        let _ = i64_type; // keep layout assumptions explicit
        Ok(decoded)
    }

    fn build_enum_value(
        &mut self,
        enum_name: &str,
        variant_info: &EnumVariantInfo,
        values: &[BasicValueEnum<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let enum_info = self
            .enums
            .get(enum_name)
            .ok_or_else(|| CodegenError::new(format!("Unknown enum '{}'", enum_name)))?;
        let mut value = enum_info.struct_type.const_zero();

        value = self
            .builder
            .build_insert_value(
                value,
                self.context
                    .i8_type()
                    .const_int(variant_info.tag as u64, false),
                0,
                "enum_tag",
            )
            .map_err(|_| CodegenError::new("failed to insert enum tag"))?
            .into_struct_value();

        for (i, field_ty) in variant_info.fields.iter().enumerate() {
            let encoded = self.encode_enum_payload(values[i], field_ty)?;
            value = self
                .builder
                .build_insert_value(value, encoded, (i + 1) as u32, "enum_payload")
                .map_err(|_| CodegenError::new("failed to insert enum payload"))?
                .into_struct_value();
        }

        Ok(value.into())
    }

    // === Expressions ===

    fn match_compound_assign_target<'a>(
        target: &'a Expr,
        value: &'a Expr,
    ) -> Option<(BinOp, &'a Spanned<Expr>)> {
        let Expr::Binary { op, left, right } = value else {
            return None;
        };
        matches!(
            op,
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
        )
        .then_some(())
        .filter(|_| Self::exprs_structurally_equal(target, &left.node))
        .map(|_| (*op, right.as_ref()))
    }

    fn exprs_structurally_equal(left: &Expr, right: &Expr) -> bool {
        match (left, right) {
            (Expr::Ident(a), Expr::Ident(b)) => a == b,
            (Expr::This, Expr::This) => true,
            (Expr::Literal(a), Expr::Literal(b)) => Self::literals_structurally_equal(a, b),
            (
                Expr::Field {
                    object: ao,
                    field: af,
                },
                Expr::Field {
                    object: bo,
                    field: bf,
                },
            ) => af == bf && Self::exprs_structurally_equal(&ao.node, &bo.node),
            (
                Expr::Index {
                    object: ao,
                    index: ai,
                },
                Expr::Index {
                    object: bo,
                    index: bi,
                },
            ) => {
                Self::exprs_structurally_equal(&ao.node, &bo.node)
                    && Self::exprs_structurally_equal(&ai.node, &bi.node)
            }
            (
                Expr::Call {
                    callee: ac,
                    args: aa,
                    type_args: at,
                },
                Expr::Call {
                    callee: bc,
                    args: ba,
                    type_args: bt,
                },
            ) => {
                at == bt
                    && aa.len() == ba.len()
                    && Self::exprs_structurally_equal(&ac.node, &bc.node)
                    && aa
                        .iter()
                        .zip(ba.iter())
                        .all(|(a, b)| Self::exprs_structurally_equal(&a.node, &b.node))
            }
            (Expr::Deref(a), Expr::Deref(b))
            | (Expr::Borrow(a), Expr::Borrow(b))
            | (Expr::MutBorrow(a), Expr::MutBorrow(b)) => {
                Self::exprs_structurally_equal(&a.node, &b.node)
            }
            _ => false,
        }
    }

    fn literals_structurally_equal(left: &Literal, right: &Literal) -> bool {
        match (left, right) {
            (Literal::Integer(a), Literal::Integer(b)) => a == b,
            (Literal::Float(a), Literal::Float(b)) => a.to_bits() == b.to_bits(),
            (Literal::Boolean(a), Literal::Boolean(b)) => a == b,
            (Literal::String(a), Literal::String(b)) => a == b,
            (Literal::Char(a), Literal::Char(b)) => a == b,
            (Literal::None, Literal::None) => true,
            _ => false,
        }
    }

    fn infer_async_block_return_type(
        &self,
        body: &[Spanned<Stmt>],
        expected_inner_return_type: Option<&Type>,
    ) -> Type {
        self.infer_block_tail_type_with_expected(body, &[], expected_inner_return_type)
            .unwrap_or(Type::None)
    }

    pub(crate) fn compile_expr_with_expected_type(
        &mut self,
        expr: &Expr,
        expected_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        match expected_ty {
            Type::Ref(_) => {
                let actual_ty = self.infer_builtin_argument_type(expr);
                if !matches!(actual_ty, Type::Ref(_) | Type::MutRef(_)) {
                    return self.compile_borrow(expr);
                }
            }
            Type::MutRef(_) => {
                let actual_ty = self.infer_builtin_argument_type(expr);
                if !matches!(actual_ty, Type::MutRef(_)) {
                    return self.compile_mut_borrow(expr);
                }
            }
            _ => {}
        }
        if let Some(actual_ty) = self.explicit_constructor_expr_type(expr) {
            self.reject_builtin_constructor_specialization_mismatch(expected_ty, &actual_ty)?;
        }
        if let Expr::GenericFunctionValue { callee, type_args } = expr {
            if let Some(closure) = self
                .compile_class_constructor_function_value_with_expected_type(
                    &callee.node,
                    Some(type_args),
                    expected_ty,
                )?
            {
                return Ok(closure);
            }
        } else if let Some(closure) = self
            .compile_class_constructor_function_value_with_expected_type(expr, None, expected_ty)?
        {
            return Ok(closure);
        }
        if let Some(closure) =
            self.compile_enum_variant_function_value_with_expected_type(expr, expected_ty)?
        {
            return Ok(closure);
        }
        if let Expr::Call {
            callee,
            args,
            type_args,
        } = expr
        {
            if let Expr::Field { object, field } = &callee.node {
                if let Expr::Ident(type_name) = &object.node {
                    if let Some(canonical_builtin) =
                        builtin_exact_import_alias_canonical(&format!("{}.{}", type_name, field))
                    {
                        match (canonical_builtin, expected_ty) {
                            ("Option__some", Type::Option(inner_ty)) => {
                                if !type_args.is_empty() {
                                    return Err(CodegenError::new(
                                        "Option static methods do not accept explicit type arguments",
                                    ));
                                }
                                if args.len() != 1 {
                                    return Err(CodegenError::new(
                                        "Option.some() requires exactly 1 argument",
                                    ));
                                }
                                let value = self.compile_expr_for_concrete_class_payload(
                                    &args[0].node,
                                    inner_ty,
                                )?;
                                return self.create_option_some_typed(value, inner_ty);
                            }
                            ("Option__none", Type::Option(inner_ty)) => {
                                if !type_args.is_empty() {
                                    return Err(CodegenError::new(
                                        "Option static methods do not accept explicit type arguments",
                                    ));
                                }
                                if !args.is_empty() {
                                    return Err(CodegenError::new(format!(
                                        "Option.none() expects 0 argument(s), got {}",
                                        args.len()
                                    )));
                                }
                                return self.create_option_none_typed(inner_ty);
                            }
                            ("Result__ok", Type::Result(ok_ty, err_ty)) => {
                                if !type_args.is_empty() {
                                    return Err(CodegenError::new(
                                        "Result static methods do not accept explicit type arguments",
                                    ));
                                }
                                if args.len() != 1 {
                                    return Err(CodegenError::new(
                                        "Result.ok() requires exactly 1 argument",
                                    ));
                                }
                                let value = self.compile_expr_for_concrete_class_payload(
                                    &args[0].node,
                                    ok_ty,
                                )?;
                                return self.create_result_ok_typed(value, ok_ty, err_ty);
                            }
                            ("Result__error", Type::Result(ok_ty, err_ty)) => {
                                if !type_args.is_empty() {
                                    return Err(CodegenError::new(
                                        "Result static methods do not accept explicit type arguments",
                                    ));
                                }
                                if args.len() != 1 {
                                    return Err(CodegenError::new(
                                        "Result.error() requires exactly 1 argument",
                                    ));
                                }
                                let value = self.compile_expr_for_concrete_class_payload(
                                    &args[0].node,
                                    err_ty,
                                )?;
                                return self.create_result_error_typed(value, ok_ty, err_ty);
                            }
                            _ => {}
                        }
                    }
                    match (type_name.as_str(), field.as_str(), expected_ty) {
                        ("Option", "some", Type::Option(inner_ty)) => {
                            if !type_args.is_empty() {
                                return Err(CodegenError::new(
                                    "Option static methods do not accept explicit type arguments",
                                ));
                            }
                            if args.len() != 1 {
                                return Err(CodegenError::new(
                                    "Option.some() requires exactly 1 argument",
                                ));
                            }
                            let value = self
                                .compile_expr_for_concrete_class_payload(&args[0].node, inner_ty)?;
                            return self.create_option_some_typed(value, inner_ty);
                        }
                        ("Option", "none", Type::Option(inner_ty)) => {
                            if !type_args.is_empty() {
                                return Err(CodegenError::new(
                                    "Option static methods do not accept explicit type arguments",
                                ));
                            }
                            if !args.is_empty() {
                                return Err(CodegenError::new(format!(
                                    "Option.none() expects 0 argument(s), got {}",
                                    args.len()
                                )));
                            }
                            return self.create_option_none_typed(inner_ty);
                        }
                        ("Result", "ok", Type::Result(ok_ty, err_ty)) => {
                            if !type_args.is_empty() {
                                return Err(CodegenError::new(
                                    "Result static methods do not accept explicit type arguments",
                                ));
                            }
                            if args.len() != 1 {
                                return Err(CodegenError::new(
                                    "Result.ok() requires exactly 1 argument",
                                ));
                            }
                            let value =
                                self.compile_expr_for_concrete_class_payload(&args[0].node, ok_ty)?;
                            return self.create_result_ok_typed(value, ok_ty, err_ty);
                        }
                        ("Result", "error", Type::Result(ok_ty, err_ty)) => {
                            if !type_args.is_empty() {
                                return Err(CodegenError::new(
                                    "Result static methods do not accept explicit type arguments",
                                ));
                            }
                            if args.len() != 1 {
                                return Err(CodegenError::new(
                                    "Result.error() requires exactly 1 argument",
                                ));
                            }
                            let value = self
                                .compile_expr_for_concrete_class_payload(&args[0].node, err_ty)?;
                            return self.create_result_error_typed(value, ok_ty, err_ty);
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Expr::Match {
            expr: match_expr,
            arms,
        } = expr
        {
            return self.compile_match_expr(&match_expr.node, arms, Some(expected_ty));
        }
        if let Expr::If {
            condition,
            then_branch,
            else_branch,
        } = expr
        {
            return self.compile_if_expr(
                &condition.node,
                then_branch,
                else_branch.as_ref(),
                Some(expected_ty),
            );
        }
        if let Expr::Block(body) = expr {
            return self.with_variable_scope(|this| {
                let mut result = this.context.i8_type().const_int(0, false).into();
                for (idx, stmt) in body.iter().enumerate() {
                    if let Stmt::Expr(inner_expr) = &stmt.node {
                        result = if idx + 1 == body.len() {
                            this.compile_expr_with_expected_type(&inner_expr.node, expected_ty)?
                        } else {
                            this.compile_expr(&inner_expr.node)?
                        };
                    } else {
                        this.compile_stmt(&stmt.node)?;
                    }
                }
                Ok(result)
            });
        }
        if let Expr::Lambda { params, body } = expr {
            if matches!(expected_ty, Type::Function(_, _)) {
                return self.compile_lambda(params, body, Some(expected_ty));
            }
        }
        if !matches!(expected_ty, Type::Function(_, _)) {
            if let Some(name) = self.resolve_contextual_function_value_name(expr) {
                if let Some(value) =
                    self.compile_builtin_zero_arg_value_with_expected_type(&name, expected_ty)?
                {
                    return Ok(value);
                }
            }
        }
        if matches!(expected_ty, Type::Function(_, _)) {
            if let Some(name) = self.resolve_contextual_function_value_name(expr) {
                if let Some(actual_ty) = self.functions.get(&name).map(|(_, ty)| ty.clone()) {
                    if let Type::Function(_, _) = actual_ty {
                        if let Some(adapted) = self
                            .compile_named_function_value_with_expected_type(&name, expected_ty)?
                        {
                            return Ok(adapted);
                        }
                        return Err(Self::function_value_signature_mismatch_error(
                            &actual_ty,
                            expected_ty,
                        ));
                    }
                }
                if let Some(adapted) =
                    self.compile_builtin_function_value_with_expected_type(&name, expected_ty)?
                {
                    return Ok(adapted);
                }
            }
        }
        if let Expr::AsyncBlock(body) = expr {
            if let Type::Task(inner) = expected_ty {
                return self.compile_async_block(body, Some(inner));
            }
        }

        if matches!(expected_ty, Type::Function(_, _)) {
            let actual_ty = match expr {
                Expr::Field { object, field } => self
                    .infer_bound_field_function_type(&object.node, field)
                    .unwrap_or_else(|| self.infer_expr_type(expr, &[])),
                _ => self.infer_expr_type(expr, &[]),
            };
            if matches!(actual_ty, Type::Function(_, _)) {
                let value = self.compile_expr(expr)?;
                if let Some(adapted) = self.compile_function_value_adapter_from_closure(
                    value,
                    &actual_ty,
                    expected_ty,
                )? {
                    return Ok(adapted);
                }
                if &actual_ty != expected_ty {
                    return Err(Self::function_value_signature_mismatch_error(
                        &actual_ty,
                        expected_ty,
                    ));
                }
                return Ok(value);
            }
        }

        let value = self.compile_expr(expr)?;
        if matches!(expected_ty, Type::Float) && value.is_int_value() {
            return Ok(self
                .builder
                .build_signed_int_to_float(value.into_int_value(), self.context.f64_type(), "ef")
                .map_err(|_| CodegenError::new("failed to cast expression to expected float"))?
                .into());
        }

        Ok(value)
    }

    pub(crate) fn compile_expr_for_concrete_class_payload(
        &mut self,
        expr: &Expr,
        expected_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let value = self.compile_expr_with_expected_type(expr, expected_ty)?;
        let actual_ty = self.infer_expr_type(expr, &[]);
        self.reject_incompatible_expected_type_value(expected_ty, &actual_ty, value)?;
        Ok(value)
    }

    fn resolve_enum_variant_function_value(
        &self,
        expr: &Expr,
    ) -> Option<(String, EnumVariantInfo)> {
        if let Expr::Ident(name) = expr {
            if let Some((enum_name, variant_name)) = name.rsplit_once("__") {
                if let Some(enum_info) = self.enums.get(enum_name) {
                    if let Some(variant_info) = enum_info.variants.get(variant_name) {
                        return Some((enum_name.to_string(), variant_info.clone()));
                    }
                }
            }
            let (enum_name, variant_name) = self.resolve_import_alias_variant(name)?;
            let enum_info = self.enums.get(&enum_name)?;
            let variant_info = enum_info.variants.get(&variant_name)?.clone();
            return Some((enum_name, variant_info));
        }

        let Expr::Field { object, field } = expr else {
            return None;
        };

        if let Some(path_parts) = flatten_field_chain(expr) {
            if path_parts.len() >= 2 {
                let owner_source = path_parts[..path_parts.len() - 1].join(".");
                if let Some(resolved_owner) =
                    self.resolve_alias_qualified_codegen_type_name(&owner_source)
                {
                    if let Some(enum_info) = self.enums.get(&resolved_owner) {
                        let variant_name = path_parts.last()?;
                        if let Some(variant_info) = enum_info.variants.get(variant_name) {
                            return Some((resolved_owner, variant_info.clone()));
                        }
                    }
                }
            }
        }

        let Expr::Ident(owner_name) = &object.node else {
            return None;
        };
        let resolved_owner = self
            .resolve_alias_qualified_codegen_type_name(owner_name)
            .unwrap_or_else(|| self.resolve_module_alias(owner_name));
        let enum_info = self.enums.get(&resolved_owner)?;
        let variant_info = enum_info.variants.get(field)?.clone();
        Some((resolved_owner, variant_info))
    }

    fn compile_enum_variant_function_value_with_expected_type(
        &mut self,
        expr: &Expr,
        expected_ty: &Type,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let Type::Function(_, _) = expected_ty else {
            return Ok(None);
        };
        let Some((enum_name, variant_info)) = self.resolve_enum_variant_function_value(expr) else {
            return Ok(None);
        };
        let actual_ty = Type::Function(
            variant_info.fields.clone(),
            Box::new(Type::Named(enum_name.clone())),
        );
        let Type::Function(param_types, ret_type) = &actual_ty else {
            return Ok(None);
        };

        let wrapper_name = format!(
            "__enum_variant_fn_value_{}_{}_{}",
            enum_name,
            variant_info.tag,
            Self::type_specialization_suffix(&actual_ty)
        );
        let wrapper_fn = if let Some(existing) = self.module.get_function(&wrapper_name) {
            existing
        } else {
            let ptr_type = self.context.ptr_type(AddressSpace::default());
            let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![ptr_type.into()];
            for param_ty in param_types {
                llvm_params.push(self.llvm_type(param_ty).into());
            }
            let wrapper_fn_type = match self.llvm_type(ret_type) {
                BasicTypeEnum::IntType(i) => i.fn_type(&llvm_params, false),
                BasicTypeEnum::FloatType(f) => f.fn_type(&llvm_params, false),
                BasicTypeEnum::PointerType(p) => p.fn_type(&llvm_params, false),
                BasicTypeEnum::StructType(s) => s.fn_type(&llvm_params, false),
                _ => self.context.i8_type().fn_type(&llvm_params, false),
            };
            let wrapper_fn = self
                .module
                .add_function(&wrapper_name, wrapper_fn_type, None);

            let saved_function = self.current_function;
            let saved_return_type = self.current_return_type.clone();
            let saved_insert_block = self.builder.get_insert_block();

            self.current_function = Some(wrapper_fn);
            self.current_return_type = Some(actual_ty.clone());

            let entry = self.context.append_basic_block(wrapper_fn, "entry");
            self.builder.position_at_end(entry);

            let mut values = Vec::with_capacity(variant_info.fields.len());
            for index in 0..variant_info.fields.len() {
                let llvm_param = wrapper_fn
                    .get_nth_param((index + 1) as u32)
                    .ok_or_else(|| CodegenError::new("missing enum variant wrapper parameter"))?;
                values.push(llvm_param);
            }
            let value = self.build_enum_value(&enum_name, &variant_info, &values)?;
            self.builder.build_return(Some(&value)).map_err(|e| {
                CodegenError::new(format!("enum variant wrapper return failed: {}", e))
            })?;

            self.current_function = saved_function;
            self.current_return_type = saved_return_type;
            if let Some(block) = saved_insert_block {
                self.builder.position_at_end(block);
            }

            wrapper_fn
        };

        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let closure_ty = self.llvm_type(&actual_ty).into_struct_type();
        let mut closure = closure_ty.get_undef();
        closure = self
            .builder
            .build_insert_value(
                closure,
                wrapper_fn.as_global_value().as_pointer_value(),
                0,
                "enum_variant_fn",
            )
            .map_err(|_| CodegenError::new("failed to build enum variant closure function"))?
            .into_struct_value();
        closure = self
            .builder
            .build_insert_value(closure, ptr_type.const_null(), 1, "enum_variant_env")
            .map_err(|_| CodegenError::new("failed to build enum variant closure environment"))?
            .into_struct_value();

        if &actual_ty == expected_ty {
            return Ok(Some(closure.into()));
        }

        if let Some(adapted) = self.compile_function_value_adapter_from_closure(
            closure.into(),
            &actual_ty,
            expected_ty,
        )? {
            return Ok(Some(adapted));
        }

        Err(CodegenError::new(format!(
            "Type mismatch: expected {}, got {}",
            Self::format_diagnostic_type(expected_ty),
            Self::format_diagnostic_type(&actual_ty)
        )))
    }

    fn compile_expr_for_llvm_param(
        &mut self,
        expr: &Expr,
        expected_param_ty: BasicMetadataTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let value = self.compile_expr(expr)?;
        Ok(match expected_param_ty {
            BasicMetadataTypeEnum::FloatType(_) if value.is_int_value() => self
                .builder
                .build_signed_int_to_float(value.into_int_value(), self.context.f64_type(), "apf")
                .map_err(|_| CodegenError::new("failed to cast call argument to float"))?
                .into(),
            _ => value,
        })
    }

    fn compile_named_function_value_with_expected_type(
        &mut self,
        name: &str,
        expected_ty: &Type,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let Some((func, actual_ty)) = self.functions.get(name).cloned() else {
            return Ok(None);
        };
        if self.extern_functions.contains(name) {
            return Err(CodegenError::new(format!(
                "extern function '{}' cannot be used as a first-class value yet",
                name
            )));
        }
        let (expected_params, expected_ret) = match expected_ty {
            Type::Function(params, ret) => (params, ret),
            _ => return Ok(None),
        };
        let (actual_params, actual_ret) = match actual_ty {
            Type::Function(params, ret) => (params, ret),
            _ => return Ok(None),
        };
        if actual_params.len() != expected_params.len() {
            return Ok(None);
        }
        let params_need_adapter = expected_params
            .iter()
            .zip(actual_params.iter())
            .any(|(expected, actual)| expected != actual);
        let return_needs_adapter = actual_ret.as_ref() != expected_ret.as_ref();
        if !params_need_adapter && !return_needs_adapter {
            let struct_ty = self.llvm_type(expected_ty).into_struct_type();
            let mut closure = struct_ty.get_undef();
            let fn_ptr = func.as_global_value().as_pointer_value();
            let null_env = self.context.ptr_type(AddressSpace::default()).const_null();

            closure = self
                .builder
                .build_insert_value(closure, fn_ptr, 0, "fn")
                .map_err(|_| CodegenError::new("failed to build function closure pointer"))?
                .into_struct_value();
            closure = self
                .builder
                .build_insert_value(closure, null_env, 1, "env")
                .map_err(|_| CodegenError::new("failed to build function closure environment"))?
                .into_struct_value();
            return Ok(Some(closure.into()));
        }
        if expected_params
            .iter()
            .zip(actual_params.iter())
            .any(|(expected, actual)| !self.is_supported_function_adapter_param(expected, actual))
        {
            return Ok(None);
        }
        if !self.is_supported_function_adapter_return(actual_ret.as_ref(), expected_ret.as_ref()) {
            return Ok(None);
        }

        let adapter_name = format!("__fn_adapter_{}", self.lambda_counter);
        self.lambda_counter += 1;
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let expected_ret_llvm = self.llvm_type(expected_ret);
        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![ptr_type.into()];
        for param_ty in expected_params {
            llvm_params.push(self.llvm_type(param_ty).into());
        }
        let adapter_fn_type = match expected_ret_llvm {
            BasicTypeEnum::IntType(i) => i.fn_type(&llvm_params, false),
            BasicTypeEnum::FloatType(f) => f.fn_type(&llvm_params, false),
            BasicTypeEnum::PointerType(p) => p.fn_type(&llvm_params, false),
            BasicTypeEnum::StructType(s) => s.fn_type(&llvm_params, false),
            _ => self.context.i8_type().fn_type(&llvm_params, false),
        };
        let adapter_fn = self
            .module
            .add_function(&adapter_name, adapter_fn_type, None);

        let saved_function = self.current_function;
        let saved_return_type = self.current_return_type.clone();
        let saved_insert_block = self.builder.get_insert_block();

        self.current_function = Some(adapter_fn);
        self.current_return_type = Some(expected_ty.clone());
        let entry = self.context.append_basic_block(adapter_fn, "entry");
        self.builder.position_at_end(entry);

        let mut call_args: Vec<BasicMetadataValueEnum> = vec![ptr_type.const_null().into()];
        for (index, (expected_param_ty, actual_param_ty)) in
            expected_params.iter().zip(actual_params.iter()).enumerate()
        {
            let param = adapter_fn
                .get_nth_param((index + 1) as u32)
                .ok_or_else(|| CodegenError::new("adapter parameter missing"))?;
            let adapted_param =
                self.adapt_function_adapter_param(param, expected_param_ty, actual_param_ty)?;
            call_args.push(adapted_param.into());
        }
        let call = self
            .builder
            .build_call(func, &call_args, "fn_adapter_call")
            .map_err(|_| CodegenError::new("failed to emit named function adapter call"))?;
        let result = self.extract_call_value(call);
        let adapted = self.adapt_function_adapter_return(
            result?,
            actual_ret.as_ref(),
            expected_ret.as_ref(),
            "fn_adapter_return",
        )?;
        self.builder
            .build_return(Some(&adapted))
            .map_err(|_| CodegenError::new("failed to emit named function adapter return"))?;

        self.current_function = saved_function;
        self.current_return_type = saved_return_type;
        if let Some(block) = saved_insert_block {
            self.builder.position_at_end(block);
        }

        let closure_ty = self
            .context
            .struct_type(&[ptr_type.into(), ptr_type.into()], false);
        let mut closure = closure_ty.get_undef();
        closure = self
            .builder
            .build_insert_value(
                closure,
                adapter_fn.as_global_value().as_pointer_value(),
                0,
                "fn",
            )
            .map_err(|_| CodegenError::new("failed to build adapter closure pointer"))?
            .into_struct_value();
        closure = self
            .builder
            .build_insert_value(closure, ptr_type.const_null(), 1, "env")
            .map_err(|_| CodegenError::new("failed to build adapter closure environment"))?
            .into_struct_value();
        Ok(Some(closure.into()))
    }

    fn compile_builtin_function_value_with_expected_type(
        &mut self,
        name: &str,
        expected_ty: &Type,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let actual_ty = if Self::builtin_matches_expected_function_type(name, expected_ty) {
            expected_ty.clone()
        } else if let Some(actual_ty) =
            Self::builtin_function_value_concrete_type_for_expected(name, expected_ty)
        {
            actual_ty
        } else {
            return Err(CodegenError::new(format!(
                "Type mismatch: expected {}, got {}",
                Self::format_diagnostic_type(expected_ty),
                Self::builtin_function_value_diagnostic_signature(name)
            )));
        };

        let actual_closure = self.compile_builtin_function_value_with_type(name, &actual_ty)?;
        if &actual_ty == expected_ty {
            return Ok(Some(actual_closure));
        }

        self.compile_function_value_adapter_from_closure(actual_closure, &actual_ty, expected_ty)
    }

    fn compile_builtin_zero_arg_value_with_expected_type(
        &mut self,
        name: &str,
        expected_ty: &Type,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let synthetic_function_type = Type::Function(vec![], Box::new(expected_ty.clone()));
        if !Self::builtin_matches_expected_function_type(name, &synthetic_function_type) {
            return Ok(None);
        }
        match (name, expected_ty) {
            ("Option__none", Type::Option(inner_ty)) => {
                Ok(Some(self.create_option_none_typed(inner_ty.as_ref())?))
            }
            _ => self.compile_stdlib_function(name, &[]),
        }
    }

    fn builtin_function_value_diagnostic_signature(name: &str) -> &'static str {
        match name {
            "Option__some" => "(unknown) -> Option<unknown>",
            "Option__none" => "() -> Option<unknown>",
            "Result__ok" => "(unknown) -> Result<unknown, unknown>",
            "Result__error" => "(unknown) -> Result<unknown, unknown>",
            "read_line" | "System__cwd" | "System__os" => "() -> String",
            "File__read" | "System__getenv" | "System__exec" | "Time__now" => "(String) -> String",
            "System__shell" => "(String) -> Integer",
            "File__write" => "(String, String) -> Boolean",
            "File__exists" | "File__delete" => "(String) -> Boolean",
            "System__exit" | "exit" | "Time__sleep" => "(Integer) -> None",
            "Time__unix" | "Args__count" => "() -> Integer",
            "Args__get" => "(Integer) -> String",
            "Math__abs" => "(unknown) -> unknown",
            "Math__min" | "Math__max" => "(unknown, unknown) -> unknown",
            "Math__pow" => "(unknown, unknown) -> Float",
            "Math__sqrt" | "Math__sin" | "Math__cos" | "Math__tan" | "Math__floor"
            | "Math__ceil" | "Math__round" | "Math__log" | "Math__log10" | "Math__exp" => {
                "(unknown) -> Float"
            }
            "Math__pi" | "Math__e" | "Math__random" => "() -> Float",
            "Str__len" => "(String) -> Integer",
            "Str__compare" => "(String, String) -> Integer",
            "Str__concat" => "(String, String) -> String",
            "Str__upper" | "Str__lower" | "Str__trim" => "(String) -> String",
            "Str__contains" | "Str__startsWith" | "Str__endsWith" => "(String, String) -> Boolean",
            "to_float" => "(unknown) -> Float",
            "to_int" => "(unknown) -> Integer",
            "to_string" => "(unknown) -> String",
            "assert" | "assert_true" | "assert_false" | "fail" => "(unknown) -> None",
            "assert_eq" | "assert_ne" => "(unknown, unknown) -> None",
            "range" => "(unknown, unknown) -> unknown",
            _ => "builtin function",
        }
    }

    fn builtin_function_value_concrete_type_for_expected(
        name: &str,
        expected: &Type,
    ) -> Option<Type> {
        match (name, expected) {
            ("Option__some", Type::Function(params, ret))
                if params.len() == 1
                    && matches!(ret.as_ref(), Type::Option(inner) if params[0] == inner.as_ref().clone()) =>
            {
                Some(expected.clone())
            }
            ("Option__none", Type::Function(params, ret))
                if params.is_empty() && matches!(ret.as_ref(), Type::Option(_)) =>
            {
                Some(expected.clone())
            }
            ("Result__ok", Type::Function(params, ret))
                if params.len() == 1
                    && matches!(ret.as_ref(), Type::Result(ok, _) if params[0] == ok.as_ref().clone()) =>
            {
                Some(expected.clone())
            }
            ("Result__error", Type::Function(params, ret))
                if params.len() == 1
                    && matches!(ret.as_ref(), Type::Result(_, err) if params[0] == err.as_ref().clone()) =>
            {
                Some(expected.clone())
            }
            ("Math__abs", Type::Function(params, ret))
                if params.len() == 1
                    && matches!(params[0], Type::Integer)
                    && matches!(ret.as_ref(), Type::Float) =>
            {
                Some(Type::Function(vec![Type::Integer], Box::new(Type::Integer)))
            }
            ("Math__min" | "Math__max", Type::Function(params, ret))
                if params.len() == 2
                    && matches!(params[0], Type::Integer)
                    && matches!(params[1], Type::Integer)
                    && matches!(ret.as_ref(), Type::Float) =>
            {
                Some(Type::Function(
                    vec![Type::Integer, Type::Integer],
                    Box::new(Type::Integer),
                ))
            }
            ("Math__pow", Type::Function(params, ret))
                if params.len() == 2
                    && params
                        .iter()
                        .all(|param| matches!(param, Type::Integer | Type::Float))
                    && matches!(ret.as_ref(), Type::Float) =>
            {
                Some(Type::Function(
                    vec![Type::Float, Type::Float],
                    Box::new(Type::Float),
                ))
            }
            _ => None,
        }
    }

    fn compile_builtin_function_value_with_type(
        &mut self,
        name: &str,
        function_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let Type::Function(param_types, ret_type) = function_ty else {
            return Err(CodegenError::new(
                "builtin function value requires function type",
            ));
        };

        let wrapper_name = format!(
            "__builtin_fn_value_{}_{}",
            name.replace("__", "_"),
            Self::type_specialization_suffix(function_ty)
        );
        let wrapper_fn = if let Some(existing) = self.module.get_function(&wrapper_name) {
            existing
        } else {
            let ptr_type = self.context.ptr_type(AddressSpace::default());
            let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![ptr_type.into()];
            for param_ty in param_types {
                llvm_params.push(self.llvm_type(param_ty).into());
            }
            let wrapper_fn_type = match self.llvm_type(ret_type) {
                BasicTypeEnum::IntType(i) => i.fn_type(&llvm_params, false),
                BasicTypeEnum::FloatType(f) => f.fn_type(&llvm_params, false),
                BasicTypeEnum::PointerType(p) => p.fn_type(&llvm_params, false),
                BasicTypeEnum::StructType(s) => s.fn_type(&llvm_params, false),
                _ => self.context.i8_type().fn_type(&llvm_params, false),
            };
            let wrapper_fn = self
                .module
                .add_function(&wrapper_name, wrapper_fn_type, None);

            let saved_function = self.current_function;
            let saved_return_type = self.current_return_type.clone();
            let saved_insert_block = self.builder.get_insert_block();
            let saved_variables = self.variables.clone();
            let saved_non_negative_locals = self.non_negative_locals.clone();
            let saved_non_zero_locals = self.non_zero_locals.clone();
            let saved_exact_integer_locals = self.exact_integer_locals.clone();
            let saved_upper_bound_locals = self.upper_bound_locals.clone();
            let saved_exact_list_lengths = self.exact_list_lengths.clone();
            let saved_exact_list_capacities = self.exact_list_capacities.clone();
            let saved_list_element_upper_bounds = self.list_element_upper_bounds.clone();
            let saved_distinct_list_alloc_ids = self.distinct_list_alloc_ids.clone();

            self.current_function = Some(wrapper_fn);
            self.current_return_type = Some(function_ty.clone());
            self.variables.clear();
            self.non_negative_locals.clear();
            self.non_zero_locals.clear();
            self.exact_integer_locals.clear();
            self.upper_bound_locals.clear();
            self.exact_list_lengths.clear();
            self.exact_list_capacities.clear();
            self.list_element_upper_bounds.clear();
            self.distinct_list_alloc_ids.clear();

            let entry = self.context.append_basic_block(wrapper_fn, "entry");
            self.builder.position_at_end(entry);

            let mut wrapper_args = Vec::with_capacity(param_types.len());
            let mut synthetic_args = Vec::with_capacity(param_types.len());
            for (index, param_ty) in param_types.iter().enumerate() {
                let param_name = format!("__builtin_arg_{index}");
                let llvm_param = wrapper_fn
                    .get_nth_param((index + 1) as u32)
                    .ok_or_else(|| CodegenError::new("missing builtin wrapper parameter"))?;
                wrapper_args.push(llvm_param);
                let alloca = self
                    .builder
                    .build_alloca(self.llvm_type(param_ty), &param_name)
                    .map_err(|e| {
                        CodegenError::new(format!("builtin wrapper alloca failed: {}", e))
                    })?;
                self.builder.build_store(alloca, llvm_param).map_err(|e| {
                    CodegenError::new(format!("builtin wrapper store failed: {}", e))
                })?;
                self.variables.insert(
                    param_name.clone(),
                    Variable {
                        ptr: alloca,
                        ty: param_ty.clone(),
                        mutable: false,
                    },
                );
                synthetic_args.push(Spanned::new(Expr::Ident(param_name), Span::default()));
            }

            let result = match name {
                "Option__some" => {
                    let inner_ty = match ret_type.as_ref() {
                        Type::Option(inner_ty) => inner_ty.as_ref(),
                        _ => {
                            return Err(CodegenError::new(
                                "Option.some function value requires Option return type",
                            ))
                        }
                    };
                    let value = wrapper_args.first().copied().ok_or_else(|| {
                        CodegenError::new("Option.some function value missing argument")
                    })?;
                    Some(self.create_option_some_typed(value, inner_ty)?)
                }
                "Option__none" => {
                    let inner_ty = match ret_type.as_ref() {
                        Type::Option(inner_ty) => inner_ty.as_ref(),
                        _ => {
                            return Err(CodegenError::new(
                                "Option.none function value requires Option return type",
                            ))
                        }
                    };
                    Some(self.create_option_none_typed(inner_ty)?)
                }
                "Result__ok" => {
                    let (ok_ty, err_ty) = match ret_type.as_ref() {
                        Type::Result(ok_ty, err_ty) => (ok_ty.as_ref(), err_ty.as_ref()),
                        _ => {
                            return Err(CodegenError::new(
                                "Result.ok function value requires Result return type",
                            ))
                        }
                    };
                    let value = wrapper_args.first().copied().ok_or_else(|| {
                        CodegenError::new("Result.ok function value missing argument")
                    })?;
                    Some(self.create_result_ok_typed(value, ok_ty, err_ty)?)
                }
                "Result__error" => {
                    let (ok_ty, err_ty) = match ret_type.as_ref() {
                        Type::Result(ok_ty, err_ty) => (ok_ty.as_ref(), err_ty.as_ref()),
                        _ => {
                            return Err(CodegenError::new(
                                "Result.error function value requires Result return type",
                            ))
                        }
                    };
                    let value = wrapper_args.first().copied().ok_or_else(|| {
                        CodegenError::new("Result.error function value missing argument")
                    })?;
                    Some(self.create_result_error_typed(value, ok_ty, err_ty)?)
                }
                _ => self.compile_stdlib_function(name, &synthetic_args)?,
            };
            if matches!(ret_type.as_ref(), Type::None) {
                self.builder
                    .build_return(Some(&self.context.i8_type().const_int(0, false)))
                    .map_err(|e| {
                        CodegenError::new(format!("builtin wrapper return failed: {}", e))
                    })?;
            } else {
                let value = result.ok_or_else(|| {
                    CodegenError::new(format!(
                        "builtin wrapper '{}' produced no value for non-void function",
                        name
                    ))
                })?;
                self.builder.build_return(Some(&value)).map_err(|e| {
                    CodegenError::new(format!("builtin wrapper return failed: {}", e))
                })?;
            }

            self.current_function = saved_function;
            self.current_return_type = saved_return_type;
            self.variables = saved_variables;
            self.non_negative_locals = saved_non_negative_locals;
            self.non_zero_locals = saved_non_zero_locals;
            self.exact_integer_locals = saved_exact_integer_locals;
            self.upper_bound_locals = saved_upper_bound_locals;
            self.exact_list_lengths = saved_exact_list_lengths;
            self.exact_list_capacities = saved_exact_list_capacities;
            self.list_element_upper_bounds = saved_list_element_upper_bounds;
            self.distinct_list_alloc_ids = saved_distinct_list_alloc_ids;
            if let Some(block) = saved_insert_block {
                self.builder.position_at_end(block);
            }

            wrapper_fn
        };

        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let closure_ty = self
            .context
            .struct_type(&[ptr_type.into(), ptr_type.into()], false);
        let mut closure = closure_ty.get_undef();
        closure = self
            .builder
            .build_insert_value(
                closure,
                wrapper_fn.as_global_value().as_pointer_value(),
                0,
                "builtin_fn",
            )
            .map_err(|_| CodegenError::new("failed to build builtin closure pointer"))?
            .into_struct_value();
        closure = self
            .builder
            .build_insert_value(closure, ptr_type.const_null(), 1, "builtin_env")
            .map_err(|_| CodegenError::new("failed to build builtin closure environment"))?
            .into_struct_value();
        Ok(closure.into())
    }

    fn compile_function_value_adapter_from_closure(
        &mut self,
        closure_value: BasicValueEnum<'ctx>,
        actual_ty: &Type,
        expected_ty: &Type,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let (expected_params, expected_ret) = match expected_ty {
            Type::Function(params, ret) => (params, ret),
            _ => return Ok(None),
        };
        let (actual_params, actual_ret) = match actual_ty {
            Type::Function(params, ret) => (params, ret),
            _ => return Ok(None),
        };
        if actual_params.len() != expected_params.len() {
            return Ok(None);
        }
        let params_need_adapter = expected_params
            .iter()
            .zip(actual_params.iter())
            .any(|(expected, actual)| expected != actual);
        let return_needs_adapter = actual_ret.as_ref() != expected_ret.as_ref();
        if !params_need_adapter && !return_needs_adapter {
            return Ok(None);
        }
        if expected_params
            .iter()
            .zip(actual_params.iter())
            .any(|(expected, actual)| !self.is_supported_function_adapter_param(expected, actual))
        {
            return Ok(None);
        }
        if !self.is_supported_function_adapter_return(actual_ret.as_ref(), expected_ret.as_ref()) {
            return Ok(None);
        }

        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let closure_struct = closure_value.into_struct_value();
        let closure_ty = closure_struct.get_type();
        let env_struct_ty = self.context.struct_type(&[closure_ty.into()], false);
        let env_size = env_struct_ty
            .size_of()
            .ok_or_else(|| CodegenError::new("failed to size function adapter env"))?;
        let env_alloc = self.build_malloc_call(
            env_size,
            "fn_adapter_env_alloc",
            "failed to call malloc for function adapter env",
        )?;
        let env_ptr =
            self.extract_call_pointer_value(env_alloc, "malloc failed for function adapter env")?;
        let stored_closure_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(
                    env_struct_ty,
                    env_ptr,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context.i32_type().const_int(0, false),
                    ],
                    "fn_adapter_closure_ptr",
                )
                .map_err(|e| {
                    CodegenError::new(format!(
                        "failed to get function adapter closure storage: {e}"
                    ))
                })?
        };
        self.builder
            .build_store(stored_closure_ptr, closure_struct)
            .map_err(|e| {
                CodegenError::new(format!("failed to store function adapter closure: {e}"))
            })?;

        let adapter_name = format!("__fn_closure_adapter_{}", self.lambda_counter);
        self.lambda_counter += 1;
        let expected_ret_llvm = self.llvm_type(expected_ret);
        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![ptr_type.into()];
        for param_ty in expected_params {
            llvm_params.push(self.llvm_type(param_ty).into());
        }
        let adapter_fn_type = match expected_ret_llvm {
            BasicTypeEnum::IntType(i) => i.fn_type(&llvm_params, false),
            BasicTypeEnum::FloatType(f) => f.fn_type(&llvm_params, false),
            BasicTypeEnum::PointerType(p) => p.fn_type(&llvm_params, false),
            BasicTypeEnum::StructType(s) => s.fn_type(&llvm_params, false),
            _ => self.context.i8_type().fn_type(&llvm_params, false),
        };
        let adapter_fn = self
            .module
            .add_function(&adapter_name, adapter_fn_type, None);

        let saved_function = self.current_function;
        let saved_return_type = self.current_return_type.clone();
        let saved_insert_block = self.builder.get_insert_block();

        self.current_function = Some(adapter_fn);
        self.current_return_type = Some(expected_ty.clone());
        let entry = self.context.append_basic_block(adapter_fn, "entry");
        self.builder.position_at_end(entry);

        let adapter_env = adapter_fn
            .get_nth_param(0)
            .ok_or_else(|| CodegenError::new("function adapter env param missing"))?
            .into_pointer_value();
        let closure_field_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(
                    env_struct_ty,
                    adapter_env,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context.i32_type().const_int(0, false),
                    ],
                    "fn_adapter_env_closure_ptr",
                )
                .map_err(|e| {
                    CodegenError::new(format!(
                        "failed to get function adapter env closure pointer: {e}"
                    ))
                })?
        };
        let loaded_closure = self
            .builder
            .build_load(closure_ty, closure_field_ptr, "fn_adapter_closure")
            .map_err(|e| {
                CodegenError::new(format!("failed to load function adapter closure: {e}"))
            })?
            .into_struct_value();
        let loaded_fn_ptr = self
            .builder
            .build_extract_value(loaded_closure, 0, "fn_adapter_fn_ptr")
            .map_err(|e| {
                CodegenError::new(format!(
                    "failed to extract function adapter fn pointer: {e}"
                ))
            })?
            .into_pointer_value();
        let loaded_env_ptr = self
            .builder
            .build_extract_value(loaded_closure, 1, "fn_adapter_env_ptr")
            .map_err(|e| {
                CodegenError::new(format!(
                    "failed to extract function adapter env pointer: {e}"
                ))
            })?
            .into_pointer_value();

        let mut actual_llvm_params: Vec<BasicMetadataTypeEnum> = vec![ptr_type.into()];
        for param_ty in actual_params {
            actual_llvm_params.push(self.llvm_type(param_ty).into());
        }
        let actual_ret_llvm = self.llvm_type(actual_ret);
        let actual_fn_type = match actual_ret_llvm {
            BasicTypeEnum::IntType(i) => i.fn_type(&actual_llvm_params, false),
            BasicTypeEnum::FloatType(f) => f.fn_type(&actual_llvm_params, false),
            BasicTypeEnum::PointerType(p) => p.fn_type(&actual_llvm_params, false),
            BasicTypeEnum::StructType(s) => s.fn_type(&actual_llvm_params, false),
            _ => self.context.i8_type().fn_type(&actual_llvm_params, false),
        };
        let typed_fn_ptr = self
            .builder
            .build_pointer_cast(
                loaded_fn_ptr,
                self.context.ptr_type(AddressSpace::default()),
                "fn_adapter_typed_fn_ptr",
            )
            .map_err(|e| {
                CodegenError::new(format!("failed to cast function adapter fn pointer: {e}"))
            })?;
        let mut call_args: Vec<BasicMetadataValueEnum> = vec![loaded_env_ptr.into()];
        for (index, (expected_param_ty, actual_param_ty)) in
            expected_params.iter().zip(actual_params.iter()).enumerate()
        {
            let param = adapter_fn
                .get_nth_param((index + 1) as u32)
                .ok_or_else(|| CodegenError::new("function adapter parameter missing"))?;
            let adapted_param =
                self.adapt_function_adapter_param(param, expected_param_ty, actual_param_ty)?;
            call_args.push(adapted_param.into());
        }
        let call = self
            .builder
            .build_indirect_call(actual_fn_type, typed_fn_ptr, &call_args, "fn_adapter_call")
            .map_err(|e| {
                CodegenError::new(format!("failed to build function adapter call: {e}"))
            })?;
        let result = self.extract_call_value(call);
        let adapted = self.adapt_function_adapter_return(
            result?,
            actual_ret.as_ref(),
            expected_ret.as_ref(),
            "fn_adapter_return",
        )?;
        self.builder.build_return(Some(&adapted)).map_err(|e| {
            CodegenError::new(format!("failed to return function adapter result: {e}"))
        })?;

        self.current_function = saved_function;
        self.current_return_type = saved_return_type;
        if let Some(block) = saved_insert_block {
            self.builder.position_at_end(block);
        }

        let wrapper_closure_ty = self
            .context
            .struct_type(&[ptr_type.into(), ptr_type.into()], false);
        let mut wrapper_closure = wrapper_closure_ty.get_undef();
        wrapper_closure = self
            .builder
            .build_insert_value(
                wrapper_closure,
                adapter_fn.as_global_value().as_pointer_value(),
                0,
                "fn",
            )
            .map_err(|e| CodegenError::new(format!("failed to build adapter closure fn: {e}")))?
            .into_struct_value();
        wrapper_closure = self
            .builder
            .build_insert_value(wrapper_closure, env_ptr, 1, "env")
            .map_err(|e| CodegenError::new(format!("failed to build adapter closure env: {e}")))?
            .into_struct_value();
        Ok(Some(wrapper_closure.into()))
    }

    fn is_supported_function_adapter_param(&self, expected: &Type, actual: &Type) -> bool {
        self.is_supported_function_adapter_assignment(actual, expected)
    }

    fn is_supported_function_adapter_return(&self, actual: &Type, expected: &Type) -> bool {
        self.is_supported_function_adapter_assignment(expected, actual)
    }

    fn is_supported_function_adapter_assignment(&self, expected: &Type, actual: &Type) -> bool {
        let expected = self.normalize_codegen_type(expected);
        let actual = self.normalize_codegen_type(actual);
        if expected == actual {
            return true;
        }
        if matches!((&expected, &actual), (Type::Float, Type::Integer)) {
            return true;
        }

        match (&expected, &actual) {
            (Type::Named(_), Type::Named(_))
            | (Type::Named(_), Type::Generic(_, _))
            | (Type::Generic(_, _), Type::Named(_))
            | (Type::Generic(_, _), Type::Generic(_, _)) => {
                self.is_supported_function_adapter_nominal_assignment(&expected, &actual)
            }
            (Type::Ref(expected), Type::Ref(actual))
            | (Type::MutRef(expected), Type::MutRef(actual))
            | (Type::Ptr(expected), Type::Ptr(actual))
            | (Type::Box(expected), Type::Box(actual))
            | (Type::Rc(expected), Type::Rc(actual))
            | (Type::Arc(expected), Type::Arc(actual))
            | (Type::List(expected), Type::List(actual))
            | (Type::Set(expected), Type::Set(actual))
            | (Type::Option(expected), Type::Option(actual))
            | (Type::Task(expected), Type::Task(actual))
            | (Type::Range(expected), Type::Range(actual)) => {
                self.is_supported_function_adapter_invariant(expected, actual)
            }
            (Type::Ref(expected), Type::MutRef(actual)) => {
                self.is_supported_function_adapter_invariant(expected, actual)
            }
            (Type::Result(expected_ok, expected_err), Type::Result(actual_ok, actual_err)) => {
                self.is_supported_function_adapter_invariant(expected_ok, actual_ok)
                    && self.is_supported_function_adapter_invariant(expected_err, actual_err)
            }
            (Type::Map(expected_key, expected_value), Type::Map(actual_key, actual_value)) => {
                self.is_supported_function_adapter_invariant(expected_key, actual_key)
                    && self.is_supported_function_adapter_invariant(expected_value, actual_value)
            }
            (
                Type::Function(expected_params, expected_ret),
                Type::Function(actual_params, actual_ret),
            ) => {
                expected_params.len() == actual_params.len()
                    && expected_params.iter().zip(actual_params.iter()).all(
                        |(expected_param, actual_param)| {
                            self.is_supported_function_adapter_assignment(
                                actual_param,
                                expected_param,
                            )
                        },
                    )
                    && self.is_supported_function_adapter_assignment(expected_ret, actual_ret)
            }
            _ => false,
        }
    }

    fn is_supported_function_adapter_invariant(&self, expected: &Type, actual: &Type) -> bool {
        let expected = self.normalize_codegen_type(expected);
        let actual = self.normalize_codegen_type(actual);
        if expected == actual {
            return true;
        }

        match (&expected, &actual) {
            (Type::Named(_), Type::Named(_))
            | (Type::Named(_), Type::Generic(_, _))
            | (Type::Generic(_, _), Type::Named(_))
            | (Type::Generic(_, _), Type::Generic(_, _)) => {
                self.function_adapter_same_nominal_arguments(&expected, &actual)
            }
            (Type::Ref(expected), Type::Ref(actual))
            | (Type::MutRef(expected), Type::MutRef(actual))
            | (Type::Ptr(expected), Type::Ptr(actual))
            | (Type::Box(expected), Type::Box(actual))
            | (Type::Rc(expected), Type::Rc(actual))
            | (Type::Arc(expected), Type::Arc(actual))
            | (Type::List(expected), Type::List(actual))
            | (Type::Set(expected), Type::Set(actual))
            | (Type::Option(expected), Type::Option(actual))
            | (Type::Task(expected), Type::Task(actual))
            | (Type::Range(expected), Type::Range(actual)) => {
                self.is_supported_function_adapter_invariant(expected, actual)
            }
            (Type::Ref(expected), Type::MutRef(actual)) => {
                self.is_supported_function_adapter_invariant(expected, actual)
            }
            (Type::Result(expected_ok, expected_err), Type::Result(actual_ok, actual_err)) => {
                self.is_supported_function_adapter_invariant(expected_ok, actual_ok)
                    && self.is_supported_function_adapter_invariant(expected_err, actual_err)
            }
            (Type::Map(expected_key, expected_value), Type::Map(actual_key, actual_value)) => {
                self.is_supported_function_adapter_invariant(expected_key, actual_key)
                    && self.is_supported_function_adapter_invariant(expected_value, actual_value)
            }
            (
                Type::Function(expected_params, expected_ret),
                Type::Function(actual_params, actual_ret),
            ) => {
                expected_params.len() == actual_params.len()
                    && expected_params.iter().zip(actual_params.iter()).all(
                        |(expected_param, actual_param)| {
                            self.is_supported_function_adapter_invariant(
                                expected_param,
                                actual_param,
                            )
                        },
                    )
                    && self.is_supported_function_adapter_invariant(expected_ret, actual_ret)
            }
            _ => false,
        }
    }

    fn is_supported_function_adapter_nominal_assignment(
        &self,
        expected: &Type,
        actual: &Type,
    ) -> bool {
        let Some(expected_name) = Self::function_adapter_nominal_base_name(expected) else {
            return false;
        };
        let Some(actual_name) = Self::function_adapter_nominal_base_name(actual) else {
            return false;
        };

        if expected_name == actual_name {
            return self.function_adapter_same_nominal_arguments(expected, actual);
        }

        if self.interfaces.contains_key(expected_name) {
            return self
                .interface_implementors
                .get(expected_name)
                .is_some_and(|implementors| implementors.contains(actual_name));
        }

        if self.classes.contains_key(expected_name) && self.classes.contains_key(actual_name) {
            return self.is_same_or_subclass_for_function_adapter(actual_name, expected_name);
        }

        false
    }

    fn function_adapter_same_nominal_arguments(&self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            (Type::Named(_), Type::Named(_)) => true,
            (
                Type::Generic(expected_name, expected_args),
                Type::Generic(actual_name, actual_args),
            ) if expected_name == actual_name && expected_args.len() == actual_args.len() => {
                expected_args
                    .iter()
                    .zip(actual_args.iter())
                    .all(|(expected_arg, actual_arg)| {
                        self.is_supported_function_adapter_invariant(expected_arg, actual_arg)
                    })
            }
            (Type::Named(expected_name), Type::Generic(actual_name, actual_args))
                if expected_name == actual_name && actual_args.is_empty() =>
            {
                true
            }
            (Type::Generic(expected_name, expected_args), Type::Named(actual_name))
                if expected_name == actual_name && expected_args.is_empty() =>
            {
                true
            }
            _ => false,
        }
    }

    fn function_adapter_nominal_base_name(ty: &Type) -> Option<&str> {
        match ty {
            Type::Named(name) | Type::Generic(name, _) => {
                Some(name.split('<').next().unwrap_or(name))
            }
            _ => None,
        }
    }

    fn is_same_or_subclass_for_function_adapter(&self, class_name: &str, ancestor: &str) -> bool {
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
            let parent_base = parent.split('<').next().unwrap_or(parent);
            if parent_base == ancestor {
                return true;
            }
            current = parent_base;
            depth += 1;
        }
        false
    }

    pub(crate) fn reject_unrelated_concrete_class_assignment(
        &self,
        expected: &Type,
        actual: &Type,
    ) -> Result<()> {
        let expected = self.normalize_codegen_type(expected);
        let actual = self.normalize_codegen_type(actual);
        let Some(expected_name) = Self::function_adapter_nominal_base_name(&expected) else {
            return Ok(());
        };
        let Some(actual_name) = Self::function_adapter_nominal_base_name(&actual) else {
            return Ok(());
        };

        if !self.classes.contains_key(expected_name) || !self.classes.contains_key(actual_name) {
            return Ok(());
        }

        if expected_name == actual_name {
            if self.function_adapter_same_nominal_arguments(&expected, &actual) {
                return Ok(());
            }
        } else if self.is_same_or_subclass_for_function_adapter(actual_name, expected_name) {
            return Ok(());
        }

        Err(CodegenError::new(format!(
            "Type mismatch: expected {}, got {}",
            Self::format_diagnostic_type(&expected),
            Self::format_diagnostic_type(&actual)
        )))
    }

    pub(crate) fn reject_incompatible_expected_type_value(
        &self,
        expected_ty: &Type,
        actual_ty: &Type,
        value: BasicValueEnum<'ctx>,
    ) -> Result<()> {
        self.reject_unrelated_concrete_class_assignment(expected_ty, actual_ty)?;
        self.reject_builtin_invariant_specialization_mismatch(expected_ty, actual_ty)?;

        if !self.type_contains_active_generic_placeholder(expected_ty)
            && !self.type_contains_active_generic_placeholder(actual_ty)
            && value.get_type() != self.llvm_type(expected_ty)
        {
            return Err(Self::type_mismatch_error(expected_ty, actual_ty));
        }

        Ok(())
    }

    fn reject_builtin_constructor_specialization_mismatch(
        &self,
        expected: &Type,
        actual: &Type,
    ) -> Result<()> {
        let expected = self.normalize_codegen_type(expected);
        let actual = self.normalize_codegen_type(actual);

        if self.type_contains_active_generic_placeholder(&expected)
            || self.type_contains_active_generic_placeholder(&actual)
        {
            return Ok(());
        }

        let expected_key = Self::builtin_constructor_specialization_key(&expected);
        let actual_key = Self::builtin_constructor_specialization_key(&actual);
        if let (Some(expected_key), Some(actual_key)) = (expected_key, actual_key) {
            if expected_key != actual_key {
                return Err(Self::type_mismatch_error(&expected, &actual));
            }
        }

        Ok(())
    }

    fn explicit_constructor_expr_type(&self, expr: &Expr) -> Option<Type> {
        let Expr::Construct { ty, .. } = expr else {
            return None;
        };
        let (base_name, explicit_type_args) = Self::parse_construct_nominal_type_source(ty)?;

        if let Some(resolved_name) = self.resolve_alias_qualified_codegen_type_name(&base_name) {
            if explicit_type_args.is_empty() {
                return Some(Type::Named(resolved_name));
            }
            if resolved_name.contains("__spec__") {
                return Some(Type::Named(resolved_name));
            }
            if let Some(normalized) =
                self.normalize_user_defined_generic_type(&resolved_name, &explicit_type_args)
            {
                return Some(normalized);
            }
        }

        Some(self.normalize_codegen_type(&Type::Generic(base_name, explicit_type_args)))
    }

    fn builtin_constructor_specialization_key(ty: &Type) -> Option<String> {
        match ty {
            Type::Named(name) if name.contains("__spec__") => {
                let (base_name, _) = name.split_once("__spec__")?;
                matches!(
                    base_name,
                    "Option" | "Result" | "List" | "Map" | "Set" | "Box" | "Rc" | "Arc"
                )
                .then(|| name.clone())
            }
            Type::Generic(name, args)
                if matches!(
                    name.as_str(),
                    "Option" | "Result" | "List" | "Map" | "Set" | "Box" | "Rc" | "Arc"
                ) =>
            {
                Some(Self::generic_class_spec_name(name, args))
            }
            Type::Option(inner) => Some(Self::generic_class_spec_name(
                "Option",
                std::slice::from_ref(inner.as_ref()),
            )),
            Type::Result(ok, err) => Some(Self::generic_class_spec_name(
                "Result",
                &[ok.as_ref().clone(), err.as_ref().clone()],
            )),
            Type::List(inner) => Some(Self::generic_class_spec_name(
                "List",
                std::slice::from_ref(inner.as_ref()),
            )),
            Type::Map(key, value) => Some(Self::generic_class_spec_name(
                "Map",
                &[key.as_ref().clone(), value.as_ref().clone()],
            )),
            Type::Set(inner) => Some(Self::generic_class_spec_name(
                "Set",
                std::slice::from_ref(inner.as_ref()),
            )),
            Type::Box(inner) => Some(Self::generic_class_spec_name(
                "Box",
                std::slice::from_ref(inner.as_ref()),
            )),
            Type::Rc(inner) => Some(Self::generic_class_spec_name(
                "Rc",
                std::slice::from_ref(inner.as_ref()),
            )),
            Type::Arc(inner) => Some(Self::generic_class_spec_name(
                "Arc",
                std::slice::from_ref(inner.as_ref()),
            )),
            _ => None,
        }
    }

    fn reject_builtin_invariant_specialization_mismatch(
        &self,
        expected: &Type,
        actual: &Type,
    ) -> Result<()> {
        let expected = self.normalize_codegen_type(expected);
        let actual = self.normalize_codegen_type(actual);

        if self.type_contains_active_generic_placeholder(&expected)
            || self.type_contains_active_generic_placeholder(&actual)
        {
            return Ok(());
        }

        let expected_key = Self::builtin_invariant_specialization_key(&expected);
        let actual_key = Self::builtin_invariant_specialization_key(&actual);
        if (expected_key.is_some() || actual_key.is_some()) && expected_key != actual_key {
            return Err(Self::type_mismatch_error(&expected, &actual));
        }

        Ok(())
    }

    fn builtin_invariant_specialization_key(ty: &Type) -> Option<String> {
        match ty {
            Type::Named(name) if name.contains("__spec__") => {
                let (base_name, _) = name.split_once("__spec__")?;
                matches!(base_name, "List" | "Map" | "Set" | "Box" | "Rc" | "Arc")
                    .then(|| name.clone())
            }
            Type::Generic(name, args)
                if matches!(name.as_str(), "List" | "Map" | "Set" | "Box" | "Rc" | "Arc") =>
            {
                Some(Self::generic_class_spec_name(name, args))
            }
            Type::List(inner) => Some(Self::generic_class_spec_name(
                "List",
                std::slice::from_ref(inner.as_ref()),
            )),
            Type::Map(key, value) => Some(Self::generic_class_spec_name(
                "Map",
                &[key.as_ref().clone(), value.as_ref().clone()],
            )),
            Type::Set(inner) => Some(Self::generic_class_spec_name(
                "Set",
                std::slice::from_ref(inner.as_ref()),
            )),
            Type::Box(inner) => Some(Self::generic_class_spec_name(
                "Box",
                std::slice::from_ref(inner.as_ref()),
            )),
            Type::Rc(inner) => Some(Self::generic_class_spec_name(
                "Rc",
                std::slice::from_ref(inner.as_ref()),
            )),
            Type::Arc(inner) => Some(Self::generic_class_spec_name(
                "Arc",
                std::slice::from_ref(inner.as_ref()),
            )),
            _ => None,
        }
    }

    pub(crate) fn with_variable_scope<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let saved_variables = self.variables.clone();
        let saved_non_negative_locals = self.non_negative_locals.clone();
        let saved_non_zero_locals = self.non_zero_locals.clone();
        let saved_exact_integer_locals = self.exact_integer_locals.clone();
        let saved_upper_bound_locals = self.upper_bound_locals.clone();
        let saved_exact_list_lengths = self.exact_list_lengths.clone();
        let saved_exact_list_capacities = self.exact_list_capacities.clone();
        let saved_list_element_upper_bounds = self.list_element_upper_bounds.clone();
        let saved_distinct_list_alloc_ids = self.distinct_list_alloc_ids.clone();
        let result = f(self);
        self.variables = saved_variables;
        self.non_negative_locals = saved_non_negative_locals;
        self.non_zero_locals = saved_non_zero_locals;
        self.exact_integer_locals = saved_exact_integer_locals;
        self.upper_bound_locals = saved_upper_bound_locals;
        self.exact_list_lengths = saved_exact_list_lengths;
        self.exact_list_capacities = saved_exact_list_capacities;
        self.list_element_upper_bounds = saved_list_element_upper_bounds;
        self.distinct_list_alloc_ids = saved_distinct_list_alloc_ids;
        result
    }

    fn adapt_function_adapter_param(
        &self,
        value: BasicValueEnum<'ctx>,
        expected_ty: &Type,
        actual_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        if expected_ty == actual_ty {
            return Ok(value);
        }
        match (expected_ty, actual_ty) {
            (Type::Integer, Type::Float) if value.is_int_value() => {
                return Ok(self
                    .builder
                    .build_signed_int_to_float(
                        value.into_int_value(),
                        self.context.f64_type(),
                        "fn_adapter_param_float",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to adapt function parameter from int to float")
                    })?
                    .into())
            }
            _ => {}
        }
        if self.is_supported_function_adapter_assignment(actual_ty, expected_ty) {
            return Ok(value);
        }
        Err(CodegenError::new(format!(
            "unsupported function adapter parameter conversion: {:?} -> {:?}",
            expected_ty, actual_ty
        )))
    }

    fn adapt_function_adapter_return(
        &self,
        value: BasicValueEnum<'ctx>,
        actual_ty: &Type,
        expected_ty: &Type,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>> {
        if actual_ty == expected_ty {
            return Ok(value);
        }
        match (actual_ty, expected_ty) {
            (Type::Integer, Type::Float) if value.is_int_value() => {
                return Ok(self
                    .builder
                    .build_signed_int_to_float(
                        value.into_int_value(),
                        self.context.f64_type(),
                        name,
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to adapt function return from int to float")
                    })?
                    .into())
            }
            _ => {}
        }
        if self.is_supported_function_adapter_assignment(expected_ty, actual_ty) {
            return Ok(value);
        }
        Err(CodegenError::new(format!(
            "unsupported function adapter return conversion: {:?} -> {:?}",
            actual_ty, expected_ty
        )))
    }

    fn compile_concat_display_strings(
        &mut self,
        left: PointerValue<'ctx>,
        right: PointerValue<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>> {
        let strlen_fn = self.get_or_declare_strlen();
        let strcpy_fn = self.get_or_declare_strcpy();
        let strcat_fn = self.get_or_declare_strcat();

        let left_len_call = self
            .builder
            .build_call(strlen_fn, &[left.into()], &format!("{name}_len1"))
            .map_err(|_| CodegenError::new("failed to emit strlen for left display string"))?;
        let left_len = self.extract_call_value(left_len_call)?.into_int_value();
        let right_len_call = self
            .builder
            .build_call(strlen_fn, &[right.into()], &format!("{name}_len2"))
            .map_err(|_| CodegenError::new("failed to emit strlen for right display string"))?;
        let right_len = self.extract_call_value(right_len_call)?.into_int_value();
        let total_len = self
            .builder
            .build_int_add(left_len, right_len, &format!("{name}_total"))
            .map_err(|_| CodegenError::new("failed to compute concatenated display length"))?;
        let buffer_size = self
            .builder
            .build_int_add(
                total_len,
                self.context.i64_type().const_int(1, false),
                &format!("{name}_bufsize"),
            )
            .map_err(|_| CodegenError::new("failed to compute display buffer size"))?;
        let buffer_call = self.build_malloc_call(
            buffer_size,
            &format!("{name}_buf"),
            "failed to allocate display concatenation buffer",
        )?;
        let buffer = self.extract_call_value(buffer_call)?.into_pointer_value();
        self.builder
            .build_call(strcpy_fn, &[buffer.into(), left.into()], "")
            .map_err(|_| CodegenError::new("failed to emit strcpy for display concatenation"))?;
        self.builder
            .build_call(strcat_fn, &[buffer.into(), right.into()], "")
            .map_err(|_| CodegenError::new("failed to emit strcat for display concatenation"))?;
        Ok(buffer)
    }

    pub fn compile_if_expr(
        &mut self,
        condition: &Expr,
        then_branch: &[Spanned<Stmt>],
        else_branch: Option<&Vec<Spanned<Stmt>>>,
        expected_ty: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let cond = self.compile_condition_expr(condition)?;

        let current_fn = self
            .current_function
            .ok_or(CodegenError::new("if expression outside of function"))?;

        let then_block = self.context.append_basic_block(current_fn, "if.then");
        let else_block = self.context.append_basic_block(current_fn, "if.else");
        let merge_block = self.context.append_basic_block(current_fn, "if.merge");

        self.builder
            .build_conditional_branch(cond, then_block, else_block)
            .map_err(|_| CodegenError::new("failed to branch for if expression"))?;

        let inferred_result_ty =
            self.infer_if_expr_result_type(then_branch, else_branch, &[], expected_ty);
        let expected_result_ty = expected_ty.or(match inferred_result_ty {
            Type::None => None,
            ref ty => Some(ty),
        });

        // Then branch
        self.builder.position_at_end(then_block);
        let then_result = self.with_variable_scope(|this| {
            let mut then_result = this.context.i8_type().const_int(0, false).into();
            for stmt in then_branch {
                if let Stmt::Expr(expr) = &stmt.node {
                    then_result = if let Some(expected_ty) = expected_result_ty {
                        let inferred_expr_ty = this.infer_expr_type(&expr.node, &[]);
                        let value =
                            this.compile_expr_with_expected_type(&expr.node, expected_ty)?;
                        this.reject_incompatible_expected_type_value(
                            expected_ty,
                            &inferred_expr_ty,
                            value,
                        )?;
                        value
                    } else {
                        this.compile_expr(&expr.node)?
                    };
                } else {
                    this.compile_stmt(&stmt.node)?;
                }
            }
            Ok(then_result)
        })?;
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(|_| CodegenError::new("failed to branch from if expression then block"))?;
        let then_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::new("if expression then predecessor block missing"))?;

        // Else branch
        self.builder.position_at_end(else_block);
        let else_result = self.with_variable_scope(|this| {
            let mut else_result = this.context.i8_type().const_int(0, false).into();
            if let Some(else_stmts) = else_branch {
                for stmt in else_stmts {
                    if let Stmt::Expr(expr) = &stmt.node {
                        else_result = if let Some(expected_ty) = expected_result_ty {
                            let inferred_expr_ty = this.infer_expr_type(&expr.node, &[]);
                            let value =
                                this.compile_expr_with_expected_type(&expr.node, expected_ty)?;
                            this.reject_incompatible_expected_type_value(
                                expected_ty,
                                &inferred_expr_ty,
                                value,
                            )?;
                            value
                        } else {
                            this.compile_expr(&expr.node)?
                        };
                    } else {
                        this.compile_stmt(&stmt.node)?;
                    }
                }
            }
            Ok(else_result)
        })?;
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(|_| CodegenError::new("failed to branch from if expression else block"))?;
        let else_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::new("if expression else predecessor block missing"))?;

        // Merge block with phi node
        self.builder.position_at_end(merge_block);
        if then_result.get_type() == else_result.get_type() {
            let phi = self
                .builder
                .build_phi(then_result.get_type(), "if.result")
                .map_err(|_| CodegenError::new("failed to build if expression phi"))?;
            phi.add_incoming(&[(&then_result, then_block), (&else_result, else_block)]);
            Ok(phi.as_basic_value())
        } else {
            Ok(then_result)
        }
    }

    fn compile_condition_expr(&mut self, expr: &Expr) -> Result<IntValue<'ctx>> {
        let cond_ty = self.infer_expr_type(expr, &[]);
        if !matches!(cond_ty, Type::Boolean) {
            return Err(CodegenError::new(format!(
                "Condition must be Boolean, found {}",
                Self::format_diagnostic_type(&cond_ty)
            )));
        }
        Ok(self.compile_expr(expr)?.into_int_value())
    }

    pub(crate) fn compile_integer_index_expr(&mut self, expr: &Expr) -> Result<IntValue<'ctx>> {
        let index_ty = self.infer_builtin_argument_type(expr);
        if !matches!(index_ty, Type::Integer) {
            return Err(CodegenError::new(format!(
                "Index must be Integer, found {}",
                Self::format_diagnostic_type(&index_ty)
            )));
        }
        Ok(self
            .compile_expr_with_expected_type(expr, &index_ty)?
            .into_int_value())
    }

    fn expr_is_provably_positive_in_scope(
        expr: &Expr,
        exact_integer_locals: &HashMap<String, i64>,
        non_negative_names: &HashSet<String>,
        call_non_negative: &HashSet<String>,
    ) -> bool {
        matches!(
            TypeChecker::eval_numeric_const_expr(expr),
            Some(NumericConst::Integer(value)) if value > 0
        ) || Self::exact_integer_value_in_scope(expr, exact_integer_locals)
            .is_some_and(|value| value > 0)
            || match expr {
                Expr::Binary {
                    op: BinOp::Add,
                    left,
                    right,
                } => {
                    Self::expr_is_provably_positive_in_scope(
                        &left.node,
                        exact_integer_locals,
                        non_negative_names,
                        call_non_negative,
                    ) && Self::expr_is_provably_non_negative_in_scope(
                        &right.node,
                        exact_integer_locals,
                        non_negative_names,
                        call_non_negative,
                    ) || Self::expr_is_provably_non_negative_in_scope(
                        &left.node,
                        exact_integer_locals,
                        non_negative_names,
                        call_non_negative,
                    ) && Self::expr_is_provably_positive_in_scope(
                        &right.node,
                        exact_integer_locals,
                        non_negative_names,
                        call_non_negative,
                    )
                }
                Expr::Binary {
                    op: BinOp::Mul,
                    left,
                    right,
                } => {
                    Self::expr_is_provably_positive_in_scope(
                        &left.node,
                        exact_integer_locals,
                        non_negative_names,
                        call_non_negative,
                    ) && Self::expr_is_provably_positive_in_scope(
                        &right.node,
                        exact_integer_locals,
                        non_negative_names,
                        call_non_negative,
                    )
                }
                _ => false,
            }
    }

    fn expr_is_provably_non_negative_in_scope(
        expr: &Expr,
        exact_integer_locals: &HashMap<String, i64>,
        non_negative_names: &HashSet<String>,
        call_non_negative: &HashSet<String>,
    ) -> bool {
        if matches!(
            TypeChecker::eval_numeric_const_expr(expr),
            Some(NumericConst::Integer(value)) if value >= 0
        ) {
            return true;
        }

        match expr {
            Expr::Ident(name) => non_negative_names.contains(name),
            Expr::Binary { op, left, right } => match op {
                BinOp::Add | BinOp::Mul => {
                    Self::expr_is_provably_non_negative_in_scope(
                        &left.node,
                        exact_integer_locals,
                        non_negative_names,
                        call_non_negative,
                    ) && Self::expr_is_provably_non_negative_in_scope(
                        &right.node,
                        exact_integer_locals,
                        non_negative_names,
                        call_non_negative,
                    )
                }
                BinOp::Sub => {
                    Self::expr_is_provably_non_negative_in_scope(
                        &left.node,
                        exact_integer_locals,
                        non_negative_names,
                        call_non_negative,
                    ) && matches!(
                        TypeChecker::eval_numeric_const_expr(&right.node),
                        Some(NumericConst::Integer(0))
                    )
                }
                BinOp::Div | BinOp::Mod => {
                    Self::expr_is_provably_non_negative_in_scope(
                        &left.node,
                        exact_integer_locals,
                        non_negative_names,
                        call_non_negative,
                    ) && Self::expr_is_provably_positive_in_scope(
                        &right.node,
                        exact_integer_locals,
                        non_negative_names,
                        call_non_negative,
                    )
                }
                _ => false,
            },
            Expr::Call { callee, args, .. } => match &callee.node {
                Expr::Ident(name) if call_non_negative.contains(name) => args.iter().all(|arg| {
                    Self::expr_is_provably_non_negative_in_scope(
                        &arg.node,
                        exact_integer_locals,
                        non_negative_names,
                        call_non_negative,
                    )
                }),
                _ => false,
            },
            _ => false,
        }
    }

    fn expr_is_provably_non_negative(&self, expr: &Expr) -> bool {
        Self::expr_is_provably_non_negative_in_scope(
            expr,
            &self.exact_integer_locals,
            &self.non_negative_locals,
            &self.non_negative_functions,
        )
    }

    fn expr_is_provably_non_zero_in_scope(
        expr: &Expr,
        exact_integer_locals: &HashMap<String, i64>,
        non_negative_names: &HashSet<String>,
        non_zero_names: &HashSet<String>,
        call_non_negative: &HashSet<String>,
    ) -> bool {
        matches!(
            TypeChecker::eval_numeric_const_expr(expr),
            Some(NumericConst::Integer(value)) if value != 0
        ) || matches!(expr, Expr::Ident(name) if non_zero_names.contains(name))
            || Self::expr_is_provably_positive_in_scope(
                expr,
                exact_integer_locals,
                non_negative_names,
                call_non_negative,
            )
    }

    fn expr_is_provably_non_zero(&self, expr: &Expr) -> bool {
        Self::expr_is_provably_non_zero_in_scope(
            expr,
            &self.exact_integer_locals,
            &self.non_negative_locals,
            &self.non_zero_locals,
            &self.non_negative_functions,
        )
    }

    fn expr_is_provably_not_negative_one(&self, expr: &Expr) -> bool {
        self.exact_integer_value(expr)
            .is_some_and(|value| value != -1)
            || self.expr_is_provably_non_negative(expr)
    }

    fn exact_integer_value_in_scope(
        expr: &Expr,
        exact_integer_locals: &HashMap<String, i64>,
    ) -> Option<i64> {
        if let Some(NumericConst::Integer(value)) = TypeChecker::eval_numeric_const_expr(expr) {
            return Some(value);
        }

        match expr {
            Expr::Ident(name) => exact_integer_locals.get(name).copied(),
            Expr::Binary { op, left, right } => {
                let left_value =
                    Self::exact_integer_value_in_scope(&left.node, exact_integer_locals)?;
                let right_value =
                    Self::exact_integer_value_in_scope(&right.node, exact_integer_locals)?;
                match op {
                    BinOp::Add => left_value.checked_add(right_value),
                    BinOp::Sub => left_value.checked_sub(right_value),
                    BinOp::Mul => left_value.checked_mul(right_value),
                    BinOp::Div => (right_value != 0).then(|| left_value / right_value),
                    BinOp::Mod => (right_value != 0).then(|| left_value % right_value),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn exact_integer_value(&self, expr: &Expr) -> Option<i64> {
        Self::exact_integer_value_in_scope(expr, &self.exact_integer_locals)
    }

    fn expr_upper_bound_exclusive_in_scope(
        expr: &Expr,
        exact_integer_locals: &HashMap<String, i64>,
        upper_bound_locals: &HashMap<String, i64>,
        non_negative_names: &HashSet<String>,
        call_non_negative: &HashSet<String>,
    ) -> Option<i64> {
        if let Some(exact) = Self::exact_integer_value_in_scope(expr, exact_integer_locals) {
            return exact.checked_add(1);
        }

        match expr {
            Expr::Ident(name) => upper_bound_locals.get(name).copied(),
            Expr::Binary {
                op: BinOp::Add,
                left,
                right,
            } => {
                let left_bound = Self::expr_upper_bound_exclusive_in_scope(
                    &left.node,
                    exact_integer_locals,
                    upper_bound_locals,
                    non_negative_names,
                    call_non_negative,
                )?;
                let right_bound = Self::expr_upper_bound_exclusive_in_scope(
                    &right.node,
                    exact_integer_locals,
                    upper_bound_locals,
                    non_negative_names,
                    call_non_negative,
                )?;
                left_bound.checked_add(right_bound)
            }
            Expr::Binary {
                op: BinOp::Sub,
                left,
                right,
            } => {
                let left_bound = Self::expr_upper_bound_exclusive_in_scope(
                    &left.node,
                    exact_integer_locals,
                    upper_bound_locals,
                    non_negative_names,
                    call_non_negative,
                )?;
                let right_exact =
                    Self::exact_integer_value_in_scope(&right.node, exact_integer_locals)?;
                left_bound.checked_sub(right_exact)
            }
            Expr::Binary {
                op: BinOp::Mul,
                left,
                right,
            } => {
                let left_bound = Self::expr_upper_bound_exclusive_in_scope(
                    &left.node,
                    exact_integer_locals,
                    upper_bound_locals,
                    non_negative_names,
                    call_non_negative,
                )?;
                let right_bound = Self::expr_upper_bound_exclusive_in_scope(
                    &right.node,
                    exact_integer_locals,
                    upper_bound_locals,
                    non_negative_names,
                    call_non_negative,
                )?;
                left_bound.checked_mul(right_bound)
            }
            Expr::Binary {
                op: BinOp::Mod,
                left,
                right,
            } if Self::expr_is_provably_non_negative_in_scope(
                &left.node,
                exact_integer_locals,
                non_negative_names,
                call_non_negative,
            ) =>
            {
                Self::exact_integer_value_in_scope(&right.node, exact_integer_locals)
                    .filter(|bound| *bound > 0)
            }
            _ => None,
        }
    }

    fn expr_upper_bound_exclusive(&self, expr: &Expr) -> Option<i64> {
        Self::expr_upper_bound_exclusive_in_scope(
            expr,
            &self.exact_integer_locals,
            &self.upper_bound_locals,
            &self.non_negative_locals,
            &self.non_negative_functions,
        )
    }

    pub(crate) fn expr_is_provably_below_exact_limit(&self, expr: &Expr, exact_limit: i64) -> bool {
        self.exact_integer_value(expr)
            .is_some_and(|value| value < exact_limit)
            || self
                .expr_upper_bound_exclusive(expr)
                .is_some_and(|bound| bound <= exact_limit)
    }

    fn collect_condition_non_negative_facts(expr: &Expr, names: &mut HashSet<String>) {
        match expr {
            Expr::Binary {
                op: BinOp::And,
                left,
                right,
            } => {
                Self::collect_condition_non_negative_facts(&left.node, names);
                Self::collect_condition_non_negative_facts(&right.node, names);
            }
            Expr::Binary {
                op: BinOp::GtEq,
                left,
                right,
            } => {
                if let Expr::Ident(name) = &left.node {
                    if matches!(
                        TypeChecker::eval_numeric_const_expr(&right.node),
                        Some(NumericConst::Integer(0))
                    ) {
                        names.insert(name.clone());
                    }
                }
                if let Expr::Ident(name) = &right.node {
                    if matches!(
                        TypeChecker::eval_numeric_const_expr(&left.node),
                        Some(NumericConst::Integer(0))
                    ) {
                        names.insert(name.clone());
                    }
                }
            }
            Expr::Binary {
                op: BinOp::Gt,
                left,
                right,
            } => {
                if let Expr::Ident(name) = &left.node {
                    if matches!(
                        TypeChecker::eval_numeric_const_expr(&right.node),
                        Some(NumericConst::Integer(-1))
                    ) {
                        names.insert(name.clone());
                    }
                }
                if let Expr::Ident(name) = &right.node {
                    if matches!(
                        TypeChecker::eval_numeric_const_expr(&left.node),
                        Some(NumericConst::Integer(-1))
                    ) {
                        names.insert(name.clone());
                    }
                }
            }
            _ => {}
        }
    }

    fn collect_condition_upper_bound_facts(
        expr: &Expr,
        exact_integer_locals: &HashMap<String, i64>,
        bounds: &mut HashMap<String, i64>,
    ) {
        match expr {
            Expr::Binary {
                op: BinOp::And,
                left,
                right,
            } => {
                Self::collect_condition_upper_bound_facts(&left.node, exact_integer_locals, bounds);
                Self::collect_condition_upper_bound_facts(
                    &right.node,
                    exact_integer_locals,
                    bounds,
                );
            }
            Expr::Binary {
                op: BinOp::Lt,
                left,
                right,
            } => {
                if let Expr::Ident(name) = &left.node {
                    if let Some(bound) =
                        Self::exact_integer_value_in_scope(&right.node, exact_integer_locals)
                    {
                        bounds
                            .entry(name.clone())
                            .and_modify(|current| *current = (*current).min(bound))
                            .or_insert(bound);
                    }
                }
            }
            Expr::Binary {
                op: BinOp::LtEq,
                left,
                right,
            } => {
                if let Expr::Ident(name) = &left.node {
                    if let Some(bound) =
                        Self::exact_integer_value_in_scope(&right.node, exact_integer_locals)
                            .and_then(|value| value.checked_add(1))
                    {
                        bounds
                            .entry(name.clone())
                            .and_modify(|current| *current = (*current).min(bound))
                            .or_insert(bound);
                    }
                }
            }
            _ => {}
        }
    }

    fn with_condition_non_negative_facts<T>(
        &mut self,
        condition: &Expr,
        f: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let saved_non_negative_locals = self.non_negative_locals.clone();
        let saved_upper_bound_locals = self.upper_bound_locals.clone();
        Self::collect_condition_non_negative_facts(condition, &mut self.non_negative_locals);
        Self::collect_condition_upper_bound_facts(
            condition,
            &self.exact_integer_locals,
            &mut self.upper_bound_locals,
        );
        let result = f(self);
        self.non_negative_locals = saved_non_negative_locals;
        self.upper_bound_locals = saved_upper_bound_locals;
        result
    }

    fn update_binding_non_negative_fact(&mut self, name: &str, ty: &Type, value: &Expr) {
        if matches!(ty, Type::Integer) {
            let derived_list_bound = self.list_element_upper_bound_from_index_like_expr(value);
            if self.expr_is_provably_non_negative(value) || derived_list_bound.is_some() {
                self.non_negative_locals.insert(name.to_string());
            } else {
                self.non_negative_locals.remove(name);
            }

            if self.expr_is_provably_non_zero(value) {
                self.non_zero_locals.insert(name.to_string());
            } else {
                self.non_zero_locals.remove(name);
            }
            if let Some(exact_value) = self.exact_integer_value(value) {
                self.exact_integer_locals
                    .insert(name.to_string(), exact_value);
            } else {
                self.exact_integer_locals.remove(name);
            }
            if let Some(upper_bound) =
                derived_list_bound.or_else(|| self.expr_upper_bound_exclusive(value))
            {
                self.upper_bound_locals
                    .insert(name.to_string(), upper_bound);
            } else {
                self.upper_bound_locals.remove(name);
            }
        } else {
            self.non_negative_locals.remove(name);
            self.non_zero_locals.remove(name);
            self.exact_integer_locals.remove(name);
            self.upper_bound_locals.remove(name);
        }
    }

    fn next_distinct_list_alloc_id(&mut self) -> u64 {
        let id = self.next_distinct_list_alloc_id;
        self.next_distinct_list_alloc_id = self.next_distinct_list_alloc_id.saturating_add(1);
        id
    }

    fn update_binding_list_alias_fact(&mut self, name: &str, ty: &Type, value: &Expr) {
        if !matches!(self.deref_codegen_type(ty), Type::List(_)) {
            self.distinct_list_alloc_ids.remove(name);
            return;
        }

        if self.expr_creates_empty_list(value, ty) {
            let alloc_id = self.next_distinct_list_alloc_id();
            self.distinct_list_alloc_ids
                .insert(name.to_string(), alloc_id);
            return;
        }

        if let Expr::Ident(source_name) = value {
            if let Some(existing_id) = self.distinct_list_alloc_ids.get(source_name).copied() {
                self.distinct_list_alloc_ids
                    .insert(name.to_string(), existing_id);
                return;
            }
        }

        self.distinct_list_alloc_ids.remove(name);
    }

    fn list_alias_domain_metadata(&self) -> Result<MetadataValue<'ctx>> {
        let function_name = self
            .current_function
            .map(|function| function.get_name().to_string_lossy().into_owned())
            .unwrap_or_else(|| "global".to_string());
        Ok(self.context.metadata_node(&[self
            .context
            .metadata_string(&format!("arden.list.domain.{function_name}"))
            .into()]))
    }

    fn list_alias_scope_metadata(
        &self,
        owner_name: &str,
        alloc_id: u64,
    ) -> Result<MetadataValue<'ctx>> {
        let domain = self.list_alias_domain_metadata()?;
        Ok(self.context.metadata_node(&[
            self.context
                .metadata_string(&format!("arden.list.scope.{owner_name}.{alloc_id}"))
                .into(),
            domain.into(),
        ]))
    }

    fn list_alias_scope_list_metadata(
        &self,
        scopes: impl IntoIterator<Item = MetadataValue<'ctx>>,
    ) -> MetadataValue<'ctx> {
        let values = scopes.into_iter().map(Into::into).collect::<Vec<_>>();
        self.context.metadata_node(&values)
    }

    fn list_alias_metadata_for_owner(
        &self,
        owner_name: &str,
    ) -> Result<Option<(MetadataValue<'ctx>, MetadataValue<'ctx>)>> {
        let Some(owner_alloc_id) = self.distinct_list_alloc_ids.get(owner_name).copied() else {
            return Ok(None);
        };

        let mut disjoint_scopes = Vec::new();
        for (other_name, other_alloc_id) in &self.distinct_list_alloc_ids {
            if other_name != owner_name && *other_alloc_id != owner_alloc_id {
                disjoint_scopes.push(self.list_alias_scope_metadata(other_name, *other_alloc_id)?);
            }
        }
        if disjoint_scopes.is_empty() {
            return Ok(None);
        }

        let owner_scope = self.list_alias_scope_metadata(owner_name, owner_alloc_id)?;
        Ok(Some((
            self.list_alias_scope_list_metadata([owner_scope]),
            self.list_alias_scope_list_metadata(disjoint_scopes),
        )))
    }

    pub(crate) fn apply_list_alias_metadata(
        &self,
        instruction: InstructionValue<'ctx>,
        owner_name: Option<&str>,
    ) -> Result<()> {
        let Some(owner_name) = owner_name else {
            return Ok(());
        };
        let Some((alias_scope, noalias)) = self.list_alias_metadata_for_owner(owner_name)? else {
            return Ok(());
        };

        instruction
            .set_metadata(alias_scope, self.context.get_kind_id("alias.scope"))
            .map_err(|_| CodegenError::new("failed to attach alias.scope metadata"))?;
        instruction
            .set_metadata(noalias, self.context.get_kind_id("noalias"))
            .map_err(|_| CodegenError::new("failed to attach noalias metadata"))?;
        Ok(())
    }

    fn list_element_upper_bound_from_index_like_expr(&self, expr: &Expr) -> Option<i64> {
        match expr {
            Expr::Call { callee, args, .. } if args.len() == 1 => {
                let Expr::Field { object, field } = &callee.node else {
                    return None;
                };
                if field != "get" {
                    return None;
                }
                let Expr::Ident(owner_name) = &object.node else {
                    return None;
                };
                let upper_bound = self.list_element_upper_bounds.get(owner_name).copied()?;
                let exact_length = self.exact_list_lengths.get(owner_name).copied()?;
                self.expr_is_provably_below_exact_limit(&args[0].node, exact_length)
                    .then_some(upper_bound)
            }
            Expr::Index { object, index } => {
                let Expr::Ident(owner_name) = &object.node else {
                    return None;
                };
                let upper_bound = self.list_element_upper_bounds.get(owner_name).copied()?;
                let exact_length = self.exact_list_lengths.get(owner_name).copied()?;
                self.expr_is_provably_below_exact_limit(&index.node, exact_length)
                    .then_some(upper_bound)
            }
            _ => None,
        }
    }

    fn function_returns_provably_non_negative(func: &FunctionDecl) -> bool {
        if !matches!(func.return_type, Type::Integer) {
            return false;
        }

        let non_negative_params = func
            .params
            .iter()
            .filter(|param| matches!(param.ty, Type::Integer))
            .map(|param| param.name.clone())
            .collect::<HashSet<_>>();

        let return_expr = match func.body.as_slice() {
            [Spanned {
                node: Stmt::Return(Some(expr)),
                ..
            }] => &expr.node,
            [Spanned {
                node: Stmt::Expr(expr),
                ..
            }] => &expr.node,
            _ => return false,
        };

        Self::expr_is_provably_non_negative_in_scope(
            return_expr,
            &HashMap::new(),
            &non_negative_params,
            &HashSet::new(),
        )
    }

    pub(crate) fn compile_non_negative_integer_index_expr_with_proof(
        &mut self,
        expr: &Expr,
        negative_diagnostic: &str,
    ) -> Result<(IntValue<'ctx>, bool)> {
        if matches!(
            TypeChecker::eval_numeric_const_expr(expr),
            Some(NumericConst::Integer(value)) if value < 0
        ) {
            return Err(CodegenError::new(negative_diagnostic));
        }
        let provably_non_negative = self.expr_is_provably_non_negative(expr);
        Ok((
            self.compile_integer_index_expr(expr)?,
            provably_non_negative,
        ))
    }

    fn compile_integer_iteration_bound(&mut self, expr: &Expr) -> Result<IntValue<'ctx>> {
        let expr_ty = self.infer_builtin_argument_type(expr);
        if !matches!(expr_ty, Type::Integer) {
            return Err(CodegenError::new(format!(
                "Cannot iterate over {}",
                Self::format_diagnostic_type(&expr_ty)
            )));
        }
        Ok(self
            .compile_expr_with_expected_type(expr, &expr_ty)?
            .into_int_value())
    }

    fn compile_string_argument_expr(
        &mut self,
        expr: &Expr,
        diagnostic: impl Into<String>,
    ) -> Result<PointerValue<'ctx>> {
        let expr_ty = self.infer_builtin_argument_type(expr);
        if !matches!(expr_ty, Type::String) {
            return Err(CodegenError::new(diagnostic.into()));
        }
        Ok(self
            .compile_expr_with_expected_type(expr, &expr_ty)?
            .into_pointer_value())
    }

    fn concrete_zero_arg_builtin_value_type(name: &str) -> Option<Type> {
        match name {
            "read_line" | "System__cwd" | "System__os" => Some(Type::String),
            "Time__unix" | "Args__count" => Some(Type::Integer),
            "Math__pi" | "Math__e" | "Math__random" => Some(Type::Float),
            _ => None,
        }
    }

    pub(crate) fn builtin_argument_type_hint(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Ident(_) | Expr::Field { .. } => self
                .resolve_contextual_function_value_name(expr)
                .and_then(|name| Self::concrete_zero_arg_builtin_value_type(&name)),
            Expr::Literal(lit) => Some(match lit {
                Literal::Integer(_) => Type::Integer,
                Literal::Float(_) => Type::Float,
                Literal::Boolean(_) => Type::Boolean,
                Literal::String(_) => Type::String,
                Literal::Char(_) => Type::Char,
                Literal::None => Type::None,
            }),
            Expr::StringInterp(_) => Some(Type::String),
            Expr::Block(body) => self.builtin_argument_block_type_hint(body),
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_ty = self.builtin_argument_block_type_hint(then_branch)?;
                let else_ty = self.builtin_argument_block_type_hint(else_branch.as_ref()?)?;
                self.common_compatible_codegen_type(&then_ty, &else_ty)
            }
            Expr::Match { arms, .. } => {
                let mut arm_types = arms
                    .iter()
                    .filter_map(|arm| self.builtin_argument_block_type_hint(&arm.body));
                let first = arm_types.next()?;
                arm_types.try_fold(first, |acc, ty| {
                    self.common_compatible_codegen_type(&acc, &ty)
                })
            }
            _ => None,
        }
    }

    fn builtin_argument_block_type_hint(&self, body: &[Spanned<Stmt>]) -> Option<Type> {
        body.iter().rev().find_map(|stmt| match &stmt.node {
            Stmt::Expr(expr) => self.builtin_argument_type_hint(&expr.node),
            _ => None,
        })
    }

    pub(crate) fn infer_builtin_argument_type(&self, expr: &Expr) -> Type {
        if let Some(ty) = self.builtin_argument_type_hint(expr) {
            return ty;
        }
        self.infer_expr_type(expr, &[])
    }

    pub fn compile_literal(&mut self, lit: &Literal) -> Result<BasicValueEnum<'ctx>> {
        match lit {
            Literal::Integer(n) => Ok(self.context.i64_type().const_int(*n as u64, true).into()),
            Literal::Float(n) => Ok(self.context.f64_type().const_float(*n).into()),
            Literal::Boolean(b) => Ok(self.context.bool_type().const_int(*b as u64, false).into()),
            Literal::String(s) => {
                let str_val = self.context.const_string(s.as_bytes(), true);
                let name = format!("str.{}", self.str_counter);
                self.str_counter += 1;
                let global = self.module.add_global(str_val.get_type(), None, &name);
                global.set_linkage(Linkage::Private);
                global.set_initializer(&str_val);
                global.set_constant(true);
                Ok(global.as_pointer_value().into())
            }
            Literal::Char(c) => Ok(self.context.i32_type().const_int(*c as u64, false).into()),
            Literal::None => Ok(self.context.i8_type().const_int(0, false).into()),
        }
    }

    pub fn compile_binary(
        &mut self,
        op: BinOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<BasicValueEnum<'ctx>> {
        let left_ty = self.infer_builtin_argument_type(left);
        let right_ty = self.infer_builtin_argument_type(right);

        if matches!(op, BinOp::Eq | BinOp::NotEq) {
            let try_expected_eq = |this: &mut Self,
                                   expected_ty: &Type,
                                   expected_expr: &Expr,
                                   other_expr: &Expr|
             -> Result<Option<BasicValueEnum<'ctx>>> {
                match expected_ty {
                    Type::Option(_) | Type::Result(_, _) => {
                        let lhs =
                            this.compile_expr_with_expected_type(expected_expr, expected_ty)?;
                        let rhs = this.compile_expr_with_expected_type(other_expr, expected_ty)?;
                        let eq = this.build_value_equality(lhs, rhs, expected_ty, "eq")?;
                        let result = if matches!(op, BinOp::Eq) {
                            eq
                        } else {
                            this.builder.build_not(eq, "ne").map_err(|_| {
                                CodegenError::new("failed to negate equality result")
                            })?
                        };
                        Ok(Some(result.into()))
                    }
                    _ => Ok(None),
                }
            };

            if left_ty == right_ty {
                let lhs = self.compile_expr_with_expected_type(left, &left_ty)?;
                let rhs = self.compile_expr_with_expected_type(right, &right_ty)?;
                let eq = self.build_value_equality(lhs, rhs, &left_ty, "eq")?;
                let result = if matches!(op, BinOp::Eq) {
                    eq
                } else {
                    self.builder
                        .build_not(eq, "ne")
                        .map_err(|_| CodegenError::new("failed to negate equality result"))?
                };
                return Ok(result.into());
            }

            if let Some(result) = try_expected_eq(self, &left_ty, left, right)? {
                return Ok(result);
            }
            if let Some(result) = try_expected_eq(self, &right_ty, left, right)? {
                return Ok(result);
            }
        }

        self.ensure_binary_operator_supported(op, &left_ty, &right_ty)?;
        let lhs = self.compile_expr_with_expected_type(left, &left_ty)?;
        let rhs = self.compile_expr_with_expected_type(right, &right_ty)?;
        if matches!(op, BinOp::Mod)
            && matches!(left_ty, Type::Integer)
            && matches!(right_ty, Type::Integer)
            && self.expr_is_provably_non_negative(left)
            && self
                .exact_integer_value(right)
                .is_some_and(|value| value > 0 && (value as u64).is_power_of_two())
        {
            let lhs = lhs.into_int_value();
            let rhs = rhs.into_int_value();
            let mask = self
                .builder
                .build_int_sub(
                    rhs,
                    self.context.i64_type().const_int(1, false),
                    "pow2_mod_mask",
                )
                .map_err(|_| CodegenError::new("failed to compute power-of-two modulo mask"))?;
            let reduced = self
                .builder
                .build_and(lhs, mask, "pow2_mod")
                .map_err(|_| CodegenError::new("failed to emit power-of-two modulo"))?;
            return Ok(reduced.into());
        }
        let skip_signed_division_overflow_guard = matches!(op, BinOp::Div | BinOp::Mod)
            && (self.expr_is_provably_non_negative(left)
                || self.expr_is_provably_not_negative_one(right));
        let options = BinaryCodegenOptions {
            skip_nonzero_divisor_guard: self.expr_is_provably_non_zero(right),
            skip_signed_division_overflow_guard,
        };
        self.compile_binary_values_unchecked_with_options(
            op, lhs, rhs, &left_ty, &right_ty, options,
        )
    }

    fn compile_binary_values(
        &mut self,
        op: BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
        left_ty: &Type,
        right_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        self.ensure_binary_operator_supported(op, left_ty, right_ty)?;
        self.compile_binary_values_unchecked_with_options(
            op,
            lhs,
            rhs,
            left_ty,
            right_ty,
            BinaryCodegenOptions::default(),
        )
    }

    fn compile_binary_values_unchecked_with_options(
        &mut self,
        op: BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
        left_ty: &Type,
        right_ty: &Type,
        options: BinaryCodegenOptions,
    ) -> Result<BasicValueEnum<'ctx>> {
        if left_ty.is_numeric()
            && right_ty.is_numeric()
            && (matches!(left_ty, Type::Float) || matches!(right_ty, Type::Float))
        {
            let l = if lhs.is_float_value() {
                lhs.into_float_value()
            } else {
                self.builder
                    .build_signed_int_to_float(lhs.into_int_value(), self.context.f64_type(), "lf")
                    .map_err(|_| CodegenError::new("failed to promote left numeric operand"))?
            };
            let r = if rhs.is_float_value() {
                rhs.into_float_value()
            } else {
                self.builder
                    .build_signed_int_to_float(rhs.into_int_value(), self.context.f64_type(), "rf")
                    .map_err(|_| CodegenError::new("failed to promote right numeric operand"))?
            };

            let result = match op {
                BinOp::Add => self
                    .builder
                    .build_float_add(l, r, "fadd")
                    .map_err(|_| CodegenError::new("failed to emit float addition"))?
                    .into(),
                BinOp::Sub => self
                    .builder
                    .build_float_sub(l, r, "fsub")
                    .map_err(|_| CodegenError::new("failed to emit float subtraction"))?
                    .into(),
                BinOp::Mul => self
                    .builder
                    .build_float_mul(l, r, "fmul")
                    .map_err(|_| CodegenError::new("failed to emit float multiplication"))?
                    .into(),
                BinOp::Div => self
                    .builder
                    .build_float_div(l, r, "fdiv")
                    .map_err(|_| CodegenError::new("failed to emit float division"))?
                    .into(),
                BinOp::Mod => self
                    .builder
                    .build_float_rem(l, r, "frem")
                    .map_err(|_| CodegenError::new("failed to emit float remainder"))?
                    .into(),
                BinOp::Eq => self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, "feq")
                    .map_err(|_| CodegenError::new("failed to emit float equality comparison"))?
                    .into(),
                BinOp::NotEq => self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, l, r, "fne")
                    .map_err(|_| CodegenError::new("failed to emit float inequality comparison"))?
                    .into(),
                BinOp::Lt => self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, l, r, "flt")
                    .map_err(|_| CodegenError::new("failed to emit float less-than comparison"))?
                    .into(),
                BinOp::LtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, l, r, "fle")
                    .map_err(|_| {
                        CodegenError::new("failed to emit float less-or-equal comparison")
                    })?
                    .into(),
                BinOp::Gt => self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, l, r, "fgt")
                    .map_err(|_| CodegenError::new("failed to emit float greater-than comparison"))?
                    .into(),
                BinOp::GtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, l, r, "fge")
                    .map_err(|_| {
                        CodegenError::new("failed to emit float greater-or-equal comparison")
                    })?
                    .into(),
                _ => return Err(CodegenError::new("Invalid float operation")),
            };
            return Ok(result);
        }

        // Integer operations
        if lhs.is_int_value() && rhs.is_int_value() {
            let l = lhs.into_int_value();
            let r = rhs.into_int_value();

            if matches!(op, BinOp::Div | BinOp::Mod) {
                let (message, global_name) = if matches!(op, BinOp::Div) {
                    ("Integer division by zero", "int_div_zero")
                } else {
                    ("Integer modulo by zero", "int_mod_zero")
                };
                if !options.skip_nonzero_divisor_guard {
                    self.guard_nonzero_integer_divisor(r, message, global_name)?;
                }
                let (overflow_message, overflow_prefix) = if matches!(op, BinOp::Div) {
                    ("Integer division overflow", "int_div_overflow")
                } else {
                    ("Integer modulo overflow", "int_mod_overflow")
                };
                if !options.skip_signed_division_overflow_guard {
                    self.guard_signed_division_overflow(l, r, overflow_message, overflow_prefix)?;
                }
            }

            let result = match op {
                BinOp::Add => self
                    .builder
                    .build_int_add(l, r, "add")
                    .map_err(|_| CodegenError::new("failed to emit integer addition"))?,
                BinOp::Sub => self
                    .builder
                    .build_int_sub(l, r, "sub")
                    .map_err(|_| CodegenError::new("failed to emit integer subtraction"))?,
                BinOp::Mul => self
                    .builder
                    .build_int_mul(l, r, "mul")
                    .map_err(|_| CodegenError::new("failed to emit integer multiplication"))?,
                BinOp::Div => self
                    .builder
                    .build_int_signed_div(l, r, "div")
                    .map_err(|_| CodegenError::new("failed to emit integer division"))?,
                BinOp::Mod => self
                    .builder
                    .build_int_signed_rem(l, r, "mod")
                    .map_err(|_| CodegenError::new("failed to emit integer remainder"))?,
                BinOp::Eq => self
                    .builder
                    .build_int_compare(IntPredicate::EQ, l, r, "eq")
                    .map_err(|_| CodegenError::new("failed to emit integer equality comparison"))?,
                BinOp::NotEq => self
                    .builder
                    .build_int_compare(IntPredicate::NE, l, r, "ne")
                    .map_err(|_| {
                        CodegenError::new("failed to emit integer inequality comparison")
                    })?,
                BinOp::Lt => self
                    .builder
                    .build_int_compare(IntPredicate::SLT, l, r, "lt")
                    .map_err(|_| {
                        CodegenError::new("failed to emit integer less-than comparison")
                    })?,
                BinOp::LtEq => self
                    .builder
                    .build_int_compare(IntPredicate::SLE, l, r, "le")
                    .map_err(|_| {
                        CodegenError::new("failed to emit integer less-or-equal comparison")
                    })?,
                BinOp::Gt => self
                    .builder
                    .build_int_compare(IntPredicate::SGT, l, r, "gt")
                    .map_err(|_| {
                        CodegenError::new("failed to emit integer greater-than comparison")
                    })?,
                BinOp::GtEq => self
                    .builder
                    .build_int_compare(IntPredicate::SGE, l, r, "ge")
                    .map_err(|_| {
                        CodegenError::new("failed to emit integer greater-or-equal comparison")
                    })?,
                BinOp::And => self
                    .builder
                    .build_and(l, r, "and")
                    .map_err(|_| CodegenError::new("failed to emit integer and"))?,
                BinOp::Or => self
                    .builder
                    .build_or(l, r, "or")
                    .map_err(|_| CodegenError::new("failed to emit integer or"))?,
            };
            return Ok(result.into());
        }

        // Float operations
        if lhs.is_float_value() && rhs.is_float_value() {
            let l = lhs.into_float_value();
            let r = rhs.into_float_value();

            let result = match op {
                BinOp::Add => self
                    .builder
                    .build_float_add(l, r, "fadd")
                    .map_err(|_| CodegenError::new("failed to emit float addition"))?
                    .into(),
                BinOp::Sub => self
                    .builder
                    .build_float_sub(l, r, "fsub")
                    .map_err(|_| CodegenError::new("failed to emit float subtraction"))?
                    .into(),
                BinOp::Mul => self
                    .builder
                    .build_float_mul(l, r, "fmul")
                    .map_err(|_| CodegenError::new("failed to emit float multiplication"))?
                    .into(),
                BinOp::Div => self
                    .builder
                    .build_float_div(l, r, "fdiv")
                    .map_err(|_| CodegenError::new("failed to emit float division"))?
                    .into(),
                BinOp::Mod => self
                    .builder
                    .build_float_rem(l, r, "frem")
                    .map_err(|_| CodegenError::new("failed to emit float remainder"))?
                    .into(),
                BinOp::Eq => self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, "feq")
                    .map_err(|_| CodegenError::new("failed to emit float equality comparison"))?
                    .into(),
                BinOp::NotEq => self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, l, r, "fne")
                    .map_err(|_| CodegenError::new("failed to emit float inequality comparison"))?
                    .into(),
                BinOp::Lt => self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, l, r, "flt")
                    .map_err(|_| CodegenError::new("failed to emit float less-than comparison"))?
                    .into(),
                BinOp::LtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, l, r, "fle")
                    .map_err(|_| {
                        CodegenError::new("failed to emit float less-or-equal comparison")
                    })?
                    .into(),
                BinOp::Gt => self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, l, r, "fgt")
                    .map_err(|_| CodegenError::new("failed to emit float greater-than comparison"))?
                    .into(),
                BinOp::GtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, l, r, "fge")
                    .map_err(|_| {
                        CodegenError::new("failed to emit float greater-or-equal comparison")
                    })?
                    .into(),
                _ => return Err(CodegenError::new("Invalid float operation")),
            };
            return Ok(result);
        }

        // String concatenation
        if matches!(op, BinOp::Add)
            && matches!(left_ty, Type::String)
            && matches!(right_ty, Type::String)
        {
            let strlen_fn = self.get_or_declare_strlen();
            let strcpy_fn = self.get_or_declare_strcpy();
            let strcat_fn = self.get_or_declare_strcat();
            let s1 = lhs.into_pointer_value();
            let s2 = rhs.into_pointer_value();

            let len1_call = self
                .builder
                .build_call(strlen_fn, &[s1.into()], "len1")
                .map_err(|_| CodegenError::new("failed to emit strlen for left string"))?;
            let len1 = self.extract_call_value(len1_call)?.into_int_value();
            let len2_call = self
                .builder
                .build_call(strlen_fn, &[s2.into()], "len2")
                .map_err(|_| CodegenError::new("failed to emit strlen for right string"))?;
            let len2 = self.extract_call_value(len2_call)?.into_int_value();
            let total_len = self
                .builder
                .build_int_add(len1, len2, "total")
                .map_err(|_| CodegenError::new("failed to compute concatenated string length"))?;
            let buffer_size = self
                .builder
                .build_int_add(
                    total_len,
                    self.context.i64_type().const_int(1, false),
                    "bufsize",
                )
                .map_err(|_| CodegenError::new("failed to compute string buffer size"))?;
            let buffer_call = self.build_malloc_call(
                buffer_size,
                "buf",
                "failed to allocate concatenated string buffer",
            )?;
            let buffer = self.extract_call_value(buffer_call)?.into_pointer_value();
            self.builder
                .build_call(strcpy_fn, &[buffer.into(), s1.into()], "")
                .map_err(|_| CodegenError::new("failed to emit strcpy for string concatenation"))?;
            self.builder
                .build_call(strcat_fn, &[buffer.into(), s2.into()], "")
                .map_err(|_| CodegenError::new("failed to emit strcat for string concatenation"))?;
            return Ok(buffer.into());
        }

        let left_is_string = matches!(left_ty, Type::String);
        let right_is_string = matches!(right_ty, Type::String);
        if left_is_string && right_is_string && matches!(op, BinOp::Eq | BinOp::NotEq) {
            let lhs = lhs.into_pointer_value();
            let rhs = rhs.into_pointer_value();
            let strcmp = self.get_or_declare_strcmp();
            let cmp = self
                .builder
                .build_call(strcmp, &[lhs.into(), rhs.into()], "strcmp")
                .map_err(|e| CodegenError::new(format!("strcmp call failed: {}", e)))?;
            let cmp = self.extract_call_value(cmp)?.into_int_value();
            let result = if matches!(op, BinOp::Eq) {
                self.builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        cmp,
                        self.context.i32_type().const_zero(),
                        "str_eq",
                    )
                    .map_err(|_| CodegenError::new("failed to emit string equality comparison"))?
            } else {
                self.builder
                    .build_int_compare(
                        IntPredicate::NE,
                        cmp,
                        self.context.i32_type().const_zero(),
                        "str_ne",
                    )
                    .map_err(|_| CodegenError::new("failed to emit string inequality comparison"))?
            };
            return Ok(result.into());
        }

        Err(CodegenError::new(format!(
            "Type mismatch in binary operation {:?}: left={}, right={}",
            op,
            Self::format_type_string(left_ty),
            Self::format_type_string(right_ty)
        )))
    }

    fn ensure_binary_operator_supported(
        &self,
        op: BinOp,
        left_ty: &Type,
        right_ty: &Type,
    ) -> Result<()> {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                if matches!(op, BinOp::Add)
                    && matches!(left_ty, Type::String)
                    && matches!(right_ty, Type::String)
                {
                    return Ok(());
                }
                if left_ty.is_numeric() && right_ty.is_numeric() {
                    Ok(())
                } else {
                    Err(CodegenError::new(format!(
                        "Arithmetic operator requires numeric types, got {} and {}",
                        Self::format_diagnostic_type(left_ty),
                        Self::format_diagnostic_type(right_ty)
                    )))
                }
            }
            BinOp::Eq | BinOp::NotEq => {
                if left_ty == right_ty || (left_ty.is_numeric() && right_ty.is_numeric()) {
                    Ok(())
                } else {
                    Err(CodegenError::new(format!(
                        "Cannot compare {} and {}",
                        Self::format_diagnostic_type(left_ty),
                        Self::format_diagnostic_type(right_ty)
                    )))
                }
            }
            BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                if left_ty.supports_ordered_comparison_with(right_ty) {
                    Ok(())
                } else {
                    Err(CodegenError::new(format!(
                        "Comparison requires ordered types, got {} and {}",
                        Self::format_diagnostic_type(left_ty),
                        Self::format_diagnostic_type(right_ty)
                    )))
                }
            }
            BinOp::And | BinOp::Or => {
                if matches!(left_ty, Type::Boolean) && matches!(right_ty, Type::Boolean) {
                    Ok(())
                } else {
                    Err(CodegenError::new(format!(
                        "Logical operator requires Boolean types, got {} and {}",
                        Self::format_diagnostic_type(left_ty),
                        Self::format_diagnostic_type(right_ty)
                    )))
                }
            }
        }
    }

    fn guard_nonzero_integer_divisor(
        &mut self,
        divisor: IntValue<'ctx>,
        message: &str,
        block_prefix: &str,
    ) -> Result<()> {
        let current_fn = self
            .current_function
            .ok_or_else(|| CodegenError::new("integer division guard emitted outside function"))?;
        let zero = divisor.get_type().const_zero();
        let is_zero = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                divisor,
                zero,
                &format!("{block_prefix}_is_zero"),
            )
            .map_err(|_| CodegenError::new("failed to compare integer divisor against zero"))?;
        let ok_block = self
            .context
            .append_basic_block(current_fn, &format!("{block_prefix}_ok"));
        let error_block = self
            .context
            .append_basic_block(current_fn, &format!("{block_prefix}_error"));
        self.builder
            .build_conditional_branch(is_zero, error_block, ok_block)
            .map_err(|_| CodegenError::new("failed to branch for integer division guard"))?;

        self.builder.position_at_end(error_block);
        self.emit_runtime_error(message, &format!("{block_prefix}_runtime_error"))?;

        self.builder.position_at_end(ok_block);
        Ok(())
    }

    fn guard_signed_division_overflow(
        &mut self,
        dividend: IntValue<'ctx>,
        divisor: IntValue<'ctx>,
        message: &str,
        block_prefix: &str,
    ) -> Result<()> {
        let current_fn = self.current_function.ok_or_else(|| {
            CodegenError::new("integer division overflow guard emitted outside function")
        })?;
        let int_ty = dividend.get_type();
        if int_ty != divisor.get_type() {
            return Err(CodegenError::new(
                "integer division overflow guard requires same-width operands",
            ));
        }
        let bit_width = int_ty.get_bit_width();
        if bit_width == 0 {
            return Err(CodegenError::new(
                "integer division overflow guard requires non-zero-width integer type",
            ));
        }
        if bit_width > 64 {
            return Err(CodegenError::new(
                "integer division overflow guard supports up to 64-bit integers",
            ));
        }

        let min_value_bits = 1u128 << (bit_width - 1);
        let min_value = int_ty.const_int(min_value_bits as u64, false);
        let minus_one = int_ty.const_all_ones();
        let dividend_is_min = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                dividend,
                min_value,
                &format!("{block_prefix}_dividend_is_min"),
            )
            .map_err(|_| CodegenError::new("failed to compare signed dividend minimum guard"))?;
        let divisor_is_minus_one = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                divisor,
                minus_one,
                &format!("{block_prefix}_divisor_is_minus_one"),
            )
            .map_err(|_| CodegenError::new("failed to compare signed divisor minus-one guard"))?;
        let is_overflow = self
            .builder
            .build_and(
                dividend_is_min,
                divisor_is_minus_one,
                &format!("{block_prefix}_is_overflow"),
            )
            .map_err(|_| CodegenError::new("failed to combine signed division overflow guards"))?;
        let ok_block = self
            .context
            .append_basic_block(current_fn, &format!("{block_prefix}_ok"));
        let error_block = self
            .context
            .append_basic_block(current_fn, &format!("{block_prefix}_error"));
        self.builder
            .build_conditional_branch(is_overflow, error_block, ok_block)
            .map_err(|_| {
                CodegenError::new("failed to branch for signed division overflow guard")
            })?;

        self.builder.position_at_end(error_block);
        self.emit_runtime_error(message, &format!("{block_prefix}_runtime_error"))?;

        self.builder.position_at_end(ok_block);
        Ok(())
    }

    pub fn compile_unary(&mut self, op: UnaryOp, expr: &Expr) -> Result<BasicValueEnum<'ctx>> {
        let expr_ty = self.infer_builtin_argument_type(expr);

        match op {
            UnaryOp::Neg => {
                if !expr_ty.is_numeric() {
                    return Err(CodegenError::new(format!(
                        "Cannot negate non-numeric type {}",
                        Self::format_diagnostic_type(&expr_ty)
                    )));
                }
                let val = self.compile_expr_with_expected_type(expr, &expr_ty)?;
                if val.is_int_value() {
                    Ok(self
                        .builder
                        .build_int_neg(val.into_int_value(), "neg")
                        .map_err(|_| CodegenError::new("failed to emit integer negation"))?
                        .into())
                } else if val.is_float_value() {
                    Ok(self
                        .builder
                        .build_float_neg(val.into_float_value(), "fneg")
                        .map_err(|_| CodegenError::new("failed to emit float negation"))?
                        .into())
                } else {
                    Err(CodegenError::new("Cannot negate non-numeric value"))
                }
            }
            UnaryOp::Not => {
                if !matches!(expr_ty, Type::Boolean) {
                    return Err(CodegenError::new(format!(
                        "Cannot apply '!' to non-boolean type {}",
                        Self::format_diagnostic_type(&expr_ty)
                    )));
                }
                let val = self.compile_expr_with_expected_type(expr, &expr_ty)?;
                Ok(self
                    .builder
                    .build_not(val.into_int_value(), "not")
                    .map_err(|_| CodegenError::new("failed to emit boolean negation"))?
                    .into())
            }
        }
    }

    pub fn compile_method_call(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        if let Some(err) = self.call_expr_arity_error(object) {
            return Err(err);
        }
        if let Some(name) = self.member_root_undefined_variable(object) {
            return Err(Self::undefined_variable_error(&name));
        }
        // Infer object type first
        let inferred_obj_ty = self
            .infer_object_type(object)
            .or_else(|| Some(self.infer_builtin_argument_type(object)));
        let obj_ty = inferred_obj_ty
            .clone()
            .or_else(|| {
                let Expr::Call {
                    callee, type_args, ..
                } = object
                else {
                    return None;
                };
                let path_parts = flatten_field_chain(&callee.node)?;
                let full_path = path_parts.join(".");
                if !type_args.is_empty() {
                    let normalized = self
                        .resolve_alias_qualified_codegen_type_name(&full_path)
                        .and_then(|resolved| {
                            self.normalize_user_defined_generic_type(&resolved, type_args)
                        })
                        .unwrap_or_else(|| {
                            self.normalize_codegen_type(&Type::Generic(
                                full_path.clone(),
                                type_args.clone(),
                            ))
                        });
                    if self.type_to_class_name(&normalized).is_some() {
                        return Some(normalized);
                    }
                }
                self.resolve_alias_qualified_codegen_type_name(&full_path)
                    .filter(|resolved| self.classes.contains_key(resolved))
                    .map(Type::Named)
            })
            .or_else(|| Some(self.infer_builtin_argument_type(object)));
        let deref_obj_ty = obj_ty
            .clone()
            .map(|ty| self.deref_codegen_type(&ty).clone());
        let is_reference_receiver = matches!(obj_ty, Some(Type::Ref(_)) | Some(Type::MutRef(_)));

        // Handle built-in types (List, Map, Set, Option, Result) for any expression
        if let Some(ref ty) = deref_obj_ty {
            match ty {
                Type::List(_) => {
                    let list_ptr = match object {
                        Expr::Ident(name) if !is_reference_receiver => Ok(self
                            .variables
                            .get(name)
                            .map(|v| (v.ptr, Some(name.as_str())))),
                        Expr::Field { object: obj, field } => self
                            .compile_field_ptr(&obj.node, field)
                            .map(|ptr| Some((ptr, None))),
                        Expr::This if !is_reference_receiver => {
                            Ok(self.variables.get("this").map(|v| (v.ptr, None)))
                        }
                        _ => Ok(None),
                    }?;
                    if let Some((ptr, owner_name)) = list_ptr {
                        let result =
                            self.compile_list_method_ptr(ptr, ty, method, args, owner_name)?;
                        if let Some(owner_name) = owner_name {
                            match method {
                                "push" => {
                                    if let Some(length) =
                                        self.exact_list_lengths.get_mut(owner_name)
                                    {
                                        if let Some(next_length) = length.checked_add(1) {
                                            *length = next_length;
                                        } else {
                                            self.exact_list_lengths.remove(owner_name);
                                        }
                                    }
                                    if let Some(push_value) = args.first() {
                                        if self.expr_is_provably_non_negative(&push_value.node) {
                                            if let Some(upper_bound) =
                                                self.expr_upper_bound_exclusive(&push_value.node)
                                            {
                                                self.list_element_upper_bounds
                                                    .entry(owner_name.to_string())
                                                    .and_modify(|current| {
                                                        *current = (*current).max(upper_bound)
                                                    })
                                                    .or_insert(upper_bound);
                                            } else {
                                                self.list_element_upper_bounds.remove(owner_name);
                                            }
                                        } else {
                                            self.list_element_upper_bounds.remove(owner_name);
                                        }
                                    }
                                }
                                "pop" => {
                                    if let Some(length) =
                                        self.exact_list_lengths.get_mut(owner_name)
                                    {
                                        if *length > 0 {
                                            *length -= 1;
                                        } else {
                                            self.exact_list_lengths.remove(owner_name);
                                        }
                                    }
                                }
                                "set" => {
                                    if let Some(set_value) = args.get(1) {
                                        if self.expr_is_provably_non_negative(&set_value.node) {
                                            if let Some(upper_bound) =
                                                self.expr_upper_bound_exclusive(&set_value.node)
                                            {
                                                self.list_element_upper_bounds
                                                    .entry(owner_name.to_string())
                                                    .and_modify(|current| {
                                                        *current = (*current).max(upper_bound)
                                                    })
                                                    .or_insert(upper_bound);
                                            } else {
                                                self.list_element_upper_bounds.remove(owner_name);
                                            }
                                        } else {
                                            self.list_element_upper_bounds.remove(owner_name);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        return Ok(result);
                    }
                    let list_val = self.compile_expr(object)?;
                    return self.compile_list_method_on_value(list_val, ty, method, args);
                }
                Type::Map(_, _) => {
                    match object {
                        Expr::Ident(name) if !is_reference_receiver => {
                            return self.compile_map_method(name, method, args);
                        }
                        Expr::Field { object: obj, field } => {
                            let map_ptr = self.compile_field_ptr(&obj.node, field)?;
                            return self.compile_map_method_on_value(
                                map_ptr.into(),
                                ty,
                                method,
                                args,
                            );
                        }
                        _ => {}
                    }
                    let map_val = self.compile_expr(object)?;
                    return self.compile_map_method_on_value(map_val, ty, method, args);
                }
                Type::Set(_) => {
                    match object {
                        Expr::Ident(name) if !is_reference_receiver => {
                            return self.compile_set_method(name, method, args);
                        }
                        Expr::Field { object: obj, field } => {
                            let set_ptr = self.compile_field_ptr(&obj.node, field)?;
                            return self.compile_set_method_on_value(
                                set_ptr.into(),
                                ty,
                                method,
                                args,
                            );
                        }
                        _ => {}
                    }
                    let set_val = self.compile_expr(object)?;
                    return self.compile_set_method_on_value(set_val, ty, method, args);
                }
                Type::Option(_) => {
                    let option_val = self.compile_expr_with_expected_type(object, ty)?;
                    return self.compile_option_method_on_value(option_val, ty, method, args);
                }
                Type::Result(_, _) => {
                    let result_val = self.compile_expr_with_expected_type(object, ty)?;
                    return self.compile_result_method_on_value(result_val, ty, method, args);
                }
                Type::Range(_) => {
                    match object {
                        Expr::Ident(name) if !is_reference_receiver => {
                            return self.compile_range_method(name, method, args);
                        }
                        Expr::Field { object: obj, field } => {
                            let range_ptr_ptr = self.compile_field_ptr(&obj.node, field)?;
                            let range_ptr = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    range_ptr_ptr,
                                    "range_field_ptr",
                                )
                                .map_err(|_| {
                                    CodegenError::new("failed to load range field pointer")
                                })?;
                            return self.compile_range_method_on_value(range_ptr, ty, method, args);
                        }
                        _ => {}
                    }
                    let range_val = if is_reference_receiver {
                        self.compile_deref(object)?
                    } else {
                        self.compile_expr(object)?
                    };
                    return self.compile_range_method_on_value(range_val, ty, method, args);
                }
                Type::Task(inner) => {
                    return self.compile_task_method(object, inner, method, args);
                }
                Type::String => {
                    if method == "length" {
                        if !args.is_empty() {
                            return Err(CodegenError::new(format!(
                                "String.length() expects 0 argument(s), got {}",
                                args.len()
                            )));
                        }
                        let s = if is_reference_receiver {
                            self.compile_deref(object)?
                        } else {
                            self.compile_expr_with_expected_type(object, ty)?
                        };
                        return self.compile_utf8_string_length_runtime(s.into_pointer_value());
                    }
                    return Err(CodegenError::new(format!(
                        "Unknown String method: {}",
                        method
                    )));
                }
                _ => {}
            }
        }

        if let Some(obj_ty) = deref_obj_ty.as_ref() {
            if self.type_to_class_name(obj_ty).is_none() {
                if inferred_obj_ty.is_none() {
                    let _ = self.compile_expr(object)?;
                }
                return Err(CodegenError::new(format!(
                    "Cannot call method on type {}",
                    Self::format_type_string(obj_ty)
                )));
            }
        }

        // Get class name from inferred type
        let class_name = deref_obj_ty
            .as_ref()
            .and_then(|ty| self.type_to_class_name(ty))
            .ok_or_else(|| {
                CodegenError::new(format!(
                    "Cannot determine object type for method call: {:?}",
                    object
                ))
            })?;
        let generic_bound_interfaces = deref_obj_ty
            .as_ref()
            .map(|ty| self.resolved_generic_bound_interfaces(ty))
            .unwrap_or_default();
        let receiver_interfaces = if !generic_bound_interfaces.is_empty() {
            generic_bound_interfaces
        } else if self.interfaces.contains_key(&class_name) {
            vec![class_name.clone()]
        } else {
            Vec::new()
        };

        let obj_val = if matches!(obj_ty, Some(Type::Ref(_)) | Some(Type::MutRef(_))) {
            self.compile_deref(object)?
        } else {
            self.compile_expr(object)?
        };

        let (resolved_func_name, func, func_ty) = if self.classes.contains_key(&class_name) {
            let func_name = self
                .resolve_method_function_name(&class_name, method)
                .ok_or_else(|| {
                    obj_ty.as_ref().map_or_else(
                        || {
                            CodegenError::new(format!(
                                "Unknown method '{}' for class '{}'",
                                method,
                                Self::format_diagnostic_name(&class_name)
                            ))
                        },
                        |ty| Self::unknown_method_error(method, ty),
                    )
                })?;
            let (func, func_ty) = self
                .functions
                .get(&func_name)
                .ok_or_else(|| CodegenError::new(format!("Unknown method: {}", func_name)))?
                .clone();
            (func_name, func, func_ty)
        } else if self.enums.contains_key(&class_name) {
            return Err(obj_ty.as_ref().map_or_else(
                || {
                    CodegenError::new(format!(
                        "Unknown method '{}' for class '{}'",
                        method,
                        Self::format_diagnostic_name(&class_name)
                    ))
                },
                |ty| Self::unknown_method_error(method, ty),
            ));
        } else {
            let implementors = if receiver_interfaces.is_empty() {
                HashSet::new()
            } else {
                self.matching_interface_implementors(&receiver_interfaces, method)?
            };
            // Interface-typed object (or unknown Named type): no vtable yet.
            // We allow codegen only when there is a single unambiguous method implementation.
            let suffix = format!("__{}", method);
            let mut candidates =
                self.functions
                    .iter()
                    .filter_map(|(name, (func, ty))| {
                        let owner = name.strip_suffix(&suffix)?;
                        (!receiver_interfaces.is_empty() && implementors.contains(owner))
                            .then_some((name.clone(), *func, ty.clone()))
                    })
                    .collect::<Vec<_>>();
            if candidates.len() == 1 {
                let (name, func, func_ty) = candidates.pop().ok_or_else(|| {
                    CodegenError::new("interface method candidate disappeared during dispatch")
                })?;
                (name, func, func_ty)
            } else if candidates.is_empty() {
                return Err(CodegenError::new(format!(
                    "Unknown interface method implementation: {}",
                    method
                )));
            } else {
                return Err(CodegenError::new(format!(
                    "Ambiguous interface dispatch for method '{}': {} candidates",
                    method,
                    candidates.len()
                )));
            }
        };

        let mut compiled_args: Vec<BasicValueEnum> = vec![
            self.context
                .ptr_type(AddressSpace::default())
                .const_null()
                .into(), // env_ptr
            obj_val, // this
        ];
        let specialized_func_ty = self.specialize_method_signature_for_receiver(
            deref_obj_ty.as_ref(),
            &class_name,
            &func_ty,
        );
        let arden_param_types = match &specialized_func_ty {
            Type::Function(params, _) => Some(params.as_slice()),
            _ => None,
        };
        if let Some(param_types) = arden_param_types {
            if args.len() != param_types.len() {
                return Err(Self::method_call_arity_error(
                    deref_obj_ty.as_ref().ok_or_else(|| {
                        CodegenError::new("method receiver type missing after class resolution")
                    })?,
                    method,
                    param_types.len(),
                    args.len(),
                ));
            }
            for (index, (arg, param_ty)) in args.iter().zip(param_types.iter()).enumerate() {
                compiled_args.push(self.compile_argument_for_param(
                    &resolved_func_name,
                    index,
                    arg,
                    param_ty,
                )?);
            }
        } else {
            let llvm_param_types = func.get_type().get_param_types();
            for (arg, llvm_param_ty) in args.iter().zip(llvm_param_types.into_iter().skip(2)) {
                compiled_args.push(self.compile_expr_for_llvm_param(&arg.node, llvm_param_ty)?);
            }
        }

        let args_meta: Vec<BasicMetadataValueEnum> =
            compiled_args.iter().map(|a| (*a).into()).collect();
        let call = self
            .builder
            .build_call(func, &args_meta, "call")
            .map_err(|_| CodegenError::new("failed to emit method call"))?;

        match call.try_as_basic_value() {
            ValueKind::Basic(val) => Ok(val),
            ValueKind::Instruction(_) => Ok(self.context.i8_type().const_int(0, false).into()),
        }
    }

    pub fn compile_field(&mut self, object: &Expr, field: &str) -> Result<BasicValueEnum<'ctx>> {
        let nested_field_expr = Expr::Field {
            object: Box::new(Spanned::new(object.clone(), Span::default())),
            field: field.to_string(),
        };
        if let Some(path_parts) = flatten_field_chain(&nested_field_expr) {
            if path_parts.len() >= 2 {
                let owner_source = path_parts[..path_parts.len() - 1].join(".");
                let variant_name = path_parts.last().cloned().unwrap_or_default();
                if let Some(resolved_owner) =
                    self.resolve_alias_qualified_codegen_type_name(&owner_source)
                {
                    if let Some(enum_info) = self.enums.get(&resolved_owner) {
                        if let Some(variant_info) = enum_info.variants.get(&variant_name).cloned() {
                            if variant_info.fields.is_empty() {
                                return self.build_enum_value(&resolved_owner, &variant_info, &[]);
                            }
                        }
                    }
                }
            }
        }
        if let Expr::Ident(owner_name) = object {
            let resolved_owner = self
                .resolve_alias_qualified_codegen_type_name(owner_name)
                .unwrap_or_else(|| self.resolve_module_alias(owner_name));
            if let Some(enum_info) = self.enums.get(&resolved_owner) {
                if let Some(variant_info) = enum_info.variants.get(field).cloned() {
                    if variant_info.fields.is_empty() {
                        return self.build_enum_value(&resolved_owner, &variant_info, &[]);
                    }
                    return Err(CodegenError::new(format!(
                        "Enum variant '{}.{}' requires constructor arguments",
                        resolved_owner, field
                    )));
                }
            }
        }

        if let Some(err) = self.call_expr_arity_error(object) {
            return Err(err);
        }
        if let Expr::Ident(name) = object {
            if !self.variables.contains_key(name) {
                return Err(Self::undefined_variable_error(name));
            }
        }

        // Get class name using type inference
        let inferred_obj_ty = self.infer_object_type(object);
        let obj_ty = inferred_obj_ty
            .clone()
            .or_else(|| Some(self.infer_expr_type(object, &[])));
        if let Some(obj_ty) = obj_ty.as_ref() {
            if self.unwrap_class_like_type(obj_ty).is_none() {
                if inferred_obj_ty.is_none() {
                    let _ = self.compile_expr(object)?;
                }
                return Err(CodegenError::new(format!(
                    "Cannot access field on type {}",
                    Self::format_diagnostic_type(obj_ty)
                )));
            }
        }
        let class_name = obj_ty
            .as_ref()
            .and_then(|ty| self.unwrap_class_like_type(ty).map(|(name, _)| name))
            .ok_or_else(|| {
                CodegenError::new(format!(
                    "Cannot determine object type for field access: {:?}.{}",
                    object, field
                ))
            })?;
        let generic_bound_interfaces = obj_ty
            .as_ref()
            .map(|ty| self.resolved_generic_bound_interfaces(ty))
            .unwrap_or_default();
        let receiver_interfaces = if !generic_bound_interfaces.is_empty() {
            generic_bound_interfaces
        } else if self.interfaces.contains_key(&class_name) {
            vec![class_name.clone()]
        } else {
            Vec::new()
        };

        if self.enums.contains_key(&class_name) {
            return Err(Self::unknown_field_error(
                field,
                obj_ty.as_ref().ok_or_else(|| {
                    CodegenError::new("class-like field receiver type missing after validation")
                })?,
            ));
        }
        let interface_implementors = if receiver_interfaces.is_empty() {
            HashSet::new()
        } else {
            self.matching_interface_implementors(&receiver_interfaces, field)?
        };

        let class_info = self.classes.get(&class_name);
        if class_info.is_none() {
            let suffix = format!("__{}", field);
            let mut candidates = self
                .functions
                .iter()
                .filter_map(|(name, (_, ty))| {
                    let owner = name.strip_suffix(&suffix)?;
                    (!receiver_interfaces.is_empty() && interface_implementors.contains(owner))
                        .then_some((name.clone(), ty.clone()))
                })
                .collect::<Vec<_>>();
            if candidates.len() == 1 {
                let (method_name, func_ty) = candidates.pop().ok_or_else(|| {
                    CodegenError::new("field method candidate disappeared during dispatch")
                })?;
                return self.compile_bound_method_value(
                    object,
                    obj_ty.as_ref(),
                    &method_name,
                    &func_ty,
                );
            }
            if candidates.is_empty() {
                if !receiver_interfaces.is_empty() {
                    return Err(CodegenError::new(format!(
                        "Unknown interface method implementation: {}",
                        field
                    )));
                }
                return Err(CodegenError::new(format!("Unknown class: {}", class_name)));
            }
            return Err(CodegenError::new(format!(
                "Ambiguous interface dispatch for method '{}': {} candidates",
                field,
                candidates.len()
            )));
        }
        let class_info = class_info
            .ok_or_else(|| CodegenError::new(format!("Unknown class metadata: {}", class_name)))?;
        let Some(field_idx) = class_info.field_indices.get(field).copied() else {
            if let Some(method_name) = self.resolve_method_function_name(&class_name, field) {
                let (_, func_ty) =
                    self.functions.get(&method_name).cloned().ok_or_else(|| {
                        CodegenError::new(format!("Unknown method: {}", method_name))
                    })?;
                let arden_func_ty = self.specialize_method_signature_for_receiver(
                    obj_ty.as_ref(),
                    &class_name,
                    &func_ty,
                );
                return self.compile_bound_method_value(
                    object,
                    obj_ty.as_ref(),
                    &method_name,
                    &arden_func_ty,
                );
            }
            return Err(Self::unknown_field_error(
                field,
                obj_ty.as_ref().ok_or_else(|| {
                    CodegenError::new("class-like field receiver type missing after lookup")
                })?,
            ));
        };
        let struct_type = class_info.struct_type;
        let obj_ptr = if matches!(obj_ty, Some(Type::Ref(_)) | Some(Type::MutRef(_))) {
            self.compile_deref(object)?.into_pointer_value()
        } else {
            self.compile_expr(object)?.into_pointer_value()
        };

        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);
        let idx = i32_type.const_int(field_idx as u64, false);

        let field_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(
                    struct_type.as_basic_type_enum(),
                    obj_ptr,
                    &[zero, idx],
                    field,
                )
                .map_err(|_| {
                    CodegenError::new(format!("failed to access field pointer for '{}'", field))
                })?
        };

        let field_type = struct_type
            .get_field_type_at_index(field_idx)
            .ok_or_else(|| {
                CodegenError::new(format!("missing LLVM field type metadata for '{}'", field))
            })?;
        self.builder
            .build_load(field_type, field_ptr, field)
            .map_err(|_| CodegenError::new(format!("failed to load field '{}'", field)))
    }

    fn compile_bound_method_value(
        &mut self,
        object: &Expr,
        object_ty: Option<&Type>,
        method_name: &str,
        arden_func_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let (method_fn, _) = self
            .functions
            .get(method_name)
            .cloned()
            .ok_or_else(|| CodegenError::new(format!("Unknown method: {}", method_name)))?;
        let Type::Function(param_types, ret_type) = arden_func_ty else {
            return Err(CodegenError::new(
                "bound method value requires function type",
            ));
        };

        let receiver_value = if matches!(object_ty, Some(Type::Ref(_)) | Some(Type::MutRef(_))) {
            self.compile_deref(object)?
        } else {
            self.compile_expr(object)?
        };
        let receiver_llvm_ty = receiver_value.get_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let env_struct_ty = self.context.struct_type(&[receiver_llvm_ty], false);
        let env_size = env_struct_ty
            .size_of()
            .ok_or_else(|| CodegenError::new("failed to size bound-method env"))?;
        let env_alloc = self.build_malloc_call(
            env_size,
            "bound_method_env_alloc",
            "failed to allocate bound-method environment",
        )?;
        let env_ptr =
            self.extract_call_pointer_value(env_alloc, "malloc failed for bound-method env")?;
        let receiver_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(
                    env_struct_ty,
                    env_ptr,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context.i32_type().const_int(0, false),
                    ],
                    "bound_method_receiver_ptr",
                )
                .map_err(|_| CodegenError::new("failed to access bound-method receiver slot"))?
        };
        self.builder
            .build_store(receiver_ptr, receiver_value)
            .map_err(|_| CodegenError::new("failed to store bound-method receiver"))?;

        let adapter_name = format!("__bound_method_adapter_{}", self.lambda_counter);
        self.lambda_counter += 1;
        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![ptr_type.into()];
        for (index, param_ty) in param_types.iter().enumerate() {
            llvm_params.push(match self.param_mode_for_function(method_name, index) {
                ParamMode::Owned => self.llvm_type(param_ty).into(),
                ParamMode::Borrow | ParamMode::BorrowMut => ptr_type.into(),
            });
        }
        let adapter_fn_type = match self.llvm_type(ret_type) {
            BasicTypeEnum::IntType(i) => i.fn_type(&llvm_params, false),
            BasicTypeEnum::FloatType(f) => f.fn_type(&llvm_params, false),
            BasicTypeEnum::PointerType(p) => p.fn_type(&llvm_params, false),
            BasicTypeEnum::StructType(s) => s.fn_type(&llvm_params, false),
            _ => self.context.i8_type().fn_type(&llvm_params, false),
        };
        let adapter_fn = self
            .module
            .add_function(&adapter_name, adapter_fn_type, None);

        let saved_function = self.current_function;
        let saved_return_type = self.current_return_type.clone();
        let saved_insert_block = self.builder.get_insert_block();

        self.current_function = Some(adapter_fn);
        self.current_return_type = Some(arden_func_ty.clone());
        let entry = self.context.append_basic_block(adapter_fn, "entry");
        self.builder.position_at_end(entry);

        let adapter_env = adapter_fn
            .get_nth_param(0)
            .ok_or_else(|| CodegenError::new("bound-method env param missing"))?
            .into_pointer_value();
        let stored_receiver_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(
                    env_struct_ty,
                    adapter_env,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context.i32_type().const_int(0, false),
                    ],
                    "bound_method_loaded_receiver_ptr",
                )
                .map_err(|_| {
                    CodegenError::new("failed to access bound-method receiver during load")
                })?
        };
        let loaded_receiver = self
            .builder
            .build_load(
                receiver_llvm_ty,
                stored_receiver_ptr,
                "bound_method_receiver",
            )
            .map_err(|_| CodegenError::new("failed to load bound-method receiver"))?;

        let mut call_args: Vec<BasicMetadataValueEnum> =
            vec![ptr_type.const_null().into(), loaded_receiver.into()];
        for (index, _) in param_types.iter().enumerate() {
            let param = adapter_fn
                .get_nth_param((index + 1) as u32)
                .ok_or_else(|| CodegenError::new("bound-method parameter missing"))?;
            call_args.push(param.into());
        }
        let call = self
            .builder
            .build_call(method_fn, &call_args, "bound_method_call")
            .map_err(|_| CodegenError::new("failed to emit bound-method adapter call"))?;
        match call.try_as_basic_value() {
            ValueKind::Basic(val) => {
                self.builder.build_return(Some(&val)).map_err(|_| {
                    CodegenError::new("failed to return bound-method adapter value")
                })?;
            }
            ValueKind::Instruction(_) => {
                self.builder
                    .build_return(Some(&self.context.i8_type().const_int(0, false)))
                    .map_err(|_| {
                        CodegenError::new("failed to return bound-method adapter placeholder")
                    })?;
            }
        }

        self.current_function = saved_function;
        self.current_return_type = saved_return_type;
        if let Some(block) = saved_insert_block {
            self.builder.position_at_end(block);
        }

        let closure_ty = self.llvm_type(arden_func_ty).into_struct_type();
        let mut closure = closure_ty.get_undef();
        closure = self
            .builder
            .build_insert_value(
                closure,
                adapter_fn.as_global_value().as_pointer_value(),
                0,
                "fn",
            )
            .map_err(|_| {
                CodegenError::new("failed to store bound-method function pointer in closure")
            })?
            .into_struct_value();
        closure = self
            .builder
            .build_insert_value(closure, env_ptr, 1, "env")
            .map_err(|_| CodegenError::new("failed to store bound-method environment in closure"))?
            .into_struct_value();
        Ok(closure.into())
    }

    /// Get pointer to a field (for in-place modifications on collections)
    pub fn compile_field_ptr(&mut self, object: &Expr, field: &str) -> Result<PointerValue<'ctx>> {
        if let Some(err) = self.call_expr_arity_error(object) {
            return Err(err);
        }
        if let Expr::Ident(name) = object {
            if !self.variables.contains_key(name) {
                return Err(Self::undefined_variable_error(name));
            }
        }
        let obj_ty = self
            .infer_object_type(object)
            .or_else(|| Some(self.infer_expr_type(object, &[])));
        if let Some(obj_ty) = obj_ty.as_ref() {
            if self.unwrap_class_like_type(obj_ty).is_none() {
                return Err(CodegenError::new(format!(
                    "Cannot access field on type {}",
                    Self::format_diagnostic_type(obj_ty)
                )));
            }
        }
        let class_name = obj_ty
            .as_ref()
            .and_then(|ty| self.unwrap_class_like_type(ty).map(|(name, _)| name))
            .ok_or_else(|| CodegenError::new("Cannot determine object type for field ptr"))?;

        if self.enums.contains_key(&class_name) {
            return Err(Self::unknown_field_error(
                field,
                obj_ty.as_ref().ok_or_else(|| {
                    CodegenError::new("class-like field receiver type missing after validation")
                })?,
            ));
        }
        let class_info = self
            .classes
            .get(&class_name)
            .ok_or_else(|| CodegenError::new(format!("Unknown class: {}", class_name)))?;

        let field_idx = *class_info.field_indices.get(field).ok_or_else(|| {
            Self::unknown_field_error(
                field,
                obj_ty.as_ref().unwrap_or(&Type::Named(class_name.clone())),
            )
        })?;
        let struct_type = class_info.struct_type;
        let obj_ptr = if matches!(obj_ty, Some(Type::Ref(_)) | Some(Type::MutRef(_))) {
            self.compile_deref(object)?.into_pointer_value()
        } else {
            self.compile_expr(object)?.into_pointer_value()
        };

        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);
        let idx = i32_type.const_int(field_idx as u64, false);

        let field_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(
                    struct_type.as_basic_type_enum(),
                    obj_ptr,
                    &[zero, idx],
                    field,
                )
                .map_err(|_| {
                    CodegenError::new(format!("failed to access field pointer for '{}'", field))
                })?
        };
        if let Some(instruction) = field_ptr.as_instruction() {
            instruction.set_in_bounds_flag(true).map_err(|_| {
                CodegenError::new(format!(
                    "failed to mark field pointer for '{}' as inbounds",
                    field
                ))
            })?;
        }

        Ok(field_ptr)
    }

    pub fn compile_index(&mut self, object: &Expr, index: &Expr) -> Result<BasicValueEnum<'ctx>> {
        if let Some(name) = self.member_root_undefined_variable(object) {
            return Err(Self::undefined_variable_error(&name));
        }
        let inferred_object_ty = self.infer_object_type(object);
        let object_ty = inferred_object_ty
            .clone()
            .or_else(|| Some(self.infer_builtin_argument_type(object)));
        let deref_object_ty = object_ty
            .clone()
            .map(|ty| self.deref_codegen_type(&ty).clone());
        let supports_indexing = matches!(
            deref_object_ty,
            Some(Type::Map(_, _)) | Some(Type::String) | Some(Type::List(_))
        );

        if !supports_indexing {
            if inferred_object_ty.is_none() {
                let _ = self.compile_expr_with_expected_type(
                    object,
                    &self.infer_builtin_argument_type(object),
                )?;
            }
            let diagnostic_ty = deref_object_ty.clone().unwrap_or_else(|| {
                self.deref_codegen_type(&self.infer_builtin_argument_type(object))
                    .clone()
            });
            return Err(CodegenError::new(format!(
                "Cannot index type {}",
                Self::format_diagnostic_type(&diagnostic_ty)
            )));
        }

        let obj_val = self
            .compile_expr_with_expected_type(object, &self.infer_builtin_argument_type(object))?;
        if let Some(Type::Map(_, _)) = &deref_object_ty {
            let index_arg = [Spanned::new(index.clone(), 0..0)];
            if let Some(map_ty) = &deref_object_ty {
                return self.compile_map_method_on_value(obj_val, map_ty, "get", &index_arg);
            }
        }

        let negative_diagnostic = if matches!(deref_object_ty, Some(Type::String)) {
            "String index cannot be negative"
        } else if matches!(deref_object_ty, Some(Type::List(_))) {
            "List index cannot be negative"
        } else {
            "Index cannot be negative"
        };
        let (idx, index_provably_non_negative) =
            self.compile_non_negative_integer_index_expr_with_proof(index, negative_diagnostic)?;

        if matches!(deref_object_ty, Some(Type::String)) {
            if let Expr::Literal(Literal::String(text)) = object {
                let char_values = text.chars().collect::<Vec<_>>();
                if let Some(NumericConst::Integer(index_value)) =
                    TypeChecker::eval_numeric_const_expr(index)
                {
                    if index_value >= 0 && (index_value as usize) >= char_values.len() {
                        return Err(CodegenError::new("String index out of bounds"));
                    }
                    if let Some(ch) = (index_value >= 0)
                        .then_some(index_value as usize)
                        .and_then(|idx| char_values.get(idx).copied())
                    {
                        return Ok(self.context.i32_type().const_int(ch as u64, false).into());
                    }
                }

                let i64_type = self.context.i64_type();
                let length = i64_type.const_int(char_values.len() as u64, false);
                let in_bounds = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        idx,
                        length,
                        "string_literal_index_in_bounds",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to validate string literal index upper bound")
                    })?;
                let current_fn = self.current_function.ok_or_else(|| {
                    CodegenError::new("string literal index used outside function")
                })?;
                let ok_bb = self
                    .context
                    .append_basic_block(current_fn, "string_literal_index_ok");
                let fail_bb = self
                    .context
                    .append_basic_block(current_fn, "string_literal_index_fail");
                if index_provably_non_negative {
                    self.builder
                        .build_conditional_branch(in_bounds, ok_bb, fail_bb)
                        .map_err(|_| {
                            CodegenError::new(
                                "failed to branch for string literal upper-bound check",
                            )
                        })?;
                } else {
                    let non_negative = self
                        .builder
                        .build_int_compare(
                            IntPredicate::SGE,
                            idx,
                            i64_type.const_zero(),
                            "string_literal_index_non_negative",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to validate string literal index lower bound")
                        })?;
                    let valid = self
                        .builder
                        .build_and(non_negative, in_bounds, "string_literal_index_valid")
                        .map_err(|_| {
                            CodegenError::new("failed to combine string literal index bounds")
                        })?;
                    self.builder
                        .build_conditional_branch(valid, ok_bb, fail_bb)
                        .map_err(|_| {
                            CodegenError::new("failed to branch for string literal index bounds")
                        })?;
                }

                self.builder.position_at_end(fail_bb);
                self.emit_runtime_error("String index out of bounds", "string_literal_index_oob")?;

                self.builder.position_at_end(ok_bb);
                let scalar_name = format!("strchars.{}", self.str_counter);
                self.str_counter += 1;
                let scalar_values = if char_values.is_empty() {
                    vec![self.context.i32_type().const_zero()]
                } else {
                    char_values
                        .iter()
                        .map(|ch| self.context.i32_type().const_int(*ch as u64, false))
                        .collect::<Vec<_>>()
                };
                let scalar_array = self.context.i32_type().const_array(&scalar_values);
                let scalar_global =
                    self.module
                        .add_global(scalar_array.get_type(), None, &scalar_name);
                scalar_global.set_linkage(Linkage::Private);
                scalar_global.set_initializer(&scalar_array);
                scalar_global.set_constant(true);
                let zero = self.context.i32_type().const_zero();
                let scalar_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(
                            scalar_array.get_type(),
                            scalar_global.as_pointer_value(),
                            &[zero, zero],
                            "string_literal_scalar_ptr",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to access string literal scalar array")
                        })?
                };
                let scalar_char_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(
                            self.context.i32_type(),
                            scalar_ptr,
                            &[idx],
                            "string_literal_char_ptr",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to access indexed string literal character")
                        })?
                };
                return self
                    .builder
                    .build_load(
                        self.context.i32_type(),
                        scalar_char_ptr,
                        "string_literal_char",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to load indexed string literal character")
                    });
            }

            let string_value = if matches!(object_ty, Some(Type::Ref(_)) | Some(Type::MutRef(_))) {
                self.compile_deref(object)?
            } else {
                obj_val
            };
            return self.compile_utf8_string_index_runtime(string_value.into_pointer_value(), idx);
        }

        if let Some(Type::List(_)) = &deref_object_ty {
            let index_provably_in_bounds = if let Expr::Ident(owner_name) = object {
                self.exact_list_lengths
                    .get(owner_name)
                    .copied()
                    .filter(|length| {
                        index_provably_non_negative
                            && self.expr_is_provably_below_exact_limit(index, *length)
                    })
            } else {
                None
            };
            let i64_type = self.context.i64_type();
            let (length, data_ptr, elem_ty) =
                if let BasicValueEnum::StructValue(list_struct) = obj_val {
                    let length = if index_provably_in_bounds.is_some() {
                        i64_type.const_zero()
                    } else {
                        self.builder
                            .build_extract_value(list_struct, 1, "list_len")
                            .map_err(|_| CodegenError::new("Invalid list value for index access"))?
                            .into_int_value()
                    };
                    let data_ptr = self
                        .builder
                        .build_extract_value(list_struct, 2, "list_data")
                        .map_err(|_| CodegenError::new("Invalid list value for index access"))?
                        .into_pointer_value();
                    let elem_ty = match &deref_object_ty {
                        Some(list_ty @ Type::List(_)) => {
                            self.list_element_layout_from_list_type(list_ty).0
                        }
                        _ => self.list_element_layout_default().0,
                    };
                    (length, data_ptr, elem_ty)
                } else {
                    let list_ptr = obj_val.into_pointer_value();
                    let list_struct_ty = self.context.struct_type(
                        &[
                            i64_type.into(),
                            i64_type.into(),
                            self.context.ptr_type(AddressSpace::default()).into(),
                        ],
                        false,
                    );
                    let i32_type = self.context.i32_type();
                    let zero = i32_type.const_zero();
                    let len_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                        self.builder
                            .build_gep(
                                list_struct_ty.as_basic_type_enum(),
                                list_ptr,
                                &[zero, i32_type.const_int(1, false)],
                                "list_len_ptr",
                            )
                            .map_err(|_| CodegenError::new("failed to access list length field"))?
                    };
                    let data_ptr_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                        self.builder
                            .build_gep(
                                list_struct_ty.as_basic_type_enum(),
                                list_ptr,
                                &[zero, i32_type.const_int(2, false)],
                                "list_data_ptr_ptr",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to access list data pointer field")
                            })?
                    };
                    let length = if index_provably_in_bounds.is_some() {
                        i64_type.const_zero()
                    } else {
                        self.builder
                            .build_load(i64_type, len_ptr, "list_len")
                            .map_err(|_| CodegenError::new("failed to load list length"))?
                            .into_int_value()
                    };
                    let data_ptr = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            data_ptr_ptr,
                            "list_data_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to load list data pointer"))?
                        .into_pointer_value();
                    let elem_ty = match &deref_object_ty {
                        Some(list_ty @ Type::List(_)) => {
                            self.list_element_layout_from_list_type(list_ty).0
                        }
                        _ => self.list_element_layout_default().0,
                    };
                    (length, data_ptr, elem_ty)
                };

            if index_provably_in_bounds.is_none() {
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, idx, length, "index_in_bounds")
                    .map_err(|_| CodegenError::new("failed to validate list index upper bound"))?;
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("list index used outside function"))?;
                let ok_bb = self.context.append_basic_block(current_fn, "index_ok");
                let fail_bb = self.context.append_basic_block(current_fn, "index_fail");
                if index_provably_non_negative {
                    self.builder
                        .build_conditional_branch(in_bounds, ok_bb, fail_bb)
                        .map_err(|_| {
                            CodegenError::new("failed to branch for list index upper-bound check")
                        })?;
                } else {
                    let non_negative = self
                        .builder
                        .build_int_compare(
                            IntPredicate::SGE,
                            idx,
                            i64_type.const_zero(),
                            "index_non_negative",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to validate list index lower bound")
                        })?;
                    let valid = self
                        .builder
                        .build_and(non_negative, in_bounds, "index_valid")
                        .map_err(|_| CodegenError::new("failed to combine list index bounds"))?;
                    self.builder
                        .build_conditional_branch(valid, ok_bb, fail_bb)
                        .map_err(|_| CodegenError::new("failed to branch for list index bounds"))?;
                }

                self.builder.position_at_end(fail_bb);
                self.emit_runtime_error("List index out of bounds", "list_index_oob")?;

                self.builder.position_at_end(ok_bb);
            }
            let elem_ptr = self.build_indexed_element_ptr(data_ptr, elem_ty, idx, "list_index")?;
            return self
                .builder
                .build_load(elem_ty, elem_ptr, "load")
                .map_err(|_| CodegenError::new("failed to load indexed list element"));
        }

        // List indexing may come either as:
        // 1) direct data pointer, or
        // 2) materialized list struct value {capacity, length, data_ptr}.
        if let BasicValueEnum::StructValue(list_struct) = obj_val {
            let data_ptr = self
                .builder
                .build_extract_value(list_struct, 2, "list_data")
                .map_err(|_| CodegenError::new("Invalid list value for index access"))?
                .into_pointer_value();
            let elem_ty = match self.infer_object_type(object) {
                Some(list_ty @ Type::List(_)) => {
                    self.list_element_layout_from_list_type(&list_ty).0
                }
                _ => self.list_element_layout_default().0,
            };
            let typed_data_ptr = self
                .builder
                .build_pointer_cast(
                    data_ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "list_typed_data",
                )
                .map_err(|_| CodegenError::new("failed to cast materialized list data pointer"))?;
            let elem_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                self.builder
                    .build_gep(elem_ty, typed_data_ptr, &[idx], "elem")
                    .map_err(|_| {
                        CodegenError::new("failed to access indexed materialized list element")
                    })?
            };
            return self
                .builder
                .build_load(elem_ty, elem_ptr, "load")
                .map_err(|_| {
                    CodegenError::new("failed to load indexed materialized list element")
                });
        }

        let obj_ptr = obj_val.into_pointer_value();
        let elem_ty = match self.infer_object_type(object) {
            Some(list_ty @ Type::List(_)) => self.list_element_layout_from_list_type(&list_ty).0,
            _ => self.list_element_layout_default().0,
        };
        let elem_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
            self.builder
                .build_gep(elem_ty, obj_ptr, &[idx], "elem")
                .map_err(|_| CodegenError::new("failed to access indexed list element pointer"))?
        };
        self.builder
            .build_load(elem_ty, elem_ptr, "load")
            .map_err(|_| CodegenError::new("failed to load indexed list element"))
    }

    pub fn compile_construct(
        &mut self,
        ty: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let parsed_ty = parse_type_source(ty).ok();
        let normalized_ty = parsed_ty
            .as_ref()
            .map(|parsed| self.normalize_codegen_type(parsed))
            .unwrap_or_else(|| Type::Named(ty.to_string()));

        if let Some((base_name, explicit_type_args)) = Self::parse_construct_nominal_type_source(ty)
        {
            let resolved_builtin = self.resolve_function_alias(&base_name);
            if resolved_builtin != base_name
                && Self::is_supported_builtin_function_name(&resolved_builtin)
            {
                if !explicit_type_args.is_empty() {
                    return Err(CodegenError::new(format!(
                        "Built-in function '{}' does not accept type arguments",
                        resolved_builtin.replace("__", ".")
                    )));
                }
                if let Some(value_ty) =
                    Self::concrete_zero_arg_builtin_value_type(&resolved_builtin)
                {
                    return Err(Self::non_function_call_error(&value_ty));
                }
                if resolved_builtin == "println" || resolved_builtin == "print" {
                    return self.compile_print(args, resolved_builtin == "println");
                }
                if Self::is_stdlib_function(&resolved_builtin) {
                    if let Some(result) = self.compile_stdlib_function(&resolved_builtin, args)? {
                        return Ok(result);
                    }
                    return Ok(self.context.i8_type().const_int(0, false).into());
                }
                match resolved_builtin.as_str() {
                    "Option__some" => {
                        if args.len() != 1 {
                            return Err(CodegenError::new(
                                "Option.some() requires exactly 1 argument",
                            ));
                        }
                        let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                        let value = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                        return self.create_option_some(value);
                    }
                    "Option__none" => {
                        if !args.is_empty() {
                            return Err(CodegenError::new(format!(
                                "Option.none() expects 0 argument(s), got {}",
                                args.len()
                            )));
                        }
                        return self.create_option_none();
                    }
                    "Result__ok" => {
                        if args.len() != 1 {
                            return Err(CodegenError::new(
                                "Result.ok() requires exactly 1 argument",
                            ));
                        }
                        let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                        let value = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                        return self.create_result_ok(value);
                    }
                    "Result__error" => {
                        if args.len() != 1 {
                            return Err(CodegenError::new(
                                "Result.error() requires exactly 1 argument",
                            ));
                        }
                        let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                        let value = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                        return self.create_result_error(value);
                    }
                    _ => {}
                }
            }
            if let Some((enum_name, variant_name)) = self.resolve_import_alias_variant(&base_name) {
                if !explicit_type_args.is_empty() {
                    return Err(CodegenError::new(format!(
                        "Enum variant '{}.{}' does not accept type arguments",
                        Self::format_diagnostic_name(&enum_name),
                        variant_name
                    )));
                }

                let Some(variant_info) = self
                    .enums
                    .get(&enum_name)
                    .and_then(|enum_info| enum_info.variants.get(&variant_name))
                    .cloned()
                else {
                    return Err(CodegenError::new(format!(
                        "Unknown variant '{}' for enum '{}'",
                        variant_name,
                        Self::format_diagnostic_name(&enum_name)
                    )));
                };

                if args.len() != variant_info.fields.len() {
                    return Err(CodegenError::new(format!(
                        "Enum variant '{}.{}' expects {} argument(s), got {}",
                        Self::format_diagnostic_name(&enum_name),
                        variant_name,
                        variant_info.fields.len(),
                        args.len()
                    )));
                }

                let mut values = Vec::with_capacity(args.len());
                for (arg, expected_ty) in args.iter().zip(variant_info.fields.iter()) {
                    values.push(
                        self.compile_expr_for_concrete_class_payload(&arg.node, expected_ty)?,
                    );
                }
                return self.build_enum_value(&enum_name, &variant_info, &values);
            }
        }

        match &normalized_ty {
            Type::List(_) => {
                if args.len() > 1 {
                    return Err(CodegenError::new(format!(
                        "Constructor {} expects 0 or 1 arguments, got {}",
                        Self::format_diagnostic_type(&normalized_ty),
                        args.len()
                    )));
                }
                if args.len() == 1 {
                    if matches!(
                        TypeChecker::eval_numeric_const_expr(&args[0].node),
                        Some(NumericConst::Integer(value)) if value < 0
                    ) {
                        return Err(CodegenError::new(
                            "List constructor capacity cannot be negative",
                        ));
                    }
                    let capacity_ty = self.infer_builtin_argument_type(&args[0].node);
                    let capacity =
                        self.compile_expr_with_expected_type(&args[0].node, &capacity_ty)?;
                    if !capacity.is_int_value() {
                        return Err(CodegenError::new(format!(
                            "Constructor {} expects optional Integer capacity, got {}",
                            Self::format_diagnostic_type(&normalized_ty),
                            Self::format_diagnostic_type(&capacity_ty)
                        )));
                    }
                    return self.create_list_with_capacity_value(
                        capacity.into_int_value(),
                        Some(&normalized_ty),
                    );
                }
                return self.create_empty_list(Some(&normalized_ty));
            }
            Type::Map(_, _) => {
                if !args.is_empty() {
                    return Err(CodegenError::new(format!(
                        "Constructor {} expects 0 arguments, got {}",
                        Self::format_diagnostic_type(&normalized_ty),
                        args.len()
                    )));
                }
                return self.create_empty_map_for_type(&normalized_ty);
            }
            Type::Option(inner) => {
                if !args.is_empty() {
                    return Err(CodegenError::new(format!(
                        "Constructor {} expects 0 arguments, got {}",
                        Self::format_diagnostic_type(&normalized_ty),
                        args.len()
                    )));
                }
                return self.create_option_none_typed(inner);
            }
            Type::Result(ok, err) => {
                if !args.is_empty() {
                    return Err(CodegenError::new(format!(
                        "Constructor {} expects 0 arguments, got {}",
                        Self::format_diagnostic_type(&normalized_ty),
                        args.len()
                    )));
                }
                return self.create_default_result_typed(ok, err);
            }
            Type::Set(_) => {
                if !args.is_empty() {
                    return Err(CodegenError::new(format!(
                        "Constructor {} expects 0 arguments, got {}",
                        Self::format_diagnostic_type(&normalized_ty),
                        args.len()
                    )));
                }
                return self.create_empty_set_for_type(&normalized_ty);
            }
            Type::Box(inner) => {
                if args.len() > 1 {
                    return Err(CodegenError::new(format!(
                        "Constructor {} expects 0 or 1 arguments, got {}",
                        Self::format_diagnostic_type(&normalized_ty),
                        args.len()
                    )));
                }
                if let Some(arg) = args.first() {
                    let payload =
                        self.compile_expr_for_concrete_class_payload(&arg.node, inner.as_ref())?;
                    return self.create_box_typed(payload, &normalized_ty);
                }
                return self.create_empty_box_typed(&normalized_ty);
            }
            Type::Rc(inner) => {
                if args.len() > 1 {
                    return Err(CodegenError::new(format!(
                        "Constructor {} expects 0 or 1 arguments, got {}",
                        Self::format_diagnostic_type(&normalized_ty),
                        args.len()
                    )));
                }
                if let Some(arg) = args.first() {
                    let payload =
                        self.compile_expr_for_concrete_class_payload(&arg.node, inner.as_ref())?;
                    return self.create_rc_typed(payload, &normalized_ty);
                }
                return self.create_empty_rc_typed(&normalized_ty);
            }
            Type::Arc(inner) => {
                if args.len() > 1 {
                    return Err(CodegenError::new(format!(
                        "Constructor {} expects 0 or 1 arguments, got {}",
                        Self::format_diagnostic_type(&normalized_ty),
                        args.len()
                    )));
                }
                if let Some(arg) = args.first() {
                    let payload =
                        self.compile_expr_for_concrete_class_payload(&arg.node, inner.as_ref())?;
                    return self.create_arc_typed(payload, &normalized_ty);
                }
                return self.create_empty_arc_typed(&normalized_ty);
            }
            _ => {}
        }

        let ctor_ty = match &normalized_ty {
            Type::Named(name) => name.clone(),
            Type::Generic(name, _) => name.clone(),
            _ => ty.split('<').next().unwrap_or(ty).to_string(),
        };
        let func_name = format!("{}__new", ctor_ty);

        let (func, func_ty) = self
            .functions
            .get(&func_name)
            .ok_or_else(|| {
                CodegenError::new(format!(
                    "Unknown type: {}",
                    Self::format_diagnostic_type(&normalized_ty)
                ))
            })?
            .clone();

        let mut compiled_args: Vec<BasicValueEnum> = vec![
            self.context
                .ptr_type(AddressSpace::default())
                .const_null()
                .into(), // env_ptr
        ];
        let ctor_params = match func_ty {
            Type::Function(params, _) => params,
            _ => {
                return Err(CodegenError::new(format!(
                    "Constructor metadata for '{}' is not a function type",
                    func_name
                )))
            }
        };
        let ctor_params = self.specialize_constructor_param_types(
            parsed_ty.as_ref(),
            &normalized_ty,
            &ctor_params,
        );
        if args.len() != ctor_params.len() {
            return Err(Self::constructor_call_arity_error(
                &normalized_ty,
                ctor_params.len(),
                args.len(),
            ));
        }
        for (index, (arg, expected_ty)) in args.iter().zip(ctor_params.iter()).enumerate() {
            compiled_args.push(self.compile_argument_for_param(
                &func_name,
                index,
                arg,
                expected_ty,
            )?);
        }

        let args_meta: Vec<BasicMetadataValueEnum> =
            compiled_args.iter().map(|a| (*a).into()).collect();
        let call = self
            .builder
            .build_call(func, &args_meta, "new")
            .map_err(|_| {
                CodegenError::new(format!("failed to emit constructor call '{}'", func_name))
            })?;

        self.extract_call_value_with_context(
            call,
            &format!("Constructor '{}' did not produce a value result", func_name),
        )
    }

    pub fn compile_print(
        &mut self,
        args: &[Spanned<Expr>],
        newline: bool,
    ) -> Result<BasicValueEnum<'ctx>> {
        let printf = self.get_or_declare_printf();

        for arg in args {
            let arg_ty = self.infer_builtin_argument_type(&arg.node);
            if !Self::supports_display_expr(&arg.node, &arg_ty) {
                return Err(CodegenError::new(format!(
                    "println() currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got {}",
                    Self::format_diagnostic_type(&arg_ty)
                )));
            }
            let val = self.compile_expr_with_expected_type(&arg.node, &arg_ty)?;
            let display = self.compile_value_to_display_string(val, &arg_ty)?;
            let fmt = "%s";

            let fmt_str = self.context.const_string(fmt.as_bytes(), true);
            let fmt_name = format!("fmt.{}", self.str_counter);
            self.str_counter += 1;
            let fmt_global = self.module.add_global(fmt_str.get_type(), None, &fmt_name);
            fmt_global.set_linkage(Linkage::Private);
            fmt_global.set_initializer(&fmt_str);

            let call_args: Vec<BasicMetadataValueEnum> =
                vec![fmt_global.as_pointer_value().into(), display.into()];

            self.builder
                .build_call(printf, &call_args, "printf")
                .map_err(|_| CodegenError::new("failed to emit printf for print argument"))?;
        }

        if newline {
            let nl_str = self.context.const_string(b"\n", true);
            let nl_name = format!("nl.{}", self.str_counter);
            self.str_counter += 1;
            let nl_global = self.module.add_global(nl_str.get_type(), None, &nl_name);
            nl_global.set_linkage(Linkage::Private);
            nl_global.set_initializer(&nl_str);
            self.builder
                .build_call(printf, &[nl_global.as_pointer_value().into()], "printf")
                .map_err(|_| CodegenError::new("failed to emit trailing newline printf"))?;
        }

        Ok(self.context.i32_type().const_int(0, false).into())
    }

    pub fn compile_string_interp(&mut self, parts: &[StringPart]) -> Result<BasicValueEnum<'ctx>> {
        // Build format string and collect arguments
        let mut fmt_str = String::new();
        let mut args: Vec<BasicMetadataValueEnum> = Vec::new();
        let i64_type = self.context.i64_type();
        let strlen = self.get_or_declare_strlen();
        let mut rendered_len = i64_type.const_zero();

        for part in parts {
            match part {
                StringPart::Literal(s) => {
                    // Escape % characters for printf
                    fmt_str.push_str(&s.replace('%', "%%"));
                    rendered_len = self
                        .builder
                        .build_int_add(
                            rendered_len,
                            i64_type.const_int(s.len() as u64, false),
                            "interp_literal_len",
                        )
                        .map_err(|_| {
                            CodegenError::new(
                                "failed to accumulate string interpolation literal length",
                            )
                        })?;
                }
                StringPart::Expr(expr) => {
                    let expr_ty = self.infer_builtin_argument_type(&expr.node);
                    if !Self::supports_display_expr(&expr.node, &expr_ty) {
                        return Err(CodegenError::new(format!(
                            "display formatting currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got {}",
                            Self::format_diagnostic_name(&Self::format_type_string(&expr_ty))
                        )));
                    }
                    let val = self.compile_expr_with_expected_type(&expr.node, &expr_ty)?;
                    let display = self.compile_value_to_display_string(val, &expr_ty)?;
                    let display_len_call = self
                        .builder
                        .build_call(strlen, &[display.into()], "interp_display_len")
                        .map_err(|_| {
                            CodegenError::new("failed to emit strlen for interpolation segment")
                        })?;
                    let display_len = self
                        .extract_call_value_with_context(
                            display_len_call,
                            "strlen did not produce a value for string interpolation",
                        )?
                        .into_int_value();
                    rendered_len = self
                        .builder
                        .build_int_add(rendered_len, display_len, "interp_total_expr_len")
                        .map_err(|_| {
                            CodegenError::new(
                                "failed to accumulate string interpolation expression length",
                            )
                        })?;
                    fmt_str.push_str("%s");
                    args.push(display.into());
                }
            }
        }

        // Allocate the exact output size plus the trailing null terminator.
        let snprintf = self.get_or_declare_snprintf();
        let buffer_size = self
            .builder
            .build_int_add(
                rendered_len,
                i64_type.const_int(1, false),
                "interp_buffer_size",
            )
            .map_err(|_| CodegenError::new("failed to compute string interpolation buffer size"))?;
        let buffer_call = self.build_malloc_call(
            buffer_size,
            "strbuf",
            "failed to allocate string interpolation buffer",
        )?;
        let buffer = self.extract_call_pointer_value(
            buffer_call,
            "malloc did not produce a buffer pointer for string interpolation",
        )?;

        // Create format string
        let fmt_val = self.context.const_string(fmt_str.as_bytes(), true);
        let fmt_name = format!("fmt.{}", self.str_counter);
        self.str_counter += 1;
        let fmt_global = self.module.add_global(fmt_val.get_type(), None, &fmt_name);
        fmt_global.set_linkage(Linkage::Private);
        fmt_global.set_initializer(&fmt_val);

        // Call snprintf with the exact output size to avoid heap overwrites on long strings.
        let buffer_size_size_t =
            self.cast_int_to_libc_size_type(buffer_size, "interp_buffer_size_size_t")?;
        let mut snprintf_args: Vec<BasicMetadataValueEnum> = vec![
            buffer.into(),
            buffer_size_size_t.into(),
            fmt_global.as_pointer_value().into(),
        ];
        snprintf_args.extend(args);
        self.builder
            .build_call(snprintf, &snprintf_args, "snprintf")
            .map_err(|_| CodegenError::new("failed to emit snprintf for string interpolation"))?;

        Ok(buffer.into())
    }

    pub fn compile_try(&mut self, inner: &Expr) -> Result<BasicValueEnum<'ctx>> {
        // Get current function and return type
        let function = self
            .current_function
            .ok_or_else(|| CodegenError::new("? operator used outside function"))?;
        let return_type = self
            .current_return_type
            .clone()
            .ok_or_else(|| CodegenError::new("? operator used outside function"))?;

        // Compile the inner expression (should be Option<T> or Result<T, E>)
        let inner_ty = self.infer_builtin_argument_type(inner);
        if !matches!(inner_ty, Type::Option(_) | Type::Result(_, _)) {
            return Err(CodegenError::new(format!(
                "'?' operator can only be used on Option or Result, got {}",
                Self::format_diagnostic_type(&inner_ty)
            )));
        }
        let value = self.compile_expr_with_expected_type(inner, &inner_ty)?;
        let struct_val = value.into_struct_value();

        // Extract the tag (field 0): 0 = None/Error, 1 = Some/Ok
        let tag = self
            .builder
            .build_extract_value(struct_val, 0, "tag")
            .map_err(|_| CodegenError::new("failed to extract try operator tag"))?;
        let tag_int = tag.into_int_value();

        // Compare tag with 1 (Some/Ok)
        let is_some_or_ok = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                tag_int,
                self.context.i8_type().const_int(1, false),
                "is_some_or_ok",
            )
            .map_err(|_| CodegenError::new("failed to compare try operator tag"))?;

        // Create basic blocks
        let success_block = self.context.append_basic_block(function, "try.success");
        let error_block = self.context.append_basic_block(function, "try.error");
        let merge_block = self.context.append_basic_block(function, "try.merge");

        // Branch based on tag
        self.builder
            .build_conditional_branch(is_some_or_ok, success_block, error_block)
            .map_err(|_| CodegenError::new("failed to branch for try operator"))?;

        // Error block: return early with None/Error
        self.builder.position_at_end(error_block);
        match &return_type {
            Type::Option(inner_ty) => {
                // Return None - create Option with tag = 0
                let inner_llvm = self.llvm_type(inner_ty);
                let option_type = self
                    .context
                    .struct_type(&[self.context.i8_type().into(), inner_llvm], false);
                let alloca = self
                    .builder
                    .build_alloca(option_type, "none_ret")
                    .map_err(|_| {
                        CodegenError::new("failed to allocate Option return slot for try operator")
                    })?;
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let tag_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(
                            option_type.as_basic_type_enum(),
                            alloca,
                            &[zero, i32_type.const_int(0, false)],
                            "tag",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to access Option return tag for try operator")
                        })?
                };
                self.builder
                    .build_store(tag_ptr, self.context.i8_type().const_int(0, false))
                    .map_err(|_| {
                        CodegenError::new("failed to store Option return tag for try operator")
                    })?;
                let loaded = self
                    .builder
                    .build_load(option_type, alloca, "ret")
                    .map_err(|_| {
                        CodegenError::new("failed to load Option return value for try operator")
                    })?;
                self.builder.build_return(Some(&loaded)).map_err(|_| {
                    CodegenError::new("failed to return Option value from try operator")
                })?;
            }
            Type::Result(ok_ty, err_ty) => {
                // Return Error - propagate the error from the inner Result
                let ok_llvm = self.llvm_type(ok_ty);
                let err_llvm = self.llvm_type(err_ty);
                let result_type = self
                    .context
                    .struct_type(&[self.context.i8_type().into(), ok_llvm, err_llvm], false);

                // Extract error value from inner and build new Error result
                let err_val = self
                    .builder
                    .build_extract_value(struct_val, 2, "err_val")
                    .map_err(|_| {
                        CodegenError::new("failed to extract Result error value for try operator")
                    })?;
                let alloca = self
                    .builder
                    .build_alloca(result_type, "err_ret")
                    .map_err(|_| {
                        CodegenError::new("failed to allocate Result return slot for try operator")
                    })?;
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let tag_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(
                            result_type.as_basic_type_enum(),
                            alloca,
                            &[zero, i32_type.const_int(0, false)],
                            "tag",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to access Result return tag for try operator")
                        })?
                };
                self.builder
                    .build_store(tag_ptr, self.context.i8_type().const_int(0, false))
                    .map_err(|_| {
                        CodegenError::new("failed to store Result return tag for try operator")
                    })?;
                let err_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(
                            result_type.as_basic_type_enum(),
                            alloca,
                            &[zero, i32_type.const_int(2, false)],
                            "err",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to access Result error slot for try operator")
                        })?
                };
                self.builder.build_store(err_ptr, err_val).map_err(|_| {
                    CodegenError::new("failed to store Result error value for try operator")
                })?;
                let loaded = self
                    .builder
                    .build_load(result_type, alloca, "ret")
                    .map_err(|_| {
                        CodegenError::new("failed to load Result return value for try operator")
                    })?;
                self.builder.build_return(Some(&loaded)).map_err(|_| {
                    CodegenError::new("failed to return Result value from try operator")
                })?;
            }
            _ => {
                return Err(CodegenError::new(
                    "? operator can only be used in functions returning Option or Result",
                ));
            }
        }

        // Success block: extract the value and continue
        self.builder.position_at_end(success_block);
        let extracted = self
            .builder
            .build_extract_value(struct_val, 1, "unwrapped")
            .map_err(|_| CodegenError::new("failed to extract success value for try operator"))?;
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(|_| CodegenError::new("failed to branch from try success block"))?;

        // Merge block: return the extracted value
        self.builder.position_at_end(merge_block);

        Ok(extracted)
    }

    fn ensure_argc_global(&self) -> GlobalValue<'ctx> {
        match self.module.get_global("_arden_argc") {
            Some(global) => global,
            None => {
                let global = self
                    .module
                    .add_global(self.context.i32_type(), None, "_arden_argc");
                global.set_initializer(&self.context.i32_type().const_int(0, false));
                global
            }
        }
    }

    fn ensure_argv_global(&self) -> GlobalValue<'ctx> {
        match self.module.get_global("_arden_argv") {
            Some(global) => global,
            None => {
                let argv_ty = self.context.ptr_type(AddressSpace::default());
                let global = self.module.add_global(argv_ty, None, "_arden_argv");
                global.set_initializer(&argv_ty.const_null());
                global
            }
        }
    }
}
