# 31_extern_abi_link_name

Focused example: **Extern ABI and Link Name**.

What this demonstrates:
- `extern(c|system, "name")` forms
- binding to exact native symbols
- ABI-specific interop patterns

Run:

```bash
arden run examples/single_file/tooling_and_ffi/31_extern_abi_link_name/31_extern_abi_link_name.arden
```

Useful command variants:

```bash
arden check examples/single_file/tooling_and_ffi/31_extern_abi_link_name/31_extern_abi_link_name.arden
arden compile examples/single_file/tooling_and_ffi/31_extern_abi_link_name/31_extern_abi_link_name.arden --emit-llvm
arden run examples/single_file/tooling_and_ffi/31_extern_abi_link_name/31_extern_abi_link_name.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
