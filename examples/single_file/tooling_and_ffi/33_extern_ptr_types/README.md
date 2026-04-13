# 33_extern_ptr_types

Focused example: **Extern Ptr Types**.

What this demonstrates:
- `Ptr<T>` on FFI boundaries
- malloc/free pointer ownership flow
- `memset` rebind pattern after pointer-consuming extern call
- explicit low-level type handling

Run:

```bash
arden run examples/single_file/tooling_and_ffi/33_extern_ptr_types/33_extern_ptr_types.arden
```

Useful command variants:

```bash
arden check examples/single_file/tooling_and_ffi/33_extern_ptr_types/33_extern_ptr_types.arden
arden compile examples/single_file/tooling_and_ffi/33_extern_ptr_types/33_extern_ptr_types.arden --emit-llvm
arden run examples/single_file/tooling_and_ffi/33_extern_ptr_types/33_extern_ptr_types.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
