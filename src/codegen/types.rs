//! Type-specific codegen helpers for collections, Option, and Result types
#![allow(dead_code)]

use crate::ast::{Expr, Spanned, Type};
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, PointerValue, ValueKind};
use inkwell::{AddressSpace, IntPredicate};

use crate::codegen::core::{Codegen, CodegenError, Result};

impl<'ctx> Codegen<'ctx> {
    // === Set<T> methods ===

    pub fn compile_set_method(
        &mut self,
        set_name: &str,
        method: &str,
        _args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let var = self.variables.get(set_name).unwrap();
        let set_ptr = var.ptr;
        let set_type = self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );

        match method {
            "length" => {
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            set_type.as_basic_type_enum(),
                            set_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .unwrap()
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .unwrap();
                Ok(length)
            }
            "add" | "contains" | "remove" => {
                // Stubs for now
                Ok(self.context.bool_type().const_int(0, false).into())
            }
            _ => Err(CodegenError::new(format!("Unknown Set method: {}", method))),
        }
    }

    // === Option<T> methods ===

    pub fn compile_option_method(
        &mut self,
        option_name: &str,
        method: &str,
        _args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let var = self.variables.get(option_name).unwrap();
        let option_ptr = var.ptr;
        // Assuming Option<T> is { is_some: i8, value: T }
        // We need to infer T from var.ty
        let option_ty = match &var.ty {
            Type::Option(inner_ty) => inner_ty,
            _ => return Err(CodegenError::new("Expected Option type")),
        };
        let llvm_inner_ty = self.llvm_type(option_ty);

        let option_struct_type = self
            .context
            .struct_type(&[self.context.i8_type().into(), llvm_inner_ty], false);

        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);

        match method {
            "is_some" => {
                let is_some_ptr = unsafe {
                    self.builder
                        .build_gep(
                            option_struct_type.as_basic_type_enum(),
                            option_ptr,
                            &[zero, i32_type.const_int(0, false)],
                            "is_some_ptr",
                        )
                        .unwrap()
                };
                let is_some = self
                    .builder
                    .build_load(self.context.i8_type(), is_some_ptr, "is_some")
                    .unwrap();
                Ok(is_some)
            }
            "is_none" => {
                let is_some_ptr = unsafe {
                    self.builder
                        .build_gep(
                            option_struct_type.as_basic_type_enum(),
                            option_ptr,
                            &[zero, i32_type.const_int(0, false)],
                            "is_some_ptr",
                        )
                        .unwrap()
                };
                let is_some = self
                    .builder
                    .build_load(self.context.i8_type(), is_some_ptr, "is_some")
                    .unwrap()
                    .into_int_value();
                let is_none = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        is_some,
                        self.context.i8_type().const_int(0, false),
                        "is_none",
                    )
                    .unwrap();
                Ok(is_none.into())
            }
            "unwrap" => {
                // For now, just return the value without checking if it's Some
                // A proper implementation would add a runtime check and panic if None
                let value_ptr = unsafe {
                    self.builder
                        .build_gep(
                            option_struct_type.as_basic_type_enum(),
                            option_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "value_ptr",
                        )
                        .unwrap()
                };
                let value = self
                    .builder
                    .build_load(llvm_inner_ty, value_ptr, "unwrapped_value")
                    .unwrap();
                Ok(value)
            }
            _ => Err(CodegenError::new(format!(
                "Unknown Option method: {}",
                method
            ))),
        }
    }

    // === Result<T, E> methods ===

    pub fn compile_result_method(
        &mut self,
        result_name: &str,
        method: &str,
        _args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let var = self.variables.get(result_name).unwrap();
        let result_ptr = var.ptr;
        // Result<T, E> is struct { is_ok: i8, ok_value: T, err_value: E }
        let (ok_ty, err_ty) = match &var.ty {
            Type::Result(ok, err) => (ok, err),
            _ => return Err(CodegenError::new("Expected Result type")),
        };
        let ok_llvm = self.llvm_type(ok_ty);
        let err_llvm = self.llvm_type(err_ty);

        let result_struct_type = self
            .context
            .struct_type(&[self.context.i8_type().into(), ok_llvm, err_llvm], false);

        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);

        match method {
            "is_ok" => {
                let tag_ptr = unsafe {
                    self.builder
                        .build_gep(
                            result_struct_type.as_basic_type_enum(),
                            result_ptr,
                            &[zero, i32_type.const_int(0, false)],
                            "tag_ptr",
                        )
                        .unwrap()
                };
                let tag = self
                    .builder
                    .build_load(self.context.i8_type(), tag_ptr, "tag")
                    .unwrap();
                Ok(tag)
            }
            "is_error" => {
                let tag_ptr = unsafe {
                    self.builder
                        .build_gep(
                            result_struct_type.as_basic_type_enum(),
                            result_ptr,
                            &[zero, i32_type.const_int(0, false)],
                            "tag_ptr",
                        )
                        .unwrap()
                };
                let tag = self
                    .builder
                    .build_load(self.context.i8_type(), tag_ptr, "tag")
                    .unwrap()
                    .into_int_value();
                let is_error = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        tag,
                        self.context.i8_type().const_int(0, false),
                        "is_error",
                    )
                    .unwrap();
                Ok(is_error.into())
            }
            "unwrap" => {
                // Returns Ok value, panics if Error (runtime check omitted for now)
                let ok_ptr = unsafe {
                    self.builder
                        .build_gep(
                            result_struct_type.as_basic_type_enum(),
                            result_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "ok_ptr",
                        )
                        .unwrap()
                };
                let value = self
                    .builder
                    .build_load(ok_llvm, ok_ptr, "unwrapped_ok")
                    .unwrap();
                Ok(value)
            }
            _ => Err(CodegenError::new(format!(
                "Unknown Result method: {}",
                method
            ))),
        }
    }

    // === Option<T> helpers ===

    pub fn create_option_some(
        &mut self,
        value: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        // Option is struct { is_some: i8, value: T }
        let option_type = self
            .context
            .struct_type(&[self.context.i8_type().into(), value.get_type()], false);

        let alloca = self.builder.build_alloca(option_type, "option").unwrap();

        // Set is_some = 1
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
            .build_store(tag_ptr, self.context.i8_type().const_int(1, false))
            .unwrap();

        // Set value
        let value_ptr = unsafe {
            self.builder
                .build_gep(
                    option_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "value",
                )
                .unwrap()
        };
        self.builder.build_store(value_ptr, value).unwrap();

        Ok(self
            .builder
            .build_load(option_type, alloca, "option")
            .unwrap())
    }

    pub fn create_option_none(&mut self) -> Result<BasicValueEnum<'ctx>> {
        // Option<i64> as default - struct { is_some: i8, value: i64 }
        let option_type = self.context.struct_type(
            &[
                self.context.i8_type().into(),
                self.context.i64_type().into(),
            ],
            false,
        );

        let alloca = self.builder.build_alloca(option_type, "option").unwrap();

        // Set is_some = 0
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

        // Set value to 0 (unused)
        let value_ptr = unsafe {
            self.builder
                .build_gep(
                    option_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "value",
                )
                .unwrap()
        };
        self.builder
            .build_store(value_ptr, self.context.i64_type().const_int(0, false))
            .unwrap();

        Ok(self
            .builder
            .build_load(option_type, alloca, "option")
            .unwrap())
    }

    // === Result<T, E> helpers ===

    pub fn create_result_ok(
        &mut self,
        value: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        // Result is struct { is_ok: i8, ok_value: T, err_value: ptr }
        let result_type = self.context.struct_type(
            &[
                self.context.i8_type().into(),
                value.get_type(),
                self.context.ptr_type(AddressSpace::default()).into(), // error as string ptr
            ],
            false,
        );

        let alloca = self.builder.build_alloca(result_type, "result").unwrap();

        // Set is_ok = 1
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
            .build_store(tag_ptr, self.context.i8_type().const_int(1, false))
            .unwrap();

        // Set ok_value
        let ok_ptr = unsafe {
            self.builder
                .build_gep(
                    result_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "ok",
                )
                .unwrap()
        };
        self.builder.build_store(ok_ptr, value).unwrap();

        // Set err_value to null
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
        let null = self.context.ptr_type(AddressSpace::default()).const_null();
        self.builder.build_store(err_ptr, null).unwrap();

        Ok(self
            .builder
            .build_load(result_type, alloca, "result")
            .unwrap())
    }

    pub fn create_result_error(
        &mut self,
        error: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        // Result is struct { is_ok: i8, ok_value: i64, err_value: ptr }
        let result_type = self.context.struct_type(
            &[
                self.context.i8_type().into(),
                self.context.i64_type().into(), // default ok type
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );

        let alloca = self.builder.build_alloca(result_type, "result").unwrap();

        // Set is_ok = 0
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

        // Set ok_value to 0
        let ok_ptr = unsafe {
            self.builder
                .build_gep(
                    result_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "ok",
                )
                .unwrap()
        };
        self.builder
            .build_store(ok_ptr, self.context.i64_type().const_int(0, false))
            .unwrap();

        // Set err_value
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
        self.builder.build_store(err_ptr, error).unwrap();

        Ok(self
            .builder
            .build_load(result_type, alloca, "result")
            .unwrap())
    }

    pub fn create_default_result(&mut self) -> Result<BasicValueEnum<'ctx>> {
        // Result is struct { is_ok: i8, ok_value: i64, err_value: ptr }
        // We default to Error (tag=0) with null pointer
        let result_type = self.context.struct_type(
            &[
                self.context.i8_type().into(),
                self.context.i64_type().into(), // default ok type
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );

        let alloca = self
            .builder
            .build_alloca(result_type, "default_result")
            .unwrap();

        // Set is_ok = 0
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

        // Set ok_value to 0
        let ok_ptr = unsafe {
            self.builder
                .build_gep(
                    result_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "ok",
                )
                .unwrap()
        };
        self.builder
            .build_store(ok_ptr, self.context.i64_type().const_int(0, false))
            .unwrap();

        // Set err_value to null
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
        let null = self.context.ptr_type(AddressSpace::default()).const_null();
        self.builder.build_store(err_ptr, null).unwrap();

        Ok(self
            .builder
            .build_load(result_type, alloca, "result")
            .unwrap())
    }

    // === List<T> helpers ===

    pub fn create_empty_list(&mut self) -> Result<BasicValueEnum<'ctx>> {
        // List struct: { capacity: i64, length: i64, data: ptr }
        let list_type = self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );

        let alloca = self.builder.build_alloca(list_type, "list").unwrap();

        // Initial capacity = 8
        let initial_capacity: u64 = 8;
        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);
        let capacity_ptr = unsafe {
            self.builder
                .build_gep(
                    list_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(0, false)],
                    "capacity",
                )
                .unwrap()
        };
        self.builder
            .build_store(
                capacity_ptr,
                self.context.i64_type().const_int(initial_capacity, false),
            )
            .unwrap();

        // Length = 0
        let length_ptr = unsafe {
            self.builder
                .build_gep(
                    list_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "length",
                )
                .unwrap()
        };
        self.builder
            .build_store(length_ptr, self.context.i64_type().const_int(0, false))
            .unwrap();

        // Allocate data - malloc(capacity * 8) for i64 elements
        let malloc = self.get_or_declare_malloc();
        let size = self
            .context
            .i64_type()
            .const_int(initial_capacity * 8, false);
        let call_result = self
            .builder
            .build_call(malloc, &[size.into()], "data")
            .unwrap();
        let data_ptr = match call_result.try_as_basic_value() {
            ValueKind::Basic(val) => val,
            _ => panic!("malloc should return a value"),
        };

        let data_ptr_field = unsafe {
            self.builder
                .build_gep(
                    list_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(2, false)],
                    "data_ptr",
                )
                .unwrap()
        };
        self.builder.build_store(data_ptr_field, data_ptr).unwrap();

        Ok(self.builder.build_load(list_type, alloca, "list").unwrap())
    }

    // === Map<K,V> helpers ===

    pub fn create_empty_map(&mut self) -> Result<BasicValueEnum<'ctx>> {
        // Map struct: { capacity: i64, length: i64, keys: ptr, values: ptr }
        let map_type = self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );

        let alloca = self.builder.build_alloca(map_type, "map").unwrap();

        // Initial capacity = 8
        let initial_capacity: u64 = 8;
        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);
        let capacity_ptr = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(0, false)],
                    "capacity",
                )
                .unwrap()
        };
        self.builder
            .build_store(
                capacity_ptr,
                self.context.i64_type().const_int(initial_capacity, false),
            )
            .unwrap();

        // Length = 0
        let length_ptr = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "length",
                )
                .unwrap()
        };
        self.builder
            .build_store(length_ptr, self.context.i64_type().const_int(0, false))
            .unwrap();

        // Allocate keys and values arrays
        let malloc = self.get_or_declare_malloc();
        let size = self
            .context
            .i64_type()
            .const_int(initial_capacity * 8, false);

        let keys_call = self
            .builder
            .build_call(malloc, &[size.into()], "keys")
            .unwrap();
        let keys_ptr = match keys_call.try_as_basic_value() {
            ValueKind::Basic(val) => val,
            _ => panic!("malloc should return a value"),
        };
        let keys_field = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(2, false)],
                    "keys_ptr",
                )
                .unwrap()
        };
        self.builder.build_store(keys_field, keys_ptr).unwrap();

        let values_call = self
            .builder
            .build_call(malloc, &[size.into()], "values")
            .unwrap();
        let values_ptr = match values_call.try_as_basic_value() {
            ValueKind::Basic(val) => val,
            _ => panic!("malloc should return a value"),
        };
        let values_field = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(3, false)],
                    "values_ptr",
                )
                .unwrap()
        };
        self.builder.build_store(values_field, values_ptr).unwrap();

        Ok(self.builder.build_load(map_type, alloca, "map").unwrap())
    }

    pub fn create_empty_set(&mut self) -> Result<BasicValueEnum<'ctx>> {
        // Set struct: { capacity: i64, length: i64, data: ptr }
        let set_type = self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );

        let alloca = self.builder.build_alloca(set_type, "set").unwrap();

        // Initial capacity = 8
        let initial_capacity: u64 = 8;
        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);
        let capacity_ptr = unsafe {
            self.builder
                .build_gep(
                    set_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(0, false)],
                    "capacity",
                )
                .unwrap()
        };
        self.builder
            .build_store(
                capacity_ptr,
                self.context.i64_type().const_int(initial_capacity, false),
            )
            .unwrap();

        // Length = 0
        let length_ptr = unsafe {
            self.builder
                .build_gep(
                    set_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "length",
                )
                .unwrap()
        };
        self.builder
            .build_store(length_ptr, self.context.i64_type().const_int(0, false))
            .unwrap();

        // Allocate data - malloc(capacity * 8)
        let malloc = self.get_or_declare_malloc();
        let size = self
            .context
            .i64_type()
            .const_int(initial_capacity * 8, false);
        let call_result = self
            .builder
            .build_call(malloc, &[size.into()], "data")
            .unwrap();
        let data_ptr = match call_result.try_as_basic_value() {
            ValueKind::Basic(val) => val,
            _ => panic!("malloc should return a value"),
        };

        let data_ptr_field = unsafe {
            self.builder
                .build_gep(
                    set_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(2, false)],
                    "data_ptr",
                )
                .unwrap()
        };
        self.builder.build_store(data_ptr_field, data_ptr).unwrap();

        Ok(self.builder.build_load(set_type, alloca, "set").unwrap())
    }

    pub fn create_empty_box(&mut self) -> Result<BasicValueEnum<'ctx>> {
        let malloc = self.get_or_declare_malloc();
        let size = self.context.i64_type().const_int(8, false);
        let call_result = self
            .builder
            .build_call(malloc, &[size.into()], "box")
            .unwrap();
        match call_result.try_as_basic_value() {
            ValueKind::Basic(val) => Ok(val),
            _ => panic!("malloc should return a value"),
        }
    }

    pub fn create_empty_rc(&mut self) -> Result<BasicValueEnum<'ctx>> {
        let malloc = self.get_or_declare_malloc();
        let size = self.context.i64_type().const_int(16, false); // refcount + data
        let call_result = self
            .builder
            .build_call(malloc, &[size.into()], "rc")
            .unwrap();
        match call_result.try_as_basic_value() {
            ValueKind::Basic(val) => Ok(val),
            _ => panic!("malloc should return a value"),
        }
    }

    pub fn create_empty_arc(&mut self) -> Result<BasicValueEnum<'ctx>> {
        let malloc = self.get_or_declare_malloc();
        let size = self.context.i64_type().const_int(16, false); // atomic refcount + data
        let call_result = self
            .builder
            .build_call(malloc, &[size.into()], "arc")
            .unwrap();
        match call_result.try_as_basic_value() {
            ValueKind::Basic(val) => Ok(val),
            _ => panic!("malloc should return a value"),
        }
    }

    pub fn compile_list_method(
        &mut self,
        list_name: &str,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let var = self.variables.get(list_name).unwrap();
        let list_ptr = var.ptr;
        let list_type = self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );

        match method {
            "push" => {
                // Get current length and capacity
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .unwrap()
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .unwrap()
                    .into_int_value();

                // Get data pointer
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .unwrap()
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .unwrap()
                    .into_pointer_value();

                // Calculate element pointer: data + length * 8
                let offset = self
                    .builder
                    .build_int_mul(
                        length,
                        self.context.i64_type().const_int(8, false),
                        "offset",
                    )
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };

                // Store the value
                let value = self.compile_expr(&args[0].node)?;
                self.builder.build_store(elem_ptr, value).unwrap();

                // Increment length
                let new_length = self
                    .builder
                    .build_int_add(
                        length,
                        self.context.i64_type().const_int(1, false),
                        "new_len",
                    )
                    .unwrap();
                self.builder.build_store(length_ptr, new_length).unwrap();

                Ok(self.context.i8_type().const_int(0, false).into())
            }
            "get" => {
                // Get data pointer
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .unwrap()
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .unwrap()
                    .into_pointer_value();

                // Calculate element pointer: data + index * 8
                let index = self.compile_expr(&args[0].node)?.into_int_value();
                let offset = self
                    .builder
                    .build_int_mul(index, self.context.i64_type().const_int(8, false), "offset")
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };

                // Load and return the value
                let val = self
                    .builder
                    .build_load(self.context.i64_type(), elem_ptr, "val")
                    .unwrap();
                Ok(val)
            }
            "length" => {
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .unwrap()
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .unwrap();
                Ok(length)
            }
            "pop" => {
                // Get current length
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .unwrap()
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .unwrap()
                    .into_int_value();

                // new_length = length - 1
                let new_length = self
                    .builder
                    .build_int_sub(
                        length,
                        self.context.i64_type().const_int(1, false),
                        "new_len",
                    )
                    .unwrap();

                // Update length
                self.builder.build_store(length_ptr, new_length).unwrap();

                // Get data pointer
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .unwrap()
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .unwrap()
                    .into_pointer_value();

                // Get value at new_length (the old last element)
                let offset = self
                    .builder
                    .build_int_mul(
                        new_length,
                        self.context.i64_type().const_int(8, false),
                        "offset",
                    )
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };

                let val = self
                    .builder
                    .build_load(self.context.i64_type(), elem_ptr, "val")
                    .unwrap();
                Ok(val)
            }
            "set" => {
                // Get data pointer
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .unwrap()
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .unwrap()
                    .into_pointer_value();

                // Calculate element pointer
                let index = self.compile_expr(&args[0].node)?.into_int_value();
                let offset = self
                    .builder
                    .build_int_mul(index, self.context.i64_type().const_int(8, false), "offset")
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };

                // Store the value
                let value = self.compile_expr(&args[1].node)?;
                self.builder.build_store(elem_ptr, value).unwrap();

                Ok(self.context.i8_type().const_int(0, false).into())
            }
            _ => Err(CodegenError::new(format!(
                "Unknown List method: {}",
                method
            ))),
        }
    }

    /// Compile List method call with pointer (for non-identifier expressions like this.items)
    pub fn compile_list_method_ptr(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        _list_ty: &Type,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let list_type = self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );
        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);

        match method {
            "push" => {
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .unwrap()
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .unwrap()
                    .into_int_value();

                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .unwrap()
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .unwrap()
                    .into_pointer_value();

                let offset = self
                    .builder
                    .build_int_mul(
                        length,
                        self.context.i64_type().const_int(8, false),
                        "offset",
                    )
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };

                let value = self.compile_expr(&args[0].node)?;
                self.builder.build_store(elem_ptr, value).unwrap();

                let new_length = self
                    .builder
                    .build_int_add(
                        length,
                        self.context.i64_type().const_int(1, false),
                        "new_len",
                    )
                    .unwrap();
                self.builder.build_store(length_ptr, new_length).unwrap();

                Ok(self.context.i8_type().const_int(0, false).into())
            }
            "length" => {
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .unwrap()
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .unwrap();
                Ok(length)
            }
            "get" => {
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .unwrap()
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .unwrap()
                    .into_pointer_value();

                let index = self.compile_expr(&args[0].node)?.into_int_value();
                let offset = self
                    .builder
                    .build_int_mul(index, self.context.i64_type().const_int(8, false), "offset")
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };

                let val = self
                    .builder
                    .build_load(self.context.i64_type(), elem_ptr, "val")
                    .unwrap();
                Ok(val)
            }
            "set" => {
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .unwrap()
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .unwrap()
                    .into_pointer_value();

                let index = self.compile_expr(&args[0].node)?.into_int_value();
                let offset = self
                    .builder
                    .build_int_mul(index, self.context.i64_type().const_int(8, false), "offset")
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };

                // Store the value
                let value = self.compile_expr(&args[1].node)?;
                self.builder.build_store(elem_ptr, value).unwrap();

                Ok(self.context.i8_type().const_int(0, false).into())
            }
            "pop" => {
                // Get current length
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .unwrap()
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .unwrap()
                    .into_int_value();

                // new_length = length - 1
                let new_length = self
                    .builder
                    .build_int_sub(
                        length,
                        self.context.i64_type().const_int(1, false),
                        "new_len",
                    )
                    .unwrap();

                // Update length
                self.builder.build_store(length_ptr, new_length).unwrap();

                // Get data pointer
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .unwrap()
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .unwrap()
                    .into_pointer_value();

                // Get value at new_length (the old last element)
                let offset = self
                    .builder
                    .build_int_mul(
                        new_length,
                        self.context.i64_type().const_int(8, false),
                        "offset",
                    )
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };

                let val = self
                    .builder
                    .build_load(self.context.i64_type(), elem_ptr, "val")
                    .unwrap();
                Ok(val)
            }
            _ => Err(CodegenError::new(format!(
                "Unknown List method: {}",
                method
            ))),
        }
    }

    pub fn compile_map_method(
        &mut self,
        map_name: &str,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let var = self.variables.get(map_name).unwrap();
        let map_ptr = var.ptr;
        let map_type = self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );

        match method {
            "length" => {
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            map_type.as_basic_type_enum(),
                            map_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .unwrap()
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .unwrap();
                Ok(length)
            }
            "set" => {
                // Simple implementation: append key-value at end (no duplicate check)
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            map_type.as_basic_type_enum(),
                            map_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .unwrap()
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .unwrap()
                    .into_int_value();

                let keys_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            map_type.as_basic_type_enum(),
                            map_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "keys_ptr_ptr",
                        )
                        .unwrap()
                };
                let keys_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        keys_ptr_ptr,
                        "keys",
                    )
                    .unwrap()
                    .into_pointer_value();

                let values_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            map_type.as_basic_type_enum(),
                            map_ptr,
                            &[zero, i32_type.const_int(3, false)],
                            "vals_ptr_ptr",
                        )
                        .unwrap()
                };
                let values_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        values_ptr_ptr,
                        "vals",
                    )
                    .unwrap()
                    .into_pointer_value();

                let offset = self
                    .builder
                    .build_int_mul(
                        length,
                        self.context.i64_type().const_int(8, false),
                        "offset",
                    )
                    .unwrap();

                // Store key
                let key_elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), keys_ptr, &[offset], "key_ptr")
                        .unwrap()
                };
                let key = self.compile_expr(&args[0].node)?;
                self.builder.build_store(key_elem_ptr, key).unwrap();

                // Store value
                let val_elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), values_ptr, &[offset], "val_ptr")
                        .unwrap()
                };
                let value = self.compile_expr(&args[1].node)?;
                self.builder.build_store(val_elem_ptr, value).unwrap();

                // Increment length
                let new_length = self
                    .builder
                    .build_int_add(
                        length,
                        self.context.i64_type().const_int(1, false),
                        "new_len",
                    )
                    .unwrap();
                self.builder.build_store(length_ptr, new_length).unwrap();

                Ok(self.context.i8_type().const_int(0, false).into())
            }
            "insert" => {
                // Alias for set
                self.compile_map_method(map_name, "set", args)
            }
            "get" => {
                // Simple linear search
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_int(0, false);
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            map_type.as_basic_type_enum(),
                            map_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .unwrap()
                };
                let _length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .unwrap();

                let keys_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            map_type.as_basic_type_enum(),
                            map_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "keys_ptr_ptr",
                        )
                        .unwrap()
                };
                let _keys_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        keys_ptr_ptr,
                        "keys",
                    )
                    .unwrap();

                let values_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            map_type.as_basic_type_enum(),
                            map_ptr,
                            &[zero, i32_type.const_int(3, false)],
                            "vals_ptr_ptr",
                        )
                        .unwrap()
                };
                let _values_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        values_ptr_ptr,
                        "vals",
                    )
                    .unwrap();

                // For now, just return 0 - proper implementation would need loops
                // TODO: Implement proper key search
                Ok(self.context.i64_type().const_int(0, false).into())
            }
            "contains" => {
                // For now, just return false
                Ok(self.context.bool_type().const_int(0, false).into())
            }
            _ => Err(CodegenError::new(format!("Unknown Map method: {}", method))),
        }
    }

    /// Compile range method calls
    pub fn compile_range_method(
        &mut self,
        range_name: &str,
        method: &str,
        _args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let var = self.variables.get(range_name).unwrap();
        let var_ptr = var.ptr;
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        
        // Load the range pointer from the variable alloca
        let range_ptr = self.builder
            .build_load(ptr_type, var_ptr, "range_ptr")
            .unwrap()
            .into_pointer_value();
        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);
        let one = i32_type.const_int(1, false);
        let two = i32_type.const_int(2, false);
        let three = i32_type.const_int(3, false);
        
        // Get range struct type: { i64, i64, i64, i64 }
        let range_type = self.context.struct_type(
            &[
                i64_type.into(),
                i64_type.into(),
                i64_type.into(),
                i64_type.into(),
            ],
            false,
        );
        
        // Range struct layout: { start: i64, end: i64, step: i64, current: i64 }
        match method {
            "has_next" => {
                // Load step
                let step_ptr = unsafe {
                    self.builder
                        .build_gep(
                            range_type,
                            range_ptr,
                            &[zero, two],
                            "step_ptr",
                        )
                        .unwrap()
                };
                let step = self
                    .builder
                    .build_load(i64_type, step_ptr, "step")
                    .unwrap()
                    .into_int_value();
                
                // Load current
                let current_ptr = unsafe {
                    self.builder
                        .build_gep(
                            range_type,
                            range_ptr,
                            &[zero, three],
                            "current_ptr",
                        )
                        .unwrap()
                };
                let current = self
                    .builder
                    .build_load(i64_type, current_ptr, "current")
                    .unwrap()
                    .into_int_value();
                
                // Load end
                let end_ptr = unsafe {
                    self.builder
                        .build_gep(
                            range_type,
                            range_ptr,
                            &[zero, one],
                            "end_ptr",
                        )
                        .unwrap()
                };
                let end = self
                    .builder
                    .build_load(i64_type, end_ptr, "end")
                    .unwrap()
                    .into_int_value();
                
                // Check if step > 0: current < end
                // Check if step < 0: current > end
                let zero_i64 = i64_type.const_int(0, false);
                let step_positive = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::SGT,
                        step,
                        zero_i64,
                        "step_positive",
                    )
                    .unwrap();
                
                let current_lt_end = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::SLT,
                        current,
                        end,
                        "current_lt_end",
                    )
                    .unwrap();
                
                let current_gt_end = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::SGT,
                        current,
                        end,
                        "current_gt_end",
                    )
                    .unwrap();
                
                // Select based on step direction
                let result = self
                    .builder
                    .build_select(step_positive, current_lt_end, current_gt_end, "has_next")
                    .unwrap();
                
                Ok(result.into_int_value().into())
            }
            "next" => {
                // Load current
                let current_ptr = unsafe {
                    self.builder
                        .build_gep(
                            range_type,
                            range_ptr,
                            &[zero, three],
                            "current_ptr",
                        )
                        .unwrap()
                };
                let current = self
                    .builder
                    .build_load(i64_type, current_ptr, "current")
                    .unwrap()
                    .into_int_value();
                
                // Load step
                let step_ptr = unsafe {
                    self.builder
                        .build_gep(
                            range_type,
                            range_ptr,
                            &[zero, two],
                            "step_ptr",
                        )
                        .unwrap()
                };
                let step = self
                    .builder
                    .build_load(i64_type, step_ptr, "step")
                    .unwrap()
                    .into_int_value();
                
                // Increment current: current + step
                let new_current = self
                    .builder
                    .build_int_add(current, step, "new_current")
                    .unwrap();
                
                // Store new current
                self.builder.build_store(current_ptr, new_current).unwrap();
                
                // Return old current
                Ok(current.into())
            }
            _ => Err(CodegenError::new(format!("Unknown Range method: {}", method))),
        }
    }
}
