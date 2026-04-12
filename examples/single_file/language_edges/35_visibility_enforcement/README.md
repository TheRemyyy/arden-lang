# 35_visibility_enforcement

Focused example: **Visibility Enforcement**.

What this demonstrates:
- `public/private/protected` usage
- compile-time access control
- encapsulation boundaries

Run:

```bash
arden run examples/single_file/language_edges/35_visibility_enforcement/35_visibility_enforcement.arden
```

Useful command variants:

```bash
arden check examples/single_file/language_edges/35_visibility_enforcement/35_visibility_enforcement.arden
arden compile examples/single_file/language_edges/35_visibility_enforcement/35_visibility_enforcement.arden --emit-llvm
arden run examples/single_file/language_edges/35_visibility_enforcement/35_visibility_enforcement.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
