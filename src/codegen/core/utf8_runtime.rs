use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn compile_utf8_string_index_runtime(
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
            .map_err(|_| CodegenError::new("failed to compare string index against zero"))?;

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
            .map_err(|_| CodegenError::new("failed to allocate UTF-8 string pointer slot"))?;
        let char_index_slot = self
            .builder
            .build_alloca(i64_type, "utf8_string_char_index")
            .map_err(|_| CodegenError::new("failed to allocate UTF-8 string index slot"))?;
        let char_result_slot = self
            .builder
            .build_alloca(i32_type, "utf8_string_char_result")
            .map_err(|_| CodegenError::new("failed to allocate UTF-8 string result slot"))?;

        self.builder
            .build_store(ptr_slot, string_ptr)
            .map_err(|_| CodegenError::new("failed to initialize UTF-8 string pointer slot"))?;
        self.builder
            .build_store(char_index_slot, i64_type.const_zero())
            .map_err(|_| CodegenError::new("failed to initialize UTF-8 string index slot"))?;
        self.builder
            .build_conditional_branch(index_non_negative, loop_bb, fail_oob_bb)
            .map_err(|_| CodegenError::new("failed to branch into UTF-8 string index flow"))?;

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
            .map_err(|_| CodegenError::new("failed to load UTF-8 string pointer"))?
            .into_pointer_value();
        let current_char_index = self
            .builder
            .build_load(i64_type, char_index_slot, "utf8_string_char_index_load")
            .map_err(|_| CodegenError::new("failed to load UTF-8 string character index"))?
            .into_int_value();
        let lead_byte = self
            .builder
            .build_load(i8_type, current_ptr, "utf8_string_lead_byte")
            .map_err(|_| CodegenError::new("failed to load UTF-8 lead byte"))?
            .into_int_value();
        let lead_u32 = self
            .builder
            .build_int_z_extend(lead_byte, i32_type, "utf8_string_lead_u32")
            .map_err(|_| CodegenError::new("failed to extend UTF-8 lead byte"))?;
        let is_end = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_byte,
                i8_type.const_zero(),
                "utf8_string_is_end",
            )
            .map_err(|_| CodegenError::new("failed to check UTF-8 string end"))?;
        let is_ascii = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                lead_u32,
                i32_type.const_int(0x80, false),
                "utf8_string_is_ascii",
            )
            .map_err(|_| CodegenError::new("failed to classify ASCII UTF-8 byte"))?;
        let lead_mask_e0 = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0xE0, false),
                "utf8_string_mask_e0",
            )
            .map_err(|_| CodegenError::new("failed to mask UTF-8 lead byte with 0xE0"))?;
        let lead_mask_f0 = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0xF0, false),
                "utf8_string_mask_f0",
            )
            .map_err(|_| CodegenError::new("failed to mask UTF-8 lead byte with 0xF0"))?;
        let lead_mask_f8 = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0xF8, false),
                "utf8_string_mask_f8",
            )
            .map_err(|_| CodegenError::new("failed to mask UTF-8 lead byte with 0xF8"))?;
        let is_two = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_mask_e0,
                i32_type.const_int(0xC0, false),
                "utf8_string_is_two",
            )
            .map_err(|_| CodegenError::new("failed to classify 2-byte UTF-8 sequence"))?;
        let is_three = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_mask_f0,
                i32_type.const_int(0xE0, false),
                "utf8_string_is_three",
            )
            .map_err(|_| CodegenError::new("failed to classify 3-byte UTF-8 sequence"))?;
        let is_four = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_mask_f8,
                i32_type.const_int(0xF0, false),
                "utf8_string_is_four",
            )
            .map_err(|_| CodegenError::new("failed to classify 4-byte UTF-8 sequence"))?;
        let width_two_or_zero = self
            .builder
            .build_select(
                is_two,
                i64_type.const_int(2, false),
                i64_type.const_zero(),
                "utf8_string_width_two",
            )
            .map_err(|_| CodegenError::new("failed to select UTF-8 width for 2-byte sequence"))?
            .into_int_value();
        let width_three_or_prev = self
            .builder
            .build_select(
                is_three,
                i64_type.const_int(3, false),
                width_two_or_zero,
                "utf8_string_width_three",
            )
            .map_err(|_| CodegenError::new("failed to select UTF-8 width for 3-byte sequence"))?
            .into_int_value();
        let width_nonzero = self
            .builder
            .build_select(
                is_ascii,
                i64_type.const_int(1, false),
                width_three_or_prev,
                "utf8_string_width_ascii",
            )
            .map_err(|_| CodegenError::new("failed to select UTF-8 width for ASCII sequence"))?
            .into_int_value();
        let width = self
            .builder
            .build_select(
                is_four,
                i64_type.const_int(4, false),
                width_nonzero,
                "utf8_string_width",
            )
            .map_err(|_| CodegenError::new("failed to select UTF-8 width for 4-byte sequence"))?
            .into_int_value();
        let width_is_valid = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                width,
                i64_type.const_zero(),
                "utf8_string_width_is_valid",
            )
            .map_err(|_| CodegenError::new("failed to validate UTF-8 width"))?;

        self.builder
            .build_conditional_branch(is_end, fail_oob_bb, not_end_bb)
            .map_err(|_| CodegenError::new("failed to branch on UTF-8 string end"))?;

        self.builder.position_at_end(not_end_bb);
        let is_target = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                current_char_index,
                idx,
                "utf8_string_is_target",
            )
            .map_err(|_| CodegenError::new("failed to compare UTF-8 string index target"))?;
        self.builder
            .build_conditional_branch(is_target, target_dispatch_bb, advance_check_bb)
            .map_err(|_| CodegenError::new("failed to branch for UTF-8 target dispatch"))?;

        self.builder.position_at_end(advance_check_bb);
        self.builder
            .build_conditional_branch(width_is_valid, advance_bb, fail_utf8_bb)
            .map_err(|_| CodegenError::new("failed to branch on UTF-8 width validity"))?;

        self.builder.position_at_end(advance_bb);
        let advanced_ptr = unsafe {
            self.builder
                .build_gep(i8_type, current_ptr, &[width], "utf8_string_advance_ptr")
                .map_err(|_| CodegenError::new("failed to advance UTF-8 string pointer"))?
        };
        let next_char_index = self
            .builder
            .build_int_add(
                current_char_index,
                i64_type.const_int(1, false),
                "utf8_string_next_char_index",
            )
            .map_err(|_| CodegenError::new("failed to increment UTF-8 string character index"))?;
        self.builder
            .build_store(ptr_slot, advanced_ptr)
            .map_err(|_| CodegenError::new("failed to store advanced UTF-8 string pointer"))?;
        self.builder
            .build_store(char_index_slot, next_char_index)
            .map_err(|_| CodegenError::new("failed to store advanced UTF-8 character index"))?;
        self.builder
            .build_unconditional_branch(loop_bb)
            .map_err(|_| CodegenError::new("failed to branch UTF-8 string index loop"))?;

        self.builder.position_at_end(target_dispatch_bb);
        self.builder
            .build_conditional_branch(is_ascii, decode_ascii_bb, target_non_ascii_bb)
            .map_err(|_| CodegenError::new("failed to branch for UTF-8 ASCII decode"))?;

        self.builder.position_at_end(target_non_ascii_bb);
        self.builder
            .build_conditional_branch(is_two, decode_two_bb, target_not_two_bb)
            .map_err(|_| CodegenError::new("failed to branch for UTF-8 two-byte decode"))?;

        self.builder.position_at_end(target_not_two_bb);
        self.builder
            .build_conditional_branch(is_three, decode_three_bb, target_not_three_bb)
            .map_err(|_| CodegenError::new("failed to branch for UTF-8 three-byte decode"))?;

        self.builder.position_at_end(target_not_three_bb);
        self.builder
            .build_conditional_branch(is_four, decode_four_bb, fail_utf8_bb)
            .map_err(|_| CodegenError::new("failed to branch for UTF-8 four-byte decode"))?;

        self.builder.position_at_end(decode_ascii_bb);
        self.builder
            .build_store(char_result_slot, lead_u32)
            .map_err(|_| CodegenError::new("failed to store ASCII UTF-8 decode result"))?;
        self.builder
            .build_unconditional_branch(return_bb)
            .map_err(|_| CodegenError::new("failed to branch from UTF-8 ASCII decode"))?;

        self.builder.position_at_end(decode_two_bb);
        let cont1_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[i64_type.const_int(1, false)],
                    "utf8_cont1_ptr",
                )
                .map_err(|_| CodegenError::new("failed to access first UTF-8 continuation byte"))?
        };
        let cont1 = self
            .builder
            .build_load(i8_type, cont1_ptr, "utf8_cont1")
            .map_err(|_| CodegenError::new("failed to load first UTF-8 continuation byte"))?
            .into_int_value();
        let cont1_u32 = self
            .builder
            .build_int_z_extend(cont1, i32_type, "utf8_cont1_u32")
            .map_err(|_| CodegenError::new("failed to extend first UTF-8 continuation byte"))?;
        let cont1_mask = self
            .builder
            .build_and(
                cont1_u32,
                i32_type.const_int(0xC0, false),
                "utf8_cont1_mask",
            )
            .map_err(|_| CodegenError::new("failed to mask first UTF-8 continuation byte"))?;
        let cont1_valid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                cont1_mask,
                i32_type.const_int(0x80, false),
                "utf8_cont1_valid",
            )
            .map_err(|_| CodegenError::new("failed to validate first UTF-8 continuation byte"))?;
        self.builder
            .build_conditional_branch(cont1_valid, decode_two_ok_bb, fail_utf8_bb)
            .map_err(|_| CodegenError::new("failed to branch for UTF-8 two-byte validation"))?;

        self.builder.position_at_end(decode_two_ok_bb);
        let lead_bits = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0x1F, false),
                "utf8_two_lead_bits",
            )
            .map_err(|_| CodegenError::new("failed to isolate UTF-8 two-byte lead bits"))?;
        let cont1_bits = self
            .builder
            .build_and(
                cont1_u32,
                i32_type.const_int(0x3F, false),
                "utf8_two_cont1_bits",
            )
            .map_err(|_| CodegenError::new("failed to isolate UTF-8 two-byte continuation bits"))?;
        let lead_shifted = self
            .builder
            .build_left_shift(
                lead_bits,
                i32_type.const_int(6, false),
                "utf8_two_lead_shifted",
            )
            .map_err(|_| CodegenError::new("failed to shift UTF-8 two-byte lead bits"))?;
        let codepoint = self
            .builder
            .build_or(lead_shifted, cont1_bits, "utf8_two_codepoint")
            .map_err(|_| CodegenError::new("failed to combine UTF-8 two-byte codepoint"))?;
        self.builder
            .build_store(char_result_slot, codepoint)
            .map_err(|_| CodegenError::new("failed to store UTF-8 two-byte codepoint"))?;
        self.builder
            .build_unconditional_branch(return_bb)
            .map_err(|_| CodegenError::new("failed to branch from UTF-8 two-byte decode"))?;

        self.builder.position_at_end(decode_three_bb);
        let cont2_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[i64_type.const_int(1, false)],
                    "utf8_cont2_ptr",
                )
                .map_err(|_| CodegenError::new("failed to access second UTF-8 continuation byte"))?
        };
        let cont3_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[i64_type.const_int(2, false)],
                    "utf8_cont3_ptr",
                )
                .map_err(|_| CodegenError::new("failed to access third UTF-8 continuation byte"))?
        };
        let cont2 = self
            .builder
            .build_load(i8_type, cont2_ptr, "utf8_cont2")
            .map_err(|_| CodegenError::new("failed to load second UTF-8 continuation byte"))?
            .into_int_value();
        let cont3 = self
            .builder
            .build_load(i8_type, cont3_ptr, "utf8_cont3")
            .map_err(|_| CodegenError::new("failed to load third UTF-8 continuation byte"))?
            .into_int_value();
        let cont2_u32 = self
            .builder
            .build_int_z_extend(cont2, i32_type, "utf8_cont2_u32")
            .map_err(|_| CodegenError::new("failed to extend second UTF-8 continuation byte"))?;
        let cont3_u32 = self
            .builder
            .build_int_z_extend(cont3, i32_type, "utf8_cont3_u32")
            .map_err(|_| CodegenError::new("failed to extend third UTF-8 continuation byte"))?;
        let cont2_mask = self
            .builder
            .build_and(
                cont2_u32,
                i32_type.const_int(0xC0, false),
                "utf8_cont2_mask",
            )
            .map_err(|_| CodegenError::new("failed to mask second UTF-8 continuation byte"))?;
        let cont3_mask = self
            .builder
            .build_and(
                cont3_u32,
                i32_type.const_int(0xC0, false),
                "utf8_cont3_mask",
            )
            .map_err(|_| CodegenError::new("failed to mask third UTF-8 continuation byte"))?;
        let cont2_valid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                cont2_mask,
                i32_type.const_int(0x80, false),
                "utf8_cont2_valid",
            )
            .map_err(|_| CodegenError::new("failed to validate second UTF-8 continuation byte"))?;
        let cont3_valid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                cont3_mask,
                i32_type.const_int(0x80, false),
                "utf8_cont3_valid",
            )
            .map_err(|_| CodegenError::new("failed to validate third UTF-8 continuation byte"))?;
        let cont23_valid = self
            .builder
            .build_and(cont2_valid, cont3_valid, "utf8_cont23_valid")
            .map_err(|_| {
                CodegenError::new("failed to combine UTF-8 three-byte continuation validation")
            })?;
        self.builder
            .build_conditional_branch(cont23_valid, decode_three_ok_bb, fail_utf8_bb)
            .map_err(|_| CodegenError::new("failed to branch for UTF-8 three-byte validation"))?;

        self.builder.position_at_end(decode_three_ok_bb);
        let lead_bits = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0x0F, false),
                "utf8_three_lead_bits",
            )
            .map_err(|_| CodegenError::new("failed to isolate UTF-8 three-byte lead bits"))?;
        let cont2_bits = self
            .builder
            .build_and(
                cont2_u32,
                i32_type.const_int(0x3F, false),
                "utf8_three_cont2_bits",
            )
            .map_err(|_| {
                CodegenError::new("failed to isolate UTF-8 three-byte second continuation bits")
            })?;
        let cont3_bits = self
            .builder
            .build_and(
                cont3_u32,
                i32_type.const_int(0x3F, false),
                "utf8_three_cont3_bits",
            )
            .map_err(|_| {
                CodegenError::new("failed to isolate UTF-8 three-byte third continuation bits")
            })?;
        let lead_shifted = self
            .builder
            .build_left_shift(
                lead_bits,
                i32_type.const_int(12, false),
                "utf8_three_lead_shifted",
            )
            .map_err(|_| CodegenError::new("failed to shift UTF-8 three-byte lead bits"))?;
        let cont2_shifted = self
            .builder
            .build_left_shift(
                cont2_bits,
                i32_type.const_int(6, false),
                "utf8_three_cont2_shifted",
            )
            .map_err(|_| CodegenError::new("failed to shift UTF-8 three-byte continuation bits"))?;
        let partial = self
            .builder
            .build_or(lead_shifted, cont2_shifted, "utf8_three_partial")
            .map_err(|_| {
                CodegenError::new("failed to combine UTF-8 three-byte partial codepoint")
            })?;
        let codepoint = self
            .builder
            .build_or(partial, cont3_bits, "utf8_three_codepoint")
            .map_err(|_| CodegenError::new("failed to combine UTF-8 three-byte codepoint"))?;
        self.builder
            .build_store(char_result_slot, codepoint)
            .map_err(|_| CodegenError::new("failed to store UTF-8 three-byte codepoint"))?;
        self.builder
            .build_unconditional_branch(return_bb)
            .map_err(|_| CodegenError::new("failed to branch from UTF-8 three-byte decode"))?;

        self.builder.position_at_end(decode_four_bb);
        let cont4_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[i64_type.const_int(1, false)],
                    "utf8_cont4_ptr",
                )
                .map_err(|_| CodegenError::new("failed to access fourth UTF-8 continuation byte"))?
        };
        let cont5_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[i64_type.const_int(2, false)],
                    "utf8_cont5_ptr",
                )
                .map_err(|_| CodegenError::new("failed to access fifth UTF-8 continuation byte"))?
        };
        let cont6_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[i64_type.const_int(3, false)],
                    "utf8_cont6_ptr",
                )
                .map_err(|_| CodegenError::new("failed to access sixth UTF-8 continuation byte"))?
        };
        let cont4 = self
            .builder
            .build_load(i8_type, cont4_ptr, "utf8_cont4")
            .map_err(|_| CodegenError::new("failed to load fourth UTF-8 continuation byte"))?
            .into_int_value();
        let cont5 = self
            .builder
            .build_load(i8_type, cont5_ptr, "utf8_cont5")
            .map_err(|_| CodegenError::new("failed to load fifth UTF-8 continuation byte"))?
            .into_int_value();
        let cont6 = self
            .builder
            .build_load(i8_type, cont6_ptr, "utf8_cont6")
            .map_err(|_| CodegenError::new("failed to load sixth UTF-8 continuation byte"))?
            .into_int_value();
        let cont4_u32 = self
            .builder
            .build_int_z_extend(cont4, i32_type, "utf8_cont4_u32")
            .map_err(|_| CodegenError::new("failed to extend fourth UTF-8 continuation byte"))?;
        let cont5_u32 = self
            .builder
            .build_int_z_extend(cont5, i32_type, "utf8_cont5_u32")
            .map_err(|_| CodegenError::new("failed to extend fifth UTF-8 continuation byte"))?;
        let cont6_u32 = self
            .builder
            .build_int_z_extend(cont6, i32_type, "utf8_cont6_u32")
            .map_err(|_| CodegenError::new("failed to extend sixth UTF-8 continuation byte"))?;
        let cont4_mask = self
            .builder
            .build_and(
                cont4_u32,
                i32_type.const_int(0xC0, false),
                "utf8_cont4_mask",
            )
            .map_err(|_| CodegenError::new("failed to mask fourth UTF-8 continuation byte"))?;
        let cont5_mask = self
            .builder
            .build_and(
                cont5_u32,
                i32_type.const_int(0xC0, false),
                "utf8_cont5_mask",
            )
            .map_err(|_| CodegenError::new("failed to mask fifth UTF-8 continuation byte"))?;
        let cont6_mask = self
            .builder
            .build_and(
                cont6_u32,
                i32_type.const_int(0xC0, false),
                "utf8_cont6_mask",
            )
            .map_err(|_| CodegenError::new("failed to mask sixth UTF-8 continuation byte"))?;
        let cont4_valid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                cont4_mask,
                i32_type.const_int(0x80, false),
                "utf8_cont4_valid",
            )
            .map_err(|_| CodegenError::new("failed to validate fourth UTF-8 continuation byte"))?;
        let cont5_valid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                cont5_mask,
                i32_type.const_int(0x80, false),
                "utf8_cont5_valid",
            )
            .map_err(|_| CodegenError::new("failed to validate fifth UTF-8 continuation byte"))?;
        let cont6_valid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                cont6_mask,
                i32_type.const_int(0x80, false),
                "utf8_cont6_valid",
            )
            .map_err(|_| CodegenError::new("failed to validate sixth UTF-8 continuation byte"))?;
        let cont45_valid = self
            .builder
            .build_and(cont4_valid, cont5_valid, "utf8_cont45_valid")
            .map_err(|_| {
                CodegenError::new("failed to combine UTF-8 four-byte continuation validation")
            })?;
        let cont456_valid = self
            .builder
            .build_and(cont45_valid, cont6_valid, "utf8_cont456_valid")
            .map_err(|_| {
                CodegenError::new("failed to finalize UTF-8 four-byte continuation validation")
            })?;
        self.builder
            .build_conditional_branch(cont456_valid, decode_four_ok_bb, fail_utf8_bb)
            .map_err(|_| CodegenError::new("failed to branch for UTF-8 four-byte validation"))?;

        self.builder.position_at_end(decode_four_ok_bb);
        let lead_bits = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0x07, false),
                "utf8_four_lead_bits",
            )
            .map_err(|_| CodegenError::new("failed to isolate UTF-8 four-byte lead bits"))?;
        let cont4_bits = self
            .builder
            .build_and(
                cont4_u32,
                i32_type.const_int(0x3F, false),
                "utf8_four_cont4_bits",
            )
            .map_err(|_| {
                CodegenError::new("failed to isolate UTF-8 four-byte first continuation bits")
            })?;
        let cont5_bits = self
            .builder
            .build_and(
                cont5_u32,
                i32_type.const_int(0x3F, false),
                "utf8_four_cont5_bits",
            )
            .map_err(|_| {
                CodegenError::new("failed to isolate UTF-8 four-byte second continuation bits")
            })?;
        let cont6_bits = self
            .builder
            .build_and(
                cont6_u32,
                i32_type.const_int(0x3F, false),
                "utf8_four_cont6_bits",
            )
            .map_err(|_| {
                CodegenError::new("failed to isolate UTF-8 four-byte third continuation bits")
            })?;
        let lead_shifted = self
            .builder
            .build_left_shift(
                lead_bits,
                i32_type.const_int(18, false),
                "utf8_four_lead_shifted",
            )
            .map_err(|_| CodegenError::new("failed to shift UTF-8 four-byte lead bits"))?;
        let cont4_shifted = self
            .builder
            .build_left_shift(
                cont4_bits,
                i32_type.const_int(12, false),
                "utf8_four_cont4_shifted",
            )
            .map_err(|_| {
                CodegenError::new("failed to shift UTF-8 four-byte first continuation bits")
            })?;
        let cont5_shifted = self
            .builder
            .build_left_shift(
                cont5_bits,
                i32_type.const_int(6, false),
                "utf8_four_cont5_shifted",
            )
            .map_err(|_| {
                CodegenError::new("failed to shift UTF-8 four-byte second continuation bits")
            })?;
        let partial = self
            .builder
            .build_or(lead_shifted, cont4_shifted, "utf8_four_partial_1")
            .map_err(|_| {
                CodegenError::new("failed to combine UTF-8 four-byte partial codepoint")
            })?;
        let partial = self
            .builder
            .build_or(partial, cont5_shifted, "utf8_four_partial_2")
            .map_err(|_| CodegenError::new("failed to extend UTF-8 four-byte partial codepoint"))?;
        let codepoint = self
            .builder
            .build_or(partial, cont6_bits, "utf8_four_codepoint")
            .map_err(|_| CodegenError::new("failed to combine UTF-8 four-byte codepoint"))?;
        self.builder
            .build_store(char_result_slot, codepoint)
            .map_err(|_| CodegenError::new("failed to store UTF-8 four-byte codepoint"))?;
        self.builder
            .build_unconditional_branch(return_bb)
            .map_err(|_| CodegenError::new("failed to branch from UTF-8 four-byte decode"))?;

        self.builder.position_at_end(return_bb);
        self.builder
            .build_load(i32_type, char_result_slot, "utf8_string_char_result_load")
            .map_err(|_| CodegenError::new("failed to load UTF-8 string index result"))
    }

    pub(super) fn compile_utf8_string_length_runtime(
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
            .map_err(|_| {
                CodegenError::new("failed to allocate UTF-8 string length pointer slot")
            })?;
        let char_count_slot = self
            .builder
            .build_alloca(i64_type, "utf8_string_length_count")
            .map_err(|_| {
                CodegenError::new("failed to allocate UTF-8 string length counter slot")
            })?;

        self.builder
            .build_store(ptr_slot, string_ptr)
            .map_err(|_| {
                CodegenError::new("failed to initialize UTF-8 string length pointer slot")
            })?;
        self.builder
            .build_store(char_count_slot, i64_type.const_zero())
            .map_err(|_| CodegenError::new("failed to initialize UTF-8 string length counter"))?;
        self.builder
            .build_unconditional_branch(loop_bb)
            .map_err(|_| CodegenError::new("failed to branch into UTF-8 string length loop"))?;

        self.builder.position_at_end(fail_utf8_bb);
        self.emit_runtime_error(
            "Invalid UTF-8 sequence in String",
            "string_length_invalid_utf8",
        )?;

        self.builder.position_at_end(loop_bb);
        let current_ptr = self
            .builder
            .build_load(ptr_type, ptr_slot, "utf8_string_length_ptr_load")
            .map_err(|_| CodegenError::new("failed to load UTF-8 string length pointer"))?
            .into_pointer_value();
        let current_count = self
            .builder
            .build_load(i64_type, char_count_slot, "utf8_string_length_count_load")
            .map_err(|_| CodegenError::new("failed to load UTF-8 string length counter"))?
            .into_int_value();
        let lead_byte = self
            .builder
            .build_load(i8_type, current_ptr, "utf8_string_length_lead_byte")
            .map_err(|_| CodegenError::new("failed to load UTF-8 string length lead byte"))?
            .into_int_value();
        let lead_u32 = self
            .builder
            .build_int_z_extend(lead_byte, i32_type, "utf8_string_length_lead_u32")
            .map_err(|_| CodegenError::new("failed to extend UTF-8 string length lead byte"))?;
        let is_end = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_byte,
                i8_type.const_zero(),
                "utf8_string_length_is_end",
            )
            .map_err(|_| CodegenError::new("failed to check UTF-8 string length end"))?;
        let is_ascii = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                lead_u32,
                i32_type.const_int(0x80, false),
                "utf8_string_length_is_ascii",
            )
            .map_err(|_| CodegenError::new("failed to classify ASCII UTF-8 length byte"))?;
        let lead_mask_e0 = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0xE0, false),
                "utf8_string_length_mask_e0",
            )
            .map_err(|_| CodegenError::new("failed to mask UTF-8 length lead byte with 0xE0"))?;
        let lead_mask_f0 = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0xF0, false),
                "utf8_string_length_mask_f0",
            )
            .map_err(|_| CodegenError::new("failed to mask UTF-8 length lead byte with 0xF0"))?;
        let lead_mask_f8 = self
            .builder
            .build_and(
                lead_u32,
                i32_type.const_int(0xF8, false),
                "utf8_string_length_mask_f8",
            )
            .map_err(|_| CodegenError::new("failed to mask UTF-8 length lead byte with 0xF8"))?;
        let is_two = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_mask_e0,
                i32_type.const_int(0xC0, false),
                "utf8_string_length_is_two",
            )
            .map_err(|_| CodegenError::new("failed to classify UTF-8 two-byte length sequence"))?;
        let is_three = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_mask_f0,
                i32_type.const_int(0xE0, false),
                "utf8_string_length_is_three",
            )
            .map_err(|_| {
                CodegenError::new("failed to classify UTF-8 three-byte length sequence")
            })?;
        let is_four = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lead_mask_f8,
                i32_type.const_int(0xF0, false),
                "utf8_string_length_is_four",
            )
            .map_err(|_| CodegenError::new("failed to classify UTF-8 four-byte length sequence"))?;
        let width_two_or_zero = self
            .builder
            .build_select(
                is_two,
                i64_type.const_int(2, false),
                i64_type.const_zero(),
                "utf8_string_length_width_two",
            )
            .map_err(|_| {
                CodegenError::new(
                    "failed to select UTF-8 string length width for two-byte sequence",
                )
            })?
            .into_int_value();
        let width_three_or_prev = self
            .builder
            .build_select(
                is_three,
                i64_type.const_int(3, false),
                width_two_or_zero,
                "utf8_string_length_width_three",
            )
            .map_err(|_| {
                CodegenError::new(
                    "failed to select UTF-8 string length width for three-byte sequence",
                )
            })?
            .into_int_value();
        let width_nonzero = self
            .builder
            .build_select(
                is_ascii,
                i64_type.const_int(1, false),
                width_three_or_prev,
                "utf8_string_length_width_ascii",
            )
            .map_err(|_| {
                CodegenError::new("failed to select UTF-8 string length width for ASCII sequence")
            })?
            .into_int_value();
        let width = self
            .builder
            .build_select(
                is_four,
                i64_type.const_int(4, false),
                width_nonzero,
                "utf8_string_length_width",
            )
            .map_err(|_| {
                CodegenError::new(
                    "failed to select UTF-8 string length width for four-byte sequence",
                )
            })?
            .into_int_value();
        let width_is_valid = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                width,
                i64_type.const_zero(),
                "utf8_string_length_width_is_valid",
            )
            .map_err(|_| CodegenError::new("failed to validate UTF-8 string length width"))?;

        self.builder
            .build_conditional_branch(is_end, return_bb, continue_bb)
            .map_err(|_| CodegenError::new("failed to branch on UTF-8 string length end"))?;

        self.builder.position_at_end(continue_bb);
        self.builder
            .build_conditional_branch(width_is_valid, advance_bb, fail_utf8_bb)
            .map_err(|_| {
                CodegenError::new("failed to branch on UTF-8 string length width validity")
            })?;

        self.builder.position_at_end(advance_bb);
        let advanced_ptr = unsafe {
            self.builder
                .build_gep(
                    i8_type,
                    current_ptr,
                    &[width],
                    "utf8_string_length_advance_ptr",
                )
                .map_err(|_| CodegenError::new("failed to advance UTF-8 string length pointer"))?
        };
        let next_char_count = self
            .builder
            .build_int_add(
                current_count,
                i64_type.const_int(1, false),
                "utf8_string_length_next_count",
            )
            .map_err(|_| CodegenError::new("failed to increment UTF-8 string length counter"))?;
        self.builder
            .build_store(ptr_slot, advanced_ptr)
            .map_err(|_| {
                CodegenError::new("failed to store advanced UTF-8 string length pointer")
            })?;
        self.builder
            .build_store(char_count_slot, next_char_count)
            .map_err(|_| {
                CodegenError::new("failed to store advanced UTF-8 string length counter")
            })?;
        self.builder
            .build_unconditional_branch(loop_bb)
            .map_err(|_| CodegenError::new("failed to branch UTF-8 string length loop"))?;

        self.builder.position_at_end(return_bb);
        self.builder
            .build_load(i64_type, char_count_slot, "utf8_string_length_result")
            .map_err(|_| CodegenError::new("failed to load UTF-8 string length result"))
    }
}
