//! Arden Code Generator - LLVM IR generation

#![allow(dead_code)]

use crate::cache::elapsed_nanos_u64;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};

use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, FunctionValue, GlobalValue, IntValue, PointerValue,
    ValueKind,
};
use inkwell::{AddressSpace, AtomicOrdering, FloatPredicate, IntPredicate};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use crate::ast::*;
use crate::parser::parse_type_source;
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

/// Code generator
pub struct Codegen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub variables: HashMap<String, Variable<'ctx>>,
    pub functions: HashMap<String, (FunctionValue<'ctx>, Type)>,
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
            _ => unreachable!(),
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
            Expr::IfExpr {
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

    #[allow(clippy::too_many_arguments)]
    fn rewrite_stmt_generic_calls(
        stmt: &Stmt,
        function_templates: &HashMap<String, GenericTemplate>,
        method_templates: &HashMap<String, Vec<GenericTemplate>>,
        class_templates: &HashMap<String, GenericClassTemplate>,
        import_aliases: &HashMap<String, String>,
        emitted: &mut HashSet<String>,
        generated_functions: &mut Vec<Spanned<Decl>>,
        generated_methods: &mut HashMap<String, Vec<FunctionDecl>>,
    ) -> Result<Stmt> {
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
                    Self::rewrite_expr_generic_calls(
                        &value.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    value.span.clone(),
                ),
                mutable: *mutable,
            },
            Stmt::Assign { target, value } => Stmt::Assign {
                target: Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &target.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    target.span.clone(),
                ),
                value: Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &value.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    value.span.clone(),
                ),
            },
            Stmt::Expr(expr) => Stmt::Expr(Spanned::new(
                Self::rewrite_expr_generic_calls(
                    &expr.node,
                    function_templates,
                    method_templates,
                    class_templates,
                    import_aliases,
                    emitted,
                    generated_functions,
                    generated_methods,
                )?,
                expr.span.clone(),
            )),
            Stmt::Return(expr) => Stmt::Return(
                expr.as_ref()
                    .map(|e| {
                        Ok(Spanned::new(
                            Self::rewrite_expr_generic_calls(
                                &e.node,
                                function_templates,
                                method_templates,
                                class_templates,
                                import_aliases,
                                emitted,
                                generated_functions,
                                generated_methods,
                            )?,
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
                    Self::rewrite_expr_generic_calls(
                        &condition.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    condition.span.clone(),
                ),
                then_block: then_block
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(
                                &s.node,
                                function_templates,
                                method_templates,
                                class_templates,
                                import_aliases,
                                emitted,
                                generated_functions,
                                generated_methods,
                            )?,
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
                                    Self::rewrite_stmt_generic_calls(
                                        &s.node,
                                        function_templates,
                                        method_templates,
                                        class_templates,
                                        import_aliases,
                                        emitted,
                                        generated_functions,
                                        generated_methods,
                                    )?,
                                    s.span.clone(),
                                ))
                            })
                            .collect::<Result<Vec<_>>>()
                    })
                    .transpose()?,
            },
            Stmt::While { condition, body } => Stmt::While {
                condition: Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &condition.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    condition.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(
                                &s.node,
                                function_templates,
                                method_templates,
                                class_templates,
                                import_aliases,
                                emitted,
                                generated_functions,
                                generated_methods,
                            )?,
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
                    Self::rewrite_expr_generic_calls(
                        &iterable.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    iterable.span.clone(),
                ),
                body: body
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(
                                &s.node,
                                function_templates,
                                method_templates,
                                class_templates,
                                import_aliases,
                                emitted,
                                generated_functions,
                                generated_methods,
                            )?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
            Stmt::Match { expr, arms } => Stmt::Match {
                expr: Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &expr.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
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
                                            &s.node,
                                            function_templates,
                                            method_templates,
                                            class_templates,
                                            import_aliases,
                                            emitted,
                                            generated_functions,
                                            generated_methods,
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
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => Expr::IfExpr {
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
                        mutable: param.mutable,
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
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => Expr::IfExpr {
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
            Expr::IfExpr {
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

    #[allow(clippy::too_many_arguments)]
    fn rewrite_expr_generic_calls(
        expr: &Expr,
        function_templates: &HashMap<String, GenericTemplate>,
        method_templates: &HashMap<String, Vec<GenericTemplate>>,
        class_templates: &HashMap<String, GenericClassTemplate>,
        import_aliases: &HashMap<String, String>,
        emitted: &mut HashSet<String>,
        generated_functions: &mut Vec<Spanned<Decl>>,
        generated_methods: &mut HashMap<String, Vec<FunctionDecl>>,
    ) -> Result<Expr> {
        Ok(match expr {
            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                let rewritten_callee = Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &callee.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    callee.span.clone(),
                );
                let rewritten_args = args
                    .iter()
                    .map(|arg| {
                        Ok(Spanned::new(
                            Self::rewrite_expr_generic_calls(
                                &arg.node,
                                function_templates,
                                method_templates,
                                class_templates,
                                import_aliases,
                                emitted,
                                generated_functions,
                                generated_methods,
                            )?,
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
                                    if !emitted.insert(emitted_key) {
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
                                                    &s.node,
                                                    function_templates,
                                                    method_templates,
                                                    class_templates,
                                                    import_aliases,
                                                    emitted,
                                                    generated_functions,
                                                    generated_methods,
                                                )?,
                                                s.span.clone(),
                                            ))
                                        })
                                        .collect::<Result<Vec<_>>>()?;
                                    spec_func.body = rewritten_body;
                                    if template.owner_class.is_some() {
                                        generated_methods
                                            .entry(owner_key.clone())
                                            .or_default()
                                            .push(spec_func.clone());
                                        if owner_key != default_owner {
                                            generated_methods
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

                            if emitted.insert(spec_name.clone()) {
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
                                                &s.node,
                                                function_templates,
                                                method_templates,
                                                class_templates,
                                                import_aliases,
                                                emitted,
                                                generated_functions,
                                                generated_methods,
                                            )?,
                                            s.span.clone(),
                                        ))
                                    })
                                    .collect::<Result<Vec<_>>>()?;
                                spec_func.body = rewritten_body;
                                generated_functions.push(Spanned::new(
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
                    Self::rewrite_expr_generic_calls(
                        &callee.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
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

                            if emitted.insert(spec_name.clone()) {
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
                                                &s.node,
                                                function_templates,
                                                method_templates,
                                                class_templates,
                                                import_aliases,
                                                emitted,
                                                generated_functions,
                                                generated_methods,
                                            )?,
                                            s.span.clone(),
                                        ))
                                    })
                                    .collect::<Result<Vec<_>>>()?;
                                spec_func.body = rewritten_body;
                                generated_functions.push(Spanned::new(
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
                    Self::rewrite_expr_generic_calls(
                        &left.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    left.span.clone(),
                )),
                right: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &right.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    right.span.clone(),
                )),
            },
            Expr::Unary { op, expr } => Expr::Unary {
                op: *op,
                expr: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &expr.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    expr.span.clone(),
                )),
            },
            Expr::Field { object, field } => Expr::Field {
                object: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &object.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    object.span.clone(),
                )),
                field: field.clone(),
            },
            Expr::Index { object, index } => Expr::Index {
                object: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &object.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    object.span.clone(),
                )),
                index: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &index.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    index.span.clone(),
                )),
            },
            Expr::Lambda { params, body } => Expr::Lambda {
                params: params.clone(),
                body: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &body.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    body.span.clone(),
                )),
            },
            Expr::Match { expr, arms } => Expr::Match {
                expr: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &expr.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
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
                                            &s.node,
                                            function_templates,
                                            method_templates,
                                            class_templates,
                                            import_aliases,
                                            emitted,
                                            generated_functions,
                                            generated_methods,
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
                            Self::rewrite_expr_generic_calls(
                                &e.node,
                                function_templates,
                                method_templates,
                                class_templates,
                                import_aliases,
                                emitted,
                                generated_functions,
                                generated_methods,
                            )?,
                            e.span.clone(),
                        ))),
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            Expr::Try(inner) => Expr::Try(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(
                    &inner.node,
                    function_templates,
                    method_templates,
                    class_templates,
                    import_aliases,
                    emitted,
                    generated_functions,
                    generated_methods,
                )?,
                inner.span.clone(),
            ))),
            Expr::Borrow(inner) => Expr::Borrow(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(
                    &inner.node,
                    function_templates,
                    method_templates,
                    class_templates,
                    import_aliases,
                    emitted,
                    generated_functions,
                    generated_methods,
                )?,
                inner.span.clone(),
            ))),
            Expr::MutBorrow(inner) => Expr::MutBorrow(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(
                    &inner.node,
                    function_templates,
                    method_templates,
                    class_templates,
                    import_aliases,
                    emitted,
                    generated_functions,
                    generated_methods,
                )?,
                inner.span.clone(),
            ))),
            Expr::Deref(inner) => Expr::Deref(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(
                    &inner.node,
                    function_templates,
                    method_templates,
                    class_templates,
                    import_aliases,
                    emitted,
                    generated_functions,
                    generated_methods,
                )?,
                inner.span.clone(),
            ))),
            Expr::Await(inner) => Expr::Await(Box::new(Spanned::new(
                Self::rewrite_expr_generic_calls(
                    &inner.node,
                    function_templates,
                    method_templates,
                    class_templates,
                    import_aliases,
                    emitted,
                    generated_functions,
                    generated_methods,
                )?,
                inner.span.clone(),
            ))),
            Expr::AsyncBlock(block) => Expr::AsyncBlock(
                block
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(
                                &s.node,
                                function_templates,
                                method_templates,
                                class_templates,
                                import_aliases,
                                emitted,
                                generated_functions,
                                generated_methods,
                            )?,
                            s.span.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            Expr::Require { condition, message } => Expr::Require {
                condition: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &condition.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    condition.span.clone(),
                )),
                message: message
                    .as_ref()
                    .map(|m| {
                        Ok(Box::new(Spanned::new(
                            Self::rewrite_expr_generic_calls(
                                &m.node,
                                function_templates,
                                method_templates,
                                class_templates,
                                import_aliases,
                                emitted,
                                generated_functions,
                                generated_methods,
                            )?,
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
                            Self::rewrite_expr_generic_calls(
                                &s.node,
                                function_templates,
                                method_templates,
                                class_templates,
                                import_aliases,
                                emitted,
                                generated_functions,
                                generated_methods,
                            )?,
                            s.span.clone(),
                        )))
                    })
                    .transpose()?,
                end: end
                    .as_ref()
                    .map(|e| {
                        Ok(Box::new(Spanned::new(
                            Self::rewrite_expr_generic_calls(
                                &e.node,
                                function_templates,
                                method_templates,
                                class_templates,
                                import_aliases,
                                emitted,
                                generated_functions,
                                generated_methods,
                            )?,
                            e.span.clone(),
                        )))
                    })
                    .transpose()?,
                inclusive: *inclusive,
            },
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => Expr::IfExpr {
                condition: Box::new(Spanned::new(
                    Self::rewrite_expr_generic_calls(
                        &condition.node,
                        function_templates,
                        method_templates,
                        class_templates,
                        import_aliases,
                        emitted,
                        generated_functions,
                        generated_methods,
                    )?,
                    condition.span.clone(),
                )),
                then_branch: then_branch
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(
                                &s.node,
                                function_templates,
                                method_templates,
                                class_templates,
                                import_aliases,
                                emitted,
                                generated_functions,
                                generated_methods,
                            )?,
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
                                    Self::rewrite_stmt_generic_calls(
                                        &s.node,
                                        function_templates,
                                        method_templates,
                                        class_templates,
                                        import_aliases,
                                        emitted,
                                        generated_functions,
                                        generated_methods,
                                    )?,
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
                            Self::rewrite_stmt_generic_calls(
                                &s.node,
                                function_templates,
                                method_templates,
                                class_templates,
                                import_aliases,
                                emitted,
                                generated_functions,
                                generated_methods,
                            )?,
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
                        mutable: param.mutable,
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
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => Expr::IfExpr {
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
                            mutable: param.mutable,
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
                        mutable: param.mutable,
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
                                mutable: param.mutable,
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
                                    mutable: param.mutable,
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
                            mutable: param.mutable,
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
                                mutable: param.mutable,
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
                            mutable: param.mutable,
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
                                mutable: param.mutable,
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

    #[allow(clippy::too_many_arguments)]
    fn rewrite_decl_generic_calls(
        decl: &Spanned<Decl>,
        function_templates: &HashMap<String, GenericTemplate>,
        method_templates: &HashMap<String, Vec<GenericTemplate>>,
        class_templates: &HashMap<String, GenericClassTemplate>,
        import_aliases: &HashMap<String, String>,
        emitted: &mut HashSet<String>,
        generated_functions: &mut Vec<Spanned<Decl>>,
        generated_methods: &mut HashMap<String, Vec<FunctionDecl>>,
    ) -> Result<Spanned<Decl>> {
        Ok(match &decl.node {
            Decl::Function(func) => {
                let mut f = func.clone();
                f.body = f
                    .body
                    .iter()
                    .map(|s| {
                        Ok(Spanned::new(
                            Self::rewrite_stmt_generic_calls(
                                &s.node,
                                function_templates,
                                method_templates,
                                class_templates,
                                import_aliases,
                                emitted,
                                generated_functions,
                                generated_methods,
                            )?,
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
                    .map(|inner| {
                        Self::rewrite_decl_generic_calls(
                            inner,
                            function_templates,
                            method_templates,
                            class_templates,
                            import_aliases,
                            emitted,
                            generated_functions,
                            generated_methods,
                        )
                    })
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
                                Self::rewrite_stmt_generic_calls(
                                    &s.node,
                                    function_templates,
                                    method_templates,
                                    class_templates,
                                    import_aliases,
                                    emitted,
                                    generated_functions,
                                    generated_methods,
                                )?,
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
                                Self::rewrite_stmt_generic_calls(
                                    &s.node,
                                    function_templates,
                                    method_templates,
                                    class_templates,
                                    import_aliases,
                                    emitted,
                                    generated_functions,
                                    generated_methods,
                                )?,
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
                                    Self::rewrite_stmt_generic_calls(
                                        &s.node,
                                        function_templates,
                                        method_templates,
                                        class_templates,
                                        import_aliases,
                                        emitted,
                                        generated_functions,
                                        generated_methods,
                                    )?,
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
            Expr::IfExpr {
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
        let rewritten = program
            .declarations
            .iter()
            .map(|decl| {
                Self::rewrite_decl_generic_calls(
                    decl,
                    &function_templates,
                    &method_templates,
                    &class_templates,
                    &import_aliases,
                    &mut emitted_specs,
                    &mut generated_functions,
                    &mut generated_methods,
                )
            })
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
            functions: HashMap::new(),
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

    /// Compile program while only emitting bodies for selected top-level symbols.
    /// Declarations are still emitted for the whole program to keep cross-file references valid.
    pub fn compile_filtered(
        &mut self,
        program: &Program,
        active_symbols: &HashSet<String>,
    ) -> Result<()> {
        self.compile_internal(program, Some(active_symbols), None)
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

        // First pass (0): declare all enums first so Named(Enum) resolves correctly.
        let enum_declare_pass_started_at = Instant::now();
        let mut enum_declare_decl_filter_ns = 0_u64;
        let mut enum_declare_work_ns = 0_u64;
        let mut declared_enum_count = 0_usize;
        for decl in &program.declarations {
            let decl_filter_started_at = Instant::now();
            let should_declare = specialized_declaration_symbols
                .as_ref()
                .map(|symbols| self.should_compile_decl(&decl.node, symbols))
                .unwrap_or(true);
            enum_declare_decl_filter_ns += elapsed_nanos_u64(decl_filter_started_at);
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
        let mut decl_pass_decl_filter_ns = 0_u64;
        let mut decl_pass_class_work_ns = 0_u64;
        let mut decl_pass_function_work_ns = 0_u64;
        let mut decl_pass_module_work_ns = 0_u64;
        let mut declared_class_count = 0_usize;
        let mut declared_function_count = 0_usize;
        let mut declared_module_count = 0_usize;
        let mut pending_classes = Vec::new();
        for decl in &program.declarations {
            if let Decl::Class(class) = &decl.node {
                pending_classes.push(class.clone());
                continue;
            }
            let decl_filter_started_at = Instant::now();
            let should_declare = specialized_declaration_symbols
                .as_ref()
                .map(|symbols| self.should_compile_decl(&decl.node, symbols))
                .unwrap_or(true);
            decl_pass_decl_filter_ns += elapsed_nanos_u64(decl_filter_started_at);
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
        for decl in &program.declarations {
            let decl_filter_started_at = Instant::now();
            let should_declare = specialized_declaration_symbols
                .as_ref()
                .map(|symbols| self.should_compile_decl(&decl.node, symbols))
                .unwrap_or(true);
            decl_pass_decl_filter_ns += elapsed_nanos_u64(decl_filter_started_at);
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
        let mut body_pass_decl_filter_ns = 0_u64;
        let mut body_pass_function_work_ns = 0_u64;
        let mut body_pass_class_work_ns = 0_u64;
        let mut body_pass_module_work_ns = 0_u64;
        let mut compiled_function_count = 0_usize;
        let mut compiled_class_count = 0_usize;
        let mut compiled_module_count = 0_usize;
        for decl in &program.declarations {
            let decl_filter_started_at = Instant::now();
            let should_compile = specialized_active_symbols
                .as_ref()
                .map(|symbols| self.should_emit_decl_body(&decl.node, symbols))
                .unwrap_or(true);
            body_pass_decl_filter_ns += elapsed_nanos_u64(decl_filter_started_at);
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
        for decl in &program.declarations {
            let decl_filter_started_at = Instant::now();
            let should_compile = specialized_active_symbols
                .as_ref()
                .map(|symbols| self.should_emit_decl_body(&decl.node, symbols))
                .unwrap_or(true);
            body_pass_decl_filter_ns += elapsed_nanos_u64(decl_filter_started_at);
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

    fn split_generic_args_static(s: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut angle_depth = 0usize;
        let mut paren_depth = 0usize;

        for ch in s.chars() {
            match ch {
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
                ',' if angle_depth == 0 && paren_depth == 0 => {
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        parts.push(trimmed.to_string());
                    }
                    current.clear();
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
            let args = Self::split_generic_args_static(&trimmed[start + 1..end])
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

    fn nominal_function_value_type_source(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Ident(name) => Some(name.clone()),
            Expr::Field { .. } => Some(flatten_field_chain(expr)?.join(".")),
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

    fn is_contextual_static_container_function_value(name: &str) -> bool {
        matches!(
            name,
            "Option__some" | "Option__none" | "Result__ok" | "Result__error"
        )
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

    pub fn declare_module(&mut self, module: &ModuleDecl) -> Result<()> {
        self.declare_module_functions_with_prefix(module, &module.name)
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
            .map(|(name, _)| self.llvm_type(field_types.get(name).expect("field type must exist")))
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
        let param_types: Vec<BasicMetadataTypeEnum> = normalized_ctor_params
            .iter()
            .map(|ty| self.llvm_type(ty).into())
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
                    normalized_ctor_params,
                    Box::new(self.normalize_codegen_type(&Type::Named(class.name.clone()))),
                ),
            ),
        );

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
        for param_ty in &normalized_params {
            llvm_params.push(self.llvm_type(param_ty).into());
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
                Type::Function(normalized_params, Box::new(normalized_return)),
            ),
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
            let alloca = self
                .builder
                .build_alloca(self.llvm_type(&normalized_param_ty), &param.name)
                .map_err(|e| {
                    CodegenError::new(format!("alloca failed for '{}': {}", param.name, e))
                })?;
            self.builder.build_store(alloca, llvm_param).map_err(|e| {
                CodegenError::new(format!("store failed for '{}': {}", param.name, e))
            })?;
            self.variables.insert(
                param.name.clone(),
                Variable {
                    ptr: alloca,
                    ty: normalized_param_ty,
                    mutable: param.mutable,
                },
            );
        }

        // Allocate instance
        let class_info = self
            .classes
            .get(&class.name)
            .ok_or_else(|| CodegenError::new(format!("Unknown class: {}", class.name)))?;
        let struct_type = class_info.struct_type;
        let malloc = self.get_or_declare_malloc();
        let size = struct_type
            .size_of()
            .ok_or_else(|| CodegenError::new("Failed to compute class struct size"))?;
        let ptr = self
            .builder
            .build_call(malloc, &[size.into()], "instance")
            .map_err(|e| CodegenError::new(format!("malloc call failed: {}", e)))?;
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
            let alloca = self
                .builder
                .build_alloca(self.llvm_type(&normalized_param_ty), &param.name)
                .map_err(|e| {
                    CodegenError::new(format!("alloca failed for '{}': {}", param.name, e))
                })?;
            self.builder.build_store(alloca, llvm_param).map_err(|e| {
                CodegenError::new(format!("store failed for '{}': {}", param.name, e))
            })?;
            self.variables.insert(
                param.name.clone(),
                Variable {
                    ptr: alloca,
                    ty: normalized_param_ty,
                    mutable: param.mutable,
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
        let normalized_return = self.normalize_codegen_type(&func.return_type);

        let param_types: Vec<BasicMetadataTypeEnum> = normalized_params
            .iter()
            .map(|ty| self.llvm_type(ty).into())
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

        // Function will return (no infinite loops in analyzed functions)
        let will_return = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("willreturn"), 0);
        function.add_attribute(AttributeLoc::Function, will_return);

        self.functions.insert(
            func.name.clone(),
            (
                function,
                Type::Function(normalized_params, Box::new(normalized_return)),
            ),
        );
        Ok(function)
    }

    fn declare_extern_function(&mut self, func: &FunctionDecl) -> Result<FunctionValue<'ctx>> {
        let param_types: Vec<BasicMetadataTypeEnum> = func
            .params
            .iter()
            .map(|p| self.llvm_type(&p.ty).into())
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
                    func.params.iter().map(|p| p.ty.clone()).collect(),
                    Box::new(func.return_type.clone()),
                ),
            ),
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

        let mut wrapper_params: Vec<BasicMetadataTypeEnum> = vec![ptr_type.into()];
        let mut env_fields: Vec<BasicTypeEnum<'ctx>> = Vec::new();
        for param in &func.params {
            let llvm = self.llvm_type(&param.ty);
            wrapper_params.push(llvm.into());
            env_fields.push(llvm);
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
                    func.params.iter().map(|p| p.ty.clone()).collect(),
                    Box::new(task_return),
                ),
            ),
        );

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
        let malloc = self.get_or_declare_malloc();
        let size = task_ty
            .size_of()
            .ok_or_else(|| CodegenError::new("failed to compute Task runtime size"))?;

        let raw = self
            .builder
            .build_call(malloc, &[size.into()], "task_alloc")
            .map_err(|e| CodegenError::new(format!("failed to call malloc for Task: {e}")))?;
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

        let thread_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, thread_idx], "task_thread_field")
                .map_err(|e| CodegenError::new(format!("failed to get Task thread field: {e}")))?
        };
        self.builder
            .build_store(thread_field, self.context.i64_type().const_int(0, false))
            .map_err(|e| {
                CodegenError::new(format!("failed to initialize Task thread field: {e}"))
            })?;

        let result_field = unsafe {
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

        let done_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done")
                .map_err(|e| CodegenError::new(format!("failed to get Task done field: {e}")))?
        };
        self.builder
            .build_store(done_field, self.context.i8_type().const_int(0, false))
            .map_err(|e| CodegenError::new(format!("failed to initialize Task done field: {e}")))?;
        let completed_field = unsafe {
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
                let thread_tmp = self
                    .builder
                    .build_alloca(self.context.i64_type(), "task_thread_tmp")
                    .map_err(|e| {
                        CodegenError::new(format!("failed to allocate task thread temp: {e}"))
                    })?;
                self.builder
                    .build_store(thread_tmp, self.context.i64_type().const_int(0, false))
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

                self.builder
                    .build_load(self.context.i64_type(), thread_tmp, "task_thread")
                    .map_err(|e| {
                        CodegenError::new(format!("failed to load pthread task handle: {e}"))
                    })?
                    .into_int_value()
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
            .unwrap();

        let i32_ty = self.context.i32_type();
        let zero = i32_ty.const_int(0, false);
        let thread_idx = i32_ty.const_int(0, false);
        let result_idx = i32_ty.const_int(1, false);
        let done_idx = i32_ty.const_int(2, false);

        let done_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done_ptr")
                .unwrap()
        };
        let done_val = self
            .builder
            .build_load(self.context.i8_type(), done_field, "task_done")
            .unwrap()
            .into_int_value();
        let done_ready = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                done_val,
                self.context.i8_type().const_zero(),
                "task_done_ready",
            )
            .unwrap();

        let result_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, result_idx], "task_result_field")
                .unwrap()
        };
        let existing_result = self
            .builder
            .build_load(ptr_ty, result_field, "task_result_existing")
            .unwrap()
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
            .unwrap();

        self.builder.position_at_end(join_bb);
        let thread_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, thread_idx], "task_thread_ptr")
                .unwrap()
        };
        let thread_id = self
            .builder
            .build_load(self.context.i64_type(), thread_field, "task_thread_id")
            .unwrap()
            .into_int_value();
        #[cfg(windows)]
        let new_result = {
            let wait_fn = self.get_or_declare_wait_for_single_object_win();
            let close_fn = self.get_or_declare_close_handle_win();
            let handle = self
                .builder
                .build_int_to_ptr(thread_id, ptr_ty, "task_thread_handle")
                .unwrap();
            self.builder
                .build_call(
                    wait_fn,
                    &[
                        handle.into(),
                        self.context.i32_type().const_all_ones().into(),
                    ],
                    "task_join_call",
                )
                .unwrap();
            self.builder
                .build_call(close_fn, &[handle.into()], "")
                .unwrap();
            self.builder
                .build_store(thread_field, self.context.i64_type().const_zero())
                .unwrap();
            self.builder
                .build_load(ptr_ty, result_field, "task_joined_result")
                .unwrap()
                .into_pointer_value()
        };
        #[cfg(not(windows))]
        let new_result = {
            let pthread_join = self.get_or_declare_pthread_join();
            let join_result_ptr = self
                .builder
                .build_alloca(ptr_ty, "task_join_result")
                .unwrap();
            self.builder
                .build_store(join_result_ptr, ptr_ty.const_null())
                .unwrap();
            self.builder
                .build_call(
                    pthread_join,
                    &[thread_id.into(), join_result_ptr.into()],
                    "task_join_call",
                )
                .unwrap();
            self.builder
                .build_load(ptr_ty, join_result_ptr, "task_joined_result")
                .unwrap()
                .into_pointer_value()
        };
        self.builder.build_store(result_field, new_result).unwrap();
        self.builder
            .build_store(done_field, self.context.i8_type().const_int(1, false))
            .unwrap();
        self.builder.build_unconditional_branch(cont_bb).unwrap();

        self.builder.position_at_end(cont_bb);
        let phi = self.builder.build_phi(ptr_ty, "task_result_phi").unwrap();
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
            .unwrap();
        Ok(self
            .builder
            .build_load(result_ty, typed_ptr, "task_result")
            .unwrap())
    }

    fn compile_async_function(&mut self, func: &FunctionDecl) -> Result<()> {
        let plan = self
            .async_functions
            .get(&func.name)
            .ok_or_else(|| CodegenError::new(format!("Missing async plan for {}", func.name)))?;
        let wrapper = plan.wrapper;
        let body = plan.body;
        let thunk = plan.thunk;
        let env_type = plan.env_type;
        let inner_return_type = plan.inner_return_type.clone();
        let ptr_type = self.context.ptr_type(AddressSpace::default());

        // 1) Compile body function: __arden_async_body__*
        self.current_function = Some(body);
        self.current_return_type = Some(inner_return_type.clone());
        self.variables.clear();
        self.loop_stack.clear();
        self.reset_current_generic_bounds();
        self.extend_current_generic_bounds(&func.generic_params);
        let body_entry = self.context.append_basic_block(body, "entry");
        self.builder.position_at_end(body_entry);

        let env_raw = body
            .get_nth_param(0)
            .ok_or_else(|| CodegenError::new("Async body missing env parameter"))?
            .into_pointer_value();
        let env_ptr = self
            .builder
            .build_pointer_cast(
                env_raw,
                self.context.ptr_type(AddressSpace::default()),
                "async_env_cast",
            )
            .unwrap();

        for (i, param) in func.params.iter().enumerate() {
            let field_ptr = unsafe {
                self.builder
                    .build_gep(
                        env_type,
                        env_ptr,
                        &[
                            self.context.i32_type().const_int(0, false),
                            self.context.i32_type().const_int(i as u64, false),
                        ],
                        "async_param_field",
                    )
                    .unwrap()
            };
            let loaded = self
                .builder
                .build_load(self.llvm_type(&param.ty), field_ptr, &param.name)
                .unwrap();
            let alloca = self
                .builder
                .build_alloca(self.llvm_type(&param.ty), &param.name)
                .unwrap();
            self.builder.build_store(alloca, loaded).unwrap();
            self.variables.insert(
                param.name.clone(),
                Variable {
                    ptr: alloca,
                    ty: param.ty.clone(),
                    mutable: param.mutable,
                },
            );
        }

        for stmt in &func.body {
            self.compile_stmt(&stmt.node)?;
        }
        if self.needs_terminator() {
            if matches!(inner_return_type, Type::None) {
                self.builder.build_return(None).unwrap();
            } else {
                self.builder.build_unreachable().unwrap();
            }
        }

        // 2) Compile thunk: __arden_async_thunk__*
        self.current_function = Some(thunk);
        self.current_return_type = None;
        self.variables.clear();
        self.loop_stack.clear();
        self.reset_current_generic_bounds();
        let thunk_entry = self.context.append_basic_block(thunk, "entry");
        self.builder.position_at_end(thunk_entry);

        let thunk_env = thunk
            .get_nth_param(0)
            .ok_or_else(|| CodegenError::new("Async thunk missing env parameter"))?
            .into_pointer_value();
        let thunk_env_cast = self
            .builder
            .build_pointer_cast(thunk_env, ptr_type, "async_thunk_env_cast")
            .unwrap();

        let body_call = self
            .builder
            .build_call(body, &[thunk_env.into()], "async_body_call")
            .unwrap();

        let malloc = self.get_or_declare_malloc();
        let result_storage = if matches!(inner_return_type, Type::None) {
            let raw = self
                .builder
                .build_call(
                    malloc,
                    &[self.context.i64_type().const_int(1, false).into()],
                    "async_none_alloc",
                )
                .unwrap();
            let ptr =
                self.extract_call_pointer_value(raw, "malloc failed for async Task<None> result")?;
            let none_ptr = self
                .builder
                .build_pointer_cast(
                    ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "async_none_ptr",
                )
                .unwrap();
            self.builder
                .build_store(none_ptr, self.context.i8_type().const_int(0, false))
                .unwrap();
            ptr
        } else {
            let ret_ty = self.llvm_type(&inner_return_type);
            let size = ret_ty
                .size_of()
                .ok_or_else(|| CodegenError::new("failed to compute async result size"))?;
            let raw = self
                .builder
                .build_call(malloc, &[size.into()], "async_result_alloc")
                .unwrap();
            let ptr = self.extract_call_pointer_value(raw, "malloc failed for async result")?;
            let typed_ptr = self
                .builder
                .build_pointer_cast(
                    ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "async_result_ptr",
                )
                .unwrap();
            let result = self.extract_call_value_with_context(
                body_call,
                "async body should return value for non-None Task",
            )?;
            self.builder.build_store(typed_ptr, result).unwrap();
            ptr
        };
        let task_field_ptr = unsafe {
            self.builder
                .build_gep(
                    env_type,
                    thunk_env_cast,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context
                            .i32_type()
                            .const_int(func.params.len() as u64, false),
                    ],
                    "async_task_field",
                )
                .unwrap()
        };
        let task_ptr = self
            .builder
            .build_load(ptr_type, task_field_ptr, "async_task_ptr")
            .unwrap()
            .into_pointer_value();
        let result_field = unsafe {
            self.builder
                .build_gep(
                    self.task_struct_type(),
                    task_ptr,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context.i32_type().const_int(1, false),
                    ],
                    "async_task_result_ptr",
                )
                .unwrap()
        };
        self.builder
            .build_store(result_field, result_storage)
            .unwrap();
        let completed_field = unsafe {
            self.builder
                .build_gep(
                    self.task_struct_type(),
                    task_ptr,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context.i32_type().const_int(3, false),
                    ],
                    "async_task_completed_ptr",
                )
                .unwrap()
        };
        self.build_atomic_bool_store(
            completed_field,
            self.context.i8_type().const_int(1, false),
            AtomicOrdering::Release,
        )?;
        #[cfg(windows)]
        self.builder
            .build_return(Some(&self.context.i32_type().const_int(0, false)))
            .unwrap();
        #[cfg(not(windows))]
        self.builder.build_return(Some(&result_storage)).unwrap();

        // 3) Compile public wrapper: function name(...)
        self.current_function = Some(wrapper);
        self.current_return_type = Some(Type::Task(Box::new(inner_return_type.clone())));
        self.variables.clear();
        self.loop_stack.clear();
        self.reset_current_generic_bounds();
        let wrapper_entry = self.context.append_basic_block(wrapper, "entry");
        self.builder.position_at_end(wrapper_entry);

        let env_size = env_type
            .size_of()
            .ok_or_else(|| CodegenError::new("failed to compute async environment size"))?;
        let env_alloc = self
            .builder
            .build_call(malloc, &[env_size.into()], "async_env_alloc")
            .unwrap();
        let env_raw_ptr =
            self.extract_call_pointer_value(env_alloc, "malloc failed for async environment")?;
        let env_cast = self
            .builder
            .build_pointer_cast(
                env_raw_ptr,
                self.context.ptr_type(AddressSpace::default()),
                "async_env_store",
            )
            .unwrap();

        for (i, param) in func.params.iter().enumerate() {
            let param_val = wrapper.get_nth_param((i + 1) as u32).ok_or_else(|| {
                CodegenError::new(format!("Missing async wrapper parameter {}", param.name))
            })?;
            let field_ptr = unsafe {
                self.builder
                    .build_gep(
                        env_type,
                        env_cast,
                        &[
                            self.context.i32_type().const_int(0, false),
                            self.context.i32_type().const_int(i as u64, false),
                        ],
                        "async_env_field",
                    )
                    .unwrap()
            };
            self.builder.build_store(field_ptr, param_val).unwrap();
        }
        let task_slot_ptr = unsafe {
            self.builder
                .build_gep(
                    env_type,
                    env_cast,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context
                            .i32_type()
                            .const_int(func.params.len() as u64, false),
                    ],
                    "async_env_task_slot",
                )
                .unwrap()
        };
        self.builder
            .build_store(task_slot_ptr, ptr_type.const_null())
            .unwrap();

        let task = self.create_task(
            thunk.as_global_value().as_pointer_value(),
            env_raw_ptr,
            task_slot_ptr,
        )?;
        self.builder.build_return(Some(&task)).unwrap();

        self.current_function = None;
        self.current_return_type = None;
        self.reset_current_generic_bounds();
        Ok(())
    }

    pub fn compile_function(&mut self, func: &FunctionDecl) -> Result<()> {
        if func.is_extern {
            return Ok(());
        }

        if func.is_async {
            return self.compile_async_function(func);
        }

        let (function, _) = self.functions.get(&func.name).unwrap().clone();

        self.current_function = Some(function);
        self.current_return_type = Some(self.normalize_codegen_type(&func.return_type));
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);
        self.variables.clear();
        self.loop_stack.clear();
        self.reset_current_generic_bounds();
        self.extend_current_generic_bounds(&func.generic_params);

        // Special handling for main: store argc/argv in globals
        if func.name == "main" {
            let argc = function.get_nth_param(0).unwrap().into_int_value();
            let argv = function.get_nth_param(1).unwrap().into_pointer_value();

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
                .unwrap();

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
                .unwrap();
        }

        // Allocate parameters
        // Param 0 is argc for main, but for other functions 0 is env_ptr
        // We skip argc/argv for main in the regular parameter allocation loop
        // because main() in Arden is usually main(): None
        let start_idx = if func.name == "main" { 2 } else { 1 };
        for (i, param) in func.params.iter().enumerate() {
            let normalized_param_ty = self.normalize_codegen_type(&param.ty);
            let llvm_param = function.get_nth_param((i + start_idx) as u32).unwrap();
            let alloca = self
                .builder
                .build_alloca(self.llvm_type(&normalized_param_ty), &param.name)
                .unwrap();
            self.builder.build_store(alloca, llvm_param).unwrap();
            self.variables.insert(
                param.name.clone(),
                Variable {
                    ptr: alloca,
                    ty: normalized_param_ty,
                    mutable: param.mutable,
                },
            );
        }

        // Compile body
        for stmt in &func.body {
            self.compile_stmt(&stmt.node)?;
        }

        // Add implicit return
        if self.needs_terminator() {
            if func.name == "main" {
                // Main returns 0 for success
                let zero = self.context.i32_type().const_int(0, false);
                self.builder.build_return(Some(&zero)).unwrap();
            } else {
                match &func.return_type {
                    Type::None => {
                        self.builder.build_return(None).unwrap();
                    }
                    _ => {
                        self.builder.build_unreachable().unwrap();
                    }
                }
            }
        }

        self.current_function = None;
        self.reset_current_generic_bounds();
        Ok(())
    }

    // === Statements ===

    pub fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
            } => {
                let normalized_ty = self.normalize_codegen_type(ty);
                let val = self.compile_expr_with_expected_type(&value.node, &normalized_ty)?;
                let actual_ty = self.infer_expr_type(&value.node, &[]);
                self.reject_incompatible_expected_type_value(&normalized_ty, &actual_ty, val)?;
                let alloca = self
                    .builder
                    .build_alloca(self.llvm_type(&normalized_ty), name)
                    .unwrap();
                self.builder.build_store(alloca, val).unwrap();
                self.variables.insert(
                    name.clone(),
                    Variable {
                        ptr: alloca,
                        ty: normalized_ty,
                        mutable: *mutable,
                    },
                );
            }

            Stmt::Assign { target, value } => {
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
                            .unwrap();
                        let rhs_value =
                            self.compile_expr_with_expected_type(&rhs.node, &target_ty)?;
                        let result = self
                            .compile_binary_values(op, current, rhs_value, &target_ty, &rhs_ty)?;
                        self.builder.build_store(ptr, result).unwrap();
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
                            let current = self.compile_map_get_on_value_with_compiled_key(
                                map_value, &map_ty, key,
                            )?;
                            let rhs_value =
                                self.compile_expr_with_expected_type(&rhs.node, &val_ty)?;
                            let result = self
                                .compile_binary_values(op, current, rhs_value, &val_ty, &val_ty)?;
                            self.compile_map_set_on_value_with_compiled_key_value(
                                map_value, &map_ty, key, result,
                            )?;
                            return Ok(());
                        }
                        let args = [
                            Spanned::new(index.node.clone(), index.span.clone()),
                            Spanned::new(value.node.clone(), value.span.clone()),
                        ];
                        self.compile_map_method_on_value(map_value, &map_ty, "set", &args)?;
                        return Ok(());
                    }
                }

                let target_ty = self.infer_expr_type(&target.node, &[]);
                let ptr = self.compile_lvalue(&target.node)?;
                let val = self.compile_expr_with_expected_type(&value.node, &target_ty)?;
                let actual_ty = self.infer_expr_type(&value.node, &[]);
                self.reject_incompatible_expected_type_value(&target_ty, &actual_ty, val)?;
                self.builder.build_store(ptr, val).unwrap();
            }

            Stmt::Expr(expr) => {
                self.compile_expr(&expr.node)?;
            }

            Stmt::Return(value) => {
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
                                self.builder.build_return(Some(&zero)).unwrap();
                            } else {
                                self.builder.build_return(None).unwrap();
                            }
                        } else {
                            let val = if let Some(ret_ty) = self.current_return_type.clone() {
                                let inferred_expr_ty = self.infer_expr_type(&expr.node, &[]);
                                let compiled =
                                    self.compile_expr_with_expected_type(&expr.node, &ret_ty)?;
                                self.reject_incompatible_expected_type_value(
                                    &ret_ty,
                                    &inferred_expr_ty,
                                    compiled,
                                )?;
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
                                        .unwrap()
                                        .into()
                                } else {
                                    val
                                }
                            } else {
                                val
                            };
                            self.builder.build_return(Some(&ret_val)).unwrap();
                        }
                    }
                    None => {
                        if is_main {
                            let zero = self.context.i32_type().const_int(0, false);
                            self.builder.build_return(Some(&zero)).unwrap();
                        } else {
                            self.builder.build_return(None).unwrap();
                        }
                    }
                }
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
                        .unwrap();
                }
            }

            Stmt::Continue => {
                if let Some(loop_ctx) = self.loop_stack.last() {
                    self.builder
                        .build_unconditional_branch(loop_ctx.loop_block)
                        .unwrap();
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
            .unwrap();

        // Then
        self.builder.position_at_end(then_bb);
        self.with_variable_scope(|this| {
            for stmt in then_block {
                this.compile_stmt(&stmt.node)?;
            }
            Ok(())
        })?;
        if self.needs_terminator() {
            self.builder.build_unconditional_branch(merge_bb).unwrap();
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
            self.builder.build_unconditional_branch(merge_bb).unwrap();
        }

        self.builder.position_at_end(merge_bb);
        Ok(())
    }

    pub fn compile_while(&mut self, cond: &Spanned<Expr>, body: &Block) -> Result<()> {
        let func = self
            .current_function
            .ok_or_else(|| CodegenError::new("while loop used outside function"))?;

        // LOOP ROTATION OPTIMIZATION:
        // Instead of: while (cond) { body }
        // We generate: if (cond) { do { body } while (cond) }
        // This eliminates one branch per iteration!

        let entry_bb = self.context.append_basic_block(func, "while.entry");
        let body_bb = self.context.append_basic_block(func, "while.body");
        let cond_bb = self.context.append_basic_block(func, "while.cond");
        let after_bb = self.context.append_basic_block(func, "while.after");

        // First, check condition (entry test)
        self.builder.build_unconditional_branch(entry_bb).unwrap();
        self.builder.position_at_end(entry_bb);
        let entry_cond = self.compile_condition_expr(&cond.node)?;
        self.builder
            .build_conditional_branch(entry_cond, body_bb, after_bb)
            .unwrap();

        // Body (executed at least once if we get here)
        self.builder.position_at_end(body_bb);
        self.loop_stack.push(LoopContext {
            loop_block: cond_bb,
            after_block: after_bb,
        });
        self.with_variable_scope(|this| {
            for stmt in body {
                this.compile_stmt(&stmt.node)?;
            }
            Ok(())
        })?;
        self.loop_stack.pop();
        if self.needs_terminator() {
            self.builder.build_unconditional_branch(cond_bb).unwrap();
        }

        // Loop condition check at end (loop rotation)
        self.builder.position_at_end(cond_bb);
        let loop_cond = self.compile_condition_expr(&cond.node)?;

        // Branch prediction: likely to continue looping
        self.builder
            .build_conditional_branch(loop_cond, body_bb, after_bb)
            .unwrap();

        self.builder.position_at_end(after_bb);
        Ok(())
    }

    pub fn compile_for(
        &mut self,
        var: &str,
        var_type: Option<&Type>,
        iterable: &Spanned<Expr>,
        body: &Block,
    ) -> Result<()> {
        let saved_variables = self.variables.clone();
        let func = self
            .current_function
            .ok_or_else(|| CodegenError::new("for loop used outside function"))?;
        let iterable_ty = self.infer_builtin_argument_type(&iterable.node);
        let deref_iterable_ty = self.deref_codegen_type(&iterable_ty).clone();

        if let Type::List(inner) = deref_iterable_ty.clone() {
            let iter_ty = var_type.cloned().unwrap_or((*inner).clone());
            let var_alloca = self
                .builder
                .build_alloca(self.llvm_type(&iter_ty), var)
                .unwrap();
            self.variables.insert(
                var.to_string(),
                Variable {
                    ptr: var_alloca,
                    ty: iter_ty.clone(),
                    mutable: false,
                },
            );

            let list_value = self.compile_expr_with_expected_type(&iterable.node, &iterable_ty)?;
            let list_ptr = self.materialize_value_pointer_for_type(
                list_value,
                &deref_iterable_ty,
                "for_list_tmp",
            )?;
            let i64_type = self.context.i64_type();
            let i32_type = self.context.i32_type();
            let zero_i64 = i64_type.const_zero();
            let one_i64 = i64_type.const_int(1, false);
            let elem_size = if matches!(*inner, Type::Boolean) {
                1
            } else {
                8
            };
            let elem_llvm = self.llvm_type(&inner);
            let list_type = self.context.struct_type(
                &[
                    i64_type.into(),
                    i64_type.into(),
                    self.context.ptr_type(AddressSpace::default()).into(),
                ],
                false,
            );

            let idx_alloca = self.builder.build_alloca(i64_type, "for_list_idx").unwrap();
            self.builder.build_store(idx_alloca, zero_i64).unwrap();

            let len_ptr = unsafe {
                self.builder
                    .build_gep(
                        list_type.as_basic_type_enum(),
                        list_ptr,
                        &[i32_type.const_int(0, false), i32_type.const_int(1, false)],
                        "list_len_ptr",
                    )
                    .unwrap()
            };
            let data_ptr_ptr = unsafe {
                self.builder
                    .build_gep(
                        list_type.as_basic_type_enum(),
                        list_ptr,
                        &[i32_type.const_int(0, false), i32_type.const_int(2, false)],
                        "list_data_ptr_ptr",
                    )
                    .unwrap()
            };

            let cond_bb = self.context.append_basic_block(func, "for_list.cond");
            let body_bb = self.context.append_basic_block(func, "for_list.body");
            let inc_bb = self.context.append_basic_block(func, "for_list.inc");
            let after_bb = self.context.append_basic_block(func, "for_list.after");
            self.builder.build_unconditional_branch(cond_bb).unwrap();

            self.builder.position_at_end(cond_bb);
            let idx_val = self
                .builder
                .build_load(i64_type, idx_alloca, "for_list_idx_val")
                .unwrap()
                .into_int_value();
            let len_val = self
                .builder
                .build_load(i64_type, len_ptr, "for_list_len")
                .unwrap()
                .into_int_value();
            let cond = self
                .builder
                .build_int_compare(IntPredicate::SLT, idx_val, len_val, "for_list_cmp")
                .unwrap();
            self.builder
                .build_conditional_branch(cond, body_bb, after_bb)
                .unwrap();

            self.builder.position_at_end(body_bb);
            let data_ptr = self
                .builder
                .build_load(
                    self.context.ptr_type(AddressSpace::default()),
                    data_ptr_ptr,
                    "for_list_data",
                )
                .unwrap()
                .into_pointer_value();
            let byte_offset = self
                .builder
                .build_int_mul(
                    idx_val,
                    i64_type.const_int(elem_size, false),
                    "for_list_off",
                )
                .unwrap();
            let elem_ptr = unsafe {
                self.builder
                    .build_gep(
                        self.context.i8_type(),
                        data_ptr,
                        &[byte_offset],
                        "for_list_elem_ptr",
                    )
                    .unwrap()
            };
            let typed_ptr = self
                .builder
                .build_pointer_cast(
                    elem_ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "for_list_typed_ptr",
                )
                .unwrap();
            let elem_val = self
                .builder
                .build_load(elem_llvm, typed_ptr, "for_list_elem")
                .unwrap();
            let iter_val =
                self.adapt_for_loop_binding_value(elem_val, &inner, &iter_ty, "for_list_iter")?;
            self.builder.build_store(var_alloca, iter_val).unwrap();

            self.loop_stack.push(LoopContext {
                loop_block: inc_bb,
                after_block: after_bb,
            });
            self.with_variable_scope(|this| {
                for stmt in body {
                    this.compile_stmt(&stmt.node)?;
                }
                Ok(())
            })?;
            self.loop_stack.pop();
            if self.needs_terminator() {
                self.builder.build_unconditional_branch(inc_bb).unwrap();
            }

            self.builder.position_at_end(inc_bb);
            let next_idx = self
                .builder
                .build_int_add(idx_val, one_i64, "for_list_next")
                .unwrap();
            self.builder.build_store(idx_alloca, next_idx).unwrap();
            self.builder.build_unconditional_branch(cond_bb).unwrap();

            self.builder.position_at_end(after_bb);
            self.variables = saved_variables;
            return Ok(());
        }

        if !matches!(iterable.node, Expr::Range { .. })
            && matches!(deref_iterable_ty, Type::Range(_))
        {
            let Type::Range(inner) = deref_iterable_ty.clone() else {
                return Err(CodegenError::new(
                    "internal error: expected range type for range iteration",
                ));
            };
            let iter_ty = var_type.cloned().unwrap_or((*inner).clone());
            let var_alloca = self
                .builder
                .build_alloca(self.llvm_type(&iter_ty), var)
                .unwrap();
            self.variables.insert(
                var.to_string(),
                Variable {
                    ptr: var_alloca,
                    ty: iter_ty.clone(),
                    mutable: false,
                },
            );

            let range_alloca = self
                .builder
                .build_alloca(self.llvm_type(&iterable_ty), &format!("{var}_range"))
                .unwrap();
            let range_value = if matches!(iterable_ty, Type::Ref(_) | Type::MutRef(_)) {
                self.compile_deref(&iterable.node)?
            } else {
                self.compile_expr_with_expected_type(&iterable.node, &iterable_ty)?
            };
            self.builder.build_store(range_alloca, range_value).unwrap();

            let cond_bb = self.context.append_basic_block(func, "for_range_obj.cond");
            let body_bb = self.context.append_basic_block(func, "for_range_obj.body");
            let inc_bb = self.context.append_basic_block(func, "for_range_obj.inc");
            let after_bb = self.context.append_basic_block(func, "for_range_obj.after");

            self.builder.build_unconditional_branch(cond_bb).unwrap();

            self.builder.position_at_end(cond_bb);
            let loaded_range = self
                .builder
                .build_load(
                    self.llvm_type(&iterable_ty),
                    range_alloca,
                    "for_range_obj_val",
                )
                .unwrap();
            let has_next = self
                .compile_range_method_on_value(loaded_range, &iterable_ty, "has_next", &[])?
                .into_int_value();
            self.builder
                .build_conditional_branch(has_next, body_bb, after_bb)
                .unwrap();

            self.builder.position_at_end(body_bb);
            let loaded_range = self
                .builder
                .build_load(
                    self.llvm_type(&iterable_ty),
                    range_alloca,
                    "for_range_obj_next",
                )
                .unwrap();
            let next_value =
                self.compile_range_method_on_value(loaded_range, &iterable_ty, "next", &[])?;
            let iter_value =
                self.adapt_for_loop_binding_value(next_value, &inner, &iter_ty, "for_range_obj")?;
            self.builder.build_store(var_alloca, iter_value).unwrap();

            self.loop_stack.push(LoopContext {
                loop_block: inc_bb,
                after_block: after_bb,
            });
            self.with_variable_scope(|this| {
                for stmt in body {
                    this.compile_stmt(&stmt.node)?;
                }
                Ok(())
            })?;
            self.loop_stack.pop();
            if self.needs_terminator() {
                self.builder.build_unconditional_branch(inc_bb).unwrap();
            }

            self.builder.position_at_end(inc_bb);
            self.builder.build_unconditional_branch(cond_bb).unwrap();

            self.builder.position_at_end(after_bb);
            self.variables = saved_variables;
            return Ok(());
        }

        if matches!(deref_iterable_ty, Type::String) {
            let iter_ty = var_type.cloned().unwrap_or(Type::Char);
            let var_alloca = self
                .builder
                .build_alloca(self.llvm_type(&iter_ty), var)
                .unwrap();
            self.variables.insert(
                var.to_string(),
                Variable {
                    ptr: var_alloca,
                    ty: iter_ty.clone(),
                    mutable: false,
                },
            );

            let string_value = if matches!(iterable_ty, Type::Ref(_) | Type::MutRef(_)) {
                self.compile_deref(&iterable.node)?.into_pointer_value()
            } else {
                self.compile_expr_with_expected_type(&iterable.node, &iterable_ty)?
                    .into_pointer_value()
            };
            let len_alloca = self
                .builder
                .build_alloca(self.context.i64_type(), &format!("{var}_string_len"))
                .unwrap();
            let idx_alloca = self
                .builder
                .build_alloca(self.context.i64_type(), &format!("{var}_string_idx"))
                .unwrap();
            let length = self.compile_utf8_string_length_runtime(string_value)?;
            self.builder.build_store(len_alloca, length).unwrap();
            self.builder
                .build_store(idx_alloca, self.context.i64_type().const_zero())
                .unwrap();

            let cond_bb = self.context.append_basic_block(func, "for_string.cond");
            let body_bb = self.context.append_basic_block(func, "for_string.body");
            let inc_bb = self.context.append_basic_block(func, "for_string.inc");
            let after_bb = self.context.append_basic_block(func, "for_string.after");
            self.builder.build_unconditional_branch(cond_bb).unwrap();

            self.builder.position_at_end(cond_bb);
            let idx_val = self
                .builder
                .build_load(self.context.i64_type(), idx_alloca, "for_string_idx")
                .unwrap()
                .into_int_value();
            let len_val = self
                .builder
                .build_load(self.context.i64_type(), len_alloca, "for_string_len")
                .unwrap()
                .into_int_value();
            let cond = self
                .builder
                .build_int_compare(IntPredicate::SLT, idx_val, len_val, "for_string_cmp")
                .unwrap();
            self.builder
                .build_conditional_branch(cond, body_bb, after_bb)
                .unwrap();

            self.builder.position_at_end(body_bb);
            let ch = self.compile_utf8_string_index_runtime(string_value, idx_val)?;
            let iter_val =
                self.adapt_for_loop_binding_value(ch, &Type::Char, &iter_ty, "for_string_iter")?;
            self.builder.build_store(var_alloca, iter_val).unwrap();

            self.loop_stack.push(LoopContext {
                loop_block: inc_bb,
                after_block: after_bb,
            });
            self.with_variable_scope(|this| {
                for stmt in body {
                    this.compile_stmt(&stmt.node)?;
                }
                Ok(())
            })?;
            self.loop_stack.pop();
            if self.needs_terminator() {
                self.builder.build_unconditional_branch(inc_bb).unwrap();
            }

            self.builder.position_at_end(inc_bb);
            let next_idx = self
                .builder
                .build_int_add(
                    idx_val,
                    self.context.i64_type().const_int(1, false),
                    "for_string_next",
                )
                .unwrap();
            self.builder.build_store(idx_alloca, next_idx).unwrap();
            self.builder.build_unconditional_branch(cond_bb).unwrap();

            self.builder.position_at_end(after_bb);
            self.variables = saved_variables;
            return Ok(());
        }

        let ty = var_type.cloned().unwrap_or(Type::Integer);
        let var_alloca = self.builder.build_alloca(self.llvm_type(&ty), var).unwrap();
        let counter_alloca = self
            .builder
            .build_alloca(self.context.i64_type(), &format!("{var}_counter"))
            .unwrap();

        // Default range values
        let mut start_val: BasicValueEnum<'ctx> =
            self.context.i64_type().const_int(0, false).into();
        let mut end_val: BasicValueEnum<'ctx> = self.context.i64_type().const_int(0, false).into();
        let mut inclusive = false;

        match &iterable.node {
            Expr::Range {
                start,
                end,
                inclusive: inc,
            } => {
                if let Some(s) = start {
                    start_val = self.compile_integer_iteration_bound(&s.node)?.into();
                }
                if let Some(e) = end {
                    end_val = self.compile_integer_iteration_bound(&e.node)?.into();
                }
                inclusive = *inc;
            }
            _ => {
                // Treat as 0..N where N is the expression value
                end_val = self.compile_integer_iteration_bound(&iterable.node)?.into();
            }
        }

        let start_val = start_val.into_int_value();
        let end_val = end_val.into_int_value();
        self.builder.build_store(counter_alloca, start_val).unwrap();

        self.variables.insert(
            var.to_string(),
            Variable {
                ptr: var_alloca,
                ty: ty.clone(),
                mutable: false,
            },
        );

        let cond_bb = self.context.append_basic_block(func, "for.cond");
        let body_bb = self.context.append_basic_block(func, "for.body");
        let inc_bb = self.context.append_basic_block(func, "for.inc");
        let after_bb = self.context.append_basic_block(func, "for.after");

        self.builder.build_unconditional_branch(cond_bb).unwrap();

        // Condition
        self.builder.position_at_end(cond_bb);
        let current = self
            .builder
            .build_load(
                self.context.i64_type(),
                counter_alloca,
                &format!("{var}_current"),
            )
            .unwrap()
            .into_int_value();

        let cond = if inclusive {
            self.builder
                .build_int_compare(IntPredicate::SLE, current, end_val, "cmp")
                .unwrap()
        } else {
            self.builder
                .build_int_compare(IntPredicate::SLT, current, end_val, "cmp")
                .unwrap()
        };

        self.builder
            .build_conditional_branch(cond, body_bb, after_bb)
            .unwrap();

        // Body
        self.builder.position_at_end(body_bb);
        let iter_val =
            self.adapt_for_loop_binding_value(current.into(), &Type::Integer, &ty, "for_range")?;
        self.builder.build_store(var_alloca, iter_val).unwrap();
        self.loop_stack.push(LoopContext {
            loop_block: inc_bb,
            after_block: after_bb,
        });
        self.with_variable_scope(|this| {
            for stmt in body {
                this.compile_stmt(&stmt.node)?;
            }
            Ok(())
        })?;
        self.loop_stack.pop();
        if self.needs_terminator() {
            self.builder.build_unconditional_branch(inc_bb).unwrap();
        }

        // Increment
        self.builder.position_at_end(inc_bb);
        let current = self
            .builder
            .build_load(
                self.context.i64_type(),
                counter_alloca,
                &format!("{var}_counter"),
            )
            .unwrap()
            .into_int_value();
        let one = self.context.i64_type().const_int(1, false);
        let next = self.builder.build_int_add(current, one, "inc").unwrap();
        self.builder.build_store(counter_alloca, next).unwrap();
        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(after_bb);
        self.variables = saved_variables;
        Ok(())
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
                    .unwrap()
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

    fn encode_enum_payload(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<IntValue<'ctx>> {
        let i64_type = self.context.i64_type();
        let normalized_ty = self.normalize_codegen_type(ty);
        let encoded = match &normalized_ty {
            Type::Integer => value.into_int_value(),
            Type::Boolean => self
                .builder
                .build_int_z_extend(value.into_int_value(), i64_type, "bool_to_i64")
                .unwrap(),
            Type::Char => self
                .builder
                .build_int_z_extend(value.into_int_value(), i64_type, "char_to_i64")
                .unwrap(),
            Type::Float => self
                .builder
                .build_bit_cast(value.into_float_value(), i64_type, "float_bits")
                .unwrap()
                .into_int_value(),
            Type::String | Type::Named(_) | Type::Ref(_) | Type::MutRef(_) | Type::Ptr(_) => self
                .builder
                .build_ptr_to_int(value.into_pointer_value(), i64_type, "ptr_to_i64")
                .unwrap(),
            Type::Generic(name, _)
                if self
                    .canonical_codegen_type_name(name)
                    .is_some_and(|canonical| self.classes.contains_key(&canonical)) =>
            {
                self.builder
                    .build_ptr_to_int(value.into_pointer_value(), i64_type, "ptr_to_i64")
                    .unwrap()
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
                .unwrap()
                .into(),
            Type::Char => self
                .builder
                .build_int_truncate(raw, self.context.i32_type(), "i64_to_char")
                .unwrap()
                .into(),
            Type::Float => self
                .builder
                .build_bit_cast(raw, self.context.f64_type(), "bits_to_float")
                .unwrap(),
            Type::String | Type::Named(_) | Type::Ref(_) | Type::MutRef(_) | Type::Ptr(_) => self
                .builder
                .build_int_to_ptr(
                    raw,
                    self.context.ptr_type(AddressSpace::default()),
                    "i64_to_ptr",
                )
                .unwrap()
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
                    .unwrap()
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
            .unwrap()
            .into_struct_value();

        for (i, field_ty) in variant_info.fields.iter().enumerate() {
            let encoded = self.encode_enum_payload(values[i], field_ty)?;
            value = self
                .builder
                .build_insert_value(value, encoded, (i + 1) as u32, "enum_payload")
                .unwrap()
                .into_struct_value();
        }

        Ok(value.into())
    }

    pub fn compile_match_stmt(&mut self, expr: &Spanned<Expr>, arms: &[MatchArm]) -> Result<()> {
        let imported_variant = |this: &Self, name: &str| -> Option<(String, String, bool)> {
            this.resolve_pattern_variant_alias(name)
        };
        let imported_unit_variant = |this: &Self, name: &str| -> Option<(String, String, u8)> {
            let (enum_name, variant_name, is_unit) = this.resolve_pattern_variant_alias(name)?;
            if !is_unit {
                return None;
            }
            let variant_info = this.enums.get(&enum_name)?.variants.get(&variant_name)?;
            Some((enum_name, variant_name, variant_info.tag))
        };
        let match_ty = self.infer_builtin_argument_type(&expr.node);
        let option_inner_ty = match &match_ty {
            Type::Option(inner) => Some((**inner).clone()),
            _ => None,
        };
        let is_option_match = option_inner_ty.is_some();
        let result_inner_tys = match &match_ty {
            Type::Result(ok, err) => Some(((**ok).clone(), (**err).clone())),
            _ => None,
        };
        let is_result_match = result_inner_tys.is_some();
        let enum_match_name = match &match_ty {
            Type::Named(name) if self.enums.contains_key(name) => Some(name.clone()),
            _ => None,
        };

        for arm in arms {
            fn pattern_variant_leaf(name: &str) -> &str {
                name.rsplit('.').next().unwrap_or(name)
            }

            match &arm.pattern {
                Pattern::Literal(lit) => {
                    let pattern_ty = self.infer_expr_type(&Expr::Literal(lit.clone()), &[]);
                    if self
                        .common_compatible_codegen_type(&match_ty, &pattern_ty)
                        .is_none()
                    {
                        return Err(CodegenError::new(format!(
                            "Pattern type mismatch: expected {}, found {}",
                            Self::format_diagnostic_type(&match_ty),
                            Self::format_diagnostic_type(&pattern_ty)
                        )));
                    }
                }
                Pattern::Variant(variant_name, _) => {
                    let resolved_variant = if !variant_name.contains('.') {
                        imported_variant(self, variant_name)
                    } else {
                        None
                    };
                    let variant_leaf = resolved_variant
                        .as_ref()
                        .map(|(_, resolved_variant_name, _)| resolved_variant_name.as_str())
                        .unwrap_or_else(|| pattern_variant_leaf(variant_name));
                    let resolved_enum_name =
                        resolved_variant.as_ref().map(|(enum_name, _, _)| enum_name);
                    let matches_builtin_variant = (is_option_match
                        && matches!(variant_leaf, "Some" | "None"))
                        || (is_result_match && matches!(variant_leaf, "Ok" | "Error"));
                    let enum_variant_exists = resolved_enum_name
                        .or(enum_match_name.as_ref())
                        .and_then(|enum_name| self.enums.get(enum_name))
                        .is_some_and(|enum_info| enum_info.variants.contains_key(variant_leaf));
                    if !matches_builtin_variant && !enum_variant_exists {
                        return Err(CodegenError::new(format!(
                            "Cannot match variant {} on type {}",
                            variant_leaf,
                            Self::format_diagnostic_type(&match_ty)
                        )));
                    }
                }
                _ => {}
            }
        }

        let val = self.compile_expr_with_expected_type(&expr.node, &match_ty)?;
        let func = self
            .current_function
            .ok_or_else(|| CodegenError::new("match statement used outside function"))?;
        let merge_bb = self.context.append_basic_block(func, "match.merge");

        let mut dispatch_bb = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::new("match statement insert block missing"))?;

        for arm in arms {
            let arm_bb = self.context.append_basic_block(func, "match.arm");
            let next_bb = self.context.append_basic_block(func, "match.next");

            self.builder.position_at_end(dispatch_bb);
            fn pattern_variant_leaf(name: &str) -> &str {
                name.rsplit('.').next().unwrap_or(name)
            }

            match &arm.pattern {
                Pattern::Wildcard => {
                    self.builder.build_unconditional_branch(arm_bb).unwrap();
                }
                Pattern::Ident(name) => {
                    if let Some((enum_name, variant_name, variant_tag)) =
                        imported_unit_variant(self, name)
                    {
                        let is_builtin_variant = (is_option_match
                            && matches!(variant_name.as_str(), "Some" | "None"))
                            || (is_result_match && matches!(variant_name.as_str(), "Ok" | "Error"));
                        let enum_matches = enum_match_name
                            .as_ref()
                            .is_some_and(|expected_enum| expected_enum == &enum_name);
                        if is_builtin_variant || enum_matches {
                            let tag = self
                                .builder
                                .build_extract_value(val.into_struct_value(), 0, "tag")
                                .unwrap()
                                .into_int_value();
                            let cond = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::EQ,
                                    tag,
                                    self.context.i8_type().const_int(variant_tag as u64, false),
                                    "match_ident_variant_eq",
                                )
                                .unwrap();
                            self.builder
                                .build_conditional_branch(cond, arm_bb, next_bb)
                                .unwrap();
                        } else {
                            self.builder.build_unconditional_branch(next_bb).unwrap();
                        }
                    } else {
                        self.builder.build_unconditional_branch(arm_bb).unwrap();
                    }
                }
                Pattern::Literal(lit) => {
                    let pattern_ty = self.infer_expr_type(&Expr::Literal(lit.clone()), &[]);
                    if self
                        .common_compatible_codegen_type(&match_ty, &pattern_ty)
                        .is_none()
                    {
                        return Err(CodegenError::new(format!(
                            "Pattern type mismatch: expected {}, found {}",
                            Self::format_diagnostic_type(&match_ty),
                            Self::format_diagnostic_type(&pattern_ty)
                        )));
                    }
                    let pattern_val = self.compile_literal(lit)?;
                    let cond = if val.is_float_value() || pattern_val.is_float_value() {
                        let match_val = if val.is_float_value() {
                            val.into_float_value()
                        } else {
                            self.builder
                                .build_signed_int_to_float(
                                    val.into_int_value(),
                                    self.context.f64_type(),
                                    "match_lit_lf",
                                )
                                .unwrap()
                        };
                        let pattern_float = if pattern_val.is_float_value() {
                            pattern_val.into_float_value()
                        } else {
                            self.builder
                                .build_signed_int_to_float(
                                    pattern_val.into_int_value(),
                                    self.context.f64_type(),
                                    "match_lit_rf",
                                )
                                .unwrap()
                        };
                        self.builder
                            .build_float_compare(
                                FloatPredicate::OEQ,
                                match_val,
                                pattern_float,
                                "match_float_eq",
                            )
                            .unwrap()
                    } else if val.is_int_value() && pattern_val.is_int_value() {
                        self.builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                val.into_int_value(),
                                pattern_val.into_int_value(),
                                "match_lit_eq",
                            )
                            .unwrap()
                    } else if val.is_pointer_value() && pattern_val.is_pointer_value() {
                        let strcmp = self.get_or_declare_strcmp();
                        let cmp = self
                            .builder
                            .build_call(
                                strcmp,
                                &[
                                    val.into_pointer_value().into(),
                                    pattern_val.into_pointer_value().into(),
                                ],
                                "match_strcmp",
                            )
                            .unwrap();
                        let cmp_val = self.extract_call_value(cmp)?.into_int_value();
                        self.builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                cmp_val,
                                self.context.i32_type().const_int(0, false),
                                "match_str_eq",
                            )
                            .unwrap()
                    } else {
                        self.context.bool_type().const_int(0, false)
                    };
                    self.builder
                        .build_conditional_branch(cond, arm_bb, next_bb)
                        .unwrap();
                }
                Pattern::Variant(variant_name, _) => {
                    let resolved_variant = if !variant_name.contains('.') {
                        imported_variant(self, variant_name)
                    } else {
                        None
                    };
                    let variant_leaf = resolved_variant
                        .as_ref()
                        .map(|(_, resolved_variant_name, _)| resolved_variant_name.as_str())
                        .unwrap_or_else(|| pattern_variant_leaf(variant_name));
                    let resolved_enum_name =
                        resolved_variant.as_ref().map(|(enum_name, _, _)| enum_name);
                    let matches_builtin_variant = (is_option_match
                        && matches!(variant_leaf, "Some" | "None"))
                        || (is_result_match && matches!(variant_leaf, "Ok" | "Error"));
                    let enum_variant_info = resolved_enum_name
                        .or(enum_match_name.as_ref())
                        .and_then(|enum_name| {
                            self.enums
                                .get(enum_name)
                                .and_then(|enum_info| enum_info.variants.get(variant_leaf))
                                .map(|variant_info| (enum_name, variant_info))
                        });
                    // Built-in Option / Result matching
                    if matches_builtin_variant {
                        let expected_tag = match variant_leaf {
                            "Some" | "Ok" => 1u64,
                            _ => 0u64,
                        };
                        let tag = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 0, "tag")
                            .unwrap()
                            .into_int_value();
                        let cond = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                tag,
                                self.context.i8_type().const_int(expected_tag, false),
                                "match_variant_eq",
                            )
                            .unwrap();
                        self.builder
                            .build_conditional_branch(cond, arm_bb, next_bb)
                            .unwrap();
                    } else if let Some((_, variant_info)) = enum_variant_info {
                        let tag = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 0, "tag")
                            .unwrap()
                            .into_int_value();
                        let cond = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                tag,
                                self.context
                                    .i8_type()
                                    .const_int(variant_info.tag as u64, false),
                                "match_enum_variant_eq",
                            )
                            .unwrap();
                        self.builder
                            .build_conditional_branch(cond, arm_bb, next_bb)
                            .unwrap();
                    } else {
                        return Err(CodegenError::new(format!(
                            "Cannot match variant {} on type {}",
                            variant_leaf,
                            Self::format_diagnostic_type(&match_ty)
                        )));
                    }
                }
            }

            self.builder.position_at_end(arm_bb);
            self.with_variable_scope(|this| {
                match &arm.pattern {
                    Pattern::Ident(binding) => {
                        if imported_unit_variant(this, binding).is_none() {
                            let alloca =
                                this.builder.build_alloca(val.get_type(), binding).unwrap();
                            this.builder.build_store(alloca, val).unwrap();
                            this.variables.insert(
                                binding.clone(),
                                Variable {
                                    ptr: alloca,
                                    ty: match_ty.clone(),
                                    mutable: false,
                                },
                            );
                        }
                    }
                    Pattern::Variant(variant_name, bindings) => {
                        let resolved_variant = if !variant_name.contains('.') {
                            imported_variant(this, variant_name)
                        } else {
                            None
                        };
                        let variant_leaf = resolved_variant
                            .as_ref()
                            .map(|(_, resolved_variant_name, _)| resolved_variant_name.as_str())
                            .unwrap_or_else(|| pattern_variant_leaf(variant_name));
                        let resolved_enum_name =
                            resolved_variant.as_ref().map(|(enum_name, _, _)| enum_name);
                        if is_option_match && variant_leaf == "Some" && !bindings.is_empty() {
                            let inner = this
                                .builder
                                .build_extract_value(val.into_struct_value(), 1, "some_inner")
                                .unwrap();
                            let alloca = this
                                .builder
                                .build_alloca(inner.get_type(), &bindings[0])
                                .unwrap();
                            this.builder.build_store(alloca, inner).unwrap();
                            this.variables.insert(
                                bindings[0].clone(),
                                Variable {
                                    ptr: alloca,
                                    ty: option_inner_ty.clone().unwrap_or(Type::Integer),
                                    mutable: false,
                                },
                            );
                        } else if is_result_match && variant_leaf == "Ok" && !bindings.is_empty() {
                            let inner = this
                                .builder
                                .build_extract_value(val.into_struct_value(), 1, "ok_inner")
                                .unwrap();
                            let alloca = this
                                .builder
                                .build_alloca(inner.get_type(), &bindings[0])
                                .unwrap();
                            this.builder.build_store(alloca, inner).unwrap();
                            this.variables.insert(
                                bindings[0].clone(),
                                Variable {
                                    ptr: alloca,
                                    ty: result_inner_tys
                                        .as_ref()
                                        .map(|(ok, _)| ok.clone())
                                        .unwrap_or(Type::Integer),
                                    mutable: false,
                                },
                            );
                        } else if is_result_match && variant_leaf == "Error" && !bindings.is_empty()
                        {
                            let inner = this
                                .builder
                                .build_extract_value(val.into_struct_value(), 2, "err_inner")
                                .unwrap();
                            let alloca = this
                                .builder
                                .build_alloca(inner.get_type(), &bindings[0])
                                .unwrap();
                            this.builder.build_store(alloca, inner).unwrap();
                            this.variables.insert(
                                bindings[0].clone(),
                                Variable {
                                    ptr: alloca,
                                    ty: result_inner_tys
                                        .as_ref()
                                        .map(|(_, err)| err.clone())
                                        .unwrap_or(Type::String),
                                    mutable: false,
                                },
                            );
                        } else if let Some(enum_name) =
                            resolved_enum_name.or(enum_match_name.as_ref())
                        {
                            if let Some(enum_info) = this.enums.get(enum_name) {
                                if let Some(variant_info) = enum_info.variants.get(variant_leaf) {
                                    for (idx, binding) in bindings.iter().enumerate() {
                                        if let Some(field_ty) = variant_info.fields.get(idx) {
                                            let raw = this
                                                .builder
                                                .build_extract_value(
                                                    val.into_struct_value(),
                                                    (idx + 1) as u32,
                                                    "enum_payload_raw",
                                                )
                                                .unwrap()
                                                .into_int_value();
                                            let decoded =
                                                this.decode_enum_payload(raw, field_ty)?;
                                            let alloca = this
                                                .builder
                                                .build_alloca(decoded.get_type(), binding)
                                                .unwrap();
                                            this.builder.build_store(alloca, decoded).unwrap();
                                            this.variables.insert(
                                                binding.clone(),
                                                Variable {
                                                    ptr: alloca,
                                                    ty: field_ty.clone(),
                                                    mutable: false,
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }

                for stmt in &arm.body {
                    this.compile_stmt(&stmt.node)?;
                }
                Ok(())
            })?;
            if self.needs_terminator() {
                self.builder.build_unconditional_branch(merge_bb).unwrap();
            }

            dispatch_bb = next_bb;
            self.builder.position_at_end(dispatch_bb);
        }

        if self.needs_terminator() {
            self.builder.build_unconditional_branch(merge_bb).unwrap();
        }

        self.builder.position_at_end(merge_bb);
        Ok(())
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

    fn compile_async_block(
        &mut self,
        body: &[Spanned<Stmt>],
        expected_inner_return_type: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let mut captures: Vec<(String, Type)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut params = std::collections::HashSet::new();
        for stmt in body {
            self.walk_stmt_for_captures(&stmt.node, &mut params, &mut captures, &mut seen);
        }

        let inner_return_type = expected_inner_return_type.cloned().unwrap_or_else(|| {
            self.infer_async_block_return_type(body, expected_inner_return_type)
        });
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let mut env_fields = Vec::new();
        for (_, ty) in &captures {
            env_fields.push(self.llvm_type(ty));
        }
        env_fields.push(ptr_ty.into());
        let env_type = self.context.struct_type(&env_fields, false);

        let malloc = self.get_or_declare_malloc();
        let env_size = env_type
            .size_of()
            .ok_or_else(|| CodegenError::new("failed to compute async block env size"))?;
        let env_alloc = self
            .builder
            .build_call(malloc, &[env_size.into()], "async_block_env")
            .unwrap();
        let env_raw =
            self.extract_call_pointer_value(env_alloc, "malloc failed for async block env")?;
        let env_cast = self
            .builder
            .build_pointer_cast(
                env_raw,
                self.context.ptr_type(AddressSpace::default()),
                "async_block_env_cast",
            )
            .unwrap();

        for (i, (name, ty)) in captures.iter().enumerate() {
            let var = self.variables.get(name).ok_or_else(|| {
                CodegenError::new(format!("async block capture '{}' not found", name))
            })?;
            let val = self
                .builder
                .build_load(self.llvm_type(ty), var.ptr, name)
                .unwrap();
            let field_ptr = unsafe {
                self.builder
                    .build_gep(
                        env_type,
                        env_cast,
                        &[
                            self.context.i32_type().const_int(0, false),
                            self.context.i32_type().const_int(i as u64, false),
                        ],
                        "async_block_capture",
                    )
                    .unwrap()
            };
            self.builder.build_store(field_ptr, val).unwrap();
        }
        let task_slot_ptr = unsafe {
            self.builder
                .build_gep(
                    env_type,
                    env_cast,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context
                            .i32_type()
                            .const_int(captures.len() as u64, false),
                    ],
                    "async_block_task_slot",
                )
                .unwrap()
        };
        self.builder
            .build_store(task_slot_ptr, ptr_ty.const_null())
            .unwrap();

        let saved_function = self.current_function;
        let saved_return_type = self.current_return_type.clone();
        let saved_variables = std::mem::take(&mut self.variables);
        let saved_loop_stack = std::mem::take(&mut self.loop_stack);
        let saved_insert_block = self.builder.get_insert_block();

        let id = self.async_counter;
        self.async_counter += 1;

        let body_name = format!("__arden_async_block_body_{}", id);
        let body_fn_type = match &inner_return_type {
            Type::None => self.context.void_type().fn_type(&[ptr_ty.into()], false),
            ty => self.llvm_type(ty).fn_type(&[ptr_ty.into()], false),
        };
        let body_fn = self.module.add_function(&body_name, body_fn_type, None);

        self.current_function = Some(body_fn);
        self.current_return_type = Some(inner_return_type.clone());
        self.loop_stack.clear();
        let body_entry = self.context.append_basic_block(body_fn, "entry");
        self.builder.position_at_end(body_entry);

        let body_env_raw = body_fn
            .get_nth_param(0)
            .ok_or_else(|| CodegenError::new("async block body missing env param"))?
            .into_pointer_value();
        let body_env = self
            .builder
            .build_pointer_cast(
                body_env_raw,
                self.context.ptr_type(AddressSpace::default()),
                "async_block_body_env",
            )
            .unwrap();

        for (i, (name, ty)) in captures.iter().enumerate() {
            let field_ptr = unsafe {
                self.builder
                    .build_gep(
                        env_type,
                        body_env,
                        &[
                            self.context.i32_type().const_int(0, false),
                            self.context.i32_type().const_int(i as u64, false),
                        ],
                        "async_block_body_field",
                    )
                    .unwrap()
            };
            let loaded = self
                .builder
                .build_load(self.llvm_type(ty), field_ptr, "async_capture_load")
                .unwrap();
            let alloca = self.builder.build_alloca(self.llvm_type(ty), name).unwrap();
            self.builder.build_store(alloca, loaded).unwrap();
            self.variables.insert(
                name.clone(),
                Variable {
                    ptr: alloca,
                    ty: ty.clone(),
                    mutable: false,
                },
            );
        }

        for (index, stmt) in body.iter().enumerate() {
            let is_last = index + 1 == body.len();
            if is_last && self.needs_terminator() {
                if let Stmt::Expr(expr) = &stmt.node {
                    if !matches!(inner_return_type, Type::None) {
                        let inferred_expr_ty = self.infer_expr_type(&expr.node, &[]);
                        let value =
                            self.compile_expr_with_expected_type(&expr.node, &inner_return_type)?;
                        self.reject_incompatible_expected_type_value(
                            &inner_return_type,
                            &inferred_expr_ty,
                            value,
                        )?;
                        self.builder.build_return(Some(&value)).unwrap();
                        continue;
                    }
                }
            }
            self.compile_stmt(&stmt.node)?;
        }
        if self.needs_terminator() {
            if matches!(inner_return_type, Type::None) {
                self.builder.build_return(None).unwrap();
            } else {
                self.builder.build_unreachable().unwrap();
            }
        }

        let thunk_name = format!("__arden_async_block_thunk_{}", id);
        #[cfg(windows)]
        let thunk_fn_type = self.context.i32_type().fn_type(&[ptr_ty.into()], false);
        #[cfg(not(windows))]
        let thunk_fn_type = ptr_ty.fn_type(&[ptr_ty.into()], false);
        let thunk_fn = self.module.add_function(&thunk_name, thunk_fn_type, None);

        self.current_function = Some(thunk_fn);
        self.current_return_type = None;
        self.variables.clear();
        self.loop_stack.clear();
        let thunk_entry = self.context.append_basic_block(thunk_fn, "entry");
        self.builder.position_at_end(thunk_entry);

        let thunk_env = thunk_fn
            .get_nth_param(0)
            .ok_or_else(|| CodegenError::new("async block thunk missing env param"))?
            .into_pointer_value();
        let thunk_env_cast = self
            .builder
            .build_pointer_cast(thunk_env, ptr_ty, "async_block_thunk_env")
            .unwrap();
        let body_call = self
            .builder
            .build_call(body_fn, &[thunk_env.into()], "async_block_call")
            .unwrap();

        let result_ptr = if matches!(inner_return_type, Type::None) {
            let alloc = self
                .builder
                .build_call(
                    malloc,
                    &[self.context.i64_type().const_int(1, false).into()],
                    "async_block_none_alloc",
                )
                .unwrap();
            let ptr = self
                .extract_call_pointer_value(alloc, "malloc failed for async block none result")?;
            let none_ptr = self
                .builder
                .build_pointer_cast(
                    ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "async_block_none_ptr",
                )
                .unwrap();
            self.builder
                .build_store(none_ptr, self.context.i8_type().const_int(0, false))
                .unwrap();
            ptr
        } else {
            let ret_ty = self.llvm_type(&inner_return_type);
            let size = ret_ty
                .size_of()
                .ok_or_else(|| CodegenError::new("failed to compute async block result size"))?;
            let alloc = self
                .builder
                .build_call(malloc, &[size.into()], "async_block_alloc")
                .unwrap();
            let ptr =
                self.extract_call_pointer_value(alloc, "malloc failed for async block result")?;
            let typed_ptr = self
                .builder
                .build_pointer_cast(
                    ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "async_block_result_ptr",
                )
                .unwrap();
            let result_val = self.extract_call_value_with_context(
                body_call,
                "async block body should return value for non-None Task",
            )?;
            self.builder.build_store(typed_ptr, result_val).unwrap();
            ptr
        };
        let task_field_ptr = unsafe {
            self.builder
                .build_gep(
                    env_type,
                    thunk_env_cast,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context
                            .i32_type()
                            .const_int(captures.len() as u64, false),
                    ],
                    "async_block_task_field",
                )
                .unwrap()
        };
        let task_ptr = self
            .builder
            .build_load(ptr_ty, task_field_ptr, "async_block_task_ptr")
            .unwrap()
            .into_pointer_value();
        let result_field = unsafe {
            self.builder
                .build_gep(
                    self.task_struct_type(),
                    task_ptr,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context.i32_type().const_int(1, false),
                    ],
                    "async_block_result_field",
                )
                .unwrap()
        };
        self.builder.build_store(result_field, result_ptr).unwrap();
        let completed_field = unsafe {
            self.builder
                .build_gep(
                    self.task_struct_type(),
                    task_ptr,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context.i32_type().const_int(3, false),
                    ],
                    "async_block_completed_field",
                )
                .unwrap()
        };
        self.build_atomic_bool_store(
            completed_field,
            self.context.i8_type().const_int(1, false),
            AtomicOrdering::Release,
        )?;
        #[cfg(windows)]
        self.builder
            .build_return(Some(&self.context.i32_type().const_int(0, false)))
            .unwrap();
        #[cfg(not(windows))]
        self.builder.build_return(Some(&result_ptr)).unwrap();

        self.current_function = saved_function;
        self.current_return_type = saved_return_type;
        self.variables = saved_variables;
        self.loop_stack = saved_loop_stack;
        if let Some(block) = saved_insert_block {
            self.builder.position_at_end(block);
        }

        let task = self.create_task(
            thunk_fn.as_global_value().as_pointer_value(),
            env_raw,
            task_slot_ptr,
        )?;
        Ok(task.into())
    }

    pub fn compile_expr(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>> {
        match expr {
            Expr::Literal(lit) => self.compile_literal(lit),

            Expr::Ident(name) => {
                if let Some(var) = self.variables.get(name) {
                    let val = self
                        .builder
                        .build_load(self.llvm_type(&var.ty), var.ptr, name)
                        .unwrap();
                    Ok(val)
                } else {
                    let resolved_name = self.resolve_function_alias(name);
                    let lookup_name = resolved_name.as_str();
                    if let Some((func, ty)) = self.functions.get(lookup_name) {
                        if self.extern_functions.contains(lookup_name) {
                            return Err(CodegenError::new(format!(
                                "extern function '{}' cannot be used as a first-class value yet",
                                lookup_name
                            )));
                        }
                        // Create a closure struct { fn_ptr, null_env }
                        let struct_ty = self.llvm_type(ty).into_struct_type();
                        let mut closure = struct_ty.get_undef();

                        let fn_ptr = func.as_global_value().as_pointer_value();
                        let null_env = self.context.ptr_type(AddressSpace::default()).const_null();

                        closure = self
                            .builder
                            .build_insert_value(closure, fn_ptr, 0, "fn")
                            .unwrap()
                            .into_struct_value();
                        closure = self
                            .builder
                            .build_insert_value(closure, null_env, 1, "env")
                            .unwrap()
                            .into_struct_value();

                        Ok(closure.into())
                    } else if let Some((enum_name, variant_name)) =
                        self.resolve_import_alias_variant(name)
                    {
                        if let Some(enum_info) = self.enums.get(&enum_name) {
                            if let Some(variant_info) =
                                enum_info.variants.get(&variant_name).cloned()
                            {
                                if variant_info.fields.is_empty() {
                                    self.build_enum_value(&enum_name, &variant_info, &[])
                                } else {
                                    Err(CodegenError::new(format!(
                                        "Enum variant '{}.{}' requires constructor arguments",
                                        enum_name, variant_name
                                    )))
                                }
                            } else {
                                Err(CodegenError::new(format!(
                                    "Unknown variant '{}' for enum '{}'",
                                    variant_name, enum_name
                                )))
                            }
                        } else {
                            Err(CodegenError::new(format!("Unknown enum '{}'", enum_name)))
                        }
                    } else {
                        Err(Self::undefined_variable_error(name))
                    }
                }
            }

            Expr::Binary { op, left, right } => self.compile_binary(*op, &left.node, &right.node),

            Expr::Unary { op, expr } => self.compile_unary(*op, &expr.node),

            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                if !type_args.is_empty() {
                    if let Some(err) = self.explicit_generic_field_access_error(&callee.node, true)
                    {
                        return Err(err);
                    }
                    if let Some(path_parts) = flatten_field_chain(&callee.node) {
                        let full_path = path_parts.join(".");
                        if self
                            .resolve_alias_qualified_codegen_type_name(&full_path)
                            .is_some_and(|resolved| self.classes.contains_key(&resolved))
                        {
                            let ty_source = format!(
                                "{}<{}>",
                                full_path,
                                type_args
                                    .iter()
                                    .map(Self::format_type_string)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            );
                            return self.compile_construct(&ty_source, args);
                        }
                    }
                    if let Some((enum_name, variant_info)) =
                        self.resolve_enum_variant_function_value(&callee.node)
                    {
                        let variant_name = self
                            .enums
                            .get(&enum_name)
                            .and_then(|enum_info| {
                                enum_info.variants.iter().find_map(|(name, info)| {
                                    (info.tag == variant_info.tag
                                        && info.fields == variant_info.fields)
                                        .then(|| name.clone())
                                })
                            })
                            .unwrap_or_else(|| "<unknown>".to_string());
                        return Err(CodegenError::new(format!(
                            "Enum variant '{}.{}' does not accept type arguments",
                            Self::format_diagnostic_name(&enum_name),
                            variant_name
                        )));
                    }
                    if let Expr::Field { object, field } = &callee.node {
                        if let Expr::Ident(owner_name) = &object.node {
                            let owner = self.resolve_module_alias(owner_name);
                            match (owner.as_str(), field.as_str()) {
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
                    if let Some(canonical_name) =
                        self.resolve_contextual_function_value_name(&callee.node)
                    {
                        match canonical_name.as_str() {
                            _ if Self::is_supported_builtin_function_name(&canonical_name) => {
                                return Err(CodegenError::new(format!(
                                    "Built-in function '{}' does not accept type arguments",
                                    canonical_name.replace("__", ".")
                                )));
                            }
                            _ => {}
                        }
                    }
                    let _ = self.compile_call(&callee.node, args)?;
                    return Err(CodegenError::new(
                        "Explicit generic call code generation is not supported yet".to_string(),
                    ));
                }
                self.compile_call(&callee.node, args)
            }

            Expr::GenericFunctionValue { callee, .. } => {
                if let Some((enum_name, variant_info)) =
                    self.resolve_enum_variant_function_value(&callee.node)
                {
                    let variant_name = self
                        .enums
                        .get(&enum_name)
                        .and_then(|enum_info| {
                            enum_info.variants.iter().find_map(|(name, info)| {
                                (info.tag == variant_info.tag && info.fields == variant_info.fields)
                                    .then(|| name.clone())
                            })
                        })
                        .unwrap_or_else(|| "<unknown>".to_string());
                    return Err(CodegenError::new(format!(
                        "Enum variant '{}.{}' does not accept type arguments",
                        Self::format_diagnostic_name(&enum_name),
                        variant_name
                    )));
                }
                if let Some(canonical_name) =
                    self.resolve_contextual_function_value_name(&callee.node)
                {
                    if Self::is_supported_builtin_function_name(&canonical_name) {
                        return Err(CodegenError::new(format!(
                            "Built-in function '{}' does not accept type arguments",
                            canonical_name.replace("__", ".")
                        )));
                    }
                }
                if let Some(err) = self.explicit_generic_field_access_error(&callee.node, false) {
                    return Err(err);
                }
                self.compile_expr(&callee.node)?;
                Err(CodegenError::new(
                    "Explicit generic function value should be specialized before code generation"
                        .to_string(),
                ))
            }

            Expr::Field { object, field } => self.compile_field(&object.node, field),

            Expr::Index { object, index } => self.compile_index(&object.node, &index.node),

            Expr::Construct { ty, args } => self.compile_construct(ty, args),

            Expr::This => {
                if let Some(var) = self.variables.get("this") {
                    let val = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            var.ptr,
                            "this",
                        )
                        .unwrap();
                    Ok(val)
                } else {
                    Err(CodegenError::new("'this' not available"))
                }
            }

            Expr::StringInterp(parts) => self.compile_string_interp(parts),

            Expr::Lambda { params, body } => self.compile_lambda(params, body, None),

            Expr::Match { expr, arms } => self.compile_match_expr(&expr.node, arms, None),

            Expr::Try(inner) => self.compile_try(&inner.node),

            Expr::Borrow(inner) | Expr::MutBorrow(inner) => {
                // Get pointer to the expression (lvalue)
                self.compile_borrow(&inner.node)
            }

            Expr::Deref(inner) => {
                // Dereference the pointer
                self.compile_deref(&inner.node)
            }

            Expr::Await(inner) => {
                let task_ty = self.infer_expr_type(&inner.node, &[]);
                let Type::Task(inner_ty) = task_ty else {
                    return Err(CodegenError::new(format!(
                        "'await' can only be used on Task types, got {}",
                        Self::format_diagnostic_type(&task_ty)
                    )));
                };

                let task = self.compile_expr(&inner.node)?;
                if !task.is_pointer_value() {
                    return Err(CodegenError::new("await expects lowered Task<T> value"));
                }
                self.await_task(task.into_pointer_value(), &inner_ty)
            }

            Expr::AsyncBlock(body) => self.compile_async_block(body, None),

            Expr::Require { condition, message } => {
                // Compile require(condition) as an assert
                let cond = self.compile_condition_expr(&condition.node)?;

                let current_fn = self
                    .current_function
                    .ok_or(CodegenError::new("require outside of function"))?;

                let assert_block = self.context.append_basic_block(current_fn, "require.ok");
                let fail_block = self.context.append_basic_block(current_fn, "require.fail");

                self.builder
                    .build_conditional_branch(cond, assert_block, fail_block)
                    .unwrap();

                // Fail block - call abort or print message
                self.builder.position_at_end(fail_block);
                if let Some(msg) = message {
                    let msg_ty = self.infer_builtin_argument_type(&msg.node);
                    if !matches!(msg_ty, Type::String) {
                        return Err(CodegenError::new(format!(
                            "require() message must be String, got {}",
                            Self::format_diagnostic_type(&msg_ty)
                        )));
                    }
                    // Print the error message
                    let msg_spanned = Spanned::new(msg.node.clone(), msg.span.clone());
                    self.compile_print(&[msg_spanned], true)?;
                }
                // Call exit(1) or abort
                if let Some(exit_fn) = self.module.get_function("exit") {
                    self.builder
                        .build_call(
                            exit_fn,
                            &[self.context.i32_type().const_int(1, false).into()],
                            "exit",
                        )
                        .unwrap();
                }
                self.builder.build_unreachable().unwrap();

                // Continue in assert block
                self.builder.position_at_end(assert_block);
                Ok(self.context.i8_type().const_int(0, false).into())
            }

            Expr::Range {
                start,
                end,
                inclusive,
            } => {
                let start_val = if let Some(s) = start {
                    self.compile_expr_with_expected_type(&s.node, &Type::Integer)?
                } else {
                    self.context.i64_type().const_int(0, false).into()
                };
                let end_val = if let Some(e) = end {
                    self.compile_expr_with_expected_type(&e.node, &Type::Integer)?
                } else {
                    self.context.i64_type().const_int(0, false).into()
                };
                let step = self.context.i64_type().const_int(1, false).into();
                let end_val = if *inclusive {
                    let incremented = self
                        .builder
                        .build_int_add(
                            end_val.into_int_value(),
                            self.context.i64_type().const_int(1, false),
                            "range_inclusive_end",
                        )
                        .unwrap();
                    incremented.into()
                } else {
                    end_val
                };
                Ok(self.create_range(start_val, end_val, step)?.into())
            }

            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => self.compile_if_expr(&condition.node, then_branch, else_branch.as_ref(), None),

            Expr::Block(body) => self.with_variable_scope(|this| {
                let mut result = this.context.i8_type().const_int(0, false).into();
                for stmt in body {
                    if let Stmt::Expr(expr) = &stmt.node {
                        result = this.compile_expr(&expr.node)?;
                    } else {
                        this.compile_stmt(&stmt.node)?;
                    }
                }
                Ok(result)
            }),
        }
    }

    pub(crate) fn compile_expr_with_expected_type(
        &mut self,
        expr: &Expr,
        expected_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
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
        if let Expr::IfExpr {
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
                .unwrap()
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
            .unwrap()
            .into_struct_value();
        closure = self
            .builder
            .build_insert_value(closure, ptr_type.const_null(), 1, "enum_variant_env")
            .unwrap()
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
                .unwrap()
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
                .unwrap()
                .into_struct_value();
            closure = self
                .builder
                .build_insert_value(closure, null_env, 1, "env")
                .unwrap()
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
            .unwrap();
        let result = self.extract_call_value(call);
        let adapted = self.adapt_function_adapter_return(
            result?,
            actual_ret.as_ref(),
            expected_ret.as_ref(),
            "fn_adapter_return",
        )?;
        self.builder.build_return(Some(&adapted)).unwrap();

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
            .unwrap()
            .into_struct_value();
        closure = self
            .builder
            .build_insert_value(closure, ptr_type.const_null(), 1, "env")
            .unwrap()
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

            self.current_function = Some(wrapper_fn);
            self.current_return_type = Some(function_ty.clone());
            self.variables.clear();

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
            .unwrap()
            .into_struct_value();
        closure = self
            .builder
            .build_insert_value(closure, ptr_type.const_null(), 1, "builtin_env")
            .unwrap()
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
        let malloc = self.get_or_declare_malloc();
        let env_size = env_struct_ty
            .size_of()
            .ok_or_else(|| CodegenError::new("failed to size function adapter env"))?;
        let env_alloc = self
            .builder
            .build_call(malloc, &[env_size.into()], "fn_adapter_env_alloc")
            .map_err(|e| {
                CodegenError::new(format!(
                    "failed to call malloc for function adapter env: {e}"
                ))
            })?;
        let env_ptr =
            self.extract_call_pointer_value(env_alloc, "malloc failed for function adapter env")?;
        let stored_closure_ptr = unsafe {
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
        let closure_field_ptr = unsafe {
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
        self.reject_builtin_heap_wrapper_specialization_mismatch(expected_ty, actual_ty)?;

        if !self.type_contains_active_generic_placeholder(expected_ty)
            && !self.type_contains_active_generic_placeholder(actual_ty)
            && value.get_type() != self.llvm_type(expected_ty)
        {
            return Err(Self::type_mismatch_error(expected_ty, actual_ty));
        }

        Ok(())
    }

    fn reject_builtin_heap_wrapper_specialization_mismatch(
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

        let expected_key = Self::builtin_heap_wrapper_specialization_key(&expected);
        let actual_key = Self::builtin_heap_wrapper_specialization_key(&actual);
        if (expected_key.is_some() || actual_key.is_some()) && expected_key != actual_key {
            return Err(Self::type_mismatch_error(&expected, &actual));
        }

        Ok(())
    }

    fn builtin_heap_wrapper_specialization_key(ty: &Type) -> Option<String> {
        match ty {
            Type::Named(name) if name.contains("__spec__") => {
                let (base_name, _) = name.split_once("__spec__")?;
                matches!(base_name, "Box" | "Rc" | "Arc").then(|| name.clone())
            }
            Type::Generic(name, args) if matches!(name.as_str(), "Box" | "Rc" | "Arc") => {
                Some(Self::generic_class_spec_name(name, args))
            }
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
        let result = f(self);
        self.variables = saved_variables;
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
                    .unwrap()
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
                    .unwrap()
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

    fn compile_char_to_string(&mut self, codepoint: IntValue<'ctx>) -> Result<PointerValue<'ctx>> {
        let malloc = self.get_or_declare_malloc();
        let current_fn = self
            .current_function
            .ok_or_else(|| CodegenError::new("char-to-string used outside function"))?;
        let i8_type = self.context.i8_type();
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let buf_call = self
            .builder
            .build_call(
                malloc,
                &[i64_type.const_int(5, false).into()],
                "char_str_buf",
            )
            .unwrap();
        let buffer = self.extract_call_value(buf_call)?.into_pointer_value();

        let one_byte_bb = self.context.append_basic_block(current_fn, "char_str_one");
        let two_byte_bb = self.context.append_basic_block(current_fn, "char_str_two");
        let three_byte_bb = self
            .context
            .append_basic_block(current_fn, "char_str_three");
        let four_byte_bb = self.context.append_basic_block(current_fn, "char_str_four");
        let done_bb = self.context.append_basic_block(current_fn, "char_str_done");

        let is_one_byte = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                codepoint,
                i32_type.const_int(0x80, false),
                "char_str_is_one_byte",
            )
            .unwrap();
        let is_two_byte = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                codepoint,
                i32_type.const_int(0x800, false),
                "char_str_is_two_byte",
            )
            .unwrap();
        let is_three_byte = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                codepoint,
                i32_type.const_int(0x10000, false),
                "char_str_is_three_byte",
            )
            .unwrap();
        let not_one_byte_bb = self
            .context
            .append_basic_block(current_fn, "char_str_not_one");
        let not_two_byte_bb = self
            .context
            .append_basic_block(current_fn, "char_str_not_two");

        self.builder
            .build_conditional_branch(is_one_byte, one_byte_bb, not_one_byte_bb)
            .unwrap();

        self.builder.position_at_end(not_one_byte_bb);
        self.builder
            .build_conditional_branch(is_two_byte, two_byte_bb, not_two_byte_bb)
            .unwrap();

        self.builder.position_at_end(not_two_byte_bb);
        self.builder
            .build_conditional_branch(is_three_byte, three_byte_bb, four_byte_bb)
            .unwrap();

        self.builder.position_at_end(one_byte_bb);
        let byte0 = self
            .builder
            .build_int_truncate(codepoint, i8_type, "char_str_b0")
            .unwrap();
        let byte0_ptr = unsafe {
            self.builder
                .build_gep(i8_type, buffer, &[i64_type.const_zero()], "char_str_b0_ptr")
                .unwrap()
        };
        let byte1_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    buffer,
                    &[i64_type.const_int(1, false)],
                    "char_str_term1_ptr",
                )
                .unwrap()
        };
        self.builder.build_store(byte0_ptr, byte0).unwrap();
        self.builder
            .build_store(byte1_ptr, i8_type.const_zero())
            .unwrap();
        self.builder.build_unconditional_branch(done_bb).unwrap();

        self.builder.position_at_end(two_byte_bb);
        let top5 = self
            .builder
            .build_right_shift(
                codepoint,
                i32_type.const_int(6, false),
                false,
                "char_str_top5",
            )
            .unwrap();
        let byte0 = self
            .builder
            .build_or(top5, i32_type.const_int(0xC0, false), "char_str_two_b0")
            .unwrap();
        let low6 = self
            .builder
            .build_and(codepoint, i32_type.const_int(0x3F, false), "char_str_low6")
            .unwrap();
        let byte1 = self
            .builder
            .build_or(low6, i32_type.const_int(0x80, false), "char_str_two_b1")
            .unwrap();
        for (idx, byte) in [(0u64, byte0), (1u64, byte1)] {
            let ptr = unsafe {
                self.builder
                    .build_gep(
                        i8_type,
                        buffer,
                        &[i64_type.const_int(idx, false)],
                        "char_str_ptr",
                    )
                    .unwrap()
            };
            let stored = self
                .builder
                .build_int_truncate(byte, i8_type, "char_str_byte")
                .unwrap();
            self.builder.build_store(ptr, stored).unwrap();
        }
        let term_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    buffer,
                    &[i64_type.const_int(2, false)],
                    "char_str_term2_ptr",
                )
                .unwrap()
        };
        self.builder
            .build_store(term_ptr, i8_type.const_zero())
            .unwrap();
        self.builder.build_unconditional_branch(done_bb).unwrap();

        self.builder.position_at_end(three_byte_bb);
        let top4 = self
            .builder
            .build_right_shift(
                codepoint,
                i32_type.const_int(12, false),
                false,
                "char_str_top4",
            )
            .unwrap();
        let byte0 = self
            .builder
            .build_or(top4, i32_type.const_int(0xE0, false), "char_str_three_b0")
            .unwrap();
        let mid6 = self
            .builder
            .build_and(
                self.builder
                    .build_right_shift(
                        codepoint,
                        i32_type.const_int(6, false),
                        false,
                        "char_str_mid_shift",
                    )
                    .unwrap(),
                i32_type.const_int(0x3F, false),
                "char_str_mid6",
            )
            .unwrap();
        let byte1 = self
            .builder
            .build_or(mid6, i32_type.const_int(0x80, false), "char_str_three_b1")
            .unwrap();
        let low6 = self
            .builder
            .build_and(
                codepoint,
                i32_type.const_int(0x3F, false),
                "char_str_three_low6",
            )
            .unwrap();
        let byte2 = self
            .builder
            .build_or(low6, i32_type.const_int(0x80, false), "char_str_three_b2")
            .unwrap();
        for (idx, byte) in [(0u64, byte0), (1u64, byte1), (2u64, byte2)] {
            let ptr = unsafe {
                self.builder
                    .build_gep(
                        i8_type,
                        buffer,
                        &[i64_type.const_int(idx, false)],
                        "char_str_ptr",
                    )
                    .unwrap()
            };
            let stored = self
                .builder
                .build_int_truncate(byte, i8_type, "char_str_byte")
                .unwrap();
            self.builder.build_store(ptr, stored).unwrap();
        }
        let term_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    buffer,
                    &[i64_type.const_int(3, false)],
                    "char_str_term3_ptr",
                )
                .unwrap()
        };
        self.builder
            .build_store(term_ptr, i8_type.const_zero())
            .unwrap();
        self.builder.build_unconditional_branch(done_bb).unwrap();

        self.builder.position_at_end(four_byte_bb);
        let top3 = self
            .builder
            .build_right_shift(
                codepoint,
                i32_type.const_int(18, false),
                false,
                "char_str_top3",
            )
            .unwrap();
        let byte0 = self
            .builder
            .build_or(top3, i32_type.const_int(0xF0, false), "char_str_four_b0")
            .unwrap();
        let high6 = self
            .builder
            .build_and(
                self.builder
                    .build_right_shift(
                        codepoint,
                        i32_type.const_int(12, false),
                        false,
                        "char_str_high_shift",
                    )
                    .unwrap(),
                i32_type.const_int(0x3F, false),
                "char_str_high6",
            )
            .unwrap();
        let byte1 = self
            .builder
            .build_or(high6, i32_type.const_int(0x80, false), "char_str_four_b1")
            .unwrap();
        let mid6 = self
            .builder
            .build_and(
                self.builder
                    .build_right_shift(
                        codepoint,
                        i32_type.const_int(6, false),
                        false,
                        "char_str_four_mid_shift",
                    )
                    .unwrap(),
                i32_type.const_int(0x3F, false),
                "char_str_four_mid6",
            )
            .unwrap();
        let byte2 = self
            .builder
            .build_or(mid6, i32_type.const_int(0x80, false), "char_str_four_b2")
            .unwrap();
        let low6 = self
            .builder
            .build_and(
                codepoint,
                i32_type.const_int(0x3F, false),
                "char_str_four_low6",
            )
            .unwrap();
        let byte3 = self
            .builder
            .build_or(low6, i32_type.const_int(0x80, false), "char_str_four_b3")
            .unwrap();
        for (idx, byte) in [(0u64, byte0), (1u64, byte1), (2u64, byte2), (3u64, byte3)] {
            let ptr = unsafe {
                self.builder
                    .build_gep(
                        i8_type,
                        buffer,
                        &[i64_type.const_int(idx, false)],
                        "char_str_ptr",
                    )
                    .unwrap()
            };
            let stored = self
                .builder
                .build_int_truncate(byte, i8_type, "char_str_byte")
                .unwrap();
            self.builder.build_store(ptr, stored).unwrap();
        }
        let term_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    buffer,
                    &[i64_type.const_int(4, false)],
                    "char_str_term4_ptr",
                )
                .unwrap()
        };
        self.builder
            .build_store(term_ptr, i8_type.const_zero())
            .unwrap();
        self.builder.build_unconditional_branch(done_bb).unwrap();

        self.builder.position_at_end(done_bb);
        Ok(buffer)
    }

    fn compile_concat_display_strings(
        &mut self,
        left: PointerValue<'ctx>,
        right: PointerValue<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>> {
        let strlen_fn = self.get_or_declare_strlen();
        let malloc = self.get_or_declare_malloc();
        let strcpy_fn = self.get_or_declare_strcpy();
        let strcat_fn = self.get_or_declare_strcat();

        let left_len_call = self
            .builder
            .build_call(strlen_fn, &[left.into()], &format!("{name}_len1"))
            .unwrap();
        let left_len = self.extract_call_value(left_len_call)?.into_int_value();
        let right_len_call = self
            .builder
            .build_call(strlen_fn, &[right.into()], &format!("{name}_len2"))
            .unwrap();
        let right_len = self.extract_call_value(right_len_call)?.into_int_value();
        let total_len = self
            .builder
            .build_int_add(left_len, right_len, &format!("{name}_total"))
            .unwrap();
        let buffer_size = self
            .builder
            .build_int_add(
                total_len,
                self.context.i64_type().const_int(1, false),
                &format!("{name}_bufsize"),
            )
            .unwrap();
        let buffer_call = self
            .builder
            .build_call(malloc, &[buffer_size.into()], &format!("{name}_buf"))
            .unwrap();
        let buffer = self.extract_call_value(buffer_call)?.into_pointer_value();
        self.builder
            .build_call(strcpy_fn, &[buffer.into(), left.into()], "")
            .unwrap();
        self.builder
            .build_call(strcat_fn, &[buffer.into(), right.into()], "")
            .unwrap();
        Ok(buffer)
    }

    fn compile_value_to_display_string(
        &mut self,
        value: BasicValueEnum<'ctx>,
        source_ty: &Type,
    ) -> Result<PointerValue<'ctx>> {
        let display_ty = self.deref_codegen_type(source_ty).clone();
        match &display_ty {
            Type::String => Ok(value.into_pointer_value()),
            Type::Boolean => {
                let int_val = value.into_int_value();
                let true_s = self.context.const_string(b"true", true);
                let false_s = self.context.const_string(b"false", true);

                let t_name = format!("str.bool.true.{}", self.str_counter);
                let f_name = format!("str.bool.false.{}", self.str_counter);
                self.str_counter += 1;

                let t_glob = self.module.add_global(true_s.get_type(), None, &t_name);
                t_glob.set_linkage(Linkage::Private);
                t_glob.set_initializer(&true_s);
                t_glob.set_constant(true);

                let f_glob = self.module.add_global(false_s.get_type(), None, &f_name);
                f_glob.set_linkage(Linkage::Private);
                f_glob.set_initializer(&false_s);
                f_glob.set_constant(true);

                Ok(self
                    .builder
                    .build_select(
                        int_val,
                        t_glob.as_pointer_value(),
                        f_glob.as_pointer_value(),
                        "bool_str",
                    )
                    .unwrap()
                    .into_pointer_value())
            }
            Type::None => {
                let none_s = self.context.const_string(b"None", true);
                let name = format!("str.none.{}", self.str_counter);
                self.str_counter += 1;
                let global = self.module.add_global(none_s.get_type(), None, &name);
                global.set_linkage(Linkage::Private);
                global.set_initializer(&none_s);
                global.set_constant(true);
                Ok(global.as_pointer_value())
            }
            Type::Option(inner_ty) => {
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Option display formatting used outside function"))?;
                let option_ptr = self.materialize_value_pointer_for_type(
                    value,
                    &display_ty,
                    "display_option_tmp",
                )?;
                let llvm_inner_ty = self.llvm_type(inner_ty);
                let option_struct_type = self
                    .context
                    .struct_type(&[self.context.i8_type().into(), llvm_inner_ty], false);
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_zero();
                let one = i32_type.const_int(1, false);
                let tag_ptr = unsafe {
                    self.builder
                        .build_gep(
                            option_struct_type.as_basic_type_enum(),
                            option_ptr,
                            &[zero, zero],
                            "display_option_tag_ptr",
                        )
                        .unwrap()
                };
                let tag = self
                    .builder
                    .build_load(self.context.i8_type(), tag_ptr, "display_option_tag")
                    .unwrap()
                    .into_int_value();
                let is_some = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        tag,
                        self.context.i8_type().const_int(1, false),
                        "display_option_is_some",
                    )
                    .unwrap();

                let some_bb = self.context.append_basic_block(current_fn, "display_option_some");
                let none_bb = self.context.append_basic_block(current_fn, "display_option_none");
                let merge_bb = self.context.append_basic_block(current_fn, "display_option_merge");

                self.builder
                    .build_conditional_branch(is_some, some_bb, none_bb)
                    .unwrap();

                self.builder.position_at_end(some_bb);
                let value_ptr = unsafe {
                    self.builder
                        .build_gep(
                            option_struct_type.as_basic_type_enum(),
                            option_ptr,
                            &[zero, one],
                            "display_option_value_ptr",
                        )
                        .unwrap()
                };
                let inner_value = self
                    .builder
                    .build_load(llvm_inner_ty, value_ptr, "display_option_value")
                    .unwrap();
                let inner_display =
                    self.compile_value_to_display_string(inner_value, inner_ty.as_ref())?;
                let some_prefix = self
                    .builder
                    .build_global_string_ptr(
                        "Some(",
                        &format!("display_option_some_prefix_{}", self.str_counter),
                    )
                    .unwrap();
                self.str_counter += 1;
                let some_suffix = self
                    .builder
                    .build_global_string_ptr(
                        ")",
                        &format!("display_option_some_suffix_{}", self.str_counter),
                    )
                    .unwrap();
                self.str_counter += 1;
                let prefixed = self.compile_concat_display_strings(
                    some_prefix.as_pointer_value(),
                    inner_display,
                    "display_option_prefixed",
                )?;
                let some_display = self.compile_concat_display_strings(
                    prefixed,
                    some_suffix.as_pointer_value(),
                    "display_option_joined",
                )?;
                let some_end = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::new("Option display some block missing predecessor"))?;
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(none_bb);
                let none_display = self
                    .builder
                    .build_global_string_ptr(
                        "None",
                        &format!("display_option_none_{}", self.str_counter),
                    )
                    .unwrap();
                self.str_counter += 1;
                let none_end = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::new("Option display none block missing predecessor"))?;
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(merge_bb);
                let display_phi = self
                    .builder
                    .build_phi(self.context.ptr_type(AddressSpace::default()), "display_option")
                    .unwrap();
                display_phi.add_incoming(&[
                    (&some_display, some_end),
                    (&none_display.as_pointer_value(), none_end),
                ]);
                Ok(display_phi.as_basic_value().into_pointer_value())
            }
            Type::Result(ok_ty, err_ty) => {
                let current_fn = self.current_function.ok_or_else(|| {
                    CodegenError::new("Result display formatting used outside function")
                })?;
                let result_ptr =
                    self.materialize_value_pointer_for_type(value, &display_ty, "display_result_tmp")?;
                let ok_llvm = self.llvm_type(ok_ty);
                let err_llvm = self.llvm_type(err_ty);
                let result_struct_type = self.context.struct_type(
                    &[self.context.i8_type().into(), ok_llvm, err_llvm],
                    false,
                );
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_zero();
                let one = i32_type.const_int(1, false);
                let two = i32_type.const_int(2, false);
                let tag_ptr = unsafe {
                    self.builder
                        .build_gep(
                            result_struct_type.as_basic_type_enum(),
                            result_ptr,
                            &[zero, zero],
                            "display_result_tag_ptr",
                        )
                        .unwrap()
                };
                let tag = self
                    .builder
                    .build_load(self.context.i8_type(), tag_ptr, "display_result_tag")
                    .unwrap()
                    .into_int_value();
                let is_ok = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        tag,
                        self.context.i8_type().const_int(1, false),
                        "display_result_is_ok",
                    )
                    .unwrap();

                let ok_bb = self.context.append_basic_block(current_fn, "display_result_ok");
                let err_bb = self
                    .context
                    .append_basic_block(current_fn, "display_result_error");
                let merge_bb = self
                    .context
                    .append_basic_block(current_fn, "display_result_merge");

                self.builder
                    .build_conditional_branch(is_ok, ok_bb, err_bb)
                    .unwrap();

                self.builder.position_at_end(ok_bb);
                let ok_ptr = unsafe {
                    self.builder
                        .build_gep(
                            result_struct_type.as_basic_type_enum(),
                            result_ptr,
                            &[zero, one],
                            "display_result_ok_ptr",
                        )
                        .unwrap()
                };
                let ok_value = self
                    .builder
                    .build_load(ok_llvm, ok_ptr, "display_result_ok_value")
                    .unwrap();
                let ok_display = self.compile_value_to_display_string(ok_value, ok_ty.as_ref())?;
                let ok_prefix = self
                    .builder
                    .build_global_string_ptr(
                        "Ok(",
                        &format!("display_result_ok_prefix_{}", self.str_counter),
                    )
                    .unwrap();
                self.str_counter += 1;
                let ok_suffix = self
                    .builder
                    .build_global_string_ptr(
                        ")",
                        &format!("display_result_ok_suffix_{}", self.str_counter),
                    )
                    .unwrap();
                self.str_counter += 1;
                let ok_prefixed = self.compile_concat_display_strings(
                    ok_prefix.as_pointer_value(),
                    ok_display,
                    "display_result_ok_prefixed",
                )?;
                let ok_joined = self.compile_concat_display_strings(
                    ok_prefixed,
                    ok_suffix.as_pointer_value(),
                    "display_result_ok_joined",
                )?;
                let ok_end = self.builder.get_insert_block().ok_or_else(|| {
                    CodegenError::new("Result display ok block missing predecessor")
                })?;
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(err_bb);
                let err_ptr = unsafe {
                    self.builder
                        .build_gep(
                            result_struct_type.as_basic_type_enum(),
                            result_ptr,
                            &[zero, two],
                            "display_result_error_ptr",
                        )
                        .unwrap()
                };
                let err_value = self
                    .builder
                    .build_load(err_llvm, err_ptr, "display_result_error_value")
                    .unwrap();
                let err_display =
                    self.compile_value_to_display_string(err_value, err_ty.as_ref())?;
                let err_prefix = self
                    .builder
                    .build_global_string_ptr(
                        "Error(",
                        &format!("display_result_error_prefix_{}", self.str_counter),
                    )
                    .unwrap();
                self.str_counter += 1;
                let err_suffix = self
                    .builder
                    .build_global_string_ptr(
                        ")",
                        &format!("display_result_error_suffix_{}", self.str_counter),
                    )
                    .unwrap();
                self.str_counter += 1;
                let err_prefixed = self.compile_concat_display_strings(
                    err_prefix.as_pointer_value(),
                    err_display,
                    "display_result_error_prefixed",
                )?;
                let err_joined = self.compile_concat_display_strings(
                    err_prefixed,
                    err_suffix.as_pointer_value(),
                    "display_result_error_joined",
                )?;
                let err_end = self.builder.get_insert_block().ok_or_else(|| {
                    CodegenError::new("Result display error block missing predecessor")
                })?;
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(merge_bb);
                let display_phi = self
                    .builder
                    .build_phi(self.context.ptr_type(AddressSpace::default()), "display_result")
                    .unwrap();
                display_phi.add_incoming(&[(&ok_joined, ok_end), (&err_joined, err_end)]);
                Ok(display_phi.as_basic_value().into_pointer_value())
            }
            Type::Char => self.compile_char_to_string(value.into_int_value()),
            Type::Integer | Type::Float => {
                let sprintf = self.get_or_declare_sprintf();
                let malloc = self.get_or_declare_malloc();
                let buffer_call = self
                    .builder
                    .build_call(
                        malloc,
                        &[self.context.i64_type().const_int(64, false).into()],
                        "display_buf",
                    )
                    .unwrap();
                let buffer = self.extract_call_value(buffer_call)?.into_pointer_value();

                let (fmt, print_arg): (&str, BasicMetadataValueEnum) = if value.is_float_value() {
                    ("%f", value.into())
                } else if value.is_int_value() {
                    if matches!(display_ty, Type::Float) {
                        let promoted = self
                            .builder
                            .build_signed_int_to_float(
                                value.into_int_value(),
                                self.context.f64_type(),
                                "display_f64",
                            )
                            .unwrap();
                        ("%f", promoted.into())
                    } else {
                        let promoted = self
                            .builder
                            .build_int_s_extend(
                                value.into_int_value(),
                                self.context.i64_type(),
                                "display_i64",
                            )
                            .unwrap();
                        ("%lld", promoted.into())
                    }
                } else {
                    return Err(CodegenError::new(format!(
                        "display formatting expected numeric runtime value for {}, got LLVM value kind mismatch",
                        Self::format_type_string(&display_ty)
                    )));
                };

                let fmt_val = self.context.const_string(fmt.as_bytes(), true);
                let fmt_name = format!("fmt.{}", self.str_counter);
                self.str_counter += 1;
                let fmt_global = self.module.add_global(fmt_val.get_type(), None, &fmt_name);
                fmt_global.set_linkage(Linkage::Private);
                fmt_global.set_initializer(&fmt_val);
                self.builder
                    .build_call(
                        sprintf,
                        &[buffer.into(), fmt_global.as_pointer_value().into(), print_arg],
                        "sprintf",
                    )
                    .unwrap();
                Ok(buffer)
            }
            _ => Err(CodegenError::new(format!(
                "display formatting currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got {}",
                Self::format_diagnostic_name(&Self::format_type_string(&display_ty))
            ))),
        }
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
            .unwrap();

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
            .unwrap();
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
            .unwrap();
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
                .unwrap();
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

    pub(crate) fn compile_non_negative_integer_index_expr(
        &mut self,
        expr: &Expr,
        negative_diagnostic: &str,
    ) -> Result<IntValue<'ctx>> {
        if matches!(
            TypeChecker::eval_numeric_const_expr(expr),
            Some(NumericConst::Integer(value)) if value < 0
        ) {
            return Err(CodegenError::new(negative_diagnostic));
        }
        self.compile_integer_index_expr(expr)
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
            Expr::IfExpr {
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
                            this.builder.build_not(eq, "ne").unwrap()
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
                    self.builder.build_not(eq, "ne").unwrap()
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
        self.compile_binary_values(op, lhs, rhs, &left_ty, &right_ty)
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

        if left_ty.is_numeric()
            && right_ty.is_numeric()
            && (matches!(left_ty, Type::Float) || matches!(right_ty, Type::Float))
        {
            let l = if lhs.is_float_value() {
                lhs.into_float_value()
            } else {
                self.builder
                    .build_signed_int_to_float(lhs.into_int_value(), self.context.f64_type(), "lf")
                    .unwrap()
            };
            let r = if rhs.is_float_value() {
                rhs.into_float_value()
            } else {
                self.builder
                    .build_signed_int_to_float(rhs.into_int_value(), self.context.f64_type(), "rf")
                    .unwrap()
            };

            let result = match op {
                BinOp::Add => self.builder.build_float_add(l, r, "fadd").unwrap().into(),
                BinOp::Sub => self.builder.build_float_sub(l, r, "fsub").unwrap().into(),
                BinOp::Mul => self.builder.build_float_mul(l, r, "fmul").unwrap().into(),
                BinOp::Div => self.builder.build_float_div(l, r, "fdiv").unwrap().into(),
                BinOp::Mod => self.builder.build_float_rem(l, r, "frem").unwrap().into(),
                BinOp::Eq => self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, "feq")
                    .unwrap()
                    .into(),
                BinOp::NotEq => self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, l, r, "fne")
                    .unwrap()
                    .into(),
                BinOp::Lt => self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, l, r, "flt")
                    .unwrap()
                    .into(),
                BinOp::LtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, l, r, "fle")
                    .unwrap()
                    .into(),
                BinOp::Gt => self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, l, r, "fgt")
                    .unwrap()
                    .into(),
                BinOp::GtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, l, r, "fge")
                    .unwrap()
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
                self.guard_nonzero_integer_divisor(r, message, global_name)?;
            }

            let result = match op {
                BinOp::Add => self.builder.build_int_add(l, r, "add").unwrap(),
                BinOp::Sub => self.builder.build_int_sub(l, r, "sub").unwrap(),
                BinOp::Mul => self.builder.build_int_mul(l, r, "mul").unwrap(),
                BinOp::Div => self.builder.build_int_signed_div(l, r, "div").unwrap(),
                BinOp::Mod => self.builder.build_int_signed_rem(l, r, "mod").unwrap(),
                BinOp::Eq => self
                    .builder
                    .build_int_compare(IntPredicate::EQ, l, r, "eq")
                    .unwrap(),
                BinOp::NotEq => self
                    .builder
                    .build_int_compare(IntPredicate::NE, l, r, "ne")
                    .unwrap(),
                BinOp::Lt => self
                    .builder
                    .build_int_compare(IntPredicate::SLT, l, r, "lt")
                    .unwrap(),
                BinOp::LtEq => self
                    .builder
                    .build_int_compare(IntPredicate::SLE, l, r, "le")
                    .unwrap(),
                BinOp::Gt => self
                    .builder
                    .build_int_compare(IntPredicate::SGT, l, r, "gt")
                    .unwrap(),
                BinOp::GtEq => self
                    .builder
                    .build_int_compare(IntPredicate::SGE, l, r, "ge")
                    .unwrap(),
                BinOp::And => self.builder.build_and(l, r, "and").unwrap(),
                BinOp::Or => self.builder.build_or(l, r, "or").unwrap(),
            };
            return Ok(result.into());
        }

        // Float operations
        if lhs.is_float_value() && rhs.is_float_value() {
            let l = lhs.into_float_value();
            let r = rhs.into_float_value();

            let result = match op {
                BinOp::Add => self.builder.build_float_add(l, r, "fadd").unwrap().into(),
                BinOp::Sub => self.builder.build_float_sub(l, r, "fsub").unwrap().into(),
                BinOp::Mul => self.builder.build_float_mul(l, r, "fmul").unwrap().into(),
                BinOp::Div => self.builder.build_float_div(l, r, "fdiv").unwrap().into(),
                BinOp::Mod => self.builder.build_float_rem(l, r, "frem").unwrap().into(),
                BinOp::Eq => self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, "feq")
                    .unwrap()
                    .into(),
                BinOp::NotEq => self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, l, r, "fne")
                    .unwrap()
                    .into(),
                BinOp::Lt => self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, l, r, "flt")
                    .unwrap()
                    .into(),
                BinOp::LtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, l, r, "fle")
                    .unwrap()
                    .into(),
                BinOp::Gt => self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, l, r, "fgt")
                    .unwrap()
                    .into(),
                BinOp::GtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, l, r, "fge")
                    .unwrap()
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
            let malloc = self.get_or_declare_malloc();
            let strcpy_fn = self.get_or_declare_strcpy();
            let strcat_fn = self.get_or_declare_strcat();
            let s1 = lhs.into_pointer_value();
            let s2 = rhs.into_pointer_value();

            let len1_call = self
                .builder
                .build_call(strlen_fn, &[s1.into()], "len1")
                .unwrap();
            let len1 = self.extract_call_value(len1_call)?.into_int_value();
            let len2_call = self
                .builder
                .build_call(strlen_fn, &[s2.into()], "len2")
                .unwrap();
            let len2 = self.extract_call_value(len2_call)?.into_int_value();
            let total_len = self.builder.build_int_add(len1, len2, "total").unwrap();
            let buffer_size = self
                .builder
                .build_int_add(
                    total_len,
                    self.context.i64_type().const_int(1, false),
                    "bufsize",
                )
                .unwrap();
            let buffer_call = self
                .builder
                .build_call(malloc, &[buffer_size.into()], "buf")
                .unwrap();
            let buffer = self.extract_call_value(buffer_call)?.into_pointer_value();
            self.builder
                .build_call(strcpy_fn, &[buffer.into(), s1.into()], "")
                .unwrap();
            self.builder
                .build_call(strcat_fn, &[buffer.into(), s2.into()], "")
                .unwrap();
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
                    .unwrap()
            } else {
                self.builder
                    .build_int_compare(
                        IntPredicate::NE,
                        cmp,
                        self.context.i32_type().const_zero(),
                        "str_ne",
                    )
                    .unwrap()
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
                if left_ty.is_numeric() && right_ty.is_numeric() {
                    Ok(())
                } else {
                    Err(CodegenError::new(format!(
                        "Comparison requires numeric types, got {} and {}",
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
            .unwrap();
        let ok_block = self
            .context
            .append_basic_block(current_fn, &format!("{block_prefix}_ok"));
        let error_block = self
            .context
            .append_basic_block(current_fn, &format!("{block_prefix}_error"));
        self.builder
            .build_conditional_branch(is_zero, error_block, ok_block)
            .unwrap();

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
                        .unwrap()
                        .into())
                } else if val.is_float_value() {
                    Ok(self
                        .builder
                        .build_float_neg(val.into_float_value(), "fneg")
                        .unwrap()
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
                    .unwrap()
                    .into())
            }
        }
    }

    pub fn compile_call(
        &mut self,
        callee: &Expr,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let resolved_ident = if let Expr::Ident(name) = callee {
            if self.variables.contains_key(name) {
                name.clone()
            } else {
                self.resolve_function_alias(name)
            }
        } else {
            String::new()
        };

        // Check for built-in functions
        if let Expr::Ident(name) = callee {
            let builtin_name = if resolved_ident.is_empty() {
                name.as_str()
            } else {
                resolved_ident.as_str()
            };
            if !self.variables.contains_key(name)
                && self
                    .resolve_alias_qualified_codegen_type_name(builtin_name)
                    .is_some_and(|resolved| self.classes.contains_key(&resolved))
            {
                return self.compile_construct(builtin_name, args);
            }
            if builtin_name == "println" || builtin_name == "print" {
                return self.compile_print(args, builtin_name == "println");
            }

            // Standard library functions
            if Self::is_stdlib_function(builtin_name) {
                if let Some(result) = self.compile_stdlib_function(builtin_name, args)? {
                    return Ok(result);
                } else {
                    // Void stdlib function - return dummy value
                    return Ok(self.context.i8_type().const_int(0, false).into());
                }
            }
        }

        // Check for Option/Result static methods
        if let Expr::Field { object, field } = callee {
            if let Expr::Ident(type_name) = &object.node {
                let call_expr = Expr::Call {
                    callee: Box::new(Spanned::new(callee.clone(), Span::default())),
                    args: args.to_vec(),
                    type_args: Vec::new(),
                };
                let inferred_expr_ty = self.infer_expr_type(&call_expr, &[]);
                if let Some(canonical_builtin) =
                    builtin_exact_import_alias_canonical(&format!("{}.{}", type_name, field))
                {
                    match canonical_builtin {
                        "Option__some" => {
                            if args.len() != 1 {
                                return Err(CodegenError::new(
                                    "Option.some() requires exactly 1 argument",
                                ));
                            }
                            if let Type::Option(inner_ty) = &inferred_expr_ty {
                                let val = self.compile_expr_for_concrete_class_payload(
                                    &args[0].node,
                                    inner_ty,
                                )?;
                                return self.create_option_some_typed(val, inner_ty);
                            }
                            let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                            let val =
                                self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                            return self.create_option_some(val);
                        }
                        "Option__none" => {
                            if !args.is_empty() {
                                return Err(CodegenError::new(format!(
                                    "Option.none() expects 0 argument(s), got {}",
                                    args.len()
                                )));
                            }
                            if let Type::Option(inner_ty) = &inferred_expr_ty {
                                return self.create_option_none_typed(inner_ty);
                            }
                            return self.create_option_none();
                        }
                        "Result__ok" => {
                            if args.len() != 1 {
                                return Err(CodegenError::new(
                                    "Result.ok() requires exactly 1 argument",
                                ));
                            }
                            if let Type::Result(ok_ty, err_ty) = &inferred_expr_ty {
                                let val = self.compile_expr_for_concrete_class_payload(
                                    &args[0].node,
                                    ok_ty,
                                )?;
                                return self.create_result_ok_typed(val, ok_ty, err_ty);
                            }
                            let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                            let val =
                                self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                            return self.create_result_ok(val);
                        }
                        "Result__error" => {
                            if args.len() != 1 {
                                return Err(CodegenError::new(
                                    "Result.error() requires exactly 1 argument",
                                ));
                            }
                            if let Type::Result(ok_ty, err_ty) = &inferred_expr_ty {
                                let val = self.compile_expr_for_concrete_class_payload(
                                    &args[0].node,
                                    err_ty,
                                )?;
                                return self.create_result_error_typed(val, ok_ty, err_ty);
                            }
                            let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                            let val =
                                self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                            return self.create_result_error(val);
                        }
                        _ => {}
                    }
                }
                match (type_name.as_str(), field.as_str()) {
                    ("Option", "some") => {
                        if args.len() != 1 {
                            return Err(CodegenError::new(
                                "Option.some() requires exactly 1 argument",
                            ));
                        }
                        if let Type::Option(inner_ty) = &inferred_expr_ty {
                            let val = self
                                .compile_expr_for_concrete_class_payload(&args[0].node, inner_ty)?;
                            return self.create_option_some_typed(val, inner_ty);
                        }
                        let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                        let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                        return self.create_option_some(val);
                    }
                    ("Option", "none") => {
                        if !args.is_empty() {
                            return Err(CodegenError::new(format!(
                                "Option.none() expects 0 argument(s), got {}",
                                args.len()
                            )));
                        }
                        if let Type::Option(inner_ty) = &inferred_expr_ty {
                            return self.create_option_none_typed(inner_ty);
                        }
                        return self.create_option_none();
                    }
                    ("Result", "ok") => {
                        if args.len() != 1 {
                            return Err(CodegenError::new(
                                "Result.ok() requires exactly 1 argument",
                            ));
                        }
                        if let Type::Result(ok_ty, err_ty) = &inferred_expr_ty {
                            let val =
                                self.compile_expr_for_concrete_class_payload(&args[0].node, ok_ty)?;
                            return self.create_result_ok_typed(val, ok_ty, err_ty);
                        }
                        let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                        let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                        return self.create_result_ok(val);
                    }
                    ("Result", "error") => {
                        if args.len() != 1 {
                            return Err(CodegenError::new(
                                "Result.error() requires exactly 1 argument",
                            ));
                        }
                        if let Type::Result(ok_ty, err_ty) = &inferred_expr_ty {
                            let val = self
                                .compile_expr_for_concrete_class_payload(&args[0].node, err_ty)?;
                            return self.create_result_error_typed(val, ok_ty, err_ty);
                        }
                        let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                        let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                        return self.create_result_error(val);
                    }
                    _ => {}
                }
            }
        }

        // Check for enum variant constructors and module-qualified functions.
        if let Expr::Field { object, field } = callee {
            if let Expr::Ident(owner_name) = &object.node {
                let resolved_owner = self
                    .resolve_alias_qualified_codegen_type_name(owner_name)
                    .unwrap_or_else(|| self.resolve_module_alias(owner_name));
                let type_path = format!("{}.{}", resolved_owner, field);
                if let Some(resolved_type_name) =
                    self.resolve_alias_qualified_codegen_type_name(&type_path)
                {
                    if self.classes.contains_key(&resolved_type_name) {
                        return self.compile_construct(&type_path, args);
                    }
                }
                // Enum constructor: `MyEnum.Variant(...)`
                if let Some(enum_info) = self.enums.get(&resolved_owner) {
                    if let Some(variant_info) = enum_info.variants.get(field).cloned() {
                        if args.len() != variant_info.fields.len() {
                            return Err(CodegenError::new(format!(
                                "Enum variant '{}.{}' expects {} argument(s), got {}",
                                resolved_owner,
                                field,
                                variant_info.fields.len(),
                                args.len()
                            )));
                        }
                        let mut values = Vec::with_capacity(args.len());
                        for (arg, expected_ty) in args.iter().zip(variant_info.fields.iter()) {
                            values.push(
                                self.compile_expr_for_concrete_class_payload(
                                    &arg.node,
                                    expected_ty,
                                )?,
                            );
                        }
                        return self.build_enum_value(&resolved_owner, &variant_info, &values);
                    }
                }

                // Module dot syntax: Module.func(...) -> Module__func(...)
                let mangled = format!("{}__{}", resolved_owner, field);
                if let Some((func, func_ty)) = self.functions.get(&mangled).cloned() {
                    if let Type::Function(params, _) = &func_ty {
                        if args.len() != params.len() {
                            return Err(Self::function_call_arity_error(&func_ty, args.len()));
                        }
                    }
                    let mut compiled_args: Vec<BasicValueEnum> = vec![self
                        .context
                        .ptr_type(AddressSpace::default())
                        .const_null()
                        .into()];
                    if let Type::Function(params, _) = &func_ty {
                        for (arg, param_ty) in args.iter().zip(params.iter()) {
                            compiled_args.push(
                                self.compile_expr_for_concrete_class_payload(&arg.node, param_ty)?,
                            );
                        }
                    } else {
                        for arg in args {
                            compiled_args.push(self.compile_expr(&arg.node)?);
                        }
                    }
                    let args_meta: Vec<BasicMetadataValueEnum> =
                        compiled_args.iter().map(|a| (*a).into()).collect();
                    let call = self.builder.build_call(func, &args_meta, "call").unwrap();
                    return match call.try_as_basic_value() {
                        ValueKind::Basic(val) => Ok(val),
                        ValueKind::Instruction(_) => {
                            Ok(self.context.i8_type().const_int(0, false).into())
                        }
                    };
                }
                if let Some(candidate) = self.resolve_wildcard_import_module_function_candidate(
                    owner_name,
                    std::slice::from_ref(field),
                ) {
                    if let Some((func, func_ty)) = self.functions.get(&candidate).cloned() {
                        if let Type::Function(params, _) = &func_ty {
                            if args.len() != params.len() {
                                return Err(Self::function_call_arity_error(&func_ty, args.len()));
                            }
                        }
                        let mut compiled_args: Vec<BasicValueEnum> = vec![self
                            .context
                            .ptr_type(AddressSpace::default())
                            .const_null()
                            .into()];
                        if let Type::Function(params, _) = &func_ty {
                            for (arg, param_ty) in args.iter().zip(params.iter()) {
                                compiled_args.push(self.compile_expr_for_concrete_class_payload(
                                    &arg.node, param_ty,
                                )?);
                            }
                        } else {
                            for arg in args {
                                compiled_args.push(self.compile_expr(&arg.node)?);
                            }
                        }
                        let args_meta: Vec<BasicMetadataValueEnum> =
                            compiled_args.iter().map(|a| (*a).into()).collect();
                        let call = self.builder.build_call(func, &args_meta, "call").unwrap();
                        return match call.try_as_basic_value() {
                            ValueKind::Basic(val) => Ok(val),
                            ValueKind::Instruction(_) => {
                                Ok(self.context.i8_type().const_int(0, false).into())
                            }
                        };
                    }
                }
            }
        }

        // Nested module-style calls: A.X.f(...) -> A__X__f(...)
        if let Some(path_parts) = flatten_field_chain(callee) {
            if path_parts.len() >= 3 {
                let full_path = path_parts.join(".");
                if let Some(resolved_type_name) =
                    self.resolve_alias_qualified_codegen_type_name(&full_path)
                {
                    if self.classes.contains_key(&resolved_type_name) {
                        return self.compile_construct(&full_path, args);
                    }
                }

                let owner_source = path_parts[..path_parts.len() - 1].join(".");
                let variant_name = path_parts.last().cloned().unwrap_or_default();
                if let Some(resolved_owner) =
                    self.resolve_alias_qualified_codegen_type_name(&owner_source)
                {
                    if let Some(enum_info) = self.enums.get(&resolved_owner) {
                        if let Some(variant_info) = enum_info.variants.get(&variant_name).cloned() {
                            if args.len() != variant_info.fields.len() {
                                return Err(CodegenError::new(format!(
                                    "Enum variant '{}.{}' expects {} argument(s), got {}",
                                    owner_source,
                                    variant_name,
                                    variant_info.fields.len(),
                                    args.len()
                                )));
                            }
                            let mut values = Vec::with_capacity(args.len());
                            for (arg, expected_ty) in args.iter().zip(variant_info.fields.iter()) {
                                values.push(self.compile_expr_for_concrete_class_payload(
                                    &arg.node,
                                    expected_ty,
                                )?);
                            }
                            return self.build_enum_value(&resolved_owner, &variant_info, &values);
                        }
                    }
                }

                let candidate = path_parts.join("__");
                if let Some((func, func_ty)) = self.functions.get(&candidate).cloned() {
                    if let Type::Function(params, _) = &func_ty {
                        if args.len() != params.len() {
                            return Err(Self::function_call_arity_error(&func_ty, args.len()));
                        }
                    }
                    let mut compiled_args: Vec<BasicValueEnum> = vec![self
                        .context
                        .ptr_type(AddressSpace::default())
                        .const_null()
                        .into()];
                    if let Type::Function(params, _) = &func_ty {
                        for (arg, param_ty) in args.iter().zip(params.iter()) {
                            compiled_args.push(
                                self.compile_expr_for_concrete_class_payload(&arg.node, param_ty)?,
                            );
                        }
                    } else {
                        for arg in args {
                            compiled_args.push(self.compile_expr(&arg.node)?);
                        }
                    }
                    let args_meta: Vec<BasicMetadataValueEnum> =
                        compiled_args.iter().map(|a| (*a).into()).collect();
                    let call = self.builder.build_call(func, &args_meta, "call").unwrap();
                    return match call.try_as_basic_value() {
                        ValueKind::Basic(val) => Ok(val),
                        ValueKind::Instruction(_) => {
                            Ok(self.context.i8_type().const_int(0, false).into())
                        }
                    };
                }
                if let Some(candidate) = self.resolve_wildcard_import_module_function_candidate(
                    &path_parts[0],
                    &path_parts[1..],
                ) {
                    if let Some((func, func_ty)) = self.functions.get(&candidate).cloned() {
                        if let Type::Function(params, _) = &func_ty {
                            if args.len() != params.len() {
                                return Err(Self::function_call_arity_error(&func_ty, args.len()));
                            }
                        }
                        let mut compiled_args: Vec<BasicValueEnum> = vec![self
                            .context
                            .ptr_type(AddressSpace::default())
                            .const_null()
                            .into()];
                        if let Type::Function(params, _) = &func_ty {
                            for (arg, param_ty) in args.iter().zip(params.iter()) {
                                compiled_args.push(self.compile_expr_for_concrete_class_payload(
                                    &arg.node, param_ty,
                                )?);
                            }
                        } else {
                            for arg in args {
                                compiled_args.push(self.compile_expr(&arg.node)?);
                            }
                        }
                        let args_meta: Vec<BasicMetadataValueEnum> =
                            compiled_args.iter().map(|a| (*a).into()).collect();
                        let call = self.builder.build_call(func, &args_meta, "call").unwrap();
                        return match call.try_as_basic_value() {
                            ValueKind::Basic(val) => Ok(val),
                            ValueKind::Instruction(_) => {
                                Ok(self.context.i8_type().const_int(0, false).into())
                            }
                        };
                    }
                }
                if let Some(candidate) = self.resolve_import_alias_module_function_candidate(
                    &path_parts[0],
                    &path_parts[1..],
                ) {
                    if Self::is_supported_builtin_function_name(&candidate) {
                        match candidate.as_str() {
                            "Option__some" => {
                                if args.len() != 1 {
                                    return Err(CodegenError::new(
                                        "Option.some() requires exactly 1 argument",
                                    ));
                                }
                                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                                let value =
                                    self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
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
                                let value =
                                    self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                                return self.create_result_ok(value);
                            }
                            "Result__error" => {
                                if args.len() != 1 {
                                    return Err(CodegenError::new(
                                        "Result.error() requires exactly 1 argument",
                                    ));
                                }
                                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                                let value =
                                    self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                                return self.create_result_error(value);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Method call on object
        if let Expr::Field { object, field } = callee {
            let field_ty = self.infer_object_type(&object.node).and_then(|obj_ty| {
                let (class_name, generic_args) = self.unwrap_class_like_type(&obj_ty)?;
                let class_info = self.classes.get(&class_name)?;
                let field_ty = class_info.field_types.get(field)?.clone();
                if let Some(args) = generic_args {
                    if class_info.generic_params.len() == args.len() {
                        let bindings = class_info
                            .generic_params
                            .iter()
                            .cloned()
                            .zip(args)
                            .collect::<HashMap<_, _>>();
                        return Some(Self::substitute_type(&field_ty, &bindings));
                    }
                }
                Some(field_ty)
            });
            if let Some(field_ty) = field_ty {
                if let Type::Function(param_types, ret_type) = field_ty {
                    let compiled_callee = self.compile_expr(callee)?;
                    let (ptr, env_ptr) = if compiled_callee.is_struct_value() {
                        let closure_val = compiled_callee.into_struct_value();
                        let ptr = self
                            .builder
                            .build_extract_value(closure_val, 0, "fn_ptr")
                            .unwrap()
                            .into_pointer_value();
                        let env_ptr = self
                            .builder
                            .build_extract_value(closure_val, 1, "env_ptr")
                            .unwrap();
                        (ptr, env_ptr)
                    } else if compiled_callee.is_pointer_value() {
                        (
                            compiled_callee.into_pointer_value(),
                            self.context
                                .ptr_type(AddressSpace::default())
                                .const_null()
                                .into(),
                        )
                    } else {
                        return Err(CodegenError::new(format!(
                            "Function-valued field '{}': expected closure or function pointer, got {:?}",
                            field, compiled_callee
                        )));
                    };

                    let llvm_ret = self.llvm_type(&ret_type);
                    let mut llvm_params: Vec<BasicMetadataTypeEnum> =
                        vec![self.context.ptr_type(AddressSpace::default()).into()];
                    for p in &param_types {
                        llvm_params.push(self.llvm_type(p).into());
                    }

                    let fn_type = match llvm_ret {
                        BasicTypeEnum::IntType(i) => i.fn_type(&llvm_params, false),
                        BasicTypeEnum::FloatType(f) => f.fn_type(&llvm_params, false),
                        BasicTypeEnum::PointerType(p) => p.fn_type(&llvm_params, false),
                        BasicTypeEnum::StructType(s) => s.fn_type(&llvm_params, false),
                        _ => self.context.i8_type().fn_type(&llvm_params, false),
                    };

                    let mut compiled_args: Vec<BasicValueEnum> = vec![env_ptr];
                    if args.len() != param_types.len() {
                        return Err(Self::function_call_arity_error(
                            &Type::Function(param_types.clone(), ret_type.clone()),
                            args.len(),
                        ));
                    }
                    for (arg, param_ty) in args.iter().zip(param_types.iter()) {
                        compiled_args.push(
                            self.compile_expr_for_concrete_class_payload(&arg.node, param_ty)?,
                        );
                    }

                    let args_meta: Vec<BasicMetadataValueEnum> =
                        compiled_args.iter().map(|a| (*a).into()).collect();
                    let call = self
                        .builder
                        .build_indirect_call(fn_type, ptr, &args_meta, "call")
                        .unwrap();

                    return Ok(match call.try_as_basic_value() {
                        ValueKind::Basic(val) => val,
                        ValueKind::Instruction(_) => {
                            self.context.i8_type().const_int(0, false).into()
                        }
                    });
                }

                let _ = self.compile_expr(callee)?;
                return Err(Self::non_function_call_error(&field_ty));
            }

            // Check for File static methods
            if let Expr::Ident(name) = &object.node {
                let resolved_name = self.resolve_module_alias(name);
                if matches!(
                    resolved_name.as_str(),
                    "File" | "Time" | "System" | "Math" | "Str" | "Args"
                ) {
                    let builtin_name = format!("{}__{}", resolved_name, field);
                    if let Some(result) = self.compile_stdlib_function(&builtin_name, args)? {
                        return Ok(result);
                    }
                }
                if resolved_name == "io" {
                    if field == "println" || field == "print" {
                        return self.compile_print(args, field == "println");
                    }
                    if let Some(result) = self.compile_stdlib_function(field, args)? {
                        return Ok(result);
                    }
                }
            }
            return self.compile_method_call(&object.node, field, args);
        }

        if !matches!(callee, Expr::Ident(_) | Expr::Field { .. }) {
            let callee_ty = self.infer_expr_type(callee, &[]);
            if let Type::Function(param_types, ret_type) = callee_ty {
                let closure_val = self.compile_expr(callee)?.into_struct_value();
                let ptr = self
                    .builder
                    .build_extract_value(closure_val, 0, "fn_ptr")
                    .unwrap()
                    .into_pointer_value();
                let env_ptr = self
                    .builder
                    .build_extract_value(closure_val, 1, "env_ptr")
                    .unwrap();

                let llvm_ret = self.llvm_type(&ret_type);
                let mut llvm_params: Vec<BasicMetadataTypeEnum> =
                    vec![self.context.ptr_type(AddressSpace::default()).into()];
                for p in &param_types {
                    llvm_params.push(self.llvm_type(p).into());
                }

                let fn_type = match llvm_ret {
                    BasicTypeEnum::IntType(i) => i.fn_type(&llvm_params, false),
                    BasicTypeEnum::FloatType(f) => f.fn_type(&llvm_params, false),
                    BasicTypeEnum::PointerType(p) => p.fn_type(&llvm_params, false),
                    BasicTypeEnum::StructType(s) => s.fn_type(&llvm_params, false),
                    _ => self.context.i8_type().fn_type(&llvm_params, false),
                };

                if args.len() != param_types.len() {
                    return Err(Self::function_call_arity_error(
                        &Type::Function(param_types.clone(), ret_type.clone()),
                        args.len(),
                    ));
                }

                let mut compiled_args: Vec<BasicValueEnum> = vec![env_ptr];
                for (arg, param_ty) in args.iter().zip(param_types.iter()) {
                    compiled_args
                        .push(self.compile_expr_for_concrete_class_payload(&arg.node, param_ty)?);
                }

                let args_meta: Vec<BasicMetadataValueEnum> =
                    compiled_args.iter().map(|a| (*a).into()).collect();
                let call = self
                    .builder
                    .build_indirect_call(fn_type, ptr, &args_meta, "call")
                    .unwrap();

                return Ok(match call.try_as_basic_value() {
                    ValueKind::Basic(val) => val,
                    ValueKind::Instruction(_) => self.context.i8_type().const_int(0, false).into(),
                });
            }

            return Err(Self::non_function_call_error(&callee_ty));
        }

        // Regular function call
        let callee_name = if let Expr::Ident(name) = callee {
            if resolved_ident.is_empty() {
                Some(name.clone())
            } else {
                Some(resolved_ident.clone())
            }
        } else {
            None
        };
        let (func, resolved_func_ty) = match callee {
            Expr::Ident(name) => {
                // First check if it's a function pointer/local variable
                if let Some(var) = self.variables.get(name) {
                    if let Type::Function(param_types, ret_type) = &var.ty {
                        let param_types = param_types.clone();
                        let ret_type = ret_type.clone();
                        let closure_val = self
                            .builder
                            .build_load(self.llvm_type(&var.ty), var.ptr, name)
                            .unwrap()
                            .into_struct_value();

                        let ptr = self
                            .builder
                            .build_extract_value(closure_val, 0, "fn_ptr")
                            .unwrap()
                            .into_pointer_value();
                        let env_ptr = self
                            .builder
                            .build_extract_value(closure_val, 1, "env_ptr")
                            .unwrap();

                        // Construct FunctionType (including env_ptr as first arg)
                        let llvm_ret = self.llvm_type(&ret_type);
                        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![
                            self.context.ptr_type(AddressSpace::default()).into(), // env_ptr
                        ];
                        for p in &param_types {
                            llvm_params.push(self.llvm_type(p).into());
                        }

                        let fn_type = match llvm_ret {
                            BasicTypeEnum::IntType(i) => i.fn_type(&llvm_params, false),
                            BasicTypeEnum::FloatType(f) => f.fn_type(&llvm_params, false),
                            BasicTypeEnum::PointerType(p) => p.fn_type(&llvm_params, false),
                            BasicTypeEnum::StructType(s) => s.fn_type(&llvm_params, false),
                            _ => {
                                // Default to i8 type for void-like returns if needed
                                self.context.i8_type().fn_type(&llvm_params, false)
                            }
                        };

                        if args.len() != param_types.len() {
                            return Err(Self::function_call_arity_error(&var.ty, args.len()));
                        }

                        let mut compiled_args: Vec<BasicValueEnum> = vec![env_ptr];
                        for (arg, param_ty) in args.iter().zip(param_types.iter()) {
                            compiled_args.push(
                                self.compile_expr_for_concrete_class_payload(&arg.node, param_ty)?,
                            );
                        }

                        let args_meta: Vec<BasicMetadataValueEnum> =
                            compiled_args.iter().map(|a| (*a).into()).collect();

                        let call = self
                            .builder
                            .build_indirect_call(fn_type, ptr, &args_meta, "call")
                            .unwrap();

                        let result = match call.try_as_basic_value() {
                            ValueKind::Basic(val) => val,
                            ValueKind::Instruction(_) => {
                                self.context.i8_type().const_int(0, false).into()
                            }
                        };
                        return Ok(result);
                    }

                    return Err(Self::non_function_call_error(&var.ty));
                }

                let looked_up_name = if resolved_ident.is_empty() {
                    name
                } else {
                    resolved_ident.as_str()
                };

                // Fall back to global function lookup
                if let Some((f, _)) = self.functions.get(looked_up_name) {
                    (
                        *f,
                        self.functions.get(looked_up_name).map(|(_, ty)| ty.clone()),
                    )
                } else if let Some(f) = self.module.get_function(looked_up_name) {
                    (f, None)
                } else {
                    return Err(Self::undefined_function_error(looked_up_name));
                }
            }
            _ => return Err(CodegenError::new("Invalid callee")),
        };

        let mut compiled_args: Vec<BasicValueEnum> = Vec::new();
        let func_name = func.get_name().to_str().unwrap_or_default().to_string();
        let is_extern_call = callee_name
            .as_deref()
            .map(|n| self.extern_functions.contains(n))
            .unwrap_or(false);
        // Add null env_ptr for direct Arden calls (except main / extern C ABI)
        if func_name != "main" && !is_extern_call {
            compiled_args.push(
                self.context
                    .ptr_type(AddressSpace::default())
                    .const_null()
                    .into(),
            );
        }

        let callee_ty = resolved_func_ty.unwrap_or_else(|| self.infer_expr_type(callee, &[]));
        let expected_param_types = match &callee_ty {
            Type::Function(param_types, _) => Some(param_types.as_slice()),
            _ => None,
        };
        if let Some(param_types) = expected_param_types {
            let is_variadic_extern_call = is_extern_call && func.get_type().is_var_arg();
            let bad_arity = if is_variadic_extern_call {
                args.len() < param_types.len()
            } else {
                args.len() != param_types.len()
            };
            if bad_arity {
                return Err(Self::function_call_arity_error(&callee_ty, args.len()));
            }
            for (arg, param_ty) in args.iter().zip(param_types.iter()) {
                compiled_args
                    .push(self.compile_expr_for_concrete_class_payload(&arg.node, param_ty)?);
            }
            if is_variadic_extern_call {
                for arg in args.iter().skip(param_types.len()) {
                    let arg_ty = self.infer_builtin_argument_type(&arg.node);
                    compiled_args.push(self.compile_expr_with_expected_type(&arg.node, &arg_ty)?);
                }
            }
        } else {
            for a in args {
                compiled_args.push(self.compile_expr(&a.node)?);
            }
        }

        let args_meta: Vec<BasicMetadataValueEnum> =
            compiled_args.iter().map(|a| (*a).into()).collect();
        let call = self.builder.build_call(func, &args_meta, "call").unwrap();

        match call.try_as_basic_value() {
            ValueKind::Basic(val) => Ok(val),
            ValueKind::Instruction(_) => Ok(self.context.i8_type().const_int(0, false).into()),
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
        if let Expr::Ident(name) = object {
            if !self.variables.contains_key(name)
                && self
                    .resolve_contextual_function_value_name(object)
                    .is_none()
            {
                return Err(Self::undefined_variable_error(name));
            }
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
                        Expr::Ident(name) if !is_reference_receiver => {
                            self.variables.get(name).map(|v| v.ptr)
                        }
                        Expr::Field { object: obj, field } => {
                            self.compile_field_ptr(&obj.node, field).ok()
                        }
                        Expr::This if !is_reference_receiver => {
                            self.variables.get("this").map(|v| v.ptr)
                        }
                        _ => None,
                    };
                    if let Some(ptr) = list_ptr {
                        return self.compile_list_method_ptr(ptr, ty, method, args);
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
                                .unwrap();
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

        let obj_val = if matches!(obj_ty, Some(Type::Ref(_)) | Some(Type::MutRef(_))) {
            self.compile_deref(object)?
        } else {
            self.compile_expr(object)?
        };

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

        let (func, func_ty) = if self.classes.contains_key(&class_name) {
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
            self.functions
                .get(&func_name)
                .ok_or_else(|| CodegenError::new(format!("Unknown method: {}", func_name)))?
                .clone()
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
                let (_, func, func_ty) = candidates.pop().unwrap();
                (func, func_ty)
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
                    deref_obj_ty
                        .as_ref()
                        .expect("method receiver type should exist after class resolution"),
                    method,
                    param_types.len(),
                    args.len(),
                ));
            }
            for (arg, param_ty) in args.iter().zip(param_types.iter()) {
                compiled_args
                    .push(self.compile_expr_for_concrete_class_payload(&arg.node, param_ty)?);
            }
        } else {
            let llvm_param_types = func.get_type().get_param_types();
            for (arg, llvm_param_ty) in args.iter().zip(llvm_param_types.into_iter().skip(2)) {
                compiled_args.push(self.compile_expr_for_llvm_param(&arg.node, llvm_param_ty)?);
            }
        }

        let args_meta: Vec<BasicMetadataValueEnum> =
            compiled_args.iter().map(|a| (*a).into()).collect();
        let call = self.builder.build_call(func, &args_meta, "call").unwrap();

        match call.try_as_basic_value() {
            ValueKind::Basic(val) => Ok(val),
            ValueKind::Instruction(_) => Ok(self.context.i8_type().const_int(0, false).into()),
        }
    }

    fn compile_task_method(
        &mut self,
        object: &Expr,
        inner: &Type,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let expected_args = match method {
            "await_timeout" => 1,
            "is_done" | "cancel" => 0,
            _ => 0,
        };
        if args.len() != expected_args {
            return Err(CodegenError::new(format!(
                "Task.{}() expects {} argument(s), got {}",
                method,
                expected_args,
                args.len()
            )));
        }

        let task_ty = self.task_struct_type();
        let object_ty = self.infer_object_type(object);
        let task_raw = if matches!(object_ty, Some(Type::Ref(_)) | Some(Type::MutRef(_))) {
            self.compile_deref(object)?
        } else {
            self.compile_expr(object)?
        };
        if !task_raw.is_pointer_value() {
            return Err(CodegenError::new("Task method call on non-task value"));
        }
        let task_ptr = self
            .builder
            .build_pointer_cast(
                task_raw.into_pointer_value(),
                self.context.ptr_type(AddressSpace::default()),
                "task_method_ptr",
            )
            .unwrap();

        let i32_ty = self.context.i32_type();
        let zero = i32_ty.const_int(0, false);
        let thread_idx = i32_ty.const_int(0, false);
        let result_idx = i32_ty.const_int(1, false);
        let done_idx = i32_ty.const_int(2, false);
        let completed_idx = i32_ty.const_int(3, false);
        let done_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done_ptr")
                .unwrap()
        };
        let completed_field = unsafe {
            self.builder
                .build_gep(
                    task_ty,
                    task_ptr,
                    &[zero, completed_idx],
                    "task_completed_ptr",
                )
                .unwrap()
        };
        let result_field = unsafe {
            self.builder
                .build_gep(task_ty, task_ptr, &[zero, result_idx], "task_result_field")
                .unwrap()
        };

        match method {
            "is_done" => {
                let done_val = self
                    .builder
                    .build_load(self.context.i8_type(), done_field, "task_done")
                    .unwrap()
                    .into_int_value();
                let done_bool = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        done_val,
                        self.context.i8_type().const_zero(),
                        "task_done_bool",
                    )
                    .unwrap();
                let completed_val = self.build_atomic_bool_load(
                    completed_field,
                    "task_completed",
                    AtomicOrdering::Acquire,
                )?;
                Ok(self
                    .builder
                    .build_or(done_bool, completed_val, "task_is_done")
                    .unwrap()
                    .into())
            }
            "cancel" => {
                let done_val = self
                    .builder
                    .build_load(self.context.i8_type(), done_field, "task_done")
                    .unwrap()
                    .into_int_value();
                let done_bool = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        done_val,
                        self.context.i8_type().const_zero(),
                        "task_done_bool",
                    )
                    .unwrap();
                let completed_val = self.build_atomic_bool_load(
                    completed_field,
                    "task_completed",
                    AtomicOrdering::Acquire,
                )?;
                let already_done = self
                    .builder
                    .build_or(done_bool, completed_val, "task_already_done")
                    .unwrap();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Task.cancel used outside function"))?;
                let cancel_bb = self.context.append_basic_block(current_fn, "task_cancel");
                let merge_bb = self
                    .context
                    .append_basic_block(current_fn, "task_cancel_merge");
                self.builder
                    .build_conditional_branch(already_done, merge_bb, cancel_bb)
                    .unwrap();

                self.builder.position_at_end(cancel_bb);
                let thread_field = unsafe {
                    self.builder
                        .build_gep(task_ty, task_ptr, &[zero, thread_idx], "task_thread_ptr")
                        .unwrap()
                };
                let thread_id = self
                    .builder
                    .build_load(self.context.i64_type(), thread_field, "task_thread_id")
                    .unwrap();

                #[cfg(windows)]
                {
                    let terminate_fn = self.get_or_declare_terminate_thread_win();
                    let close_fn = self.get_or_declare_close_handle_win();
                    let handle = self
                        .builder
                        .build_int_to_ptr(
                            thread_id.into_int_value(),
                            self.context.ptr_type(AddressSpace::default()),
                            "task_cancel_handle",
                        )
                        .unwrap();
                    self.builder
                        .build_call(
                            terminate_fn,
                            &[
                                handle.into(),
                                self.context.i32_type().const_int(1, false).into(),
                            ],
                            "task_cancel",
                        )
                        .unwrap();
                    self.builder
                        .build_call(close_fn, &[handle.into()], "")
                        .unwrap();
                    self.builder
                        .build_store(thread_field, self.context.i64_type().const_zero())
                        .unwrap();
                }
                #[cfg(not(windows))]
                {
                    let pthread_cancel = self.get_or_declare_pthread_cancel();
                    self.builder
                        .build_call(pthread_cancel, &[thread_id.into()], "task_cancel")
                        .unwrap();
                }

                let done_field = unsafe {
                    self.builder
                        .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done_ptr")
                        .unwrap()
                };
                self.builder
                    .build_store(done_field, self.context.i8_type().const_int(1, false))
                    .unwrap();
                self.build_atomic_bool_store(
                    completed_field,
                    self.context.i8_type().const_int(1, false),
                    AtomicOrdering::Release,
                )?;

                // Store a safe default payload so await after cancel stays valid for heap-backed
                // values like user classes and ranges instead of returning a null object pointer.
                let malloc = self.get_or_declare_malloc();
                let llvm_inner = self.llvm_type(inner);
                let size = llvm_inner
                    .size_of()
                    .ok_or_else(|| CodegenError::new("failed to size Task inner type"))?;
                let raw = self
                    .builder
                    .build_call(malloc, &[size.into()], "task_cancel_alloc")
                    .unwrap();
                let result_ptr = self.extract_call_pointer_value(
                    raw,
                    "malloc failed while creating canceled task value",
                )?;
                let typed_ptr = self
                    .builder
                    .build_pointer_cast(
                        result_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "task_cancel_result_ptr",
                    )
                    .unwrap();
                let default_value = self.create_default_value_for_type(inner)?;
                self.builder.build_store(typed_ptr, default_value).unwrap();
                self.builder.build_store(result_field, result_ptr).unwrap();
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(merge_bb);
                Ok(self.context.i8_type().const_int(0, false).into())
            }
            "await_timeout" => {
                if matches!(
                    TypeChecker::eval_numeric_const_expr(&args[0].node),
                    Some(NumericConst::Integer(value)) if value < 0
                ) {
                    return Err(CodegenError::new(
                        "Task.await_timeout() timeout must be non-negative",
                    ));
                }
                let ms_ty = self.infer_builtin_argument_type(&args[0].node);
                if !matches!(ms_ty, Type::Integer) {
                    return Err(CodegenError::new(
                        "Task.await_timeout(ms) requires Integer milliseconds",
                    ));
                }
                let ms = self.compile_expr_with_expected_type(&args[0].node, &ms_ty)?;
                if !ms.is_int_value() {
                    return Err(CodegenError::new(
                        "Task.await_timeout(ms) requires Integer milliseconds",
                    ));
                }
                let ms_i64 = self
                    .builder
                    .build_int_cast(ms.into_int_value(), self.context.i64_type(), "timeout_ms")
                    .unwrap();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Task.await_timeout used outside function"))?;
                let timeout_valid_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_valid");
                let timeout_invalid_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_invalid");
                let timeout_negative = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        ms_i64,
                        self.context.i64_type().const_zero(),
                        "task_timeout_negative",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(
                        timeout_negative,
                        timeout_invalid_bb,
                        timeout_valid_bb,
                    )
                    .unwrap();

                self.builder.position_at_end(timeout_invalid_bb);
                self.emit_runtime_error(
                    "Task.await_timeout() timeout must be non-negative",
                    "task_timeout_negative_runtime_error",
                )?;

                self.builder.position_at_end(timeout_valid_bb);

                let done_field = unsafe {
                    self.builder
                        .build_gep(task_ty, task_ptr, &[zero, done_idx], "task_done_ptr")
                        .unwrap()
                };
                let completed_field = unsafe {
                    self.builder
                        .build_gep(
                            task_ty,
                            task_ptr,
                            &[zero, completed_idx],
                            "task_completed_ptr",
                        )
                        .unwrap()
                };
                let done_val = self
                    .builder
                    .build_load(self.context.i8_type(), done_field, "task_done")
                    .unwrap()
                    .into_int_value();
                let done_ready = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        done_val,
                        self.context.i8_type().const_zero(),
                        "task_done_ready",
                    )
                    .unwrap();

                let done_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_done");
                let check_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_check");
                let join_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_join");
                let loop_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_loop");
                let sleep_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_sleep");
                let timeout_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_fail");
                let merge_bb = self
                    .context
                    .append_basic_block(current_fn, "task_timeout_merge");
                let thread_field = unsafe {
                    self.builder
                        .build_gep(task_ty, task_ptr, &[zero, thread_idx], "task_thread_ptr")
                        .unwrap()
                };
                let thread_id = self
                    .builder
                    .build_load(self.context.i64_type(), thread_field, "task_thread_id")
                    .unwrap();
                let join_result_ptr = self
                    .builder
                    .build_alloca(
                        self.context.ptr_type(AddressSpace::default()),
                        "timed_join_out",
                    )
                    .unwrap();
                self.builder
                    .build_store(
                        join_result_ptr,
                        self.context.ptr_type(AddressSpace::default()).const_null(),
                    )
                    .unwrap();
                let iter_ptr = self
                    .builder
                    .build_alloca(self.context.i64_type(), "task_timeout_iter")
                    .unwrap();
                self.builder
                    .build_store(iter_ptr, self.context.i64_type().const_zero())
                    .unwrap();
                let max_iters = ms_i64;

                self.builder
                    .build_conditional_branch(done_ready, done_bb, check_bb)
                    .unwrap();

                // done -> Some(result)
                self.builder.position_at_end(done_bb);
                let existing_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        result_field,
                        "task_existing_result",
                    )
                    .unwrap()
                    .into_pointer_value();
                let done_value: BasicValueEnum = if matches!(inner, Type::None) {
                    self.context.i8_type().const_int(0, false).into()
                } else {
                    let inner_llvm = self.llvm_type(inner);
                    let typed_ptr = self
                        .builder
                        .build_pointer_cast(
                            existing_ptr,
                            self.context.ptr_type(AddressSpace::default()),
                            "task_done_typed_ptr",
                        )
                        .unwrap();
                    self.builder
                        .build_load(inner_llvm, typed_ptr, "task_done_value")
                        .unwrap()
                };
                let done_some = self.create_option_some(done_value)?;
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(check_bb);
                let completed_val = self.build_atomic_bool_load(
                    completed_field,
                    "task_completed",
                    AtomicOrdering::Acquire,
                )?;
                self.builder
                    .build_conditional_branch(completed_val, join_bb, loop_bb)
                    .unwrap();

                self.builder.position_at_end(loop_bb);
                let iter_val = self
                    .builder
                    .build_load(self.context.i64_type(), iter_ptr, "task_timeout_iter_val")
                    .unwrap()
                    .into_int_value();
                let timed_out = self
                    .builder
                    .build_int_compare(
                        IntPredicate::UGE,
                        iter_val,
                        max_iters,
                        "task_timeout_reached",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(timed_out, timeout_bb, sleep_bb)
                    .unwrap();

                self.builder.position_at_end(sleep_bb);
                #[cfg(windows)]
                {
                    let sleep_fn = self.get_or_declare_sleep_win();
                    self.builder
                        .build_call(
                            sleep_fn,
                            &[self.context.i32_type().const_int(1, false).into()],
                            "task_timeout_sleep_call",
                        )
                        .unwrap();
                }
                #[cfg(not(windows))]
                {
                    let usleep_fn = self.get_or_declare_usleep();
                    self.builder
                        .build_call(
                            usleep_fn,
                            &[self.context.i32_type().const_int(1000, false).into()],
                            "task_timeout_usleep_call",
                        )
                        .unwrap();
                }
                let next_iter = self
                    .builder
                    .build_int_add(
                        iter_val,
                        self.context.i64_type().const_int(1, false),
                        "task_timeout_next_iter",
                    )
                    .unwrap();
                self.builder.build_store(iter_ptr, next_iter).unwrap();
                self.builder.build_unconditional_branch(check_bb).unwrap();

                self.builder.position_at_end(join_bb);
                #[cfg(windows)]
                let joined_ptr = {
                    let wait_fn = self.get_or_declare_wait_for_single_object_win();
                    let close_fn = self.get_or_declare_close_handle_win();
                    let handle = self
                        .builder
                        .build_int_to_ptr(
                            thread_id.into_int_value(),
                            self.context.ptr_type(AddressSpace::default()),
                            "timed_join_handle",
                        )
                        .unwrap();
                    self.builder
                        .build_call(
                            wait_fn,
                            &[
                                handle.into(),
                                self.context.i32_type().const_all_ones().into(),
                            ],
                            "timed_join_finalize",
                        )
                        .unwrap();
                    self.builder
                        .build_call(close_fn, &[handle.into()], "")
                        .unwrap();
                    self.builder
                        .build_store(thread_field, self.context.i64_type().const_zero())
                        .unwrap();
                    self.builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            result_field,
                            "joined_result",
                        )
                        .unwrap()
                        .into_pointer_value()
                };
                #[cfg(not(windows))]
                let joined_ptr = {
                    let pthread_join = self.get_or_declare_pthread_join();
                    self.builder
                        .build_call(
                            pthread_join,
                            &[thread_id.into(), join_result_ptr.into()],
                            "timed_join_finalize",
                        )
                        .unwrap();
                    self.builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            join_result_ptr,
                            "joined_result",
                        )
                        .unwrap()
                        .into_pointer_value()
                };
                self.builder.build_store(result_field, joined_ptr).unwrap();
                self.builder
                    .build_store(done_field, self.context.i8_type().const_int(1, false))
                    .unwrap();
                self.build_atomic_bool_store(
                    completed_field,
                    self.context.i8_type().const_int(1, false),
                    AtomicOrdering::Release,
                )?;
                let succ_value: BasicValueEnum = if matches!(inner, Type::None) {
                    self.context.i8_type().const_int(0, false).into()
                } else {
                    let inner_llvm = self.llvm_type(inner);
                    let typed_ptr = self
                        .builder
                        .build_pointer_cast(
                            joined_ptr,
                            self.context.ptr_type(AddressSpace::default()),
                            "joined_typed_ptr",
                        )
                        .unwrap();
                    self.builder
                        .build_load(inner_llvm, typed_ptr, "joined_value")
                        .unwrap()
                };
                let succ_some = self.create_option_some(succ_value)?;
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                // timeout -> None
                self.builder.position_at_end(timeout_bb);
                let option_ty = self.context.struct_type(
                    &[self.context.i8_type().into(), self.llvm_type(inner)],
                    false,
                );
                let timeout_none: BasicValueEnum<'ctx> = option_ty
                    .const_named_struct(&[
                        self.context.i8_type().const_int(0, false).into(),
                        match self.llvm_type(inner) {
                            BasicTypeEnum::IntType(t) => t.const_zero().into(),
                            BasicTypeEnum::FloatType(t) => t.const_float(0.0).into(),
                            BasicTypeEnum::PointerType(t) => t.const_null().into(),
                            BasicTypeEnum::StructType(t) => t.const_zero().into(),
                            _ => self.context.i8_type().const_int(0, false).into(),
                        },
                    ])
                    .into();
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(
                        self.llvm_type(&Type::Option(Box::new(inner.clone()))),
                        "timeout_phi",
                    )
                    .unwrap();
                phi.add_incoming(&[
                    (&done_some, done_bb),
                    (&succ_some, join_bb),
                    (&timeout_none, timeout_bb),
                ]);
                Ok(phi.as_basic_value())
            }
            _ => Err(CodegenError::new(format!(
                "Unknown Task method: {}",
                method
            ))),
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
                let _ = self.compile_expr(object);
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
                obj_ty.as_ref().expect("class-like type already validated"),
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
                let (method_name, func_ty) = candidates.pop().unwrap();
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
        let class_info = class_info.unwrap();
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
                obj_ty.as_ref().expect("class-like type already validated"),
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

        let field_ptr = unsafe {
            self.builder
                .build_gep(
                    struct_type.as_basic_type_enum(),
                    obj_ptr,
                    &[zero, idx],
                    field,
                )
                .unwrap()
        };

        let field_type = struct_type.get_field_type_at_index(field_idx).unwrap();
        Ok(self
            .builder
            .build_load(field_type, field_ptr, field)
            .unwrap())
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
        let malloc = self.get_or_declare_malloc();
        let env_size = env_struct_ty
            .size_of()
            .ok_or_else(|| CodegenError::new("failed to size bound-method env"))?;
        let env_alloc = self
            .builder
            .build_call(malloc, &[env_size.into()], "bound_method_env_alloc")
            .unwrap();
        let env_ptr =
            self.extract_call_pointer_value(env_alloc, "malloc failed for bound-method env")?;
        let receiver_ptr = unsafe {
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
                .unwrap()
        };
        self.builder
            .build_store(receiver_ptr, receiver_value)
            .unwrap();

        let adapter_name = format!("__bound_method_adapter_{}", self.lambda_counter);
        self.lambda_counter += 1;
        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![ptr_type.into()];
        for param_ty in param_types {
            llvm_params.push(self.llvm_type(param_ty).into());
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
        let stored_receiver_ptr = unsafe {
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
                .unwrap()
        };
        let loaded_receiver = self
            .builder
            .build_load(
                receiver_llvm_ty,
                stored_receiver_ptr,
                "bound_method_receiver",
            )
            .unwrap();

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
            .unwrap();
        match call.try_as_basic_value() {
            ValueKind::Basic(val) => {
                self.builder.build_return(Some(&val)).unwrap();
            }
            ValueKind::Instruction(_) => {
                self.builder
                    .build_return(Some(&self.context.i8_type().const_int(0, false)))
                    .unwrap();
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
            .unwrap()
            .into_struct_value();
        closure = self
            .builder
            .build_insert_value(closure, env_ptr, 1, "env")
            .unwrap()
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
                obj_ty.as_ref().expect("class-like type already validated"),
            ));
        }
        let class_info = self
            .classes
            .get(&class_name)
            .ok_or_else(|| CodegenError::new(format!("Unknown class: {}", class_name)))?;

        let field_idx = *class_info.field_indices.get(field).ok_or_else(|| {
            Self::unknown_field_error(
                field,
                obj_ty.as_ref().expect("class-like type already validated"),
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

        let field_ptr = unsafe {
            self.builder
                .build_gep(
                    struct_type.as_basic_type_enum(),
                    obj_ptr,
                    &[zero, idx],
                    field,
                )
                .unwrap()
        };

        Ok(field_ptr)
    }

    fn compile_utf8_string_index_runtime(
        &mut self,
        string_ptr: PointerValue<'ctx>,
        idx: IntValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let current_fn = self
            .current_function
            .ok_or_else(|| CodegenError::new("String indexing used outside function"))?;
        let i8_type = self.context.i8_type();
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());

        let index_non_negative = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                idx,
                i64_type.const_zero(),
                "string_index_non_negative",
            )
            .unwrap();

        let loop_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_loop");
        let fail_oob_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_oob");
        let fail_utf8_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_invalid_utf8");
        let not_end_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_not_end");
        let target_dispatch_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_target_dispatch");
        let advance_check_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_advance_check");
        let advance_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_advance");
        let decode_ascii_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_decode_ascii");
        let target_non_ascii_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_target_non_ascii");
        let decode_two_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_decode_two");
        let target_not_two_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_target_not_two");
        let decode_three_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_decode_three");
        let target_not_three_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_target_not_three");
        let decode_four_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_decode_four");
        let decode_two_ok_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_decode_two_ok");
        let decode_three_ok_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_decode_three_ok");
        let decode_four_ok_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_decode_four_ok");
        let return_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_index_return");

        let ptr_slot = self
            .builder
            .build_alloca(ptr_type, "utf8_string_ptr")
            .unwrap();
        let char_index_slot = self
            .builder
            .build_alloca(i64_type, "utf8_string_char_index")
            .unwrap();
        let char_result_slot = self
            .builder
            .build_alloca(i32_type, "utf8_string_char_result")
            .unwrap();

        self.builder.build_store(ptr_slot, string_ptr).unwrap();
        self.builder
            .build_store(char_index_slot, i64_type.const_zero())
            .unwrap();
        self.builder
            .build_conditional_branch(index_non_negative, loop_bb, fail_oob_bb)
            .unwrap();

        self.builder.position_at_end(fail_oob_bb);
        self.emit_runtime_error("String index out of bounds", "string_index_oob")?;

        self.builder.position_at_end(fail_utf8_bb);
        self.emit_runtime_error(
            "Invalid UTF-8 sequence in String",
            "string_index_invalid_utf8",
        )?;

        self.builder.position_at_end(loop_bb);
        let current_ptr = self
            .builder
            .build_load(ptr_type, ptr_slot, "utf8_string_ptr_load")
            .unwrap()
            .into_pointer_value();
        let current_char_index = self
            .builder
            .build_load(i64_type, char_index_slot, "utf8_string_char_index_load")
            .unwrap()
            .into_int_value();
        let lead_byte = self
            .builder
            .build_load(i8_type, current_ptr, "utf8_string_lead_byte")
            .unwrap()
            .into_int_value();
        let lead_u32 = self
            .builder
            .build_int_z_extend(lead_byte, i32_type, "utf8_string_lead_u32")
            .unwrap();
        let is_end = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_byte,
                i8_type.const_zero(),
                "utf8_string_is_end",
            )
            .unwrap();
        let is_ascii = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                lead_u32,
                i32_type.const_int(0x80, false),
                "utf8_string_is_ascii",
            )
            .unwrap();
        let lead_mask_e0 = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0xE0, false),
                "utf8_string_mask_e0",
            )
            .unwrap();
        let lead_mask_f0 = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0xF0, false),
                "utf8_string_mask_f0",
            )
            .unwrap();
        let lead_mask_f8 = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0xF8, false),
                "utf8_string_mask_f8",
            )
            .unwrap();
        let is_two = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_mask_e0,
                i32_type.const_int(0xC0, false),
                "utf8_string_is_two",
            )
            .unwrap();
        let is_three = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_mask_f0,
                i32_type.const_int(0xE0, false),
                "utf8_string_is_three",
            )
            .unwrap();
        let is_four = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_mask_f8,
                i32_type.const_int(0xF0, false),
                "utf8_string_is_four",
            )
            .unwrap();
        let width_two_or_zero = self
            .builder
            .build_select(
                is_two,
                i64_type.const_int(2, false),
                i64_type.const_zero(),
                "utf8_string_width_two",
            )
            .unwrap()
            .into_int_value();
        let width_three_or_prev = self
            .builder
            .build_select(
                is_three,
                i64_type.const_int(3, false),
                width_two_or_zero,
                "utf8_string_width_three",
            )
            .unwrap()
            .into_int_value();
        let width_nonzero = self
            .builder
            .build_select(
                is_ascii,
                i64_type.const_int(1, false),
                width_three_or_prev,
                "utf8_string_width_ascii",
            )
            .unwrap()
            .into_int_value();
        let width = self
            .builder
            .build_select(
                is_four,
                i64_type.const_int(4, false),
                width_nonzero,
                "utf8_string_width",
            )
            .unwrap()
            .into_int_value();
        let width_is_valid = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                width,
                i64_type.const_zero(),
                "utf8_string_width_is_valid",
            )
            .unwrap();

        self.builder
            .build_conditional_branch(is_end, fail_oob_bb, not_end_bb)
            .unwrap();

        self.builder.position_at_end(not_end_bb);
        let is_target = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                current_char_index,
                idx,
                "utf8_string_is_target",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(is_target, target_dispatch_bb, advance_check_bb)
            .unwrap();

        self.builder.position_at_end(advance_check_bb);
        self.builder
            .build_conditional_branch(width_is_valid, advance_bb, fail_utf8_bb)
            .unwrap();

        self.builder.position_at_end(advance_bb);
        let advanced_ptr = unsafe {
            self.builder
                .build_gep(i8_type, current_ptr, &[width], "utf8_string_advance_ptr")
                .unwrap()
        };
        let next_char_index = self
            .builder
            .build_int_add(
                current_char_index,
                i64_type.const_int(1, false),
                "utf8_string_next_char_index",
            )
            .unwrap();
        self.builder.build_store(ptr_slot, advanced_ptr).unwrap();
        self.builder
            .build_store(char_index_slot, next_char_index)
            .unwrap();
        self.builder.build_unconditional_branch(loop_bb).unwrap();

        self.builder.position_at_end(target_dispatch_bb);
        self.builder
            .build_conditional_branch(is_ascii, decode_ascii_bb, target_non_ascii_bb)
            .unwrap();

        self.builder.position_at_end(target_non_ascii_bb);
        self.builder
            .build_conditional_branch(is_two, decode_two_bb, target_not_two_bb)
            .unwrap();

        self.builder.position_at_end(target_not_two_bb);
        self.builder
            .build_conditional_branch(is_three, decode_three_bb, target_not_three_bb)
            .unwrap();

        self.builder.position_at_end(target_not_three_bb);
        self.builder
            .build_conditional_branch(is_four, decode_four_bb, fail_utf8_bb)
            .unwrap();

        self.builder.position_at_end(decode_ascii_bb);
        self.builder
            .build_store(char_result_slot, lead_u32)
            .unwrap();
        self.builder.build_unconditional_branch(return_bb).unwrap();

        self.builder.position_at_end(decode_two_bb);
        let cont1_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[i64_type.const_int(1, false)],
                    "utf8_cont1_ptr",
                )
                .unwrap()
        };
        let cont1 = self
            .builder
            .build_load(i8_type, cont1_ptr, "utf8_cont1")
            .unwrap()
            .into_int_value();
        let cont1_u32 = self
            .builder
            .build_int_z_extend(cont1, i32_type, "utf8_cont1_u32")
            .unwrap();
        let cont1_mask = self
            .builder
            .build_and(
                cont1_u32,
                i32_type.const_int(0xC0, false),
                "utf8_cont1_mask",
            )
            .unwrap();
        let cont1_valid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                cont1_mask,
                i32_type.const_int(0x80, false),
                "utf8_cont1_valid",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(cont1_valid, decode_two_ok_bb, fail_utf8_bb)
            .unwrap();

        self.builder.position_at_end(decode_two_ok_bb);
        let lead_bits = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0x1F, false),
                "utf8_two_lead_bits",
            )
            .unwrap();
        let cont1_bits = self
            .builder
            .build_and(
                cont1_u32,
                i32_type.const_int(0x3F, false),
                "utf8_two_cont1_bits",
            )
            .unwrap();
        let lead_shifted = self
            .builder
            .build_left_shift(
                lead_bits,
                i32_type.const_int(6, false),
                "utf8_two_lead_shifted",
            )
            .unwrap();
        let codepoint = self
            .builder
            .build_or(lead_shifted, cont1_bits, "utf8_two_codepoint")
            .unwrap();
        self.builder
            .build_store(char_result_slot, codepoint)
            .unwrap();
        self.builder.build_unconditional_branch(return_bb).unwrap();

        self.builder.position_at_end(decode_three_bb);
        let cont2_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[i64_type.const_int(1, false)],
                    "utf8_cont2_ptr",
                )
                .unwrap()
        };
        let cont3_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[i64_type.const_int(2, false)],
                    "utf8_cont3_ptr",
                )
                .unwrap()
        };
        let cont2 = self
            .builder
            .build_load(i8_type, cont2_ptr, "utf8_cont2")
            .unwrap()
            .into_int_value();
        let cont3 = self
            .builder
            .build_load(i8_type, cont3_ptr, "utf8_cont3")
            .unwrap()
            .into_int_value();
        let cont2_u32 = self
            .builder
            .build_int_z_extend(cont2, i32_type, "utf8_cont2_u32")
            .unwrap();
        let cont3_u32 = self
            .builder
            .build_int_z_extend(cont3, i32_type, "utf8_cont3_u32")
            .unwrap();
        let cont2_mask = self
            .builder
            .build_and(
                cont2_u32,
                i32_type.const_int(0xC0, false),
                "utf8_cont2_mask",
            )
            .unwrap();
        let cont3_mask = self
            .builder
            .build_and(
                cont3_u32,
                i32_type.const_int(0xC0, false),
                "utf8_cont3_mask",
            )
            .unwrap();
        let cont2_valid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                cont2_mask,
                i32_type.const_int(0x80, false),
                "utf8_cont2_valid",
            )
            .unwrap();
        let cont3_valid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                cont3_mask,
                i32_type.const_int(0x80, false),
                "utf8_cont3_valid",
            )
            .unwrap();
        let cont23_valid = self
            .builder
            .build_and(cont2_valid, cont3_valid, "utf8_cont23_valid")
            .unwrap();
        self.builder
            .build_conditional_branch(cont23_valid, decode_three_ok_bb, fail_utf8_bb)
            .unwrap();

        self.builder.position_at_end(decode_three_ok_bb);
        let lead_bits = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0x0F, false),
                "utf8_three_lead_bits",
            )
            .unwrap();
        let cont2_bits = self
            .builder
            .build_and(
                cont2_u32,
                i32_type.const_int(0x3F, false),
                "utf8_three_cont2_bits",
            )
            .unwrap();
        let cont3_bits = self
            .builder
            .build_and(
                cont3_u32,
                i32_type.const_int(0x3F, false),
                "utf8_three_cont3_bits",
            )
            .unwrap();
        let lead_shifted = self
            .builder
            .build_left_shift(
                lead_bits,
                i32_type.const_int(12, false),
                "utf8_three_lead_shifted",
            )
            .unwrap();
        let cont2_shifted = self
            .builder
            .build_left_shift(
                cont2_bits,
                i32_type.const_int(6, false),
                "utf8_three_cont2_shifted",
            )
            .unwrap();
        let partial = self
            .builder
            .build_or(lead_shifted, cont2_shifted, "utf8_three_partial")
            .unwrap();
        let codepoint = self
            .builder
            .build_or(partial, cont3_bits, "utf8_three_codepoint")
            .unwrap();
        self.builder
            .build_store(char_result_slot, codepoint)
            .unwrap();
        self.builder.build_unconditional_branch(return_bb).unwrap();

        self.builder.position_at_end(decode_four_bb);
        let cont4_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[i64_type.const_int(1, false)],
                    "utf8_cont4_ptr",
                )
                .unwrap()
        };
        let cont5_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[i64_type.const_int(2, false)],
                    "utf8_cont5_ptr",
                )
                .unwrap()
        };
        let cont6_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[i64_type.const_int(3, false)],
                    "utf8_cont6_ptr",
                )
                .unwrap()
        };
        let cont4 = self
            .builder
            .build_load(i8_type, cont4_ptr, "utf8_cont4")
            .unwrap()
            .into_int_value();
        let cont5 = self
            .builder
            .build_load(i8_type, cont5_ptr, "utf8_cont5")
            .unwrap()
            .into_int_value();
        let cont6 = self
            .builder
            .build_load(i8_type, cont6_ptr, "utf8_cont6")
            .unwrap()
            .into_int_value();
        let cont4_u32 = self
            .builder
            .build_int_z_extend(cont4, i32_type, "utf8_cont4_u32")
            .unwrap();
        let cont5_u32 = self
            .builder
            .build_int_z_extend(cont5, i32_type, "utf8_cont5_u32")
            .unwrap();
        let cont6_u32 = self
            .builder
            .build_int_z_extend(cont6, i32_type, "utf8_cont6_u32")
            .unwrap();
        let cont4_mask = self
            .builder
            .build_and(
                cont4_u32,
                i32_type.const_int(0xC0, false),
                "utf8_cont4_mask",
            )
            .unwrap();
        let cont5_mask = self
            .builder
            .build_and(
                cont5_u32,
                i32_type.const_int(0xC0, false),
                "utf8_cont5_mask",
            )
            .unwrap();
        let cont6_mask = self
            .builder
            .build_and(
                cont6_u32,
                i32_type.const_int(0xC0, false),
                "utf8_cont6_mask",
            )
            .unwrap();
        let cont4_valid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                cont4_mask,
                i32_type.const_int(0x80, false),
                "utf8_cont4_valid",
            )
            .unwrap();
        let cont5_valid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                cont5_mask,
                i32_type.const_int(0x80, false),
                "utf8_cont5_valid",
            )
            .unwrap();
        let cont6_valid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                cont6_mask,
                i32_type.const_int(0x80, false),
                "utf8_cont6_valid",
            )
            .unwrap();
        let cont45_valid = self
            .builder
            .build_and(cont4_valid, cont5_valid, "utf8_cont45_valid")
            .unwrap();
        let cont456_valid = self
            .builder
            .build_and(cont45_valid, cont6_valid, "utf8_cont456_valid")
            .unwrap();
        self.builder
            .build_conditional_branch(cont456_valid, decode_four_ok_bb, fail_utf8_bb)
            .unwrap();

        self.builder.position_at_end(decode_four_ok_bb);
        let lead_bits = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0x07, false),
                "utf8_four_lead_bits",
            )
            .unwrap();
        let cont4_bits = self
            .builder
            .build_and(
                cont4_u32,
                i32_type.const_int(0x3F, false),
                "utf8_four_cont4_bits",
            )
            .unwrap();
        let cont5_bits = self
            .builder
            .build_and(
                cont5_u32,
                i32_type.const_int(0x3F, false),
                "utf8_four_cont5_bits",
            )
            .unwrap();
        let cont6_bits = self
            .builder
            .build_and(
                cont6_u32,
                i32_type.const_int(0x3F, false),
                "utf8_four_cont6_bits",
            )
            .unwrap();
        let lead_shifted = self
            .builder
            .build_left_shift(
                lead_bits,
                i32_type.const_int(18, false),
                "utf8_four_lead_shifted",
            )
            .unwrap();
        let cont4_shifted = self
            .builder
            .build_left_shift(
                cont4_bits,
                i32_type.const_int(12, false),
                "utf8_four_cont4_shifted",
            )
            .unwrap();
        let cont5_shifted = self
            .builder
            .build_left_shift(
                cont5_bits,
                i32_type.const_int(6, false),
                "utf8_four_cont5_shifted",
            )
            .unwrap();
        let partial = self
            .builder
            .build_or(lead_shifted, cont4_shifted, "utf8_four_partial_1")
            .unwrap();
        let partial = self
            .builder
            .build_or(partial, cont5_shifted, "utf8_four_partial_2")
            .unwrap();
        let codepoint = self
            .builder
            .build_or(partial, cont6_bits, "utf8_four_codepoint")
            .unwrap();
        self.builder
            .build_store(char_result_slot, codepoint)
            .unwrap();
        self.builder.build_unconditional_branch(return_bb).unwrap();

        self.builder.position_at_end(return_bb);
        Ok(self
            .builder
            .build_load(i32_type, char_result_slot, "utf8_string_char_result_load")
            .unwrap())
    }

    fn compile_utf8_string_length_runtime(
        &mut self,
        string_ptr: PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let current_fn = self
            .current_function
            .ok_or_else(|| CodegenError::new("String.length used outside function"))?;
        let i8_type = self.context.i8_type();
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());

        let loop_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_length_loop");
        let continue_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_length_continue");
        let advance_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_length_advance");
        let fail_utf8_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_length_invalid_utf8");
        let return_bb = self
            .context
            .append_basic_block(current_fn, "utf8_string_length_return");

        let ptr_slot = self
            .builder
            .build_alloca(ptr_type, "utf8_string_length_ptr")
            .unwrap();
        let char_count_slot = self
            .builder
            .build_alloca(i64_type, "utf8_string_length_count")
            .unwrap();

        self.builder.build_store(ptr_slot, string_ptr).unwrap();
        self.builder
            .build_store(char_count_slot, i64_type.const_zero())
            .unwrap();
        self.builder.build_unconditional_branch(loop_bb).unwrap();

        self.builder.position_at_end(fail_utf8_bb);
        self.emit_runtime_error(
            "Invalid UTF-8 sequence in String",
            "string_length_invalid_utf8",
        )?;

        self.builder.position_at_end(loop_bb);
        let current_ptr = self
            .builder
            .build_load(ptr_type, ptr_slot, "utf8_string_length_ptr_load")
            .unwrap()
            .into_pointer_value();
        let current_count = self
            .builder
            .build_load(i64_type, char_count_slot, "utf8_string_length_count_load")
            .unwrap()
            .into_int_value();
        let lead_byte = self
            .builder
            .build_load(i8_type, current_ptr, "utf8_string_length_lead_byte")
            .unwrap()
            .into_int_value();
        let lead_u32 = self
            .builder
            .build_int_z_extend(lead_byte, i32_type, "utf8_string_length_lead_u32")
            .unwrap();
        let is_end = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_byte,
                i8_type.const_zero(),
                "utf8_string_length_is_end",
            )
            .unwrap();
        let is_ascii = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                lead_u32,
                i32_type.const_int(0x80, false),
                "utf8_string_length_is_ascii",
            )
            .unwrap();
        let lead_mask_e0 = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0xE0, false),
                "utf8_string_length_mask_e0",
            )
            .unwrap();
        let lead_mask_f0 = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0xF0, false),
                "utf8_string_length_mask_f0",
            )
            .unwrap();
        let lead_mask_f8 = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0xF8, false),
                "utf8_string_length_mask_f8",
            )
            .unwrap();
        let is_two = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_mask_e0,
                i32_type.const_int(0xC0, false),
                "utf8_string_length_is_two",
            )
            .unwrap();
        let is_three = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_mask_f0,
                i32_type.const_int(0xE0, false),
                "utf8_string_length_is_three",
            )
            .unwrap();
        let is_four = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_mask_f8,
                i32_type.const_int(0xF0, false),
                "utf8_string_length_is_four",
            )
            .unwrap();
        let width_two_or_zero = self
            .builder
            .build_select(
                is_two,
                i64_type.const_int(2, false),
                i64_type.const_zero(),
                "utf8_string_length_width_two",
            )
            .unwrap()
            .into_int_value();
        let width_three_or_prev = self
            .builder
            .build_select(
                is_three,
                i64_type.const_int(3, false),
                width_two_or_zero,
                "utf8_string_length_width_three",
            )
            .unwrap()
            .into_int_value();
        let width_nonzero = self
            .builder
            .build_select(
                is_ascii,
                i64_type.const_int(1, false),
                width_three_or_prev,
                "utf8_string_length_width_ascii",
            )
            .unwrap()
            .into_int_value();
        let width = self
            .builder
            .build_select(
                is_four,
                i64_type.const_int(4, false),
                width_nonzero,
                "utf8_string_length_width",
            )
            .unwrap()
            .into_int_value();
        let width_is_valid = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                width,
                i64_type.const_zero(),
                "utf8_string_length_width_is_valid",
            )
            .unwrap();

        self.builder
            .build_conditional_branch(is_end, return_bb, continue_bb)
            .unwrap();

        self.builder.position_at_end(continue_bb);
        self.builder
            .build_conditional_branch(width_is_valid, advance_bb, fail_utf8_bb)
            .unwrap();

        self.builder.position_at_end(advance_bb);
        let advanced_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[width],
                    "utf8_string_length_advance_ptr",
                )
                .unwrap()
        };
        let next_char_count = self
            .builder
            .build_int_add(
                current_count,
                i64_type.const_int(1, false),
                "utf8_string_length_next_count",
            )
            .unwrap();
        self.builder.build_store(ptr_slot, advanced_ptr).unwrap();
        self.builder
            .build_store(char_count_slot, next_char_count)
            .unwrap();
        self.builder.build_unconditional_branch(loop_bb).unwrap();

        self.builder.position_at_end(return_bb);
        Ok(self
            .builder
            .build_load(i64_type, char_count_slot, "utf8_string_length_result")
            .unwrap())
    }

    pub fn compile_index(&mut self, object: &Expr, index: &Expr) -> Result<BasicValueEnum<'ctx>> {
        if let Expr::Ident(name) = object {
            if !self.variables.contains_key(name)
                && self
                    .resolve_contextual_function_value_name(object)
                    .is_none()
            {
                return Err(Self::undefined_variable_error(name));
            }
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
        let idx = self.compile_non_negative_integer_index_expr(index, negative_diagnostic)?;

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
                let non_negative = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SGE,
                        idx,
                        i64_type.const_zero(),
                        "string_literal_index_non_negative",
                    )
                    .unwrap();
                let length = i64_type.const_int(char_values.len() as u64, false);
                let in_bounds = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        idx,
                        length,
                        "string_literal_index_in_bounds",
                    )
                    .unwrap();
                let valid = self
                    .builder
                    .build_and(non_negative, in_bounds, "string_literal_index_valid")
                    .unwrap();
                let current_fn = self.current_function.ok_or_else(|| {
                    CodegenError::new("string literal index used outside function")
                })?;
                let ok_bb = self
                    .context
                    .append_basic_block(current_fn, "string_literal_index_ok");
                let fail_bb = self
                    .context
                    .append_basic_block(current_fn, "string_literal_index_fail");
                self.builder
                    .build_conditional_branch(valid, ok_bb, fail_bb)
                    .unwrap();

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
                let scalar_ptr = unsafe {
                    self.builder
                        .build_gep(
                            scalar_array.get_type(),
                            scalar_global.as_pointer_value(),
                            &[zero, zero],
                            "string_literal_scalar_ptr",
                        )
                        .unwrap()
                };
                let scalar_char_ptr = unsafe {
                    self.builder
                        .build_gep(
                            self.context.i32_type(),
                            scalar_ptr,
                            &[idx],
                            "string_literal_char_ptr",
                        )
                        .unwrap()
                };
                return Ok(self
                    .builder
                    .build_load(
                        self.context.i32_type(),
                        scalar_char_ptr,
                        "string_literal_char",
                    )
                    .unwrap());
            }

            let string_value = if matches!(object_ty, Some(Type::Ref(_)) | Some(Type::MutRef(_))) {
                self.compile_deref(object)?
            } else {
                obj_val
            };
            return self.compile_utf8_string_index_runtime(string_value.into_pointer_value(), idx);
        }

        if let Some(Type::List(_)) = &deref_object_ty {
            let i64_type = self.context.i64_type();
            let non_negative = self
                .builder
                .build_int_compare(
                    IntPredicate::SGE,
                    idx,
                    i64_type.const_zero(),
                    "index_non_negative",
                )
                .unwrap();

            let (length, data_ptr, elem_ty) =
                if let BasicValueEnum::StructValue(list_struct) = obj_val {
                    let length = self
                        .builder
                        .build_extract_value(list_struct, 1, "list_len")
                        .map_err(|_| CodegenError::new("Invalid list value for index access"))?
                        .into_int_value();
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
                    let len_ptr = unsafe {
                        self.builder
                            .build_gep(
                                list_struct_ty.as_basic_type_enum(),
                                list_ptr,
                                &[zero, i32_type.const_int(1, false)],
                                "list_len_ptr",
                            )
                            .unwrap()
                    };
                    let data_ptr_ptr = unsafe {
                        self.builder
                            .build_gep(
                                list_struct_ty.as_basic_type_enum(),
                                list_ptr,
                                &[zero, i32_type.const_int(2, false)],
                                "list_data_ptr_ptr",
                            )
                            .unwrap()
                    };
                    let length = self
                        .builder
                        .build_load(i64_type, len_ptr, "list_len")
                        .unwrap()
                        .into_int_value();
                    let data_ptr = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            data_ptr_ptr,
                            "list_data_ptr",
                        )
                        .unwrap()
                        .into_pointer_value();
                    let elem_ty = match &deref_object_ty {
                        Some(list_ty @ Type::List(_)) => {
                            self.list_element_layout_from_list_type(list_ty).0
                        }
                        _ => self.list_element_layout_default().0,
                    };
                    (length, data_ptr, elem_ty)
                };

            let in_bounds = self
                .builder
                .build_int_compare(IntPredicate::SLT, idx, length, "index_in_bounds")
                .unwrap();
            let valid = self
                .builder
                .build_and(non_negative, in_bounds, "index_valid")
                .unwrap();
            let current_fn = self
                .current_function
                .ok_or_else(|| CodegenError::new("list index used outside function"))?;
            let ok_bb = self.context.append_basic_block(current_fn, "index_ok");
            let fail_bb = self.context.append_basic_block(current_fn, "index_fail");
            self.builder
                .build_conditional_branch(valid, ok_bb, fail_bb)
                .unwrap();

            self.builder.position_at_end(fail_bb);
            self.emit_runtime_error("List index out of bounds", "list_index_oob")?;

            self.builder.position_at_end(ok_bb);
            let typed_data_ptr = self
                .builder
                .build_pointer_cast(
                    data_ptr,
                    self.context.ptr_type(AddressSpace::default()),
                    "list_typed_data",
                )
                .unwrap();
            let elem_ptr = unsafe {
                self.builder
                    .build_gep(elem_ty, typed_data_ptr, &[idx], "elem")
                    .unwrap()
            };
            return Ok(self.builder.build_load(elem_ty, elem_ptr, "load").unwrap());
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
                .unwrap();
            let elem_ptr = unsafe {
                self.builder
                    .build_gep(elem_ty, typed_data_ptr, &[idx], "elem")
                    .unwrap()
            };
            return Ok(self.builder.build_load(elem_ty, elem_ptr, "load").unwrap());
        }

        let obj_ptr = obj_val.into_pointer_value();
        let elem_ty = match self.infer_object_type(object) {
            Some(list_ty @ Type::List(_)) => self.list_element_layout_from_list_type(&list_ty).0,
            _ => self.list_element_layout_default().0,
        };
        let elem_ptr = unsafe {
            self.builder
                .build_gep(elem_ty, obj_ptr, &[idx], "elem")
                .unwrap()
        };
        Ok(self.builder.build_load(elem_ty, elem_ptr, "load").unwrap())
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
        for (arg, expected_ty) in args.iter().zip(ctor_params.iter()) {
            compiled_args
                .push(self.compile_expr_for_concrete_class_payload(&arg.node, expected_ty)?);
        }

        let args_meta: Vec<BasicMetadataValueEnum> =
            compiled_args.iter().map(|a| (*a).into()).collect();
        let call = self.builder.build_call(func, &args_meta, "new").unwrap();

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
                .unwrap();
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
                .unwrap();
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
                        .unwrap();
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
                        .unwrap();
                    let display_len = self
                        .extract_call_value_with_context(
                            display_len_call,
                            "strlen did not produce a value for string interpolation",
                        )?
                        .into_int_value();
                    rendered_len = self
                        .builder
                        .build_int_add(rendered_len, display_len, "interp_total_expr_len")
                        .unwrap();
                    fmt_str.push_str("%s");
                    args.push(display.into());
                }
            }
        }

        // Allocate the exact output size plus the trailing null terminator.
        let snprintf = self.get_or_declare_snprintf();
        let malloc = self.get_or_declare_malloc();
        let buffer_size = self
            .builder
            .build_int_add(
                rendered_len,
                i64_type.const_int(1, false),
                "interp_buffer_size",
            )
            .unwrap();
        let buffer_call = self
            .builder
            .build_call(malloc, &[buffer_size.into()], "strbuf")
            .unwrap();
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
        let mut snprintf_args: Vec<BasicMetadataValueEnum> = vec![
            buffer.into(),
            buffer_size.into(),
            fmt_global.as_pointer_value().into(),
        ];
        snprintf_args.extend(args);
        self.builder
            .build_call(snprintf, &snprintf_args, "snprintf")
            .unwrap();

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
            .unwrap();
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
            .unwrap();

        // Create basic blocks
        let success_block = self.context.append_basic_block(function, "try.success");
        let error_block = self.context.append_basic_block(function, "try.error");
        let merge_block = self.context.append_basic_block(function, "try.merge");

        // Branch based on tag
        self.builder
            .build_conditional_branch(is_some_or_ok, success_block, error_block)
            .unwrap();

        // Error block: return early with None/Error
        self.builder.position_at_end(error_block);
        match &return_type {
            Type::Option(inner_ty) => {
                // Return None - create Option with tag = 0
                let inner_llvm = self.llvm_type(inner_ty);
                let option_type = self
                    .context
                    .struct_type(&[self.context.i8_type().into(), inner_llvm], false);
                let alloca = self.builder.build_alloca(option_type, "none_ret").unwrap();
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let tag_ptr = unsafe {
                    self.builder
                        .build_gep(
                            option_type.as_basic_type_enum(),
                            alloca,
                            &[zero, i32_type.const_int(0, false)],
                            "tag",
                        )
                        .unwrap()
                };
                self.builder
                    .build_store(tag_ptr, self.context.i8_type().const_int(0, false))
                    .unwrap();
                let loaded = self.builder.build_load(option_type, alloca, "ret").unwrap();
                self.builder.build_return(Some(&loaded)).unwrap();
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
                    .unwrap();
                let alloca = self.builder.build_alloca(result_type, "err_ret").unwrap();
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let tag_ptr = unsafe {
                    self.builder
                        .build_gep(
                            result_type.as_basic_type_enum(),
                            alloca,
                            &[zero, i32_type.const_int(0, false)],
                            "tag",
                        )
                        .unwrap()
                };
                self.builder
                    .build_store(tag_ptr, self.context.i8_type().const_int(0, false))
                    .unwrap();
                let err_ptr = unsafe {
                    self.builder
                        .build_gep(
                            result_type.as_basic_type_enum(),
                            alloca,
                            &[zero, i32_type.const_int(2, false)],
                            "err",
                        )
                        .unwrap()
                };
                self.builder.build_store(err_ptr, err_val).unwrap();
                let loaded = self.builder.build_load(result_type, alloca, "ret").unwrap();
                self.builder.build_return(Some(&loaded)).unwrap();
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
            .unwrap();
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();

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

    pub fn compile_stdlib_function(
        &mut self,
        name: &str,
        args: &[Spanned<Expr>],
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        self.validate_stdlib_arg_count(name, args)?;
        match name {
            // Math functions
            "Math__abs" => {
                self.validate_numeric_stdlib_arg("Math.abs", &args[0].node)?;
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                if val.is_int_value() {
                    let v = val.into_int_value();
                    let current_fn = self
                        .current_function
                        .ok_or_else(|| CodegenError::new("Math.abs used outside function"))?;
                    let overflow_bb = self
                        .context
                        .append_basic_block(current_fn, "math_abs_overflow");
                    let ok_bb = self.context.append_basic_block(current_fn, "math_abs_ok");
                    let min_value = self.context.i64_type().const_int(i64::MIN as u64, true);
                    let is_min_value = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, v, min_value, "math_abs_is_min")
                        .unwrap();
                    self.builder
                        .build_conditional_branch(is_min_value, overflow_bb, ok_bb)
                        .unwrap();

                    self.builder.position_at_end(overflow_bb);
                    self.emit_runtime_error(
                        "Math.abs() overflow on minimum Integer",
                        "math_abs_min_overflow",
                    )?;

                    self.builder.position_at_end(ok_bb);
                    let is_neg = self
                        .builder
                        .build_int_compare(
                            IntPredicate::SLT,
                            v,
                            self.context.i64_type().const_int(0, false),
                            "is_neg",
                        )
                        .unwrap();
                    let neg = self.builder.build_int_neg(v, "neg").unwrap();
                    let result = self.builder.build_select(is_neg, neg, v, "abs").unwrap();
                    Ok(Some(result))
                } else {
                    let fabs = self.get_or_declare_math_func("fabs", true);
                    let call = self.builder.build_call(fabs, &[val.into()], "abs").unwrap();
                    Ok(Some(self.extract_call_value(call)?))
                }
            }
            "Math__min" => {
                self.validate_numeric_stdlib_pair("Math.min", &args[0].node, &args[1].node)?;
                let a_ty = self.infer_builtin_argument_type(&args[0].node);
                let b_ty = self.infer_builtin_argument_type(&args[1].node);
                let a = self.compile_expr_with_expected_type(&args[0].node, &a_ty)?;
                let b = self.compile_expr_with_expected_type(&args[1].node, &b_ty)?;
                if a.is_float_value() || b.is_float_value() {
                    let fmin = self.get_or_declare_math_func2("fmin");
                    let av = if a.is_float_value() {
                        a
                    } else {
                        self.builder
                            .build_signed_int_to_float(
                                a.into_int_value(),
                                self.context.f64_type(),
                                "tofloat",
                            )
                            .unwrap()
                            .into()
                    };
                    let bv = if b.is_float_value() {
                        b
                    } else {
                        self.builder
                            .build_signed_int_to_float(
                                b.into_int_value(),
                                self.context.f64_type(),
                                "tofloat",
                            )
                            .unwrap()
                            .into()
                    };
                    let call = self
                        .builder
                        .build_call(fmin, &[av.into(), bv.into()], "min")
                        .unwrap();
                    Ok(Some(self.extract_call_value(call)?))
                } else {
                    let av = a.into_int_value();
                    let bv = b.into_int_value();
                    let cond = self
                        .builder
                        .build_int_compare(IntPredicate::SLT, av, bv, "cmp")
                        .unwrap();
                    let result = self.builder.build_select(cond, av, bv, "min").unwrap();
                    Ok(Some(result))
                }
            }
            "Math__max" => {
                self.validate_numeric_stdlib_pair("Math.max", &args[0].node, &args[1].node)?;
                let a_ty = self.infer_builtin_argument_type(&args[0].node);
                let b_ty = self.infer_builtin_argument_type(&args[1].node);
                let a = self.compile_expr_with_expected_type(&args[0].node, &a_ty)?;
                let b = self.compile_expr_with_expected_type(&args[1].node, &b_ty)?;
                if a.is_float_value() || b.is_float_value() {
                    let fmax = self.get_or_declare_math_func2("fmax");
                    let av = if a.is_float_value() {
                        a
                    } else {
                        self.builder
                            .build_signed_int_to_float(
                                a.into_int_value(),
                                self.context.f64_type(),
                                "tofloat",
                            )
                            .unwrap()
                            .into()
                    };
                    let bv = if b.is_float_value() {
                        b
                    } else {
                        self.builder
                            .build_signed_int_to_float(
                                b.into_int_value(),
                                self.context.f64_type(),
                                "tofloat",
                            )
                            .unwrap()
                            .into()
                    };
                    let call = self
                        .builder
                        .build_call(fmax, &[av.into(), bv.into()], "max")
                        .unwrap();
                    Ok(Some(self.extract_call_value(call)?))
                } else {
                    let av = a.into_int_value();
                    let bv = b.into_int_value();
                    let cond = self
                        .builder
                        .build_int_compare(IntPredicate::SGT, av, bv, "cmp")
                        .unwrap();
                    let result = self.builder.build_select(cond, av, bv, "max").unwrap();
                    Ok(Some(result))
                }
            }
            "Math__sqrt" => {
                self.validate_numeric_stdlib_arg("Math.sqrt", &args[0].node)?;
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                let sqrt = self.get_or_declare_math_func("sqrt", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(sqrt, &[fval.into()], "sqrt")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)?))
            }
            "Math__pow" => {
                self.validate_numeric_stdlib_pair("Math.pow", &args[0].node, &args[1].node)?;
                let base_ty = self.infer_builtin_argument_type(&args[0].node);
                let exp_ty = self.infer_builtin_argument_type(&args[1].node);
                let base = self.compile_expr_with_expected_type(&args[0].node, &base_ty)?;
                let exp = self.compile_expr_with_expected_type(&args[1].node, &exp_ty)?;
                let pow_fn = self.get_or_declare_math_func2("pow");
                let fbase = if base.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            base.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    base
                };
                let fexp = if exp.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            exp.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    exp
                };
                let call = self
                    .builder
                    .build_call(pow_fn, &[fbase.into(), fexp.into()], "pow")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)?))
            }
            "Math__sin" => {
                self.validate_numeric_stdlib_arg("Math.sin", &args[0].node)?;
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                let sin_fn = self.get_or_declare_math_func("sin", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(sin_fn, &[fval.into()], "sin")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)?))
            }
            "Math__cos" => {
                self.validate_numeric_stdlib_arg("Math.cos", &args[0].node)?;
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                let cos_fn = self.get_or_declare_math_func("cos", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(cos_fn, &[fval.into()], "cos")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)?))
            }
            "Math__tan" => {
                self.validate_numeric_stdlib_arg("Math.tan", &args[0].node)?;
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                let tan_fn = self.get_or_declare_math_func("tan", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(tan_fn, &[fval.into()], "tan")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)?))
            }
            "Math__floor" => {
                self.validate_numeric_stdlib_arg("Math.floor", &args[0].node)?;
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                let floor_fn = self.get_or_declare_math_func("floor", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(floor_fn, &[fval.into()], "floor")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)?))
            }
            "Math__ceil" => {
                self.validate_numeric_stdlib_arg("Math.ceil", &args[0].node)?;
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                let ceil_fn = self.get_or_declare_math_func("ceil", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(ceil_fn, &[fval.into()], "ceil")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)?))
            }
            "Math__round" => {
                self.validate_numeric_stdlib_arg("Math.round", &args[0].node)?;
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                let round_fn = self.get_or_declare_math_func("round", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(round_fn, &[fval.into()], "round")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)?))
            }
            "Math__log" => {
                self.validate_numeric_stdlib_arg("Math.log", &args[0].node)?;
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                let log_fn = self.get_or_declare_math_func("log", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(log_fn, &[fval.into()], "log")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)?))
            }
            "Math__log10" => {
                self.validate_numeric_stdlib_arg("Math.log10", &args[0].node)?;
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                let log10_fn = self.get_or_declare_math_func("log10", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(log10_fn, &[fval.into()], "log10")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)?))
            }
            "Math__exp" => {
                self.validate_numeric_stdlib_arg("Math.exp", &args[0].node)?;
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                let exp_fn = self.get_or_declare_math_func("exp", true);
                let fval = if val.is_int_value() {
                    self.builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap()
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(exp_fn, &[fval.into()], "exp")
                    .unwrap();
                Ok(Some(self.extract_call_value(call)?))
            }

            "Math__random" => {
                let rand_fn = self.get_or_declare_rand();
                let res = self.builder.build_call(rand_fn, &[], "r").unwrap();
                let val = self.extract_call_value(res)?.into_int_value();
                let fval = self
                    .builder
                    .build_unsigned_int_to_float(val, self.context.f64_type(), "rf")
                    .unwrap();
                let rand_max = self.context.f64_type().const_float(32767.0);
                let norm = self.builder.build_float_div(fval, rand_max, "rnd").unwrap();
                Ok(Some(norm.into()))
            }

            "Math__pi" => Ok(Some(
                self.context
                    .f64_type()
                    .const_float(std::f64::consts::PI)
                    .into(),
            )),
            "Math__e" => Ok(Some(
                self.context
                    .f64_type()
                    .const_float(std::f64::consts::E)
                    .into(),
            )),

            // Type conversion functions
            "to_float" => {
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                if !matches!(arg_ty, Type::Integer | Type::Float) {
                    return Err(CodegenError::new(format!(
                        "to_float() requires Integer or Float, got {}",
                        Self::format_diagnostic_type(&arg_ty)
                    )));
                }
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                if val.is_int_value() {
                    let result = self
                        .builder
                        .build_signed_int_to_float(
                            val.into_int_value(),
                            self.context.f64_type(),
                            "tofloat",
                        )
                        .unwrap();
                    Ok(Some(result.into()))
                } else {
                    Ok(Some(val))
                }
            }
            "to_int" => {
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                if !matches!(arg_ty, Type::Integer | Type::Float | Type::String) {
                    return Err(CodegenError::new(format!(
                        "to_int() requires Integer, Float, or String, got {}",
                        Self::format_diagnostic_type(&arg_ty)
                    )));
                }
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                if val.is_float_value() {
                    let result = self
                        .builder
                        .build_float_to_signed_int(
                            val.into_float_value(),
                            self.context.i64_type(),
                            "toint",
                        )
                        .unwrap();
                    Ok(Some(result.into()))
                } else if val.is_pointer_value() {
                    let strtoll = self.get_or_declare_strtoll();
                    let call = self
                        .builder
                        .build_call(
                            strtoll,
                            &[
                                val.into(),
                                self.context
                                    .ptr_type(AddressSpace::default())
                                    .const_null()
                                    .into(),
                                self.context.i32_type().const_int(10, false).into(),
                            ],
                            "toint",
                        )
                        .unwrap();
                    Ok(Some(self.extract_call_value(call)?))
                } else if val.is_int_value() {
                    Ok(Some(val))
                } else {
                    Err(CodegenError::new(
                        "to_int() requires Integer, Float, or String runtime value",
                    ))
                }
            }
            "to_string" => {
                let arg_ty = self.infer_builtin_argument_type(&args[0].node);
                if !Self::supports_display_expr(&args[0].node, &arg_ty) {
                    return Err(CodegenError::new(format!(
                        "to_string() currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got {}",
                        Self::format_diagnostic_type(&arg_ty)
                    )));
                }
                let val = self.compile_expr_with_expected_type(&args[0].node, &arg_ty)?;
                let rendered = self.compile_value_to_display_string(val, &arg_ty)?;
                Ok(Some(rendered.into()))
            }

            // String functions
            "Str__len" => {
                let s_ty = self.infer_builtin_argument_type(&args[0].node);
                if !matches!(s_ty, Type::String) {
                    return Err(CodegenError::new(format!(
                        "Str.len() requires String, got {}",
                        Self::format_diagnostic_type(&s_ty)
                    )));
                }
                let s =
                    self.compile_string_argument_expr(&args[0].node, "Str.len() requires String")?;
                self.compile_utf8_string_length_runtime(s).map(Some)
            }
            "Str__compare" => {
                let s1 = self.compile_string_argument_expr(
                    &args[0].node,
                    "Str.compare() requires String arguments",
                )?;
                let s2 = self.compile_string_argument_expr(
                    &args[1].node,
                    "Str.compare() requires String arguments",
                )?;
                let strcmp_fn = self.get_or_declare_strcmp();
                let call = self
                    .builder
                    .build_call(strcmp_fn, &[s1.into(), s2.into()], "cmp")
                    .unwrap();
                // strcmp returns i32, extend to i64
                let result = self.extract_call_value(call)?.into_int_value();
                let extended = self
                    .builder
                    .build_int_s_extend(result, self.context.i64_type(), "cmp64")
                    .unwrap();
                Ok(Some(extended.into()))
            }
            "Str__concat" => {
                // Allocate new buffer and concatenate
                let s1 = self.compile_string_argument_expr(
                    &args[0].node,
                    "Str.concat() requires String arguments",
                )?;
                let s2 = self.compile_string_argument_expr(
                    &args[1].node,
                    "Str.concat() requires String arguments",
                )?;

                let strlen_fn = self.get_or_declare_strlen();
                let malloc = self.get_or_declare_malloc();
                let strcpy_fn = self.get_or_declare_strcpy();
                let strcat_fn = self.get_or_declare_strcat();

                // Get lengths
                let len1_call = self
                    .builder
                    .build_call(strlen_fn, &[s1.into()], "len1")
                    .unwrap();
                let len1 = self.extract_call_value(len1_call)?.into_int_value();
                let len2_call = self
                    .builder
                    .build_call(strlen_fn, &[s2.into()], "len2")
                    .unwrap();
                let len2 = self.extract_call_value(len2_call)?.into_int_value();

                // Allocate len1 + len2 + 1
                let total_len = self.builder.build_int_add(len1, len2, "total").unwrap();
                let buffer_size = self
                    .builder
                    .build_int_add(
                        total_len,
                        self.context.i64_type().const_int(1, false),
                        "bufsize",
                    )
                    .unwrap();

                let buffer_call = self
                    .builder
                    .build_call(malloc, &[buffer_size.into()], "buf")
                    .unwrap();
                let buffer = self.extract_call_value(buffer_call)?.into_pointer_value();

                // strcpy(buffer, s1)
                self.builder
                    .build_call(strcpy_fn, &[buffer.into(), s1.into()], "")
                    .unwrap();
                // strcat(buffer, s2)
                self.builder
                    .build_call(strcat_fn, &[buffer.into(), s2.into()], "")
                    .unwrap();

                Ok(Some(buffer.into()))
            }

            "Str__upper" => {
                let s = self
                    .compile_string_argument_expr(&args[0].node, "Str.upper() requires String")?;
                let toupper_fn = self.get_or_declare_toupper();
                self.compile_string_transform(s.into(), toupper_fn)
                    .map(Some)
            }

            "Str__lower" => {
                let s = self
                    .compile_string_argument_expr(&args[0].node, "Str.lower() requires String")?;
                let tolower_fn = self.get_or_declare_tolower();
                self.compile_string_transform(s.into(), tolower_fn)
                    .map(Some)
            }

            "Str__trim" => {
                let s_ptr =
                    self.compile_string_argument_expr(&args[0].node, "Str.trim() requires String")?;
                let strlen_fn = self.get_or_declare_strlen();
                let isspace_fn = self.get_or_declare_isspace();
                let malloc_fn = self.get_or_declare_malloc();
                let strncpy_fn = self.get_or_declare_strncpy();

                let len_call = self
                    .builder
                    .build_call(strlen_fn, &[s_ptr.into()], "len")
                    .unwrap();
                let len = self.extract_call_value(len_call)?.into_int_value();

                // Find start (first non-space)
                let start_ptr = self
                    .builder
                    .build_alloca(self.context.i64_type(), "start")
                    .unwrap();
                self.builder
                    .build_store(start_ptr, self.context.i64_type().const_int(0, false))
                    .unwrap();

                let cur_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Str.trim used outside function"))?;
                let start_cond = self.context.append_basic_block(cur_fn, "trim.start.cond");
                let start_body = self.context.append_basic_block(cur_fn, "trim.start.body");
                let start_after = self.context.append_basic_block(cur_fn, "trim.start.after");
                self.builder.build_unconditional_branch(start_cond).unwrap();

                self.builder.position_at_end(start_cond);
                let start_val = self
                    .builder
                    .build_load(self.context.i64_type(), start_ptr, "s")
                    .unwrap()
                    .into_int_value();
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, start_val, len, "bounds")
                    .unwrap();
                let char_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), s_ptr, &[start_val], "")
                        .unwrap()
                };
                let char_val = self
                    .builder
                    .build_load(self.context.i8_type(), char_ptr, "")
                    .unwrap();
                let char_i32 = self
                    .builder
                    .build_int_s_extend(char_val.into_int_value(), self.context.i32_type(), "")
                    .unwrap();
                let is_space_call = self
                    .builder
                    .build_call(isspace_fn, &[char_i32.into()], "")
                    .unwrap();
                let is_space = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        self.extract_call_value(is_space_call)?.into_int_value(),
                        self.context.i32_type().const_int(0, false),
                        "",
                    )
                    .unwrap();
                let cond = self.builder.build_and(in_bounds, is_space, "").unwrap();
                self.builder
                    .build_conditional_branch(cond, start_body, start_after)
                    .unwrap();

                self.builder.position_at_end(start_body);
                let next_start = self
                    .builder
                    .build_int_add(start_val, self.context.i64_type().const_int(1, false), "")
                    .unwrap();
                self.builder.build_store(start_ptr, next_start).unwrap();
                self.builder.build_unconditional_branch(start_cond).unwrap();

                self.builder.position_at_end(start_after);
                let start_final = self
                    .builder
                    .build_load(self.context.i64_type(), start_ptr, "start_f")
                    .unwrap()
                    .into_int_value();

                // Find end (last non-space)
                let end_ptr = self
                    .builder
                    .build_alloca(self.context.i64_type(), "end")
                    .unwrap();
                self.builder.build_store(end_ptr, len).unwrap();

                let end_cond = self.context.append_basic_block(cur_fn, "trim.end.cond");
                let end_body = self.context.append_basic_block(cur_fn, "trim.end.body");
                let end_after = self.context.append_basic_block(cur_fn, "trim.end.after");
                self.builder.build_unconditional_branch(end_cond).unwrap();

                self.builder.position_at_end(end_cond);
                let end_val = self
                    .builder
                    .build_load(self.context.i64_type(), end_ptr, "e")
                    .unwrap()
                    .into_int_value();
                let gt_start = self
                    .builder
                    .build_int_compare(IntPredicate::SGT, end_val, start_final, "gt_start")
                    .unwrap();
                let last_idx = self
                    .builder
                    .build_int_sub(end_val, self.context.i64_type().const_int(1, false), "")
                    .unwrap();
                let char_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), s_ptr, &[last_idx], "")
                        .unwrap()
                };
                let char_val = self
                    .builder
                    .build_load(self.context.i8_type(), char_ptr, "")
                    .unwrap();
                let char_i32 = self
                    .builder
                    .build_int_s_extend(char_val.into_int_value(), self.context.i32_type(), "")
                    .unwrap();
                let is_space_call = self
                    .builder
                    .build_call(isspace_fn, &[char_i32.into()], "")
                    .unwrap();
                let is_space = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        self.extract_call_value(is_space_call)?.into_int_value(),
                        self.context.i32_type().const_int(0, false),
                        "",
                    )
                    .unwrap();
                let cond = self.builder.build_and(gt_start, is_space, "").unwrap();
                self.builder
                    .build_conditional_branch(cond, end_body, end_after)
                    .unwrap();

                self.builder.position_at_end(end_body);
                let next_end = self
                    .builder
                    .build_int_sub(end_val, self.context.i64_type().const_int(1, false), "")
                    .unwrap();
                self.builder.build_store(end_ptr, next_end).unwrap();
                self.builder.build_unconditional_branch(end_cond).unwrap();

                self.builder.position_at_end(end_after);
                let end_final = self
                    .builder
                    .build_load(self.context.i64_type(), end_ptr, "end_f")
                    .unwrap()
                    .into_int_value();

                // Allocate and copy result
                let new_len = self
                    .builder
                    .build_int_sub(end_final, start_final, "new_len")
                    .unwrap();
                let alloc_size = self
                    .builder
                    .build_int_add(
                        new_len,
                        self.context.i64_type().const_int(1, false),
                        "alloc",
                    )
                    .unwrap();
                let buf_call = self
                    .builder
                    .build_call(malloc_fn, &[alloc_size.into()], "buf")
                    .unwrap();
                let buf = self.extract_call_value(buf_call)?.into_pointer_value();

                let src_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), s_ptr, &[start_final], "src")
                        .unwrap()
                };
                self.builder
                    .build_call(
                        strncpy_fn,
                        &[buf.into(), src_ptr.into(), new_len.into()],
                        "",
                    )
                    .unwrap();

                // Null terminate
                let term_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), buf, &[new_len], "")
                        .unwrap()
                };
                self.builder
                    .build_store(term_ptr, self.context.i8_type().const_int(0, false))
                    .unwrap();

                Ok(Some(buf.into()))
            }

            "Str__contains" => {
                let s = self.compile_string_argument_expr(
                    &args[0].node,
                    "Str.contains() requires two String arguments",
                )?;
                let sub = self.compile_string_argument_expr(
                    &args[1].node,
                    "Str.contains() requires two String arguments",
                )?;
                let strstr = self.get_or_declare_strstr();
                let res = self
                    .builder
                    .build_call(strstr, &[s.into(), sub.into()], "pos")
                    .unwrap();
                let ptr = self.extract_call_value(res)?.into_pointer_value();
                let is_null = self.builder.build_is_null(ptr, "not_found").unwrap();
                let found = self.builder.build_not(is_null, "found").unwrap();
                Ok(Some(found.into()))
            }
            "Str__startsWith" => {
                let s = self.compile_string_argument_expr(
                    &args[0].node,
                    "Str.startsWith() requires two String arguments",
                )?;
                let pre = self.compile_string_argument_expr(
                    &args[1].node,
                    "Str.startsWith() requires two String arguments",
                )?;
                let strlen = self.get_or_declare_strlen();
                let strncmp = self.get_or_declare_strncmp();

                let pre_len = self
                    .builder
                    .build_call(strlen, &[pre.into()], "pre_len")
                    .unwrap();
                let res = self
                    .builder
                    .build_call(
                        strncmp,
                        &[
                            s.into(),
                            pre.into(),
                            self.extract_call_value(pre_len)?.into_int_value().into(),
                        ],
                        "cmp",
                    )
                    .unwrap();
                let is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        self.extract_call_value(res)?.into_int_value(),
                        self.context.i32_type().const_int(0, false),
                        "is_zero",
                    )
                    .unwrap();
                Ok(Some(is_zero.into()))
            }
            "Str__endsWith" => {
                let s = self.compile_string_argument_expr(
                    &args[0].node,
                    "Str.endsWith() requires two String arguments",
                )?;
                let suf = self.compile_string_argument_expr(
                    &args[1].node,
                    "Str.endsWith() requires two String arguments",
                )?;
                let strlen = self.get_or_declare_strlen();
                let strcmp = self.get_or_declare_strcmp();

                let s_len = self
                    .builder
                    .build_call(strlen, &[s.into()], "s_len")
                    .unwrap();
                let suf_len = self
                    .builder
                    .build_call(strlen, &[suf.into()], "suf_len")
                    .unwrap();

                let s_len_val = self.extract_call_value(s_len)?.into_int_value();
                let suf_len_val = self.extract_call_value(suf_len)?.into_int_value();

                let can_end = self
                    .builder
                    .build_int_compare(IntPredicate::UGE, s_len_val, suf_len_val, "can_end")
                    .unwrap();

                let start_idx = self
                    .builder
                    .build_int_sub(s_len_val, suf_len_val, "")
                    .unwrap();
                let s_suffix_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), s, &[start_idx], "")
                        .unwrap()
                };

                let res = self
                    .builder
                    .build_call(strcmp, &[s_suffix_ptr.into(), suf.into()], "cmp")
                    .unwrap();
                let is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        self.extract_call_value(res)?.into_int_value(),
                        self.context.i32_type().const_int(0, false),
                        "is_zero",
                    )
                    .unwrap();

                let final_res = self.builder.build_and(can_end, is_zero, "").unwrap();
                Ok(Some(final_res.into()))
            }

            // I/O functions
            "read_line" => {
                // Read a line from stdin with a growing buffer so long lines do not
                // truncate and we do not depend on platform-specific stdin symbols.
                let malloc = self.get_or_declare_malloc();
                let realloc = self.get_or_declare_realloc();
                let getchar_fn = self.get_or_declare_getchar();

                let i8_type = self.context.i8_type();
                let i32_type = self.context.i32_type();
                let i64_type = self.context.i64_type();
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let chunk_chars = i64_type.const_int(1024, false);
                let initial_capacity = i64_type.const_int(1025, false);
                let buffer_call = self
                    .builder
                    .build_call(malloc, &[initial_capacity.into()], "linebuf")
                    .unwrap();
                let buffer = self.extract_call_value(buffer_call)?.into_pointer_value();
                let buffer_slot = self
                    .builder
                    .build_alloca(ptr_type, "read_line_buffer_slot")
                    .unwrap();
                let capacity_slot = self
                    .builder
                    .build_alloca(i64_type, "read_line_capacity_slot")
                    .unwrap();
                let total_read_slot = self
                    .builder
                    .build_alloca(i64_type, "read_line_total_slot")
                    .unwrap();
                self.builder.build_store(buffer_slot, buffer).unwrap();
                self.builder
                    .build_store(capacity_slot, initial_capacity)
                    .unwrap();
                self.builder
                    .build_store(total_read_slot, i64_type.const_zero())
                    .unwrap();
                self.builder
                    .build_store(buffer, i8_type.const_zero())
                    .unwrap();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("read_line used outside function"))?;
                let read_cond_bb = self
                    .context
                    .append_basic_block(current_fn, "read_line.cond");
                let read_body_bb = self
                    .context
                    .append_basic_block(current_fn, "read_line.body");
                let append_bb = self
                    .context
                    .append_basic_block(current_fn, "read_line.append");
                let grow_bb = self
                    .context
                    .append_basic_block(current_fn, "read_line.grow");
                let grow_ok_bb = self
                    .context
                    .append_basic_block(current_fn, "read_line.grow.ok");
                let eof_bb = self.context.append_basic_block(current_fn, "read_line.eof");
                let done_bb = self
                    .context
                    .append_basic_block(current_fn, "read_line.done");
                let oom_bb = self.context.append_basic_block(current_fn, "read_line.oom");

                self.builder
                    .build_unconditional_branch(read_cond_bb)
                    .unwrap();

                self.builder.position_at_end(read_cond_bb);
                let current_capacity = self
                    .builder
                    .build_load(i64_type, capacity_slot, "read_line_capacity")
                    .unwrap()
                    .into_int_value();
                let current_total = self
                    .builder
                    .build_load(i64_type, total_read_slot, "read_line_total")
                    .unwrap()
                    .into_int_value();
                let remaining_capacity = self
                    .builder
                    .build_int_sub(
                        current_capacity,
                        current_total,
                        "read_line_remaining_capacity",
                    )
                    .unwrap();
                let enough_room = self
                    .builder
                    .build_int_compare(
                        IntPredicate::UGT,
                        remaining_capacity,
                        i64_type.const_int(1, false),
                        "read_line_enough_room",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(enough_room, read_body_bb, grow_bb)
                    .unwrap();

                self.builder.position_at_end(read_body_bb);
                let getchar_call = self
                    .builder
                    .build_call(getchar_fn, &[], "read_line_char")
                    .unwrap();
                let char_i32 = self.extract_call_value(getchar_call)?.into_int_value();
                let is_eof = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        char_i32,
                        i32_type.const_int(u32::MAX as u64, false),
                        "read_line_is_eof",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(is_eof, eof_bb, append_bb)
                    .unwrap();

                self.builder.position_at_end(eof_bb);
                self.builder.build_unconditional_branch(done_bb).unwrap();

                self.builder.position_at_end(append_bb);
                let current_buffer = self
                    .builder
                    .build_load(ptr_type, buffer_slot, "read_line_buffer")
                    .unwrap()
                    .into_pointer_value();
                let write_ptr = unsafe {
                    self.builder
                        .build_gep(
                            i8_type,
                            current_buffer,
                            &[current_total],
                            "read_line_write_ptr",
                        )
                        .unwrap()
                };
                let char_i8 = self
                    .builder
                    .build_int_truncate(char_i32, i8_type, "read_line_char_i8")
                    .unwrap();
                self.builder.build_store(write_ptr, char_i8).unwrap();
                let next_total = self
                    .builder
                    .build_int_add(
                        current_total,
                        i64_type.const_int(1, false),
                        "read_line_next_total",
                    )
                    .unwrap();
                self.builder
                    .build_store(total_read_slot, next_total)
                    .unwrap();
                let term_ptr = unsafe {
                    self.builder
                        .build_gep(i8_type, current_buffer, &[next_total], "read_line_term_ptr")
                        .unwrap()
                };
                self.builder
                    .build_store(term_ptr, i8_type.const_zero())
                    .unwrap();
                let saw_newline = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        char_i8,
                        i8_type.const_int(b'\n' as u64, false),
                        "read_line_saw_newline",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(saw_newline, done_bb, read_cond_bb)
                    .unwrap();

                self.builder.position_at_end(grow_bb);
                let grown_capacity = self
                    .builder
                    .build_int_add(current_capacity, chunk_chars, "read_line_new_capacity")
                    .unwrap();
                let grown_buffer = self
                    .builder
                    .build_load(ptr_type, buffer_slot, "read_line_grow_buffer")
                    .unwrap()
                    .into_pointer_value();
                let realloc_call = self
                    .builder
                    .build_call(
                        realloc,
                        &[grown_buffer.into(), grown_capacity.into()],
                        "read_line_realloc",
                    )
                    .unwrap();
                let realloc_ptr = self.extract_call_value(realloc_call)?.into_pointer_value();
                let realloc_failed = self
                    .builder
                    .build_is_null(realloc_ptr, "read_line_realloc_failed")
                    .unwrap();
                self.builder
                    .build_conditional_branch(realloc_failed, oom_bb, grow_ok_bb)
                    .unwrap();

                self.builder.position_at_end(oom_bb);
                self.emit_runtime_error("read_line() out of memory", "read_line_out_of_memory")?;

                self.builder.position_at_end(grow_ok_bb);
                self.builder.build_store(buffer_slot, realloc_ptr).unwrap();
                self.builder
                    .build_store(capacity_slot, grown_capacity)
                    .unwrap();
                self.builder
                    .build_unconditional_branch(read_cond_bb)
                    .unwrap();

                self.builder.position_at_end(done_bb);
                let final_buffer = self
                    .builder
                    .build_load(ptr_type, buffer_slot, "read_line_final_buffer")
                    .unwrap();

                Ok(Some(final_buffer))
            }
            "System__exit" | "exit" => {
                let code_ty = self.infer_builtin_argument_type(&args[0].node);
                if !matches!(code_ty, Type::Integer) {
                    return Err(CodegenError::new("exit() requires Integer code"));
                }
                let code = self.compile_expr_with_expected_type(&args[0].node, &code_ty)?;
                let exit_fn = self.get_or_declare_exit();
                let code_i32 = self
                    .builder
                    .build_int_truncate(code.into_int_value(), self.context.i32_type(), "exitcode")
                    .unwrap();
                self.builder
                    .build_call(exit_fn, &[code_i32.into()], "")
                    .unwrap();
                Ok(None) // void function
            }
            "range" => {
                // range(start, end) or range(start, end, step)
                // Returns a Range struct { start, end, step, current }
                let arg_types = args
                    .iter()
                    .map(|arg| self.infer_builtin_argument_type(&arg.node))
                    .collect::<Vec<_>>();
                let all_integer = arg_types.iter().all(|ty| matches!(ty, Type::Integer));
                let all_float = arg_types.iter().all(|ty| matches!(ty, Type::Float));
                if !all_integer && !all_float {
                    return Err(CodegenError::new(
                        "range() arguments must be all Integer or all Float",
                    ));
                }
                if let Some(step) = args.get(2) {
                    if matches!(
                        TypeChecker::eval_numeric_const_expr(&step.node),
                        Some(NumericConst::Integer(0) | NumericConst::Float(0.0))
                    ) {
                        return Err(CodegenError::new("range() step cannot be 0"));
                    }
                }

                let start = self.compile_expr_with_expected_type(&args[0].node, &arg_types[0])?;
                let end = self.compile_expr_with_expected_type(&args[1].node, &arg_types[1])?;
                let step = if args.len() == 3 {
                    self.compile_expr_with_expected_type(&args[2].node, &arg_types[2])?
                } else {
                    match start {
                        BasicValueEnum::IntValue(v) => v.get_type().const_int(1, false).into(),
                        BasicValueEnum::FloatValue(v) => v.get_type().const_float(1.0).into(),
                        _ => {
                            return Err(CodegenError::new(
                                "range() codegen supports only Integer and Float elements",
                            ));
                        }
                    }
                };

                // Allocate and initialize Range struct
                let range_ptr = self.create_range(start, end, step)?;
                Ok(Some(range_ptr.into()))
            }

            // File I/O
            "File__write" => {
                let path = self.compile_string_argument_expr(
                    &args[0].node,
                    "File.write() path must be String",
                )?;
                let content = self.compile_string_argument_expr(
                    &args[1].node,
                    "File.write() content must be String",
                )?;

                let fopen = self.get_or_declare_fopen();
                let fputs = self.get_or_declare_fputs();
                let fclose = self.get_or_declare_fclose();

                let mode = self.context.const_string(b"w", true);
                let mode_global = self.module.add_global(mode.get_type(), None, "mode_w");
                mode_global.set_linkage(Linkage::Private);
                mode_global.set_initializer(&mode);

                let file_call = self
                    .builder
                    .build_call(
                        fopen,
                        &[path.into(), mode_global.as_pointer_value().into()],
                        "file",
                    )
                    .unwrap();
                let file_ptr = self.extract_call_value(file_call)?.into_pointer_value();

                let is_null = self.builder.build_is_null(file_ptr, "is_null").unwrap();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("File.write used outside function"))?;
                let success_block = self.context.append_basic_block(current_fn, "file.success");
                let fail_block = self.context.append_basic_block(current_fn, "file.fail");
                let merge_block = self.context.append_basic_block(current_fn, "file.merge");
                let write_ok_block = self.context.append_basic_block(current_fn, "file.write_ok");
                let write_fail_block = self
                    .context
                    .append_basic_block(current_fn, "file.write_fail");

                self.builder
                    .build_conditional_branch(is_null, fail_block, success_block)
                    .unwrap();

                // Fail
                self.builder.position_at_end(fail_block);
                self.builder
                    .build_unconditional_branch(merge_block)
                    .unwrap();

                // Success
                self.builder.position_at_end(success_block);
                let write_result = self
                    .builder
                    .build_call(fputs, &[content.into(), file_ptr.into()], "write")
                    .unwrap();
                let write_code = self.extract_call_value(write_result)?.into_int_value();
                let close_result = self
                    .builder
                    .build_call(fclose, &[file_ptr.into()], "close")
                    .unwrap();
                let close_code = self.extract_call_value(close_result)?.into_int_value();
                let write_failed = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        write_code,
                        self.context.i32_type().const_zero(),
                        "file_write_failed",
                    )
                    .unwrap();
                let close_failed = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        close_code,
                        self.context.i32_type().const_zero(),
                        "file_close_failed",
                    )
                    .unwrap();
                let io_failed = self
                    .builder
                    .build_or(write_failed, close_failed, "file_io_failed")
                    .unwrap();
                self.builder
                    .build_conditional_branch(io_failed, write_fail_block, write_ok_block)
                    .unwrap();

                self.builder.position_at_end(write_fail_block);
                self.builder
                    .build_unconditional_branch(merge_block)
                    .unwrap();

                self.builder.position_at_end(write_ok_block);
                self.builder
                    .build_unconditional_branch(merge_block)
                    .unwrap();

                // Merge
                self.builder.position_at_end(merge_block);
                let phi = self
                    .builder
                    .build_phi(self.context.bool_type(), "result")
                    .unwrap();
                let true_val = self.context.bool_type().const_int(1, false);
                let false_val = self.context.bool_type().const_int(0, false);
                phi.add_incoming(&[
                    (&false_val, fail_block),
                    (&false_val, write_fail_block),
                    (&true_val, write_ok_block),
                ]);

                Ok(Some(phi.as_basic_value()))
            }

            "File__read" => {
                let path_ty = self.infer_builtin_argument_type(&args[0].node);
                if !matches!(path_ty, Type::String) {
                    return Err(CodegenError::new(format!(
                        "File.read() requires String path, got {}",
                        Self::format_diagnostic_type(&path_ty)
                    )));
                }
                let path = self.compile_string_argument_expr(
                    &args[0].node,
                    "File.read() requires String path",
                )?;

                let fopen = self.get_or_declare_fopen();
                let fseek = self.get_or_declare_fseek();
                let ftell = self.get_or_declare_ftell();
                let rewind = self.get_or_declare_rewind();
                let fread = self.get_or_declare_fread();
                let fclose = self.get_or_declare_fclose();
                let malloc = self.get_or_declare_malloc();

                let mode = self.context.const_string(b"rb", true); // Binary mode to get exact bytes
                let mode_global = self.module.add_global(mode.get_type(), None, "mode_r");
                mode_global.set_linkage(Linkage::Private);
                mode_global.set_initializer(&mode);

                let file_call = self
                    .builder
                    .build_call(
                        fopen,
                        &[path.into(), mode_global.as_pointer_value().into()],
                        "file",
                    )
                    .unwrap();
                let file_ptr = self.extract_call_value(file_call)?.into_pointer_value();

                let is_null = self.builder.build_is_null(file_ptr, "is_null").unwrap();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("File.read used outside function"))?;
                let success_block = self.context.append_basic_block(current_fn, "read.success");
                let fail_block = self.context.append_basic_block(current_fn, "read.fail");
                let seek_ok_block = self.context.append_basic_block(current_fn, "read.seek_ok");
                let seek_fail_block = self
                    .context
                    .append_basic_block(current_fn, "read.seek_fail");
                let size_ok_block = self.context.append_basic_block(current_fn, "read.size_ok");
                let size_fail_block = self
                    .context
                    .append_basic_block(current_fn, "read.size_fail");

                self.builder
                    .build_conditional_branch(is_null, fail_block, success_block)
                    .unwrap();

                self.builder.position_at_end(fail_block);
                self.emit_runtime_error(
                    "File.read() failed to open file",
                    "file_read_open_failed",
                )?;

                // Success
                self.builder.position_at_end(success_block);
                // fseek(f, 0, SEEK_END)
                let seek_end = self.context.i32_type().const_int(2, false); // SEEK_END = 2
                let zero = self.context.i64_type().const_int(0, false);
                let seek_result = self
                    .builder
                    .build_call(fseek, &[file_ptr.into(), zero.into(), seek_end.into()], "")
                    .unwrap();
                let seek_code = self.extract_call_value(seek_result)?.into_int_value();
                let seek_succeeded = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        seek_code,
                        self.context.i32_type().const_zero(),
                        "file_read_seek_succeeded",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(seek_succeeded, seek_ok_block, seek_fail_block)
                    .unwrap();

                self.builder.position_at_end(seek_fail_block);
                self.emit_runtime_error(
                    "File.read() requires a seekable regular file",
                    "file_read_non_seekable",
                )?;

                self.builder.position_at_end(seek_ok_block);

                // size = ftell(f)
                let size_call = self
                    .builder
                    .build_call(ftell, &[file_ptr.into()], "size")
                    .unwrap();
                let size = self.extract_call_value(size_call)?.into_int_value();
                let size_non_negative = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SGE,
                        size,
                        self.context.i64_type().const_zero(),
                        "file_read_size_non_negative",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(size_non_negative, size_ok_block, size_fail_block)
                    .unwrap();

                self.builder.position_at_end(size_fail_block);
                self.emit_runtime_error(
                    "File.read() requires a seekable regular file",
                    "file_read_invalid_size",
                )?;

                self.builder.position_at_end(size_ok_block);

                // rewind(f)
                self.builder
                    .build_call(rewind, &[file_ptr.into()], "")
                    .unwrap();

                // buffer = malloc(size + 1)
                let one = self.context.i64_type().const_int(1, false);
                let alloc_size = self.builder.build_int_add(size, one, "alloc_size").unwrap();
                let buffer_call = self
                    .builder
                    .build_call(malloc, &[alloc_size.into()], "buffer")
                    .unwrap();
                let buffer = self.extract_call_value(buffer_call)?.into_pointer_value();

                // fread(buffer, 1, size, f)
                let size_size_t = size; // Assuming size_t is i64
                self.builder
                    .build_call(
                        fread,
                        &[
                            buffer.into(),
                            one.into(),
                            size_size_t.into(),
                            file_ptr.into(),
                        ],
                        "",
                    )
                    .unwrap();

                let scan_index_slot = self
                    .builder
                    .build_alloca(self.context.i64_type(), "file_read_scan_index")
                    .unwrap();
                self.builder
                    .build_store(scan_index_slot, self.context.i64_type().const_zero())
                    .unwrap();

                let scan_cond_block = self
                    .context
                    .append_basic_block(current_fn, "file_read_scan_cond");
                let scan_body_block = self
                    .context
                    .append_basic_block(current_fn, "file_read_scan_body");
                let scan_next_block = self
                    .context
                    .append_basic_block(current_fn, "file_read_scan_next");
                let scan_done_block = self
                    .context
                    .append_basic_block(current_fn, "file_read_scan_done");
                let scan_fail_block = self
                    .context
                    .append_basic_block(current_fn, "file_read_scan_fail");

                self.builder
                    .build_unconditional_branch(scan_cond_block)
                    .unwrap();

                self.builder.position_at_end(scan_cond_block);
                let scan_index = self
                    .builder
                    .build_load(
                        self.context.i64_type(),
                        scan_index_slot,
                        "file_read_scan_index_value",
                    )
                    .unwrap()
                    .into_int_value();
                let scan_has_more = self
                    .builder
                    .build_int_compare(
                        IntPredicate::ULT,
                        scan_index,
                        size,
                        "file_read_scan_has_more",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(scan_has_more, scan_body_block, scan_done_block)
                    .unwrap();

                self.builder.position_at_end(scan_body_block);
                let scan_byte_ptr = unsafe {
                    self.builder
                        .build_gep(
                            self.context.i8_type(),
                            buffer,
                            &[scan_index],
                            "file_read_scan_byte_ptr",
                        )
                        .unwrap()
                };
                let scan_byte = self
                    .builder
                    .build_load(self.context.i8_type(), scan_byte_ptr, "file_read_scan_byte")
                    .unwrap()
                    .into_int_value();
                let scan_is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        scan_byte,
                        self.context.i8_type().const_zero(),
                        "file_read_scan_is_zero",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(scan_is_zero, scan_fail_block, scan_next_block)
                    .unwrap();

                self.builder.position_at_end(scan_fail_block);
                self.emit_runtime_error("File.read() cannot load NUL bytes", "file_read_nul_byte")?;

                self.builder.position_at_end(scan_next_block);
                let next_scan_index = self
                    .builder
                    .build_int_add(
                        scan_index,
                        self.context.i64_type().const_int(1, false),
                        "file_read_next_scan_index",
                    )
                    .unwrap();
                self.builder
                    .build_store(scan_index_slot, next_scan_index)
                    .unwrap();
                self.builder
                    .build_unconditional_branch(scan_cond_block)
                    .unwrap();

                self.builder.position_at_end(scan_done_block);
                // buffer[size] = 0 (null terminate)
                let term_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), buffer, &[size], "term_ptr")
                        .unwrap()
                };
                self.builder
                    .build_store(term_ptr, self.context.i8_type().const_int(0, false))
                    .unwrap();

                self.compile_utf8_string_length_runtime(buffer)?;

                // fclose(f)
                self.builder
                    .build_call(fclose, &[file_ptr.into()], "")
                    .unwrap();

                Ok(Some(buffer.into()))
            }

            "File__exists" => {
                let path_ty = self.infer_builtin_argument_type(&args[0].node);
                if !matches!(path_ty, Type::String) {
                    return Err(CodegenError::new(format!(
                        "File.exists() requires String path, got {}",
                        Self::format_diagnostic_type(&path_ty)
                    )));
                }
                let path = self.compile_string_argument_expr(
                    &args[0].node,
                    "File.exists() requires String path",
                )?;
                let fopen = self.get_or_declare_fopen();
                let fclose = self.get_or_declare_fclose();
                let fread = self.get_or_declare_fread();
                let ferror = self.get_or_declare_ferror();

                let mode = self.context.const_string(b"rb", true);
                let mode_global = self.module.add_global(mode.get_type(), None, "mode_r");
                mode_global.set_linkage(Linkage::Private);
                mode_global.set_initializer(&mode);

                let file_call = self
                    .builder
                    .build_call(
                        fopen,
                        &[path.into(), mode_global.as_pointer_value().into()],
                        "file",
                    )
                    .unwrap();
                let file_ptr = self.extract_call_value(file_call)?.into_pointer_value();

                let is_null = self.builder.build_is_null(file_ptr, "is_null").unwrap();
                let alloca_exists_slot = self
                    .builder
                    .build_alloca(self.context.bool_type(), "exists_result_slot")
                    .unwrap();
                self.builder
                    .build_store(alloca_exists_slot, self.context.bool_type().const_zero())
                    .unwrap();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("File.exists used outside function"))?;
                let probe_block = self.context.append_basic_block(current_fn, "exists.probe");
                let end_block = self.context.append_basic_block(current_fn, "exists.end");

                self.builder
                    .build_conditional_branch(is_null, end_block, probe_block)
                    .unwrap();

                self.builder.position_at_end(probe_block);
                let buf_slot = self
                    .builder
                    .build_alloca(self.context.i8_type(), "exists_buf")
                    .unwrap();
                let one_i64 = self.context.i64_type().const_int(1, false);
                self.builder
                    .build_call(
                        fread,
                        &[
                            buf_slot.into(),
                            one_i64.into(),
                            one_i64.into(),
                            file_ptr.into(),
                        ],
                        "",
                    )
                    .unwrap();
                let err_call = self
                    .builder
                    .build_call(ferror, &[file_ptr.into()], "exists_err")
                    .unwrap();
                let err_code = self.extract_call_value(err_call)?.into_int_value();
                self.builder
                    .build_call(fclose, &[file_ptr.into()], "")
                    .unwrap();
                let is_regular = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        err_code,
                        self.context.i32_type().const_zero(),
                        "exists_is_regular",
                    )
                    .unwrap();
                self.builder
                    .build_store(alloca_exists_slot, is_regular)
                    .unwrap();
                self.builder.build_unconditional_branch(end_block).unwrap();

                self.builder.position_at_end(end_block);
                let final_exists = self
                    .builder
                    .build_load(
                        self.context.bool_type(),
                        alloca_exists_slot,
                        "exists_final_value",
                    )
                    .unwrap();
                Ok(Some(final_exists))
            }

            "File__delete" => {
                let path_ty = self.infer_builtin_argument_type(&args[0].node);
                if !matches!(path_ty, Type::String) {
                    return Err(CodegenError::new(format!(
                        "File.delete() requires String path, got {}",
                        Self::format_diagnostic_type(&path_ty)
                    )));
                }
                let path = self.compile_string_argument_expr(
                    &args[0].node,
                    "File.delete() requires String path",
                )?;
                let fopen = self.get_or_declare_fopen();
                let fclose = self.get_or_declare_fclose();
                let fread = self.get_or_declare_fread();
                let ferror = self.get_or_declare_ferror();
                let remove = self.get_or_declare_remove();

                let mode = self.context.const_string(b"rb", true);
                let mode_global = self
                    .module
                    .add_global(mode.get_type(), None, "mode_delete_r");
                mode_global.set_linkage(Linkage::Private);
                mode_global.set_initializer(&mode);

                let file_call = self
                    .builder
                    .build_call(
                        fopen,
                        &[path.into(), mode_global.as_pointer_value().into()],
                        "delete_file_probe",
                    )
                    .unwrap();
                let file_ptr = self.extract_call_value(file_call)?.into_pointer_value();
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("File.delete used outside function"))?;
                let probe_open_bb = self
                    .context
                    .append_basic_block(current_fn, "file_delete_probe_open");
                let delete_remove_bb = self
                    .context
                    .append_basic_block(current_fn, "file_delete_remove");
                let delete_fail_bb = self
                    .context
                    .append_basic_block(current_fn, "file_delete_fail");
                let delete_merge_bb = self
                    .context
                    .append_basic_block(current_fn, "file_delete_merge");
                let delete_result_slot = self
                    .builder
                    .build_alloca(self.context.bool_type(), "file_delete_result_slot")
                    .unwrap();
                self.builder
                    .build_store(delete_result_slot, self.context.bool_type().const_zero())
                    .unwrap();

                let probe_is_null = self
                    .builder
                    .build_is_null(file_ptr, "delete_probe_is_null")
                    .unwrap();
                self.builder
                    .build_conditional_branch(probe_is_null, delete_fail_bb, probe_open_bb)
                    .unwrap();

                self.builder.position_at_end(probe_open_bb);
                let buf_slot = self
                    .builder
                    .build_alloca(self.context.i8_type(), "delete_probe_buf")
                    .unwrap();
                let one_i64 = self.context.i64_type().const_int(1, false);
                self.builder
                    .build_call(
                        fread,
                        &[
                            buf_slot.into(),
                            one_i64.into(),
                            one_i64.into(),
                            file_ptr.into(),
                        ],
                        "",
                    )
                    .unwrap();
                let err_call = self
                    .builder
                    .build_call(ferror, &[file_ptr.into()], "delete_probe_err")
                    .unwrap();
                let err_code = self.extract_call_value(err_call)?.into_int_value();
                self.builder
                    .build_call(fclose, &[file_ptr.into()], "delete_probe_close")
                    .unwrap();
                let probe_is_regular = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        err_code,
                        self.context.i32_type().const_zero(),
                        "delete_probe_is_regular",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(probe_is_regular, delete_remove_bb, delete_fail_bb)
                    .unwrap();

                self.builder.position_at_end(delete_remove_bb);
                let res_call = self
                    .builder
                    .build_call(remove, &[path.into()], "res")
                    .unwrap();
                let res = self.extract_call_value(res_call)?.into_int_value();
                let zero = self.context.i32_type().const_int(0, false);
                let success = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, res, zero, "success")
                    .unwrap();
                self.builder
                    .build_store(delete_result_slot, success)
                    .unwrap();
                self.builder
                    .build_unconditional_branch(delete_merge_bb)
                    .unwrap();

                self.builder.position_at_end(delete_fail_bb);
                self.builder
                    .build_store(delete_result_slot, self.context.bool_type().const_zero())
                    .unwrap();
                self.builder
                    .build_unconditional_branch(delete_merge_bb)
                    .unwrap();

                self.builder.position_at_end(delete_merge_bb);
                let final_result = self
                    .builder
                    .build_load(
                        self.context.bool_type(),
                        delete_result_slot,
                        "file_delete_result",
                    )
                    .unwrap();
                Ok(Some(final_result))
            }

            // Time Functions
            "Time__now" => {
                let format = self.compile_string_argument_expr(
                    &args[0].node,
                    "Time.now() requires String format",
                )?;
                let time_fn = self.get_or_declare_time();
                let localtime_fn = self.get_or_declare_localtime();
                let strftime_fn = self.get_or_declare_strftime();
                let malloc = self.get_or_declare_malloc();

                // 1. Get current time
                let null = self.context.ptr_type(AddressSpace::default()).const_null();
                let t_val = self
                    .builder
                    .build_call(time_fn, &[null.into()], "t")
                    .unwrap();
                let t_raw = self.extract_call_value(t_val)?;

                // 2. Alloca for time_t (i64)
                let t_ptr = self
                    .builder
                    .build_alloca(self.context.i64_type(), "t_ptr")
                    .unwrap();
                self.builder.build_store(t_ptr, t_raw).unwrap();

                // 3. Get local time struct pointer
                let tm_ptr_val = self
                    .builder
                    .build_call(localtime_fn, &[t_ptr.into()], "tm")
                    .unwrap();
                let tm_ptr = self.extract_call_value(tm_ptr_val)?.into_pointer_value();

                // 4. Allocate a buffer sized from the format string instead of a fixed
                // 64-byte slab, which truncated longer formats and could leave invalid output.
                let strlen_fn = self.get_or_declare_strlen();
                let format_len_call = self
                    .builder
                    .build_call(strlen_fn, &[format.into()], "format_len")
                    .unwrap();
                let format_len = self.extract_call_value(format_len_call)?.into_int_value();
                let scaled_format_len = self
                    .builder
                    .build_int_mul(
                        format_len,
                        self.context.i64_type().const_int(8, false),
                        "scaled_format_len",
                    )
                    .unwrap();
                let dynamic_buf_size = self
                    .builder
                    .build_int_add(
                        scaled_format_len,
                        self.context.i64_type().const_int(64, false),
                        "dynamic_time_buf_size",
                    )
                    .unwrap();
                let min_buf_size = self.context.i64_type().const_int(64, false);
                let use_dynamic_buf = self
                    .builder
                    .build_int_compare(
                        IntPredicate::UGT,
                        dynamic_buf_size,
                        min_buf_size,
                        "use_dynamic_time_buf",
                    )
                    .unwrap();
                let buf_size = self
                    .builder
                    .build_select(
                        use_dynamic_buf,
                        dynamic_buf_size,
                        min_buf_size,
                        "time_buf_size",
                    )
                    .unwrap()
                    .into_int_value();
                let buf_ptr_val = self
                    .builder
                    .build_call(malloc, &[buf_size.into()], "buf")
                    .unwrap();
                let buf_ptr = self.extract_call_value(buf_ptr_val)?.into_pointer_value();
                let last_byte_offset = self
                    .builder
                    .build_int_sub(
                        buf_size,
                        self.context.i64_type().const_int(1, false),
                        "time_last_byte_offset",
                    )
                    .unwrap();
                let last_byte_ptr = unsafe {
                    self.builder
                        .build_gep(
                            self.context.i8_type(),
                            buf_ptr,
                            &[last_byte_offset],
                            "time_last_byte_ptr",
                        )
                        .unwrap()
                };
                self.builder
                    .build_store(buf_ptr, self.context.i8_type().const_zero())
                    .unwrap();
                self.builder
                    .build_store(last_byte_ptr, self.context.i8_type().const_zero())
                    .unwrap();

                // 5. If format is empty string, use default "%H:%M:%S"
                let is_empty = self
                    .builder
                    .build_call(strlen_fn, &[format.into()], "len")
                    .unwrap();
                let is_empty_val = self.extract_call_value(is_empty)?.into_int_value();
                let is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        is_empty_val,
                        self.context.i64_type().const_int(0, false),
                        "is_zero",
                    )
                    .unwrap();

                let default_fmt = self.context.const_string(b"%H:%M:%S", true);
                let default_fmt_global =
                    self.module
                        .add_global(default_fmt.get_type(), None, "default_time_fmt");
                default_fmt_global.set_linkage(Linkage::Private);
                default_fmt_global.set_initializer(&default_fmt);

                let actual_fmt = self
                    .builder
                    .build_select(
                        is_zero,
                        default_fmt_global.as_pointer_value(),
                        format,
                        "fmt",
                    )
                    .unwrap();

                // 6. Call strftime(buf, 64, format, tm)
                self.builder
                    .build_call(
                        strftime_fn,
                        &[
                            buf_ptr.into(),
                            buf_size.into(),
                            actual_fmt.into(),
                            tm_ptr.into(),
                        ],
                        "res",
                    )
                    .unwrap();

                Ok(Some(buf_ptr.into()))
            }

            "Time__unix" => {
                let time_fn = self.get_or_declare_time();
                let null = self.context.ptr_type(AddressSpace::default()).const_null();
                let res = self
                    .builder
                    .build_call(time_fn, &[null.into()], "time")
                    .unwrap();
                Ok(Some(self.extract_call_value(res)?))
            }

            "Time__sleep" => {
                let ms_ty = self.infer_builtin_argument_type(&args[0].node);
                if !matches!(ms_ty, Type::Integer) {
                    return Err(CodegenError::new(
                        "Time.sleep(ms) requires Integer milliseconds",
                    ));
                }
                if matches!(
                    TypeChecker::eval_numeric_const_expr(&args[0].node),
                    Some(NumericConst::Integer(value)) if value < 0
                ) {
                    return Err(CodegenError::new(
                        "Time.sleep() milliseconds must be non-negative",
                    ));
                }
                let ms = self.compile_expr_with_expected_type(&args[0].node, &ms_ty)?;
                if !ms.is_int_value() {
                    return Err(CodegenError::new(
                        "Time.sleep(ms) requires Integer milliseconds",
                    ));
                }
                let ms_i64 = self
                    .builder
                    .build_int_cast(ms.into_int_value(), self.context.i64_type(), "sleep_ms")
                    .unwrap();
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Time.sleep used outside function"))?;
                let sleep_valid_bb = self
                    .context
                    .append_basic_block(current_fn, "time_sleep_valid");
                let sleep_invalid_bb = self
                    .context
                    .append_basic_block(current_fn, "time_sleep_invalid");
                let sleep_negative = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        ms_i64,
                        self.context.i64_type().const_zero(),
                        "time_sleep_negative",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(sleep_negative, sleep_invalid_bb, sleep_valid_bb)
                    .unwrap();

                self.builder.position_at_end(sleep_invalid_bb);
                self.emit_runtime_error(
                    "Time.sleep() milliseconds must be non-negative",
                    "time_sleep_negative_runtime_error",
                )?;

                self.builder.position_at_end(sleep_valid_bb);
                #[cfg(windows)]
                {
                    let sleep_fn = self.get_or_declare_sleep_win();
                    let ms_i32 = self
                        .builder
                        .build_int_truncate(ms_i64, self.context.i32_type(), "ms32")
                        .unwrap();
                    self.builder
                        .build_call(sleep_fn, &[ms_i32.into()], "")
                        .unwrap();
                }
                #[cfg(not(windows))]
                {
                    let usleep_fn = self.get_or_declare_usleep();
                    let us = self
                        .builder
                        .build_int_mul(ms_i64, self.context.i64_type().const_int(1000, false), "us")
                        .unwrap();
                    let us_i32 = self
                        .builder
                        .build_int_truncate(us, self.context.i32_type(), "us32")
                        .unwrap();
                    self.builder
                        .build_call(usleep_fn, &[us_i32.into()], "")
                        .unwrap();
                }
                Ok(Some(self.context.i8_type().const_int(0, false).into()))
            }

            // System Functions
            "System__getenv" => {
                let name = self.compile_string_argument_expr(
                    &args[0].node,
                    "System.getenv() requires String name",
                )?;
                let getenv_fn = self.get_or_declare_getenv();
                let res = self
                    .builder
                    .build_call(getenv_fn, &[name.into()], "env")
                    .unwrap();
                let val = self.extract_call_value(res)?.into_pointer_value();

                // If NULL, return empty string
                let is_null = self.builder.build_is_null(val, "is_null").unwrap();
                let empty_str = self.get_or_create_empty_string();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("System.getenv used outside function"))?;
                let success_bb = self.context.append_basic_block(current_fn, "env.ok");
                let fail_bb = self.context.append_basic_block(current_fn, "env.fail");
                let merge_bb = self.context.append_basic_block(current_fn, "env.merge");

                self.builder
                    .build_conditional_branch(is_null, fail_bb, success_bb)
                    .unwrap();

                self.builder.position_at_end(fail_bb);
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(success_bb);
                self.compile_utf8_string_length_runtime(val)?;
                let success_merge_block = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::new("System.getenv merge predecessor missing"))?;
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(self.context.ptr_type(AddressSpace::default()), "res")
                    .unwrap();
                phi.add_incoming(&[(&empty_str, fail_bb), (&val, success_merge_block)]);
                Ok(Some(phi.as_basic_value()))
            }

            "System__shell" => {
                let cmd = self.compile_string_argument_expr(
                    &args[0].node,
                    "System.shell() requires String command",
                )?;
                let system_fn = self.get_or_declare_system();
                let res = self
                    .builder
                    .build_call(system_fn, &[cmd.into()], "exit_code")
                    .unwrap();
                let code = self.extract_call_value(res)?.into_int_value();
                #[cfg(not(windows))]
                let code = {
                    let i32_type = self.context.i32_type();
                    let current_fn = self
                        .current_function
                        .ok_or_else(|| CodegenError::new("System.shell used outside function"))?;
                    let decode_error_bb = self
                        .context
                        .append_basic_block(current_fn, "system_shell_decode_error");
                    let signal_check_bb = self
                        .context
                        .append_basic_block(current_fn, "system_shell_signal_check");
                    let signaled_bb = self
                        .context
                        .append_basic_block(current_fn, "system_shell_signaled");
                    let exited_bb = self
                        .context
                        .append_basic_block(current_fn, "system_shell_exited");
                    let merge_bb = self
                        .context
                        .append_basic_block(current_fn, "system_shell_decoded_merge");

                    let call_failed = self
                        .builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            code,
                            i32_type.const_all_ones(),
                            "system_shell_call_failed",
                        )
                        .unwrap();
                    self.builder
                        .build_conditional_branch(call_failed, decode_error_bb, signal_check_bb)
                        .unwrap();

                    self.builder.position_at_end(decode_error_bb);
                    self.builder.build_unconditional_branch(merge_bb).unwrap();

                    self.builder.position_at_end(signal_check_bb);
                    let signal_bits = self
                        .builder
                        .build_and(
                            code,
                            i32_type.const_int(0x7f, false),
                            "system_shell_signal_bits",
                        )
                        .unwrap();
                    let has_signal = self
                        .builder
                        .build_int_compare(
                            IntPredicate::NE,
                            signal_bits,
                            i32_type.const_zero(),
                            "system_shell_has_signal",
                        )
                        .unwrap();
                    self.builder
                        .build_conditional_branch(has_signal, signaled_bb, exited_bb)
                        .unwrap();

                    self.builder.position_at_end(signaled_bb);
                    let signaled_code = self
                        .builder
                        .build_int_add(
                            signal_bits,
                            i32_type.const_int(128, false),
                            "system_shell_signaled_code",
                        )
                        .unwrap();
                    self.builder.build_unconditional_branch(merge_bb).unwrap();

                    self.builder.position_at_end(exited_bb);
                    let shifted_code = self
                        .builder
                        .build_right_shift(
                            code,
                            i32_type.const_int(8, false),
                            false,
                            "system_shell_shifted_code",
                        )
                        .unwrap();
                    let exit_code = self
                        .builder
                        .build_and(
                            shifted_code,
                            i32_type.const_int(0xff, false),
                            "system_shell_exit_code",
                        )
                        .unwrap();
                    self.builder.build_unconditional_branch(merge_bb).unwrap();

                    self.builder.position_at_end(merge_bb);
                    let decoded_phi = self
                        .builder
                        .build_phi(i32_type, "system_shell_decoded")
                        .unwrap();
                    decoded_phi.add_incoming(&[
                        (&i32_type.const_all_ones(), decode_error_bb),
                        (&signaled_code, signaled_bb),
                        (&exit_code, exited_bb),
                    ]);
                    decoded_phi.as_basic_value().into_int_value()
                };
                let code64 = self
                    .builder
                    .build_int_s_extend(code, self.context.i64_type(), "code64")
                    .unwrap();
                Ok(Some(code64.into()))
            }

            "System__exec" => {
                let cmd = self.compile_string_argument_expr(
                    &args[0].node,
                    "System.exec() requires String command",
                )?;
                let popen_fn = self.get_or_declare_popen();
                let pclose_fn = self.get_or_declare_pclose();
                let fread_fn = self.get_or_declare_fread();
                let malloc = self.get_or_declare_malloc();
                let realloc = self.get_or_declare_realloc();

                let mode = self.context.const_string(b"r", true);
                let mode_global = self.module.add_global(mode.get_type(), None, "mode_pop_r");
                mode_global.set_linkage(Linkage::Private);
                mode_global.set_initializer(&mode);

                let pipe_val = self
                    .builder
                    .build_call(
                        popen_fn,
                        &[cmd.into(), mode_global.as_pointer_value().into()],
                        "pipe",
                    )
                    .unwrap();
                let pipe_ptr = self.extract_call_value(pipe_val)?.into_pointer_value();

                let is_null = self.builder.build_is_null(pipe_ptr, "is_null").unwrap();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("System.exec used outside function"))?;
                let success_bb = self.context.append_basic_block(current_fn, "exec.ok");
                let fail_bb = self.context.append_basic_block(current_fn, "exec.fail");
                let merge_bb = self.context.append_basic_block(current_fn, "exec.merge");

                self.builder
                    .build_conditional_branch(is_null, fail_bb, success_bb)
                    .unwrap();

                // Fail - return empty string
                self.builder.position_at_end(fail_bb);
                let empty_str = self.get_or_create_empty_string();
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                // Success - Read from pipe
                self.builder.position_at_end(success_bb);
                let i8_type = self.context.i8_type();
                let i64_type = self.context.i64_type();
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let chunk_size = i64_type.const_int(4096, false);
                let one = i64_type.const_int(1, false);
                let initial_capacity = i64_type.const_int(4097, false);
                let read_cond_bb = self
                    .context
                    .append_basic_block(current_fn, "exec.read.cond");
                let read_body_bb = self
                    .context
                    .append_basic_block(current_fn, "exec.read.body");
                let read_after_bb = self
                    .context
                    .append_basic_block(current_fn, "exec.read.after");
                let grow_bb = self
                    .context
                    .append_basic_block(current_fn, "exec.read.grow");
                let grow_ok_bb = self
                    .context
                    .append_basic_block(current_fn, "exec.read.grow.ok");
                let oom_bb = self.context.append_basic_block(current_fn, "exec.read.oom");
                let done_bb = self
                    .context
                    .append_basic_block(current_fn, "exec.read.done");

                let buf_slot = self
                    .builder
                    .build_alloca(ptr_type, "exec_buf_slot")
                    .unwrap();
                let capacity_slot = self
                    .builder
                    .build_alloca(i64_type, "exec_capacity_slot")
                    .unwrap();
                let total_read_slot = self
                    .builder
                    .build_alloca(i64_type, "exec_total_read_slot")
                    .unwrap();

                let buf_call = self
                    .builder
                    .build_call(malloc, &[initial_capacity.into()], "buf")
                    .unwrap();
                let buf = self.extract_call_value(buf_call)?.into_pointer_value();
                self.builder.build_store(buf_slot, buf).unwrap();
                self.builder
                    .build_store(capacity_slot, initial_capacity)
                    .unwrap();
                self.builder
                    .build_store(total_read_slot, i64_type.const_zero())
                    .unwrap();
                self.builder
                    .build_unconditional_branch(read_cond_bb)
                    .unwrap();

                self.builder.position_at_end(read_cond_bb);
                let current_capacity = self
                    .builder
                    .build_load(i64_type, capacity_slot, "exec_capacity")
                    .unwrap()
                    .into_int_value();
                let current_total = self
                    .builder
                    .build_load(i64_type, total_read_slot, "exec_total_read")
                    .unwrap()
                    .into_int_value();
                let remaining_capacity = self
                    .builder
                    .build_int_sub(
                        current_capacity,
                        self.builder
                            .build_int_add(current_total, one, "exec_total_plus_term")
                            .unwrap(),
                        "exec_remaining_capacity",
                    )
                    .unwrap();
                let needs_grow = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        remaining_capacity,
                        i64_type.const_zero(),
                        "exec_needs_grow",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(needs_grow, grow_bb, read_body_bb)
                    .unwrap();

                self.builder.position_at_end(read_body_bb);
                let current_buf = self
                    .builder
                    .build_load(ptr_type, buf_slot, "exec_buf")
                    .unwrap()
                    .into_pointer_value();
                let write_ptr = unsafe {
                    self.builder
                        .build_gep(i8_type, current_buf, &[current_total], "exec_write_ptr")
                        .unwrap()
                };
                let read_len_call = self
                    .builder
                    .build_call(
                        fread_fn,
                        &[
                            write_ptr.into(),
                            one.into(),
                            remaining_capacity.into(),
                            pipe_ptr.into(),
                        ],
                        "read_len",
                    )
                    .unwrap();
                let read_len = self.extract_call_value(read_len_call)?.into_int_value();
                let reached_eof = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        read_len,
                        i64_type.const_zero(),
                        "exec_reached_eof",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(reached_eof, done_bb, read_after_bb)
                    .unwrap();

                self.builder.position_at_end(read_after_bb);
                let next_total = self
                    .builder
                    .build_int_add(current_total, read_len, "exec_next_total")
                    .unwrap();
                self.builder
                    .build_store(total_read_slot, next_total)
                    .unwrap();
                let filled_chunk = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        read_len,
                        remaining_capacity,
                        "exec_filled_chunk",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(filled_chunk, grow_bb, read_cond_bb)
                    .unwrap();

                self.builder.position_at_end(grow_bb);
                let grow_capacity = self
                    .builder
                    .build_load(i64_type, capacity_slot, "exec_grow_capacity")
                    .unwrap()
                    .into_int_value();
                let new_capacity = self
                    .builder
                    .build_int_add(grow_capacity, chunk_size, "exec_new_capacity")
                    .unwrap();
                let grow_buf = self
                    .builder
                    .build_load(ptr_type, buf_slot, "exec_grow_buf")
                    .unwrap()
                    .into_pointer_value();
                let realloc_call = self
                    .builder
                    .build_call(
                        realloc,
                        &[grow_buf.into(), new_capacity.into()],
                        "exec_realloc",
                    )
                    .unwrap();
                let realloc_buf = self.extract_call_value(realloc_call)?.into_pointer_value();
                let realloc_failed = self
                    .builder
                    .build_is_null(realloc_buf, "exec_realloc_failed")
                    .unwrap();
                self.builder
                    .build_conditional_branch(realloc_failed, oom_bb, grow_ok_bb)
                    .unwrap();

                self.builder.position_at_end(oom_bb);
                self.emit_runtime_error("System.exec() out of memory", "exec_out_of_memory")?;

                self.builder.position_at_end(grow_ok_bb);
                self.builder.build_store(buf_slot, realloc_buf).unwrap();
                self.builder
                    .build_store(capacity_slot, new_capacity)
                    .unwrap();
                self.builder
                    .build_unconditional_branch(read_cond_bb)
                    .unwrap();

                self.builder.position_at_end(done_bb);
                let final_total = self
                    .builder
                    .build_load(i64_type, total_read_slot, "exec_final_total")
                    .unwrap()
                    .into_int_value();
                let final_buf = self
                    .builder
                    .build_load(ptr_type, buf_slot, "exec_final_buf")
                    .unwrap()
                    .into_pointer_value();
                let scan_index_slot = self
                    .builder
                    .build_alloca(i64_type, "exec_scan_index_slot")
                    .unwrap();
                self.builder
                    .build_store(scan_index_slot, i64_type.const_zero())
                    .unwrap();
                let scan_cond_bb = self
                    .context
                    .append_basic_block(current_fn, "exec.scan.cond");
                let scan_body_bb = self
                    .context
                    .append_basic_block(current_fn, "exec.scan.body");
                let scan_next_bb = self
                    .context
                    .append_basic_block(current_fn, "exec.scan.next");
                let scan_fail_bb = self
                    .context
                    .append_basic_block(current_fn, "exec.scan.fail");
                let validate_utf8_bb = self
                    .context
                    .append_basic_block(current_fn, "exec.validate_utf8");
                self.builder
                    .build_unconditional_branch(scan_cond_bb)
                    .unwrap();

                self.builder.position_at_end(scan_cond_bb);
                let scan_index = self
                    .builder
                    .build_load(i64_type, scan_index_slot, "exec_scan_index")
                    .unwrap()
                    .into_int_value();
                let scan_has_more = self
                    .builder
                    .build_int_compare(
                        IntPredicate::ULT,
                        scan_index,
                        final_total,
                        "exec_scan_has_more",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(scan_has_more, scan_body_bb, validate_utf8_bb)
                    .unwrap();

                self.builder.position_at_end(scan_body_bb);
                let scan_byte_ptr = unsafe {
                    self.builder
                        .build_gep(i8_type, final_buf, &[scan_index], "exec_scan_byte_ptr")
                        .unwrap()
                };
                let scan_byte = self
                    .builder
                    .build_load(i8_type, scan_byte_ptr, "exec_scan_byte")
                    .unwrap()
                    .into_int_value();
                let scan_is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        scan_byte,
                        i8_type.const_zero(),
                        "exec_scan_is_zero",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(scan_is_zero, scan_fail_bb, scan_next_bb)
                    .unwrap();

                self.builder.position_at_end(scan_fail_bb);
                self.emit_runtime_error(
                    "System.exec() cannot load NUL bytes",
                    "system_exec_nul_byte",
                )?;

                self.builder.position_at_end(scan_next_bb);
                let next_scan_index = self
                    .builder
                    .build_int_add(
                        scan_index,
                        i64_type.const_int(1, false),
                        "exec_next_scan_index",
                    )
                    .unwrap();
                self.builder
                    .build_store(scan_index_slot, next_scan_index)
                    .unwrap();
                self.builder
                    .build_unconditional_branch(scan_cond_bb)
                    .unwrap();

                self.builder.position_at_end(validate_utf8_bb);
                let term_ptr = unsafe {
                    self.builder
                        .build_gep(i8_type, final_buf, &[final_total], "term_ptr")
                        .unwrap()
                };
                self.builder
                    .build_store(term_ptr, i8_type.const_zero())
                    .unwrap();
                self.compile_utf8_string_length_runtime(final_buf)?;
                self.builder
                    .build_call(pclose_fn, &[pipe_ptr.into()], "")
                    .unwrap();
                let success_merge_block = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::new("System.exec merge predecessor missing"))?;
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                // Merge
                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(self.context.ptr_type(AddressSpace::default()), "res")
                    .unwrap();
                phi.add_incoming(&[(&empty_str, fail_bb), (&final_buf, success_merge_block)]);
                Ok(Some(phi.as_basic_value()))
            }

            "System__cwd" => {
                let getcwd_fn = self.get_or_declare_getcwd();
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let cwd_call = self
                    .builder
                    .build_call(
                        getcwd_fn,
                        &[
                            ptr_type.const_null().into(),
                            self.context.i64_type().const_zero().into(),
                        ],
                        "cwd",
                    )
                    .unwrap();
                let cwd_ptr = self.extract_call_value(cwd_call)?.into_pointer_value();
                let cwd_failed = self.builder.build_is_null(cwd_ptr, "cwd_failed").unwrap();
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("System.cwd used outside function"))?;
                let cwd_ok_bb = self.context.append_basic_block(current_fn, "system_cwd_ok");
                let cwd_fail_bb = self
                    .context
                    .append_basic_block(current_fn, "system_cwd_fail");
                self.builder
                    .build_conditional_branch(cwd_failed, cwd_fail_bb, cwd_ok_bb)
                    .unwrap();

                self.builder.position_at_end(cwd_fail_bb);
                self.emit_runtime_error("System.cwd() failed", "system_cwd_failed")?;

                self.builder.position_at_end(cwd_ok_bb);
                Ok(Some(cwd_ptr.into()))
            }
            "System__os" => {
                let os = if cfg!(target_os = "windows") {
                    "windows"
                } else if cfg!(target_os = "macos") {
                    "macos"
                } else if cfg!(target_os = "linux") {
                    "linux"
                } else {
                    "unknown"
                };
                let str_val = self.context.const_string(os.as_bytes(), true);
                let name = format!("str.os.{}", self.str_counter);
                self.str_counter += 1;
                let global = self.module.add_global(str_val.get_type(), None, &name);
                global.set_linkage(Linkage::Private);
                global.set_initializer(&str_val);
                Ok(Some(global.as_pointer_value().into()))
            }

            // Args Functions
            "Args__count" => {
                let argc_global = self.ensure_argc_global();
                let argc = self
                    .builder
                    .build_load(
                        self.context.i32_type(),
                        argc_global.as_pointer_value(),
                        "argc",
                    )
                    .unwrap()
                    .into_int_value();
                let argc64 = self
                    .builder
                    .build_int_s_extend(argc, self.context.i64_type(), "argc64")
                    .unwrap();
                Ok(Some(argc64.into()))
            }

            "Args__get" => {
                let index_ty = self.infer_builtin_argument_type(&args[0].node);
                if !matches!(index_ty, Type::Integer) {
                    return Err(CodegenError::new("Args.get() requires Integer index"));
                }
                if matches!(
                    TypeChecker::eval_numeric_const_expr(&args[0].node),
                    Some(NumericConst::Integer(value)) if value < 0
                ) {
                    return Err(CodegenError::new("Args.get() index cannot be negative"));
                }
                let index = self
                    .compile_expr_with_expected_type(&args[0].node, &index_ty)?
                    .into_int_value();
                let argc_global = self.ensure_argc_global();
                let argv_global = self.ensure_argv_global();
                let argc = self
                    .builder
                    .build_load(
                        self.context.i32_type(),
                        argc_global.as_pointer_value(),
                        "argc",
                    )
                    .unwrap()
                    .into_int_value();
                let argc64 = self
                    .builder
                    .build_int_s_extend(argc, self.context.i64_type(), "argc64")
                    .unwrap();
                let argv = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        argv_global.as_pointer_value(),
                        "argv",
                    )
                    .unwrap()
                    .into_pointer_value();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Args.get used outside function"))?;
                let negative_bb = self
                    .context
                    .append_basic_block(current_fn, "args_get_negative");
                let bounds_check_bb = self
                    .context
                    .append_basic_block(current_fn, "args_get_bounds_check");
                let oob_bb = self.context.append_basic_block(current_fn, "args_get_oob");
                let ok_bb = self.context.append_basic_block(current_fn, "args_get_ok");
                let non_negative = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SGE,
                        index,
                        self.context.i64_type().const_zero(),
                        "args_get_non_negative",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(non_negative, bounds_check_bb, negative_bb)
                    .unwrap();

                self.builder.position_at_end(negative_bb);
                self.emit_runtime_error(
                    "Args.get() index cannot be negative",
                    "args_get_negative_runtime_error",
                )?;

                self.builder.position_at_end(bounds_check_bb);
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, index, argc64, "args_get_in_bounds")
                    .unwrap();
                self.builder
                    .build_conditional_branch(in_bounds, ok_bb, oob_bb)
                    .unwrap();

                self.builder.position_at_end(oob_bb);
                self.emit_runtime_error(
                    "Args.get() index out of bounds",
                    "args_get_oob_runtime_error",
                )?;

                self.builder.position_at_end(ok_bb);
                // index is i64, need to truncate to i32 for GEP if needed, but ptr is 64bit
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(
                            self.context.ptr_type(AddressSpace::default()),
                            argv,
                            &[index],
                            "arg_ptr",
                        )
                        .unwrap()
                };
                let arg_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        elem_ptr,
                        "arg",
                    )
                    .unwrap();
                Ok(Some(arg_ptr))
            }

            // Assertion functions for testing
            "assert" => {
                // assert(condition: Boolean): None - panics if condition is false
                let condition_bool = self.compile_condition_expr(&args[0].node)?;

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("assert used outside function"))?;
                let panic_bb = self.context.append_basic_block(current_fn, "assert_panic");
                let ok_bb = self.context.append_basic_block(current_fn, "assert_ok");

                self.builder
                    .build_conditional_branch(condition_bool, ok_bb, panic_bb)
                    .unwrap();

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr("Assertion failed!\\n", "assert_fail")
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .unwrap();
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .unwrap();
                self.builder.build_unreachable().unwrap();

                // OK block
                self.builder.position_at_end(ok_bb);
                Ok(None) // void function
            }

            "assert_eq" => {
                // assert_eq(a: T, b: T): None - panics if a != b
                let equal = self
                    .compile_binary(BinOp::Eq, &args[0].node, &args[1].node)?
                    .into_int_value();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("assert_eq used outside function"))?;
                let panic_bb = self
                    .context
                    .append_basic_block(current_fn, "assert_eq_panic");
                let ok_bb = self.context.append_basic_block(current_fn, "assert_eq_ok");

                self.builder
                    .build_conditional_branch(equal, ok_bb, panic_bb)
                    .unwrap();

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Assertion failed: values are not equal!\\n",
                        "assert_eq_fail",
                    )
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .unwrap();
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .unwrap();
                self.builder.build_unreachable().unwrap();

                // OK block
                self.builder.position_at_end(ok_bb);
                Ok(None) // void function
            }

            "assert_ne" => {
                // assert_ne(a: T, b: T): None - panics if a == b
                let not_equal = self
                    .compile_binary(BinOp::NotEq, &args[0].node, &args[1].node)?
                    .into_int_value();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("assert_ne used outside function"))?;
                let panic_bb = self
                    .context
                    .append_basic_block(current_fn, "assert_ne_panic");
                let ok_bb = self.context.append_basic_block(current_fn, "assert_ne_ok");

                self.builder
                    .build_conditional_branch(not_equal, ok_bb, panic_bb)
                    .unwrap();

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Assertion failed: values should not be equal!\\n",
                        "assert_ne_fail",
                    )
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .unwrap();
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .unwrap();
                self.builder.build_unreachable().unwrap();

                // OK block
                self.builder.position_at_end(ok_bb);
                Ok(None) // void function
            }

            "assert_true" => {
                // assert_true(condition: Boolean): None - panics if condition is false
                let condition_bool = self.compile_condition_expr(&args[0].node)?;

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("assert_true used outside function"))?;
                let panic_bb = self
                    .context
                    .append_basic_block(current_fn, "assert_true_panic");
                let ok_bb = self
                    .context
                    .append_basic_block(current_fn, "assert_true_ok");

                self.builder
                    .build_conditional_branch(condition_bool, ok_bb, panic_bb)
                    .unwrap();

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Assertion failed: expected true!\\n",
                        "assert_true_fail",
                    )
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .unwrap();
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .unwrap();
                self.builder.build_unreachable().unwrap();

                // OK block
                self.builder.position_at_end(ok_bb);
                Ok(None) // void function
            }

            "assert_false" => {
                // assert_false(condition: Boolean): None - panics if condition is true
                let condition_bool = self.compile_condition_expr(&args[0].node)?;

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("assert_false used outside function"))?;
                let panic_bb = self
                    .context
                    .append_basic_block(current_fn, "assert_false_panic");
                let ok_bb = self
                    .context
                    .append_basic_block(current_fn, "assert_false_ok");

                self.builder
                    .build_conditional_branch(condition_bool, panic_bb, ok_bb)
                    .unwrap();

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Assertion failed: expected false!\\n",
                        "assert_false_fail",
                    )
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .unwrap();
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .unwrap();
                self.builder.build_unreachable().unwrap();

                // OK block
                self.builder.position_at_end(ok_bb);
                Ok(None) // void function (unreachable)
            }

            "fail" => {
                // fail(message: String): None - unconditionally panics
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr("Test failed: ", "fail_prefix")
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .unwrap();

                if !args.is_empty() {
                    let msg = self.compile_string_argument_expr(
                        &args[0].node,
                        "fail() requires String message",
                    )?;
                    self.builder.build_call(printf, &[msg.into()], "").unwrap();
                }

                let newline = self.builder.build_global_string_ptr("\\n", "nl").unwrap();
                self.builder
                    .build_call(printf, &[newline.as_pointer_value().into()], "")
                    .unwrap();

                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .unwrap();
                self.builder.build_unreachable().unwrap();

                Ok(Some(self.context.i64_type().const_int(0, false).into()))
            }

            // Not a stdlib function
            _ => Ok(None),
        }
    }
}
