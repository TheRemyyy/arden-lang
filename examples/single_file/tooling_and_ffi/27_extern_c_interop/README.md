# 27_extern_c_interop

Focused example: **Extern C Interop**.

What this demonstrates:
- basic `extern function` declarations
- calling C ABI symbols
- first practical FFI boundary

Run:

```bash
arden run examples/single_file/tooling_and_ffi/27_extern_c_interop/27_extern_c_interop.arden
```

Useful command variants:

```bash
arden check examples/single_file/tooling_and_ffi/27_extern_c_interop/27_extern_c_interop.arden
arden compile examples/single_file/tooling_and_ffi/27_extern_c_interop/27_extern_c_interop.arden --emit-llvm
arden run examples/single_file/tooling_and_ffi/27_extern_c_interop/27_extern_c_interop.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
