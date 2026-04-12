# 03_math

Focused example: **Math Operations**.

What this demonstrates:
- integer and float arithmetic
- `std.math` function calls
- type conversion in calculations

Run:

```bash
arden run examples/single_file/basics/03_math/03_math.arden
```

Useful command variants:

```bash
arden check examples/single_file/basics/03_math/03_math.arden
arden compile examples/single_file/basics/03_math/03_math.arden --emit-llvm
arden run examples/single_file/basics/03_math/03_math.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
