//! Type-specific codegen helpers for collections, Option, and Result types
use crate::ast::{Expr, Spanned, Type};
use inkwell::targets::{
    CodeModel, InitializationConfig, RelocMode, Target, TargetData, TargetMachine,
};
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate, OptimizationLevel};
use std::collections::HashSet;
use std::sync::OnceLock;

use crate::codegen::core::{Codegen, CodegenError, Result};

static CODEGEN_TARGET_DATA_LAYOUT: OnceLock<Option<String>> = OnceLock::new();

impl<'ctx> Codegen<'ctx> {
    fn zero_initialize_allocated_bytes(
        &mut self,
        buffer_ptr: PointerValue<'ctx>,
        byte_len: u64,
        context_name: &str,
    ) -> Result<()> {
        if byte_len == 0 {
            return Ok(());
        }

        let current_fn = self
            .current_function
            .ok_or_else(|| CodegenError::new(format!("{context_name} used outside function")))?;
        let i64_type = self.context.i64_type();
        let i8_type = self.context.i8_type();
        let index_ptr = self
            .builder
            .build_alloca(i64_type, "zero_init_idx")
            .map_err(|_| CodegenError::new("failed to allocate zero-init index"))?;
        self.builder
            .build_store(index_ptr, i64_type.const_zero())
            .map_err(|_| CodegenError::new("failed to initialize zero-init index"))?;

        let cond_bb = self
            .context
            .append_basic_block(current_fn, "zero_init_cond");
        let body_bb = self
            .context
            .append_basic_block(current_fn, "zero_init_body");
        let done_bb = self
            .context
            .append_basic_block(current_fn, "zero_init_done");

        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::new("failed to branch into zero-init loop"))?;

