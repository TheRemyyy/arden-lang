# 32_extern_safe_wrapper

Focused example: **Extern Safe Wrapper**.

What this demonstrates:
- wrapping extern calls with safe APIs
- isolating low-level boundaries
- cleaner FFI-facing design

Run:

```bash
arden run examples/single_file/tooling_and_ffi/32_extern_safe_wrapper/32_extern_safe_wrapper.arden
```

Useful command variants:

```bash
arden check examples/single_file/tooling_and_ffi/32_extern_safe_wrapper/32_extern_safe_wrapper.arden
arden compile examples/single_file/tooling_and_ffi/32_extern_safe_wrapper/32_extern_safe_wrapper.arden --emit-llvm
arden run examples/single_file/tooling_and_ffi/32_extern_safe_wrapper/32_extern_safe_wrapper.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
