# 25_range_types

Focused example: **Range Types**.

What this demonstrates:
- `range(...)` construction
- iteration with `has_next/next`
- integer and float ranges

Run:

```bash
arden run examples/single_file/stdlib_and_system/25_range_types/25_range_types.arden
```

Useful command variants:

```bash
arden check examples/single_file/stdlib_and_system/25_range_types/25_range_types.arden
arden compile examples/single_file/stdlib_and_system/25_range_types/25_range_types.arden --emit-llvm
arden run examples/single_file/stdlib_and_system/25_range_types/25_range_types.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
