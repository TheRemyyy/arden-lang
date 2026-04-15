use super::*;

impl<'ctx> Codegen<'ctx> {
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
                        .map_err(|_| {
                            CodegenError::new(
                                "failed to compare Math.abs input against minimum value",
                            )
                        })?;
                    self.builder
                        .build_conditional_branch(is_min_value, overflow_bb, ok_bb)
                        .map_err(|_| {
                            CodegenError::new("failed to branch for Math.abs overflow check")
                        })?;

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
                        .map_err(|_| {
                            CodegenError::new("failed to compare Math.abs input against zero")
                        })?;
                    let neg = self.builder.build_int_neg(v, "neg").map_err(|_| {
                        CodegenError::new("failed to negate Math.abs integer input")
                    })?;
                    let result =
                        self.builder
                            .build_select(is_neg, neg, v, "abs")
                            .map_err(|_| {
                                CodegenError::new("failed to select Math.abs integer result")
                            })?;
                    Ok(Some(result))
                } else {
                    let fabs = self.get_or_declare_math_func("fabs", true);
                    let call = self
                        .builder
                        .build_call(fabs, &[val.into()], "abs")
                        .map_err(|_| CodegenError::new("failed to emit fabs call for Math.abs"))?;
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
                            .map_err(|_| {
                                CodegenError::new(
                                    "failed to promote first Math.min operand to float",
                                )
                            })?
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
                            .map_err(|_| {
                                CodegenError::new(
                                    "failed to promote second Math.min operand to float",
                                )
                            })?
                            .into()
                    };
                    let call = self
                        .builder
                        .build_call(fmin, &[av.into(), bv.into()], "min")
                        .map_err(|_| CodegenError::new("failed to emit fmin call"))?;
                    Ok(Some(self.extract_call_value(call)?))
                } else {
                    let av = a.into_int_value();
                    let bv = b.into_int_value();
                    let cond = self
                        .builder
                        .build_int_compare(IntPredicate::SLT, av, bv, "cmp")
                        .map_err(|_| {
                            CodegenError::new("failed to compare Math.min integer operands")
                        })?;
                    let result = self
                        .builder
                        .build_select(cond, av, bv, "min")
                        .map_err(|_| {
                            CodegenError::new("failed to select Math.min integer result")
                        })?;
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
                            .map_err(|_| {
                                CodegenError::new(
                                    "failed to promote first Math.max operand to float",
                                )
                            })?
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
                            .map_err(|_| {
                                CodegenError::new(
                                    "failed to promote second Math.max operand to float",
                                )
                            })?
                            .into()
                    };
                    let call = self
                        .builder
                        .build_call(fmax, &[av.into(), bv.into()], "max")
                        .map_err(|_| CodegenError::new("failed to emit fmax call"))?;
                    Ok(Some(self.extract_call_value(call)?))
                } else {
                    let av = a.into_int_value();
                    let bv = b.into_int_value();
                    let cond = self
                        .builder
                        .build_int_compare(IntPredicate::SGT, av, bv, "cmp")
                        .map_err(|_| {
                            CodegenError::new("failed to compare Math.max integer operands")
                        })?;
                    let result = self
                        .builder
                        .build_select(cond, av, bv, "max")
                        .map_err(|_| {
                            CodegenError::new("failed to select Math.max integer result")
                        })?;
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
                        .map_err(|_| {
                            CodegenError::new("failed to promote Math.sqrt operand to float")
                        })?
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(sqrt, &[fval.into()], "sqrt")
                    .map_err(|_| CodegenError::new("failed to emit sqrt call"))?;
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
                        .map_err(|_| CodegenError::new("failed to promote Math.pow base to float"))?
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
                        .map_err(|_| {
                            CodegenError::new("failed to promote Math.pow exponent to float")
                        })?
                        .into()
                } else {
                    exp
                };
                let call = self
                    .builder
                    .build_call(pow_fn, &[fbase.into(), fexp.into()], "pow")
                    .map_err(|_| CodegenError::new("failed to emit pow call"))?;
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
                        .map_err(|_| {
                            CodegenError::new("failed to promote Math.sin operand to float")
                        })?
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(sin_fn, &[fval.into()], "sin")
                    .map_err(|_| CodegenError::new("failed to emit sin call"))?;
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
                        .map_err(|_| {
                            CodegenError::new("failed to promote Math.cos operand to float")
                        })?
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(cos_fn, &[fval.into()], "cos")
                    .map_err(|_| CodegenError::new("failed to emit cos call"))?;
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
                        .map_err(|_| {
                            CodegenError::new("failed to promote Math.tan operand to float")
                        })?
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(tan_fn, &[fval.into()], "tan")
                    .map_err(|_| CodegenError::new("failed to emit tan call"))?;
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
                        .map_err(|_| {
                            CodegenError::new("failed to promote Math.floor operand to float")
                        })?
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(floor_fn, &[fval.into()], "floor")
                    .map_err(|_| CodegenError::new("failed to emit floor call"))?;
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
                        .map_err(|_| {
                            CodegenError::new("failed to promote Math.ceil operand to float")
                        })?
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(ceil_fn, &[fval.into()], "ceil")
                    .map_err(|_| CodegenError::new("failed to emit ceil call"))?;
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
                        .map_err(|_| {
                            CodegenError::new("failed to promote Math.round operand to float")
                        })?
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(round_fn, &[fval.into()], "round")
                    .map_err(|_| CodegenError::new("failed to emit round call"))?;
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
                        .map_err(|_| {
                            CodegenError::new("failed to promote Math.log operand to float")
                        })?
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(log_fn, &[fval.into()], "log")
                    .map_err(|_| CodegenError::new("failed to emit log call"))?;
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
                        .map_err(|_| {
                            CodegenError::new("failed to promote Math.log10 operand to float")
                        })?
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(log10_fn, &[fval.into()], "log10")
                    .map_err(|_| CodegenError::new("failed to emit log10 call"))?;
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
                        .map_err(|_| {
                            CodegenError::new("failed to promote Math.exp operand to float")
                        })?
                        .into()
                } else {
                    val
                };
                let call = self
                    .builder
                    .build_call(exp_fn, &[fval.into()], "exp")
                    .map_err(|_| CodegenError::new("failed to emit exp call"))?;
                Ok(Some(self.extract_call_value(call)?))
            }

            "Math__random" => {
                let rand_fn = self.get_or_declare_rand();
                let res = self
                    .builder
                    .build_call(rand_fn, &[], "r")
                    .map_err(|_| CodegenError::new("failed to emit rand call"))?;
                let val = self.extract_call_value(res)?.into_int_value();
                let fval = self
                    .builder
                    .build_unsigned_int_to_float(val, self.context.f64_type(), "rf")
                    .map_err(|_| CodegenError::new("failed to convert rand result to float"))?;
                let rand_max = self.context.f64_type().const_float(32767.0);
                let norm = self
                    .builder
                    .build_float_div(fval, rand_max, "rnd")
                    .map_err(|_| CodegenError::new("failed to normalize rand result"))?;
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
                        .map_err(|_| CodegenError::new("failed to convert Integer to Float"))?;
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
                        .map_err(|_| CodegenError::new("failed to convert Float to Integer"))?;
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
                        .map_err(|_| CodegenError::new("failed to emit strtoll call"))?;
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
                    .map_err(|_| CodegenError::new("failed to emit strcmp for Str.compare"))?;
                // strcmp returns i32, extend to i64
                let result = self.extract_call_value(call)?.into_int_value();
                let extended = self
                    .builder
                    .build_int_s_extend(result, self.context.i64_type(), "cmp64")
                    .map_err(|_| CodegenError::new("failed to extend Str.compare result"))?;
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
                let strcpy_fn = self.get_or_declare_strcpy();
                let strcat_fn = self.get_or_declare_strcat();

                // Get lengths
                let len1_call = self
                    .builder
                    .build_call(strlen_fn, &[s1.into()], "len1")
                    .map_err(|_| {
                        CodegenError::new("failed to emit strlen for first Str.concat argument")
                    })?;
                let len1 = self.extract_call_value(len1_call)?.into_int_value();
                let len2_call = self
                    .builder
                    .build_call(strlen_fn, &[s2.into()], "len2")
                    .map_err(|_| {
                        CodegenError::new("failed to emit strlen for second Str.concat argument")
                    })?;
                let len2 = self.extract_call_value(len2_call)?.into_int_value();

                // Allocate len1 + len2 + 1
                let total_len = self
                    .builder
                    .build_int_add(len1, len2, "total")
                    .map_err(|_| CodegenError::new("failed to compute Str.concat length"))?;
                let buffer_size = self
                    .builder
                    .build_int_add(
                        total_len,
                        self.context.i64_type().const_int(1, false),
                        "bufsize",
                    )
                    .map_err(|_| CodegenError::new("failed to compute Str.concat buffer size"))?;

                let buffer_call = self.build_malloc_call(
                    buffer_size,
                    "buf",
                    "failed to allocate Str.concat buffer",
                )?;
                let buffer = self.extract_call_value(buffer_call)?.into_pointer_value();

                // strcpy(buffer, s1)
                self.builder
                    .build_call(strcpy_fn, &[buffer.into(), s1.into()], "")
                    .map_err(|_| CodegenError::new("failed to emit strcpy for Str.concat"))?;
                // strcat(buffer, s2)
                self.builder
                    .build_call(strcat_fn, &[buffer.into(), s2.into()], "")
                    .map_err(|_| CodegenError::new("failed to emit strcat for Str.concat"))?;

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
                let strncpy_fn = self.get_or_declare_strncpy();

                let len_call = self
                    .builder
                    .build_call(strlen_fn, &[s_ptr.into()], "len")
                    .map_err(|_| CodegenError::new("failed to emit strlen for Str.trim"))?;
                let len = self.extract_call_value(len_call)?.into_int_value();

                // Find start (first non-space)
                let start_ptr = self
                    .builder
                    .build_alloca(self.context.i64_type(), "start")
                    .map_err(|_| CodegenError::new("failed to allocate Str.trim start slot"))?;
                self.builder
                    .build_store(start_ptr, self.context.i64_type().const_int(0, false))
                    .map_err(|_| CodegenError::new("failed to initialize Str.trim start slot"))?;

                let cur_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("Str.trim used outside function"))?;
                let start_cond = self.context.append_basic_block(cur_fn, "trim.start.cond");
                let start_body = self.context.append_basic_block(cur_fn, "trim.start.body");
                let start_after = self.context.append_basic_block(cur_fn, "trim.start.after");
                self.builder
                    .build_unconditional_branch(start_cond)
                    .map_err(|_| CodegenError::new("failed to branch into Str.trim start scan"))?;

                self.builder.position_at_end(start_cond);
                let start_val = self
                    .builder
                    .build_load(self.context.i64_type(), start_ptr, "s")
                    .map_err(|_| CodegenError::new("failed to load Str.trim start index"))?
                    .into_int_value();
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, start_val, len, "bounds")
                    .map_err(|_| CodegenError::new("failed to compare Str.trim start bounds"))?;
                let char_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), s_ptr, &[start_val], "")
                        .map_err(|_| {
                            CodegenError::new("failed to access Str.trim start character")
                        })?
                };
                let char_val = self
                    .builder
                    .build_load(self.context.i8_type(), char_ptr, "")
                    .map_err(|_| CodegenError::new("failed to load Str.trim start character"))?;
                let char_i32 = self
                    .builder
                    .build_int_s_extend(char_val.into_int_value(), self.context.i32_type(), "")
                    .map_err(|_| CodegenError::new("failed to extend Str.trim start character"))?;
                let is_space_call = self
                    .builder
                    .build_call(isspace_fn, &[char_i32.into()], "")
                    .map_err(|_| CodegenError::new("failed to emit isspace for Str.trim start"))?;
                let is_space = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        self.extract_call_value(is_space_call)?.into_int_value(),
                        self.context.i32_type().const_int(0, false),
                        "",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compare Str.trim start whitespace test")
                    })?;
                let cond = self
                    .builder
                    .build_and(in_bounds, is_space, "")
                    .map_err(|_| CodegenError::new("failed to combine Str.trim start condition"))?;
                self.builder
                    .build_conditional_branch(cond, start_body, start_after)
                    .map_err(|_| CodegenError::new("failed to branch in Str.trim start scan"))?;

                self.builder.position_at_end(start_body);
                let next_start = self
                    .builder
                    .build_int_add(start_val, self.context.i64_type().const_int(1, false), "")
                    .map_err(|_| CodegenError::new("failed to increment Str.trim start index"))?;
                self.builder
                    .build_store(start_ptr, next_start)
                    .map_err(|_| CodegenError::new("failed to store Str.trim start index"))?;
                self.builder
                    .build_unconditional_branch(start_cond)
                    .map_err(|_| CodegenError::new("failed to loop Str.trim start scan"))?;

                self.builder.position_at_end(start_after);
                let start_final = self
                    .builder
                    .build_load(self.context.i64_type(), start_ptr, "start_f")
                    .map_err(|_| {
                        CodegenError::new("failed to load Str.trim finalized start index")
                    })?
                    .into_int_value();

                // Find end (last non-space)
                let end_ptr = self
                    .builder
                    .build_alloca(self.context.i64_type(), "end")
                    .map_err(|_| CodegenError::new("failed to allocate Str.trim end slot"))?;
                self.builder
                    .build_store(end_ptr, len)
                    .map_err(|_| CodegenError::new("failed to initialize Str.trim end slot"))?;

                let end_cond = self.context.append_basic_block(cur_fn, "trim.end.cond");
                let end_body = self.context.append_basic_block(cur_fn, "trim.end.body");
                let end_after = self.context.append_basic_block(cur_fn, "trim.end.after");
                self.builder
                    .build_unconditional_branch(end_cond)
                    .map_err(|_| CodegenError::new("failed to branch into Str.trim end scan"))?;

                self.builder.position_at_end(end_cond);
                let end_val = self
                    .builder
                    .build_load(self.context.i64_type(), end_ptr, "e")
                    .map_err(|_| CodegenError::new("failed to load Str.trim end index"))?
                    .into_int_value();
                let gt_start = self
                    .builder
                    .build_int_compare(IntPredicate::SGT, end_val, start_final, "gt_start")
                    .map_err(|_| CodegenError::new("failed to compare Str.trim end bounds"))?;
                let last_idx = self
                    .builder
                    .build_int_sub(end_val, self.context.i64_type().const_int(1, false), "")
                    .map_err(|_| CodegenError::new("failed to compute Str.trim last index"))?;
                let char_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), s_ptr, &[last_idx], "")
                        .map_err(|_| CodegenError::new("failed to access Str.trim end character"))?
                };
                let char_val = self
                    .builder
                    .build_load(self.context.i8_type(), char_ptr, "")
                    .map_err(|_| CodegenError::new("failed to load Str.trim end character"))?;
                let char_i32 = self
                    .builder
                    .build_int_s_extend(char_val.into_int_value(), self.context.i32_type(), "")
                    .map_err(|_| CodegenError::new("failed to extend Str.trim end character"))?;
                let is_space_call = self
                    .builder
                    .build_call(isspace_fn, &[char_i32.into()], "")
                    .map_err(|_| CodegenError::new("failed to emit isspace for Str.trim end"))?;
                let is_space = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        self.extract_call_value(is_space_call)?.into_int_value(),
                        self.context.i32_type().const_int(0, false),
                        "",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compare Str.trim end whitespace test")
                    })?;
                let cond = self
                    .builder
                    .build_and(gt_start, is_space, "")
                    .map_err(|_| CodegenError::new("failed to combine Str.trim end condition"))?;
                self.builder
                    .build_conditional_branch(cond, end_body, end_after)
                    .map_err(|_| CodegenError::new("failed to branch in Str.trim end scan"))?;

                self.builder.position_at_end(end_body);
                let next_end = self
                    .builder
                    .build_int_sub(end_val, self.context.i64_type().const_int(1, false), "")
                    .map_err(|_| CodegenError::new("failed to decrement Str.trim end index"))?;
                self.builder
                    .build_store(end_ptr, next_end)
                    .map_err(|_| CodegenError::new("failed to store Str.trim end index"))?;
                self.builder
                    .build_unconditional_branch(end_cond)
                    .map_err(|_| CodegenError::new("failed to loop Str.trim end scan"))?;

                self.builder.position_at_end(end_after);
                let end_final = self
                    .builder
                    .build_load(self.context.i64_type(), end_ptr, "end_f")
                    .map_err(|_| CodegenError::new("failed to load Str.trim finalized end index"))?
                    .into_int_value();

                // Allocate and copy result
                let new_len = self
                    .builder
                    .build_int_sub(end_final, start_final, "new_len")
                    .map_err(|_| CodegenError::new("failed to compute Str.trim output length"))?;
                let alloc_size = self
                    .builder
                    .build_int_add(
                        new_len,
                        self.context.i64_type().const_int(1, false),
                        "alloc",
                    )
                    .map_err(|_| CodegenError::new("failed to compute Str.trim allocation size"))?;
                let buf_call = self.build_malloc_call(
                    alloc_size,
                    "buf",
                    "failed to allocate Str.trim buffer",
                )?;
                let buf = self.extract_call_value(buf_call)?.into_pointer_value();

                let src_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), s_ptr, &[start_final], "src")
                        .map_err(|_| CodegenError::new("failed to access Str.trim source slice"))?
                };
                self.builder
                    .build_call(
                        strncpy_fn,
                        &[buf.into(), src_ptr.into(), new_len.into()],
                        "",
                    )
                    .map_err(|_| CodegenError::new("failed to emit strncpy for Str.trim"))?;

                // Null terminate
                let term_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), buf, &[new_len], "")
                        .map_err(|_| {
                            CodegenError::new("failed to access Str.trim terminator slot")
                        })?
                };
                self.builder
                    .build_store(term_ptr, self.context.i8_type().const_int(0, false))
                    .map_err(|_| CodegenError::new("failed to null-terminate Str.trim result"))?;

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
                    .map_err(|_| CodegenError::new("failed to emit strstr for Str.contains"))?;
                let ptr = self.extract_call_value(res)?.into_pointer_value();
                let is_null = self.builder.build_is_null(ptr, "not_found").map_err(|_| {
                    CodegenError::new("failed to test Str.contains result for null")
                })?;
                let found = self
                    .builder
                    .build_not(is_null, "found")
                    .map_err(|_| CodegenError::new("failed to negate Str.contains null test"))?;
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
                    .map_err(|_| {
                        CodegenError::new("failed to emit strlen for Str.startsWith prefix")
                    })?;
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
                    .map_err(|_| CodegenError::new("failed to emit strncmp for Str.startsWith"))?;
                let is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        self.extract_call_value(res)?.into_int_value(),
                        self.context.i32_type().const_int(0, false),
                        "is_zero",
                    )
                    .map_err(|_| CodegenError::new("failed to compare Str.startsWith result"))?;
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
                    .map_err(|_| {
                        CodegenError::new("failed to emit strlen for Str.endsWith input")
                    })?;
                let suf_len = self
                    .builder
                    .build_call(strlen, &[suf.into()], "suf_len")
                    .map_err(|_| {
                        CodegenError::new("failed to emit strlen for Str.endsWith suffix")
                    })?;

                let s_len_val = self.extract_call_value(s_len)?.into_int_value();
                let suf_len_val = self.extract_call_value(suf_len)?.into_int_value();

                let can_end = self
                    .builder
                    .build_int_compare(IntPredicate::UGE, s_len_val, suf_len_val, "can_end")
                    .map_err(|_| CodegenError::new("failed to compare Str.endsWith lengths"))?;

                let start_idx = self
                    .builder
                    .build_int_sub(s_len_val, suf_len_val, "")
                    .map_err(|_| {
                        CodegenError::new("failed to compute Str.endsWith suffix start")
                    })?;
                let s_suffix_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), s, &[start_idx], "")
                        .map_err(|_| {
                            CodegenError::new("failed to access Str.endsWith suffix pointer")
                        })?
                };

                let res = self
                    .builder
                    .build_call(strcmp, &[s_suffix_ptr.into(), suf.into()], "cmp")
                    .map_err(|_| CodegenError::new("failed to emit strcmp for Str.endsWith"))?;
                let is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        self.extract_call_value(res)?.into_int_value(),
                        self.context.i32_type().const_int(0, false),
                        "is_zero",
                    )
                    .map_err(|_| CodegenError::new("failed to compare Str.endsWith result"))?;

                let final_res = self
                    .builder
                    .build_and(can_end, is_zero, "")
                    .map_err(|_| CodegenError::new("failed to combine Str.endsWith conditions"))?;
                Ok(Some(final_res.into()))
            }

            // I/O functions
            "read_line" => {
                // Read a line from stdin with a growing buffer so long lines do not
                // truncate and we do not depend on platform-specific stdin symbols.
                let getchar_fn = self.get_or_declare_getchar();

                let i8_type = self.context.i8_type();
                let i32_type = self.context.i32_type();
                let i64_type = self.context.i64_type();
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let chunk_chars = i64_type.const_int(1024, false);
                let initial_capacity = i64_type.const_int(1025, false);
                let buffer_call = self.build_malloc_call(
                    initial_capacity,
                    "linebuf",
                    "failed to allocate read_line buffer",
                )?;
                let buffer = self.extract_call_value(buffer_call)?.into_pointer_value();
                let buffer_slot = self
                    .builder
                    .build_alloca(ptr_type, "read_line_buffer_slot")
                    .map_err(|_| CodegenError::new("failed to allocate read_line buffer slot"))?;
                let capacity_slot = self
                    .builder
                    .build_alloca(i64_type, "read_line_capacity_slot")
                    .map_err(|_| CodegenError::new("failed to allocate read_line capacity slot"))?;
                let total_read_slot = self
                    .builder
                    .build_alloca(i64_type, "read_line_total_slot")
                    .map_err(|_| CodegenError::new("failed to allocate read_line total slot"))?;
                self.builder
                    .build_store(buffer_slot, buffer)
                    .map_err(|_| CodegenError::new("failed to initialize read_line buffer slot"))?;
                self.builder
                    .build_store(capacity_slot, initial_capacity)
                    .map_err(|_| {
                        CodegenError::new("failed to initialize read_line capacity slot")
                    })?;
                self.builder
                    .build_store(total_read_slot, i64_type.const_zero())
                    .map_err(|_| CodegenError::new("failed to initialize read_line total slot"))?;
                self.builder
                    .build_store(buffer, i8_type.const_zero())
                    .map_err(|_| {
                        CodegenError::new("failed to initialize read_line buffer terminator")
                    })?;

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
                    .map_err(|_| CodegenError::new("failed to branch into read_line loop"))?;

                self.builder.position_at_end(read_cond_bb);
                let current_capacity = self
                    .builder
                    .build_load(i64_type, capacity_slot, "read_line_capacity")
                    .map_err(|_| CodegenError::new("failed to load read_line capacity"))?
                    .into_int_value();
                let current_total = self
                    .builder
                    .build_load(i64_type, total_read_slot, "read_line_total")
                    .map_err(|_| CodegenError::new("failed to load read_line total"))?
                    .into_int_value();
                let remaining_capacity = self
                    .builder
                    .build_int_sub(
                        current_capacity,
                        current_total,
                        "read_line_remaining_capacity",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compute read_line remaining capacity")
                    })?;
                let enough_room = self
                    .builder
                    .build_int_compare(
                        IntPredicate::UGT,
                        remaining_capacity,
                        i64_type.const_int(1, false),
                        "read_line_enough_room",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compare read_line remaining capacity")
                    })?;
                self.builder
                    .build_conditional_branch(enough_room, read_body_bb, grow_bb)
                    .map_err(|_| CodegenError::new("failed to branch on read_line capacity"))?;

                self.builder.position_at_end(read_body_bb);
                let getchar_call = self
                    .builder
                    .build_call(getchar_fn, &[], "read_line_char")
                    .map_err(|_| CodegenError::new("failed to emit getchar for read_line"))?;
                let char_i32 = self.extract_call_value(getchar_call)?.into_int_value();
                let is_eof = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        char_i32,
                        i32_type.const_int(u32::MAX as u64, false),
                        "read_line_is_eof",
                    )
                    .map_err(|_| CodegenError::new("failed to compare read_line EOF sentinel"))?;
                self.builder
                    .build_conditional_branch(is_eof, eof_bb, append_bb)
                    .map_err(|_| CodegenError::new("failed to branch on read_line EOF"))?;

                self.builder.position_at_end(eof_bb);
                self.builder
                    .build_unconditional_branch(done_bb)
                    .map_err(|_| CodegenError::new("failed to branch read_line EOF to done"))?;

                self.builder.position_at_end(append_bb);
                let current_buffer = self
                    .builder
                    .build_load(ptr_type, buffer_slot, "read_line_buffer")
                    .map_err(|_| CodegenError::new("failed to load read_line buffer pointer"))?
                    .into_pointer_value();
                let write_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(
                            i8_type,
                            current_buffer,
                            &[current_total],
                            "read_line_write_ptr",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to access read_line write pointer")
                        })?
                };
                let char_i8 = self
                    .builder
                    .build_int_truncate(char_i32, i8_type, "read_line_char_i8")
                    .map_err(|_| CodegenError::new("failed to truncate read_line character"))?;
                self.builder
                    .build_store(write_ptr, char_i8)
                    .map_err(|_| CodegenError::new("failed to store read_line character"))?;
                let next_total = self
                    .builder
                    .build_int_add(
                        current_total,
                        i64_type.const_int(1, false),
                        "read_line_next_total",
                    )
                    .map_err(|_| CodegenError::new("failed to increment read_line total"))?;
                self.builder
                    .build_store(total_read_slot, next_total)
                    .map_err(|_| CodegenError::new("failed to store read_line total"))?;
                let term_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(i8_type, current_buffer, &[next_total], "read_line_term_ptr")
                        .map_err(|_| {
                            CodegenError::new("failed to access read_line terminator slot")
                        })?
                };
                self.builder
                    .build_store(term_ptr, i8_type.const_zero())
                    .map_err(|_| CodegenError::new("failed to store read_line terminator"))?;
                let saw_newline = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        char_i8,
                        i8_type.const_int(b'\n' as u64, false),
                        "read_line_saw_newline",
                    )
                    .map_err(|_| CodegenError::new("failed to compare read_line newline"))?;
                self.builder
                    .build_conditional_branch(saw_newline, done_bb, read_cond_bb)
                    .map_err(|_| CodegenError::new("failed to branch on read_line newline"))?;

                self.builder.position_at_end(grow_bb);
                let grown_capacity = self
                    .builder
                    .build_int_add(current_capacity, chunk_chars, "read_line_new_capacity")
                    .map_err(|_| CodegenError::new("failed to compute grown read_line capacity"))?;
                let grown_buffer = self
                    .builder
                    .build_load(ptr_type, buffer_slot, "read_line_grow_buffer")
                    .map_err(|_| CodegenError::new("failed to load read_line grow buffer"))?
                    .into_pointer_value();
                let realloc_call = self.build_realloc_call(
                    grown_buffer,
                    grown_capacity,
                    "read_line_realloc",
                    "failed to emit realloc for read_line",
                )?;
                let realloc_ptr = self.extract_call_value(realloc_call)?.into_pointer_value();
                let realloc_failed = self
                    .builder
                    .build_is_null(realloc_ptr, "read_line_realloc_failed")
                    .map_err(|_| CodegenError::new("failed to test read_line realloc result"))?;
                self.builder
                    .build_conditional_branch(realloc_failed, oom_bb, grow_ok_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch on read_line realloc result")
                    })?;

                self.builder.position_at_end(oom_bb);
                self.emit_runtime_error("read_line() out of memory", "read_line_out_of_memory")?;

                self.builder.position_at_end(grow_ok_bb);
                self.builder
                    .build_store(buffer_slot, realloc_ptr)
                    .map_err(|_| CodegenError::new("failed to store grown read_line buffer"))?;
                self.builder
                    .build_store(capacity_slot, grown_capacity)
                    .map_err(|_| CodegenError::new("failed to store grown read_line capacity"))?;
                self.builder
                    .build_unconditional_branch(read_cond_bb)
                    .map_err(|_| CodegenError::new("failed to loop read_line after growth"))?;

                self.builder.position_at_end(done_bb);
                let final_buffer = self
                    .builder
                    .build_load(ptr_type, buffer_slot, "read_line_final_buffer")
                    .map_err(|_| CodegenError::new("failed to load final read_line buffer"))?;

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
                    .map_err(|_| CodegenError::new("failed to truncate exit code to i32"))?;
                self.builder
                    .build_call(exit_fn, &[code_i32.into()], "")
                    .map_err(|_| CodegenError::new("failed to emit exit call"))?;
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
                    .map_err(|_| CodegenError::new("failed to emit fopen for File.write"))?;
                let file_ptr = self.extract_call_value(file_call)?.into_pointer_value();

                let is_null = self
                    .builder
                    .build_is_null(file_ptr, "is_null")
                    .map_err(|_| CodegenError::new("failed to test File.write file pointer"))?;

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
                    .map_err(|_| CodegenError::new("failed to branch on File.write open result"))?;

                // Fail
                self.builder.position_at_end(fail_block);
                self.builder
                    .build_unconditional_branch(merge_block)
                    .map_err(|_| CodegenError::new("failed to branch File.write open failure"))?;

                // Success
                self.builder.position_at_end(success_block);
                let write_result = self
                    .builder
                    .build_call(fputs, &[content.into(), file_ptr.into()], "write")
                    .map_err(|_| CodegenError::new("failed to emit fputs for File.write"))?;
                let write_code = self.extract_call_value(write_result)?.into_int_value();
                let close_result = self
                    .builder
                    .build_call(fclose, &[file_ptr.into()], "close")
                    .map_err(|_| CodegenError::new("failed to emit fclose for File.write"))?;
                let close_code = self.extract_call_value(close_result)?.into_int_value();
                let write_failed = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        write_code,
                        self.context.i32_type().const_zero(),
                        "file_write_failed",
                    )
                    .map_err(|_| CodegenError::new("failed to compare File.write result"))?;
                let close_failed = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        close_code,
                        self.context.i32_type().const_zero(),
                        "file_close_failed",
                    )
                    .map_err(|_| CodegenError::new("failed to compare File.write close result"))?;
                let io_failed = self
                    .builder
                    .build_or(write_failed, close_failed, "file_io_failed")
                    .map_err(|_| {
                        CodegenError::new("failed to combine File.write failure conditions")
                    })?;
                self.builder
                    .build_conditional_branch(io_failed, write_fail_block, write_ok_block)
                    .map_err(|_| CodegenError::new("failed to branch on File.write I/O result"))?;

                self.builder.position_at_end(write_fail_block);
                self.builder
                    .build_unconditional_branch(merge_block)
                    .map_err(|_| {
                        CodegenError::new("failed to branch File.write failure to merge")
                    })?;

                self.builder.position_at_end(write_ok_block);
                self.builder
                    .build_unconditional_branch(merge_block)
                    .map_err(|_| {
                        CodegenError::new("failed to branch File.write success to merge")
                    })?;

                // Merge
                self.builder.position_at_end(merge_block);
                let phi = self
                    .builder
                    .build_phi(self.context.bool_type(), "result")
                    .map_err(|_| CodegenError::new("failed to build File.write result phi"))?;
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
                    .map_err(|_| CodegenError::new("failed to emit fopen for File.read"))?;
                let file_ptr = self.extract_call_value(file_call)?.into_pointer_value();

                let is_null = self
                    .builder
                    .build_is_null(file_ptr, "is_null")
                    .map_err(|_| CodegenError::new("failed to test File.read file pointer"))?;

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
                    .map_err(|_| CodegenError::new("failed to branch on File.read open result"))?;

                self.builder.position_at_end(fail_block);
                self.emit_runtime_error(
                    "File.read() failed to open file",
                    "file_read_open_failed",
                )?;

                // Success
                self.builder.position_at_end(success_block);
                // fseek(f, 0, SEEK_END)
                let seek_end = self.context.i32_type().const_int(2, false); // SEEK_END = 2
                let libc_long = self.libc_long_type();
                let zero = libc_long.const_zero();
                let seek_result = self
                    .builder
                    .build_call(fseek, &[file_ptr.into(), zero.into(), seek_end.into()], "")
                    .map_err(|_| CodegenError::new("failed to emit fseek for File.read"))?;
                let seek_code = self.extract_call_value(seek_result)?.into_int_value();
                let seek_succeeded = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        seek_code,
                        self.context.i32_type().const_zero(),
                        "file_read_seek_succeeded",
                    )
                    .map_err(|_| CodegenError::new("failed to compare File.read seek result"))?;
                self.builder
                    .build_conditional_branch(seek_succeeded, seek_ok_block, seek_fail_block)
                    .map_err(|_| CodegenError::new("failed to branch on File.read seek result"))?;

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
                    .map_err(|_| CodegenError::new("failed to emit ftell for File.read"))?;
                let size = self.extract_call_value(size_call)?.into_int_value();
                let size_non_negative = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SGE,
                        size,
                        libc_long.const_zero(),
                        "file_read_size_non_negative",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compare File.read size against zero")
                    })?;
                self.builder
                    .build_conditional_branch(size_non_negative, size_ok_block, size_fail_block)
                    .map_err(|_| {
                        CodegenError::new("failed to branch on File.read size validity")
                    })?;

                self.builder.position_at_end(size_fail_block);
                self.emit_runtime_error(
                    "File.read() requires a seekable regular file",
                    "file_read_invalid_size",
                )?;

                self.builder.position_at_end(size_ok_block);

                // rewind(f)
                self.builder
                    .build_call(rewind, &[file_ptr.into()], "")
                    .map_err(|_| CodegenError::new("failed to emit rewind for File.read"))?;

                // buffer = malloc(size + 1)
                let size_t = self.libc_size_type();
                let one = size_t.const_int(1, false);
                let size_size_t = self
                    .builder
                    .build_int_cast(size, size_t, "file_read_size_size_t")
                    .map_err(|_| CodegenError::new("failed to cast File.read size to size_t"))?;
                let alloc_size = self
                    .builder
                    .build_int_add(size_size_t, one, "alloc_size")
                    .map_err(|_| {
                        CodegenError::new("failed to compute File.read allocation size")
                    })?;
                let buffer_call = self.build_malloc_call(
                    alloc_size,
                    "buffer",
                    "failed to allocate File.read buffer",
                )?;
                let buffer = self.extract_call_value(buffer_call)?.into_pointer_value();

                // read_count = fread(buffer, 1, size, f)
                let read_call = self
                    .builder
                    .build_call(
                        fread,
                        &[
                            buffer.into(),
                            one.into(),
                            size_size_t.into(),
                            file_ptr.into(),
                        ],
                        "file_read_count",
                    )
                    .map_err(|_| CodegenError::new("failed to emit fread for File.read"))?;
                let read_count = self.extract_call_value(read_call)?.into_int_value();

                // fclose(f) as soon as raw bytes are read.
                self.builder
                    .build_call(fclose, &[file_ptr.into()], "")
                    .map_err(|_| CodegenError::new("failed to emit fclose for File.read"))?;

                let read_complete = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        read_count,
                        size_size_t,
                        "file_read_complete",
                    )
                    .map_err(|_| CodegenError::new("failed to compare File.read byte count"))?;
                let read_ok_block = self.context.append_basic_block(current_fn, "read.read_ok");
                let read_fail_block = self
                    .context
                    .append_basic_block(current_fn, "read.read_fail");
                self.builder
                    .build_conditional_branch(read_complete, read_ok_block, read_fail_block)
                    .map_err(|_| {
                        CodegenError::new("failed to branch on File.read byte count result")
                    })?;

                self.builder.position_at_end(read_fail_block);
                self.emit_runtime_error(
                    "File.read() failed to read entire file",
                    "file_read_incomplete",
                )?;

                self.builder.position_at_end(read_ok_block);
                // Reject embedded NUL bytes to preserve null-terminated String invariants.
                let memchr = self.get_or_declare_memchr();
                let nul_scan_call = self
                    .builder
                    .build_call(
                        memchr,
                        &[
                            buffer.into(),
                            self.context.i32_type().const_zero().into(),
                            read_count.into(),
                        ],
                        "file_read_nul_scan",
                    )
                    .map_err(|_| CodegenError::new("failed to emit memchr for File.read"))?;
                let nul_ptr = self.extract_call_value(nul_scan_call)?.into_pointer_value();
                let has_nul = self
                    .builder
                    .build_is_not_null(nul_ptr, "file_read_has_nul")
                    .map_err(|_| CodegenError::new("failed to test File.read memchr result"))?;
                let nul_fail_block = self.context.append_basic_block(current_fn, "read.nul_fail");
                let nul_ok_block = self.context.append_basic_block(current_fn, "read.nul_ok");
                self.builder
                    .build_conditional_branch(has_nul, nul_fail_block, nul_ok_block)
                    .map_err(|_| {
                        CodegenError::new("failed to branch on File.read NUL-byte detection")
                    })?;

                self.builder.position_at_end(nul_fail_block);
                self.emit_runtime_error("File.read() cannot load NUL bytes", "file_read_nul_byte")?;

                self.builder.position_at_end(nul_ok_block);
                // buffer[read_count] = 0 (null terminate)
                let term_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), buffer, &[read_count], "term_ptr")
                        .map_err(|_| {
                            CodegenError::new("failed to access File.read terminator slot")
                        })?
                };
                self.builder
                    .build_store(term_ptr, self.context.i8_type().const_int(0, false))
                    .map_err(|_| CodegenError::new("failed to null-terminate File.read buffer"))?;

                self.compile_utf8_string_length_runtime(buffer)?;

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
                    .map_err(|_| CodegenError::new("failed to emit fopen for File.exists"))?;
                let file_ptr = self.extract_call_value(file_call)?.into_pointer_value();

                let is_null = self
                    .builder
                    .build_is_null(file_ptr, "is_null")
                    .map_err(|_| CodegenError::new("failed to test File.exists file pointer"))?;
                let alloca_exists_slot = self
                    .builder
                    .build_alloca(self.context.bool_type(), "exists_result_slot")
                    .map_err(|_| CodegenError::new("failed to allocate File.exists result slot"))?;
                self.builder
                    .build_store(alloca_exists_slot, self.context.bool_type().const_zero())
                    .map_err(|_| {
                        CodegenError::new("failed to initialize File.exists result slot")
                    })?;

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("File.exists used outside function"))?;
                let probe_block = self.context.append_basic_block(current_fn, "exists.probe");
                let end_block = self.context.append_basic_block(current_fn, "exists.end");

                self.builder
                    .build_conditional_branch(is_null, end_block, probe_block)
                    .map_err(|_| {
                        CodegenError::new("failed to branch on File.exists open result")
                    })?;

                self.builder.position_at_end(probe_block);
                let buf_slot = self
                    .builder
                    .build_alloca(self.context.i8_type(), "exists_buf")
                    .map_err(|_| {
                        CodegenError::new("failed to allocate File.exists probe buffer")
                    })?;
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
                    .map_err(|_| CodegenError::new("failed to emit fread for File.exists"))?;
                let err_call = self
                    .builder
                    .build_call(ferror, &[file_ptr.into()], "exists_err")
                    .map_err(|_| CodegenError::new("failed to emit ferror for File.exists"))?;
                let err_code = self.extract_call_value(err_call)?.into_int_value();
                self.builder
                    .build_call(fclose, &[file_ptr.into()], "")
                    .map_err(|_| CodegenError::new("failed to emit fclose for File.exists"))?;
                let is_regular = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        err_code,
                        self.context.i32_type().const_zero(),
                        "exists_is_regular",
                    )
                    .map_err(|_| CodegenError::new("failed to compare File.exists probe result"))?;
                self.builder
                    .build_store(alloca_exists_slot, is_regular)
                    .map_err(|_| CodegenError::new("failed to store File.exists result"))?;
                self.builder
                    .build_unconditional_branch(end_block)
                    .map_err(|_| CodegenError::new("failed to branch File.exists to end"))?;

                self.builder.position_at_end(end_block);
                let final_exists = self
                    .builder
                    .build_load(
                        self.context.bool_type(),
                        alloca_exists_slot,
                        "exists_final_value",
                    )
                    .map_err(|_| CodegenError::new("failed to load File.exists final result"))?;
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
                    .map_err(|_| CodegenError::new("failed to emit fopen for File.delete probe"))?;
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
                    .map_err(|_| CodegenError::new("failed to allocate File.delete result slot"))?;
                self.builder
                    .build_store(delete_result_slot, self.context.bool_type().const_zero())
                    .map_err(|_| {
                        CodegenError::new("failed to initialize File.delete result slot")
                    })?;

                let probe_is_null = self
                    .builder
                    .build_is_null(file_ptr, "delete_probe_is_null")
                    .map_err(|_| CodegenError::new("failed to test File.delete probe pointer"))?;
                self.builder
                    .build_conditional_branch(probe_is_null, delete_fail_bb, probe_open_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch on File.delete probe open result")
                    })?;

                self.builder.position_at_end(probe_open_bb);
                let buf_slot = self
                    .builder
                    .build_alloca(self.context.i8_type(), "delete_probe_buf")
                    .map_err(|_| {
                        CodegenError::new("failed to allocate File.delete probe buffer")
                    })?;
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
                    .map_err(|_| CodegenError::new("failed to emit fread for File.delete probe"))?;
                let err_call = self
                    .builder
                    .build_call(ferror, &[file_ptr.into()], "delete_probe_err")
                    .map_err(|_| {
                        CodegenError::new("failed to emit ferror for File.delete probe")
                    })?;
                let err_code = self.extract_call_value(err_call)?.into_int_value();
                self.builder
                    .build_call(fclose, &[file_ptr.into()], "delete_probe_close")
                    .map_err(|_| {
                        CodegenError::new("failed to emit fclose for File.delete probe")
                    })?;
                let probe_is_regular = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        err_code,
                        self.context.i32_type().const_zero(),
                        "delete_probe_is_regular",
                    )
                    .map_err(|_| CodegenError::new("failed to compare File.delete probe result"))?;
                self.builder
                    .build_conditional_branch(probe_is_regular, delete_remove_bb, delete_fail_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch on File.delete probe regularity")
                    })?;

                self.builder.position_at_end(delete_remove_bb);
                let res_call = self
                    .builder
                    .build_call(remove, &[path.into()], "res")
                    .map_err(|_| CodegenError::new("failed to emit remove for File.delete"))?;
                let res = self.extract_call_value(res_call)?.into_int_value();
                let zero = self.context.i32_type().const_int(0, false);
                let success = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, res, zero, "success")
                    .map_err(|_| CodegenError::new("failed to compare File.delete result"))?;
                self.builder
                    .build_store(delete_result_slot, success)
                    .map_err(|_| CodegenError::new("failed to store File.delete success flag"))?;
                self.builder
                    .build_unconditional_branch(delete_merge_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch File.delete success to merge")
                    })?;

                self.builder.position_at_end(delete_fail_bb);
                self.builder
                    .build_store(delete_result_slot, self.context.bool_type().const_zero())
                    .map_err(|_| CodegenError::new("failed to store File.delete failure flag"))?;
                self.builder
                    .build_unconditional_branch(delete_merge_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch File.delete failure to merge")
                    })?;

                self.builder.position_at_end(delete_merge_bb);
                let final_result = self
                    .builder
                    .build_load(
                        self.context.bool_type(),
                        delete_result_slot,
                        "file_delete_result",
                    )
                    .map_err(|_| CodegenError::new("failed to load File.delete final result"))?;
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

                // 1. Get current time
                let null = self.context.ptr_type(AddressSpace::default()).const_null();
                let t_val = self
                    .builder
                    .build_call(time_fn, &[null.into()], "t")
                    .map_err(|_| CodegenError::new("failed to emit time() for Time.now"))?;
                let t_raw = self.extract_call_value(t_val)?;
                let time_ty = self.libc_time_type();

                // 2. Alloca for time_t
                let t_ptr = self.builder.build_alloca(time_ty, "t_ptr").map_err(|_| {
                    CodegenError::new("failed to allocate time_t slot for Time.now")
                })?;
                self.builder
                    .build_store(t_ptr, t_raw)
                    .map_err(|_| CodegenError::new("failed to store time_t value for Time.now"))?;

                // 3. Get local time struct pointer
                let tm_ptr_val = self
                    .builder
                    .build_call(localtime_fn, &[t_ptr.into()], "tm")
                    .map_err(|_| CodegenError::new("failed to emit localtime() for Time.now"))?;
                let tm_ptr = self.extract_call_value(tm_ptr_val)?.into_pointer_value();

                // 4. Allocate a buffer sized from the format string instead of a fixed
                // 64-byte slab, which truncated longer formats and could leave invalid output.
                let strlen_fn = self.get_or_declare_strlen();
                let format_len_call = self
                    .builder
                    .build_call(strlen_fn, &[format.into()], "format_len")
                    .map_err(|_| CodegenError::new("failed to emit strlen for Time.now format"))?;
                let format_len = self.extract_call_value(format_len_call)?.into_int_value();
                let scaled_format_len = self
                    .builder
                    .build_int_mul(
                        format_len,
                        self.context.i64_type().const_int(8, false),
                        "scaled_format_len",
                    )
                    .map_err(|_| CodegenError::new("failed to scale Time.now format length"))?;
                let dynamic_buf_size = self
                    .builder
                    .build_int_add(
                        scaled_format_len,
                        self.context.i64_type().const_int(64, false),
                        "dynamic_time_buf_size",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compute Time.now dynamic buffer size")
                    })?;
                let min_buf_size = self.context.i64_type().const_int(64, false);
                let use_dynamic_buf = self
                    .builder
                    .build_int_compare(
                        IntPredicate::UGT,
                        dynamic_buf_size,
                        min_buf_size,
                        "use_dynamic_time_buf",
                    )
                    .map_err(|_| CodegenError::new("failed to compare Time.now buffer sizes"))?;
                let buf_size = self
                    .builder
                    .build_select(
                        use_dynamic_buf,
                        dynamic_buf_size,
                        min_buf_size,
                        "time_buf_size",
                    )
                    .map_err(|_| CodegenError::new("failed to select Time.now buffer size"))?
                    .into_int_value();
                let buf_ptr_val =
                    self.build_malloc_call(buf_size, "buf", "failed to allocate Time.now buffer")?;
                let buf_ptr = self.extract_call_value(buf_ptr_val)?.into_pointer_value();
                let last_byte_offset = self
                    .builder
                    .build_int_sub(
                        buf_size,
                        self.context.i64_type().const_int(1, false),
                        "time_last_byte_offset",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compute Time.now last byte offset")
                    })?;
                let last_byte_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(
                            self.context.i8_type(),
                            buf_ptr,
                            &[last_byte_offset],
                            "time_last_byte_ptr",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to access Time.now last byte pointer")
                        })?
                };
                self.builder
                    .build_store(buf_ptr, self.context.i8_type().const_zero())
                    .map_err(|_| CodegenError::new("failed to initialize Time.now buffer"))?;
                self.builder
                    .build_store(last_byte_ptr, self.context.i8_type().const_zero())
                    .map_err(|_| CodegenError::new("failed to initialize Time.now buffer tail"))?;

                // 5. If format is empty string, use default "%H:%M:%S"
                let is_empty = self
                    .builder
                    .build_call(strlen_fn, &[format.into()], "len")
                    .map_err(|_| {
                        CodegenError::new("failed to emit strlen for Time.now empty check")
                    })?;
                let is_empty_val = self.extract_call_value(is_empty)?.into_int_value();
                let is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        is_empty_val,
                        self.context.i64_type().const_int(0, false),
                        "is_zero",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compare Time.now format length against zero")
                    })?;

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
                    .map_err(|_| CodegenError::new("failed to select Time.now format string"))?;

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
                    .map_err(|_| CodegenError::new("failed to emit strftime for Time.now"))?;

                Ok(Some(buf_ptr.into()))
            }

            "Time__unix" => {
                let time_fn = self.get_or_declare_time();
                let null = self.context.ptr_type(AddressSpace::default()).const_null();
                let res = self
                    .builder
                    .build_call(time_fn, &[null.into()], "time")
                    .map_err(|_| CodegenError::new("failed to emit time() for Time.unix"))?;
                let unix_time_raw = self.extract_call_value(res)?.into_int_value();
                let unix_time_i64 = self
                    .builder
                    .build_int_cast(unix_time_raw, self.context.i64_type(), "time_unix_i64")
                    .map_err(|_| CodegenError::new("failed to cast time_t to Integer"))?;
                Ok(Some(unix_time_i64.into()))
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
                    .map_err(|_| {
                        CodegenError::new("failed to cast Time.sleep milliseconds to i64")
                    })?;
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
                    .map_err(|_| {
                        CodegenError::new("failed to compare Time.sleep milliseconds against zero")
                    })?;
                self.builder
                    .build_conditional_branch(sleep_negative, sleep_invalid_bb, sleep_valid_bb)
                    .map_err(|_| CodegenError::new("failed to branch for Time.sleep validation"))?;

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
                        .map_err(|_| {
                            CodegenError::new("failed to truncate Time.sleep milliseconds to i32")
                        })?;
                    self.builder
                        .build_call(sleep_fn, &[ms_i32.into()], "")
                        .map_err(|_| CodegenError::new("failed to emit Sleep for Time.sleep"))?;
                }
                #[cfg(not(windows))]
                {
                    let usleep_fn = self.get_or_declare_usleep();
                    let us = self
                        .builder
                        .build_int_mul(ms_i64, self.context.i64_type().const_int(1000, false), "us")
                        .map_err(|_| {
                            CodegenError::new(
                                "failed to convert Time.sleep milliseconds to microseconds",
                            )
                        })?;
                    let us_i32 = self
                        .builder
                        .build_int_truncate(us, self.context.i32_type(), "us32")
                        .map_err(|_| {
                            CodegenError::new("failed to truncate Time.sleep microseconds to i32")
                        })?;
                    self.builder
                        .build_call(usleep_fn, &[us_i32.into()], "")
                        .map_err(|_| CodegenError::new("failed to emit usleep for Time.sleep"))?;
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
                    .map_err(|_| CodegenError::new("failed to emit getenv for System.getenv"))?;
                let val = self.extract_call_value(res)?.into_pointer_value();

                // If NULL, return empty string
                let is_null = self.builder.build_is_null(val, "is_null").map_err(|_| {
                    CodegenError::new("failed to test System.getenv result for null")
                })?;
                let empty_str = self.get_or_create_empty_string();

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("System.getenv used outside function"))?;
                let success_bb = self.context.append_basic_block(current_fn, "env.ok");
                let fail_bb = self.context.append_basic_block(current_fn, "env.fail");
                let merge_bb = self.context.append_basic_block(current_fn, "env.merge");

                self.builder
                    .build_conditional_branch(is_null, fail_bb, success_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch on System.getenv null result")
                    })?;

                self.builder.position_at_end(fail_bb);
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch System.getenv failure to merge")
                    })?;

                self.builder.position_at_end(success_bb);
                self.compile_utf8_string_length_runtime(val)?;
                let success_merge_block = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::new("System.getenv merge predecessor missing"))?;
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch System.getenv success to merge")
                    })?;

                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(self.context.ptr_type(AddressSpace::default()), "res")
                    .map_err(|_| CodegenError::new("failed to build System.getenv result phi"))?;
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
                    .map_err(|_| CodegenError::new("failed to emit system() for System.shell"))?;
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
                        .map_err(|_| {
                            CodegenError::new(
                                "failed to compare System.shell raw status against failure",
                            )
                        })?;
                    self.builder
                        .build_conditional_branch(call_failed, decode_error_bb, signal_check_bb)
                        .map_err(|_| {
                            CodegenError::new("failed to branch on System.shell call failure")
                        })?;

                    self.builder.position_at_end(decode_error_bb);
                    self.builder
                        .build_unconditional_branch(merge_bb)
                        .map_err(|_| {
                            CodegenError::new("failed to branch System.shell decode error to merge")
                        })?;

                    self.builder.position_at_end(signal_check_bb);
                    let signal_bits = self
                        .builder
                        .build_and(
                            code,
                            i32_type.const_int(0x7f, false),
                            "system_shell_signal_bits",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to mask System.shell signal bits")
                        })?;
                    let has_signal = self
                        .builder
                        .build_int_compare(
                            IntPredicate::NE,
                            signal_bits,
                            i32_type.const_zero(),
                            "system_shell_has_signal",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to compare System.shell signal bits")
                        })?;
                    self.builder
                        .build_conditional_branch(has_signal, signaled_bb, exited_bb)
                        .map_err(|_| {
                            CodegenError::new("failed to branch on System.shell signal state")
                        })?;

                    self.builder.position_at_end(signaled_bb);
                    let signaled_code = self
                        .builder
                        .build_int_add(
                            signal_bits,
                            i32_type.const_int(128, false),
                            "system_shell_signaled_code",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to compute System.shell signal exit code")
                        })?;
                    self.builder
                        .build_unconditional_branch(merge_bb)
                        .map_err(|_| {
                            CodegenError::new(
                                "failed to branch System.shell signaled path to merge",
                            )
                        })?;

                    self.builder.position_at_end(exited_bb);
                    let shifted_code = self
                        .builder
                        .build_right_shift(
                            code,
                            i32_type.const_int(8, false),
                            false,
                            "system_shell_shifted_code",
                        )
                        .map_err(|_| {
                            CodegenError::new("failed to shift System.shell exit status")
                        })?;
                    let exit_code = self
                        .builder
                        .build_and(
                            shifted_code,
                            i32_type.const_int(0xff, false),
                            "system_shell_exit_code",
                        )
                        .map_err(|_| CodegenError::new("failed to mask System.shell exit code"))?;
                    self.builder
                        .build_unconditional_branch(merge_bb)
                        .map_err(|_| {
                            CodegenError::new("failed to branch System.shell exited path to merge")
                        })?;

                    self.builder.position_at_end(merge_bb);
                    let decoded_phi = self
                        .builder
                        .build_phi(i32_type, "system_shell_decoded")
                        .map_err(|_| {
                            CodegenError::new("failed to build System.shell decoded status phi")
                        })?;
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
                    .map_err(|_| {
                        CodegenError::new("failed to extend System.shell exit code to i64")
                    })?;
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
                    .map_err(|_| CodegenError::new("failed to emit popen for System.exec"))?;
                let pipe_ptr = self.extract_call_value(pipe_val)?.into_pointer_value();

                let is_null = self
                    .builder
                    .build_is_null(pipe_ptr, "is_null")
                    .map_err(|_| CodegenError::new("failed to test System.exec pipe pointer"))?;

                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("System.exec used outside function"))?;
                let success_bb = self.context.append_basic_block(current_fn, "exec.ok");
                let fail_bb = self.context.append_basic_block(current_fn, "exec.fail");
                let merge_bb = self.context.append_basic_block(current_fn, "exec.merge");

                self.builder
                    .build_conditional_branch(is_null, fail_bb, success_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch on System.exec pipe result")
                    })?;

                // Fail - return empty string
                self.builder.position_at_end(fail_bb);
                let empty_str = self.get_or_create_empty_string();
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch System.exec failure to merge")
                    })?;

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
                    .map_err(|_| CodegenError::new("failed to allocate System.exec buffer slot"))?;
                let capacity_slot = self
                    .builder
                    .build_alloca(i64_type, "exec_capacity_slot")
                    .map_err(|_| {
                        CodegenError::new("failed to allocate System.exec capacity slot")
                    })?;
                let total_read_slot = self
                    .builder
                    .build_alloca(i64_type, "exec_total_read_slot")
                    .map_err(|_| {
                        CodegenError::new("failed to allocate System.exec total read slot")
                    })?;

                let buf_call = self.build_malloc_call(
                    initial_capacity,
                    "buf",
                    "failed to allocate System.exec buffer",
                )?;
                let buf = self.extract_call_value(buf_call)?.into_pointer_value();
                self.builder
                    .build_store(buf_slot, buf)
                    .map_err(|_| CodegenError::new("failed to store System.exec buffer pointer"))?;
                self.builder
                    .build_store(capacity_slot, initial_capacity)
                    .map_err(|_| CodegenError::new("failed to store System.exec capacity"))?;
                self.builder
                    .build_store(total_read_slot, i64_type.const_zero())
                    .map_err(|_| {
                        CodegenError::new("failed to initialize System.exec total read")
                    })?;
                self.builder
                    .build_unconditional_branch(read_cond_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch into System.exec read loop")
                    })?;

                self.builder.position_at_end(read_cond_bb);
                let current_capacity = self
                    .builder
                    .build_load(i64_type, capacity_slot, "exec_capacity")
                    .map_err(|_| CodegenError::new("failed to load System.exec capacity"))?
                    .into_int_value();
                let current_total = self
                    .builder
                    .build_load(i64_type, total_read_slot, "exec_total_read")
                    .map_err(|_| CodegenError::new("failed to load System.exec total read"))?
                    .into_int_value();
                let remaining_capacity = self
                    .builder
                    .build_int_sub(
                        current_capacity,
                        self.builder
                            .build_int_add(current_total, one, "exec_total_plus_term")
                            .map_err(|_| {
                                CodegenError::new(
                                    "failed to compute System.exec total plus terminator",
                                )
                            })?,
                        "exec_remaining_capacity",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compute System.exec remaining capacity")
                    })?;
                let needs_grow = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        remaining_capacity,
                        i64_type.const_zero(),
                        "exec_needs_grow",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compare System.exec remaining capacity")
                    })?;
                self.builder
                    .build_conditional_branch(needs_grow, grow_bb, read_body_bb)
                    .map_err(|_| CodegenError::new("failed to branch on System.exec growth"))?;

                self.builder.position_at_end(read_body_bb);
                let current_buf = self
                    .builder
                    .build_load(ptr_type, buf_slot, "exec_buf")
                    .map_err(|_| CodegenError::new("failed to load System.exec buffer"))?
                    .into_pointer_value();
                let write_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(i8_type, current_buf, &[current_total], "exec_write_ptr")
                        .map_err(|_| {
                            CodegenError::new("failed to access System.exec write pointer")
                        })?
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
                    .map_err(|_| CodegenError::new("failed to emit fread for System.exec"))?;
                let read_len = self.extract_call_value(read_len_call)?.into_int_value();
                let reached_eof = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        read_len,
                        i64_type.const_zero(),
                        "exec_reached_eof",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compare System.exec read length against zero")
                    })?;
                self.builder
                    .build_conditional_branch(reached_eof, done_bb, read_after_bb)
                    .map_err(|_| CodegenError::new("failed to branch on System.exec EOF"))?;

                self.builder.position_at_end(read_after_bb);
                let next_total = self
                    .builder
                    .build_int_add(current_total, read_len, "exec_next_total")
                    .map_err(|_| {
                        CodegenError::new("failed to compute System.exec next total read")
                    })?;
                self.builder
                    .build_store(total_read_slot, next_total)
                    .map_err(|_| {
                        CodegenError::new("failed to store System.exec next total read")
                    })?;
                let filled_chunk = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        read_len,
                        remaining_capacity,
                        "exec_filled_chunk",
                    )
                    .map_err(|_| CodegenError::new("failed to compare System.exec filled chunk"))?;
                self.builder
                    .build_conditional_branch(filled_chunk, grow_bb, read_cond_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch after System.exec read chunk")
                    })?;

                self.builder.position_at_end(grow_bb);
                let grow_capacity = self
                    .builder
                    .build_load(i64_type, capacity_slot, "exec_grow_capacity")
                    .map_err(|_| CodegenError::new("failed to load System.exec grow capacity"))?
                    .into_int_value();
                let new_capacity = self
                    .builder
                    .build_int_add(grow_capacity, chunk_size, "exec_new_capacity")
                    .map_err(|_| CodegenError::new("failed to compute System.exec new capacity"))?;
                let grow_buf = self
                    .builder
                    .build_load(ptr_type, buf_slot, "exec_grow_buf")
                    .map_err(|_| CodegenError::new("failed to load System.exec grow buffer"))?
                    .into_pointer_value();
                let realloc_call = self.build_realloc_call(
                    grow_buf,
                    new_capacity,
                    "exec_realloc",
                    "failed to emit realloc for System.exec",
                )?;
                let realloc_buf = self.extract_call_value(realloc_call)?.into_pointer_value();
                let realloc_failed = self
                    .builder
                    .build_is_null(realloc_buf, "exec_realloc_failed")
                    .map_err(|_| CodegenError::new("failed to test System.exec realloc result"))?;
                self.builder
                    .build_conditional_branch(realloc_failed, oom_bb, grow_ok_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch on System.exec realloc result")
                    })?;

                self.builder.position_at_end(oom_bb);
                self.emit_runtime_error("System.exec() out of memory", "exec_out_of_memory")?;

                self.builder.position_at_end(grow_ok_bb);
                self.builder
                    .build_store(buf_slot, realloc_buf)
                    .map_err(|_| CodegenError::new("failed to store grown System.exec buffer"))?;
                self.builder
                    .build_store(capacity_slot, new_capacity)
                    .map_err(|_| CodegenError::new("failed to store grown System.exec capacity"))?;
                self.builder
                    .build_unconditional_branch(read_cond_bb)
                    .map_err(|_| CodegenError::new("failed to loop System.exec after growth"))?;

                self.builder.position_at_end(done_bb);
                let final_total = self
                    .builder
                    .build_load(i64_type, total_read_slot, "exec_final_total")
                    .map_err(|_| CodegenError::new("failed to load System.exec final total"))?
                    .into_int_value();
                let final_buf = self
                    .builder
                    .build_load(ptr_type, buf_slot, "exec_final_buf")
                    .map_err(|_| CodegenError::new("failed to load System.exec final buffer"))?
                    .into_pointer_value();
                let scan_index_slot = self
                    .builder
                    .build_alloca(i64_type, "exec_scan_index_slot")
                    .map_err(|_| {
                        CodegenError::new("failed to allocate System.exec scan index slot")
                    })?;
                self.builder
                    .build_store(scan_index_slot, i64_type.const_zero())
                    .map_err(|_| {
                        CodegenError::new("failed to initialize System.exec scan index")
                    })?;
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
                    .map_err(|_| CodegenError::new("failed to branch into System.exec NUL scan"))?;

                self.builder.position_at_end(scan_cond_bb);
                let scan_index = self
                    .builder
                    .build_load(i64_type, scan_index_slot, "exec_scan_index")
                    .map_err(|_| CodegenError::new("failed to load System.exec scan index"))?
                    .into_int_value();
                let scan_has_more = self
                    .builder
                    .build_int_compare(
                        IntPredicate::ULT,
                        scan_index,
                        final_total,
                        "exec_scan_has_more",
                    )
                    .map_err(|_| CodegenError::new("failed to compare System.exec scan bounds"))?;
                self.builder
                    .build_conditional_branch(scan_has_more, scan_body_bb, validate_utf8_bb)
                    .map_err(|_| CodegenError::new("failed to branch in System.exec NUL scan"))?;

                self.builder.position_at_end(scan_body_bb);
                let scan_byte_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(i8_type, final_buf, &[scan_index], "exec_scan_byte_ptr")
                        .map_err(|_| {
                            CodegenError::new("failed to access System.exec scan byte pointer")
                        })?
                };
                let scan_byte = self
                    .builder
                    .build_load(i8_type, scan_byte_ptr, "exec_scan_byte")
                    .map_err(|_| CodegenError::new("failed to load System.exec scan byte"))?
                    .into_int_value();
                let scan_is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        scan_byte,
                        i8_type.const_zero(),
                        "exec_scan_is_zero",
                    )
                    .map_err(|_| {
                        CodegenError::new("failed to compare System.exec scan byte against zero")
                    })?;
                self.builder
                    .build_conditional_branch(scan_is_zero, scan_fail_bb, scan_next_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch on System.exec NUL byte detection")
                    })?;

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
                    .map_err(|_| CodegenError::new("failed to increment System.exec scan index"))?;
                self.builder
                    .build_store(scan_index_slot, next_scan_index)
                    .map_err(|_| CodegenError::new("failed to store System.exec scan index"))?;
                self.builder
                    .build_unconditional_branch(scan_cond_bb)
                    .map_err(|_| CodegenError::new("failed to loop System.exec NUL scan"))?;

                self.builder.position_at_end(validate_utf8_bb);
                let term_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(i8_type, final_buf, &[final_total], "term_ptr")
                        .map_err(|_| {
                            CodegenError::new("failed to access System.exec terminator slot")
                        })?
                };
                self.builder
                    .build_store(term_ptr, i8_type.const_zero())
                    .map_err(|_| {
                        CodegenError::new("failed to null-terminate System.exec buffer")
                    })?;
                self.compile_utf8_string_length_runtime(final_buf)?;
                self.builder
                    .build_call(pclose_fn, &[pipe_ptr.into()], "")
                    .map_err(|_| CodegenError::new("failed to emit pclose for System.exec"))?;
                let success_merge_block = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::new("System.exec merge predecessor missing"))?;
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch System.exec success to merge")
                    })?;

                // Merge
                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(self.context.ptr_type(AddressSpace::default()), "res")
                    .map_err(|_| CodegenError::new("failed to build System.exec result phi"))?;
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
                    .map_err(|_| CodegenError::new("failed to emit getcwd for System.cwd"))?;
                let cwd_ptr = self.extract_call_value(cwd_call)?.into_pointer_value();
                let cwd_failed = self
                    .builder
                    .build_is_null(cwd_ptr, "cwd_failed")
                    .map_err(|_| CodegenError::new("failed to test System.cwd result for null"))?;
                let current_fn = self
                    .current_function
                    .ok_or_else(|| CodegenError::new("System.cwd used outside function"))?;
                let cwd_ok_bb = self.context.append_basic_block(current_fn, "system_cwd_ok");
                let cwd_fail_bb = self
                    .context
                    .append_basic_block(current_fn, "system_cwd_fail");
                self.builder
                    .build_conditional_branch(cwd_failed, cwd_fail_bb, cwd_ok_bb)
                    .map_err(|_| CodegenError::new("failed to branch on System.cwd result"))?;

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
                    .map_err(|_| CodegenError::new("failed to load argc global"))?
                    .into_int_value();
                let argc64 = self
                    .builder
                    .build_int_s_extend(argc, self.context.i64_type(), "argc64")
                    .map_err(|_| CodegenError::new("failed to extend argc to i64"))?;
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
                    .map_err(|_| CodegenError::new("failed to load argc global"))?
                    .into_int_value();
                let argc64 = self
                    .builder
                    .build_int_s_extend(argc, self.context.i64_type(), "argc64")
                    .map_err(|_| CodegenError::new("failed to extend argc to i64"))?;
                let argv = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        argv_global.as_pointer_value(),
                        "argv",
                    )
                    .map_err(|_| CodegenError::new("failed to load argv global"))?
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
                    .map_err(|_| {
                        CodegenError::new("failed to compare Args.get index against zero")
                    })?;
                self.builder
                    .build_conditional_branch(non_negative, bounds_check_bb, negative_bb)
                    .map_err(|_| {
                        CodegenError::new("failed to branch on Args.get negative check")
                    })?;

                self.builder.position_at_end(negative_bb);
                self.emit_runtime_error(
                    "Args.get() index cannot be negative",
                    "args_get_negative_runtime_error",
                )?;

                self.builder.position_at_end(bounds_check_bb);
                let in_bounds = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, index, argc64, "args_get_in_bounds")
                    .map_err(|_| {
                        CodegenError::new("failed to compare Args.get index against argc")
                    })?;
                self.builder
                    .build_conditional_branch(in_bounds, ok_bb, oob_bb)
                    .map_err(|_| CodegenError::new("failed to branch on Args.get bounds check"))?;

                self.builder.position_at_end(oob_bb);
                self.emit_runtime_error(
                    "Args.get() index out of bounds",
                    "args_get_oob_runtime_error",
                )?;

                self.builder.position_at_end(ok_bb);
                // index is i64, need to truncate to i32 for GEP if needed, but ptr is 64bit
                let elem_ptr = // SAFETY: This block performs low-level pointer/layout operations in codegen; pointer provenance,
