# 28_async_runtime_control

Focused example: **Async Runtime Control**.

What this demonstrates:
- `Task.is_done()` polling
- `Task.await_timeout(ms)` timeout flow
- `Task.cancel()` behavior

Run:

```bash
arden run examples/single_file/tooling_and_ffi/28_async_runtime_control/28_async_runtime_control.arden
```

Useful command variants:

```bash
arden check examples/single_file/tooling_and_ffi/28_async_runtime_control/28_async_runtime_control.arden
arden compile examples/single_file/tooling_and_ffi/28_async_runtime_control/28_async_runtime_control.arden --emit-llvm
arden run examples/single_file/tooling_and_ffi/28_async_runtime_control/28_async_runtime_control.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
