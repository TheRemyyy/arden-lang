# 40_borrow_scope_recovery

Focused example: **Borrow Scope Recovery**.

What this demonstrates:
- borrow release at scope end
- move-after-borrow patterns
- practical borrow-check conflict resolution

Run:

```bash
arden run examples/single_file/safety_and_async/40_borrow_scope_recovery/40_borrow_scope_recovery.arden
```

Useful command variants:

```bash
arden check examples/single_file/safety_and_async/40_borrow_scope_recovery/40_borrow_scope_recovery.arden
arden compile examples/single_file/safety_and_async/40_borrow_scope_recovery/40_borrow_scope_recovery.arden --emit-llvm
arden run examples/single_file/safety_and_async/40_borrow_scope_recovery/40_borrow_scope_recovery.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
