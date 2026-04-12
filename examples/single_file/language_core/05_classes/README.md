# 05_classes

Focused example: **Classes**.

What this demonstrates:
- class fields and constructors
- methods and object state mutation
- basic object model in Arden

Run:

```bash
arden run examples/single_file/language_core/05_classes/05_classes.arden
```

Useful command variants:

```bash
arden check examples/single_file/language_core/05_classes/05_classes.arden
arden compile examples/single_file/language_core/05_classes/05_classes.arden --emit-llvm
arden run examples/single_file/language_core/05_classes/05_classes.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
