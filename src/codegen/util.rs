//! Utility functions for codegen: C library declarations and helper methods
#![allow(dead_code)]

use crate::ast::{
    BinOp, Expr, Literal, MatchArm, Parameter, Pattern, Spanned, Stmt, StringPart, Type, UnaryOp,
};
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
    // === C Library Definitions ===

    pub fn get_or_declare_fopen(&mut self) -> FunctionValue<'ctx> {
        let name = "fopen";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        // FILE* fopen(const char* filename, const char* mode)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_fclose(&mut self) -> FunctionValue<'ctx> {
        let name = "fclose";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        // int fclose(FILE* stream)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i32_type().fn_type(&[ptr_type.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_fputs(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_fseek(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_ftell(&mut self) -> FunctionValue<'ctx> {
        let name = "ftell";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        // long ftell(FILE* stream)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i64_type().fn_type(&[ptr_type.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_rewind(&mut self) -> FunctionValue<'ctx> {
        let name = "rewind";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        // void rewind(FILE* stream)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.void_type().fn_type(&[ptr_type.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_fread(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_remove(&mut self) -> FunctionValue<'ctx> {
        let name = "remove";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        // int remove(const char* filename)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i32_type().fn_type(&[ptr_type.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_rand(&mut self) -> FunctionValue<'ctx> {
        let name = "rand";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let fn_type = self.context.i32_type().fn_type(&[], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_toupper(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_tolower(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_isspace(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_strstr(&mut self) -> FunctionValue<'ctx> {
        let name = "strstr";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr.fn_type(&[ptr.into(), ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_strncpy(&mut self) -> FunctionValue<'ctx> {
        let name = "strncpy";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let size_t = self.context.i64_type();
        let fn_type = ptr.fn_type(&[ptr.into(), ptr.into(), size_t.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_create_empty_string(&mut self) -> inkwell::values::PointerValue<'ctx> {
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

    pub fn get_or_declare_time(&mut self) -> FunctionValue<'ctx> {
        let name = "time";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i64_type().fn_type(&[ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_localtime(&mut self) -> FunctionValue<'ctx> {
        let name = "localtime";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr.fn_type(&[ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_strftime(&mut self) -> FunctionValue<'ctx> {
        let name = "strftime";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let size_t = self.context.i64_type();
        let fn_type = size_t.fn_type(&[ptr.into(), size_t.into(), ptr.into(), ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_sleep_win(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_usleep(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_getenv(&mut self) -> FunctionValue<'ctx> {
        let name = "getenv";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr.fn_type(&[ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_system(&mut self) -> FunctionValue<'ctx> {
        let name = "system";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr = self.context.ptr_type(AddressSpace::default());
        let fn_type = self.context.i32_type().fn_type(&[ptr.into()], false);
        self.module.add_function(name, fn_type, None)
    }

    pub fn get_or_declare_popen(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_pclose(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_getcwd(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_math_func(
        &mut self,
        name: &str,
        single_arg: bool,
    ) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_strlen(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_strcmp(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_strncmp(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_strcpy(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_strcat(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_fgets(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_stdin(&mut self) -> PointerValue<'ctx> {
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
        // stdin is __acrt_iob_func(0) on Windows
        let call = self
            .builder
            .build_call(
                func,
                &[self.context.i32_type().const_int(0, false).into()],
                "stdin",
            )
            .unwrap();
        self.extract_call_value(call).into_pointer_value()
    }

    pub fn get_or_declare_exit(&mut self) -> FunctionValue<'ctx> {
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
        // Get pointer to the lvalue
        let ptr = self.compile_lvalue(expr)?;
        Ok(ptr.into())
    }

    pub fn compile_deref(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>> {
        // Compile the expression to get a pointer value
        let ptr_val = self.compile_expr(expr)?.into_pointer_value();

        // For now, assume i64 as the default dereferenced type
        // A full implementation would track the reference type
        let val = self
            .builder
            .build_load(self.context.i64_type(), ptr_val, "deref")
            .unwrap();
        Ok(val)
    }

    // === Lambda functions ===

    pub fn compile_lambda(
        &mut self,
        params: &[Parameter],
        body: &Spanned<Expr>,
    ) -> Result<BasicValueEnum<'ctx>> {
        // 1. Identify captures
        let captures = self.identify_captures(&body.node, params);

        // 2. Infer return type
        let ret_apex_ty = self.infer_expr_type(&body.node, params);
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
        let result = self.compile_expr(&body.node)?;

        // Build return with proper casting if needed
        let final_result = if result.get_type() != ret_llvm_ty {
            // Handle i32 to i64 (like from println)
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
    ) -> Result<BasicValueEnum<'ctx>> {
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

        for arm in arms {
            let arm_bb = self.context.append_basic_block(func, "match.expr.arm");
            let next_bb = self.context.append_basic_block(func, "match.expr.next");

            self.builder.position_at_end(dispatch_bb);
            match &arm.pattern {
                Pattern::Wildcard | Pattern::Ident(_) => {
                    self.builder.build_unconditional_branch(arm_bb).unwrap();
                }
                Pattern::Literal(lit) => {
                    let pattern_val = self.compile_literal(lit)?;
                    let cond = if val.is_int_value() && pattern_val.is_int_value() {
                        self.builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                val.into_int_value(),
                                pattern_val.into_int_value(),
                                "match_expr_lit_eq",
                            )
                            .unwrap()
                    } else if val.is_float_value() && pattern_val.is_float_value() {
                        self.builder
                            .build_float_compare(
                                FloatPredicate::OEQ,
                                val.into_float_value(),
                                pattern_val.into_float_value(),
                                "match_expr_float_eq",
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
                    if matches!(variant_name.as_str(), "Some" | "None" | "Ok" | "Error") {
                        let expected_tag = match variant_name.as_str() {
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
                    } else if let Some(enum_name) = &enum_match_name {
                        if let Some(enum_info) = self.enums.get(enum_name) {
                            if let Some(variant_info) = enum_info.variants.get(variant_name) {
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
                Pattern::Variant(variant_name, bindings) => {
                    if variant_name == "Some" && !bindings.is_empty() {
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
                    } else if variant_name == "Ok" && !bindings.is_empty() {
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
                    } else if variant_name == "Error" && !bindings.is_empty() {
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
                    } else if let Some(enum_name) = &enum_match_name {
                        if let Some(enum_info) = self.enums.get(enum_name) {
                            if let Some(variant_info) = enum_info.variants.get(variant_name) {
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
                        arm_result = self.compile_expr(&e.node)?;
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
                let obj_ptr = self.compile_expr(&object.node)?.into_pointer_value();

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

                // Prefer typed list element pointer for List<T> index assignment.
                if let Some(Type::List(inner)) = self.infer_object_type(&object.node) {
                    let list_ptr = match &object.node {
                        Expr::Ident(name) => self.variables.get(name).map(|v| v.ptr),
                        Expr::Field { object: obj, field } => {
                            self.compile_field_ptr(&obj.node, field).ok()
                        }
                        Expr::This => self.variables.get("this").map(|v| v.ptr),
                        _ => None,
                    };
                    if let Some(list_ptr) = list_ptr {
                        let elem_ty = self.llvm_type(&inner);
                        let list_type = self.context.struct_type(
                            &[
                                self.context.i64_type().into(),
                                self.context.i64_type().into(),
                                self.context.ptr_type(AddressSpace::default()).into(),
                            ],
                            false,
                        );
                        let i32_type = self.context.i32_type();
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
                        let data_ptr = self
                            .builder
                            .build_load(
                                self.context.ptr_type(AddressSpace::default()),
                                data_ptr_ptr,
                                "list_data",
                            )
                            .unwrap()
                            .into_pointer_value();
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
                }

                let obj_ptr = self.compile_expr(&object.node)?.into_pointer_value();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i64_type(), obj_ptr, &[idx_val], "idx_elem_ptr")
                        .unwrap()
                };
                Ok(elem_ptr)
            }
            _ => Err(CodegenError::new("Invalid lvalue")),
        }
    }

    // === Helpers ===

    /// Infer the Apex Type of an expression
    pub fn infer_object_type(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Ident(name) => self.variables.get(name).map(|v| v.ty.clone()),
            Expr::This => self.variables.get("this").map(|v| v.ty.clone()),
            Expr::Field { object, field } => {
                let obj_ty = self.infer_object_type(&object.node)?;
                let class_name = match &obj_ty {
                    Type::Named(n) => n.clone(),
                    Type::Generic(n, _) => n.clone(),
                    _ => return None,
                };
                let class_info = self.classes.get(&class_name)?;
                class_info.field_types.get(field).cloned()
            }
            _ => None,
        }
    }

    /// Extract class name from a Type (handles Named, Ref, MutRef, etc.)
    #[allow(clippy::only_used_in_recursion)]
    pub fn type_to_class_name(&self, ty: &Type) -> Option<String> {
        match ty {
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

    pub fn needs_terminator(&self) -> bool {
        self.builder
            .get_insert_block()
            .map(|b| b.get_terminator().is_none())
            .unwrap_or(false)
    }

    pub fn get_or_declare_printf(&mut self) -> FunctionValue<'ctx> {
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
        let mut param_names = std::collections::HashSet::new();
        for p in params {
            param_names.insert(p.name.clone());
        }

        self.walk_expr_for_captures(expr, &param_names, &mut captures, &mut seen);
        captures
    }

    pub fn walk_expr_for_captures(
        &self,
        expr: &Expr,
        params: &std::collections::HashSet<String>,
        captures: &mut Vec<(String, Type)>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        match expr {
            Expr::Ident(name) => {
                if !params.contains(name) && !seen.contains(name) {
                    if let Some(var) = self.variables.get(name) {
                        seen.insert(name.clone());
                        captures.push((name.clone(), var.ty.clone()));
                    }
                }
            }
            Expr::Binary { left, right, .. } => {
                self.walk_expr_for_captures(&left.node, params, captures, seen);
                self.walk_expr_for_captures(&right.node, params, captures, seen);
            }
            Expr::Unary { expr, .. } => {
                self.walk_expr_for_captures(&expr.node, params, captures, seen);
            }
            Expr::Call { callee, args, .. } => {
                self.walk_expr_for_captures(&callee.node, params, captures, seen);
                for arg in args {
                    self.walk_expr_for_captures(&arg.node, params, captures, seen);
                }
            }
            Expr::Field { object, .. } => {
                self.walk_expr_for_captures(&object.node, params, captures, seen);
            }
            Expr::Index { object, index } => {
                self.walk_expr_for_captures(&object.node, params, captures, seen);
                self.walk_expr_for_captures(&index.node, params, captures, seen);
            }
            Expr::Construct { args, .. } => {
                for arg in args {
                    self.walk_expr_for_captures(&arg.node, params, captures, seen);
                }
            }
            Expr::StringInterp(parts) => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        self.walk_expr_for_captures(&e.node, params, captures, seen);
                    }
                }
            }
            Expr::Lambda {
                params: l_params,
                body: l_body,
            } => {
                let mut nested_params = params.clone();
                for p in l_params {
                    nested_params.insert(p.name.clone());
                }
                self.walk_expr_for_captures(&l_body.node, &nested_params, captures, seen);
            }
            Expr::Match { expr, arms } => {
                self.walk_expr_for_captures(&expr.node, params, captures, seen);
                for arm in arms {
                    for stmt in &arm.body {
                        self.walk_stmt_for_captures(&stmt.node, params, captures, seen);
                    }
                }
            }
            Expr::Try(inner) => {
                self.walk_expr_for_captures(&inner.node, params, captures, seen);
            }
            Expr::Await(inner) => {
                self.walk_expr_for_captures(&inner.node, params, captures, seen);
            }
            Expr::AsyncBlock(stmts) => {
                for stmt in stmts {
                    self.walk_stmt_for_captures(&stmt.node, params, captures, seen);
                }
            }
            _ => {}
        }
    }

    pub fn walk_stmt_for_captures(
        &self,
        stmt: &Stmt,
        params: &std::collections::HashSet<String>,
        captures: &mut Vec<(String, Type)>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        match stmt {
            Stmt::Expr(e) => self.walk_expr_for_captures(&e.node, params, captures, seen),
            Stmt::Let { value, .. } => {
                self.walk_expr_for_captures(&value.node, params, captures, seen);
                // Let doesn't capture the variable it's declaring, but we'll ignore shadowing for now
            }
            Stmt::Assign { target, value } => {
                self.walk_expr_for_captures(&target.node, params, captures, seen);
                self.walk_expr_for_captures(&value.node, params, captures, seen);
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.walk_expr_for_captures(&condition.node, params, captures, seen);
                for s in then_block {
                    self.walk_stmt_for_captures(&s.node, params, captures, seen);
                }
                if let Some(eb) = else_block {
                    for s in eb {
                        self.walk_stmt_for_captures(&s.node, params, captures, seen);
                    }
                }
            }
            Stmt::While { condition, body } => {
                self.walk_expr_for_captures(&condition.node, params, captures, seen);
                for s in body {
                    self.walk_stmt_for_captures(&s.node, params, captures, seen);
                }
            }
            Stmt::For { iterable, body, .. } => {
                self.walk_expr_for_captures(&iterable.node, params, captures, seen);
                for s in body {
                    self.walk_stmt_for_captures(&s.node, params, captures, seen);
                }
            }
            Stmt::Return(Some(expr)) => {
                self.walk_expr_for_captures(&expr.node, params, captures, seen);
            }
            _ => {}
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
                _ => {
                    let callee_ty = self.infer_expr_type(&callee.node, params);
                    if let Type::Function(_, ret_ty) = callee_ty {
                        *ret_ty
                    } else {
                        Type::Integer
                    }
                }
            },
            Expr::Field { object, field } => {
                let obj_ty = self.infer_expr_type(&object.node, params);
                if let Some(class_name) = self.type_to_class_name(&obj_ty) {
                    if let Some(class_info) = self.classes.get(&class_name) {
                        if let Some(field_ty) = class_info.field_types.get(field) {
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
            Expr::Lambda { params, body } => {
                let ret_ty = self.infer_expr_type(&body.node, params);
                Type::Function(
                    params.iter().map(|p| p.ty.clone()).collect(),
                    Box::new(ret_ty),
                )
            }
            Expr::Await(inner) => {
                let inner_ty = self.infer_expr_type(&inner.node, params);
                if let Type::Task(task_inner) = inner_ty {
                    *task_inner
                } else {
                    Type::Integer
                }
            }
            Expr::AsyncBlock(stmts) => {
                let mut ret = Type::None;
                for stmt in stmts {
                    if let Stmt::Return(Some(expr)) = &stmt.node {
                        ret = self.infer_expr_type(&expr.node, params);
                        break;
                    }
                }
                Type::Task(Box::new(ret))
            }
            _ => Type::Integer,
        }
    }

    pub fn get_or_declare_malloc(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_realloc(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_pthread_create(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_pthread_join(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_pthread_cancel(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_pthread_timedjoin_np(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_create_thread_win(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_wait_for_single_object_win(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_terminate_thread_win(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_close_handle_win(&mut self) -> FunctionValue<'ctx> {
        let name = "CloseHandle";
        if let Some(f) = self.module.get_function(name) {
            return f;
        }

        let ptr = self.context.ptr_type(AddressSpace::default());
        let close_type = self.context.i32_type().fn_type(&[ptr.into()], false);
        self.module.add_function(name, close_type, None)
    }

    pub fn get_or_declare_clock_gettime(&mut self) -> FunctionValue<'ctx> {
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

    pub fn get_or_declare_sprintf(&mut self) -> FunctionValue<'ctx> {
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
