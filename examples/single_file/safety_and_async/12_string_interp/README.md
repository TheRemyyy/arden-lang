# 12_string_interp

Focused example: **String Interpolation**.

What this demonstrates:
- interpolation inside string literals
- mixing scalar values in output
- readable formatting patterns

Run:

```bash
arden run examples/single_file/safety_and_async/12_string_interp/12_string_interp.arden
```

Useful command variants:

```bash
arden check examples/single_file/safety_and_async/12_string_interp/12_string_interp.arden
arden compile examples/single_file/safety_and_async/12_string_interp/12_string_interp.arden --emit-llvm
arden run examples/single_file/safety_and_async/12_string_interp/12_string_interp.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
