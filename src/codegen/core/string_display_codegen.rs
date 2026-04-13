use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn compile_value_to_display_string(
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
                    .map_err(|_| CodegenError::new("failed to build boolean display selection"))?
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
                        .map_err(|_| CodegenError::new("failed to access Option display tag"))?
                };
                let tag = self
                    .builder
                    .build_load(self.context.i8_type(), tag_ptr, "display_option_tag")
                    .map_err(|_| CodegenError::new("failed to load Option display tag"))?
                    .into_int_value();
                let is_some = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        tag,
                        self.context.i8_type().const_int(1, false),
                        "display_option_is_some",
                    )
                    .map_err(|_| CodegenError::new("failed to compare Option display tag"))?;

                let some_bb = self.context.append_basic_block(current_fn, "display_option_some");
                let none_bb = self.context.append_basic_block(current_fn, "display_option_none");
                let merge_bb = self.context.append_basic_block(current_fn, "display_option_merge");

                self.builder
                    .build_conditional_branch(is_some, some_bb, none_bb)
                    .map_err(|_| CodegenError::new("failed to branch for Option display"))?;

                self.builder.position_at_end(some_bb);
                let value_ptr = unsafe {
                    self.builder
                        .build_gep(
                            option_struct_type.as_basic_type_enum(),
                            option_ptr,
                            &[zero, one],
                            "display_option_value_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to access Option display payload"))?
                };
                let inner_value = self
                    .builder
                    .build_load(llvm_inner_ty, value_ptr, "display_option_value")
                    .map_err(|_| CodegenError::new("failed to load Option display payload"))?;
                let inner_display =
                    self.compile_value_to_display_string(inner_value, inner_ty.as_ref())?;
                let some_prefix = self
                    .builder
                    .build_global_string_ptr(
                        "Some(",
                        &format!("display_option_some_prefix_{}", self.str_counter),
                    )
                    .map_err(|_| CodegenError::new("failed to build Option display prefix"))?;
                self.str_counter += 1;
                let some_suffix = self
                    .builder
                    .build_global_string_ptr(
                        ")",
                        &format!("display_option_some_suffix_{}", self.str_counter),
                    )
                    .map_err(|_| CodegenError::new("failed to build Option display suffix"))?;
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
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| CodegenError::new("failed to branch from Option some display block"))?;

                self.builder.position_at_end(none_bb);
                let none_display = self
                    .builder
                    .build_global_string_ptr(
                        "None",
                        &format!("display_option_none_{}", self.str_counter),
                    )
                    .map_err(|_| CodegenError::new("failed to build Option none display string"))?;
                self.str_counter += 1;
                let none_end = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::new("Option display none block missing predecessor"))?;
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| CodegenError::new("failed to branch from Option none display block"))?;

                self.builder.position_at_end(merge_bb);
                let display_phi = self
                    .builder
                    .build_phi(self.context.ptr_type(AddressSpace::default()), "display_option")
                    .map_err(|_| CodegenError::new("failed to build Option display phi"))?;
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
                        .map_err(|_| CodegenError::new("failed to access Result display tag"))?
                };
                let tag = self
                    .builder
                    .build_load(self.context.i8_type(), tag_ptr, "display_result_tag")
                    .map_err(|_| CodegenError::new("failed to load Result display tag"))?
                    .into_int_value();
                let is_ok = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        tag,
                        self.context.i8_type().const_int(1, false),
                        "display_result_is_ok",
                    )
                    .map_err(|_| CodegenError::new("failed to compare Result display tag"))?;

                let ok_bb = self.context.append_basic_block(current_fn, "display_result_ok");
                let err_bb = self
                    .context
                    .append_basic_block(current_fn, "display_result_error");
                let merge_bb = self
                    .context
                    .append_basic_block(current_fn, "display_result_merge");

                self.builder
                    .build_conditional_branch(is_ok, ok_bb, err_bb)
                    .map_err(|_| CodegenError::new("failed to branch for Result display"))?;

                self.builder.position_at_end(ok_bb);
                let ok_ptr = unsafe {
                    self.builder
                        .build_gep(
                            result_struct_type.as_basic_type_enum(),
                            result_ptr,
                            &[zero, one],
                            "display_result_ok_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to access Result ok display payload"))?
                };
                let ok_value = self
                    .builder
                    .build_load(ok_llvm, ok_ptr, "display_result_ok_value")
                    .map_err(|_| CodegenError::new("failed to load Result ok display payload"))?;
                let ok_display = self.compile_value_to_display_string(ok_value, ok_ty.as_ref())?;
                let ok_prefix = self
                    .builder
                    .build_global_string_ptr(
                        "Ok(",
                        &format!("display_result_ok_prefix_{}", self.str_counter),
                    )
                    .map_err(|_| CodegenError::new("failed to build Result ok display prefix"))?;
                self.str_counter += 1;
                let ok_suffix = self
                    .builder
                    .build_global_string_ptr(
                        ")",
                        &format!("display_result_ok_suffix_{}", self.str_counter),
                    )
                    .map_err(|_| CodegenError::new("failed to build Result ok display suffix"))?;
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
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| CodegenError::new("failed to branch from Result ok display block"))?;

                self.builder.position_at_end(err_bb);
                let err_ptr = unsafe {
                    self.builder
                        .build_gep(
                            result_struct_type.as_basic_type_enum(),
                            result_ptr,
                            &[zero, two],
                            "display_result_error_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to access Result error display payload"))?
                };
                let err_value = self
                    .builder
                    .build_load(err_llvm, err_ptr, "display_result_error_value")
                    .map_err(|_| CodegenError::new("failed to load Result error display payload"))?;
                let err_display =
                    self.compile_value_to_display_string(err_value, err_ty.as_ref())?;
                let err_prefix = self
                    .builder
                    .build_global_string_ptr(
                        "Error(",
                        &format!("display_result_error_prefix_{}", self.str_counter),
                    )
                    .map_err(|_| CodegenError::new("failed to build Result error display prefix"))?;
                self.str_counter += 1;
                let err_suffix = self
                    .builder
                    .build_global_string_ptr(
                        ")",
                        &format!("display_result_error_suffix_{}", self.str_counter),
                    )
                    .map_err(|_| CodegenError::new("failed to build Result error display suffix"))?;
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
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| CodegenError::new("failed to branch from Result error display block"))?;

                self.builder.position_at_end(merge_bb);
                let display_phi = self
                    .builder
                    .build_phi(self.context.ptr_type(AddressSpace::default()), "display_result")
                    .map_err(|_| CodegenError::new("failed to build Result display phi"))?;
                display_phi.add_incoming(&[(&ok_joined, ok_end), (&err_joined, err_end)]);
                Ok(display_phi.as_basic_value().into_pointer_value())
            }
            Type::Char => self.compile_char_to_string(value.into_int_value()),
            Type::Integer | Type::Float => {
                let sprintf = self.get_or_declare_sprintf();
                let buffer_call = self.build_malloc_call(
                    self.context.i64_type().const_int(64, false),
                    "display_buf",
                    "failed to allocate numeric display buffer",
                )?;
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
                            .map_err(|_| CodegenError::new("failed to promote integer to float for display"))?;
                        ("%f", promoted.into())
                    } else {
                        let promoted = self
                            .builder
                            .build_int_s_extend(
                                value.into_int_value(),
                                self.context.i64_type(),
                                "display_i64",
                            )
                            .map_err(|_| CodegenError::new("failed to extend integer for display"))?;
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
                    .map_err(|_| CodegenError::new("failed to emit numeric display sprintf"))?;
                Ok(buffer)
            }
            _ => Err(CodegenError::new(format!(
                "display formatting currently supports Integer, Float, Boolean, String, Char, None, Option<T>, and Result<T, E> when their payload types support display formatting, got {}",
                Self::format_diagnostic_name(&Self::format_type_string(&display_ty))
            ))),
        }
    }

    pub(super) fn compile_char_to_string(
        &mut self,
        codepoint: IntValue<'ctx>,
    ) -> Result<PointerValue<'ctx>> {
        let current_fn = self
            .current_function
            .ok_or_else(|| CodegenError::new("char-to-string used outside function"))?;
        let i8_type = self.context.i8_type();
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let buf_call = self.build_malloc_call(
            i64_type.const_int(5, false),
            "char_str_buf",
            "failed to allocate char display buffer",
        )?;
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
            .map_err(|_| CodegenError::new("failed to compare one-byte UTF-8 boundary"))?;
        let is_two_byte = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                codepoint,
                i32_type.const_int(0x800, false),
                "char_str_is_two_byte",
            )
            .map_err(|_| CodegenError::new("failed to compare two-byte UTF-8 boundary"))?;
        let is_three_byte = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                codepoint,
                i32_type.const_int(0x10000, false),
                "char_str_is_three_byte",
            )
            .map_err(|_| CodegenError::new("failed to compare three-byte UTF-8 boundary"))?;
        let not_one_byte_bb = self
            .context
            .append_basic_block(current_fn, "char_str_not_one");
        let not_two_byte_bb = self
            .context
            .append_basic_block(current_fn, "char_str_not_two");

        self.builder
            .build_conditional_branch(is_one_byte, one_byte_bb, not_one_byte_bb)
            .map_err(|_| CodegenError::new("failed to branch on one-byte UTF-8 case"))?;

        self.builder.position_at_end(not_one_byte_bb);
        self.builder
            .build_conditional_branch(is_two_byte, two_byte_bb, not_two_byte_bb)
            .map_err(|_| CodegenError::new("failed to branch on two-byte UTF-8 case"))?;

        self.builder.position_at_end(not_two_byte_bb);
        self.builder
            .build_conditional_branch(is_three_byte, three_byte_bb, four_byte_bb)
            .map_err(|_| CodegenError::new("failed to branch on three-byte UTF-8 case"))?;

        self.builder.position_at_end(one_byte_bb);
        let byte0 = self
            .builder
            .build_int_truncate(codepoint, i8_type, "char_str_b0")
            .map_err(|_| CodegenError::new("failed to truncate one-byte UTF-8 codepoint"))?;
        let byte0_ptr = unsafe {
            self.builder
                .build_gep(i8_type, buffer, &[i64_type.const_zero()], "char_str_b0_ptr")
                .map_err(|_| CodegenError::new("failed to compute first UTF-8 byte pointer"))?
        };
        let byte1_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    buffer,
                    &[i64_type.const_int(1, false)],
                    "char_str_term1_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute one-byte terminator pointer"))?
        };
        self.builder
            .build_store(byte0_ptr, byte0)
            .map_err(|_| CodegenError::new("failed to store one-byte UTF-8 payload"))?;
        self.builder
            .build_store(byte1_ptr, i8_type.const_zero())
            .map_err(|_| CodegenError::new("failed to store one-byte UTF-8 terminator"))?;
        self.builder
            .build_unconditional_branch(done_bb)
            .map_err(|_| CodegenError::new("failed to branch after one-byte UTF-8 encode"))?;

        self.builder.position_at_end(two_byte_bb);
        let top5 = self
            .builder
            .build_right_shift(
                codepoint,
                i32_type.const_int(6, false),
                false,
                "char_str_top5",
            )
            .map_err(|_| CodegenError::new("failed to compute two-byte UTF-8 top bits"))?;
        let byte0 = self
            .builder
            .build_or(top5, i32_type.const_int(0xC0, false), "char_str_two_b0")
            .map_err(|_| CodegenError::new("failed to build two-byte UTF-8 first byte"))?;
        let low6 = self
            .builder
            .build_and(codepoint, i32_type.const_int(0x3F, false), "char_str_low6")
            .map_err(|_| CodegenError::new("failed to compute two-byte UTF-8 low bits"))?;
        let byte1 = self
            .builder
            .build_or(low6, i32_type.const_int(0x80, false), "char_str_two_b1")
            .map_err(|_| CodegenError::new("failed to build two-byte UTF-8 second byte"))?;
        for (idx, byte) in [(0u64, byte0), (1u64, byte1)] {
            let ptr = unsafe {
                self.builder
                    .build_gep(
                        i8_type,
                        buffer,
                        &[i64_type.const_int(idx, false)],
                        "char_str_ptr",
                    )
                    .map_err(|_| CodegenError::new("failed to compute two-byte UTF-8 slot"))?
            };
            let stored = self
                .builder
                .build_int_truncate(byte, i8_type, "char_str_byte")
                .map_err(|_| CodegenError::new("failed to truncate two-byte UTF-8 byte"))?;
            self.builder
                .build_store(ptr, stored)
                .map_err(|_| CodegenError::new("failed to store two-byte UTF-8 byte"))?;
        }
        let term_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    buffer,
                    &[i64_type.const_int(2, false)],
                    "char_str_term2_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute two-byte terminator pointer"))?
        };
        self.builder
            .build_store(term_ptr, i8_type.const_zero())
            .map_err(|_| CodegenError::new("failed to store two-byte UTF-8 terminator"))?;
        self.builder
            .build_unconditional_branch(done_bb)
            .map_err(|_| CodegenError::new("failed to branch after two-byte UTF-8 encode"))?;

        self.builder.position_at_end(three_byte_bb);
        let top4 = self
            .builder
            .build_right_shift(
                codepoint,
                i32_type.const_int(12, false),
                false,
                "char_str_top4",
            )
            .map_err(|_| CodegenError::new("failed to compute three-byte UTF-8 top bits"))?;
        let byte0 = self
            .builder
            .build_or(top4, i32_type.const_int(0xE0, false), "char_str_three_b0")
            .map_err(|_| CodegenError::new("failed to build three-byte UTF-8 first byte"))?;
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
                    .map_err(|_| {
                        CodegenError::new("failed to compute three-byte UTF-8 mid shift")
                    })?,
                i32_type.const_int(0x3F, false),
                "char_str_mid6",
            )
            .map_err(|_| CodegenError::new("failed to compute three-byte UTF-8 middle bits"))?;
        let byte1 = self
            .builder
            .build_or(mid6, i32_type.const_int(0x80, false), "char_str_three_b1")
            .map_err(|_| CodegenError::new("failed to build three-byte UTF-8 second byte"))?;
        let low6 = self
            .builder
            .build_and(
                codepoint,
                i32_type.const_int(0x3F, false),
                "char_str_three_low6",
            )
            .map_err(|_| CodegenError::new("failed to compute three-byte UTF-8 low bits"))?;
        let byte2 = self
            .builder
            .build_or(low6, i32_type.const_int(0x80, false), "char_str_three_b2")
            .map_err(|_| CodegenError::new("failed to build three-byte UTF-8 third byte"))?;
        for (idx, byte) in [(0u64, byte0), (1u64, byte1), (2u64, byte2)] {
            let ptr = unsafe {
                self.builder
                    .build_gep(
                        i8_type,
                        buffer,
                        &[i64_type.const_int(idx, false)],
                        "char_str_ptr",
                    )
                    .map_err(|_| CodegenError::new("failed to compute three-byte UTF-8 slot"))?
            };
            let stored = self
                .builder
                .build_int_truncate(byte, i8_type, "char_str_byte")
                .map_err(|_| CodegenError::new("failed to truncate three-byte UTF-8 byte"))?;
            self.builder
                .build_store(ptr, stored)
                .map_err(|_| CodegenError::new("failed to store three-byte UTF-8 byte"))?;
        }
        let term_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    buffer,
                    &[i64_type.const_int(3, false)],
                    "char_str_term3_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute three-byte terminator pointer"))?
        };
        self.builder
            .build_store(term_ptr, i8_type.const_zero())
            .map_err(|_| CodegenError::new("failed to store three-byte UTF-8 terminator"))?;
        self.builder
            .build_unconditional_branch(done_bb)
            .map_err(|_| CodegenError::new("failed to branch after three-byte UTF-8 encode"))?;

        self.builder.position_at_end(four_byte_bb);
        let top3 = self
            .builder
            .build_right_shift(
                codepoint,
                i32_type.const_int(18, false),
                false,
                "char_str_top3",
            )
            .map_err(|_| CodegenError::new("failed to compute four-byte UTF-8 top bits"))?;
        let byte0 = self
            .builder
            .build_or(top3, i32_type.const_int(0xF0, false), "char_str_four_b0")
            .map_err(|_| CodegenError::new("failed to build four-byte UTF-8 first byte"))?;
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
                    .map_err(|_| {
                        CodegenError::new("failed to compute four-byte UTF-8 high shift")
                    })?,
                i32_type.const_int(0x3F, false),
                "char_str_high6",
            )
            .map_err(|_| CodegenError::new("failed to compute four-byte UTF-8 high bits"))?;
        let byte1 = self
            .builder
            .build_or(high6, i32_type.const_int(0x80, false), "char_str_four_b1")
            .map_err(|_| CodegenError::new("failed to build four-byte UTF-8 second byte"))?;
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
                    .map_err(|_| {
                        CodegenError::new("failed to compute four-byte UTF-8 mid shift")
                    })?,
                i32_type.const_int(0x3F, false),
                "char_str_four_mid6",
            )
            .map_err(|_| CodegenError::new("failed to compute four-byte UTF-8 middle bits"))?;
        let byte2 = self
            .builder
            .build_or(mid6, i32_type.const_int(0x80, false), "char_str_four_b2")
            .map_err(|_| CodegenError::new("failed to build four-byte UTF-8 third byte"))?;
        let low6 = self
            .builder
            .build_and(
                codepoint,
                i32_type.const_int(0x3F, false),
                "char_str_four_low6",
            )
            .map_err(|_| CodegenError::new("failed to compute four-byte UTF-8 low bits"))?;
        let byte3 = self
            .builder
            .build_or(low6, i32_type.const_int(0x80, false), "char_str_four_b3")
            .map_err(|_| CodegenError::new("failed to build four-byte UTF-8 fourth byte"))?;
        for (idx, byte) in [(0u64, byte0), (1u64, byte1), (2u64, byte2), (3u64, byte3)] {
            let ptr = unsafe {
                self.builder
                    .build_gep(
                        i8_type,
                        buffer,
                        &[i64_type.const_int(idx, false)],
                        "char_str_ptr",
                    )
                    .map_err(|_| CodegenError::new("failed to compute four-byte UTF-8 slot"))?
            };
            let stored = self
                .builder
                .build_int_truncate(byte, i8_type, "char_str_byte")
                .map_err(|_| CodegenError::new("failed to truncate four-byte UTF-8 byte"))?;
            self.builder
                .build_store(ptr, stored)
                .map_err(|_| CodegenError::new("failed to store four-byte UTF-8 byte"))?;
        }
        let term_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    buffer,
                    &[i64_type.const_int(4, false)],
                    "char_str_term4_ptr",
                )
                .map_err(|_| CodegenError::new("failed to compute four-byte terminator pointer"))?
        };
        self.builder
            .build_store(term_ptr, i8_type.const_zero())
            .map_err(|_| CodegenError::new("failed to store four-byte UTF-8 terminator"))?;
        self.builder
            .build_unconditional_branch(done_bb)
            .map_err(|_| CodegenError::new("failed to branch after four-byte UTF-8 encode"))?;

        self.builder.position_at_end(done_bb);
        Ok(buffer)
    }
}
