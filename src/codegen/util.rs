//! Utility functions for codegen: C library declarations and helper methods
#![allow(dead_code)]

use crate::ast::{
    BinOp, Expr, Literal, MatchArm, Parameter, Pattern, Spanned, Stmt, StringPart, Type, UnaryOp,
};
use crate::parser::parse_type_source;
use crate::project::OutputKind;

use inkwell::basic_block::BasicBlock;
use inkwell::module::Linkage;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, PointerValue, ValueKind};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate, OptimizationLevel};

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use crate::codegen::core::{Codegen, CodegenError, Result, Variable};

static LLVM_NATIVE_TARGET_INIT: OnceLock<std::result::Result<(), String>> = OnceLock::new();
static LLVM_ALL_TARGETS_INIT: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TargetMachineCacheKey {
    triple: String,
    cpu: String,
    features: String,
    opt_level: &'static str,
    reloc_mode: &'static str,
}

thread_local! {
    // LLVM target machines are not Send, so cache them per worker thread.
    static TARGET_MACHINE_CACHE: RefCell<HashMap<TargetMachineCacheKey, TargetMachine>> =
        RefCell::new(HashMap::new());
}

impl<'ctx> Codegen<'ctx> {
    fn normalize_inferred_object_type(&self, ty: Type) -> Type {
        self.normalize_codegen_type(&ty)
    }

