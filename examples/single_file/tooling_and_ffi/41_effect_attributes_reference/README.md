# 41_effect_attributes_reference

Focused example: **Effect Attributes Reference**.

What this demonstrates:
- full effect attribute set in one sample
- `@Any` orchestration across effect categories
- quick reference for effect docs and learning

Run:

```bash
arden run examples/single_file/tooling_and_ffi/41_effect_attributes_reference/41_effect_attributes_reference.arden
```

Useful command variants:

```bash
arden check examples/single_file/tooling_and_ffi/41_effect_attributes_reference/41_effect_attributes_reference.arden
arden compile examples/single_file/tooling_and_ffi/41_effect_attributes_reference/41_effect_attributes_reference.arden --emit-llvm
arden run examples/single_file/tooling_and_ffi/41_effect_attributes_reference/41_effect_attributes_reference.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
