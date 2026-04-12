# 20_system

Focused example: **System Module**.

What this demonstrates:
- `System.os`, `System.cwd`, `System.getenv`
- OS boundary usage
- basic environment/system queries

Run:

```bash
arden run examples/single_file/stdlib_and_system/20_system/20_system.arden
```

Useful command variants:

```bash
arden check examples/single_file/stdlib_and_system/20_system/20_system.arden
arden compile examples/single_file/stdlib_and_system/20_system/20_system.arden --emit-llvm
arden run examples/single_file/stdlib_and_system/20_system/20_system.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
