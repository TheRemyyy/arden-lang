# 04_control_flow

Focused example: **Control Flow**.

What this demonstrates:
- `if/else` branching
- `while` and `for` loops
- `break` and `continue` usage

Run:

```bash
arden run examples/single_file/basics/04_control_flow/04_control_flow.arden
```

Useful command variants:

```bash
arden check examples/single_file/basics/04_control_flow/04_control_flow.arden
arden compile examples/single_file/basics/04_control_flow/04_control_flow.arden --emit-llvm
arden run examples/single_file/basics/04_control_flow/04_control_flow.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
