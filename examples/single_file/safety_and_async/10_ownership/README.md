# 10_ownership

Focused example: **Ownership and Borrowing**.

What this demonstrates:
- `owned` move semantics
- `borrow` and `borrow mut` parameter modes
- `&`/`&mut` references and mutability forwarding

Practical note:
- for in-place mutation APIs, `&mut T` parameters are the reliable pattern

Run:

```bash
arden run examples/single_file/safety_and_async/10_ownership/10_ownership.arden
```

Useful command variants:

```bash
arden check examples/single_file/safety_and_async/10_ownership/10_ownership.arden
arden compile examples/single_file/safety_and_async/10_ownership/10_ownership.arden --emit-llvm
arden run examples/single_file/safety_and_async/10_ownership/10_ownership.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
