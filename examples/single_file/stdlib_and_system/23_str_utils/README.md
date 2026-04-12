# 23_str_utils

Focused example: **String Utilities**.

What this demonstrates:
- `Str.trim/upper/lower/contains/...`
- text normalization before comparison
- common string utility operations

Run:

```bash
arden run examples/single_file/stdlib_and_system/23_str_utils/23_str_utils.arden
```

Useful command variants:

```bash
arden check examples/single_file/stdlib_and_system/23_str_utils/23_str_utils.arden
arden compile examples/single_file/stdlib_and_system/23_str_utils/23_str_utils.arden --emit-llvm
arden run examples/single_file/stdlib_and_system/23_str_utils/23_str_utils.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
