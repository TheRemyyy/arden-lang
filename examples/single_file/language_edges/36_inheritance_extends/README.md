# 36_inheritance_extends

Focused example: **Inheritance with extends**.

What this demonstrates:
- class inheritance via `extends`
- method override behavior
- base and derived type model

Run:

```bash
arden run examples/single_file/language_edges/36_inheritance_extends/36_inheritance_extends.arden
```

Useful command variants:

```bash
arden check examples/single_file/language_edges/36_inheritance_extends/36_inheritance_extends.arden
arden compile examples/single_file/language_edges/36_inheritance_extends/36_inheritance_extends.arden --emit-llvm
arden run examples/single_file/language_edges/36_inheritance_extends/36_inheritance_extends.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
