# 22_args

Focused example: **Command-Line Args**.

What this demonstrates:
- `Args.count` and `Args.get`
- argv index model (`0` is executable path)
- basic argument parsing

Run:

```bash
arden run examples/single_file/stdlib_and_system/22_args/22_args.arden
```

Useful command variants:

```bash
arden check examples/single_file/stdlib_and_system/22_args/22_args.arden
arden compile examples/single_file/stdlib_and_system/22_args/22_args.arden --emit-llvm
arden run examples/single_file/stdlib_and_system/22_args/22_args.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
