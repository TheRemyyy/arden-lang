# 29_effect_inference_and_any

Focused example: **Effect Inference and @Any**.

What this demonstrates:
- automatic effect inference
- `@Any` escape hatch behavior
- mixed-effect orchestration patterns

Run:

```bash
arden run examples/single_file/tooling_and_ffi/29_effect_inference_and_any/29_effect_inference_and_any.arden
```

Useful command variants:

```bash
arden check examples/single_file/tooling_and_ffi/29_effect_inference_and_any/29_effect_inference_and_any.arden
arden compile examples/single_file/tooling_and_ffi/29_effect_inference_and_any/29_effect_inference_and_any.arden --emit-llvm
arden run examples/single_file/tooling_and_ffi/29_effect_inference_and_any/29_effect_inference_and_any.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
