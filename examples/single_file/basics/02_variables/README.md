# 02_variables

Focused example: **Variables and Mutability**.

What this demonstrates:
- typed variable declarations
- `mut` vs default immutable bindings
- basic reassignment flow

Run:

```bash
arden run examples/single_file/basics/02_variables/02_variables.arden
```

Useful command variants:

```bash
arden check examples/single_file/basics/02_variables/02_variables.arden
arden compile examples/single_file/basics/02_variables/02_variables.arden --emit-llvm
arden run examples/single_file/basics/02_variables/02_variables.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
