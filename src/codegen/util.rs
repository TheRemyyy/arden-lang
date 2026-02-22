//! Utility functions for codegen: C library declarations and helper methods
#![allow(dead_code)]

use crate::ast::{
    BinOp, Expr, Literal, MatchArm, Parameter, Pattern, Spanned, Stmt, StringPart, Type, UnaryOp,
};

use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue, ValueKind};
use inkwell::{AddressSpace, IntPredicate, OptimizationLevel};

use std::path::Path;

use crate::codegen::core::{Codegen, CodegenError, Result, Variable};

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

        // Position builder back to the original function
        if let Some(func) = saved_function {
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
        // Simplified: return value of first matching arm
        let _val = self.compile_expr(expr)?;

        for arm in arms {
            if matches!(arm.pattern, Pattern::Wildcard | Pattern::Ident(_)) {
                // Execute body and return last expression
                for (i, stmt) in arm.body.iter().enumerate() {
                    if i == arm.body.len() - 1 {
                        if let Stmt::Expr(e) = &stmt.node {
                            return self.compile_expr(&e.node);
                        }
                    }
                    self.compile_stmt(&stmt.node)?;
                }
            }
        }

        Ok(self.context.i64_type().const_int(0, false).into())
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

                let class_name = match &object.node {
                    Expr::Ident(name) => self.variables.get(name).and_then(|v| match &v.ty {
                        Type::Named(n) => Some(n.clone()),
                        _ => None,
                    }),
                    Expr::This => self.variables.get("this").and_then(|v| match &v.ty {
                        Type::Named(n) => Some(n.clone()),
                        _ => None,
                    }),
                    _ => None,
                }
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

    pub fn write_object(&self, path: &Path) -> std::result::Result<(), String> {
        Target::initialize_native(&InitializationConfig::default())
            .map_err(|e| format!("Failed to init target: {}", e))?;

        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple).map_err(|e| e.to_string())?;

        let machine = target
            .create_target_machine(
                &triple,
                "native",
                "+avx2,+fma",
                OptimizationLevel::Aggressive,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or("Failed to create target machine")?;

        machine
            .write_to_file(&self.module, FileType::Object, path)
            .map_err(|e| e.to_string())
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
            Expr::Call { callee, args } => {
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
            Expr::Field { object, .. } => {
                let _obj_ty = self.infer_expr_type(&object.node, params);
                Type::Integer
            }
            Expr::Lambda { params, body } => {
                let ret_ty = self.infer_expr_type(&body.node, params);
                Type::Function(
                    params.iter().map(|p| p.ty.clone()).collect(),
                    Box::new(ret_ty),
                )
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

    /// Get or create the Range struct type: { i64 start, i64 end, i64 step, i64 current }
    pub fn get_range_type(&self) -> StructType<'ctx> {
        let range_name = "Range";
        if let Some(s) = self.module.get_struct_type(range_name) {
            return s;
        }
        let range_type = self.context.struct_type(
            &[
                self.context.i64_type().into(), // start
                self.context.i64_type().into(), // end
                self.context.i64_type().into(), // step
                self.context.i64_type().into(), // current
            ],
            false,
        );
        range_type.set_body(
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.i64_type().into(),
            ],
            false,
        );
        range_type
    }

    /// Create a new Range instance
    pub fn create_range(
        &mut self,
        start: BasicValueEnum<'ctx>,
        end: BasicValueEnum<'ctx>,
        step: BasicValueEnum<'ctx>,
    ) -> Result<PointerValue<'ctx>> {
        let range_type = self.get_range_type();
        let malloc = self.get_or_declare_malloc();

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
        let range_type = self.get_range_type();
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
        let range_type = self.get_range_type();
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
