# 06_enums

Focused example: **Enums**.

What this demonstrates:
- enum variants as explicit states
- pattern matching over enums
- finite-state modeling

Run:

```bash
arden run examples/single_file/language_core/06_enums/06_enums.arden
```

Useful command variants:

```bash
arden check examples/single_file/language_core/06_enums/06_enums.arden
arden compile examples/single_file/language_core/06_enums/06_enums.arden --emit-llvm
arden run examples/single_file/language_core/06_enums/06_enums.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
