//! Type-specific codegen helpers for collections, Option, and Result types
#![allow(dead_code)]

use crate::ast::{Expr, Spanned, Type};
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue, ValueKind};
use inkwell::{AddressSpace, IntPredicate};

use crate::codegen::core::{Codegen, CodegenError, Result};

impl<'ctx> Codegen<'ctx> {
    pub(crate) fn materialize_value_pointer_for_type(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        name: &str,
    ) -> Result<PointerValue<'ctx>> {
        if let BasicValueEnum::PointerValue(ptr) = value {
            return Ok(ptr);
        }
        let alloca = self
            .builder
            .build_alloca(self.llvm_type(ty), name)
            .map_err(|_| CodegenError::new("failed to allocate temporary value storage"))?;
        self.builder
            .build_store(alloca, value)
            .map_err(|_| CodegenError::new("failed to store temporary value"))?;
        Ok(alloca)
    }

    pub(crate) fn list_element_layout_from_list_type(
        &self,
        list_ty: &Type,
    ) -> (inkwell::types::BasicTypeEnum<'ctx>, u64) {
        if let Type::List(inner) = list_ty {
            let elem_llvm_ty = self.llvm_type(inner);
            let elem_size = elem_llvm_ty
                .size_of()
                .and_then(|size| size.get_zero_extended_constant())
                .unwrap_or(8);
            return (elem_llvm_ty, elem_size);
        }
        (self.context.i64_type().into(), 8)
    }

    pub(crate) fn list_element_layout_default(&self) -> (inkwell::types::BasicTypeEnum<'ctx>, u64) {
        (self.context.i64_type().into(), 8)
    }

    // === Set<T> methods ===

    pub fn compile_set_method(
        &mut self,
        set_name: &str,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let (set_ptr, set_ty) = {
            let var = self.variables.get(set_name).unwrap();
            (var.ptr, var.ty.clone())
        };
        self.compile_set_method_on_value(set_ptr.into(), &set_ty, method, args)
    }

    pub fn compile_set_method_on_value(
        &mut self,
        set_value: BasicValueEnum<'ctx>,
        set_ty: &Type,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let set_ptr = self.materialize_value_pointer_for_type(set_value, set_ty, "set_tmp")?;
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
                let inner_ty = match set_ty {
                    Type::Set(inner) => &**inner,
                    _ => return Err(CodegenError::new("Expected Set type")),
                };
                let elem_llvm_ty = self.llvm_type(inner_ty);
                let elem_size = elem_llvm_ty
                    .size_of()
                    .and_then(|size| size.get_zero_extended_constant())
                    .unwrap_or(8);
                let i32_type = self.context.i32_type();
                let i64_type = self.context.i64_type();
                let zero = i32_type.const_int(0, false);
                let capacity_ptr = unsafe {
                    self.builder
                        .build_gep(
                            set_type.as_basic_type_enum(),
                            set_ptr,
                            &[zero, i32_type.const_int(0, false)],
                            "set_capacity_ptr",
                        )
                        .unwrap()
                };
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            set_type.as_basic_type_enum(),
                            set_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "set_length_ptr",
                        )
                        .unwrap()
                };
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            set_type.as_basic_type_enum(),
                            set_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "set_data_ptr_ptr",
                        )
                        .unwrap()
                };
                let needle = self.compile_expr(&args[0].node)?;
                let length = self
                    .builder
                    .build_load(i64_type, length_ptr, "set_len")
                    .unwrap()
                    .into_int_value();
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "set_data_ptr",
                    )
                    .unwrap()
                    .into_pointer_value();
                let idx_ptr = self.builder.build_alloca(i64_type, "set_idx").unwrap();
                self.builder
                    .build_store(idx_ptr, i64_type.const_zero())
                    .unwrap();
                let found_ptr = self
                    .builder
                    .build_alloca(i64_type, "set_found_idx")
                    .unwrap();
                self.builder
                    .build_store(found_ptr, i64_type.const_all_ones())
                    .unwrap();

                let current_fn = self.current_function.unwrap();
                let cond_bb = self
                    .context
                    .append_basic_block(current_fn, "set_search.cond");
                let body_bb = self
                    .context
                    .append_basic_block(current_fn, "set_search.body");
                let found_bb = self
                    .context
                    .append_basic_block(current_fn, "set_search.found");
                let done_bb = self
                    .context
                    .append_basic_block(current_fn, "set_search.done");

                self.builder.build_unconditional_branch(cond_bb).unwrap();
                self.builder.position_at_end(cond_bb);
                let idx = self
                    .builder
                    .build_load(i64_type, idx_ptr, "set_idx_val")
                    .unwrap()
                    .into_int_value();
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::ULT, idx, length, "set_idx_in_bounds")
                    .unwrap();
                self.builder
                    .build_conditional_branch(in_bounds, body_bb, done_bb)
                    .unwrap();

                self.builder.position_at_end(body_bb);
                let offset = self
                    .builder
                    .build_int_mul(idx, i64_type.const_int(elem_size, false), "set_offset")
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "set_elem_ptr")
                        .unwrap()
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "set_typed_elem_ptr",
                    )
                    .unwrap();
                let existing = self
                    .builder
                    .build_load(elem_llvm_ty, typed_elem_ptr, "set_existing")
                    .unwrap();
                let eq = if matches!(inner_ty, Type::String) {
                    let strcmp = self.get_or_declare_strcmp();
                    let cmp = self
                        .builder
                        .build_call(strcmp, &[existing.into(), needle.into()], "set_strcmp")
                        .unwrap();
                    let cmp_v = match cmp.try_as_basic_value() {
                        ValueKind::Basic(v) => v.into_int_value(),
                        _ => self.context.i32_type().const_int(1, false),
                    };
                    self.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            cmp_v,
                            self.context.i32_type().const_zero(),
                            "set_eq",
                        )
                        .unwrap()
                } else if existing.is_int_value() && needle.is_int_value() {
                    self.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            existing.into_int_value(),
                            needle.into_int_value(),
                            "set_eq",
                        )
                        .unwrap()
                } else if existing.is_float_value() && needle.is_float_value() {
                    self.builder
                        .build_float_compare(
                            inkwell::FloatPredicate::OEQ,
                            existing.into_float_value(),
                            needle.into_float_value(),
                            "set_eq",
                        )
                        .unwrap()
                } else {
                    self.context.bool_type().const_zero()
                };
                let next_bb = self
                    .context
                    .append_basic_block(current_fn, "set_search.next");
                self.builder
                    .build_conditional_branch(eq, found_bb, next_bb)
                    .unwrap();

                self.builder.position_at_end(found_bb);
                self.builder.build_store(found_ptr, idx).unwrap();
                self.builder.build_unconditional_branch(done_bb).unwrap();

                self.builder.position_at_end(next_bb);
                let next_idx = self
                    .builder
                    .build_int_add(idx, i64_type.const_int(1, false), "set_next_idx")
                    .unwrap();
                self.builder.build_store(idx_ptr, next_idx).unwrap();
                self.builder.build_unconditional_branch(cond_bb).unwrap();

                self.builder.position_at_end(done_bb);
                let found_idx = self
                    .builder
                    .build_load(i64_type, found_ptr, "set_found_idx_val")
                    .unwrap()
                    .into_int_value();
                let found = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        found_idx,
                        i64_type.const_all_ones(),
                        "set_found",
                    )
                    .unwrap();

                match method {
                    "contains" => Ok(found.into()),
                    "add" => {
                        let append_bb = self
                            .context
                            .append_basic_block(current_fn, "set_add.append");
                        let merge_bb = self.context.append_basic_block(current_fn, "set_add.merge");
                        self.builder
                            .build_conditional_branch(found, merge_bb, append_bb)
                            .unwrap();

                        self.builder.position_at_end(append_bb);
                        let capacity = self
                            .builder
                            .build_load(i64_type, capacity_ptr, "set_capacity")
                            .unwrap()
                            .into_int_value();
                        let need_growth = self
                            .builder
                            .build_int_compare(
                                IntPredicate::UGE,
                                length,
                                capacity,
                                "set_need_growth",
                            )
                            .unwrap();
                        let grow_bb = self.context.append_basic_block(current_fn, "set_add.grow");
                        let store_bb = self.context.append_basic_block(current_fn, "set_add.store");
                        self.builder
                            .build_conditional_branch(need_growth, grow_bb, store_bb)
                            .unwrap();

                        self.builder.position_at_end(grow_bb);
                        let realloc = self.get_or_declare_realloc();
                        let grown_capacity = self
                            .builder
                            .build_int_mul(
                                capacity,
                                i64_type.const_int(2, false),
                                "set_grown_capacity",
                            )
                            .unwrap();
                        let new_size = self
                            .builder
                            .build_int_mul(
                                grown_capacity,
                                i64_type.const_int(elem_size, false),
                                "set_new_size",
                            )
                            .unwrap();
                        let grown_ptr = self
                            .builder
                            .build_call(
                                realloc,
                                &[data_ptr.into(), new_size.into()],
                                "set_grown_ptr",
                            )
                            .unwrap()
                            .try_as_basic_value();
                        let grown_ptr = match grown_ptr {
                            ValueKind::Basic(BasicValueEnum::PointerValue(ptr)) => ptr,
                            _ => return Err(CodegenError::new("realloc failed for Set growth")),
                        };
                        self.builder.build_store(data_ptr_ptr, grown_ptr).unwrap();
                        self.builder
                            .build_store(capacity_ptr, grown_capacity)
                            .unwrap();
                        self.builder.build_unconditional_branch(store_bb).unwrap();

                        self.builder.position_at_end(store_bb);
                        let active_data_ptr = self
                            .builder
                            .build_load(
                                self.context.ptr_type(AddressSpace::default()),
                                data_ptr_ptr,
                                "set_active_data_ptr",
                            )
                            .unwrap()
                            .into_pointer_value();
                        let offset = self
                            .builder
                            .build_int_mul(
                                length,
                                i64_type.const_int(elem_size, false),
                                "set_append_offset",
                            )
                            .unwrap();
                        let elem_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    self.context.i8_type(),
                                    active_data_ptr,
                                    &[offset],
                                    "set_append_ptr",
                                )
                                .unwrap()
                        };
                        let typed_elem_ptr = self
                            .builder
                            .build_pointer_cast(
                                elem_ptr,
                                self.context.ptr_type(AddressSpace::default()),
                                "set_append_typed_ptr",
                            )
                            .unwrap();
                        self.builder.build_store(typed_elem_ptr, needle).unwrap();
                        let new_length = self
                            .builder
                            .build_int_add(length, i64_type.const_int(1, false), "set_new_length")
                            .unwrap();
                        self.builder.build_store(length_ptr, new_length).unwrap();
                        self.builder.build_unconditional_branch(merge_bb).unwrap();

                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(self.context.bool_type(), "set_add_phi")
                            .unwrap();
                        phi.add_incoming(&[
                            (&self.context.bool_type().const_zero(), done_bb),
                            (&self.context.bool_type().const_int(1, false), store_bb),
                        ]);
                        Ok(phi.as_basic_value())
                    }
                    "remove" => {
                        let remove_bb = self
                            .context
                            .append_basic_block(current_fn, "set_remove.body");
                        let merge_bb = self
                            .context
                            .append_basic_block(current_fn, "set_remove.merge");
                        self.builder
                            .build_conditional_branch(found, remove_bb, merge_bb)
                            .unwrap();

                        self.builder.position_at_end(remove_bb);
                        let last_idx = self
                            .builder
                            .build_int_sub(length, i64_type.const_int(1, false), "set_last_idx")
                            .unwrap();
                        let remove_is_last = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                found_idx,
                                last_idx,
                                "set_remove_is_last",
                            )
                            .unwrap();
                        let shift_bb = self
                            .context
                            .append_basic_block(current_fn, "set_remove.shift");
                        let shrink_bb = self
                            .context
                            .append_basic_block(current_fn, "set_remove.shrink");
                        self.builder
                            .build_conditional_branch(remove_is_last, shrink_bb, shift_bb)
                            .unwrap();

                        self.builder.position_at_end(shift_bb);
                        let src_offset = self
                            .builder
                            .build_int_mul(
                                last_idx,
                                i64_type.const_int(elem_size, false),
                                "set_src_offset",
                            )
                            .unwrap();
                        let dst_offset = self
                            .builder
                            .build_int_mul(
                                found_idx,
                                i64_type.const_int(elem_size, false),
                                "set_dst_offset",
                            )
                            .unwrap();
                        let src_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    self.context.i8_type(),
                                    data_ptr,
                                    &[src_offset],
                                    "set_src_ptr",
                                )
                                .unwrap()
                        };
                        let dst_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    self.context.i8_type(),
                                    data_ptr,
                                    &[dst_offset],
                                    "set_dst_ptr",
                                )
                                .unwrap()
                        };
                        let typed_src_ptr = self
                            .builder
                            .build_pointer_cast(
                                src_ptr,
                                self.context.ptr_type(AddressSpace::default()),
                                "set_typed_src_ptr",
                            )
                            .unwrap();
                        let typed_dst_ptr = self
                            .builder
                            .build_pointer_cast(
                                dst_ptr,
                                self.context.ptr_type(AddressSpace::default()),
                                "set_typed_dst_ptr",
                            )
                            .unwrap();
                        let last_value = self
                            .builder
                            .build_load(elem_llvm_ty, typed_src_ptr, "set_last_value")
                            .unwrap();
                        self.builder.build_store(typed_dst_ptr, last_value).unwrap();
                        self.builder.build_unconditional_branch(shrink_bb).unwrap();

                        self.builder.position_at_end(shrink_bb);
                        let new_length = self
                            .builder
                            .build_int_sub(
                                length,
                                i64_type.const_int(1, false),
                                "set_removed_length",
                            )
                            .unwrap();
                        self.builder.build_store(length_ptr, new_length).unwrap();
                        self.builder.build_unconditional_branch(merge_bb).unwrap();

                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(self.context.bool_type(), "set_remove_phi")
                            .unwrap();
                        phi.add_incoming(&[
                            (&self.context.bool_type().const_zero(), done_bb),
                            (&self.context.bool_type().const_int(1, false), shrink_bb),
                        ]);
                        Ok(phi.as_basic_value())
                    }
                    _ => unreachable!(),
                }
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
        let (ptr, ty) = {
            let var = self.variables.get(option_name).unwrap();
            (var.ptr, var.ty.clone())
        };
        self.compile_option_method_on_value(ptr.into(), &ty, method)
    }

    pub fn compile_option_method_on_value(
        &mut self,
        option_value: BasicValueEnum<'ctx>,
        option_expr_ty: &Type,
        method: &str,
    ) -> Result<BasicValueEnum<'ctx>> {
        let option_ptr =
            self.materialize_value_pointer_for_type(option_value, option_expr_ty, "option_tmp")?;
        // Assuming Option<T> is { is_some: i8, value: T }
        // We need to infer T from var.ty
        let option_ty = match option_expr_ty {
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
                    .unwrap()
                    .into_int_value();
                let is_some_bool = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        is_some,
                        self.context.i8_type().const_int(0, false),
                        "is_some_bool",
                    )
                    .unwrap();
                Ok(is_some_bool.into())
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
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Option.unwrap outside function"))?;
                let panic_bb = self
                    .context
                    .append_basic_block(current_fn, "option_unwrap_panic");
                let ok_bb = self
                    .context
                    .append_basic_block(current_fn, "option_unwrap_ok");

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
                let is_some_bool = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        is_some,
                        self.context.i8_type().const_int(0, false),
                        "is_some_bool",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(is_some_bool, ok_bb, panic_bb)
                    .unwrap();

                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let exit_fn = self.get_or_declare_exit();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Option.unwrap() called on None\\n",
                        "opt_unwrap_panic",
                    )
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
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
        let (ptr, ty) = {
            let var = self.variables.get(result_name).unwrap();
            (var.ptr, var.ty.clone())
        };
        self.compile_result_method_on_value(ptr.into(), &ty, method)
    }

    pub fn compile_result_method_on_value(
        &mut self,
        result_value: BasicValueEnum<'ctx>,
        result_expr_ty: &Type,
        method: &str,
    ) -> Result<BasicValueEnum<'ctx>> {
        let result_ptr =
            self.materialize_value_pointer_for_type(result_value, result_expr_ty, "result_tmp")?;
        // Result<T, E> is struct { is_ok: i8, ok_value: T, err_value: E }
        let (ok_ty, err_ty) = match result_expr_ty {
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
                    .unwrap()
                    .into_int_value();
                let is_ok = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        tag,
                        self.context.i8_type().const_int(0, false),
                        "is_ok",
                    )
                    .unwrap();
                Ok(is_ok.into())
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
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Result.unwrap outside function"))?;
                let panic_bb = self
                    .context
                    .append_basic_block(current_fn, "result_unwrap_panic");
                let ok_bb = self
                    .context
                    .append_basic_block(current_fn, "result_unwrap_ok");

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
                let is_ok = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        tag,
                        self.context.i8_type().const_int(0, false),
                        "is_ok",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(is_ok, ok_bb, panic_bb)
                    .unwrap();

                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let exit_fn = self.get_or_declare_exit();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Result.unwrap() called on Error\\n",
                        "res_unwrap_panic",
                    )
                    .unwrap();
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
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

    pub fn create_fixed_list(
        &mut self,
        size: u64,
        list_ty: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>> {
        if size == 0 {
            return self.create_empty_list(list_ty);
        }
        let (elem_llvm_ty, _) = if let Some(list_ty) = list_ty {
            self.list_element_layout_from_list_type(list_ty)
        } else {
            self.list_element_layout_default()
        };

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
            .build_store(capacity_ptr, self.context.i64_type().const_int(size, false))
            .unwrap();

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

        let arr_ty = elem_llvm_ty.array_type(size as u32);
        let data_alloca = self
            .builder
            .build_alloca(arr_ty, "list_fixed_data")
            .unwrap();
        let data_ptr = unsafe {
            self.builder
                .build_gep(
                    arr_ty.as_basic_type_enum(),
                    data_alloca,
                    &[zero, zero],
                    "list_fixed_data_ptr",
                )
                .unwrap()
        };
        let data_i8_ptr = self
            .builder
            .build_pointer_cast(
                data_ptr,
                self.context.ptr_type(AddressSpace::default()),
                "list_fixed_data_i8",
            )
            .unwrap();
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
        self.builder
            .build_store(data_ptr_field, data_i8_ptr)
            .unwrap();

        Ok(self.builder.build_load(list_type, alloca, "list").unwrap())
    }

    pub fn create_empty_list(&mut self, list_ty: Option<&Type>) -> Result<BasicValueEnum<'ctx>> {
        let (_, elem_size) = if let Some(list_ty) = list_ty {
            self.list_element_layout_from_list_type(list_ty)
        } else {
            self.list_element_layout_default()
        };
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
            .const_int(initial_capacity * elem_size, false);
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

    fn grow_list_data_with_copy(
        &mut self,
        function: FunctionValue<'ctx>,
        data_ptr_ptr: PointerValue<'ctx>,
        capacity_ptr: PointerValue<'ctx>,
        capacity: IntValue<'ctx>,
        length: IntValue<'ctx>,
        elem_size: u64,
    ) -> Result<()> {
        let i64_type = self.context.i64_type();
        let new_capacity = self
            .builder
            .build_int_mul(capacity, i64_type.const_int(2, false), "new_cap")
            .unwrap();
        let new_size = self
            .builder
            .build_int_mul(
                new_capacity,
                i64_type.const_int(elem_size, false),
                "new_size",
            )
            .unwrap();
        let old_data = self
            .builder
            .build_load(
                self.context.ptr_type(AddressSpace::default()),
                data_ptr_ptr,
                "old_data",
            )
            .unwrap()
            .into_pointer_value();

        let malloc = self.get_or_declare_malloc();
        let grown_call = self
            .builder
            .build_call(malloc, &[new_size.into()], "grown_data")
            .unwrap();
        let grown_data = match grown_call.try_as_basic_value() {
            ValueKind::Basic(v) => v.into_pointer_value(),
            _ => panic!("malloc should return a value"),
        };

        let bytes_to_copy = self
            .builder
            .build_int_mul(length, i64_type.const_int(elem_size, false), "copy_bytes")
            .unwrap();
        let has_bytes = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                bytes_to_copy,
                i64_type.const_zero(),
                "has_copy_bytes",
            )
            .unwrap();

        let copy_cond_bb = self.context.append_basic_block(function, "list_copy_cond");
        let copy_body_bb = self.context.append_basic_block(function, "list_copy_body");
        let copy_done_bb = self.context.append_basic_block(function, "list_copy_done");
        self.builder
            .build_conditional_branch(has_bytes, copy_cond_bb, copy_done_bb)
            .unwrap();

        self.builder.position_at_end(copy_cond_bb);
        let idx_ptr = self.builder.build_alloca(i64_type, "copy_idx").unwrap();
        self.builder
            .build_store(idx_ptr, i64_type.const_zero())
            .unwrap();
        let cond_bb = self
            .context
            .append_basic_block(function, "list_copy_loop_cond");
        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(cond_bb);
        let idx = self
            .builder
            .build_load(i64_type, idx_ptr, "copy_idx_val")
            .unwrap()
            .into_int_value();
        let keep_copying = self
            .builder
            .build_int_compare(IntPredicate::SLT, idx, bytes_to_copy, "copy_continue")
            .unwrap();
        self.builder
            .build_conditional_branch(keep_copying, copy_body_bb, copy_done_bb)
            .unwrap();

        self.builder.position_at_end(copy_body_bb);
        let src = unsafe {
            self.builder
                .build_gep(self.context.i8_type(), old_data, &[idx], "copy_src")
                .unwrap()
        };
        let dst = unsafe {
            self.builder
                .build_gep(self.context.i8_type(), grown_data, &[idx], "copy_dst")
                .unwrap()
        };
        let byte = self
            .builder
            .build_load(self.context.i8_type(), src, "copy_byte")
            .unwrap();
        self.builder.build_store(dst, byte).unwrap();
        let next_idx = self
            .builder
            .build_int_add(idx, i64_type.const_int(1, false), "copy_next_idx")
            .unwrap();
        self.builder.build_store(idx_ptr, next_idx).unwrap();
        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(copy_done_bb);
        self.builder.build_store(data_ptr_ptr, grown_data).unwrap();
        self.builder
            .build_store(capacity_ptr, new_capacity)
            .unwrap();

        Ok(())
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
        let (list_ptr, list_ty) = {
            let var = self.variables.get(list_name).unwrap();
            (var.ptr, var.ty.clone())
        };
        self.compile_list_method_ptr(list_ptr, &list_ty, method, args)
    }

    pub fn compile_list_method_on_value(
        &mut self,
        list_value: BasicValueEnum<'ctx>,
        list_ty: &Type,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let list_ptr = self.materialize_value_pointer_for_type(list_value, list_ty, "list_tmp")?;
        let (elem_llvm_ty, elem_size) = self.list_element_layout_from_list_type(list_ty);
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
                let i32_type = self.context.i32_type();
                let i64_type = self.context.i64_type();
                let zero = i32_type.const_int(0, false);
                let one_i64 = i64_type.const_int(1, false);
                let elem_size_i64 = i64_type.const_int(elem_size, false);

                // Get current capacity/length/data pointers.
                let capacity_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(0, false)],
                            "cap_ptr",
                        )
                        .unwrap()
                };
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

                let capacity = self
                    .builder
                    .build_load(i64_type, capacity_ptr, "cap")
                    .unwrap()
                    .into_int_value();
                let length = self
                    .builder
                    .build_load(i64_type, length_ptr, "len")
                    .unwrap()
                    .into_int_value();

                // Grow backing storage when length reaches capacity.
                let need_grow = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, length, capacity, "need_grow")
                    .unwrap();
                let function = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("No current function for list push"))?;
                let grow_bb = self.context.append_basic_block(function, "list_grow");
                let cont_bb = self.context.append_basic_block(function, "list_push_cont");
                self.builder
                    .build_conditional_branch(need_grow, grow_bb, cont_bb)
                    .unwrap();

                self.builder.position_at_end(grow_bb);
                self.grow_list_data_with_copy(
                    function,
                    data_ptr_ptr,
                    capacity_ptr,
                    capacity,
                    length,
                    elem_size,
                )?;
                self.builder.build_unconditional_branch(cont_bb).unwrap();

                self.builder.position_at_end(cont_bb);
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
                    .build_int_mul(length, elem_size_i64, "offset")
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .unwrap();

                // Store the value
                let value = self.compile_expr(&args[0].node)?;
                self.builder.build_store(typed_elem_ptr, value).unwrap();

                // Increment length
                let new_length = self
                    .builder
                    .build_int_add(length, one_i64, "new_len")
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
                    .build_int_mul(
                        index,
                        self.context.i64_type().const_int(elem_size, false),
                        "offset",
                    )
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .unwrap();

                // Load and return the value
                let val = self
                    .builder
                    .build_load(elem_llvm_ty, typed_elem_ptr, "val")
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
                        self.context.i64_type().const_int(elem_size, false),
                        "offset",
                    )
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .unwrap();
                let val = self
                    .builder
                    .build_load(elem_llvm_ty, typed_elem_ptr, "val")
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
                    .build_int_mul(
                        index,
                        self.context.i64_type().const_int(elem_size, false),
                        "offset",
                    )
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .unwrap();

                // Store the value
                let value = self.compile_expr(&args[1].node)?;
                self.builder.build_store(typed_elem_ptr, value).unwrap();

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
        list_ty: &Type,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let (elem_llvm_ty, elem_size) = self.list_element_layout_from_list_type(list_ty);
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
                let i64_type = self.context.i64_type();
                let one_i64 = i64_type.const_int(1, false);
                let elem_size_i64 = i64_type.const_int(elem_size, false);

                let capacity_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(0, false)],
                            "cap_ptr",
                        )
                        .unwrap()
                };
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

                let capacity = self
                    .builder
                    .build_load(i64_type, capacity_ptr, "cap")
                    .unwrap()
                    .into_int_value();
                let length = self
                    .builder
                    .build_load(i64_type, length_ptr, "len")
                    .unwrap()
                    .into_int_value();

                let need_grow = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, length, capacity, "need_grow")
                    .unwrap();
                let function = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("No current function for list push"))?;
                let grow_bb = self.context.append_basic_block(function, "list_grow");
                let cont_bb = self.context.append_basic_block(function, "list_push_cont");
                self.builder
                    .build_conditional_branch(need_grow, grow_bb, cont_bb)
                    .unwrap();

                self.builder.position_at_end(grow_bb);
                self.grow_list_data_with_copy(
                    function,
                    data_ptr_ptr,
                    capacity_ptr,
                    capacity,
                    length,
                    elem_size,
                )?;
                self.builder.build_unconditional_branch(cont_bb).unwrap();

                self.builder.position_at_end(cont_bb);
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
                    .build_int_mul(length, elem_size_i64, "offset")
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .unwrap();

                let value = self.compile_expr(&args[0].node)?;
                self.builder.build_store(typed_elem_ptr, value).unwrap();

                let new_length = self
                    .builder
                    .build_int_add(length, one_i64, "new_len")
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
                    .build_int_mul(
                        index,
                        self.context.i64_type().const_int(elem_size, false),
                        "offset",
                    )
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .unwrap();

                let val = self
                    .builder
                    .build_load(elem_llvm_ty, typed_elem_ptr, "val")
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
                    .build_int_mul(
                        index,
                        self.context.i64_type().const_int(elem_size, false),
                        "offset",
                    )
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .unwrap();

                // Store the value
                let value = self.compile_expr(&args[1].node)?;
                self.builder.build_store(typed_elem_ptr, value).unwrap();

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
                        self.context.i64_type().const_int(elem_size, false),
                        "offset",
                    )
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .unwrap()
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .unwrap();
                let val = self
                    .builder
                    .build_load(elem_llvm_ty, typed_elem_ptr, "val")
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
        let (map_ptr, map_ty) = {
            let var = self.variables.get(map_name).unwrap();
            (var.ptr.into(), var.ty.clone())
        };
        self.compile_map_method_on_value(map_ptr, &map_ty, method, args)
    }

    pub fn compile_map_method_on_value(
        &mut self,
        map_value: BasicValueEnum<'ctx>,
        map_expr_ty: &Type,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let map_ptr = self.materialize_value_pointer_for_type(map_value, map_expr_ty, "map_tmp")?;
        let map_type = self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );

        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let zero = i32_type.const_int(0, false);
        let (key_ty, val_ty) = match map_expr_ty {
            Type::Map(k, v) => ((**k).clone(), (**v).clone()),
            _ => return Err(CodegenError::new("Expected Map type")),
        };
        let key_llvm = self.llvm_type(&key_ty);
        let val_llvm = self.llvm_type(&val_ty);

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

        match method {
            "length" => Ok(self
                .builder
                .build_load(i64_type, length_ptr, "len")
                .unwrap()),
            "insert" => self.compile_map_method_on_value(map_value, map_expr_ty, "set", args),
            "set" => {
                let key = self.compile_expr(&args[0].node)?;
                let value = self.compile_expr(&args[1].node)?;
                let length = self
                    .builder
                    .build_load(i64_type, length_ptr, "len")
                    .unwrap()
                    .into_int_value();
                let keys_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        keys_ptr_ptr,
                        "keys",
                    )
                    .unwrap()
                    .into_pointer_value();
                let values_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        values_ptr_ptr,
                        "vals",
                    )
                    .unwrap()
                    .into_pointer_value();

                // Very small linear map: update existing key if present.
                let idx_ptr = self.builder.build_alloca(i64_type, "map_idx").unwrap();
                self.builder
                    .build_store(idx_ptr, i64_type.const_int(0, false))
                    .unwrap();
                let current_fn = self.current_function.unwrap();
                let cond_bb = self.context.append_basic_block(current_fn, "map_set.cond");
                let body_bb = self.context.append_basic_block(current_fn, "map_set.body");
                let cont_bb = self.context.append_basic_block(current_fn, "map_set.cont");
                let update_bb = self
                    .context
                    .append_basic_block(current_fn, "map_set.update");
                let append_bb = self
                    .context
                    .append_basic_block(current_fn, "map_set.append");
                let done_bb = self.context.append_basic_block(current_fn, "map_set.done");

                self.builder.build_unconditional_branch(cond_bb).unwrap();
                self.builder.position_at_end(cond_bb);
                let i = self
                    .builder
                    .build_load(i64_type, idx_ptr, "i")
                    .unwrap()
                    .into_int_value();
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, i, length, "i_lt_len")
                    .unwrap();
                self.builder
                    .build_conditional_branch(in_bounds, body_bb, append_bb)
                    .unwrap();

                self.builder.position_at_end(body_bb);
                let offset = self
                    .builder
                    .build_int_mul(i, i64_type.const_int(8, false), "offset")
                    .unwrap();
                let key_slot = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), keys_ptr, &[offset], "key_slot")
                        .unwrap()
                };
                let existing = self
                    .builder
                    .build_load(key_llvm, key_slot, "existing")
                    .unwrap();
                let eq = if matches!(key_ty, Type::String) {
                    let strcmp = self.get_or_declare_strcmp();
                    let cmp = self
                        .builder
                        .build_call(strcmp, &[existing.into(), key.into()], "strcmp")
                        .unwrap();
                    let cmp_v = match cmp.try_as_basic_value() {
                        ValueKind::Basic(v) => v.into_int_value(),
                        _ => self.context.i32_type().const_int(1, false),
                    };
                    self.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            cmp_v,
                            self.context.i32_type().const_int(0, false),
                            "eq",
                        )
                        .unwrap()
                } else if existing.is_int_value() && key.is_int_value() {
                    self.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            existing.into_int_value(),
                            key.into_int_value(),
                            "eq",
                        )
                        .unwrap()
                } else {
                    self.context.bool_type().const_int(0, false)
                };
                self.builder
                    .build_conditional_branch(eq, update_bb, cont_bb)
                    .unwrap();

                self.builder.position_at_end(update_bb);
                let val_slot = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), values_ptr, &[offset], "val_slot")
                        .unwrap()
                };
                self.builder.build_store(val_slot, value).unwrap();
                self.builder.build_unconditional_branch(done_bb).unwrap();

                self.builder.position_at_end(cont_bb);
                let next_i = self
                    .builder
                    .build_int_add(i, i64_type.const_int(1, false), "next_i")
                    .unwrap();
                self.builder.build_store(idx_ptr, next_i).unwrap();
                self.builder.build_unconditional_branch(cond_bb).unwrap();

                self.builder.position_at_end(append_bb);
                let offset = self
                    .builder
                    .build_int_mul(length, i64_type.const_int(8, false), "append_off")
                    .unwrap();
                let key_slot = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), keys_ptr, &[offset], "key_slot_new")
                        .unwrap()
                };
                self.builder.build_store(key_slot, key).unwrap();
                let val_slot = unsafe {
                    self.builder
                        .build_gep(
                            self.context.i8_type(),
                            values_ptr,
                            &[offset],
                            "val_slot_new",
                        )
                        .unwrap()
                };
                self.builder.build_store(val_slot, value).unwrap();
                let new_len = self
                    .builder
                    .build_int_add(length, i64_type.const_int(1, false), "new_len")
                    .unwrap();
                self.builder.build_store(length_ptr, new_len).unwrap();
                self.builder.build_unconditional_branch(done_bb).unwrap();

                self.builder.position_at_end(done_bb);
                Ok(self.context.i8_type().const_int(0, false).into())
            }
            "get" => {
                let key = self.compile_expr(&args[0].node)?;
                let length = self
                    .builder
                    .build_load(i64_type, length_ptr, "len")
                    .unwrap()
                    .into_int_value();
                let keys_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        keys_ptr_ptr,
                        "keys",
                    )
                    .unwrap()
                    .into_pointer_value();
                let values_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        values_ptr_ptr,
                        "vals",
                    )
                    .unwrap()
                    .into_pointer_value();

                let idx_ptr = self.builder.build_alloca(i64_type, "map_idx").unwrap();
                let res_ptr = self.builder.build_alloca(val_llvm, "map_get_res").unwrap();
                self.builder
                    .build_store(idx_ptr, i64_type.const_int(0, false))
                    .unwrap();
                self.builder
                    .build_store(res_ptr, val_llvm.const_zero())
                    .unwrap();

                let current_fn = self.current_function.unwrap();
                let cond_bb = self.context.append_basic_block(current_fn, "map_get.cond");
                let body_bb = self.context.append_basic_block(current_fn, "map_get.body");
                let done_bb = self.context.append_basic_block(current_fn, "map_get.done");
                self.builder.build_unconditional_branch(cond_bb).unwrap();

                self.builder.position_at_end(cond_bb);
                let i = self
                    .builder
                    .build_load(i64_type, idx_ptr, "i")
                    .unwrap()
                    .into_int_value();
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, i, length, "i_lt_len")
                    .unwrap();
                self.builder
                    .build_conditional_branch(in_bounds, body_bb, done_bb)
                    .unwrap();

                self.builder.position_at_end(body_bb);
                let offset = self
                    .builder
                    .build_int_mul(i, i64_type.const_int(8, false), "offset")
                    .unwrap();
                let key_slot = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), keys_ptr, &[offset], "key_slot")
                        .unwrap()
                };
                let existing = self
                    .builder
                    .build_load(key_llvm, key_slot, "existing")
                    .unwrap();
                let eq = if matches!(key_ty, Type::String) {
                    let strcmp = self.get_or_declare_strcmp();
                    let cmp = self
                        .builder
                        .build_call(strcmp, &[existing.into(), key.into()], "strcmp")
                        .unwrap();
                    let cmp_v = match cmp.try_as_basic_value() {
                        ValueKind::Basic(v) => v.into_int_value(),
                        _ => self.context.i32_type().const_int(1, false),
                    };
                    self.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            cmp_v,
                            self.context.i32_type().const_int(0, false),
                            "eq",
                        )
                        .unwrap()
                } else if existing.is_int_value() && key.is_int_value() {
                    self.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            existing.into_int_value(),
                            key.into_int_value(),
                            "eq",
                        )
                        .unwrap()
                } else {
                    self.context.bool_type().const_int(0, false)
                };
                let next_bb = self.context.append_basic_block(current_fn, "map_get.next");
                let found_bb = self.context.append_basic_block(current_fn, "map_get.found");
                self.builder
                    .build_conditional_branch(eq, found_bb, next_bb)
                    .unwrap();

                self.builder.position_at_end(found_bb);
                let val_slot = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), values_ptr, &[offset], "val_slot")
                        .unwrap()
                };
                let found = self
                    .builder
                    .build_load(val_llvm, val_slot, "found")
                    .unwrap();
                self.builder.build_store(res_ptr, found).unwrap();
                self.builder.build_unconditional_branch(done_bb).unwrap();

                self.builder.position_at_end(next_bb);
                let next_i = self
                    .builder
                    .build_int_add(i, i64_type.const_int(1, false), "next_i")
                    .unwrap();
                self.builder.build_store(idx_ptr, next_i).unwrap();
                self.builder.build_unconditional_branch(cond_bb).unwrap();

                self.builder.position_at_end(done_bb);
                Ok(self
                    .builder
                    .build_load(val_llvm, res_ptr, "map_get_res")
                    .unwrap())
            }
            "contains" => {
                let key = self.compile_expr(&args[0].node)?;
                let length = self
                    .builder
                    .build_load(i64_type, length_ptr, "len")
                    .unwrap()
                    .into_int_value();
                let keys_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        keys_ptr_ptr,
                        "keys",
                    )
                    .unwrap()
                    .into_pointer_value();

                let idx_ptr = self.builder.build_alloca(i64_type, "map_idx").unwrap();
                let res_ptr = self
                    .builder
                    .build_alloca(self.context.bool_type(), "contains_res")
                    .unwrap();
                self.builder
                    .build_store(idx_ptr, i64_type.const_int(0, false))
                    .unwrap();
                self.builder
                    .build_store(res_ptr, self.context.bool_type().const_int(0, false))
                    .unwrap();

                let current_fn = self.current_function.unwrap();
                let cond_bb = self
                    .context
                    .append_basic_block(current_fn, "map_contains.cond");
                let body_bb = self
                    .context
                    .append_basic_block(current_fn, "map_contains.body");
                let done_bb = self
                    .context
                    .append_basic_block(current_fn, "map_contains.done");
                self.builder.build_unconditional_branch(cond_bb).unwrap();

                self.builder.position_at_end(cond_bb);
                let i = self
                    .builder
                    .build_load(i64_type, idx_ptr, "i")
                    .unwrap()
                    .into_int_value();
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, i, length, "i_lt_len")
                    .unwrap();
                self.builder
                    .build_conditional_branch(in_bounds, body_bb, done_bb)
                    .unwrap();

                self.builder.position_at_end(body_bb);
                let offset = self
                    .builder
                    .build_int_mul(i, i64_type.const_int(8, false), "offset")
                    .unwrap();
                let key_slot = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), keys_ptr, &[offset], "key_slot")
                        .unwrap()
                };
                let existing = self
                    .builder
                    .build_load(key_llvm, key_slot, "existing")
                    .unwrap();
                let eq = if matches!(key_ty, Type::String) {
                    let strcmp = self.get_or_declare_strcmp();
                    let cmp = self
                        .builder
                        .build_call(strcmp, &[existing.into(), key.into()], "strcmp")
                        .unwrap();
                    let cmp_v = match cmp.try_as_basic_value() {
                        ValueKind::Basic(v) => v.into_int_value(),
                        _ => self.context.i32_type().const_int(1, false),
                    };
                    self.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            cmp_v,
                            self.context.i32_type().const_int(0, false),
                            "eq",
                        )
                        .unwrap()
                } else if existing.is_int_value() && key.is_int_value() {
                    self.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            existing.into_int_value(),
                            key.into_int_value(),
                            "eq",
                        )
                        .unwrap()
                } else {
                    self.context.bool_type().const_int(0, false)
                };
                let next_bb = self
                    .context
                    .append_basic_block(current_fn, "map_contains.next");
                let found_bb = self
                    .context
                    .append_basic_block(current_fn, "map_contains.found");
                self.builder
                    .build_conditional_branch(eq, found_bb, next_bb)
                    .unwrap();

                self.builder.position_at_end(found_bb);
                self.builder
                    .build_store(res_ptr, self.context.bool_type().const_int(1, false))
                    .unwrap();
                self.builder.build_unconditional_branch(done_bb).unwrap();

                self.builder.position_at_end(next_bb);
                let next_i = self
                    .builder
                    .build_int_add(i, i64_type.const_int(1, false), "next_i")
                    .unwrap();
                self.builder.build_store(idx_ptr, next_i).unwrap();
                self.builder.build_unconditional_branch(cond_bb).unwrap();

                self.builder.position_at_end(done_bb);
                Ok(self
                    .builder
                    .build_load(self.context.bool_type(), res_ptr, "contains_res")
                    .unwrap())
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
        let (range_ptr, range_ty) = {
            let var = self.variables.get(range_name).unwrap();
            let ptr_type = self.context.ptr_type(AddressSpace::default());
            let range_ptr = self
                .builder
                .build_load(ptr_type, var.ptr, "range_ptr")
                .unwrap()
                .into_pointer_value();
            (range_ptr, var.ty.clone())
        };
        self.compile_range_method_on_value(range_ptr.into(), &range_ty, method)
    }

    pub fn compile_range_method_on_value(
        &mut self,
        range_value: BasicValueEnum<'ctx>,
        range_expr_ty: &Type,
        method: &str,
    ) -> Result<BasicValueEnum<'ctx>> {
        let range_ptr = match range_value {
            BasicValueEnum::PointerValue(ptr) => ptr,
            _ => {
                self.materialize_value_pointer_for_type(range_value, range_expr_ty, "range_tmp")?
            }
        };
        let range_element_ty = match range_expr_ty {
            Type::Range(inner) => &**inner,
            _ => return Err(CodegenError::new("Expected Range type")),
        };
        let element_llvm_ty = self.llvm_type(range_element_ty);
        let range_type = self.get_range_type(element_llvm_ty)?;
        let i32_type = self.context.i32_type();
        let zero = i32_type.const_int(0, false);
        let one = i32_type.const_int(1, false);
        let two = i32_type.const_int(2, false);
        let three = i32_type.const_int(3, false);

        // Range struct layout: { start: i64, end: i64, step: i64, current: i64 }
        match method {
            "has_next" => {
                let step_ptr = unsafe {
                    self.builder
                        .build_gep(range_type, range_ptr, &[zero, two], "step_ptr")
                        .unwrap()
                };
                let current_ptr = unsafe {
                    self.builder
                        .build_gep(range_type, range_ptr, &[zero, three], "current_ptr")
                        .unwrap()
                };
                let end_ptr = unsafe {
                    self.builder
                        .build_gep(range_type, range_ptr, &[zero, one], "end_ptr")
                        .unwrap()
                };

                match element_llvm_ty {
                    inkwell::types::BasicTypeEnum::IntType(int_ty) => {
                        let step = self
                            .builder
                            .build_load(int_ty, step_ptr, "step")
                            .unwrap()
                            .into_int_value();
                        let current = self
                            .builder
                            .build_load(int_ty, current_ptr, "current")
                            .unwrap()
                            .into_int_value();
                        let end = self
                            .builder
                            .build_load(int_ty, end_ptr, "end")
                            .unwrap()
                            .into_int_value();

                        let step_positive = self
                            .builder
                            .build_int_compare(
                                IntPredicate::SGT,
                                step,
                                int_ty.const_zero(),
                                "step_positive",
                            )
                            .unwrap();
                        let current_lt_end = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, current, end, "current_lt_end")
                            .unwrap();
                        let current_gt_end = self
                            .builder
                            .build_int_compare(IntPredicate::SGT, current, end, "current_gt_end")
                            .unwrap();
                        let result = self
                            .builder
                            .build_select(step_positive, current_lt_end, current_gt_end, "has_next")
                            .unwrap();
                        Ok(result.into_int_value().into())
                    }
                    inkwell::types::BasicTypeEnum::FloatType(float_ty) => {
                        let step = self
                            .builder
                            .build_load(float_ty, step_ptr, "step")
                            .unwrap()
                            .into_float_value();
                        let current = self
                            .builder
                            .build_load(float_ty, current_ptr, "current")
                            .unwrap()
                            .into_float_value();
                        let end = self
                            .builder
                            .build_load(float_ty, end_ptr, "end")
                            .unwrap()
                            .into_float_value();

                        let step_positive = self
                            .builder
                            .build_float_compare(
                                inkwell::FloatPredicate::OGT,
                                step,
                                float_ty.const_float(0.0),
                                "step_positive",
                            )
                            .unwrap();
                        let current_lt_end = self
                            .builder
                            .build_float_compare(
                                inkwell::FloatPredicate::OLT,
                                current,
                                end,
                                "current_lt_end",
                            )
                            .unwrap();
                        let current_gt_end = self
                            .builder
                            .build_float_compare(
                                inkwell::FloatPredicate::OGT,
                                current,
                                end,
                                "current_gt_end",
                            )
                            .unwrap();
                        let result = self
                            .builder
                            .build_select(step_positive, current_lt_end, current_gt_end, "has_next")
                            .unwrap();
                        Ok(result.into_int_value().into())
                    }
                    _ => Err(CodegenError::new(
                        "Range<T> codegen supports only Integer and Float elements",
                    )),
                }
            }
            "next" => {
                let current_ptr = unsafe {
                    self.builder
                        .build_gep(range_type, range_ptr, &[zero, three], "current_ptr")
                        .unwrap()
                };
                let step_ptr = unsafe {
                    self.builder
                        .build_gep(range_type, range_ptr, &[zero, two], "step_ptr")
                        .unwrap()
                };

                match element_llvm_ty {
                    inkwell::types::BasicTypeEnum::IntType(int_ty) => {
                        let current = self
                            .builder
                            .build_load(int_ty, current_ptr, "current")
                            .unwrap()
                            .into_int_value();
                        let step = self
                            .builder
                            .build_load(int_ty, step_ptr, "step")
                            .unwrap()
                            .into_int_value();
                        let new_current = self
                            .builder
                            .build_int_add(current, step, "new_current")
                            .unwrap();
                        self.builder.build_store(current_ptr, new_current).unwrap();
                        Ok(current.into())
                    }
                    inkwell::types::BasicTypeEnum::FloatType(float_ty) => {
                        let current = self
                            .builder
                            .build_load(float_ty, current_ptr, "current")
                            .unwrap()
                            .into_float_value();
                        let step = self
                            .builder
                            .build_load(float_ty, step_ptr, "step")
                            .unwrap()
                            .into_float_value();
                        let new_current = self
                            .builder
                            .build_float_add(current, step, "new_current")
                            .unwrap();
                        self.builder.build_store(current_ptr, new_current).unwrap();
                        Ok(current.into())
                    }
                    _ => Err(CodegenError::new(
                        "Range<T> codegen supports only Integer and Float elements",
                    )),
                }
            }
            _ => Err(CodegenError::new(format!(
                "Unknown Range method: {}",
                method
            ))),
        }
    }
}
