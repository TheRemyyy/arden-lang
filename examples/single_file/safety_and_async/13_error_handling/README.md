# 13_error_handling

Focused example: **Error Handling**.

What this demonstrates:
- `Option` and `Result` patterns
- `?` error propagation
- assert/require validation style

Run:

```bash
arden run examples/single_file/safety_and_async/13_error_handling/13_error_handling.arden
```

Useful command variants:

```bash
arden check examples/single_file/safety_and_async/13_error_handling/13_error_handling.arden
arden compile examples/single_file/safety_and_async/13_error_handling/13_error_handling.arden --emit-llvm
arden run examples/single_file/safety_and_async/13_error_handling/13_error_handling.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
