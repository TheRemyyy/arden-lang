# 14_async

Focused example: **Async and Await**.

What this demonstrates:
- `async function` and `Task<T>` model
- `await` in normal control flow
- explicitly typed async boundaries

Run:

```bash
arden run examples/single_file/safety_and_async/14_async/14_async.arden
```

Useful command variants:

```bash
arden check examples/single_file/safety_and_async/14_async/14_async.arden
arden compile examples/single_file/safety_and_async/14_async/14_async.arden --emit-llvm
arden run examples/single_file/safety_and_async/14_async/14_async.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