    fn infer_builtin_call_type(&self, callee: &Expr, args: &[Spanned<Expr>]) -> Option<Type> {
        match callee {
            Expr::Ident(name) if name == "println" => Some(Type::None),
            Expr::Ident(name) if name == "to_string" => Some(Type::String),
            Expr::Ident(name) if name == "range" => args
                .first()
                .map(|arg| Type::Range(Box::new(self.infer_expr_type(&arg.node, &[]))))
                .or_else(|| Some(Type::Range(Box::new(Type::Integer)))),
            Expr::Field { object, field } => {
                let Expr::Ident(owner_name) = &object.node else {
                    return None;
                };
                let resolved_owner = self.resolve_module_alias(owner_name);
                match (resolved_owner.as_str(), field.as_str()) {
                    ("Str", "len") | ("Str", "compare") => Some(Type::Integer),
                    ("Str", "concat") | ("Str", "upper") | ("Str", "lower") | ("Str", "trim") => {
                        Some(Type::String)
                    }
                    ("Str", "contains") | ("Str", "startsWith") | ("Str", "endsWith") => {
                        Some(Type::Boolean)
                    }
                    ("Option", "some") => args.first().map(|first_arg| {
                        Type::Option(Box::new(self.infer_expr_type(&first_arg.node, &[])))
                    }),
                    ("Option", "none") => Some(Type::Option(Box::new(Type::Integer))),
                    ("Result", "ok") => args.first().map(|first_arg| {
                        Type::Result(
                            Box::new(self.infer_expr_type(&first_arg.node, &[])),
                            Box::new(Type::String),
                        )
                    }),
                    ("Result", "error") => args.first().map(|first_arg| {
                        Type::Result(
                            Box::new(Type::Integer),
                            Box::new(self.infer_expr_type(&first_arg.node, &[])),
                        )
                    }),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub(crate) fn unwrap_class_like_type(&self, ty: &Type) -> Option<(String, Option<Vec<Type>>)> {
        match self.normalize_codegen_type(ty) {
            Type::Named(name) => Some((name, None)),
            Type::Generic(name, args) => Some((name, Some(args))),
            Type::Ref(inner)
            | Type::MutRef(inner)
            | Type::Box(inner)
            | Type::Rc(inner)
            | Type::Arc(inner) => self.unwrap_class_like_type(&inner),
            _ => None,
        }
    }

    pub(crate) fn deref_codegen_type<'a>(&self, ty: &'a Type) -> &'a Type {
        match ty {
            Type::Ref(inner) | Type::MutRef(inner) | Type::Ptr(inner) => {
                self.deref_codegen_type(inner)
            }
            _ => ty,
        }
    }

    pub(crate) fn infer_block_tail_type(&self, block: &[Spanned<Stmt>]) -> Option<Type> {
        let last = block.last()?;
        match &last.node {
            Stmt::Expr(expr) => self.infer_object_type(&expr.node),
            _ => None,
        }
    }

    fn builtin_method_return_type(&self, obj_ty: &Type, field: &str) -> Option<Type> {
        match self.deref_codegen_type(obj_ty) {
            Type::List(inner) => match field {
                "get" | "pop" => Some((**inner).clone()),
                "length" => Some(Type::Integer),
                "push" | "set" => Some(Type::None),
                _ => None,
            },
            Type::Map(_, value) => match field {
                "get" => Some((**value).clone()),
                "contains" => Some(Type::Boolean),
                "length" => Some(Type::Integer),
                "insert" | "set" => Some(Type::None),
                _ => None,
            },
            Type::Set(_) => match field {
                "add" | "contains" | "remove" => Some(Type::Boolean),
                "length" => Some(Type::Integer),
                _ => None,
            },
            Type::Option(inner) => match field {
                "unwrap" => Some((**inner).clone()),
                "is_some" | "is_none" => Some(Type::Boolean),
                _ => None,
            },
            Type::Result(ok, _) => match field {
                "unwrap" => Some((**ok).clone()),
                "is_ok" | "is_error" => Some(Type::Boolean),
                _ => None,
            },
            Type::Task(inner) => match field {
                "await_timeout" => Some(Type::Option(inner.clone())),
                "is_done" => Some(Type::Boolean),
                "cancel" => Some(Type::None),
                _ => None,
            },
            Type::Range(inner) => match field {
                "next" => Some((**inner).clone()),
                "has_next" => Some(Type::Boolean),
                _ => None,
            },
            Type::String => match field {
                "length" => Some(Type::Integer),
                _ => None,
            },
            _ => None,
        }
    }

    // === C Library Definitions ===

    pub fn get_or_declare_fopen(&self) -> FunctionValue<'ctx> {
        let name = "fopen";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        // FILE* fopen(const char* filename, const char* mode)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_fclose(&self) -> FunctionValue<'ctx> {
        let name = "fclose";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        // int fclose(FILE* stream)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i32_type().fn_type(&[ptr_type.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_fputs(&self) -> FunctionValue<'ctx> {
        let name = "fputs";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        // int fputs(const char* str, FILE* stream)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[ptr_type.into(), ptr_type.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_fseek(&self) -> FunctionValue<'ctx> {
        let name = "fseek";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        // int fseek(FILE* stream, long offset, int origin)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i32_type().fn_type(
            &[
                ptr_type.into(),
                self.context.i64_type().into(),
                self.context.i32_type().into(),
            ],
            false,
        );
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_ftell(&self) -> FunctionValue<'ctx> {
        let name = "ftell";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        // long ftell(FILE* stream)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i64_type().fn_type(&[ptr_type.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_rewind(&self) -> FunctionValue<'ctx> {
        let name = "rewind";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        // void rewind(FILE* stream)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.void_type().fn_type(&[ptr_type.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_fread(&self) -> FunctionValue<'ctx> {
        let name = "fread";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        // size_t fread(void* ptr, size_t size, size_t count, FILE* stream)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let size_t = self.context.i64_type();
        let fn_type = size_t.fn_type(
            &[
                ptr_type.into(),
                size_t.into(),
                size_t.into(),
                ptr_type.into(),
            ],
            false,
        );
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_remove(&self) -> FunctionValue<'ctx> {
        let name = "remove";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        // int remove(const char* filename)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i32_type().fn_type(&[ptr_type.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_rand(&self) -> FunctionValue<'ctx> {
        let name = "rand";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self.context.i32_type().fn_type(&[], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_toupper(&self) -> FunctionValue<'ctx> {
        let name = "toupper";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.context.i32_type().into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_tolower(&self) -> FunctionValue<'ctx> {
        let name = "tolower";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.context.i32_type().into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_isspace(&self) -> FunctionValue<'ctx> {
        let name = "isspace";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.context.i32_type().into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_strstr(&self) -> FunctionValue<'ctx> {
        let name = "strstr";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr.fn_type(&[ptr.into(), ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_strncpy(&self) -> FunctionValue<'ctx> {
        let name = "strncpy";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let size_t = self.context.i64_type();
        let fn_type = ptr.fn_type(&[ptr.into(), ptr.into(), size_t.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_create_empty_string(&self) -> inkwell::values::PointerValue<'ctx> {
        let name = "empty_string_const";
        if let Some(g) = self.module.get_global(name) {
            return g.as_pointer_value();
        }
        let val = self.context.const_string(b"", true);
        let global = self.module.add_global(val.get_type(), None, name);
        global.set_linkage(Linkage::Private);
        global.set_initializer(&val);
        global.set_constant(true);
        global.as_pointer_value()
    }

    pub fn get_or_declare_time(&self) -> FunctionValue<'ctx> {
        let name = "time";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i64_type().fn_type(&[ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_localtime(&self) -> FunctionValue<'ctx> {
        let name = "localtime";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr.fn_type(&[ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_strftime(&self) -> FunctionValue<'ctx> {
        let name = "strftime";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let size_t = self.context.i64_type();
        let fn_type = size_t.fn_type(&[ptr.into(), size_t.into(), ptr.into(), ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_sleep_win(&self) -> FunctionValue<'ctx> {
        let name = "Sleep";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.context.i32_type().into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_usleep(&self) -> FunctionValue<'ctx> {
        let name = "usleep";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[self.context.i32_type().into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_getenv(&self) -> FunctionValue<'ctx> {
        let name = "getenv";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr.fn_type(&[ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_system(&self) -> FunctionValue<'ctx> {
        let name = "system";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i32_type().fn_type(&[ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_popen(&self) -> FunctionValue<'ctx> {
        #[cfg(windows)]
        let name = "_popen";
        #[cfg(not(windows))]
        let name = "popen";

        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr.fn_type(&[ptr.into(), ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_pclose(&self) -> FunctionValue<'ctx> {
        #[cfg(windows)]
        let name = "_pclose";
        #[cfg(not(windows))]
        let name = "pclose";

        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i32_type().fn_type(&[ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_getcwd(&self) -> FunctionValue<'ctx> {
        #[cfg(windows)]
        let name = "_getcwd";
        #[cfg(not(windows))]
        let name = "getcwd";

        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr.fn_type(&[ptr.into(), self.context.i64_type().into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_math_func(&self, name: &str, single_arg: bool) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        if single_arg {
            let fn_type = self
                .context
                .f64_type()
                .fn_type(&[self.context.f64_type().into()], false);
            self.module.add_function(name, fn_type, None)
        } else {
            let fn_type = self.context.f64_type().fn_type(
                &[
                    self.context.f64_type().into(),
                    self.context.f64_type().into(),
                ],
                false,
            );
            self.module.add_function(name, fn_type, None)
        }
    }

    pub fn get_or_declare_math_func2(&mut self, name: &str) -> FunctionValue<'ctx> {
        self.get_or_declare_math_func(name, false)
    }

    pub fn get_or_declare_strlen(&self) -> FunctionValue<'ctx> {
        let name = "strlen";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self.context.i64_type().fn_type(
            &[self.context.ptr_type(AddressSpace::default()).into()],
            false,
        );
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_strcmp(&self) -> FunctionValue<'ctx> {
        let name = "strcmp";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self.context.i32_type().fn_type(
            &[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_strtoll(&self) -> FunctionValue<'ctx> {
        let name = "strtoll";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i64_type().fn_type(
            &[ptr.into(), ptr.into(), self.context.i32_type().into()],
            false,
        );
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_strncmp(&self) -> FunctionValue<'ctx> {
        let name = "strncmp";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self.context.i32_type().fn_type(
            &[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.i64_type().into(),
            ],
            false,
        );
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_memcmp(&self) -> FunctionValue<'ctx> {
        let name = "memcmp";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i32_type().fn_type(
            &[ptr.into(), ptr.into(), self.context.i64_type().into()],
            false,
        );
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_strcpy(&self) -> FunctionValue<'ctx> {
        let name = "strcpy";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self.context.ptr_type(AddressSpace::default()).fn_type(
            &[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_strcat(&self) -> FunctionValue<'ctx> {
        let name = "strcat";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self.context.ptr_type(AddressSpace::default()).fn_type(
            &[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_fgets(&self) -> FunctionValue<'ctx> {
        let name = "fgets";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self.context.ptr_type(AddressSpace::default()).fn_type(
            &[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.i32_type().into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_getchar(&self) -> FunctionValue<'ctx> {
        let name = "getchar";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self.context.i32_type().fn_type(&[], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_stdin(&self) -> PointerValue<'ctx> {
        #[cfg(windows)]
        {
            let name = "__acrt_iob_func";
            let func = if let Some(f) = self.module.get_function(name) {
                f
            } else {
                let fn_type = self
                    .context
                    .ptr_type(AddressSpace::default())
                    .fn_type(&[self.context.i32_type().into()], false);
                self.module.add_function(name, fn_type, None)
            };
            let call = self
                .builder
                .build_call(
                    func,
                    &[self.context.i32_type().const_int(0, false).into()],
                    "stdin",
                )
                .unwrap();
            return self.extract_call_value(call).into_pointer_value();
        }

        #[cfg(not(windows))]
        {
            let ptr_type = self.context.ptr_type(AddressSpace::default());
            let stdin_global = if let Some(global) = self.module.get_global("stdin") {
                global
            } else {
                self.module.add_global(ptr_type, None, "stdin")
            };
            self.builder
                .build_load(ptr_type, stdin_global.as_pointer_value(), "stdin")
                .unwrap()
                .into_pointer_value()
        }
    }

    pub fn get_or_declare_exit(&self) -> FunctionValue<'ctx> {
        let name = "exit";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self
            .context
            .void_type()
            .fn_type(&[self.context.i32_type().into()], false);
        self.module.add_function(name, fn_type, None)
    }

    /// Helper to extract basic value from call result
    pub fn extract_call_value(
        &self,
        call: inkwell::values::CallSiteValue<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        match call.try_as_basic_value() {
            ValueKind::Basic(val) => val,
            _ => panic!("Expected call to return a value"),
        }
    }

    /// Helper to transform a string character by character using a C function (like toupper/tolower)
    pub fn compile_string_transform(
        &mut self,
        s: BasicValueEnum<'ctx>,
        transform_fn: FunctionValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let s_ptr = s.into_pointer_value();
        let strlen_fn = self.get_or_declare_strlen();
        let malloc_fn = self.get_or_declare_malloc();

        let len_call = self
            .builder
            .build_call(strlen_fn, &[s_ptr.into()], "len")
            .unwrap();
        let len = self.extract_call_value(len_call).into_int_value();

        let one = self.context.i64_type().const_int(1, false);
        let size = self.builder.build_int_add(len, one, "size").unwrap();
        let buf_call = self
            .builder
            .build_call(malloc_fn, &[size.into()], "buf")
            .unwrap();
        let buf = self.extract_call_value(buf_call).into_pointer_value();

        let current_fn = self.current_function.unwrap();
        let cond_bb = self.context.append_basic_block(current_fn, "trans.cond");
        let body_bb = self.context.append_basic_block(current_fn, "trans.body");
        let after_bb = self.context.append_basic_block(current_fn, "trans.after");

        let index_ptr = self
            .builder
            .build_alloca(self.context.i64_type(), "i")
            .unwrap();
        self.builder
            .build_store(index_ptr, self.context.i64_type().const_int(0, false))
            .unwrap();
        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(cond_bb);
        let i = self
            .builder
            .build_load(self.context.i64_type(), index_ptr, "i")
            .unwrap()
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i, len, "cmp")
            .unwrap();
        self.builder
            .build_conditional_branch(cond, body_bb, after_bb)
            .unwrap();

        self.builder.position_at_end(body_bb);
        let char_ptr = unsafe {
            self.builder
                .build_gep(self.context.i8_type(), s_ptr, &[i], "char_ptr")
                .unwrap()
        };
        let char_val = self
            .builder
            .build_load(self.context.i8_type(), char_ptr, "char")
            .unwrap();
        let char_i32 = self
            .builder
            .build_int_s_extend(char_val.into_int_value(), self.context.i32_type(), "c32")
            .unwrap();

        let trans_call = self
            .builder
            .build_call(transform_fn, &[char_i32.into()], "t32")
            .unwrap();
        let trans_val32 = self.extract_call_value(trans_call).into_int_value();
        let trans_val = self
            .builder
            .build_int_truncate(trans_val32, self.context.i8_type(), "t8")
            .unwrap();

        let dest_ptr = unsafe {
            self.builder
                .build_gep(self.context.i8_type(), buf, &[i], "dest_ptr")
                .unwrap()
        };
        self.builder.build_store(dest_ptr, trans_val).unwrap();

        let next_i = self.builder.build_int_add(i, one, "next_i").unwrap();
        self.builder.build_store(index_ptr, next_i).unwrap();
        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(after_bb);
        let term_ptr = unsafe {
            self.builder
                .build_gep(self.context.i8_type(), buf, &[len], "term_ptr")
                .unwrap()
        };
        self.builder
            .build_store(term_ptr, self.context.i8_type().const_int(0, false))
            .unwrap();

        Ok(buf.into())
    }

    // === Borrow/Deref ===

    pub fn compile_borrow(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>> {
        if let Ok(ptr) = self.compile_lvalue(expr) {
            return Ok(ptr.into());
        }

        // Materialize non-lvalue values into a temporary slot so first-class
        // functions and other expression results can still be borrowed.
        let value = self.compile_expr(expr)?;
        let value_ty = self.infer_expr_type(expr, &[]);
        let alloca = self
            .builder
            .build_alloca(self.llvm_type(&value_ty), "borrow_tmp")
            .unwrap();
        self.builder.build_store(alloca, value).unwrap();
        Ok(alloca.into())
    }

    pub fn compile_deref(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>> {
        // Compile the expression to get a pointer value
        let ptr_val = self.compile_expr(expr)?.into_pointer_value();

        let pointee_ty = match self.infer_expr_type(expr, &[]) {
            Type::Ref(inner) | Type::MutRef(inner) | Type::Ptr(inner) => self.llvm_type(&inner),
            _ => self.context.i64_type().into(),
        };
        let val = self
            .builder
            .build_load(pointee_ty, ptr_val, "deref")
            .unwrap();
        Ok(val)
    }

    // === Lambda functions ===

    pub fn compile_lambda(
        &mut self,
        params: &[Parameter],
        body: &Spanned<Expr>,
        expected_fn_ty: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>> {
        // 1. Identify captures
        let captures = self.identify_captures(&body.node, params);

        // 2. Infer return type
        let ret_apex_ty = match expected_fn_ty {
            Some(Type::Function(_, expected_ret)) => (**expected_ret).clone(),
            _ => self.infer_expr_type(&body.node, params),
        };
        let ret_llvm_ty = self.llvm_type(&ret_apex_ty);

        // 3. Create environment struct in outer scope
        let mut env_types = Vec::new();
        for (_, ty) in &captures {
            env_types.push(self.llvm_type(ty));
        }
        let env_struct_ty = self.context.struct_type(&env_types, false);

        let malloc = self.get_or_declare_malloc();
        let size = env_struct_ty.size_of().unwrap();
        let env_ptr_raw = match self
            .builder
            .build_call(malloc, &[size.into()], "env_ptr")
            .unwrap()
            .try_as_basic_value()
        {
            ValueKind::Basic(val) => val.into_pointer_value(),
            _ => panic!("malloc should return a value"),
        };

        // Fill environment
        for (i, (name, ty)) in captures.iter().enumerate() {
            let var = self.variables.get(name).unwrap();
            let val = self
                .builder
                .build_load(self.llvm_type(ty), var.ptr, name)
                .unwrap();
            let field_ptr = unsafe {
                self.builder
                    .build_gep(
                        env_struct_ty,
                        env_ptr_raw,
                        &[
                            self.context.i32_type().const_int(0, false),
                            self.context.i32_type().const_int(i as u64, false),
                        ],
                        "capture",
                    )
                    .unwrap()
            };
            self.builder.build_store(field_ptr, val).unwrap();
        }

        // Save current function context
        let saved_function = self.current_function;
        let saved_return_type = self.current_return_type.clone();
        let saved_insert_block = self.builder.get_insert_block();
        let saved_variables = std::mem::take(&mut self.variables);

        // Create unique name for lambda
        let lambda_name = format!("__lambda_{}", self.lambda_counter);
        self.lambda_counter += 1;

        // Build parameter types (including env_ptr as first arg)
        let mut llvm_params: Vec<BasicMetadataTypeEnum> = vec![
            self.context.ptr_type(AddressSpace::default()).into(), // env_ptr
        ];
        for p in params {
            llvm_params.push(self.llvm_type(&p.ty).into());
        }

        // Create function with inferred return type
        let fn_type = match ret_llvm_ty {
            BasicTypeEnum::IntType(i) => i.fn_type(&llvm_params, false),
            BasicTypeEnum::FloatType(f) => f.fn_type(&llvm_params, false),
            BasicTypeEnum::PointerType(p) => p.fn_type(&llvm_params, false),
            BasicTypeEnum::StructType(s) => s.fn_type(&llvm_params, false),
            _ => self.context.i8_type().fn_type(&llvm_params, false),
        };
        let lambda_fn = self.module.add_function(&lambda_name, fn_type, None);

        // Set up function body
        self.current_function = Some(lambda_fn);
        self.current_return_type = Some(ret_apex_ty.clone());

        let entry = self.context.append_basic_block(lambda_fn, "entry");
        self.builder.position_at_end(entry);

        // Populate local variables from env_ptr
        let env_ptr_arg = lambda_fn.get_nth_param(0).unwrap().into_pointer_value();
        for (i, (name, ty)) in captures.iter().enumerate() {
            let field_ptr = unsafe {
                self.builder
                    .build_gep(
                        env_struct_ty,
                        env_ptr_arg,
                        &[
                            self.context.i32_type().const_int(0, false),
                            self.context.i32_type().const_int(i as u64, false),
                        ],
                        "load_capture",
                    )
                    .unwrap()
            };
            let alloca = self.builder.build_alloca(self.llvm_type(ty), name).unwrap();
            let val = self
                .builder
                .build_load(self.llvm_type(ty), field_ptr, "cap_val")
                .unwrap();
            self.builder.build_store(alloca, val).unwrap();
            self.variables.insert(
                name.clone(),
                Variable {
                    ptr: alloca,
                    ty: ty.clone(),
                },
            );
        }

        // Allocate parameters (starting from index 1)
        for (i, param) in params.iter().enumerate() {
            let llvm_param = lambda_fn.get_nth_param((i + 1) as u32).unwrap();
            let alloca = self
                .builder
                .build_alloca(self.llvm_type(&param.ty), &param.name)
                .unwrap();
            self.builder.build_store(alloca, llvm_param).unwrap();
            self.variables.insert(
                param.name.clone(),
                Variable {
                    ptr: alloca,
                    ty: param.ty.clone(),
                },
            );
        }

        // Compile body expression
        let result = self.compile_expr_with_expected_type(&body.node, &ret_apex_ty)?;

        // Build return with proper casting if needed
        let final_result = if result.get_type() != ret_llvm_ty {
            if result.is_int_value() && ret_llvm_ty.is_int_type() {
                let res_int = result.into_int_value();
                let ret_int = ret_llvm_ty.into_int_type();
                if res_int.get_type().get_bit_width() < ret_int.get_bit_width() {
                    self.builder
                        .build_int_z_extend(res_int, ret_int, "ret_cast")
                        .unwrap()
                        .into()
                } else {
                    self.builder
                        .build_int_truncate(res_int, ret_int, "ret_cast")
                        .unwrap()
                        .into()
                }
            } else {
                result
            }
        } else {
            result
        };

        self.builder.build_return(Some(&final_result)).unwrap();

        // Restore context
        self.current_function = saved_function;
        self.current_return_type = saved_return_type;
        self.variables = saved_variables;

        // Position builder back to the exact insertion point used before entering lambda codegen.
        if let Some(block) = saved_insert_block {
            self.builder.position_at_end(block);
        } else if let Some(func) = saved_function {
            if let Some(block) = func.get_last_basic_block() {
                self.builder.position_at_end(block);
            }
        }

        // Return closure struct { fn_ptr, env_ptr }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let closure_ty = self
            .context
            .struct_type(&[ptr_type.into(), ptr_type.into()], false);

        let mut closure = closure_ty.get_undef();
        closure = self
            .builder
            .build_insert_value(
                closure,
                lambda_fn.as_global_value().as_pointer_value(),
                0,
                "fn",
            )
            .unwrap()
            .into_struct_value();
        closure = self
            .builder
            .build_insert_value(closure, env_ptr_raw, 1, "env")
            .unwrap()
            .into_struct_value();

        Ok(closure.into())
    }

    pub fn compile_match_expr(
        &mut self,
        expr: &Expr,
        arms: &[MatchArm],
        expected_result_ty: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let imported_variant = |this: &Self, name: &str| -> Option<(String, String, bool)> {
            let (enum_name, variant_name) = this.resolve_import_alias_variant(name)?;
            let variant_info = this.enums.get(&enum_name)?.variants.get(&variant_name)?;
            Some((enum_name, variant_name, variant_info.fields.is_empty()))
        };
        let imported_unit_variant = |this: &Self, name: &str| -> Option<(String, String, u8)> {
            let (enum_name, variant_name) = this.resolve_import_alias_variant(name)?;
            let variant_info = this.enums.get(&enum_name)?.variants.get(&variant_name)?;
            variant_info
                .fields
                .is_empty()
                .then_some((enum_name, variant_name, variant_info.tag))
        };
        fn pattern_variant_leaf(name: &str) -> &str {
            name.rsplit('.').next().unwrap_or(name)
        }

        let val = self.compile_expr(expr)?;
        let func = self.current_function.unwrap();
        let merge_bb = self.context.append_basic_block(func, "match.expr.merge");

        let match_ty = self.infer_expr_type(expr, &[]);
        let option_inner_ty = match &match_ty {
            Type::Option(inner) => Some((**inner).clone()),
            _ => None,
        };
        let result_inner_tys = match &match_ty {
            Type::Result(ok, err) => Some(((**ok).clone(), (**err).clone())),
            _ => None,
        };
        let enum_match_name = match &match_ty {
            Type::Named(name) if self.enums.contains_key(name) => Some(name.clone()),
            _ => None,
        };

        let mut dispatch_bb = self.builder.get_insert_block().unwrap();
        let mut incoming: Vec<(BasicValueEnum<'ctx>, BasicBlock<'ctx>)> = Vec::new();
        let mut result_ty: Option<BasicTypeEnum<'ctx>> = None;
        let inferred_match_result_ty = self.infer_match_expr_result_type(arms, &[]);
        let expected_match_result_ty = expected_result_ty.or(match inferred_match_result_ty {
            Type::None => None,
            ref ty => Some(ty),
        });

        for arm in arms {
            let arm_bb = self.context.append_basic_block(func, "match.expr.arm");
            let next_bb = self.context.append_basic_block(func, "match.expr.next");

            self.builder.position_at_end(dispatch_bb);
            match &arm.pattern {
                Pattern::Wildcard => {
                    self.builder.build_unconditional_branch(arm_bb).unwrap();
                }
                Pattern::Ident(name) => {
                    if let Some((enum_name, variant_name, variant_tag)) =
                        imported_unit_variant(self, name)
                    {
                        let is_builtin_variant =
                            matches!(variant_name.as_str(), "Some" | "None" | "Ok" | "Error");
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
                                    "match_expr_ident_variant_eq",
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
                    let pattern_val = self.compile_literal(lit)?;
                    let cond = if val.is_float_value() || pattern_val.is_float_value() {
                        let match_val = if val.is_float_value() {
                            val.into_float_value()
                        } else {
                            self.builder
                                .build_signed_int_to_float(
                                    val.into_int_value(),
                                    self.context.f64_type(),
                                    "match_expr_lit_lf",
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
                                    "match_expr_lit_rf",
                                )
                                .unwrap()
                        };
                        self.builder
                            .build_float_compare(
                                FloatPredicate::OEQ,
                                match_val,
                                pattern_float,
                                "match_expr_float_eq",
                            )
                            .unwrap()
                    } else if val.is_int_value() && pattern_val.is_int_value() {
                        self.builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                val.into_int_value(),
                                pattern_val.into_int_value(),
                                "match_expr_lit_eq",
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
                                "match_expr_strcmp",
                            )
                            .unwrap();
                        let cmp_val = self.extract_call_value(cmp).into_int_value();
                        self.builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                cmp_val,
                                self.context.i32_type().const_int(0, false),
                                "match_expr_str_eq",
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
                    if matches!(variant_leaf, "Some" | "None" | "Ok" | "Error") {
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
                                "match_expr_variant_eq",
                            )
                            .unwrap();
                        self.builder
                            .build_conditional_branch(cond, arm_bb, next_bb)
                            .unwrap();
                    } else if let Some(enum_name) = resolved_enum_name.or(enum_match_name.as_ref())
                    {
                        if let Some(enum_info) = self.enums.get(enum_name) {
                            if let Some(variant_info) = enum_info.variants.get(variant_leaf) {
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
                                        "match_expr_enum_variant_eq",
                                    )
                                    .unwrap();
                                self.builder
                                    .build_conditional_branch(cond, arm_bb, next_bb)
                                    .unwrap();
                            } else {
                                self.builder.build_unconditional_branch(next_bb).unwrap();
                            }
                        } else {
                            self.builder.build_unconditional_branch(next_bb).unwrap();
                        }
                    } else {
                        self.builder.build_unconditional_branch(next_bb).unwrap();
                    }
                }
            }

            self.builder.position_at_end(arm_bb);
            match &arm.pattern {
                Pattern::Ident(binding) => {
                    if imported_unit_variant(self, binding).is_none() {
                        let alloca = self.builder.build_alloca(val.get_type(), binding).unwrap();
                        self.builder.build_store(alloca, val).unwrap();
                        self.variables.insert(
                            binding.clone(),
                            Variable {
                                ptr: alloca,
                                ty: match_ty.clone(),
                            },
                        );
                    }
                }
                Pattern::Variant(variant_name, bindings) => {
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
                    if variant_leaf == "Some" && !bindings.is_empty() {
                        let inner = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 1, "some_inner")
                            .unwrap();
                        let alloca = self
                            .builder
                            .build_alloca(inner.get_type(), &bindings[0])
                            .unwrap();
                        self.builder.build_store(alloca, inner).unwrap();
                        self.variables.insert(
                            bindings[0].clone(),
                            Variable {
                                ptr: alloca,
                                ty: option_inner_ty.clone().unwrap_or(Type::Integer),
                            },
                        );
                    } else if variant_leaf == "Ok" && !bindings.is_empty() {
                        let inner = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 1, "ok_inner")
                            .unwrap();
                        let alloca = self
                            .builder
                            .build_alloca(inner.get_type(), &bindings[0])
                            .unwrap();
                        self.builder.build_store(alloca, inner).unwrap();
                        self.variables.insert(
                            bindings[0].clone(),
                            Variable {
                                ptr: alloca,
                                ty: result_inner_tys
                                    .as_ref()
                                    .map(|(ok, _)| ok.clone())
                                    .unwrap_or(Type::Integer),
                            },
                        );
                    } else if variant_leaf == "Error" && !bindings.is_empty() {
                        let inner = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 2, "err_inner")
                            .unwrap();
                        let alloca = self
                            .builder
                            .build_alloca(inner.get_type(), &bindings[0])
                            .unwrap();
                        self.builder.build_store(alloca, inner).unwrap();
                        self.variables.insert(
                            bindings[0].clone(),
                            Variable {
                                ptr: alloca,
                                ty: result_inner_tys
                                    .as_ref()
                                    .map(|(_, err)| err.clone())
                                    .unwrap_or(Type::String),
                            },
                        );
                    } else if let Some(enum_name) = resolved_enum_name.or(enum_match_name.as_ref())
                    {
                        if let Some(enum_info) = self.enums.get(enum_name) {
                            if let Some(variant_info) = enum_info.variants.get(variant_leaf) {
                                for (idx, binding) in bindings.iter().enumerate() {
                                    if let Some(field_ty) = variant_info.fields.get(idx) {
                                        let raw = self
                                            .builder
                                            .build_extract_value(
                                                val.into_struct_value(),
                                                (idx + 1) as u32,
                                                "enum_payload_raw",
                                            )
                                            .unwrap()
                                            .into_int_value();
                                        let decoded = self.decode_enum_payload(raw, field_ty)?;
                                        let alloca = self
                                            .builder
                                            .build_alloca(decoded.get_type(), binding)
                                            .unwrap();
                                        self.builder.build_store(alloca, decoded).unwrap();
                                        self.variables.insert(
                                            binding.clone(),
                                            Variable {
                                                ptr: alloca,
                                                ty: field_ty.clone(),
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

            let mut arm_result = self.context.i8_type().const_int(0, false).into();
            for (idx, stmt) in arm.body.iter().enumerate() {
                if idx + 1 == arm.body.len() {
                    if let Stmt::Expr(e) = &stmt.node {
                        arm_result = if let Some(expected_ty) = &expected_match_result_ty {
                            self.compile_expr_with_expected_type(&e.node, expected_ty)?
                        } else {
                            self.compile_expr(&e.node)?
                        };
                    } else {
                        self.compile_stmt(&stmt.node)?;
                    }
                } else {
                    self.compile_stmt(&stmt.node)?;
                }
            }

            if result_ty.is_none() {
                result_ty = Some(arm_result.get_type());
            }

            if self.needs_terminator() {
                self.builder.build_unconditional_branch(merge_bb).unwrap();
                let pred = self.builder.get_insert_block().unwrap();
                incoming.push((arm_result, pred));
            }

            dispatch_bb = next_bb;
            self.builder.position_at_end(dispatch_bb);
        }

        if let Some(ty) = result_ty {
            let fallback = ty.const_zero();
            if self.needs_terminator() {
                self.builder.build_unconditional_branch(merge_bb).unwrap();
                let pred = self.builder.get_insert_block().unwrap();
                incoming.push((fallback, pred));
            }

            self.builder.position_at_end(merge_bb);
            let phi = self.builder.build_phi(ty, "match_expr.result").unwrap();
            let incoming_refs: Vec<(&dyn BasicValue<'ctx>, BasicBlock<'ctx>)> = incoming
                .iter()
                .map(|(value, bb)| (value as &dyn BasicValue<'ctx>, *bb))
                .collect();
            phi.add_incoming(&incoming_refs);
            Ok(phi.as_basic_value())
        } else {
            self.builder.position_at_end(merge_bb);
            Ok(self.context.i8_type().const_int(0, false).into())
        }
    }

    pub fn compile_lvalue(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>> {
        match expr {
            Expr::Ident(name) => self
                .variables
                .get(name)
                .map(|v| v.ptr)
                .ok_or_else(|| CodegenError::new(format!("Unknown variable: {}", name))),
            Expr::Field { object, field } => {
                let object_ty = self.infer_object_type(&object.node);
                let obj_ptr = if matches!(object_ty, Some(Type::Ref(_)) | Some(Type::MutRef(_))) {
                    self.compile_deref(&object.node)?.into_pointer_value()
                } else {
                    self.compile_expr(&object.node)?.into_pointer_value()
                };

                let class_name = self
                    .infer_object_type(&object.node)
                    .and_then(|ty| self.type_to_class_name(&ty))
                    .ok_or_else(|| CodegenError::new("Cannot determine object type"))?;

                let class_info = self
                    .classes
                    .get(&class_name)
                    .ok_or_else(|| CodegenError::new(format!("Unknown class: {}", class_name)))?;

                let field_idx = *class_info
                    .field_indices
                    .get(field)
                    .ok_or_else(|| CodegenError::new(format!("Unknown field: {}", field)))?;

                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let idx = i32_type.const_int(field_idx as u64, false);

                unsafe {
                    Ok(self
                        .builder
                        .build_gep(
                            class_info.struct_type.as_basic_type_enum(),
                            obj_ptr,
                            &[zero, idx],
                            field,
                        )
                        .unwrap())
                }
            }
            Expr::Index { object, index } => {
                let idx_val = self.compile_expr(&index.node)?.into_int_value();
                let object_ty = self.infer_object_type(&object.node);
                let deref_object_ty = object_ty
                    .clone()
                    .map(|ty| self.deref_codegen_type(&ty).clone());

                // Prefer typed list element pointer for List<T> index assignment.
                if let Some(Type::List(inner)) = deref_object_ty {
                    let elem_ty = self.llvm_type(&inner);
                    let list_type = self.context.struct_type(
                        &[
                            self.context.i64_type().into(),
                            self.context.i64_type().into(),
                            self.context.ptr_type(AddressSpace::default()).into(),
                        ],
                        false,
                    );
                    let obj_val = if matches!(object_ty, Some(Type::Ref(_)) | Some(Type::MutRef(_)))
                    {
                        self.compile_deref(&object.node)?
                    } else {
                        self.compile_expr(&object.node)?
                    };
                    let (length, data_ptr) = if let BasicValueEnum::StructValue(list_struct) =
                        obj_val
                    {
                        let length = self
                            .builder
                            .build_extract_value(list_struct, 1, "list_len")
                            .map_err(|_| {
                                CodegenError::new("Invalid list value for index assignment")
                            })?
                            .into_int_value();
                        let data_ptr = self
                            .builder
                            .build_extract_value(list_struct, 2, "list_data")
                            .map_err(|_| {
                                CodegenError::new("Invalid list value for index assignment")
                            })?
                            .into_pointer_value();
                        (length, data_ptr)
                    } else {
                        let list_ptr = obj_val.into_pointer_value();
                        let i32_type = self.context.i32_type();
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
                        let length = self
                            .builder
                            .build_load(self.context.i64_type(), len_ptr, "list_len")
                            .unwrap()
                            .into_int_value();
                        let data_ptr = self
                            .builder
                            .build_load(
                                self.context.ptr_type(AddressSpace::default()),
                                data_ptr_ptr,
                                "list_data",
                            )
                            .unwrap()
                            .into_pointer_value();
                        (length, data_ptr)
                    };
                    let non_negative = self
                        .builder
                        .build_int_compare(
                            IntPredicate::SGE,
                            idx_val,
                            self.context.i64_type().const_zero(),
                            "list_assign_non_negative",
                        )
                        .unwrap();
                    let in_bounds = self
                        .builder
                        .build_int_compare(
                            IntPredicate::SLT,
                            idx_val,
                            length,
                            "list_assign_in_bounds",
                        )
                        .unwrap();
                    let valid = self
                        .builder
                        .build_and(non_negative, in_bounds, "list_assign_valid")
                        .unwrap();
                    let current_fn = self.current_function.unwrap();
                    let ok_bb = self
                        .context
                        .append_basic_block(current_fn, "list_assign_ok");
                    let fail_bb = self
                        .context
                        .append_basic_block(current_fn, "list_assign_fail");
                    self.builder
                        .build_conditional_branch(valid, ok_bb, fail_bb)
                        .unwrap();

                    self.builder.position_at_end(fail_bb);
                    self.emit_runtime_error(
                        "List assignment index out of bounds",
                        "list_assign_index_oob",
                    )?;

                    self.builder.position_at_end(ok_bb);
                    let typed_data_ptr = self
                        .builder
                        .build_pointer_cast(
                            data_ptr,
                            self.context.ptr_type(AddressSpace::default()),
                            "list_data_typed",
                        )
                        .unwrap();
                    let elem_ptr = unsafe {
                        self.builder
                            .build_gep(elem_ty, typed_data_ptr, &[idx_val], "idx_elem_ptr")
                            .unwrap()
                    };
                    return Ok(elem_ptr);
                }

                let obj_ptr = self.compile_expr(&object.node)?.into_pointer_value();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i64_type(), obj_ptr, &[idx_val], "idx_elem_ptr")
                        .unwrap()
                };
                Ok(elem_ptr)
            }
            Expr::Deref(inner) => Ok(self.compile_expr(&inner.node)?.into_pointer_value()),
            _ => Err(CodegenError::new("Invalid lvalue")),
        }
    }

    // === Helpers ===

    /// Infer the Apex Type of an expression
    pub fn infer_object_type(&self, expr: &Expr) -> Option<Type> {
        let inferred = match expr {
            Expr::Ident(name) => self
                .variables
                .get(name)
                .map(|v| v.ty.clone())
                .or_else(|| self.functions.get(name).map(|(_, ty)| ty.clone()))
                .or_else(|| {
                    let resolved_name = self.resolve_function_alias(name);
                    if resolved_name == *name {
                        None
                    } else {
                        self.functions.get(&resolved_name).map(|(_, ty)| ty.clone())
                    }
                }),
            Expr::This => self.variables.get("this").map(|v| v.ty.clone()),
            Expr::Literal(Literal::Integer(_)) => Some(Type::Integer),
            Expr::Literal(Literal::Float(_)) => Some(Type::Float),
            Expr::Literal(Literal::Boolean(_)) => Some(Type::Boolean),
            Expr::Literal(Literal::String(_)) => Some(Type::String),
            Expr::Literal(Literal::Char(_)) => Some(Type::Char),
            Expr::Literal(Literal::None) => Some(Type::None),
            Expr::StringInterp(_) => Some(Type::String),
            Expr::Construct { ty, .. } => parse_type_source(ty).ok(),
            Expr::Unary { op, expr } => match op {
                UnaryOp::Neg => {
                    let inner_ty = self.infer_object_type(&expr.node)?;
                    match inner_ty {
                        Type::Integer | Type::Float => Some(inner_ty),
                        _ => None,
                    }
                }
                UnaryOp::Not => Some(Type::Boolean),
            },
            Expr::Borrow(expr) => {
                let inner_ty = self.infer_object_type(&expr.node)?;
                Some(Type::Ref(Box::new(inner_ty)))
            }
            Expr::MutBorrow(expr) => {
                let inner_ty = self.infer_object_type(&expr.node)?;
                Some(Type::MutRef(Box::new(inner_ty)))
            }
            Expr::Binary { op, left, right } => {
                let left_ty = self.infer_object_type(&left.node)?;
                let right_ty = self.infer_object_type(&right.node)?;
                match op {
                    BinOp::Eq
                    | BinOp::NotEq
                    | BinOp::Lt
                    | BinOp::LtEq
                    | BinOp::Gt
                    | BinOp::GtEq
                    | BinOp::And
                    | BinOp::Or => Some(Type::Boolean),
                    BinOp::Add => {
                        if matches!(left_ty, Type::String) && matches!(right_ty, Type::String) {
                            Some(Type::String)
                        } else if left_ty == right_ty
                            && matches!(left_ty, Type::Integer | Type::Float)
                        {
                            Some(left_ty)
                        } else {
                            None
                        }
                    }
                    BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        if left_ty == right_ty && matches!(left_ty, Type::Integer | Type::Float) {
                            Some(left_ty)
                        } else {
                            None
                        }
                    }
                    BinOp::Mod => {
                        if matches!(left_ty, Type::Integer) && matches!(right_ty, Type::Integer) {
                            Some(Type::Integer)
                        } else {
                            None
                        }
                    }
                }
            }
            Expr::Call { callee, args, .. } => match &callee.node {
                Expr::Ident(name) if matches!(name.as_str(), "println" | "to_string" | "range") => {
                    self.infer_builtin_call_type(&callee.node, args)
                }
                Expr::Field { object, field } => {
                    if let Some(function_name) =
                        self.resolve_contextual_function_value_name(&callee.node)
                    {
                        if let Some((_, Type::Function(_, ret))) =
                            self.functions.get(&function_name)
                        {
                            return Some((**ret).clone());
                        }
                    }
                    if let Some(ret_ty) = self.infer_builtin_call_type(&callee.node, args) {
                        return Some(ret_ty);
                    }
                    let obj_ty = self.infer_object_type(&object.node)?;
                    if let Some(ret_ty) = self.builtin_method_return_type(&obj_ty, field) {
                        Some(ret_ty)
                    } else {
                        let class_name = self.type_to_class_name(&obj_ty)?;
                        let method_name = self.resolve_method_function_name(&class_name, field)?;
                        let (_, ty) = self.functions.get(&method_name)?;
                        match ty {
                            Type::Function(_, ret) => Some((**ret).clone()),
                            _ => None,
                        }
                    }
                }
                Expr::Ident(name) => {
                    let callee_ty = self
                        .variables
                        .get(name)
                        .map(|v| v.ty.clone())
                        .or_else(|| self.functions.get(name).map(|(_, ty)| ty.clone()))
                        .or_else(|| {
                            let resolved_name = self.resolve_function_alias(name);
                            if resolved_name == *name {
                                None
                            } else {
                                self.functions.get(&resolved_name).map(|(_, ty)| ty.clone())
                            }
                        })?;
                    match callee_ty {
                        Type::Function(_, ret) => Some((*ret).clone()),
                        _ => None,
                    }
                }
                _ => {
                    let callee_ty = self.infer_object_type(&callee.node)?;
                    match callee_ty {
                        Type::Function(_, ret) => Some((*ret).clone()),
                        _ => None,
                    }
                }
            },
            Expr::Try(inner) => match self.infer_object_type(&inner.node)? {
                Type::Result(ok, _) => Some((*ok).clone()),
                Type::Option(inner) => Some((*inner).clone()),
                _ => None,
            },
            Expr::Await(inner) => match self.infer_object_type(&inner.node)? {
                Type::Task(inner) => Some((*inner).clone()),
                _ => None,
            },
            Expr::Deref(inner) => match self.infer_object_type(&inner.node)? {
                Type::Ref(inner) | Type::MutRef(inner) | Type::Ptr(inner) => Some((*inner).clone()),
                _ => None,
            },
            Expr::IfExpr {
                then_branch,
                else_branch,
                ..
            } => {
                let then_ty = self.infer_block_tail_type(then_branch)?;
                let else_ty = else_branch
                    .as_ref()
                    .and_then(|block| self.infer_block_tail_type(block))?;
                if then_ty == else_ty {
                    Some(then_ty)
                } else {
                    None
                }
            }
            Expr::Match { arms, .. } => {
                let mut arm_types = arms
                    .iter()
                    .filter_map(|arm| self.infer_block_tail_type(&arm.body));
                let first = arm_types.next()?;
                if arm_types.all(|ty| ty == first) {
                    Some(first)
                } else {
                    None
                }
            }
            Expr::Block(block) | Expr::AsyncBlock(block) => self.infer_block_tail_type(block),
            Expr::Index { object, .. } => {
                match self.deref_codegen_type(&self.infer_object_type(&object.node)?) {
                    Type::List(inner) => Some((**inner).clone()),
                    Type::Map(_, value) => Some((**value).clone()),
                    Type::String => Some(Type::Char),
                    _ => None,
                }
            }
            Expr::Lambda { params, body } => Some(Type::Function(
                params.iter().map(|param| param.ty.clone()).collect(),
                Box::new(self.infer_expr_type(&body.node, params)),
            )),
            Expr::Field { object, field } => {
                if let Expr::Ident(owner_name) = &object.node {
                    let resolved_owner = self.resolve_module_alias(owner_name);
                    if self
                        .enums
                        .get(&resolved_owner)
                        .and_then(|info| info.variants.get(field))
                        .is_some()
                    {
                        return Some(Type::Named(resolved_owner));
                    }
                }
                let obj_ty = self.infer_object_type(&object.node)?;
                let (class_name, generic_args) = self.unwrap_class_like_type(&obj_ty)?;
                let class_info = self.classes.get(&class_name)?;
                let field_ty = class_info.field_types.get(field)?.clone();
                if let Some(args) = generic_args.as_ref() {
                    if class_info.generic_params.len() == args.len() {
                        let bindings = class_info
                            .generic_params
                            .iter()
                            .cloned()
                            .zip(args.iter().cloned())
                            .collect::<HashMap<_, _>>();
                        return Some(Self::substitute_type(&field_ty, &bindings));
                    }
                }
                Some(field_ty)
            }
            Expr::Require { .. } => Some(Type::None),
            Expr::Range { .. } => Some(Type::Range(Box::new(Type::Integer))),
        }?;
        Some(self.normalize_inferred_object_type(inferred))
    }

    /// Extract class name from a Type (handles Named, Ref, MutRef, etc.)
    #[allow(clippy::only_used_in_recursion)]
    pub fn type_to_class_name(&self, ty: &Type) -> Option<String> {
        let normalized = self.normalize_codegen_type(ty);
        match &normalized {
            Type::Named(name) => Some(name.clone()),
            Type::Generic(name, _) => Some(name.clone()),
            Type::Ref(inner)
            | Type::MutRef(inner)
            | Type::Box(inner)
            | Type::Rc(inner)
            | Type::Arc(inner) => self.type_to_class_name(inner),
            _ => None,
        }
    }

    pub fn task_inner_type(&self, ty: &Type) -> Option<Type> {
        let normalized = self.normalize_codegen_type(ty);
        match normalized {
            Type::Task(inner) => Some(*inner),
            Type::Generic(name, args) => {
                (name == "Task" && args.len() == 1).then(|| args[0].clone())
            }
            Type::Named(name) => self.resolve_named_task_inner_type(&name),
            _ => None,
        }
    }

    fn resolve_named_task_inner_type(&self, name: &str) -> Option<Type> {
        let (base_name, _) = name.split_once("__spec__")?;
        if base_name != "Task" {
            return None;
        }

        self.classes
            .get(name)
            .and_then(|class_info| class_info.field_types.get("result"))
            .cloned()
    }

    pub fn common_compatible_codegen_type(&self, left: &Type, right: &Type) -> Option<Type> {
        if left == right {
            return Some(left.clone());
        }
        if left.is_numeric() && right.is_numeric() {
            return Some(Type::Float);
        }
        None
    }

    fn merge_codegen_branch_type(&self, acc: Option<Type>, next: Type) -> Option<Type> {
        match acc {
            None => Some(next),
            Some(current) => self.common_compatible_codegen_type(&current, &next),
        }
    }

    pub fn infer_if_expr_result_type(
        &self,
        then_branch: &[Spanned<Stmt>],
        else_branch: Option<&Vec<Spanned<Stmt>>>,
        params: &[Parameter],
    ) -> Type {
        let then_ty = self.infer_block_tail_type_with_params(then_branch, params);
        let else_ty =
            else_branch.and_then(|branch| self.infer_block_tail_type_with_params(branch, params));
        match (then_ty, else_ty) {
            (Some(then_ty), Some(else_ty)) => self
                .common_compatible_codegen_type(&then_ty, &else_ty)
                .unwrap_or(then_ty),
            (Some(then_ty), None) => then_ty,
            _ => Type::None,
        }
    }

    pub fn infer_match_expr_result_type(&self, arms: &[MatchArm], params: &[Parameter]) -> Type {
        let mut result: Option<Type> = None;
        for arm in arms {
            let Some(arm_ty) = self.infer_block_tail_type_with_params(&arm.body, params) else {
                continue;
            };
            result = self.merge_codegen_branch_type(result, arm_ty);
        }
        result.unwrap_or(Type::None)
    }

    fn infer_block_tail_type_with_params(
        &self,
        stmts: &[Spanned<Stmt>],
        params: &[Parameter],
    ) -> Option<Type> {
        let mut ret = None;
        for stmt in stmts {
            if let Stmt::Expr(expr) = &stmt.node {
                ret = Some(self.infer_expr_type(&expr.node, params));
            }
        }
        ret
    }

    pub fn needs_terminator(&self) -> bool {
        self.builder
            .get_insert_block()
            .map(|b| b.get_terminator().is_none())
            .unwrap_or(false)
    }

    pub fn get_or_declare_printf(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("printf") {
            return f;
        }

        let printf_type = self.context.i32_type().fn_type(
            &[self.context.ptr_type(AddressSpace::default()).into()],
            true,
        );
        self.module.add_function("printf", printf_type, None)
    }

    // === Output ===

    pub fn write_ir(&self, path: &Path) -> std::result::Result<(), String> {
        self.module.print_to_file(path).map_err(|e| e.to_string())
    }

    fn resolve_optimization_level(opt_level: Option<&str>) -> OptimizationLevel {
        match opt_level
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .unwrap_or_default()
            .as_str()
        {
            "0" => OptimizationLevel::None,
            "1" => OptimizationLevel::Less,
            "2" => OptimizationLevel::Default,
            "s" | "z" | "3" | "fast" | "" => OptimizationLevel::Aggressive,
            _ => OptimizationLevel::Aggressive,
        }
    }

    fn ensure_object_emission_targets_initialized(
        target_triple: Option<&str>,
    ) -> std::result::Result<(), String> {
        if target_triple.is_some() {
            LLVM_ALL_TARGETS_INIT.get_or_init(|| {
                Target::initialize_all(&InitializationConfig::default());
            });
            return Ok(());
        }

        LLVM_NATIVE_TARGET_INIT
            .get_or_init(|| {
                Target::initialize_native(&InitializationConfig::default())
                    .map_err(|e| format!("Failed to init target: {}", e))
            })
            .clone()
    }

    fn target_machine_config(
        opt_level: Option<&str>,
        target_triple: Option<&str>,
        output_kind: &OutputKind,
    ) -> std::result::Result<(TargetMachineCacheKey, TargetTriple), String> {
        Self::ensure_object_emission_targets_initialized(target_triple)?;

        let triple = target_triple
            .map(TargetTriple::create)
            .unwrap_or_else(TargetMachine::get_default_triple);
        let triple_string = triple.as_str().to_string_lossy().into_owned();
        let host_cpu_name = TargetMachine::get_host_cpu_name();
        let host_cpu_features = TargetMachine::get_host_cpu_features();
        let cpu = if target_triple.is_some() {
            "generic".to_string()
        } else {
            host_cpu_name
                .to_str()
                .map_err(|e| format!("Failed to decode host CPU name: {}", e))?
                .to_string()
        };
        let features = if target_triple.is_some() {
            "".to_string()
        } else {
            host_cpu_features
                .to_str()
                .map_err(|e| format!("Failed to decode host CPU features: {}", e))?
                .to_string()
        };
        let opt_key = match Self::resolve_optimization_level(opt_level) {
            OptimizationLevel::None => "0",
            OptimizationLevel::Less => "1",
            OptimizationLevel::Default => "2",
            OptimizationLevel::Aggressive => "3",
        };
        let reloc_mode = match output_kind {
            OutputKind::Shared => "pic",
            OutputKind::Bin | OutputKind::Static => "default",
        };

        Ok((
            TargetMachineCacheKey {
                triple: triple_string,
                cpu,
                features,
                opt_level: opt_key,
                reloc_mode,
            },
            triple,
        ))
    }

    fn with_target_machine<R>(
        opt_level: Option<&str>,
        target_triple: Option<&str>,
        output_kind: &OutputKind,
        f: impl FnOnce(&TargetMachine, &TargetTriple) -> std::result::Result<R, String>,
    ) -> std::result::Result<R, String> {
        let (key, triple) = Self::target_machine_config(opt_level, target_triple, output_kind)?;
        TARGET_MACHINE_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let machine = cache.entry(key.clone()).or_insert_with(|| {
                Self::create_target_machine(&triple, &key, opt_level, output_kind)
                    .expect("failed to create target machine")
            });
            f(machine, &triple)
        })
    }

    fn create_target_machine(
        triple: &TargetTriple,
        key: &TargetMachineCacheKey,
        opt_level: Option<&str>,
        output_kind: &OutputKind,
    ) -> std::result::Result<TargetMachine, String> {
        let target = Target::from_triple(triple).map_err(|e| e.to_string())?;
        target
            .create_target_machine(
                triple,
                &key.cpu,
                &key.features,
                Self::resolve_optimization_level(opt_level),
                match output_kind {
                    OutputKind::Shared => RelocMode::PIC,
                    OutputKind::Bin | OutputKind::Static => RelocMode::Default,
                },
                CodeModel::Default,
            )
            .ok_or_else(|| "failed to create target machine".to_string())
    }

    pub fn emit_object_bytes(
        &self,
        opt_level: Option<&str>,
        target_triple: Option<&str>,
        output_kind: &OutputKind,
    ) -> std::result::Result<Vec<u8>, String> {
        Self::with_target_machine(opt_level, target_triple, output_kind, |machine, triple| {
            self.module.set_triple(triple);
            self.module
                .set_data_layout(&machine.get_target_data().get_data_layout());
            let buffer = machine
                .write_to_memory_buffer(&self.module, FileType::Object)
                .map_err(|e| e.to_string())?;
            Ok(buffer.as_slice().to_vec())
        })
    }

    pub fn write_object_with_config(
        &self,
        path: &Path,
        opt_level: Option<&str>,
        target_triple: Option<&str>,
        output_kind: &OutputKind,
    ) -> std::result::Result<(), String> {
        let object = self.emit_object_bytes(opt_level, target_triple, output_kind)?;
        std::fs::write(path, object).map_err(|e| e.to_string())
    }

    pub fn identify_captures(&self, expr: &Expr, params: &[Parameter]) -> Vec<(String, Type)> {
        let mut captures = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut local_names = std::collections::HashSet::new();
        for p in params {
            local_names.insert(p.name.clone());
        }

        self.walk_expr_for_captures(expr, &local_names, &mut captures, &mut seen);
        captures
    }

    fn add_pattern_bindings(
        &self,
        pattern: &Pattern,
        local_names: &mut std::collections::HashSet<String>,
    ) {
        match pattern {
            Pattern::Ident(name) => {
                let is_imported_unit_variant = self
                    .resolve_import_alias_variant(name)
                    .and_then(|(enum_name, variant_name)| {
                        self.enums
                            .get(&enum_name)
                            .and_then(|enum_info| enum_info.variants.get(&variant_name))
                            .is_some_and(|variant_info| variant_info.fields.is_empty())
                            .then_some(())
                    })
                    .is_some();
                if !is_imported_unit_variant {
                    local_names.insert(name.clone());
                }
            }
            Pattern::Variant(_, bindings) => {
                for binding in bindings {
                    local_names.insert(binding.clone());
                }
            }
            Pattern::Wildcard | Pattern::Literal(_) => {}
        }
    }

    fn walk_block_for_captures(
        &self,
        block: &[Spanned<Stmt>],
        local_names: &mut std::collections::HashSet<String>,
        captures: &mut Vec<(String, Type)>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        for stmt in block {
            self.walk_stmt_for_captures(&stmt.node, local_names, captures, seen);
        }
    }

    pub fn walk_expr_for_captures(
        &self,
        expr: &Expr,
        local_names: &std::collections::HashSet<String>,
        captures: &mut Vec<(String, Type)>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        match expr {
            Expr::This => {
                if !local_names.contains("this") && !seen.contains("this") {
                    if let Some(var) = self.variables.get("this") {
                        seen.insert("this".to_string());
                        captures.push(("this".to_string(), var.ty.clone()));
                    }
                }
            }
            Expr::Ident(name) => {
                if !local_names.contains(name) && !seen.contains(name) {
                    if let Some(var) = self.variables.get(name) {
                        seen.insert(name.clone());
                        captures.push((name.clone(), var.ty.clone()));
                    }
                }
            }
            Expr::Binary { left, right, .. } => {
                self.walk_expr_for_captures(&left.node, local_names, captures, seen);
                self.walk_expr_for_captures(&right.node, local_names, captures, seen);
            }
            Expr::Unary { expr, .. } => {
                self.walk_expr_for_captures(&expr.node, local_names, captures, seen);
            }
            Expr::Call { callee, args, .. } => {
                self.walk_expr_for_captures(&callee.node, local_names, captures, seen);
                for arg in args {
                    self.walk_expr_for_captures(&arg.node, local_names, captures, seen);
                }
            }
            Expr::Field { object, .. } => {
                self.walk_expr_for_captures(&object.node, local_names, captures, seen);
            }
            Expr::Index { object, index } => {
                self.walk_expr_for_captures(&object.node, local_names, captures, seen);
                self.walk_expr_for_captures(&index.node, local_names, captures, seen);
            }
            Expr::Construct { args, .. } => {
                for arg in args {
                    self.walk_expr_for_captures(&arg.node, local_names, captures, seen);
                }
            }
            Expr::StringInterp(parts) => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        self.walk_expr_for_captures(&e.node, local_names, captures, seen);
                    }
                }
            }
            Expr::Lambda {
                params: l_params,
                body: l_body,
            } => {
                let mut nested_params = local_names.clone();
                for p in l_params {
                    nested_params.insert(p.name.clone());
                }
                self.walk_expr_for_captures(&l_body.node, &nested_params, captures, seen);
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                self.walk_expr_for_captures(&condition.node, local_names, captures, seen);
                let mut then_locals = local_names.clone();
                self.walk_block_for_captures(then_branch, &mut then_locals, captures, seen);
                if let Some(block) = else_branch {
                    let mut else_locals = local_names.clone();
                    self.walk_block_for_captures(block, &mut else_locals, captures, seen);
                }
            }
            Expr::Match { expr, arms } => {
                self.walk_expr_for_captures(&expr.node, local_names, captures, seen);
                for arm in arms {
                    let mut arm_locals = local_names.clone();
                    self.add_pattern_bindings(&arm.pattern, &mut arm_locals);
                    self.walk_block_for_captures(&arm.body, &mut arm_locals, captures, seen);
                }
            }
            Expr::Try(inner)
            | Expr::Await(inner)
            | Expr::Borrow(inner)
            | Expr::MutBorrow(inner)
            | Expr::Deref(inner) => {
                self.walk_expr_for_captures(&inner.node, local_names, captures, seen);
            }
            Expr::AsyncBlock(stmts) | Expr::Block(stmts) => {
                let mut block_locals = local_names.clone();
                self.walk_block_for_captures(stmts, &mut block_locals, captures, seen);
            }
            Expr::Require { condition, message } => {
                self.walk_expr_for_captures(&condition.node, local_names, captures, seen);
                if let Some(message) = message {
                    self.walk_expr_for_captures(&message.node, local_names, captures, seen);
                }
            }
            Expr::Range { start, end, .. } => {
                if let Some(start) = start {
                    self.walk_expr_for_captures(&start.node, local_names, captures, seen);
                }
                if let Some(end) = end {
                    self.walk_expr_for_captures(&end.node, local_names, captures, seen);
                }
            }
            _ => {}
        }
    }

    pub fn walk_stmt_for_captures(
        &self,
        stmt: &Stmt,
        local_names: &mut std::collections::HashSet<String>,
        captures: &mut Vec<(String, Type)>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        match stmt {
            Stmt::Expr(e) => self.walk_expr_for_captures(&e.node, local_names, captures, seen),
            Stmt::Let { name, value, .. } => {
                self.walk_expr_for_captures(&value.node, local_names, captures, seen);
                local_names.insert(name.clone());
            }
            Stmt::Assign { target, value } => {
                self.walk_expr_for_captures(&target.node, local_names, captures, seen);
                self.walk_expr_for_captures(&value.node, local_names, captures, seen);
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.walk_expr_for_captures(&condition.node, local_names, captures, seen);
                let mut then_locals = local_names.clone();
                self.walk_block_for_captures(then_block, &mut then_locals, captures, seen);
                if let Some(eb) = else_block {
                    let mut else_locals = local_names.clone();
                    self.walk_block_for_captures(eb, &mut else_locals, captures, seen);
                }
            }
            Stmt::While { condition, body } => {
                self.walk_expr_for_captures(&condition.node, local_names, captures, seen);
                let mut body_locals = local_names.clone();
                self.walk_block_for_captures(body, &mut body_locals, captures, seen);
            }
            Stmt::For {
                var,
                iterable,
                body,
                ..
            } => {
                self.walk_expr_for_captures(&iterable.node, local_names, captures, seen);
                let mut body_locals = local_names.clone();
                body_locals.insert(var.clone());
                self.walk_block_for_captures(body, &mut body_locals, captures, seen);
            }
            Stmt::Return(Some(expr)) => {
                self.walk_expr_for_captures(&expr.node, local_names, captures, seen);
            }
            Stmt::Match { expr, arms } => {
                self.walk_expr_for_captures(&expr.node, local_names, captures, seen);
                for arm in arms {
                    let mut arm_locals = local_names.clone();
                    self.add_pattern_bindings(&arm.pattern, &mut arm_locals);
                    self.walk_block_for_captures(&arm.body, &mut arm_locals, captures, seen);
                }
            }
            Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
        }
    }

    pub fn infer_expr_type(&self, expr: &Expr, params: &[Parameter]) -> Type {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Integer(_) => Type::Integer,
                Literal::Float(_) => Type::Float,
                Literal::Boolean(_) => Type::Boolean,
                Literal::String(_) => Type::String,
                Literal::Char(_) => Type::Char,
                Literal::None => Type::None,
            },
            Expr::Ident(name) => {
                // Check parameters first
                if let Some(p) = params.iter().find(|p| p.name == *name) {
                    return p.ty.clone();
                }
                // Then local variables
                if let Some(var) = self.variables.get(name) {
                    return var.ty.clone();
                }
                // Then global functions
                if let Some((_, ty)) = self.functions.get(name) {
                    return ty.clone();
                }
                let resolved_name = self.resolve_function_alias(name);
                if resolved_name != *name {
                    if let Some((_, ty)) = self.functions.get(&resolved_name) {
                        return ty.clone();
                    }
                }
                if let Some((enum_name, variant_name)) = self.resolve_import_alias_variant(name) {
                    if self
                        .enums
                        .get(&enum_name)
                        .and_then(|enum_info| enum_info.variants.get(&variant_name))
                        .is_some_and(|variant_info| variant_info.fields.is_empty())
                    {
                        return Type::Named(enum_name);
                    }
                }
                Type::Integer
            }
            Expr::Binary { op, left, .. } => match op {
                BinOp::Eq
                | BinOp::NotEq
                | BinOp::Lt
                | BinOp::LtEq
                | BinOp::Gt
                | BinOp::GtEq
                | BinOp::And
                | BinOp::Or => Type::Boolean,
                _ => self.infer_expr_type(&left.node, params),
            },
            Expr::Unary { op, expr } => match op {
                UnaryOp::Not => Type::Boolean,
                UnaryOp::Neg => self.infer_expr_type(&expr.node, params),
            },
            Expr::Call { callee, .. } => match &callee.node {
                Expr::Ident(name) if name == "println" => Type::None,
                Expr::Ident(name) if name == "to_string" => Type::String,
                Expr::Ident(name) if name == "range" => {
                    if let Expr::Call { args, .. } = expr {
                        if let Some(first_arg) = args.first() {
                            Type::Range(Box::new(self.infer_expr_type(&first_arg.node, params)))
                        } else {
                            Type::Range(Box::new(Type::Integer))
                        }
                    } else {
                        Type::Range(Box::new(Type::Integer))
                    }
                }
                _ => {
                    if let Some(function_name) =
                        self.resolve_contextual_function_value_name(&callee.node)
                    {
                        if let Some((_, Type::Function(_, ret_ty))) =
                            self.functions.get(&function_name)
                        {
                            return (**ret_ty).clone();
                        }
                    }
                    match &callee.node {
                        Expr::Field { object, field } => {
                            if let Expr::Ident(owner_name) = &object.node {
                                let resolved_owner = self.resolve_module_alias(owner_name);
                                if self
                                    .enums
                                    .get(&resolved_owner)
                                    .and_then(|info| info.variants.get(field))
                                    .is_some()
                                {
                                    return Type::Named(resolved_owner);
                                }
                                if resolved_owner == "Str" {
                                    return match field.as_str() {
                                        "len" | "compare" => Type::Integer,
                                        "concat" | "upper" | "lower" | "trim" => Type::String,
                                        "contains" | "startsWith" | "endsWith" => Type::Boolean,
                                        _ => Type::Integer,
                                    };
                                }
                                match (resolved_owner.as_str(), field.as_str()) {
                                    ("Option", "some") => {
                                        if let Expr::Call { args, .. } = expr {
                                            if let Some(first_arg) = args.first() {
                                                return Type::Option(Box::new(
                                                    self.infer_expr_type(&first_arg.node, params),
                                                ));
                                            }
                                        }
                                    }
                                    ("Option", "none") => {
                                        return Type::Option(Box::new(Type::Integer));
                                    }
                                    ("Result", "ok") | ("Result", "error") => {
                                        if let Some(expected) = self.infer_object_type(expr) {
                                            return expected;
                                        }
                                        if let Expr::Call { args, .. } = expr {
                                            if let Some(first_arg) = args.first() {
                                                let arg_ty =
                                                    self.infer_expr_type(&first_arg.node, params);
                                                return match field.as_str() {
                                                    "ok" => Type::Result(
                                                        Box::new(arg_ty),
                                                        Box::new(Type::String),
                                                    ),
                                                    "error" => Type::Result(
                                                        Box::new(Type::Integer),
                                                        Box::new(arg_ty),
                                                    ),
                                                    _ => unreachable!(),
                                                };
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            let obj_ty = self.infer_expr_type(&object.node, params);
                            if let Some(ret_ty) = self.builtin_method_return_type(&obj_ty, field) {
                                return ret_ty;
                            }
                            let callee_ty = self.infer_expr_type(&callee.node, params);
                            if let Type::Function(_, ret_ty) = callee_ty {
                                *ret_ty
                            } else {
                                Type::Integer
                            }
                        }
                        _ => {
                            let callee_ty = self.infer_expr_type(&callee.node, params);
                            if let Type::Function(_, ret_ty) = callee_ty {
                                *ret_ty
                            } else {
                                Type::Integer
                            }
                        }
                    }
                }
            },
            Expr::Field { object, field } => {
                if let Some(function_name) = self.resolve_contextual_function_value_name(expr) {
                    if let Some((_, ty)) = self.functions.get(&function_name) {
                        return ty.clone();
                    }
                }
                if let Some(canonical_owner) = self.resolve_unit_enum_variant_owner(expr) {
                    return Type::Named(canonical_owner);
                }
                let obj_ty = self.infer_expr_type(&object.node, params);
                if let Some(class_name) = self.type_to_class_name(&obj_ty) {
                    if let Some(class_info) = self.classes.get(&class_name) {
                        if let Some(field_ty) = class_info.field_types.get(field) {
                            if let Type::Generic(_, args) = &obj_ty {
                                if class_info.generic_params.len() == args.len() {
                                    let bindings = class_info
                                        .generic_params
                                        .iter()
                                        .cloned()
                                        .zip(args.iter().cloned())
                                        .collect::<HashMap<_, _>>();
                                    return Self::substitute_type(field_ty, &bindings);
                                }
                            }
                            return field_ty.clone();
                        }
                    }
                    if let Some(method_name) = self.resolve_method_function_name(&class_name, field)
                    {
                        if let Some((_, ty)) = self.functions.get(&method_name) {
                            return ty.clone();
                        }
                    }
                }
                Type::Integer
            }
            Expr::Construct { ty, .. } => parse_type_source(ty).unwrap_or(Type::Integer),
            Expr::Index { object, .. } => match self.infer_expr_type(&object.node, params) {
                Type::List(inner) => *inner,
                Type::Map(_, value) => *value,
                Type::String => Type::Char,
                _ => Type::Integer,
            },
            Expr::Lambda { params, body } => {
                let ret_ty = self.infer_expr_type(&body.node, params);
                Type::Function(
                    params.iter().map(|p| p.ty.clone()).collect(),
                    Box::new(ret_ty),
                )
            }
            Expr::Block(stmts) => self
                .infer_block_tail_type_with_params(stmts, params)
                .unwrap_or(Type::None),
            Expr::IfExpr {
                then_branch,
                else_branch,
                ..
            } => self.infer_if_expr_result_type(then_branch, else_branch.as_ref(), params),
            Expr::Match { arms, .. } => self.infer_match_expr_result_type(arms, params),
            Expr::Await(inner) => {
                let inner_ty = self.infer_expr_type(&inner.node, params);
                self.task_inner_type(&inner_ty).unwrap_or(Type::Integer)
            }
            Expr::Try(inner) => match self.infer_expr_type(&inner.node, params) {
                Type::Option(inner) => *inner,
                Type::Result(ok, _) => *ok,
                _ => Type::Integer,
            },
            Expr::Borrow(inner) => Type::Ref(Box::new(self.infer_expr_type(&inner.node, params))),
            Expr::MutBorrow(inner) => {
                Type::MutRef(Box::new(self.infer_expr_type(&inner.node, params)))
            }
            Expr::Deref(inner) => match self.infer_expr_type(&inner.node, params) {
                Type::Ref(inner) | Type::MutRef(inner) => *inner,
                _ => Type::Integer,
            },
            Expr::StringInterp(_) => Type::String,
            Expr::AsyncBlock(stmts) => Type::Task(Box::new(
                self.infer_block_tail_type(stmts).unwrap_or(Type::None),
            )),
            Expr::Require { .. } => Type::None,
            Expr::Range { .. } => Type::Range(Box::new(Type::Integer)),
            _ => Type::Integer,
        }
    }

    pub fn get_or_declare_malloc(&self) -> FunctionValue<'ctx> {
        let name = "malloc";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }

        let malloc_type = self
            .context
            .ptr_type(AddressSpace::default())
            .fn_type(&[self.context.i64_type().into()], false);
        self.module.add_function(name, malloc_type, None)
    }

    pub fn get_or_declare_realloc(&self) -> FunctionValue<'ctx> {
        let name = "realloc";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }

        let realloc_type = self.context.ptr_type(AddressSpace::default()).fn_type(
            &[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.i64_type().into(),
            ],
            false,
        );
        self.module.add_function(name, realloc_type, None)
    }

    pub fn get_or_declare_pthread_create(&self) -> FunctionValue<'ctx> {
        let name = "pthread_create";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }

        let ptr = self.context.ptr_type(AddressSpace::default());
        let pthread_create_type = self
            .context
            .i32_type()
            .fn_type(&[ptr.into(), ptr.into(), ptr.into(), ptr.into()], false);
        self.module.add_function(name, pthread_create_type, None)
    }

    pub fn get_or_declare_pthread_join(&self) -> FunctionValue<'ctx> {
        let name = "pthread_join";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }

        let ptr = self.context.ptr_type(AddressSpace::default());
        let pthread_join_type = self
            .context
            .i32_type()
            .fn_type(&[self.context.i64_type().into(), ptr.into()], false);
        self.module.add_function(name, pthread_join_type, None)
    }

    pub fn get_or_declare_pthread_cancel(&self) -> FunctionValue<'ctx> {
        let name = "pthread_cancel";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }

        let pthread_cancel_type = self
            .context
            .i32_type()
            .fn_type(&[self.context.i64_type().into()], false);
        self.module.add_function(name, pthread_cancel_type, None)
    }

    pub fn get_or_declare_pthread_timedjoin_np(&self) -> FunctionValue<'ctx> {
        let name = "pthread_timedjoin_np";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }

        let ptr = self.context.ptr_type(AddressSpace::default());
        let pthread_timedjoin_type = self.context.i32_type().fn_type(
            &[self.context.i64_type().into(), ptr.into(), ptr.into()],
            false,
        );
        self.module.add_function(name, pthread_timedjoin_type, None)
    }

    pub fn get_or_declare_create_thread_win(&self) -> FunctionValue<'ctx> {
        let name = "CreateThread";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }

        let ptr = self.context.ptr_type(AddressSpace::default());
        let usize_ty = self.context.i64_type();
        let create_thread_type = ptr.fn_type(
            &[
                ptr.into(),
                usize_ty.into(),
                ptr.into(),
                ptr.into(),
                self.context.i32_type().into(),
                ptr.into(),
            ],
            false,
        );
        self.module.add_function(name, create_thread_type, None)
    }

    pub fn get_or_declare_wait_for_single_object_win(&self) -> FunctionValue<'ctx> {
        let name = "WaitForSingleObject";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }

        let ptr = self.context.ptr_type(AddressSpace::default());
        let wait_type = self
            .context
            .i32_type()
            .fn_type(&[ptr.into(), self.context.i32_type().into()], false);
        self.module.add_function(name, wait_type, None)
    }

    pub fn get_or_declare_terminate_thread_win(&self) -> FunctionValue<'ctx> {
        let name = "TerminateThread";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }

        let ptr = self.context.ptr_type(AddressSpace::default());
        let terminate_type = self
            .context
            .i32_type()
            .fn_type(&[ptr.into(), self.context.i32_type().into()], false);
        self.module.add_function(name, terminate_type, None)
    }

    pub fn get_or_declare_close_handle_win(&self) -> FunctionValue<'ctx> {
        let name = "CloseHandle";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }

        let ptr = self.context.ptr_type(AddressSpace::default());
        let close_type = self.context.i32_type().fn_type(&[ptr.into()], false);
        self.module.add_function(name, close_type, None)
    }

    pub fn get_or_declare_clock_gettime(&self) -> FunctionValue<'ctx> {
        let name = "clock_gettime";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }

        let ptr = self.context.ptr_type(AddressSpace::default());
        let clock_gettime_type = self
            .context
            .i32_type()
            .fn_type(&[self.context.i32_type().into(), ptr.into()], false);
        self.module.add_function(name, clock_gettime_type, None)
    }

    pub fn get_or_declare_sprintf(&self) -> FunctionValue<'ctx> {
        let name = "sprintf";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }

        let sprintf_type = self.context.i32_type().fn_type(
            &[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            true,
        );
        self.module.add_function(name, sprintf_type, None)
    }

    // === Range Implementation ===

    /// Get or create the Range struct type: { start, end, step, current }
    pub fn get_range_type(
        &self,
        element_type: inkwell::types::BasicTypeEnum<'ctx>,
    ) -> Result<StructType<'ctx>> {
        let range_name = match element_type {
            inkwell::types::BasicTypeEnum::IntType(_) => "RangeI64",
            inkwell::types::BasicTypeEnum::FloatType(_) => "RangeF64",
            _ => {
                return Err(CodegenError::new(
                    "Range<T> codegen supports only Integer and Float elements",
                ));
            }
        };
        if let Some(s) = self.module.get_struct_type(range_name) {
            return Ok(s);
        }
        let range_type = self.context.opaque_struct_type(range_name);
        let fields = [element_type, element_type, element_type, element_type];
        range_type.set_body(&fields, false);
        Ok(range_type)
    }

    /// Create a new Range instance
    pub fn create_range(
        &mut self,
        start: BasicValueEnum<'ctx>,
        end: BasicValueEnum<'ctx>,
        step: BasicValueEnum<'ctx>,
    ) -> Result<PointerValue<'ctx>> {
        let element_type = start.get_type();
        if end.get_type() != element_type || step.get_type() != element_type {
            return Err(CodegenError::new(
                "range() codegen requires start/end/step to share the same type",
            ));
        }
        let range_type = self.get_range_type(element_type)?;
        let malloc = self.get_or_declare_malloc();
        let printf = self.get_or_declare_printf();
        let exit_fn = self.get_or_declare_exit();
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_parent())
            .ok_or_else(|| CodegenError::new("Range creation must occur inside a function"))?;
        let zero_step_bb = self
            .context
            .append_basic_block(current_fn, "range_zero_step");
        let ok_bb = self.context.append_basic_block(current_fn, "range_init");

        // Allocate memory for Range struct
        let size = range_type.size_of().unwrap();
        let alloc_call = self
            .builder
            .build_call(malloc, &[size.into()], "range_alloc")
            .unwrap();
        let range_ptr = match alloc_call.try_as_basic_value() {
            ValueKind::Basic(inkwell::values::BasicValueEnum::PointerValue(p)) => p,
            _ => return Err(CodegenError::new("malloc should return pointer")),
        };

        // Initialize fields - use i32 for GEP indices as required by LLVM
        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);
        let one = i32_type.const_int(1, false);
        let two = i32_type.const_int(2, false);
        let three = i32_type.const_int(3, false);

        let step_is_zero = match step {
            BasicValueEnum::IntValue(step) => self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    step,
                    step.get_type().const_zero(),
                    "range_step_is_zero",
                )
                .unwrap(),
            BasicValueEnum::FloatValue(step) => self
                .builder
                .build_float_compare(
                    inkwell::FloatPredicate::OEQ,
                    step,
                    step.get_type().const_float(0.0),
                    "range_step_is_zero",
                )
                .unwrap(),
            _ => {
                return Err(CodegenError::new(
                    "range() codegen supports only Integer and Float elements",
                ));
            }
        };
        self.builder
            .build_conditional_branch(step_is_zero, zero_step_bb, ok_bb)
            .unwrap();

        self.builder.position_at_end(zero_step_bb);
        let panic_global = if let Some(existing) = self.module.get_global("range_zero_step_panic") {
            existing
        } else {
            let panic_msg = self
                .context
                .const_string(b"Runtime error: range() step cannot be 0\n\0", false);
            let global =
                self.module
                    .add_global(panic_msg.get_type(), None, "range_zero_step_panic");
            global.set_linkage(Linkage::Private);
            global.set_initializer(&panic_msg);
            global
        };
        self.builder
            .build_call(printf, &[panic_global.as_pointer_value().into()], "")
            .unwrap();
        self.builder
            .build_call(
                exit_fn,
                &[self.context.i32_type().const_int(1, false).into()],
                "",
            )
            .unwrap();
        self.builder.build_unreachable().unwrap();

        self.builder.position_at_end(ok_bb);

        // Store start
        let start_ptr = unsafe {
            self.builder
                .build_gep(range_type, range_ptr, &[zero, zero], "range_start_ptr")
                .unwrap()
        };
        self.builder.build_store(start_ptr, start).unwrap();

        // Store end
        let end_ptr = unsafe {
            self.builder
                .build_gep(range_type, range_ptr, &[zero, one], "range_end_ptr")
                .unwrap()
        };
        self.builder.build_store(end_ptr, end).unwrap();

        // Store step
        let step_ptr = unsafe {
            self.builder
                .build_gep(range_type, range_ptr, &[zero, two], "range_step_ptr")
                .unwrap()
        };
        self.builder.build_store(step_ptr, step).unwrap();

        // Store current = start
        let current_ptr = unsafe {
            self.builder
                .build_gep(range_type, range_ptr, &[zero, three], "range_current_ptr")
                .unwrap()
        };
        self.builder.build_store(current_ptr, start).unwrap();

        Ok(range_ptr)
    }

    /// Get the next value from a Range iterator
    /// Returns (value, has_more) tuple
    pub fn range_next(
        &mut self,
        range_ptr: PointerValue<'ctx>,
    ) -> Result<(BasicValueEnum<'ctx>, BasicValueEnum<'ctx>)> {
        let range_type = self.get_range_type(self.context.i64_type().into())?;
        let i64_type = self.context.i64_type();
        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);
        let one = i32_type.const_int(1, false);
        let two = i32_type.const_int(2, false);
        let three = i32_type.const_int(3, false);

        // Load current value
        let current_ptr = unsafe {
            self.builder
                .build_gep(range_type, range_ptr, &[zero, three], "current_ptr")
                .unwrap()
        };
        let current = self
            .builder
            .build_load(i64_type, current_ptr, "current")
            .unwrap();

        // Load step
        let step_ptr = unsafe {
            self.builder
                .build_gep(range_type, range_ptr, &[zero, two], "step_ptr")
                .unwrap()
        };
        let step = self.builder.build_load(i64_type, step_ptr, "step").unwrap();

        // Calculate next: current + step
        let next_val = self
            .builder
            .build_int_add(current.into_int_value(), step.into_int_value(), "next")
            .unwrap();
        self.builder.build_store(current_ptr, next_val).unwrap();

        // Load end to check if we're done
        let end_ptr = unsafe {
            self.builder
                .build_gep(range_type, range_ptr, &[zero, one], "end_ptr")
                .unwrap()
        };
        let end = self.builder.build_load(i64_type, end_ptr, "end").unwrap();

        // Load step to determine comparison direction
        let step_val = step.into_int_value();
        let step_is_positive = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::SGT,
                step_val,
                i64_type.const_int(0, false),
                "step_positive",
            )
            .unwrap();

        // has_more = step > 0 ? current < end : current > end
        let cmp_positive = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::SLT,
                current.into_int_value(),
                end.into_int_value(),
                "cmp_pos",
            )
            .unwrap();

        let cmp_negative = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::SGT,
                current.into_int_value(),
                end.into_int_value(),
                "cmp_neg",
            )
            .unwrap();

        let has_more = self
            .builder
            .build_select(step_is_positive, cmp_positive, cmp_negative, "has_more")
            .unwrap();

        Ok((current, has_more))
    }

