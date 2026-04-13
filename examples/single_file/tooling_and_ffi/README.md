# Tooling and FFI Examples

Compiler tooling surface and C interop workflows.

- `24_test_attributes/24_test_attributes.arden`
- `26_effect_system/26_effect_system.arden`
- `27_extern_c_interop/27_extern_c_interop.arden`
- `28_async_runtime_control/28_async_runtime_control.arden`
- `29_effect_inference_and_any/29_effect_inference_and_any.arden`
- `30_extern_variadic_printf/30_extern_variadic_printf.arden`
- `31_extern_abi_link_name/31_extern_abi_link_name.arden`
- `32_extern_safe_wrapper/32_extern_safe_wrapper.arden`
- `33_extern_ptr_types/33_extern_ptr_types.arden`
- `34_bindgen_workflow/34_bindgen_workflow.arden`
- `41_effect_attributes_reference/41_effect_attributes_reference.arden`
- `42_build_timings_and_shards/42_build_timings_and_shards.arden`
- `43_borrow_mut_semantics/43_borrow_mut_semantics.arden`

Recommended order for effects/FFI learning:

1. `26_effect_system`
2. `29_effect_inference_and_any`
3. `41_effect_attributes_reference`
4. `42_build_timings_and_shards` for CLI/env build diagnostics
5. `43_borrow_mut_semantics` for current borrow-mut parameter behavior
6. `27` -> `30` -> `31` -> `32` -> `33` -> `34` for extern/FFI progression