        self.builder.position_at_end(cond_bb);
        let index = self
            .builder
            .build_load(i64_type, index_ptr, "zero_init_index")
            .map_err(|_| CodegenError::new("failed to load zero-init index"))?
            .into_int_value();
        let keep_zeroing = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                index,
                i64_type.const_int(byte_len, false),
                "zero_init_continue",
            )
            .map_err(|_| CodegenError::new("failed to compare zero-init index"))?;
        self.builder
            .build_conditional_branch(keep_zeroing, body_bb, done_bb)
            .map_err(|_| CodegenError::new("failed to branch inside zero-init loop"))?;

        self.builder.position_at_end(body_bb);
        let byte_ptr = unsafe {
            self.builder
                .build_gep(i8_type, buffer_ptr, &[index], "zero_init_byte_ptr")
                .map_err(|_| CodegenError::new("failed to compute zero-init byte pointer"))?
        };
        self.builder
            .build_store(byte_ptr, i8_type.const_zero())
            .map_err(|_| CodegenError::new("failed to store zero-init byte"))?;
        let next_index = self
            .builder
            .build_int_add(index, i64_type.const_int(1, false), "zero_init_next")
            .map_err(|_| CodegenError::new("failed to increment zero-init index"))?;
        self.builder
            .build_store(index_ptr, next_index)
            .map_err(|_| CodegenError::new("failed to store zero-init index"))?;
        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::new("failed to continue zero-init loop"))?;

        self.builder.position_at_end(done_bb);
        Ok(())
    }

    fn init_codegen_target_data_layout() -> Option<String> {
        Target::initialize_native(&InitializationConfig::default()).ok()?;
        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple).ok()?;
        let cpu = TargetMachine::get_host_cpu_name();
        let features = TargetMachine::get_host_cpu_features();
        let machine = target.create_target_machine(
            &triple,
            cpu.to_str().unwrap_or("generic"),
            features.to_str().unwrap_or(""),
            OptimizationLevel::Default,
            RelocMode::Default,
            CodeModel::Default,
        )?;
        Some(
            machine
                .get_target_data()
                .get_data_layout()
                .as_str()
                .to_string_lossy()
                .into_owned(),
        )
    }

    fn fallback_storage_size_of_llvm_type(&self, ty: inkwell::types::BasicTypeEnum<'ctx>) -> u64 {
        match ty {
            inkwell::types::BasicTypeEnum::IntType(int_ty) => {
                u64::from(int_ty.get_bit_width().max(8)).div_ceil(8)
            }
            inkwell::types::BasicTypeEnum::FloatType(float_ty) => match float_ty.get_bit_width() {
                16 => 2,
                32 => 4,
                64 => 8,
                80 | 86 => 16,
                128 => 16,
                _ => 8,
            },
            inkwell::types::BasicTypeEnum::PointerType(_) => 8,
            inkwell::types::BasicTypeEnum::ArrayType(array_ty) => {
                self.fallback_storage_size_of_llvm_type(array_ty.get_element_type())
                    * array_ty.len() as u64
            }
            inkwell::types::BasicTypeEnum::StructType(struct_ty) => struct_ty
                .get_field_types()
                .iter()
                .map(|field_ty| self.fallback_storage_size_of_llvm_type(*field_ty))
                .sum::<u64>()
                .max(1),
            inkwell::types::BasicTypeEnum::VectorType(vector_ty) => {
                self.fallback_storage_size_of_llvm_type(vector_ty.get_element_type())
                    * u64::from(vector_ty.get_size())
            }
            inkwell::types::BasicTypeEnum::ScalableVectorType(_) => 16,
        }
    }

    pub(crate) fn emit_runtime_error(&mut self, message: &str, global_name: &str) -> Result<()> {
        let printf = self.get_or_declare_printf();
        let exit_fn = self.get_or_declare_exit();
        let msg = self
            .builder
            .build_global_string_ptr(&format!("{message}\n"), global_name)
            .map_err(|_| CodegenError::new("failed to materialize runtime error message"))?;
        self.builder
            .build_call(printf, &[msg.as_pointer_value().into()], "")
            .map_err(|_| CodegenError::new("failed to emit runtime error printf call"))?;
        self.builder
            .build_call(
                exit_fn,
                &[self.context.i32_type().const_int(1, false).into()],
                "",
            )
            .map_err(|_| CodegenError::new("failed to emit runtime error exit call"))?;
        self.builder
            .build_unreachable()
            .map_err(|_| CodegenError::new("failed to terminate runtime error block"))?;
        Ok(())
    }

    pub(crate) fn build_value_equality(
        &mut self,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
        ty: &Type,
        name: &str,
    ) -> Result<IntValue<'ctx>> {
        if let Type::Option(inner_ty) = ty {
            let lhs_ptr =
                self.materialize_value_pointer_for_type(lhs, ty, &format!("{name}_lhs_tmp"))?;
            let rhs_ptr =
                self.materialize_value_pointer_for_type(rhs, ty, &format!("{name}_rhs_tmp"))?;
            let llvm_inner_ty = self.llvm_type(inner_ty);
            let option_struct_type = self
                .context
                .struct_type(&[self.context.i8_type().into(), llvm_inner_ty], false);
            let i32_type = self.context.i32_type();
            let zero = i32_type.const_zero();
            let one = i32_type.const_int(1, false);

            let lhs_tag_ptr = unsafe {
                self.builder
                    .build_gep(
                        option_struct_type.as_basic_type_enum(),
                        lhs_ptr,
                        &[zero, zero],
                        &format!("{name}_lhs_tag_ptr"),
                    )
                    .map_err(|_| CodegenError::new("failed to compute option lhs tag pointer"))?
            };
            let rhs_tag_ptr = unsafe {
                self.builder
                    .build_gep(
                        option_struct_type.as_basic_type_enum(),
                        rhs_ptr,
                        &[zero, zero],
                        &format!("{name}_rhs_tag_ptr"),
                    )
                    .map_err(|_| CodegenError::new("failed to compute option rhs tag pointer"))?
            };
            let lhs_tag = self
                .builder
                .build_load(
                    self.context.i8_type(),
                    lhs_tag_ptr,
                    &format!("{name}_lhs_tag"),
                )
                .map_err(|_| CodegenError::new("failed to load option lhs tag"))?
                .into_int_value();
            let rhs_tag = self
                .builder
                .build_load(
                    self.context.i8_type(),
                    rhs_tag_ptr,
                    &format!("{name}_rhs_tag"),
                )
                .map_err(|_| CodegenError::new("failed to load option rhs tag"))?
                .into_int_value();
            let tags_eq = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    lhs_tag,
                    rhs_tag,
                    &format!("{name}_tags_eq"),
                )
                .map_err(|_| CodegenError::new("failed to compare option tags"))?;
            let lhs_some = self
                .builder
                .build_int_compare(
                    IntPredicate::NE,
                    lhs_tag,
                    self.context.i8_type().const_zero(),
                    &format!("{name}_lhs_some"),
                )
                .map_err(|_| CodegenError::new("failed to compute option Some flag"))?;
            let lhs_value_ptr = unsafe {
                self.builder
                    .build_gep(
                        option_struct_type.as_basic_type_enum(),
                        lhs_ptr,
                        &[zero, one],
                        &format!("{name}_lhs_value_ptr"),
                    )
                    .map_err(|_| CodegenError::new("failed to compute option lhs value pointer"))?
            };
            let rhs_value_ptr = unsafe {
                self.builder
                    .build_gep(
                        option_struct_type.as_basic_type_enum(),
                        rhs_ptr,
                        &[zero, one],
                        &format!("{name}_rhs_value_ptr"),
                    )
                    .map_err(|_| CodegenError::new("failed to compute option rhs value pointer"))?
            };
            let lhs_value = self
                .builder
                .build_load(llvm_inner_ty, lhs_value_ptr, &format!("{name}_lhs_value"))
                .map_err(|_| CodegenError::new("failed to load option lhs payload"))?;
            let rhs_value = self
                .builder
                .build_load(llvm_inner_ty, rhs_value_ptr, &format!("{name}_rhs_value"))
                .map_err(|_| CodegenError::new("failed to load option rhs payload"))?;
            let inner_eq = self.build_value_equality(
                lhs_value,
                rhs_value,
                inner_ty,
                &format!("{name}_inner"),
            )?;
            let payload_eq_or_none = self
                .builder
                .build_select(
                    lhs_some,
                    inner_eq,
                    self.context.bool_type().const_all_ones(),
                    &format!("{name}_payload_eq_or_none"),
                )
                .map_err(|_| CodegenError::new("failed to select option payload equality"))?
                .into_int_value();
            return self
                .builder
                .build_and(tags_eq, payload_eq_or_none, name)
                .map_err(|_| CodegenError::new("failed to combine option equality"));
        }

        if let Type::Result(ok_ty, err_ty) = ty {
            let lhs_ptr =
                self.materialize_value_pointer_for_type(lhs, ty, &format!("{name}_lhs_tmp"))?;
            let rhs_ptr =
                self.materialize_value_pointer_for_type(rhs, ty, &format!("{name}_rhs_tmp"))?;
            let ok_llvm = self.llvm_type(ok_ty);
            let err_llvm = self.llvm_type(err_ty);
            let result_struct_type = self
                .context
                .struct_type(&[self.context.i8_type().into(), ok_llvm, err_llvm], false);
            let i32_type = self.context.i32_type();
            let zero = i32_type.const_zero();
            let one = i32_type.const_int(1, false);
            let two = i32_type.const_int(2, false);

            let lhs_tag_ptr = unsafe {
                self.builder
                    .build_gep(
                        result_struct_type.as_basic_type_enum(),
                        lhs_ptr,
                        &[zero, zero],
                        &format!("{name}_lhs_tag_ptr"),
                    )
                    .map_err(|_| CodegenError::new("failed to compute result lhs tag pointer"))?
            };
            let rhs_tag_ptr = unsafe {
                self.builder
                    .build_gep(
                        result_struct_type.as_basic_type_enum(),
                        rhs_ptr,
                        &[zero, zero],
                        &format!("{name}_rhs_tag_ptr"),
                    )
                    .map_err(|_| CodegenError::new("failed to compute result rhs tag pointer"))?
            };
            let lhs_tag = self
                .builder
                .build_load(
                    self.context.i8_type(),
                    lhs_tag_ptr,
                    &format!("{name}_lhs_tag"),
                )
                .map_err(|_| CodegenError::new("failed to load result lhs tag"))?
                .into_int_value();
            let rhs_tag = self
                .builder
                .build_load(
                    self.context.i8_type(),
                    rhs_tag_ptr,
                    &format!("{name}_rhs_tag"),
                )
                .map_err(|_| CodegenError::new("failed to load result rhs tag"))?
                .into_int_value();
            let tags_eq = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    lhs_tag,
                    rhs_tag,
                    &format!("{name}_tags_eq"),
                )
                .map_err(|_| CodegenError::new("failed to compare result tags"))?;
            let lhs_ok = self
                .builder
                .build_int_compare(
                    IntPredicate::NE,
                    lhs_tag,
                    self.context.i8_type().const_zero(),
                    &format!("{name}_lhs_ok"),
                )
                .map_err(|_| CodegenError::new("failed to compute result Ok flag"))?;

            let lhs_ok_ptr = unsafe {
                self.builder
                    .build_gep(
                        result_struct_type.as_basic_type_enum(),
                        lhs_ptr,
                        &[zero, one],
                        &format!("{name}_lhs_ok_ptr"),
                    )
                    .map_err(|_| CodegenError::new("failed to compute result lhs Ok pointer"))?
            };
            let rhs_ok_ptr = unsafe {
                self.builder
                    .build_gep(
                        result_struct_type.as_basic_type_enum(),
                        rhs_ptr,
                        &[zero, one],
                        &format!("{name}_rhs_ok_ptr"),
                    )
                    .map_err(|_| CodegenError::new("failed to compute result rhs Ok pointer"))?
            };
            let lhs_err_ptr = unsafe {
                self.builder
                    .build_gep(
                        result_struct_type.as_basic_type_enum(),
                        lhs_ptr,
                        &[zero, two],
                        &format!("{name}_lhs_err_ptr"),
                    )
                    .map_err(|_| CodegenError::new("failed to compute result lhs Err pointer"))?
            };
            let rhs_err_ptr = unsafe {
                self.builder
                    .build_gep(
                        result_struct_type.as_basic_type_enum(),
                        rhs_ptr,
                        &[zero, two],
                        &format!("{name}_rhs_err_ptr"),
                    )
                    .map_err(|_| CodegenError::new("failed to compute result rhs Err pointer"))?
            };
            let ok_eq = self.build_value_equality(
                self.builder
                    .build_load(ok_llvm, lhs_ok_ptr, &format!("{name}_lhs_ok_value"))
                    .map_err(|_| CodegenError::new("failed to load result lhs Ok payload"))?,
                self.builder
                    .build_load(ok_llvm, rhs_ok_ptr, &format!("{name}_rhs_ok_value"))
                    .map_err(|_| CodegenError::new("failed to load result rhs Ok payload"))?,
                ok_ty,
                &format!("{name}_ok_eq"),
            )?;
            let err_eq = self.build_value_equality(
                self.builder
                    .build_load(err_llvm, lhs_err_ptr, &format!("{name}_lhs_err_value"))
                    .map_err(|_| CodegenError::new("failed to load result lhs Err payload"))?,
                self.builder
                    .build_load(err_llvm, rhs_err_ptr, &format!("{name}_rhs_err_value"))
                    .map_err(|_| CodegenError::new("failed to load result rhs Err payload"))?,
                err_ty,
                &format!("{name}_err_eq"),
            )?;
            let payload_eq = self
                .builder
                .build_select(lhs_ok, ok_eq, err_eq, &format!("{name}_payload_eq"))
                .map_err(|_| CodegenError::new("failed to select result payload equality"))?
                .into_int_value();
            return self
                .builder
                .build_and(tags_eq, payload_eq, name)
                .map_err(|_| CodegenError::new("failed to combine result equality"));
        }

        if let Type::Named(name) = ty {
            if let Some(enum_info) = self.enums.get(name) {
                let enum_struct_type = enum_info.struct_type;
                let payload_slots = enum_info.payload_slots;
                let lhs_ptr =
                    self.materialize_value_pointer_for_type(lhs, ty, &format!("{name}_lhs_tmp"))?;
                let rhs_ptr =
                    self.materialize_value_pointer_for_type(rhs, ty, &format!("{name}_rhs_tmp"))?;
                let i32_type = self.context.i32_type();
                let zero = i32_type.const_zero();

                let lhs_tag_ptr = unsafe {
                    self.builder
                        .build_gep(
                            enum_struct_type.as_basic_type_enum(),
                            lhs_ptr,
                            &[zero, zero],
                            &format!("{name}_lhs_tag_ptr"),
                        )
                        .map_err(|_| CodegenError::new("failed to compute enum lhs tag pointer"))?
                };
                let rhs_tag_ptr = unsafe {
                    self.builder
                        .build_gep(
                            enum_struct_type.as_basic_type_enum(),
                            rhs_ptr,
                            &[zero, zero],
                            &format!("{name}_rhs_tag_ptr"),
                        )
                        .map_err(|_| CodegenError::new("failed to compute enum rhs tag pointer"))?
                };
                let lhs_tag = self
                    .builder
                    .build_load(
                        self.context.i8_type(),
                        lhs_tag_ptr,
                        &format!("{name}_lhs_tag"),
                    )
                    .map_err(|_| CodegenError::new("failed to load enum lhs tag"))?
                    .into_int_value();
                let rhs_tag = self
                    .builder
                    .build_load(
                        self.context.i8_type(),
                        rhs_tag_ptr,
                        &format!("{name}_rhs_tag"),
                    )
                    .map_err(|_| CodegenError::new("failed to load enum rhs tag"))?
                    .into_int_value();
                let mut eq = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        lhs_tag,
                        rhs_tag,
                        &format!("{name}_tag_eq"),
                    )
                    .map_err(|_| CodegenError::new("failed to compare enum tags"))?;

                for slot in 0..payload_slots {
                    let field_index = i32_type.const_int((slot + 1) as u64, false);
                    let lhs_payload_ptr = unsafe {
                        self.builder
                            .build_gep(
                                enum_struct_type.as_basic_type_enum(),
                                lhs_ptr,
                                &[zero, field_index],
                                &format!("{name}_lhs_payload_ptr_{slot}"),
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compute enum lhs payload pointer")
                            })?
                    };
                    let rhs_payload_ptr = unsafe {
                        self.builder
                            .build_gep(
                                enum_struct_type.as_basic_type_enum(),
                                rhs_ptr,
                                &[zero, field_index],
                                &format!("{name}_rhs_payload_ptr_{slot}"),
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compute enum rhs payload pointer")
                            })?
                    };
                    let lhs_payload = self
                        .builder
                        .build_load(
                            self.context.i64_type(),
                            lhs_payload_ptr,
                            &format!("{name}_lhs_payload_{slot}"),
                        )
                        .map_err(|_| CodegenError::new("failed to load enum lhs payload"))?
                        .into_int_value();
                    let rhs_payload = self
                        .builder
                        .build_load(
                            self.context.i64_type(),
                            rhs_payload_ptr,
                            &format!("{name}_rhs_payload_{slot}"),
                        )
                        .map_err(|_| CodegenError::new("failed to load enum rhs payload"))?
                        .into_int_value();
                    let payload_eq = self
                        .builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            lhs_payload,
                            rhs_payload,
                            &format!("{name}_payload_eq_{slot}"),
                        )
                        .map_err(|_| CodegenError::new("failed to compare enum payloads"))?;
                    eq = self
                        .builder
                        .build_and(eq, payload_eq, &format!("{name}_eq_{slot}"))
                        .map_err(|_| CodegenError::new("failed to combine enum equality"))?;
                }

                return Ok(eq);
            }
        }

        if matches!(ty, Type::String) {
            let lhs_ptr = lhs.into_pointer_value();
            let rhs_ptr = rhs.into_pointer_value();
            let lhs_null = self
                .builder
                .build_is_null(lhs_ptr, &format!("{name}_lhs_null"))
                .map_err(|_| CodegenError::new("failed to test lhs string nullness"))?;
            let rhs_null = self
                .builder
                .build_is_null(rhs_ptr, &format!("{name}_rhs_null"))
                .map_err(|_| CodegenError::new("failed to test rhs string nullness"))?;
            let any_null = self
                .builder
                .build_or(lhs_null, rhs_null, &format!("{name}_any_null"))
                .map_err(|_| CodegenError::new("failed to combine string null checks"))?;
            let both_null = self
                .builder
                .build_and(lhs_null, rhs_null, &format!("{name}_both_null"))
                .map_err(|_| CodegenError::new("failed to compute both-null string check"))?;

            let current_fn = self.current_function.ok_or_else(|| {
                CodegenError::new("string equality lowering used outside function")
            })?;
            let strcmp_bb = self
                .context
                .append_basic_block(current_fn, &format!("{name}_strcmp_bb"));
            let merge_bb = self
                .context
                .append_basic_block(current_fn, &format!("{name}_strcmp_merge"));
            let result_ptr = self
                .builder
                .build_alloca(self.context.bool_type(), &format!("{name}_string_eq"))
                .map_err(|_| CodegenError::new("failed to allocate string equality slot"))?;

            self.builder
                .build_store(result_ptr, both_null)
                .map_err(|_| CodegenError::new("failed to initialize string equality slot"))?;
            self.builder
                .build_conditional_branch(any_null, merge_bb, strcmp_bb)
                .map_err(|_| CodegenError::new("failed to branch for string equality"))?;

            self.builder.position_at_end(strcmp_bb);
            let strcmp = self.get_or_declare_strcmp();
            let cmp = self
                .builder
                .build_call(strcmp, &[lhs.into(), rhs.into()], &format!("{name}_strcmp"))
                .map_err(|_| CodegenError::new("failed to emit strcmp call"))?;
            let cmp_v = self
                .extract_call_value_with_context(
                    cmp,
                    "strcmp did not produce a value during string equality lowering",
                )?
                .into_int_value();
            let strcmp_eq = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    cmp_v,
                    self.context.i32_type().const_zero(),
                    &format!("{name}_strcmp_eq"),
                )
                .map_err(|_| CodegenError::new("failed to compare strcmp result"))?;
            self.builder
                .build_store(result_ptr, strcmp_eq)
                .map_err(|_| CodegenError::new("failed to store strcmp equality result"))?;
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|_| CodegenError::new("failed to branch after strcmp"))?;

            self.builder.position_at_end(merge_bb);
            return Ok(self
                .builder
                .build_load(self.context.bool_type(), result_ptr, name)
                .map_err(|_| CodegenError::new("failed to load string equality result"))?
                .into_int_value());
        }
        if lhs.is_pointer_value() && rhs.is_pointer_value() {
            let lhs_int = self
                .builder
                .build_ptr_to_int(
                    lhs.into_pointer_value(),
                    self.context.i64_type(),
                    &format!("{name}_lhs_ptr_int"),
                )
                .map_err(|_| CodegenError::new("failed to cast lhs pointer for equality"))?;
            let rhs_int = self
                .builder
                .build_ptr_to_int(
                    rhs.into_pointer_value(),
                    self.context.i64_type(),
                    &format!("{name}_rhs_ptr_int"),
                )
                .map_err(|_| CodegenError::new("failed to cast rhs pointer for equality"))?;
            return self
                .builder
                .build_int_compare(IntPredicate::EQ, lhs_int, rhs_int, name)
                .map_err(|_| CodegenError::new("failed to compare pointers for equality"));
        }

        if lhs.is_int_value() && rhs.is_int_value() {
            return self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    name,
                )
                .map_err(|_| CodegenError::new("failed to compare integers for equality"));
        }

        if lhs.is_float_value() && rhs.is_float_value() {
            return self
                .builder
                .build_float_compare(
                    inkwell::FloatPredicate::OEQ,
                    lhs.into_float_value(),
                    rhs.into_float_value(),
                    name,
                )
                .map_err(|_| CodegenError::new("failed to compare floats for equality"));
        }

        if lhs.is_pointer_value() && rhs.is_pointer_value() {
            let lhs_i = self
                .builder
                .build_ptr_to_int(
                    lhs.into_pointer_value(),
                    self.context.i64_type(),
                    &format!("{name}_lhs"),
                )
                .map_err(|_| CodegenError::new("failed to cast lhs pointer to integer"))?;
            let rhs_i = self
                .builder
                .build_ptr_to_int(
                    rhs.into_pointer_value(),
                    self.context.i64_type(),
                    &format!("{name}_rhs"),
                )
                .map_err(|_| CodegenError::new("failed to cast rhs pointer to integer"))?;
            return self
                .builder
                .build_int_compare(IntPredicate::EQ, lhs_i, rhs_i, name)
                .map_err(|_| CodegenError::new("failed to compare pointer integers"));
        }

        let llvm_ty = self.llvm_type(ty);
        if llvm_ty.is_struct_type() {
            let lhs_ptr =
                self.materialize_value_pointer_for_type(lhs, ty, &format!("{name}_lhs_tmp"))?;
            let rhs_ptr =
                self.materialize_value_pointer_for_type(rhs, ty, &format!("{name}_rhs_tmp"))?;
            let memcmp = self.get_or_declare_memcmp();
            let size = self
                .context
                .i64_type()
                .const_int(self.storage_size_of_llvm_type(llvm_ty), false);
            let cmp = self
                .builder
                .build_call(
                    memcmp,
                    &[lhs_ptr.into(), rhs_ptr.into(), size.into()],
                    &format!("{name}_memcmp"),
                )
                .map_err(|_| CodegenError::new("failed to emit memcmp call"))?;
            let cmp_v = self
                .extract_call_value_with_context(
                    cmp,
                    "memcmp did not produce a value during structural equality lowering",
                )?
                .into_int_value();
            return self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    cmp_v,
                    self.context.i32_type().const_zero(),
                    name,
                )
                .map_err(|_| CodegenError::new("failed to compare memcmp result"));
        }

        Ok(self.context.bool_type().const_zero())
    }

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
        let llvm_ty = self.llvm_type(ty);
        if llvm_ty.is_struct_type() || llvm_ty.is_array_type() {
            self.builder
                .build_store(alloca, llvm_ty.const_zero())
                .map_err(|_| CodegenError::new("failed to zero temporary value storage"))?;
        }
        self.builder
            .build_store(alloca, value)
            .map_err(|_| CodegenError::new("failed to store temporary value"))?;
        Ok(alloca)
    }

    pub(crate) fn storage_size_of_llvm_type(&self, ty: inkwell::types::BasicTypeEnum<'ctx>) -> u64 {
        let layout_str = CODEGEN_TARGET_DATA_LAYOUT
            .get_or_init(Self::init_codegen_target_data_layout)
            .as_deref();
        if let Some(layout_str) = layout_str {
            let target_data = TargetData::create(layout_str);
            target_data.get_abi_size(&ty)
        } else {
            self.fallback_storage_size_of_llvm_type(ty)
        }
    }

    pub(crate) fn list_element_layout_from_list_type(
        &self,
        list_ty: &Type,
    ) -> (inkwell::types::BasicTypeEnum<'ctx>, u64) {
        if let Type::List(inner) = list_ty {
            let elem_llvm_ty = self.llvm_type(inner);
            let elem_size = self.storage_size_of_llvm_type(elem_llvm_ty);
            return (elem_llvm_ty, elem_size);
        }
        (self.context.i64_type().into(), 8)
    }

    pub(crate) fn list_element_layout_default(&self) -> (inkwell::types::BasicTypeEnum<'ctx>, u64) {
        (self.context.i64_type().into(), 8)
    }

    fn validate_builtin_method_arg_count(
        &self,
        receiver_type: &str,
        method: &str,
        args: &[Spanned<Expr>],
        expected: usize,
    ) -> Result<()> {
        if args.len() != expected {
            return Err(CodegenError::new(format!(
                "{}.{}() expects {} argument(s), got {}",
                receiver_type,
                method,
                expected,
                args.len()
            )));
        }
        Ok(())
    }

    // === Set<T> methods ===

    pub fn compile_set_method(
        &mut self,
        set_name: &str,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let (set_ptr, set_ty) = {
            let var = self
                .variables
                .get(set_name)
                .ok_or_else(|| Self::undefined_variable_error(set_name))?;
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
        match method {
            "length" => self.validate_builtin_method_arg_count("Set", method, args, 0)?,
            "add" | "contains" | "remove" => {
                self.validate_builtin_method_arg_count("Set", method, args, 1)?
            }
            _ => {}
        }
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
                        .map_err(|_| CodegenError::new("failed to compute Set length pointer"))?
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .map_err(|_| CodegenError::new("failed to load Set length"))?;
                Ok(length)
            }
            "add" | "contains" | "remove" => {
                let inner_ty = match self.deref_codegen_type(set_ty) {
                    Type::Set(inner) => &**inner,
                    _ => return Err(CodegenError::new("Expected Set type")),
                };
                let elem_llvm_ty = self.llvm_type(inner_ty);
                let elem_size = self.storage_size_of_llvm_type(elem_llvm_ty);
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
                        .map_err(|_| CodegenError::new("failed to compute Set capacity pointer"))?
                };
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            set_type.as_basic_type_enum(),
                            set_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "set_length_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to compute Set length pointer"))?
                };
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            set_type.as_basic_type_enum(),
                            set_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "set_data_ptr_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to compute Set data pointer"))?
                };
                let needle =
                    self.compile_expr_for_concrete_class_payload(&args[0].node, inner_ty)?;
                let length = self
                    .builder
                    .build_load(i64_type, length_ptr, "set_len")
                    .map_err(|_| CodegenError::new("failed to load Set length"))?
                    .into_int_value();
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "set_data_ptr",
                    )
                    .map_err(|_| CodegenError::new("failed to load Set data pointer"))?
                    .into_pointer_value();
                let idx_ptr = self
                    .builder
                    .build_alloca(i64_type, "set_idx")
                    .map_err(|_| CodegenError::new("failed to allocate Set search index"))?;
                self.builder
                    .build_store(idx_ptr, i64_type.const_zero())
                    .map_err(|_| CodegenError::new("failed to initialize Set search index"))?;
                let found_ptr = self
                    .builder
                    .build_alloca(i64_type, "set_found_idx")
                    .map_err(|_| CodegenError::new("failed to allocate Set found index"))?;
                self.builder
                    .build_store(found_ptr, i64_type.const_all_ones())
                    .map_err(|_| CodegenError::new("failed to initialize Set found index"))?;

                let current_fn = self.current_function.ok_or_else(|| {
                    CodegenError::new("Set.contains/remove used outside function")
                })?;
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

                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::new("failed to branch into Set search loop"))?;
                self.builder.position_at_end(cond_bb);
                let idx = self
                    .builder
                    .build_load(i64_type, idx_ptr, "set_idx_val")
                    .map_err(|_| CodegenError::new("failed to load Set search index"))?
                    .into_int_value();
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::ULT, idx, length, "set_idx_in_bounds")
                    .map_err(|_| CodegenError::new("failed to compare Set search bounds"))?;
                self.builder
                    .build_conditional_branch(in_bounds, body_bb, done_bb)
                    .map_err(|_| CodegenError::new("failed to branch in Set search loop"))?;

                self.builder.position_at_end(body_bb);
                let offset = self
                    .builder
                    .build_int_mul(idx, i64_type.const_int(elem_size, false), "set_offset")
                    .map_err(|_| CodegenError::new("failed to compute Set element offset"))?;
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "set_elem_ptr")
                        .map_err(|_| CodegenError::new("failed to compute Set element pointer"))?
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "set_typed_elem_ptr",
                    )
                    .map_err(|_| CodegenError::new("failed to cast Set element pointer"))?;
                let existing = self
                    .builder
                    .build_load(elem_llvm_ty, typed_elem_ptr, "set_existing")
                    .map_err(|_| CodegenError::new("failed to load Set element"))?;
                let eq = self.build_value_equality(existing, needle, inner_ty, "set_eq")?;
                let next_bb = self
                    .context
                    .append_basic_block(current_fn, "set_search.next");
                self.builder
                    .build_conditional_branch(eq, found_bb, next_bb)
                    .map_err(|_| CodegenError::new("failed to branch on Set equality"))?;

                self.builder.position_at_end(found_bb);
                self.builder
                    .build_store(found_ptr, idx)
                    .map_err(|_| CodegenError::new("failed to store Set found index"))?;
                self.builder
                    .build_unconditional_branch(done_bb)
                    .map_err(|_| CodegenError::new("failed to branch after Set match"))?;

                self.builder.position_at_end(next_bb);
                let next_idx = self
                    .builder
                    .build_int_add(idx, i64_type.const_int(1, false), "set_next_idx")
                    .map_err(|_| CodegenError::new("failed to increment Set search index"))?;
                self.builder
                    .build_store(idx_ptr, next_idx)
                    .map_err(|_| CodegenError::new("failed to store Set search index"))?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::new("failed to continue Set search loop"))?;

                self.builder.position_at_end(done_bb);
                let found_idx = self
                    .builder
                    .build_load(i64_type, found_ptr, "set_found_idx_val")
                    .map_err(|_| CodegenError::new("failed to load Set found index"))?
                    .into_int_value();
                let found = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        found_idx,
                        i64_type.const_all_ones(),
                        "set_found",
                    )
                    .map_err(|_| CodegenError::new("failed to compute Set found flag"))?;

                match method {
                    "contains" => Ok(found.into()),
                    "add" => {
                        let append_bb = self
                            .context
                            .append_basic_block(current_fn, "set_add.append");
                        let merge_bb = self.context.append_basic_block(current_fn, "set_add.merge");
                        self.builder
                            .build_conditional_branch(found, merge_bb, append_bb)
                            .map_err(|_| CodegenError::new("failed to branch for Set add"))?;

                        self.builder.position_at_end(append_bb);
                        let capacity = self
                            .builder
                            .build_load(i64_type, capacity_ptr, "set_capacity")
                            .map_err(|_| CodegenError::new("failed to load Set capacity"))?
                            .into_int_value();
                        let need_growth = self
                            .builder
                            .build_int_compare(
                                IntPredicate::UGE,
                                length,
                                capacity,
                                "set_need_growth",
                            )
                            .map_err(|_| CodegenError::new("failed to compare Set growth need"))?;
                        let grow_bb = self.context.append_basic_block(current_fn, "set_add.grow");
                        let store_bb = self.context.append_basic_block(current_fn, "set_add.store");
                        self.builder
                            .build_conditional_branch(need_growth, grow_bb, store_bb)
                            .map_err(|_| CodegenError::new("failed to branch for Set growth"))?;

                        self.builder.position_at_end(grow_bb);
                        let realloc = self.get_or_declare_realloc();
                        let grown_capacity = self
                            .builder
                            .build_int_mul(
                                capacity,
                                i64_type.const_int(2, false),
                                "set_grown_capacity",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compute Set grown capacity")
                            })?;
                        let new_size = self
                            .builder
                            .build_int_mul(
                                grown_capacity,
                                i64_type.const_int(elem_size, false),
                                "set_new_size",
                            )
                            .map_err(|_| CodegenError::new("failed to compute Set growth size"))?;
                        let grown_call = self
                            .builder
                            .build_call(
                                realloc,
                                &[data_ptr.into(), new_size.into()],
                                "set_grown_ptr",
                            )
                            .map_err(|_| CodegenError::new("failed to emit Set realloc call"))?;
                        let grown_ptr = self.extract_call_pointer_value(
                            grown_call,
                            "realloc failed for Set growth",
                        )?;
                        self.builder
                            .build_store(data_ptr_ptr, grown_ptr)
                            .map_err(|_| {
                                CodegenError::new("failed to store grown Set data pointer")
                            })?;
                        self.builder
                            .build_store(capacity_ptr, grown_capacity)
                            .map_err(|_| CodegenError::new("failed to store grown Set capacity"))?;
                        self.builder
                            .build_unconditional_branch(store_bb)
                            .map_err(|_| CodegenError::new("failed to branch after Set growth"))?;

                        self.builder.position_at_end(store_bb);
                        let active_data_ptr = self
                            .builder
                            .build_load(
                                self.context.ptr_type(AddressSpace::default()),
                                data_ptr_ptr,
                                "set_active_data_ptr",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to load active Set data pointer")
                            })?
                            .into_pointer_value();
                        let offset = self
                            .builder
                            .build_int_mul(
                                length,
                                i64_type.const_int(elem_size, false),
                                "set_append_offset",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compute Set append offset")
                            })?;
                        let elem_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    self.context.i8_type(),
                                    active_data_ptr,
                                    &[offset],
                                    "set_append_ptr",
                                )
                                .map_err(|_| {
                                    CodegenError::new("failed to compute Set append pointer")
                                })?
                        };
                        let typed_elem_ptr = self
                            .builder
                            .build_pointer_cast(
                                elem_ptr,
                                self.context.ptr_type(AddressSpace::default()),
                                "set_append_typed_ptr",
                            )
                            .map_err(|_| CodegenError::new("failed to cast Set append pointer"))?;
                        if elem_llvm_ty.is_struct_type() || elem_llvm_ty.is_array_type() {
                            self.builder
                                .build_store(typed_elem_ptr, elem_llvm_ty.const_zero())
                                .map_err(|_| CodegenError::new("failed to zero Set append slot"))?;
                        }
                        self.builder
                            .build_store(typed_elem_ptr, needle)
                            .map_err(|_| {
                                CodegenError::new("failed to store appended Set element")
                            })?;
                        let new_length = self
                            .builder
                            .build_int_add(length, i64_type.const_int(1, false), "set_new_length")
                            .map_err(|_| CodegenError::new("failed to compute Set new length"))?;
                        self.builder
                            .build_store(length_ptr, new_length)
                            .map_err(|_| CodegenError::new("failed to store Set new length"))?;
                        self.builder
                            .build_unconditional_branch(merge_bb)
                            .map_err(|_| CodegenError::new("failed to branch after Set append"))?;

                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(self.context.bool_type(), "set_add_phi")
                            .map_err(|_| CodegenError::new("failed to create Set add phi"))?;
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
                            .map_err(|_| CodegenError::new("failed to branch for Set remove"))?;

                        self.builder.position_at_end(remove_bb);
                        let last_idx = self
                            .builder
                            .build_int_sub(length, i64_type.const_int(1, false), "set_last_idx")
                            .map_err(|_| CodegenError::new("failed to compute Set last index"))?;
                        let remove_is_last = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                found_idx,
                                last_idx,
                                "set_remove_is_last",
                            )
                            .map_err(|_| CodegenError::new("failed to compare Set remove index"))?;
                        let shift_bb = self
                            .context
                            .append_basic_block(current_fn, "set_remove.shift");
                        let shrink_bb = self
                            .context
                            .append_basic_block(current_fn, "set_remove.shrink");
                        self.builder
                            .build_conditional_branch(remove_is_last, shrink_bb, shift_bb)
                            .map_err(|_| CodegenError::new("failed to branch inside Set remove"))?;

                        self.builder.position_at_end(shift_bb);
                        let src_offset = self
                            .builder
                            .build_int_mul(
                                last_idx,
                                i64_type.const_int(elem_size, false),
                                "set_src_offset",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compute Set source offset")
                            })?;
                        let dst_offset = self
                            .builder
                            .build_int_mul(
                                found_idx,
                                i64_type.const_int(elem_size, false),
                                "set_dst_offset",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compute Set destination offset")
                            })?;
                        let src_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    self.context.i8_type(),
                                    data_ptr,
                                    &[src_offset],
                                    "set_src_ptr",
                                )
                                .map_err(|_| {
                                    CodegenError::new("failed to compute Set source pointer")
                                })?
                        };
                        let dst_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    self.context.i8_type(),
                                    data_ptr,
                                    &[dst_offset],
                                    "set_dst_ptr",
                                )
                                .map_err(|_| {
                                    CodegenError::new("failed to compute Set destination pointer")
                                })?
                        };
                        let typed_src_ptr = self
                            .builder
                            .build_pointer_cast(
                                src_ptr,
                                self.context.ptr_type(AddressSpace::default()),
                                "set_typed_src_ptr",
                            )
                            .map_err(|_| CodegenError::new("failed to cast Set source pointer"))?;
                        let typed_dst_ptr = self
                            .builder
                            .build_pointer_cast(
                                dst_ptr,
                                self.context.ptr_type(AddressSpace::default()),
                                "set_typed_dst_ptr",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to cast Set destination pointer")
                            })?;
                        let last_value = self
                            .builder
                            .build_load(elem_llvm_ty, typed_src_ptr, "set_last_value")
                            .map_err(|_| CodegenError::new("failed to load Set tail value"))?;
                        self.builder
                            .build_store(typed_dst_ptr, last_value)
                            .map_err(|_| CodegenError::new("failed to store shifted Set value"))?;
                        self.builder
                            .build_unconditional_branch(shrink_bb)
                            .map_err(|_| CodegenError::new("failed to branch to Set shrink"))?;

                        self.builder.position_at_end(shrink_bb);
                        let new_length = self
                            .builder
                            .build_int_sub(
                                length,
                                i64_type.const_int(1, false),
                                "set_removed_length",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compute Set length after removal")
                            })?;
                        self.builder
                            .build_store(length_ptr, new_length)
                            .map_err(|_| {
                                CodegenError::new("failed to store Set length after removal")
                            })?;
                        self.builder
                            .build_unconditional_branch(merge_bb)
                            .map_err(|_| CodegenError::new("failed to branch after Set shrink"))?;

                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(self.context.bool_type(), "set_remove_phi")
                            .map_err(|_| CodegenError::new("failed to create Set remove phi"))?;
                        phi.add_incoming(&[
                            (&self.context.bool_type().const_zero(), done_bb),
                            (&self.context.bool_type().const_int(1, false), shrink_bb),
                        ]);
                        Ok(phi.as_basic_value())
                    }
                    _ => Err(CodegenError::new(format!("Unknown Set method: {}", method))),
                }
            }
            _ => Err(CodegenError::new(format!("Unknown Set method: {}", method))),
        }
    }

    // === Option<T> methods ===

    pub fn compile_option_method_on_value(
        &mut self,
        option_value: BasicValueEnum<'ctx>,
        option_expr_ty: &Type,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        match method {
            "is_some" | "is_none" | "unwrap" => {
                self.validate_builtin_method_arg_count("Option", method, args, 0)?
            }
            _ => {}
        }
        let option_ptr =
            self.materialize_value_pointer_for_type(option_value, option_expr_ty, "option_tmp")?;
        // Assuming Option<T> is { is_some: i8, value: T }
        // We need to infer T from var.ty
        let option_ty = match self.deref_codegen_type(option_expr_ty) {
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
                        .map_err(|_| CodegenError::new("failed to compute Option tag pointer"))?
                };
                let is_some = self
                    .builder
                    .build_load(self.context.i8_type(), is_some_ptr, "is_some")
                    .map_err(|_| CodegenError::new("failed to load Option tag"))?
                    .into_int_value();
                let is_some_bool = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        is_some,
                        self.context.i8_type().const_int(0, false),
                        "is_some_bool",
                    )
                    .map_err(|_| CodegenError::new("failed to compare Option tag"))?;
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
                        .map_err(|_| CodegenError::new("failed to compute Option tag pointer"))?
                };
                let is_some = self
                    .builder
                    .build_load(self.context.i8_type(), is_some_ptr, "is_some")
                    .map_err(|_| CodegenError::new("failed to load Option tag"))?
                    .into_int_value();
                let is_none = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        is_some,
                        self.context.i8_type().const_int(0, false),
                        "is_none",
                    )
                    .map_err(|_| CodegenError::new("failed to compare Option none tag"))?;
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
                        .map_err(|_| CodegenError::new("failed to compute Option tag pointer"))?
                };
                let is_some = self
                    .builder
                    .build_load(self.context.i8_type(), is_some_ptr, "is_some")
                    .map_err(|_| CodegenError::new("failed to load Option tag"))?
                    .into_int_value();
                let is_some_bool = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        is_some,
                        self.context.i8_type().const_int(0, false),
                        "is_some_bool",
                    )
                    .map_err(|_| CodegenError::new("failed to compare Option unwrap tag"))?;
                self.builder
                    .build_conditional_branch(is_some_bool, ok_bb, panic_bb)
                    .map_err(|_| CodegenError::new("failed to branch for Option.unwrap"))?;

                self.builder.position_at_end(panic_bb);
                self.emit_runtime_error("Option.unwrap() called on None", "opt_unwrap_panic")?;

                self.builder.position_at_end(ok_bb);
                let value_ptr = unsafe {
                    self.builder
                        .build_gep(
                            option_struct_type.as_basic_type_enum(),
                            option_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "value_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to compute Option value pointer"))?
                };
                let value = self
                    .builder
                    .build_load(llvm_inner_ty, value_ptr, "unwrapped_value")
                    .map_err(|_| CodegenError::new("failed to load unwrapped Option value"))?;
                Ok(value)
            }
            _ => Err(CodegenError::new(format!(
                "Unknown Option method: {}",
                method
            ))),
        }
    }

    // === Result<T, E> methods ===

    pub fn compile_result_method_on_value(
        &mut self,
        result_value: BasicValueEnum<'ctx>,
        result_expr_ty: &Type,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        match method {
            "is_ok" | "is_error" | "unwrap" => {
                self.validate_builtin_method_arg_count("Result", method, args, 0)?
            }
            _ => {}
        }
        let result_ptr =
            self.materialize_value_pointer_for_type(result_value, result_expr_ty, "result_tmp")?;
        // Result<T, E> is struct { is_ok: i8, ok_value: T, err_value: E }
        let (ok_ty, err_ty) = match self.deref_codegen_type(result_expr_ty) {
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
                        .map_err(|_| CodegenError::new("failed to compute Result tag pointer"))?
                };
                let tag = self
                    .builder
                    .build_load(self.context.i8_type(), tag_ptr, "tag")
                    .map_err(|_| CodegenError::new("failed to load Result tag"))?
                    .into_int_value();
                let is_ok = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        tag,
                        self.context.i8_type().const_int(0, false),
                        "is_ok",
                    )
                    .map_err(|_| CodegenError::new("failed to compare Result ok tag"))?;
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
                        .map_err(|_| CodegenError::new("failed to compute Result tag pointer"))?
                };
                let tag = self
                    .builder
                    .build_load(self.context.i8_type(), tag_ptr, "tag")
                    .map_err(|_| CodegenError::new("failed to load Result tag"))?
                    .into_int_value();
                let is_error = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        tag,
                        self.context.i8_type().const_int(0, false),
                        "is_error",
                    )
                    .map_err(|_| CodegenError::new("failed to compare Result error tag"))?;
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
                        .map_err(|_| CodegenError::new("failed to compute Result tag pointer"))?
                };
                let tag = self
                    .builder
                    .build_load(self.context.i8_type(), tag_ptr, "tag")
                    .map_err(|_| CodegenError::new("failed to load Result tag"))?
                    .into_int_value();
                let is_ok = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        tag,
                        self.context.i8_type().const_int(0, false),
                        "is_ok",
                    )
                    .map_err(|_| CodegenError::new("failed to compare Result unwrap tag"))?;
                self.builder
                    .build_conditional_branch(is_ok, ok_bb, panic_bb)
                    .map_err(|_| CodegenError::new("failed to branch for Result.unwrap"))?;

                self.builder.position_at_end(panic_bb);
                self.emit_runtime_error("Result.unwrap() called on Error", "res_unwrap_panic")?;

                self.builder.position_at_end(ok_bb);
                let ok_ptr = unsafe {
                    self.builder
                        .build_gep(
                            result_struct_type.as_basic_type_enum(),
                            result_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "ok_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to compute Result ok pointer"))?
                };
                let value = self
                    .builder
                    .build_load(ok_llvm, ok_ptr, "unwrapped_ok")
                    .map_err(|_| CodegenError::new("failed to load unwrapped Result value"))?;
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
        let option_type = self
            .context
            .struct_type(&[self.context.i8_type().into(), value.get_type()], false);
        let alloca = self
            .builder
            .build_alloca(option_type, "option")
            .map_err(|_| CodegenError::new("failed to allocate Option.some storage"))?;

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
                .map_err(|_| CodegenError::new("failed to compute Option.some tag pointer"))?
        };
        self.builder
            .build_store(tag_ptr, self.context.i8_type().const_int(1, false))
            .map_err(|_| CodegenError::new("failed to store Option.some tag"))?;

        let value_ptr = unsafe {
            self.builder
                .build_gep(
                    option_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "value",
                )
                .map_err(|_| CodegenError::new("failed to compute Option.some value pointer"))?
        };
        self.builder
            .build_store(value_ptr, value)
            .map_err(|_| CodegenError::new("failed to store Option.some value"))?;

        self.builder
            .build_load(option_type, alloca, "option")
            .map_err(|_| CodegenError::new("failed to load Option.some value"))
    }

    pub fn create_option_some_typed(
        &mut self,
        value: BasicValueEnum<'ctx>,
        inner_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let option_type = self.context.struct_type(
            &[self.context.i8_type().into(), self.llvm_type(inner_ty)],
            false,
        );
        let alloca = self
            .builder
            .build_alloca(option_type, "option")
            .map_err(|_| CodegenError::new("failed to allocate typed Option.some storage"))?;

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
                .map_err(|_| CodegenError::new("failed to compute typed Option.some tag pointer"))?
        };
        self.builder
            .build_store(tag_ptr, self.context.i8_type().const_int(1, false))
            .map_err(|_| CodegenError::new("failed to store typed Option.some tag"))?;

        // Set value
        let value_ptr = unsafe {
            self.builder
                .build_gep(
                    option_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "value",
                )
                .map_err(|_| {
                    CodegenError::new("failed to compute typed Option.some value pointer")
                })?
        };
        self.builder
            .build_store(value_ptr, value)
            .map_err(|_| CodegenError::new("failed to store typed Option.some value"))?;

        self.builder
            .build_load(option_type, alloca, "option")
            .map_err(|_| CodegenError::new("failed to load typed Option.some value"))
    }

    pub fn create_option_none(&mut self) -> Result<BasicValueEnum<'ctx>> {
        self.create_option_none_typed(&Type::Integer)
    }

    pub fn create_option_none_typed(&mut self, inner_ty: &Type) -> Result<BasicValueEnum<'ctx>> {
        let inner_llvm = self.llvm_type(inner_ty);
        let option_type = self
            .context
            .struct_type(&[self.context.i8_type().into(), inner_llvm], false);
        let alloca = self
            .builder
            .build_alloca(option_type, "option")
            .map_err(|_| CodegenError::new("failed to allocate Option.none storage"))?;

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
                .map_err(|_| CodegenError::new("failed to compute Option.none tag pointer"))?
        };
        self.builder
            .build_store(tag_ptr, self.context.i8_type().const_int(0, false))
            .map_err(|_| CodegenError::new("failed to store Option.none tag"))?;

        // Set value to 0 (unused)
        let value_ptr = unsafe {
            self.builder
                .build_gep(
                    option_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "value",
                )
                .map_err(|_| CodegenError::new("failed to compute Option.none value pointer"))?
        };
        self.builder
            .build_store(value_ptr, inner_llvm.const_zero())
            .map_err(|_| CodegenError::new("failed to store Option.none default value"))?;

        self.builder
            .build_load(option_type, alloca, "option")
            .map_err(|_| CodegenError::new("failed to load Option.none value"))
    }

    // === Result<T, E> helpers ===

    pub fn create_result_ok(
        &mut self,
        value: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let result_type = self.context.struct_type(
            &[
                self.context.i8_type().into(),
                value.get_type(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );
        let alloca = self
            .builder
            .build_alloca(result_type, "result")
            .map_err(|_| CodegenError::new("failed to allocate Result.ok storage"))?;

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
                .map_err(|_| CodegenError::new("failed to compute Result.ok tag pointer"))?
        };
        self.builder
            .build_store(tag_ptr, self.context.i8_type().const_int(1, false))
            .map_err(|_| CodegenError::new("failed to store Result.ok tag"))?;

        let ok_ptr = unsafe {
            self.builder
                .build_gep(
                    result_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "ok",
                )
                .map_err(|_| CodegenError::new("failed to compute Result.ok payload pointer"))?
        };
        self.builder
            .build_store(ok_ptr, value)
            .map_err(|_| CodegenError::new("failed to store Result.ok payload"))?;

        let err_ptr = unsafe {
            self.builder
                .build_gep(
                    result_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(2, false)],
                    "err",
                )
                .map_err(|_| CodegenError::new("failed to compute Result.err pointer"))?
        };
        let null = self.context.ptr_type(AddressSpace::default()).const_null();
        self.builder
            .build_store(err_ptr, null)
            .map_err(|_| CodegenError::new("failed to store Result.ok default error"))?;

        self.builder
            .build_load(result_type, alloca, "result")
            .map_err(|_| CodegenError::new("failed to load Result.ok value"))
    }

    pub fn create_result_ok_typed(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ok_ty: &Type,
        err_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let ok_llvm = self.llvm_type(ok_ty);
        let err_llvm = self.llvm_type(err_ty);
        let result_type = self
            .context
            .struct_type(&[self.context.i8_type().into(), ok_llvm, err_llvm], false);
        let alloca = self
            .builder
            .build_alloca(result_type, "result")
            .map_err(|_| CodegenError::new("failed to allocate typed Result.ok storage"))?;

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
                .map_err(|_| CodegenError::new("failed to compute typed Result.ok tag pointer"))?
        };
        self.builder
            .build_store(tag_ptr, self.context.i8_type().const_int(1, false))
            .map_err(|_| CodegenError::new("failed to store typed Result.ok tag"))?;

        // Set ok_value
        let ok_ptr = unsafe {
            self.builder
                .build_gep(
                    result_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "ok",
                )
                .map_err(|_| CodegenError::new("failed to compute typed Result.ok pointer"))?
        };
        self.builder
            .build_store(ok_ptr, value)
            .map_err(|_| CodegenError::new("failed to store typed Result.ok value"))?;

        // Set err_value to null
        let err_ptr = unsafe {
            self.builder
                .build_gep(
                    result_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(2, false)],
                    "err",
                )
                .map_err(|_| CodegenError::new("failed to compute typed Result.err pointer"))?
        };
        self.builder
            .build_store(err_ptr, err_llvm.const_zero())
            .map_err(|_| CodegenError::new("failed to store typed Result.ok error default"))?;

        self.builder
            .build_load(result_type, alloca, "result")
            .map_err(|_| CodegenError::new("failed to load typed Result.ok value"))
    }

    pub fn create_result_error(
        &mut self,
        error: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        self.create_result_error_typed(error, &Type::Integer, &Type::String)
    }

    pub fn create_result_error_typed(
        &mut self,
        error: BasicValueEnum<'ctx>,
        ok_ty: &Type,
        err_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let ok_llvm = self.llvm_type(ok_ty);
        let err_llvm = self.llvm_type(err_ty);
        let result_type = self
            .context
            .struct_type(&[self.context.i8_type().into(), ok_llvm, err_llvm], false);
        let alloca = self
            .builder
            .build_alloca(result_type, "result")
            .map_err(|_| CodegenError::new("failed to allocate Result.error storage"))?;

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
                .map_err(|_| CodegenError::new("failed to compute Result.error tag pointer"))?
        };
        self.builder
            .build_store(tag_ptr, self.context.i8_type().const_int(0, false))
            .map_err(|_| CodegenError::new("failed to store Result.error tag"))?;

        // Set ok_value to 0
        let ok_ptr = unsafe {
            self.builder
                .build_gep(
                    result_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "ok",
                )
                .map_err(|_| CodegenError::new("failed to compute Result.error ok pointer"))?
        };
        self.builder
            .build_store(ok_ptr, ok_llvm.const_zero())
            .map_err(|_| CodegenError::new("failed to store Result.error ok default"))?;

        // Set err_value
        let err_ptr = unsafe {
            self.builder
                .build_gep(
                    result_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(2, false)],
                    "err",
                )
                .map_err(|_| CodegenError::new("failed to compute Result.error payload pointer"))?
        };
        self.builder
            .build_store(err_ptr, error)
            .map_err(|_| CodegenError::new("failed to store Result.error payload"))?;

        self.builder
            .build_load(result_type, alloca, "result")
            .map_err(|_| CodegenError::new("failed to load Result.error value"))
    }

    fn create_default_result_typed_with_guard(
        &mut self,
        ok_ty: &Type,
        err_ty: &Type,
        visited_classes: &mut HashSet<String>,
    ) -> Result<BasicValueEnum<'ctx>> {
        // Result is struct { is_ok: i8, ok_value: i64, err_value: ptr }
        // We default to Error (tag=0) with null pointer
        let ok_llvm = self.llvm_type(ok_ty);
        let err_llvm = self.llvm_type(err_ty);
        let result_type = self
            .context
            .struct_type(&[self.context.i8_type().into(), ok_llvm, err_llvm], false);

        let alloca = self
            .builder
            .build_alloca(result_type, "default_result")
            .map_err(|_| CodegenError::new("failed to allocate default Result storage"))?;

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
                .map_err(|_| CodegenError::new("failed to compute default Result tag pointer"))?
        };
        self.builder
            .build_store(tag_ptr, self.context.i8_type().const_int(0, false))
            .map_err(|_| CodegenError::new("failed to store default Result tag"))?;

        // Set ok_value to 0
        let ok_ptr = unsafe {
            self.builder
                .build_gep(
                    result_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "ok",
                )
                .map_err(|_| CodegenError::new("failed to compute default Result ok pointer"))?
        };
        self.builder
            .build_store(ok_ptr, ok_llvm.const_zero())
            .map_err(|_| CodegenError::new("failed to store default Result ok value"))?;

        // Set err_value to a safe default for the active Error variant.
        let err_ptr = unsafe {
            self.builder
                .build_gep(
                    result_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(2, false)],
                    "err",
                )
                .map_err(|_| CodegenError::new("failed to compute default Result err pointer"))?
        };
        let default_err_value =
            self.create_default_value_for_type_with_guard(err_ty, visited_classes)?;
        self.builder
            .build_store(err_ptr, default_err_value)
            .map_err(|_| CodegenError::new("failed to store default Result err value"))?;

        self.builder
            .build_load(result_type, alloca, "result")
            .map_err(|_| CodegenError::new("failed to load default Result value"))
    }

    pub fn create_default_result_typed(
        &mut self,
        ok_ty: &Type,
        err_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let mut visited_classes = HashSet::new();
        self.create_default_result_typed_with_guard(ok_ty, err_ty, &mut visited_classes)
    }

    // === List<T> helpers ===

    pub fn create_list_with_capacity_value(
        &mut self,
        requested_capacity: IntValue<'ctx>,
        list_ty: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let current_fn = self
            .current_function
            .ok_or_else(|| CodegenError::new("List constructor capacity used outside function"))?;
        let (_, elem_size) = if let Some(list_ty) = list_ty {
            self.list_element_layout_from_list_type(list_ty)
        } else {
            self.list_element_layout_default()
        };

        let list_type = self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );
        let alloca = self
            .builder
            .build_alloca(list_type, "list")
            .map_err(|_| CodegenError::new("failed to allocate List storage"))?;
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let zero = i32_type.const_zero();

        let negative_bb = self
            .context
            .append_basic_block(current_fn, "list_ctor_negative_capacity");
        let valid_bb = self
            .context
            .append_basic_block(current_fn, "list_ctor_capacity_valid");
        let is_negative = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                requested_capacity,
                i64_type.const_zero(),
                "list_ctor_capacity_negative",
            )
            .map_err(|_| CodegenError::new("failed to compare List constructor capacity"))?;
        self.builder
            .build_conditional_branch(is_negative, negative_bb, valid_bb)
            .map_err(|_| CodegenError::new("failed to branch on List constructor capacity"))?;

        self.builder.position_at_end(negative_bb);
        self.emit_runtime_error(
            "List constructor capacity cannot be negative",
            "list_ctor_negative_capacity_runtime_error",
        )?;

        self.builder.position_at_end(valid_bb);
        let use_default_capacity = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                requested_capacity,
                i64_type.const_zero(),
                "list_ctor_capacity_is_zero",
            )
            .map_err(|_| CodegenError::new("failed to test zero List constructor capacity"))?;
        let effective_capacity = self
            .builder
            .build_select(
                use_default_capacity,
                i64_type.const_int(8, false),
                requested_capacity,
                "list_ctor_effective_capacity",
            )
            .map_err(|_| CodegenError::new("failed to select effective List capacity"))?
            .into_int_value();

        let capacity_ptr = unsafe {
            self.builder
                .build_gep(
                    list_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_zero()],
                    "capacity",
                )
                .map_err(|_| CodegenError::new("failed to compute List capacity pointer"))?
        };
        self.builder
            .build_store(capacity_ptr, effective_capacity)
            .map_err(|_| CodegenError::new("failed to store List capacity"))?;

        let length_ptr = unsafe {
            self.builder
                .build_gep(
                    list_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "length",
                )
                .map_err(|_| CodegenError::new("failed to compute List length pointer"))?
        };
        self.builder
            .build_store(length_ptr, i64_type.const_zero())
            .map_err(|_| CodegenError::new("failed to initialize List length"))?;

        let malloc = self.get_or_declare_malloc();
        let total_size = self
            .builder
            .build_int_mul(
                effective_capacity,
                i64_type.const_int(elem_size, false),
                "list_ctor_total_size",
            )
            .map_err(|_| CodegenError::new("failed to compute List allocation size"))?;
        let data_call = self
            .builder
            .build_call(malloc, &[total_size.into()], "list_ctor_data")
            .map_err(|_| CodegenError::new("failed to emit List malloc call"))?;
        let data_ptr = self.extract_call_pointer_value(
            data_call,
            "malloc did not produce a pointer while allocating list constructor storage",
        )?;
        let data_ptr_field = unsafe {
            self.builder
                .build_gep(
                    list_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(2, false)],
                    "data_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute List data pointer field"))?
        };
        self.builder
            .build_store(data_ptr_field, data_ptr)
            .map_err(|_| CodegenError::new("failed to store List data pointer"))?;

        self.builder
            .build_load(list_type, alloca, "list")
            .map_err(|_| CodegenError::new("failed to load constructed List"))
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

        let alloca = self
            .builder
            .build_alloca(list_type, "list")
            .map_err(|_| CodegenError::new("failed to allocate empty List storage"))?;

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
                .map_err(|_| CodegenError::new("failed to compute empty List capacity pointer"))?
        };
        self.builder
            .build_store(
                capacity_ptr,
                self.context.i64_type().const_int(initial_capacity, false),
            )
            .map_err(|_| CodegenError::new("failed to store empty List capacity"))?;

        // Length = 0
        let length_ptr = unsafe {
            self.builder
                .build_gep(
                    list_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "length",
                )
                .map_err(|_| CodegenError::new("failed to compute empty List length pointer"))?
        };
        self.builder
            .build_store(length_ptr, self.context.i64_type().const_int(0, false))
            .map_err(|_| CodegenError::new("failed to initialize empty List length"))?;

        // Allocate data - malloc(capacity * 8) for i64 elements
        let malloc = self.get_or_declare_malloc();
        let size = self
            .context
            .i64_type()
            .const_int(initial_capacity * elem_size, false);
        let call_result = self
            .builder
            .build_call(malloc, &[size.into()], "data")
            .map_err(|_| CodegenError::new("failed to emit empty List malloc call"))?;
        let data_ptr = self.extract_call_value_with_context(
            call_result,
            "malloc did not produce a value while allocating list storage",
        )?;

        let data_ptr_field = unsafe {
            self.builder
                .build_gep(
                    list_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(2, false)],
                    "data_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute empty List data pointer field"))?
        };
        self.builder
            .build_store(data_ptr_field, data_ptr)
            .map_err(|_| CodegenError::new("failed to store empty List data pointer"))?;

        self.builder
            .build_load(list_type, alloca, "list")
            .map_err(|_| CodegenError::new("failed to load empty List"))
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
            .map_err(|_| CodegenError::new("failed to compute grown List capacity"))?;
        let new_size = self
            .builder
            .build_int_mul(
                new_capacity,
                i64_type.const_int(elem_size, false),
                "new_size",
            )
            .map_err(|_| CodegenError::new("failed to compute grown List allocation size"))?;
        let old_data = self
            .builder
            .build_load(
                self.context.ptr_type(AddressSpace::default()),
                data_ptr_ptr,
                "old_data",
            )
            .map_err(|_| CodegenError::new("failed to load old List data pointer"))?
            .into_pointer_value();

        let malloc = self.get_or_declare_malloc();
        let grown_call = self
            .builder
            .build_call(malloc, &[new_size.into()], "grown_data")
            .map_err(|_| CodegenError::new("failed to emit List growth malloc call"))?;
        let grown_data = self.extract_call_pointer_value(
            grown_call,
            "malloc did not produce a pointer while growing list storage",
        )?;

        let bytes_to_copy = self
            .builder
            .build_int_mul(length, i64_type.const_int(elem_size, false), "copy_bytes")
            .map_err(|_| CodegenError::new("failed to compute List copy byte count"))?;
        let has_bytes = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                bytes_to_copy,
                i64_type.const_zero(),
                "has_copy_bytes",
            )
            .map_err(|_| CodegenError::new("failed to compare List copy byte count"))?;

        let copy_cond_bb = self.context.append_basic_block(function, "list_copy_cond");
        let copy_body_bb = self.context.append_basic_block(function, "list_copy_body");
        let copy_done_bb = self.context.append_basic_block(function, "list_copy_done");
        self.builder
            .build_conditional_branch(has_bytes, copy_cond_bb, copy_done_bb)
            .map_err(|_| CodegenError::new("failed to branch for List copy"))?;

        self.builder.position_at_end(copy_cond_bb);
        let idx_ptr = self
            .builder
            .build_alloca(i64_type, "copy_idx")
            .map_err(|_| CodegenError::new("failed to allocate List copy index"))?;
        self.builder
            .build_store(idx_ptr, i64_type.const_zero())
            .map_err(|_| CodegenError::new("failed to initialize List copy index"))?;
        let cond_bb = self
            .context
            .append_basic_block(function, "list_copy_loop_cond");
        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::new("failed to enter List copy loop"))?;

        self.builder.position_at_end(cond_bb);
        let idx = self
            .builder
            .build_load(i64_type, idx_ptr, "copy_idx_val")
            .map_err(|_| CodegenError::new("failed to load List copy index"))?
            .into_int_value();
        let keep_copying = self
            .builder
            .build_int_compare(IntPredicate::SLT, idx, bytes_to_copy, "copy_continue")
            .map_err(|_| CodegenError::new("failed to compare List copy loop bound"))?;
        self.builder
            .build_conditional_branch(keep_copying, copy_body_bb, copy_done_bb)
            .map_err(|_| CodegenError::new("failed to branch in List copy loop"))?;

        self.builder.position_at_end(copy_body_bb);
        let src = unsafe {
            self.builder
                .build_gep(self.context.i8_type(), old_data, &[idx], "copy_src")
                .map_err(|_| CodegenError::new("failed to compute List copy source pointer"))?
        };
        let dst = unsafe {
            self.builder
                .build_gep(self.context.i8_type(), grown_data, &[idx], "copy_dst")
                .map_err(|_| CodegenError::new("failed to compute List copy destination pointer"))?
        };
        let byte = self
            .builder
            .build_load(self.context.i8_type(), src, "copy_byte")
            .map_err(|_| CodegenError::new("failed to load copied List byte"))?;
        self.builder
            .build_store(dst, byte)
            .map_err(|_| CodegenError::new("failed to store copied List byte"))?;
        let next_idx = self
            .builder
            .build_int_add(idx, i64_type.const_int(1, false), "copy_next_idx")
            .map_err(|_| CodegenError::new("failed to increment List copy index"))?;
        self.builder
            .build_store(idx_ptr, next_idx)
            .map_err(|_| CodegenError::new("failed to store List copy index"))?;
        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::new("failed to continue List copy loop"))?;

        self.builder.position_at_end(copy_done_bb);
        self.builder
            .build_store(data_ptr_ptr, grown_data)
            .map_err(|_| CodegenError::new("failed to store grown List data pointer"))?;
        self.builder
            .build_store(capacity_ptr, new_capacity)
            .map_err(|_| CodegenError::new("failed to store grown List capacity"))?;

        Ok(())
    }

    // === Map<K,V> helpers ===

    pub fn create_empty_map_for_type(
        &mut self,
        map_expr_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
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

        let alloca = self
            .builder
            .build_alloca(map_type, "map")
            .map_err(|_| CodegenError::new("failed to allocate Map storage"))?;

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
                .map_err(|_| CodegenError::new("failed to compute Map capacity pointer"))?
        };
        self.builder
            .build_store(
                capacity_ptr,
                self.context.i64_type().const_int(initial_capacity, false),
            )
            .map_err(|_| CodegenError::new("failed to store Map capacity"))?;

        // Length = 0
        let length_ptr = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "length",
                )
                .map_err(|_| CodegenError::new("failed to compute Map length pointer"))?
        };
        self.builder
            .build_store(length_ptr, self.context.i64_type().const_int(0, false))
            .map_err(|_| CodegenError::new("failed to initialize Map length"))?;

        // Allocate keys and values arrays
        let malloc = self.get_or_declare_malloc();
        let (key_ty, value_ty) = match map_expr_ty {
            Type::Map(key, value) => (&**key, &**value),
            _ => (&Type::Integer, &Type::Integer),
        };
        let key_size = self.storage_size_of_llvm_type(self.llvm_type(key_ty));
        let value_size = self.storage_size_of_llvm_type(self.llvm_type(value_ty));
        let keys_size = self
            .context
            .i64_type()
            .const_int(initial_capacity * key_size, false);
        let values_size = self
            .context
            .i64_type()
            .const_int(initial_capacity * value_size, false);

        let keys_call = self
            .builder
            .build_call(malloc, &[keys_size.into()], "keys")
            .map_err(|_| CodegenError::new("failed to emit Map keys malloc call"))?;
        let keys_ptr = self.extract_call_value_with_context(
            keys_call,
            "malloc did not produce a value while allocating map keys storage",
        )?;
        let keys_field = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(2, false)],
                    "keys_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute Map keys field pointer"))?
        };
        self.builder
            .build_store(keys_field, keys_ptr)
            .map_err(|_| CodegenError::new("failed to store Map keys pointer"))?;

        let values_call = self
            .builder
            .build_call(malloc, &[values_size.into()], "values")
            .map_err(|_| CodegenError::new("failed to emit Map values malloc call"))?;
        let values_ptr = self.extract_call_value_with_context(
            values_call,
            "malloc did not produce a value while allocating map values storage",
        )?;
        let values_field = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(3, false)],
                    "values_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute Map values field pointer"))?
        };
        self.builder
            .build_store(values_field, values_ptr)
            .map_err(|_| CodegenError::new("failed to store Map values pointer"))?;

        self.builder
            .build_load(map_type, alloca, "map")
            .map_err(|_| CodegenError::new("failed to load empty Map"))
    }

    pub fn create_empty_set_for_type(
        &mut self,
        set_expr_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        // Set struct: { capacity: i64, length: i64, data: ptr }
        let set_type = self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );

        let alloca = self
            .builder
            .build_alloca(set_type, "set")
            .map_err(|_| CodegenError::new("failed to allocate Set storage"))?;

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
                .map_err(|_| CodegenError::new("failed to compute Set capacity pointer"))?
        };
        self.builder
            .build_store(
                capacity_ptr,
                self.context.i64_type().const_int(initial_capacity, false),
            )
            .map_err(|_| CodegenError::new("failed to store Set capacity"))?;

        // Length = 0
        let length_ptr = unsafe {
            self.builder
                .build_gep(
                    set_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(1, false)],
                    "length",
                )
                .map_err(|_| CodegenError::new("failed to compute Set length pointer"))?
        };
        self.builder
            .build_store(length_ptr, self.context.i64_type().const_int(0, false))
            .map_err(|_| CodegenError::new("failed to initialize Set length"))?;

        // Allocate data - malloc(capacity * 8)
        let malloc = self.get_or_declare_malloc();
        let elem_size = match set_expr_ty {
            Type::Set(inner) => self.storage_size_of_llvm_type(self.llvm_type(inner)),
            _ => 8,
        };
        let size = self
            .context
            .i64_type()
            .const_int(initial_capacity * elem_size, false);
        let call_result = self
            .builder
            .build_call(malloc, &[size.into()], "data")
            .map_err(|_| CodegenError::new("failed to emit Set malloc call"))?;
        let data_ptr = self.extract_call_value_with_context(
            call_result,
            "malloc did not produce a value while allocating set storage",
        )?;

        let data_ptr_field = unsafe {
            self.builder
                .build_gep(
                    set_type.as_basic_type_enum(),
                    alloca,
                    &[zero, i32_type.const_int(2, false)],
                    "data_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute Set data field pointer"))?
        };
        self.builder
            .build_store(data_ptr_field, data_ptr)
            .map_err(|_| CodegenError::new("failed to store Set data pointer"))?;

        self.builder
            .build_load(set_type, alloca, "set")
            .map_err(|_| CodegenError::new("failed to load empty Set"))
    }

    fn create_zero_initialized_heap_value(
        &mut self,
        value_ty: &Type,
        allocation_name: &str,
        context_name: &str,
    ) -> Result<BasicValueEnum<'ctx>> {
        let malloc = self.get_or_declare_malloc();
        let llvm_ty = self.llvm_type(value_ty);
        let size = self
            .context
            .i64_type()
            .const_int(self.storage_size_of_llvm_type(llvm_ty), false);
        let call_result = self
            .builder
            .build_call(malloc, &[size.into()], allocation_name)
            .map_err(|_| {
                CodegenError::new(format!(
                    "failed to emit malloc call for {context_name} storage"
                ))
            })?;
        let ptr = self.extract_call_pointer_value(
            call_result,
            &format!("malloc did not produce a pointer while allocating {context_name} storage"),
        )?;
        self.zero_initialize_allocated_bytes(
            ptr,
            size.get_zero_extended_constant().unwrap_or(0),
            context_name,
        )?;
        Ok(ptr.into())
    }

    fn create_heap_value_with_payload(
        &mut self,
        value_ty: &Type,
        payload: BasicValueEnum<'ctx>,
        allocation_name: &str,
        context_name: &str,
    ) -> Result<BasicValueEnum<'ctx>> {
        let malloc = self.get_or_declare_malloc();
        let llvm_ty = self.llvm_type(value_ty);
        let size_bytes = self.storage_size_of_llvm_type(llvm_ty);
        let size = self.context.i64_type().const_int(size_bytes, false);
        let call_result = self
            .builder
            .build_call(malloc, &[size.into()], allocation_name)
            .map_err(|_| {
                CodegenError::new(format!(
                    "failed to emit malloc call for {context_name} storage"
                ))
            })?;
        let ptr = self.extract_call_pointer_value(
            call_result,
            &format!("malloc did not produce a pointer while allocating {context_name} storage"),
        )?;
        self.builder.build_store(ptr, payload).map_err(|_| {
            CodegenError::new(format!("failed to store payload for {context_name}"))
        })?;
        Ok(ptr.into())
    }

    fn create_zero_initialized_class_instance(
        &mut self,
        class_name: &str,
        visited_classes: &mut HashSet<String>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let class_info = self.classes.get(class_name).ok_or_else(|| {
            CodegenError::new(format!("Unknown class for default value: {class_name}"))
        })?;
        let struct_ty = class_info.struct_type;
        let field_types = class_info.field_types.clone();
        let field_indices = class_info.field_indices.clone();
        let malloc = self.get_or_declare_malloc();
        let size_bytes = self.storage_size_of_llvm_type(struct_ty.into());
        let size = self.context.i64_type().const_int(size_bytes, false);
        let call_result = self
            .builder
            .build_call(malloc, &[size.into()], "default_class_alloc")
            .map_err(|_| {
                CodegenError::new("failed to emit malloc call for class default storage")
            })?;
        let ptr = self.extract_call_pointer_value(
            call_result,
            "malloc did not produce a pointer while allocating class default storage",
        )?;
        self.zero_initialize_allocated_bytes(ptr, size_bytes, class_name)?;
        let inserted_class = visited_classes.insert(class_name.to_string());
        let i32_type = self.context.i32_type();
        let zero = i32_type.const_zero();
        for (field_name, field_ty) in &field_types {
            let Some(field_index) = field_indices.get(field_name) else {
                continue;
            };
            if let Type::Named(nested_class_name) = self.deref_codegen_type(field_ty) {
                if !inserted_class && visited_classes.contains(nested_class_name) {
                    continue;
                }
            }
            let default_value =
                self.create_default_value_for_type_with_guard(field_ty, visited_classes)?;
            let field_ptr = unsafe {
                self.builder
                    .build_gep(
                        struct_ty.as_basic_type_enum(),
                        ptr,
                        &[zero, i32_type.const_int(*field_index as u64, false)],
                        &format!("default_{}_{}", class_name, field_name),
                    )
                    .map_err(|_| {
                        CodegenError::new(format!(
                            "failed to compute default field pointer for {class_name}.{field_name}"
                        ))
                    })?
            };
            self.builder
                .build_store(field_ptr, default_value)
                .map_err(|_| {
                    CodegenError::new(format!(
                        "failed to store default field value for {class_name}.{field_name}"
                    ))
                })?;
        }
        if inserted_class {
            visited_classes.remove(class_name);
        }
        Ok(ptr.into())
    }

    pub fn create_empty_range_typed(&mut self, range_ty: &Type) -> Result<BasicValueEnum<'ctx>> {
        let inner_ty = match self.deref_codegen_type(range_ty) {
            Type::Range(inner) => inner.as_ref(),
            _ => &Type::Integer,
        };
        match inner_ty {
            Type::Integer => Ok(self
                .create_range(
                    self.context.i64_type().const_zero().into(),
                    self.context.i64_type().const_zero().into(),
                    self.context.i64_type().const_int(1, false).into(),
                )?
                .into()),
            Type::Float => Ok(self
                .create_range(
                    self.context.f64_type().const_float(0.0).into(),
                    self.context.f64_type().const_float(0.0).into(),
                    self.context.f64_type().const_float(1.0).into(),
                )?
                .into()),
            _ => Err(CodegenError::new(
                "Range<T> default value creation supports only Integer and Float elements",
            )),
        }
    }

    fn create_default_value_for_type_with_guard(
        &mut self,
        ty: &Type,
        visited_classes: &mut HashSet<String>,
    ) -> Result<BasicValueEnum<'ctx>> {
        match self.deref_codegen_type(ty) {
            Type::String => {
                let malloc = self.get_or_declare_malloc();
                let call_result = self
                    .builder
                    .build_call(
                        malloc,
                        &[self.context.i64_type().const_int(1, false).into()],
                        "default_string_alloc",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to emit malloc call for default String storage")
                    })?;
                let ptr = self.extract_call_pointer_value(
                    call_result,
                    "malloc did not produce a pointer while allocating default String storage",
                )?;
                self.builder
                    .build_store(ptr, self.context.i8_type().const_zero())
                    .map_err(|_| CodegenError::new("failed to store default String terminator"))?;
                Ok(ptr.into())
            }
            Type::Named(name) if self.classes.contains_key(name) => {
                self.create_zero_initialized_class_instance(name, visited_classes)
            }
            Type::Box(_) => self.create_empty_box_typed(ty),
            Type::Rc(_) => self.create_empty_rc_typed(ty),
            Type::Arc(_) => self.create_empty_arc_typed(ty),
            Type::Range(_) => self.create_empty_range_typed(ty),
            Type::Result(ok, err) => {
                self.create_default_result_typed_with_guard(ok, err, visited_classes)
            }
            _ => {
                let llvm_ty = self.llvm_type(ty);
                Ok(match llvm_ty {
                    inkwell::types::BasicTypeEnum::IntType(t) => t.const_zero().into(),
                    inkwell::types::BasicTypeEnum::FloatType(t) => t.const_float(0.0).into(),
                    inkwell::types::BasicTypeEnum::PointerType(t) => t.const_null().into(),
                    inkwell::types::BasicTypeEnum::StructType(t) => t.const_zero().into(),
                    _ => self.context.i8_type().const_zero().into(),
                })
            }
        }
    }

    pub fn create_default_value_for_type(&mut self, ty: &Type) -> Result<BasicValueEnum<'ctx>> {
        let mut visited_classes = HashSet::new();
        self.create_default_value_for_type_with_guard(ty, &mut visited_classes)
    }

    pub fn create_empty_box_typed(&mut self, box_ty: &Type) -> Result<BasicValueEnum<'ctx>> {
        let inner_ty = match self.deref_codegen_type(box_ty) {
            Type::Box(inner) => inner.as_ref(),
            _ => &Type::Integer,
        };
        self.create_zero_initialized_heap_value(inner_ty, "box", "Box")
    }

    pub fn create_box_typed(
        &mut self,
        payload: BasicValueEnum<'ctx>,
        box_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let inner_ty = match self.deref_codegen_type(box_ty) {
            Type::Box(inner) => inner.as_ref(),
            _ => &Type::Integer,
        };
        self.create_heap_value_with_payload(inner_ty, payload, "box", "Box")
    }

    pub fn create_empty_rc_typed(&mut self, rc_ty: &Type) -> Result<BasicValueEnum<'ctx>> {
        let inner_ty = match self.deref_codegen_type(rc_ty) {
            Type::Rc(inner) => inner.as_ref(),
            _ => &Type::Integer,
        };
        self.create_zero_initialized_heap_value(inner_ty, "rc", "Rc")
    }

    pub fn create_rc_typed(
        &mut self,
        payload: BasicValueEnum<'ctx>,
        rc_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let inner_ty = match self.deref_codegen_type(rc_ty) {
            Type::Rc(inner) => inner.as_ref(),
            _ => &Type::Integer,
        };
        self.create_heap_value_with_payload(inner_ty, payload, "rc", "Rc")
    }

    pub fn create_empty_arc_typed(&mut self, arc_ty: &Type) -> Result<BasicValueEnum<'ctx>> {
        let inner_ty = match self.deref_codegen_type(arc_ty) {
            Type::Arc(inner) => inner.as_ref(),
            _ => &Type::Integer,
        };
        self.create_zero_initialized_heap_value(inner_ty, "arc", "Arc")
    }

    pub fn create_arc_typed(
        &mut self,
        payload: BasicValueEnum<'ctx>,
        arc_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>> {
        let inner_ty = match self.deref_codegen_type(arc_ty) {
            Type::Arc(inner) => inner.as_ref(),
            _ => &Type::Integer,
        };
        self.create_heap_value_with_payload(inner_ty, payload, "arc", "Arc")
    }

    pub fn compile_list_method_on_value(
        &mut self,
        list_value: BasicValueEnum<'ctx>,
        list_ty: &Type,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        match method {
            "length" | "pop" => self.validate_builtin_method_arg_count("List", method, args, 0)?,
            "push" | "get" => self.validate_builtin_method_arg_count("List", method, args, 1)?,
            "set" => self.validate_builtin_method_arg_count("List", method, args, 2)?,
            _ => {}
        }
        let list_ptr = self.materialize_value_pointer_for_type(list_value, list_ty, "list_tmp")?;
        let (elem_llvm_ty, elem_size) =
            self.list_element_layout_from_list_type(self.deref_codegen_type(list_ty));
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
                        .map_err(|_| CodegenError::new("failed to compute List capacity pointer"))?
                };
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to compute List length pointer"))?
                };
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to compute List data pointer"))?
                };

                let capacity = self
                    .builder
                    .build_load(i64_type, capacity_ptr, "cap")
                    .map_err(|_| CodegenError::new("failed to load List capacity"))?
                    .into_int_value();
                let length = self
                    .builder
                    .build_load(i64_type, length_ptr, "len")
                    .map_err(|_| CodegenError::new("failed to load List length"))?
                    .into_int_value();

                // Grow backing storage when length reaches capacity.
                let need_grow = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, length, capacity, "need_grow")
                    .map_err(|_| CodegenError::new("failed to compare List growth need"))?;
                let function = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("No current function for list push"))?;
                let grow_bb = self.context.append_basic_block(function, "list_grow");
                let cont_bb = self.context.append_basic_block(function, "list_push_cont");
                self.builder
                    .build_conditional_branch(need_grow, grow_bb, cont_bb)
                    .map_err(|_| CodegenError::new("failed to branch for List growth"))?;

                self.builder.position_at_end(grow_bb);
                self.grow_list_data_with_copy(
                    function,
                    data_ptr_ptr,
                    capacity_ptr,
                    capacity,
                    length,
                    elem_size,
                )?;
                self.builder
                    .build_unconditional_branch(cont_bb)
                    .map_err(|_| CodegenError::new("failed to continue List push after growth"))?;

                self.builder.position_at_end(cont_bb);
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .map_err(|_| CodegenError::new("failed to load List data pointer"))?
                    .into_pointer_value();

                // Calculate element pointer: data + length * 8
                let offset = self
                    .builder
                    .build_int_mul(length, elem_size_i64, "offset")
                    .map_err(|_| CodegenError::new("failed to compute List push offset"))?;
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .map_err(|_| CodegenError::new("failed to compute List element pointer"))?
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .map_err(|_| CodegenError::new("failed to cast List element pointer"))?;

                // Store the value
                let inner_ty = match self.deref_codegen_type(list_ty) {
                    Type::List(inner) => &**inner,
                    _ => return Err(CodegenError::new("Expected List type")),
                };
                let value =
                    self.compile_expr_for_concrete_class_payload(&args[0].node, inner_ty)?;
                self.builder
                    .build_store(typed_elem_ptr, value)
                    .map_err(|_| CodegenError::new("failed to store pushed List value"))?;

                // Increment length
                let new_length = self
                    .builder
                    .build_int_add(length, one_i64, "new_len")
                    .map_err(|_| CodegenError::new("failed to increment List length"))?;
                self.builder
                    .build_store(length_ptr, new_length)
                    .map_err(|_| CodegenError::new("failed to store List length"))?;

                Ok(self.context.i8_type().const_int(0, false).into())
            }
            "get" => {
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
                        .map_err(|_| CodegenError::new("failed to compute List length pointer"))?
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .map_err(|_| CodegenError::new("failed to load List length"))?
                    .into_int_value();
                let index = self.compile_non_negative_integer_index_expr(
                    &args[0].node,
                    "List.get() index cannot be negative",
                )?;
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("List.get used outside function"))?;
                let ok_bb = self.context.append_basic_block(current_fn, "list_get.ok");
                let fail_bb = self.context.append_basic_block(current_fn, "list_get.fail");
                let non_negative = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SGE,
                        index,
                        self.context.i64_type().const_zero(),
                        "list_get_non_negative",
                    )
                    .map_err(|_| CodegenError::new("failed to check List.get index sign"))?;
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, index, length, "list_get_in_bounds")
                    .map_err(|_| CodegenError::new("failed to check List.get bounds"))?;
                let valid = self
                    .builder
                    .build_and(non_negative, in_bounds, "list_get_valid")
                    .map_err(|_| CodegenError::new("failed to combine List.get bounds checks"))?;
                self.builder
                    .build_conditional_branch(valid, ok_bb, fail_bb)
                    .map_err(|_| CodegenError::new("failed to branch for List.get"))?;

                self.builder.position_at_end(fail_bb);
                self.emit_runtime_error("List.get() index out of bounds", "list_get_oob")?;

                self.builder.position_at_end(ok_bb);
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to compute List data pointer"))?
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .map_err(|_| CodegenError::new("failed to load List data pointer"))?
                    .into_pointer_value();

                let offset = self
                    .builder
                    .build_int_mul(
                        index,
                        self.context.i64_type().const_int(elem_size, false),
                        "offset",
                    )
                    .map_err(|_| CodegenError::new("failed to compute List.get offset"))?;
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .map_err(|_| {
                            CodegenError::new("failed to compute List.get element pointer")
                        })?
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .map_err(|_| CodegenError::new("failed to cast List.get element pointer"))?;

                // Load and return the value
                let val = self
                    .builder
                    .build_load(elem_llvm_ty, typed_elem_ptr, "val")
                    .map_err(|_| CodegenError::new("failed to load List.get value"))?;
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
                        .map_err(|_| CodegenError::new("failed to compute List length pointer"))?
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .map_err(|_| CodegenError::new("failed to load List length"))?;
                Ok(length)
            }
            "pop" => {
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
                        .map_err(|_| CodegenError::new("failed to compute List length pointer"))?
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .map_err(|_| CodegenError::new("failed to load List length"))?
                    .into_int_value();
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("List.pop used outside function"))?;
                let ok_bb = self.context.append_basic_block(current_fn, "list_pop.ok");
                let fail_bb = self.context.append_basic_block(current_fn, "list_pop.fail");
                let has_items = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SGT,
                        length,
                        self.context.i64_type().const_zero(),
                        "list_pop_has_items",
                    )
                    .map_err(|_| CodegenError::new("failed to check List.pop emptiness"))?;
                self.builder
                    .build_conditional_branch(has_items, ok_bb, fail_bb)
                    .map_err(|_| CodegenError::new("failed to branch for List.pop"))?;

                self.builder.position_at_end(fail_bb);
                self.emit_runtime_error("List.pop() on empty list", "list_pop_empty")?;

                self.builder.position_at_end(ok_bb);

                // new_length = length - 1
                let new_length = self
                    .builder
                    .build_int_sub(
                        length,
                        self.context.i64_type().const_int(1, false),
                        "new_len",
                    )
                    .map_err(|_| CodegenError::new("failed to decrement List length"))?;

                // Update length
                self.builder
                    .build_store(length_ptr, new_length)
                    .map_err(|_| CodegenError::new("failed to store List length after pop"))?;

                // Get data pointer
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to compute List data pointer"))?
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .map_err(|_| CodegenError::new("failed to load List data pointer"))?
                    .into_pointer_value();

                // Get value at new_length (the old last element)
                let offset = self
                    .builder
                    .build_int_mul(
                        new_length,
                        self.context.i64_type().const_int(elem_size, false),
                        "offset",
                    )
                    .map_err(|_| CodegenError::new("failed to compute List.pop offset"))?;
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .map_err(|_| {
                            CodegenError::new("failed to compute List.pop element pointer")
                        })?
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .map_err(|_| CodegenError::new("failed to cast List.pop element pointer"))?;
                let val = self
                    .builder
                    .build_load(elem_llvm_ty, typed_elem_ptr, "val")
                    .map_err(|_| CodegenError::new("failed to load List.pop value"))?;
                Ok(val)
            }
            "set" => {
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
                        .map_err(|_| CodegenError::new("failed to compute List length pointer"))?
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .map_err(|_| CodegenError::new("failed to load List length"))?
                    .into_int_value();
                let index = self.compile_non_negative_integer_index_expr(
                    &args[0].node,
                    "List.set() index cannot be negative",
                )?;
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("List.set used outside function"))?;
                let ok_bb = self.context.append_basic_block(current_fn, "list_set.ok");
                let fail_bb = self.context.append_basic_block(current_fn, "list_set.fail");
                let non_negative = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SGE,
                        index,
                        self.context.i64_type().const_zero(),
                        "list_set_non_negative",
                    )
                    .map_err(|_| CodegenError::new("failed to check List.set index sign"))?;
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, index, length, "list_set_in_bounds")
                    .map_err(|_| CodegenError::new("failed to check List.set bounds"))?;
                let valid = self
                    .builder
                    .build_and(non_negative, in_bounds, "list_set_valid")
                    .map_err(|_| CodegenError::new("failed to combine List.set bounds checks"))?;
                self.builder
                    .build_conditional_branch(valid, ok_bb, fail_bb)
                    .map_err(|_| CodegenError::new("failed to branch for List.set"))?;

                self.builder.position_at_end(fail_bb);
                self.emit_runtime_error("List.set() index out of bounds", "list_set_oob")?;

                self.builder.position_at_end(ok_bb);
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to compute List data pointer"))?
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .map_err(|_| CodegenError::new("failed to load List data pointer"))?
                    .into_pointer_value();

                let offset = self
                    .builder
                    .build_int_mul(
                        index,
                        self.context.i64_type().const_int(elem_size, false),
                        "offset",
                    )
                    .map_err(|_| CodegenError::new("failed to compute List.set offset"))?;
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .map_err(|_| {
                            CodegenError::new("failed to compute List.set element pointer")
                        })?
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .map_err(|_| CodegenError::new("failed to cast List.set element pointer"))?;

                // Store the value
                let inner_ty = match self.deref_codegen_type(list_ty) {
                    Type::List(inner) => &**inner,
                    _ => return Err(CodegenError::new("Expected List type")),
                };
                let actual_value_ty = self.infer_expr_type(&args[1].node, &[]);
                let value = self.compile_expr_with_expected_type(&args[1].node, inner_ty)?;
                self.reject_incompatible_expected_type_value(inner_ty, &actual_value_ty, value)?;
                self.builder
                    .build_store(typed_elem_ptr, value)
                    .map_err(|_| CodegenError::new("failed to store List.set value"))?;

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
        match method {
            "length" | "pop" => self.validate_builtin_method_arg_count("List", method, args, 0)?,
            "push" | "get" => self.validate_builtin_method_arg_count("List", method, args, 1)?,
            "set" => self.validate_builtin_method_arg_count("List", method, args, 2)?,
            _ => {}
        }
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
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer capacity pointer")
                        })?
                };
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer length pointer")
                        })?
                };
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer data pointer")
                        })?
                };

                let capacity = self
                    .builder
                    .build_load(i64_type, capacity_ptr, "cap")
                    .map_err(|_| CodegenError::new("failed to load List pointer capacity"))?
                    .into_int_value();
                let length = self
                    .builder
                    .build_load(i64_type, length_ptr, "len")
                    .map_err(|_| CodegenError::new("failed to load List pointer length"))?
                    .into_int_value();

                let need_grow = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, length, capacity, "need_grow")
                    .map_err(|_| CodegenError::new("failed to compare List pointer growth need"))?;
                let function = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("No current function for list push"))?;
                let grow_bb = self.context.append_basic_block(function, "list_grow");
                let cont_bb = self.context.append_basic_block(function, "list_push_cont");
                self.builder
                    .build_conditional_branch(need_grow, grow_bb, cont_bb)
                    .map_err(|_| CodegenError::new("failed to branch for List pointer growth"))?;

                self.builder.position_at_end(grow_bb);
                self.grow_list_data_with_copy(
                    function,
                    data_ptr_ptr,
                    capacity_ptr,
                    capacity,
                    length,
                    elem_size,
                )?;
                self.builder
                    .build_unconditional_branch(cont_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to continue List pointer push after growth")
                    })?;

                self.builder.position_at_end(cont_bb);
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .map_err(|_| CodegenError::new("failed to load List pointer data"))?
                    .into_pointer_value();

                let offset = self
                    .builder
                    .build_int_mul(length, elem_size_i64, "offset")
                    .map_err(|_| CodegenError::new("failed to compute List pointer push offset"))?;
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer element pointer")
                        })?
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to cast List pointer element pointer")
                    })?;

                let inner_ty = match self.deref_codegen_type(list_ty) {
                    Type::List(inner) => &**inner,
                    _ => return Err(CodegenError::new("Expected List type")),
                };
                let value =
                    self.compile_expr_for_concrete_class_payload(&args[0].node, inner_ty)?;
                self.builder
                    .build_store(typed_elem_ptr, value)
                    .map_err(|_| CodegenError::new("failed to store pushed List pointer value"))?;

                let new_length = self
                    .builder
                    .build_int_add(length, one_i64, "new_len")
                    .map_err(|_| CodegenError::new("failed to increment List pointer length"))?;
                self.builder
                    .build_store(length_ptr, new_length)
                    .map_err(|_| CodegenError::new("failed to store List pointer length"))?;

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
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer length pointer")
                        })?
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .map_err(|_| CodegenError::new("failed to load List pointer length"))?;
                Ok(length)
            }
            "get" => {
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer length pointer")
                        })?
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .map_err(|_| CodegenError::new("failed to load List pointer length"))?
                    .into_int_value();
                let index = self.compile_non_negative_integer_index_expr(
                    &args[0].node,
                    "List.get() index cannot be negative",
                )?;
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("List.get used outside function"))?;
                let ok_bb = self
                    .context
                    .append_basic_block(current_fn, "list_ptr_get.ok");
                let fail_bb = self
                    .context
                    .append_basic_block(current_fn, "list_ptr_get.fail");
                let non_negative = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SGE,
                        index,
                        self.context.i64_type().const_zero(),
                        "list_ptr_get_non_negative",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to check List pointer get index sign")
                    })?;
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, index, length, "list_ptr_get_in_bounds")
                    .map_err(|_| CodegenError::new("failed to check List pointer get bounds"))?;
                let valid = self
                    .builder
                    .build_and(non_negative, in_bounds, "list_ptr_get_valid")
                    .map_err(|_| {
                        CodegenError::new("failed to combine List pointer get bounds checks")
                    })?;
                self.builder
                    .build_conditional_branch(valid, ok_bb, fail_bb)
                    .map_err(|_| CodegenError::new("failed to branch for List pointer get"))?;

                self.builder.position_at_end(fail_bb);
                self.emit_runtime_error("List.get() index out of bounds", "list_ptr_get_oob")?;

                self.builder.position_at_end(ok_bb);
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer data pointer")
                        })?
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .map_err(|_| CodegenError::new("failed to load List pointer data"))?
                    .into_pointer_value();

                let offset = self
                    .builder
                    .build_int_mul(
                        index,
                        self.context.i64_type().const_int(elem_size, false),
                        "offset",
                    )
                    .map_err(|_| CodegenError::new("failed to compute List pointer get offset"))?;
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer get element pointer")
                        })?
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to cast List pointer get element pointer")
                    })?;

                let val = self
                    .builder
                    .build_load(elem_llvm_ty, typed_elem_ptr, "val")
                    .map_err(|_| CodegenError::new("failed to load List pointer get value"))?;
                Ok(val)
            }
            "set" => {
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer length pointer")
                        })?
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .map_err(|_| CodegenError::new("failed to load List pointer length"))?
                    .into_int_value();
                let index = self.compile_non_negative_integer_index_expr(
                    &args[0].node,
                    "List.set() index cannot be negative",
                )?;
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("List.set used outside function"))?;
                let ok_bb = self
                    .context
                    .append_basic_block(current_fn, "list_ptr_set.ok");
                let fail_bb = self
                    .context
                    .append_basic_block(current_fn, "list_ptr_set.fail");
                let non_negative = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SGE,
                        index,
                        self.context.i64_type().const_zero(),
                        "list_ptr_set_non_negative",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to check List pointer set index sign")
                    })?;
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, index, length, "list_ptr_set_in_bounds")
                    .map_err(|_| CodegenError::new("failed to check List pointer set bounds"))?;
                let valid = self
                    .builder
                    .build_and(non_negative, in_bounds, "list_ptr_set_valid")
                    .map_err(|_| {
                        CodegenError::new("failed to combine List pointer set bounds checks")
                    })?;
                self.builder
                    .build_conditional_branch(valid, ok_bb, fail_bb)
                    .map_err(|_| CodegenError::new("failed to branch for List pointer set"))?;

                self.builder.position_at_end(fail_bb);
                self.emit_runtime_error("List.set() index out of bounds", "list_ptr_set_oob")?;

                self.builder.position_at_end(ok_bb);
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer data pointer")
                        })?
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .map_err(|_| CodegenError::new("failed to load List pointer data"))?
                    .into_pointer_value();

                let offset = self
                    .builder
                    .build_int_mul(
                        index,
                        self.context.i64_type().const_int(elem_size, false),
                        "offset",
                    )
                    .map_err(|_| CodegenError::new("failed to compute List pointer set offset"))?;
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer set element pointer")
                        })?
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to cast List pointer set element pointer")
                    })?;

                // Store the value
                let inner_ty = match self.deref_codegen_type(list_ty) {
                    Type::List(inner) => &**inner,
                    _ => return Err(CodegenError::new("Expected List type")),
                };
                let actual_value_ty = self.infer_expr_type(&args[1].node, &[]);
                let value = self.compile_expr_with_expected_type(&args[1].node, inner_ty)?;
                self.reject_incompatible_expected_type_value(inner_ty, &actual_value_ty, value)?;
                self.builder
                    .build_store(typed_elem_ptr, value)
                    .map_err(|_| CodegenError::new("failed to store List pointer set value"))?;

                Ok(self.context.i8_type().const_int(0, false).into())
            }
            "pop" => {
                let length_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(1, false)],
                            "len_ptr",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer length pointer")
                        })?
                };
                let length = self
                    .builder
                    .build_load(self.context.i64_type(), length_ptr, "len")
                    .map_err(|_| CodegenError::new("failed to load List pointer length"))?
                    .into_int_value();
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("List.pop used outside function"))?;
                let ok_bb = self
                    .context
                    .append_basic_block(current_fn, "list_ptr_pop.ok");
                let fail_bb = self
                    .context
                    .append_basic_block(current_fn, "list_ptr_pop.fail");
                let has_items = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SGT,
                        length,
                        self.context.i64_type().const_zero(),
                        "list_ptr_pop_has_items",
                    )
                    .map_err(|_| CodegenError::new("failed to check List pointer pop emptiness"))?;
                self.builder
                    .build_conditional_branch(has_items, ok_bb, fail_bb)
                    .map_err(|_| CodegenError::new("failed to branch for List pointer pop"))?;

                self.builder.position_at_end(fail_bb);
                self.emit_runtime_error("List.pop() on empty list", "list_ptr_pop_empty")?;

                self.builder.position_at_end(ok_bb);

                // new_length = length - 1
                let new_length = self
                    .builder
                    .build_int_sub(
                        length,
                        self.context.i64_type().const_int(1, false),
                        "new_len",
                    )
                    .map_err(|_| CodegenError::new("failed to decrement List pointer length"))?;

                // Update length
                self.builder
                    .build_store(length_ptr, new_length)
                    .map_err(|_| {
                        CodegenError::new("failed to store List pointer length after pop")
                    })?;

                // Get data pointer
                let data_ptr_ptr = unsafe {
                    self.builder
                        .build_gep(
                            list_type.as_basic_type_enum(),
                            list_ptr,
                            &[zero, i32_type.const_int(2, false)],
                            "data_ptr_ptr",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer data pointer")
                        })?
                };
                let data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .map_err(|_| CodegenError::new("failed to load List pointer data"))?
                    .into_pointer_value();

                // Get value at new_length (the old last element)
                let offset = self
                    .builder
                    .build_int_mul(
                        new_length,
                        self.context.i64_type().const_int(elem_size, false),
                        "offset",
                    )
                    .map_err(|_| CodegenError::new("failed to compute List pointer pop offset"))?;
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), data_ptr, &[offset], "elem_ptr")
                        .map_err(|_| {
                            CodegenError::new("failed to compute List pointer pop element pointer")
                        })?
                };
                let typed_elem_ptr = self
                    .builder
                    .build_pointer_cast(
                        elem_ptr,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_elem_ptr",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to cast List pointer pop element pointer")
                    })?;
                let val = self
                    .builder
                    .build_load(elem_llvm_ty, typed_elem_ptr, "val")
                    .map_err(|_| CodegenError::new("failed to load List pointer pop value"))?;
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
            let var = self
                .variables
                .get(map_name)
                .ok_or_else(|| Self::undefined_variable_error(map_name))?;
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
        match method {
            "length" => self.validate_builtin_method_arg_count("Map", method, args, 0)?,
            "get" | "contains" => self.validate_builtin_method_arg_count("Map", method, args, 1)?,
            "insert" | "set" => self.validate_builtin_method_arg_count("Map", method, args, 2)?,
            _ => {}
        }
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
        let (key_ty, val_ty) = match self.deref_codegen_type(map_expr_ty) {
            Type::Map(k, v) => ((**k).clone(), (**v).clone()),
            _ => return Err(CodegenError::new("Expected Map type")),
        };
        let key_llvm = self.llvm_type(&key_ty);
        let val_llvm = self.llvm_type(&val_ty);
        let key_size = self.storage_size_of_llvm_type(key_llvm);
        let val_size = self.storage_size_of_llvm_type(val_llvm);

        let length_ptr = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    map_ptr,
                    &[zero, i32_type.const_int(1, false)],
                    "len_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute Map length pointer"))?
        };
        let keys_ptr_ptr = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    map_ptr,
                    &[zero, i32_type.const_int(2, false)],
                    "keys_ptr_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute Map keys pointer"))?
        };
        let values_ptr_ptr = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    map_ptr,
                    &[zero, i32_type.const_int(3, false)],
                    "vals_ptr_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute Map values pointer"))?
        };

        match method {
            "length" => self
                .builder
                .build_load(i64_type, length_ptr, "len")
                .map_err(|_| CodegenError::new("failed to load Map length")),
            "insert" => self.compile_map_method_on_value(map_value, map_expr_ty, "set", args),
            "set" => {
                let actual_key_ty = self.infer_expr_type(&args[0].node, &[]);
                let key = self.compile_expr_with_expected_type(&args[0].node, &key_ty)?;
                self.reject_incompatible_expected_type_value(&key_ty, &actual_key_ty, key)?;
                let actual_value_ty = self.infer_expr_type(&args[1].node, &[]);
                let value = self.compile_expr_with_expected_type(&args[1].node, &val_ty)?;
                self.reject_incompatible_expected_type_value(&val_ty, &actual_value_ty, value)?;
                self.compile_map_set_on_value_with_compiled_key_value(
                    map_value,
                    map_expr_ty,
                    key,
                    value,
                )
            }
            "get" => {
                let actual_key_ty = self.infer_expr_type(&args[0].node, &[]);
                let key = self.compile_expr_with_expected_type(&args[0].node, &key_ty)?;
                self.reject_incompatible_expected_type_value(&key_ty, &actual_key_ty, key)?;
                let length = self
                    .builder
                    .build_load(i64_type, length_ptr, "len")
                    .map_err(|_| CodegenError::new("failed to load Map length"))?
                    .into_int_value();
                let keys_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        keys_ptr_ptr,
                        "keys",
                    )
                    .map_err(|_| CodegenError::new("failed to load Map keys data pointer"))?
                    .into_pointer_value();
                let values_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        values_ptr_ptr,
                        "vals",
                    )
                    .map_err(|_| CodegenError::new("failed to load Map values data pointer"))?
                    .into_pointer_value();

                let idx_ptr = self
                    .builder
                    .build_alloca(i64_type, "map_idx")
                    .map_err(|_| CodegenError::new("failed to allocate Map.get index"))?;
                let res_ptr = self
                    .builder
                    .build_alloca(val_llvm, "map_get_res")
                    .map_err(|_| CodegenError::new("failed to allocate Map.get result slot"))?;
                let found_ptr = self
                    .builder
                    .build_alloca(self.context.bool_type(), "map_get_found")
                    .map_err(|_| CodegenError::new("failed to allocate Map.get found flag"))?;
                self.builder
                    .build_store(idx_ptr, i64_type.const_int(0, false))
                    .map_err(|_| CodegenError::new("failed to initialize Map.get index"))?;
                self.builder
                    .build_store(res_ptr, val_llvm.const_zero())
                    .map_err(|_| CodegenError::new("failed to initialize Map.get result slot"))?;
                self.builder
                    .build_store(found_ptr, self.context.bool_type().const_zero())
                    .map_err(|_| CodegenError::new("failed to initialize Map.get found flag"))?;

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Map.get used outside function"))?;
                let cond_bb = self.context.append_basic_block(current_fn, "map_get.cond");
                let body_bb = self.context.append_basic_block(current_fn, "map_get.body");
                let done_bb = self.context.append_basic_block(current_fn, "map_get.done");
                let merge_bb = self.context.append_basic_block(current_fn, "map_get.merge");
                let fail_bb = self.context.append_basic_block(current_fn, "map_get.fail");
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::new("failed to enter Map.get loop"))?;

                self.builder.position_at_end(cond_bb);
                let i = self
                    .builder
                    .build_load(i64_type, idx_ptr, "i")
                    .map_err(|_| CodegenError::new("failed to load Map.get index"))?
                    .into_int_value();
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, i, length, "i_lt_len")
                    .map_err(|_| CodegenError::new("failed to compare Map.get bounds"))?;
                self.builder
                    .build_conditional_branch(in_bounds, body_bb, done_bb)
                    .map_err(|_| CodegenError::new("failed to branch in Map.get loop"))?;

                self.builder.position_at_end(body_bb);
                let offset = self
                    .builder
                    .build_int_mul(i, i64_type.const_int(key_size, false), "offset")
                    .map_err(|_| CodegenError::new("failed to compute Map.get key offset"))?;
                let key_slot = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), keys_ptr, &[offset], "key_slot")
                        .map_err(|_| CodegenError::new("failed to compute Map.get key slot"))?
                };
                let typed_key_slot = self
                    .builder
                    .build_pointer_cast(
                        key_slot,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_key_slot",
                    )
                    .map_err(|_| CodegenError::new("failed to cast Map.get key slot"))?;
                let existing = self
                    .builder
                    .build_load(key_llvm, typed_key_slot, "existing")
                    .map_err(|_| CodegenError::new("failed to load Map.get existing key"))?;
                let eq = self.build_value_equality(existing, key, &key_ty, "eq")?;
                let next_bb = self.context.append_basic_block(current_fn, "map_get.next");
                let found_bb = self.context.append_basic_block(current_fn, "map_get.found");
                self.builder
                    .build_conditional_branch(eq, found_bb, next_bb)
                    .map_err(|_| CodegenError::new("failed to branch on Map.get key equality"))?;

                self.builder.position_at_end(found_bb);
                let value_offset = self
                    .builder
                    .build_int_mul(i, i64_type.const_int(val_size, false), "value_offset")
                    .map_err(|_| CodegenError::new("failed to compute Map.get value offset"))?;
                let val_slot = unsafe {
                    self.builder
                        .build_gep(
                            self.context.i8_type(),
                            values_ptr,
                            &[value_offset],
                            "val_slot",
                        )
                        .map_err(|_| CodegenError::new("failed to compute Map.get value slot"))?
                };
                let typed_val_slot = self
                    .builder
                    .build_pointer_cast(
                        val_slot,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_val_slot",
                    )
                    .map_err(|_| CodegenError::new("failed to cast Map.get value slot"))?;
                let found = self
                    .builder
                    .build_load(val_llvm, typed_val_slot, "found")
                    .map_err(|_| CodegenError::new("failed to load Map.get value"))?;
                self.builder
                    .build_store(res_ptr, found)
                    .map_err(|_| CodegenError::new("failed to store Map.get value"))?;
                self.builder
                    .build_store(found_ptr, self.context.bool_type().const_all_ones())
                    .map_err(|_| CodegenError::new("failed to mark Map.get as found"))?;
                self.builder
                    .build_unconditional_branch(done_bb)
                    .map_err(|_| CodegenError::new("failed to finish Map.get after match"))?;

                self.builder.position_at_end(next_bb);
                let next_i = self
                    .builder
                    .build_int_add(i, i64_type.const_int(1, false), "next_i")
                    .map_err(|_| CodegenError::new("failed to increment Map.get index"))?;
                self.builder
                    .build_store(idx_ptr, next_i)
                    .map_err(|_| CodegenError::new("failed to store Map.get index"))?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::new("failed to continue Map.get loop"))?;

                self.builder.position_at_end(done_bb);
                let found = self
                    .builder
                    .build_load(self.context.bool_type(), found_ptr, "map_get_found")
                    .map_err(|_| CodegenError::new("failed to load Map.get found flag"))?
                    .into_int_value();
                self.builder
                    .build_conditional_branch(found, merge_bb, fail_bb)
                    .map_err(|_| CodegenError::new("failed to branch on Map.get result"))?;

                self.builder.position_at_end(fail_bb);
                self.emit_runtime_error("Map.get() missing key", "map_get_missing_key")?;

                self.builder.position_at_end(merge_bb);
                self.builder
                    .build_load(val_llvm, res_ptr, "map_get_res")
                    .map_err(|_| CodegenError::new("failed to load Map.get result"))
            }
            "contains" => {
                let actual_key_ty = self.infer_expr_type(&args[0].node, &[]);
                let key = self.compile_expr_with_expected_type(&args[0].node, &key_ty)?;
                self.reject_incompatible_expected_type_value(&key_ty, &actual_key_ty, key)?;
                let length = self
                    .builder
                    .build_load(i64_type, length_ptr, "len")
                    .map_err(|_| CodegenError::new("failed to load Map length"))?
                    .into_int_value();
                let keys_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        keys_ptr_ptr,
                        "keys",
                    )
                    .map_err(|_| CodegenError::new("failed to load Map keys data pointer"))?
                    .into_pointer_value();

                let idx_ptr = self
                    .builder
                    .build_alloca(i64_type, "map_idx")
                    .map_err(|_| CodegenError::new("failed to allocate Map.contains index"))?;
                let res_ptr = self
                    .builder
                    .build_alloca(self.context.bool_type(), "contains_res")
                    .map_err(|_| CodegenError::new("failed to allocate Map.contains result"))?;
                self.builder
                    .build_store(idx_ptr, i64_type.const_int(0, false))
                    .map_err(|_| CodegenError::new("failed to initialize Map.contains index"))?;
                self.builder
                    .build_store(res_ptr, self.context.bool_type().const_int(0, false))
                    .map_err(|_| CodegenError::new("failed to initialize Map.contains result"))?;

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Map.contains used outside function"))?;
                let cond_bb = self
                    .context
                    .append_basic_block(current_fn, "map_contains.cond");
                let body_bb = self
                    .context
                    .append_basic_block(current_fn, "map_contains.body");
                let done_bb = self
                    .context
                    .append_basic_block(current_fn, "map_contains.done");
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::new("failed to enter Map.contains loop"))?;

                self.builder.position_at_end(cond_bb);
                let i = self
                    .builder
                    .build_load(i64_type, idx_ptr, "i")
                    .map_err(|_| CodegenError::new("failed to load Map.contains index"))?
                    .into_int_value();
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, i, length, "i_lt_len")
                    .map_err(|_| CodegenError::new("failed to compare Map.contains bounds"))?;
                self.builder
                    .build_conditional_branch(in_bounds, body_bb, done_bb)
                    .map_err(|_| CodegenError::new("failed to branch in Map.contains loop"))?;

                self.builder.position_at_end(body_bb);
                let offset = self
                    .builder
                    .build_int_mul(i, i64_type.const_int(key_size, false), "offset")
                    .map_err(|_| CodegenError::new("failed to compute Map.contains key offset"))?;
                let key_slot = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), keys_ptr, &[offset], "key_slot")
                        .map_err(|_| CodegenError::new("failed to compute Map.contains key slot"))?
                };
                let typed_key_slot = self
                    .builder
                    .build_pointer_cast(
                        key_slot,
                        self.context.ptr_type(AddressSpace::default()),
                        "typed_key_slot",
                    )
                    .map_err(|_| CodegenError::new("failed to cast Map.contains key slot"))?;
                let existing = self
                    .builder
                    .build_load(key_llvm, typed_key_slot, "existing")
                    .map_err(|_| CodegenError::new("failed to load Map.contains existing key"))?;
                let eq = self.build_value_equality(existing, key, &key_ty, "eq")?;
                let next_bb = self
                    .context
                    .append_basic_block(current_fn, "map_contains.next");
                let found_bb = self
                    .context
                    .append_basic_block(current_fn, "map_contains.found");
                self.builder
                    .build_conditional_branch(eq, found_bb, next_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch on Map.contains key equality")
                    })?;

                self.builder.position_at_end(found_bb);
                self.builder
                    .build_store(res_ptr, self.context.bool_type().const_int(1, false))
                    .map_err(|_| CodegenError::new("failed to store Map.contains result"))?;
                self.builder
                    .build_unconditional_branch(done_bb)
                    .map_err(|_| CodegenError::new("failed to finish Map.contains after match"))?;

                self.builder.position_at_end(next_bb);
                let next_i = self
                    .builder
                    .build_int_add(i, i64_type.const_int(1, false), "next_i")
                    .map_err(|_| CodegenError::new("failed to increment Map.contains index"))?;
                self.builder
                    .build_store(idx_ptr, next_i)
                    .map_err(|_| CodegenError::new("failed to store Map.contains index"))?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::new("failed to continue Map.contains loop"))?;

                self.builder.position_at_end(done_bb);
                self.builder
                    .build_load(self.context.bool_type(), res_ptr, "contains_res")
                    .map_err(|_| CodegenError::new("failed to load Map.contains result"))
            }
            _ => Err(CodegenError::new(format!("Unknown Map method: {}", method))),
        }
    }

    pub fn compile_map_set_on_value_with_compiled_key_value(
        &mut self,
        map_value: BasicValueEnum<'ctx>,
        map_expr_ty: &Type,
        key: BasicValueEnum<'ctx>,
        value: BasicValueEnum<'ctx>,
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
        let (key_ty, val_ty) = match self.deref_codegen_type(map_expr_ty) {
            Type::Map(k, v) => ((**k).clone(), (**v).clone()),
            _ => return Err(CodegenError::new("Expected Map type")),
        };
        let key_llvm = self.llvm_type(&key_ty);
        let val_llvm = self.llvm_type(&val_ty);
        let key_size = self.storage_size_of_llvm_type(key_llvm);
        let val_size = self.storage_size_of_llvm_type(val_llvm);

        let length_ptr = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    map_ptr,
                    &[zero, i32_type.const_int(1, false)],
                    "len_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute Map.set length pointer"))?
        };
        let keys_ptr_ptr = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    map_ptr,
                    &[zero, i32_type.const_int(2, false)],
                    "keys_ptr_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute Map.set keys pointer"))?
        };
        let values_ptr_ptr = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    map_ptr,
                    &[zero, i32_type.const_int(3, false)],
                    "vals_ptr_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute Map.set values pointer"))?
        };
        let length = self
            .builder
            .build_load(i64_type, length_ptr, "len")
            .map_err(|_| CodegenError::new("failed to load Map.set length"))?
            .into_int_value();
        let keys_ptr = self
            .builder
            .build_load(
                self.context.ptr_type(AddressSpace::default()),
                keys_ptr_ptr,
                "keys",
            )
            .map_err(|_| CodegenError::new("failed to load Map.set keys data"))?
            .into_pointer_value();
        let values_ptr = self
            .builder
            .build_load(
                self.context.ptr_type(AddressSpace::default()),
                values_ptr_ptr,
                "vals",
            )
            .map_err(|_| CodegenError::new("failed to load Map.set values data"))?
            .into_pointer_value();

        let idx_ptr = self
            .builder
            .build_alloca(i64_type, "map_idx")
            .map_err(|_| CodegenError::new("failed to allocate Map.set index"))?;
        self.builder
            .build_store(idx_ptr, i64_type.const_int(0, false))
            .map_err(|_| CodegenError::new("failed to initialize Map.set index"))?;
        let current_fn = self
            .current_function
            .ok_or_else(|| CodegenError::new("Map.set used outside function"))?;
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

        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::new("failed to enter Map.set loop"))?;
        self.builder.position_at_end(cond_bb);
        let i = self
            .builder
            .build_load(i64_type, idx_ptr, "i")
            .map_err(|_| CodegenError::new("failed to load Map.set index"))?
            .into_int_value();
        let in_bounds = self
            .builder
            .build_int_compare(IntPredicate::SLT, i, length, "i_lt_len")
            .map_err(|_| CodegenError::new("failed to compare Map.set bounds"))?;
        self.builder
            .build_conditional_branch(in_bounds, body_bb, append_bb)
            .map_err(|_| CodegenError::new("failed to branch in Map.set loop"))?;

        self.builder.position_at_end(body_bb);
        let offset = self
            .builder
            .build_int_mul(i, i64_type.const_int(key_size, false), "offset")
            .map_err(|_| CodegenError::new("failed to compute Map.set key offset"))?;
        let key_slot = unsafe {
            self.builder
                .build_gep(self.context.i8_type(), keys_ptr, &[offset], "key_slot")
                .map_err(|_| CodegenError::new("failed to compute Map.set key slot"))?
        };
        let typed_key_slot = self
            .builder
            .build_pointer_cast(
                key_slot,
                self.context.ptr_type(AddressSpace::default()),
                "typed_key_slot",
            )
            .map_err(|_| CodegenError::new("failed to cast Map.set key slot"))?;
        let existing = self
            .builder
            .build_load(key_llvm, typed_key_slot, "existing")
            .map_err(|_| CodegenError::new("failed to load Map.set existing key"))?;
        let eq = self.build_value_equality(existing, key, &key_ty, "eq")?;
        self.builder
            .build_conditional_branch(eq, update_bb, cont_bb)
            .map_err(|_| CodegenError::new("failed to branch on Map.set key equality"))?;

        self.builder.position_at_end(update_bb);
        let value_offset = self
            .builder
            .build_int_mul(i, i64_type.const_int(val_size, false), "value_offset")
            .map_err(|_| CodegenError::new("failed to compute Map.set value offset"))?;
        let val_slot = unsafe {
            self.builder
                .build_gep(
                    self.context.i8_type(),
                    values_ptr,
                    &[value_offset],
                    "val_slot",
                )
                .map_err(|_| CodegenError::new("failed to compute Map.set value slot"))?
        };
        let typed_val_slot = self
            .builder
            .build_pointer_cast(
                val_slot,
                self.context.ptr_type(AddressSpace::default()),
                "typed_val_slot",
            )
            .map_err(|_| CodegenError::new("failed to cast Map.set value slot"))?;
        if val_llvm.is_struct_type() || val_llvm.is_array_type() {
            self.builder
                .build_store(typed_val_slot, val_llvm.const_zero())
                .map_err(|_| CodegenError::new("failed to clear existing Map.set value slot"))?;
        }
        self.builder
            .build_store(typed_val_slot, value)
            .map_err(|_| CodegenError::new("failed to store updated Map value"))?;
        self.builder
            .build_unconditional_branch(done_bb)
            .map_err(|_| CodegenError::new("failed to finish Map.set update"))?;

        self.builder.position_at_end(cont_bb);
        let next_i = self
            .builder
            .build_int_add(i, i64_type.const_int(1, false), "next_i")
            .map_err(|_| CodegenError::new("failed to increment Map.set index"))?;
        self.builder
            .build_store(idx_ptr, next_i)
            .map_err(|_| CodegenError::new("failed to store Map.set index"))?;
        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::new("failed to continue Map.set loop"))?;

        self.builder.position_at_end(append_bb);
        let capacity_ptr = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    map_ptr,
                    &[zero, i32_type.const_int(0, false)],
                    "capacity_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute Map capacity pointer"))?
        };
        let capacity = self
            .builder
            .build_load(i64_type, capacity_ptr, "capacity")
            .map_err(|_| CodegenError::new("failed to load Map capacity"))?
            .into_int_value();
        let need_growth = self
            .builder
            .build_int_compare(IntPredicate::UGE, length, capacity, "need_growth")
            .map_err(|_| CodegenError::new("failed to compare Map growth need"))?;
        let grow_bb = self.context.append_basic_block(current_fn, "map_set.grow");
        let store_bb = self.context.append_basic_block(current_fn, "map_set.store");
        self.builder
            .build_conditional_branch(need_growth, grow_bb, store_bb)
            .map_err(|_| CodegenError::new("failed to branch for Map growth"))?;

        self.builder.position_at_end(grow_bb);
        let realloc = self.get_or_declare_realloc();
        let grown_capacity = self
            .builder
            .build_int_mul(capacity, i64_type.const_int(2, false), "grown_capacity")
            .map_err(|_| CodegenError::new("failed to compute grown Map capacity"))?;
        let new_key_size = self
            .builder
            .build_int_mul(
                grown_capacity,
                i64_type.const_int(key_size, false),
                "new_key_size",
            )
            .map_err(|_| CodegenError::new("failed to compute grown Map key storage"))?;
        let grown_keys_call = self
            .builder
            .build_call(
                realloc,
                &[keys_ptr.into(), new_key_size.into()],
                "grown_keys",
            )
            .map_err(|_| CodegenError::new("failed to emit realloc for Map keys"))?;
        let grown_keys =
            self.extract_call_pointer_value(grown_keys_call, "realloc failed for Map key growth")?;
        let new_val_size = self
            .builder
            .build_int_mul(
                grown_capacity,
                i64_type.const_int(val_size, false),
                "new_val_size",
            )
            .map_err(|_| CodegenError::new("failed to compute grown Map value storage"))?;
        let grown_vals_call = self
            .builder
            .build_call(
                realloc,
                &[values_ptr.into(), new_val_size.into()],
                "grown_vals",
            )
            .map_err(|_| CodegenError::new("failed to emit realloc for Map values"))?;
        let grown_vals = self
            .extract_call_pointer_value(grown_vals_call, "realloc failed for Map value growth")?;
        self.builder
            .build_store(keys_ptr_ptr, grown_keys)
            .map_err(|_| CodegenError::new("failed to store grown Map keys pointer"))?;
        self.builder
            .build_store(values_ptr_ptr, grown_vals)
            .map_err(|_| CodegenError::new("failed to store grown Map values pointer"))?;
        self.builder
            .build_store(capacity_ptr, grown_capacity)
            .map_err(|_| CodegenError::new("failed to store grown Map capacity"))?;
        self.builder
            .build_unconditional_branch(store_bb)
            .map_err(|_| CodegenError::new("failed to continue after Map growth"))?;

        self.builder.position_at_end(store_bb);
        let active_keys_ptr = self
            .builder
            .build_load(
                self.context.ptr_type(AddressSpace::default()),
                keys_ptr_ptr,
                "active_keys",
            )
            .map_err(|_| CodegenError::new("failed to load active Map keys pointer"))?
            .into_pointer_value();
        let active_values_ptr = self
            .builder
            .build_load(
                self.context.ptr_type(AddressSpace::default()),
                values_ptr_ptr,
                "active_vals",
            )
            .map_err(|_| CodegenError::new("failed to load active Map values pointer"))?
            .into_pointer_value();
        let offset = self
            .builder
            .build_int_mul(length, i64_type.const_int(key_size, false), "append_off")
            .map_err(|_| CodegenError::new("failed to compute Map append key offset"))?;
        let key_slot = unsafe {
            self.builder
                .build_gep(
                    self.context.i8_type(),
                    active_keys_ptr,
                    &[offset],
                    "key_slot_new",
                )
                .map_err(|_| CodegenError::new("failed to compute Map append key slot"))?
        };
        let typed_key_slot = self
            .builder
            .build_pointer_cast(
                key_slot,
                self.context.ptr_type(AddressSpace::default()),
                "typed_key_slot_new",
            )
            .map_err(|_| CodegenError::new("failed to cast Map append key slot"))?;
        if key_llvm.is_struct_type() || key_llvm.is_array_type() {
            self.builder
                .build_store(typed_key_slot, key_llvm.const_zero())
                .map_err(|_| CodegenError::new("failed to clear Map append key slot"))?;
        }
        self.builder
            .build_store(typed_key_slot, key)
            .map_err(|_| CodegenError::new("failed to store appended Map key"))?;
        let value_offset = self
            .builder
            .build_int_mul(
                length,
                i64_type.const_int(val_size, false),
                "append_val_off",
            )
            .map_err(|_| CodegenError::new("failed to compute Map append value offset"))?;
        let val_slot = unsafe {
            self.builder
                .build_gep(
                    self.context.i8_type(),
                    active_values_ptr,
                    &[value_offset],
                    "val_slot_new",
                )
                .map_err(|_| CodegenError::new("failed to compute Map append value slot"))?
        };
        let typed_val_slot = self
            .builder
            .build_pointer_cast(
                val_slot,
                self.context.ptr_type(AddressSpace::default()),
                "typed_val_slot_new",
            )
            .map_err(|_| CodegenError::new("failed to cast Map append value slot"))?;
        if val_llvm.is_struct_type() || val_llvm.is_array_type() {
            self.builder
                .build_store(typed_val_slot, val_llvm.const_zero())
                .map_err(|_| CodegenError::new("failed to clear Map append value slot"))?;
        }
        self.builder
            .build_store(typed_val_slot, value)
            .map_err(|_| CodegenError::new("failed to store appended Map value"))?;
        let new_len = self
            .builder
            .build_int_add(length, i64_type.const_int(1, false), "new_len")
            .map_err(|_| CodegenError::new("failed to increment Map length"))?;
        self.builder
            .build_store(length_ptr, new_len)
            .map_err(|_| CodegenError::new("failed to store Map length"))?;
        self.builder
            .build_unconditional_branch(done_bb)
            .map_err(|_| CodegenError::new("failed to finish Map append"))?;

        self.builder.position_at_end(done_bb);
        Ok(self.context.i8_type().const_int(0, false).into())
    }

    pub fn compile_map_get_on_value_with_compiled_key(
        &mut self,
        map_value: BasicValueEnum<'ctx>,
        map_expr_ty: &Type,
        key: BasicValueEnum<'ctx>,
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
        let (key_ty, val_ty) = match self.deref_codegen_type(map_expr_ty) {
            Type::Map(k, v) => ((**k).clone(), (**v).clone()),
            _ => return Err(CodegenError::new("Expected Map type")),
        };
        let key_llvm = self.llvm_type(&key_ty);
        let val_llvm = self.llvm_type(&val_ty);
        let key_size = self.storage_size_of_llvm_type(key_llvm);
        let val_size = self.storage_size_of_llvm_type(val_llvm);

        let length_ptr = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    map_ptr,
                    &[zero, i32_type.const_int(1, false)],
                    "len_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute Map.get length pointer"))?
        };
        let keys_ptr_ptr = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    map_ptr,
                    &[zero, i32_type.const_int(2, false)],
                    "keys_ptr_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute Map.get keys pointer"))?
        };
        let values_ptr_ptr = unsafe {
            self.builder
                .build_gep(
                    map_type.as_basic_type_enum(),
                    map_ptr,
                    &[zero, i32_type.const_int(3, false)],
                    "vals_ptr_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute Map.get values pointer"))?
        };

        let length = self
            .builder
            .build_load(i64_type, length_ptr, "len")
            .map_err(|_| CodegenError::new("failed to load Map.get length"))?
            .into_int_value();
        let keys_ptr = self
            .builder
            .build_load(
                self.context.ptr_type(AddressSpace::default()),
                keys_ptr_ptr,
                "keys",
            )
            .map_err(|_| CodegenError::new("failed to load Map.get keys data"))?
            .into_pointer_value();
        let values_ptr = self
            .builder
            .build_load(
                self.context.ptr_type(AddressSpace::default()),
                values_ptr_ptr,
                "vals",
            )
            .map_err(|_| CodegenError::new("failed to load Map.get values data"))?
            .into_pointer_value();

        let idx_ptr = self
            .builder
            .build_alloca(i64_type, "map_idx")
            .map_err(|_| CodegenError::new("failed to allocate Map.get index"))?;
        let res_ptr = self
            .builder
            .build_alloca(val_llvm, "map_get_res")
            .map_err(|_| CodegenError::new("failed to allocate Map.get result slot"))?;
        let found_ptr = self
            .builder
            .build_alloca(self.context.bool_type(), "map_get_found")
            .map_err(|_| CodegenError::new("failed to allocate Map.get found flag"))?;
        self.builder
            .build_store(idx_ptr, i64_type.const_int(0, false))
            .map_err(|_| CodegenError::new("failed to initialize Map.get index"))?;
        self.builder
            .build_store(res_ptr, val_llvm.const_zero())
            .map_err(|_| CodegenError::new("failed to initialize Map.get result slot"))?;
        self.builder
            .build_store(found_ptr, self.context.bool_type().const_zero())
            .map_err(|_| CodegenError::new("failed to initialize Map.get found flag"))?;

        let current_fn = self
            .current_function
            .ok_or_else(|| CodegenError::new("Map.get used outside function"))?;
        let cond_bb = self.context.append_basic_block(current_fn, "map_get.cond");
        let body_bb = self.context.append_basic_block(current_fn, "map_get.body");
        let done_bb = self.context.append_basic_block(current_fn, "map_get.done");
        let merge_bb = self.context.append_basic_block(current_fn, "map_get.merge");
        let fail_bb = self.context.append_basic_block(current_fn, "map_get.fail");
        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::new("failed to enter compiled Map.get loop"))?;

        self.builder.position_at_end(cond_bb);
        let i = self
            .builder
            .build_load(i64_type, idx_ptr, "i")
            .map_err(|_| CodegenError::new("failed to load compiled Map.get index"))?
            .into_int_value();
        let in_bounds = self
            .builder
            .build_int_compare(IntPredicate::SLT, i, length, "i_lt_len")
            .map_err(|_| CodegenError::new("failed to compare compiled Map.get bounds"))?;
        self.builder
            .build_conditional_branch(in_bounds, body_bb, done_bb)
            .map_err(|_| CodegenError::new("failed to branch in compiled Map.get loop"))?;

        self.builder.position_at_end(body_bb);
        let offset = self
            .builder
            .build_int_mul(i, i64_type.const_int(key_size, false), "offset")
            .map_err(|_| CodegenError::new("failed to compute compiled Map.get key offset"))?;
        let key_slot = unsafe {
            self.builder
                .build_gep(self.context.i8_type(), keys_ptr, &[offset], "key_slot")
                .map_err(|_| CodegenError::new("failed to compute compiled Map.get key slot"))?
        };
        let typed_key_slot = self
            .builder
            .build_pointer_cast(
                key_slot,
                self.context.ptr_type(AddressSpace::default()),
                "typed_key_slot",
            )
            .map_err(|_| CodegenError::new("failed to cast compiled Map.get key slot"))?;
        let existing = self
            .builder
            .build_load(key_llvm, typed_key_slot, "existing")
            .map_err(|_| CodegenError::new("failed to load compiled Map.get existing key"))?;
        let eq = self.build_value_equality(existing, key, &key_ty, "eq")?;
        let next_bb = self.context.append_basic_block(current_fn, "map_get.next");
        let found_bb = self.context.append_basic_block(current_fn, "map_get.found");
        self.builder
            .build_conditional_branch(eq, found_bb, next_bb)
            .map_err(|_| CodegenError::new("failed to branch on compiled Map.get equality"))?;

        self.builder.position_at_end(found_bb);
        let value_offset = self
            .builder
            .build_int_mul(i, i64_type.const_int(val_size, false), "value_offset")
            .map_err(|_| CodegenError::new("failed to compute compiled Map.get value offset"))?;
        let val_slot = unsafe {
            self.builder
                .build_gep(
                    self.context.i8_type(),
                    values_ptr,
                    &[value_offset],
                    "val_slot",
                )
                .map_err(|_| CodegenError::new("failed to compute compiled Map.get value slot"))?
        };
        let typed_val_slot = self
            .builder
            .build_pointer_cast(
                val_slot,
                self.context.ptr_type(AddressSpace::default()),
                "typed_val_slot",
            )
            .map_err(|_| CodegenError::new("failed to cast compiled Map.get value slot"))?;
        let found = self
            .builder
            .build_load(val_llvm, typed_val_slot, "found")
            .map_err(|_| CodegenError::new("failed to load compiled Map.get value"))?;
        self.builder
            .build_store(res_ptr, found)
            .map_err(|_| CodegenError::new("failed to store compiled Map.get value"))?;
        self.builder
            .build_store(found_ptr, self.context.bool_type().const_all_ones())
            .map_err(|_| CodegenError::new("failed to mark compiled Map.get as found"))?;
        self.builder
            .build_unconditional_branch(done_bb)
            .map_err(|_| CodegenError::new("failed to finish compiled Map.get after match"))?;

        self.builder.position_at_end(next_bb);
        let next_i = self
            .builder
            .build_int_add(i, i64_type.const_int(1, false), "next_i")
            .map_err(|_| CodegenError::new("failed to increment compiled Map.get index"))?;
        self.builder
            .build_store(idx_ptr, next_i)
            .map_err(|_| CodegenError::new("failed to store compiled Map.get index"))?;
        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::new("failed to continue compiled Map.get loop"))?;

        self.builder.position_at_end(done_bb);
        let found = self
            .builder
            .build_load(self.context.bool_type(), found_ptr, "map_get_found")
            .map_err(|_| CodegenError::new("failed to load compiled Map.get found flag"))?
            .into_int_value();
        self.builder
            .build_conditional_branch(found, merge_bb, fail_bb)
            .map_err(|_| CodegenError::new("failed to branch on compiled Map.get result"))?;

        self.builder.position_at_end(fail_bb);
        self.emit_runtime_error("Map.get() missing key", "map_get_missing_key")?;

        self.builder.position_at_end(merge_bb);
        self.builder
            .build_load(val_llvm, res_ptr, "map_get_res")
            .map_err(|_| CodegenError::new("failed to load compiled Map.get result"))
    }

    /// Compile range method calls
    pub fn compile_range_method(
        &mut self,
        range_name: &str,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        let (range_ptr, range_ty) = {
            let var = self
                .variables
                .get(range_name)
                .ok_or_else(|| Self::undefined_variable_error(range_name))?;
            let ptr_type = self.context.ptr_type(AddressSpace::default());
            let range_ptr = self
                .builder
                .build_load(ptr_type, var.ptr, "range_ptr")
                .map_err(|_| CodegenError::new("failed to load range pointer"))?
                .into_pointer_value();
            (range_ptr, var.ty.clone())
        };
        self.compile_range_method_on_value(range_ptr.into(), &range_ty, method, args)
    }

    pub fn compile_range_method_on_value(
        &mut self,
        range_value: BasicValueEnum<'ctx>,
        range_expr_ty: &Type,
        method: &str,
        args: &[Spanned<Expr>],
    ) -> Result<BasicValueEnum<'ctx>> {
        match method {
            "has_next" | "next" => {
                self.validate_builtin_method_arg_count("Range", method, args, 0)?
            }
            _ => {}
        }
        let range_ptr = match range_value {
            BasicValueEnum::PointerValue(ptr) => ptr,
            _ => {
                self.materialize_value_pointer_for_type(range_value, range_expr_ty, "range_tmp")?
            }
        };
        let range_element_ty = match self.deref_codegen_type(range_expr_ty) {
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
                        .map_err(|_| CodegenError::new("failed to compute range step pointer"))?
                };
                let current_ptr = unsafe {
                    self.builder
                        .build_gep(range_type, range_ptr, &[zero, three], "current_ptr")
                        .map_err(|_| CodegenError::new("failed to compute range current pointer"))?
                };
                let end_ptr = unsafe {
                    self.builder
                        .build_gep(range_type, range_ptr, &[zero, one], "end_ptr")
                        .map_err(|_| CodegenError::new("failed to compute range end pointer"))?
                };

                match element_llvm_ty {
                    inkwell::types::BasicTypeEnum::IntType(int_ty) => {
                        let step = self
                            .builder
                            .build_load(int_ty, step_ptr, "step")
                            .map_err(|_| CodegenError::new("failed to load integer range step"))?
                            .into_int_value();
                        let current = self
                            .builder
                            .build_load(int_ty, current_ptr, "current")
                            .map_err(|_| {
                                CodegenError::new("failed to load integer range current value")
                            })?
                            .into_int_value();
                        let end = self
                            .builder
                            .build_load(int_ty, end_ptr, "end")
                            .map_err(|_| CodegenError::new("failed to load integer range end"))?
                            .into_int_value();

                        let step_positive = self
                            .builder
                            .build_int_compare(
                                IntPredicate::SGT,
                                step,
                                int_ty.const_zero(),
                                "step_positive",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compare integer range step direction")
                            })?;
                        let current_lt_end = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, current, end, "current_lt_end")
                            .map_err(|_| {
                                CodegenError::new("failed to compare integer range current < end")
                            })?;
                        let current_gt_end = self
                            .builder
                            .build_int_compare(IntPredicate::SGT, current, end, "current_gt_end")
                            .map_err(|_| {
                                CodegenError::new("failed to compare integer range current > end")
                            })?;
                        let result = self
                            .builder
                            .build_select(step_positive, current_lt_end, current_gt_end, "has_next")
                            .map_err(|_| {
                                CodegenError::new("failed to select integer range has_next result")
                            })?;
                        Ok(result.into_int_value().into())
                    }
                    inkwell::types::BasicTypeEnum::FloatType(float_ty) => {
                        let step = self
                            .builder
                            .build_load(float_ty, step_ptr, "step")
                            .map_err(|_| CodegenError::new("failed to load float range step"))?
                            .into_float_value();
                        let current = self
                            .builder
                            .build_load(float_ty, current_ptr, "current")
                            .map_err(|_| {
                                CodegenError::new("failed to load float range current value")
                            })?
                            .into_float_value();
                        let end = self
                            .builder
                            .build_load(float_ty, end_ptr, "end")
                            .map_err(|_| CodegenError::new("failed to load float range end"))?
                            .into_float_value();

                        let step_positive = self
                            .builder
                            .build_float_compare(
                                inkwell::FloatPredicate::OGT,
                                step,
                                float_ty.const_float(0.0),
                                "step_positive",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compare float range step direction")
                            })?;
                        let current_lt_end = self
                            .builder
                            .build_float_compare(
                                inkwell::FloatPredicate::OLT,
                                current,
                                end,
                                "current_lt_end",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compare float range current < end")
                            })?;
                        let current_gt_end = self
                            .builder
                            .build_float_compare(
                                inkwell::FloatPredicate::OGT,
                                current,
                                end,
                                "current_gt_end",
                            )
                            .map_err(|_| {
                                CodegenError::new("failed to compare float range current > end")
                            })?;
                        let result = self
                            .builder
                            .build_select(step_positive, current_lt_end, current_gt_end, "has_next")
                            .map_err(|_| {
                                CodegenError::new("failed to select float range has_next result")
                            })?;
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
                        .map_err(|_| CodegenError::new("failed to compute range current pointer"))?
                };
                let step_ptr = unsafe {
                    self.builder
                        .build_gep(range_type, range_ptr, &[zero, two], "step_ptr")
                        .map_err(|_| CodegenError::new("failed to compute range step pointer"))?
                };

                match element_llvm_ty {
                    inkwell::types::BasicTypeEnum::IntType(int_ty) => {
                        let current = self
                            .builder
                            .build_load(int_ty, current_ptr, "current")
                            .map_err(|_| CodegenError::new("failed to load integer range current"))?
                            .into_int_value();
                        let step = self
                            .builder
                            .build_load(int_ty, step_ptr, "step")
                            .map_err(|_| CodegenError::new("failed to load integer range step"))?
                            .into_int_value();
                        let new_current = self
                            .builder
                            .build_int_add(current, step, "new_current")
                            .map_err(|_| {
                                CodegenError::new("failed to advance integer range current")
                            })?;
                        self.builder
                            .build_store(current_ptr, new_current)
                            .map_err(|_| {
                                CodegenError::new("failed to store integer range current")
                            })?;
                        Ok(current.into())
                    }
                    inkwell::types::BasicTypeEnum::FloatType(float_ty) => {
                        let current = self
                            .builder
                            .build_load(float_ty, current_ptr, "current")
                            .map_err(|_| CodegenError::new("failed to load float range current"))?
                            .into_float_value();
                        let step = self
                            .builder
                            .build_load(float_ty, step_ptr, "step")
                            .map_err(|_| CodegenError::new("failed to load float range step"))?
                            .into_float_value();
                        let new_current = self
                            .builder
                            .build_float_add(current, step, "new_current")
                            .map_err(|_| {
                                CodegenError::new("failed to advance float range current")
                            })?;
                        self.builder
                            .build_store(current_ptr, new_current)
                            .map_err(|_| {
                                CodegenError::new("failed to store float range current")
                            })?;
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
