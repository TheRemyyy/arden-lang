# 30_extern_variadic_printf

Focused example: **Extern Variadic**.

What this demonstrates:
- variadic extern calls
- printf-style C interop
- FFI argument boundary behavior

Run:

```bash
arden run examples/single_file/tooling_and_ffi/30_extern_variadic_printf/30_extern_variadic_printf.arden
```

Useful command variants:

```bash
arden check examples/single_file/tooling_and_ffi/30_extern_variadic_printf/30_extern_variadic_printf.arden
arden compile examples/single_file/tooling_and_ffi/30_extern_variadic_printf/30_extern_variadic_printf.arden --emit-llvm
arden run examples/single_file/tooling_and_ffi/30_extern_variadic_printf/30_extern_variadic_printf.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