// alignment, and bounds are validated by the surrounding control flow and runtime layout invariants.
unsafe {
                    self.builder
                        .build_gep(
                            self.context.ptr_type(AddressSpace::default()),
                            argv,
                            &[index],
                            "arg_ptr",
                        )
                        .map_err(|_| CodegenError::new("failed to access argv element pointer"))?
                };
                let arg_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        elem_ptr,
                        "arg",
                    )
                    .map_err(|_| CodegenError::new("failed to load argv element"))?;
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
                    .map_err(|_| CodegenError::new("failed to branch for assert()"))?;

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr("Assertion failed!\\n", "assert_fail")
                    .map_err(|_| CodegenError::new("failed to build assert panic message"))?;
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .map_err(|_| CodegenError::new("failed to emit printf for assert()"))?;
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .map_err(|_| CodegenError::new("failed to emit exit for assert()"))?;
                self.builder
                    .build_unreachable()
                    .map_err(|_| CodegenError::new("failed to emit unreachable for assert()"))?;

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
                    .map_err(|_| CodegenError::new("failed to branch for assert_eq()"))?;

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Assertion failed: values are not equal!\\n",
                        "assert_eq_fail",
                    )
                    .map_err(|_| CodegenError::new("failed to build assert_eq panic message"))?;
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .map_err(|_| CodegenError::new("failed to emit printf for assert_eq()"))?;
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .map_err(|_| CodegenError::new("failed to emit exit for assert_eq()"))?;
                self.builder
                    .build_unreachable()
                    .map_err(|_| CodegenError::new("failed to emit unreachable for assert_eq()"))?;

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
                    .map_err(|_| CodegenError::new("failed to branch for assert_ne()"))?;

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Assertion failed: values should not be equal!\\n",
                        "assert_ne_fail",
                    )
                    .map_err(|_| CodegenError::new("failed to build assert_ne panic message"))?;
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .map_err(|_| CodegenError::new("failed to emit printf for assert_ne()"))?;
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .map_err(|_| CodegenError::new("failed to emit exit for assert_ne()"))?;
                self.builder
                    .build_unreachable()
                    .map_err(|_| CodegenError::new("failed to emit unreachable for assert_ne()"))?;

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
                    .map_err(|_| CodegenError::new("failed to branch for assert_true()"))?;

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Assertion failed: expected true!\\n",
                        "assert_true_fail",
                    )
                    .map_err(|_| CodegenError::new("failed to build assert_true panic message"))?;
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .map_err(|_| CodegenError::new("failed to emit printf for assert_true()"))?;
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .map_err(|_| CodegenError::new("failed to emit exit for assert_true()"))?;
                self.builder.build_unreachable().map_err(|_| {
                    CodegenError::new("failed to emit unreachable for assert_true()")
                })?;

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
                    .map_err(|_| CodegenError::new("failed to branch for assert_false()"))?;

                // Panic block
                self.builder.position_at_end(panic_bb);
                let printf = self.get_or_declare_printf();
                let panic_msg = self
                    .builder
                    .build_global_string_ptr(
                        "Assertion failed: expected false!\\n",
                        "assert_false_fail",
                    )
                    .map_err(|_| CodegenError::new("failed to build assert_false panic message"))?;
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .map_err(|_| CodegenError::new("failed to emit printf for assert_false()"))?;
                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .map_err(|_| CodegenError::new("failed to emit exit for assert_false()"))?;
                self.builder.build_unreachable().map_err(|_| {
                    CodegenError::new("failed to emit unreachable for assert_false()")
                })?;

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
                    .map_err(|_| CodegenError::new("failed to build fail() prefix string"))?;
                self.builder
                    .build_call(printf, &[panic_msg.as_pointer_value().into()], "")
                    .map_err(|_| CodegenError::new("failed to emit printf for fail() prefix"))?;

                if !args.is_empty() {
                    let msg = self.compile_string_argument_expr(
                        &args[0].node,
                        "fail() requires String message",
                    )?;
                    self.builder
                        .build_call(printf, &[msg.into()], "")
                        .map_err(|_| {
                            CodegenError::new("failed to emit printf for fail() message")
                        })?;
                }

                let newline = self
                    .builder
                    .build_global_string_ptr("\\n", "nl")
                    .map_err(|_| CodegenError::new("failed to build fail() newline string"))?;
                self.builder
                    .build_call(printf, &[newline.as_pointer_value().into()], "")
                    .map_err(|_| CodegenError::new("failed to emit printf for fail() newline"))?;

                let exit_fn = self.get_or_declare_exit();
                self.builder
                    .build_call(
                        exit_fn,
                        &[self.context.i32_type().const_int(1, false).into()],
                        "",
                    )
                    .map_err(|_| CodegenError::new("failed to emit exit for fail()"))?;
                self.builder
                    .build_unreachable()
                    .map_err(|_| CodegenError::new("failed to emit unreachable for fail()"))?;

                Ok(Some(self.context.i64_type().const_int(0, false).into()))
            }

            // Not a stdlib function
            _ => Ok(None),
        }
    }
}
