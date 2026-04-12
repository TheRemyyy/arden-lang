# 26_effect_system

Focused example: **Effect System**.

What this demonstrates:
- `@Pure/@Io/@Thread` contracts
- effect propagation across calls
- compile-time effect enforcement

Run:

```bash
arden run examples/single_file/tooling_and_ffi/26_effect_system/26_effect_system.arden
```

Useful command variants:

```bash
arden check examples/single_file/tooling_and_ffi/26_effect_system/26_effect_system.arden
arden compile examples/single_file/tooling_and_ffi/26_effect_system/26_effect_system.arden --emit-llvm
arden run examples/single_file/tooling_and_ffi/26_effect_system/26_effect_system.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
