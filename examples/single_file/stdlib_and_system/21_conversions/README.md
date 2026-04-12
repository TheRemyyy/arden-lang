# 21_conversions

Focused example: **Type Conversions**.

What this demonstrates:
- `to_int`, `to_float`, `to_string`
- safe scalar conversion flow
- combining conversions with math/output

Run:

```bash
arden run examples/single_file/stdlib_and_system/21_conversions/21_conversions.arden
```

Useful command variants:

```bash
arden check examples/single_file/stdlib_and_system/21_conversions/21_conversions.arden
arden compile examples/single_file/stdlib_and_system/21_conversions/21_conversions.arden --emit-llvm
arden run examples/single_file/stdlib_and_system/21_conversions/21_conversions.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
