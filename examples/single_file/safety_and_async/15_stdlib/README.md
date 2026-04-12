# 15_stdlib

Focused example: **Stdlib Tour**.

What this demonstrates:
- quick overview of key stdlib APIs
- I/O, math, string, and system usage
- combining modules in one file

Run:

```bash
arden run examples/single_file/safety_and_async/15_stdlib/15_stdlib.arden
```

Useful command variants:

```bash
arden check examples/single_file/safety_and_async/15_stdlib/15_stdlib.arden
arden compile examples/single_file/safety_and_async/15_stdlib/15_stdlib.arden --emit-llvm
arden run examples/single_file/safety_and_async/15_stdlib/15_stdlib.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
