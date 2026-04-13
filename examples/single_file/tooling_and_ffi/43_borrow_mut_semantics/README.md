# 43_borrow_mut_semantics

Focused example: **`borrow mut` behavior vs explicit `&mut`**.

What this demonstrates:
- `borrow mut` call-site requires mutable binding
- in-callee reads/writes are allowed for `borrow mut` params
- caller-visible mutation propagation can differ by value category
- `&mut` provides explicit/predictable in-place mutation semantics

Run:

```bash
arden run examples/single_file/tooling_and_ffi/43_borrow_mut_semantics/43_borrow_mut_semantics.arden
```

Useful command variants:

```bash
arden check examples/single_file/tooling_and_ffi/43_borrow_mut_semantics/43_borrow_mut_semantics.arden
arden compile examples/single_file/tooling_and_ffi/43_borrow_mut_semantics/43_borrow_mut_semantics.arden --emit-llvm
arden run examples/single_file/tooling_and_ffi/43_borrow_mut_semantics/43_borrow_mut_semantics.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
