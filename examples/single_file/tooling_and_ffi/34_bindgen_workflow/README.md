# 34_bindgen_workflow

Focused example: **Bindgen Workflow**.

What this demonstrates:
- `arden bindgen` from C headers
- generated extern surface
- typical FFI generation workflow

Run:

```bash
arden run examples/single_file/tooling_and_ffi/34_bindgen_workflow/34_bindgen_workflow.arden
```

Useful command variants:

```bash
arden check examples/single_file/tooling_and_ffi/34_bindgen_workflow/34_bindgen_workflow.arden
arden compile examples/single_file/tooling_and_ffi/34_bindgen_workflow/34_bindgen_workflow.arden --emit-llvm
arden run examples/single_file/tooling_and_ffi/34_bindgen_workflow/34_bindgen_workflow.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