    /// Check if Range has more elements
    pub fn range_has_next(
        &mut self,
        range_ptr: PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let range_type = self.get_range_type(self.context.i64_type().into())?;
        let i64_type = self.context.i64_type();
        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);
        let one = i32_type.const_int(1, false);
        let two = i32_type.const_int(2, false);
        let three = i32_type.const_int(3, false);

        // Load current
        let current_ptr = unsafe {
            self.builder
                .build_gep(range_type, range_ptr, &[zero, three], "current_ptr")
                .unwrap()
        };
        let current = self
            .builder
            .build_load(i64_type, current_ptr, "current")
            .unwrap();

        // Load end
        let end_ptr = unsafe {
            self.builder
                .build_gep(range_type, range_ptr, &[zero, one], "end_ptr")
                .unwrap()
        };
        let end = self.builder.build_load(i64_type, end_ptr, "end").unwrap();

        // Load step
        let step_ptr = unsafe {
            self.builder
                .build_gep(range_type, range_ptr, &[zero, two], "step_ptr")
                .unwrap()
        };
        let step = self.builder.build_load(i64_type, step_ptr, "step").unwrap();

        // Check step direction
        let step_is_positive = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::SGT,
                step.into_int_value(),
                i64_type.const_int(0, false),
                "step_positive",
            )
            .unwrap();

        // Compare based on direction
        let cmp_positive = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::SLT,
                current.into_int_value(),
                end.into_int_value(),
                "cmp_pos",
            )
            .unwrap();

        let cmp_negative = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::SGT,
                current.into_int_value(),
                end.into_int_value(),
                "cmp_neg",
            )
            .unwrap();

        let has_more = self
            .builder
            .build_select(step_is_positive, cmp_positive, cmp_negative, "has_more")
            .unwrap();

        Ok(has_more)
    }
}
