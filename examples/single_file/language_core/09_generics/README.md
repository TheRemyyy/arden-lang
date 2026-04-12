# 09_generics

Focused example: **Generics**.

What this demonstrates:
- generic types and functions
- `List<T>/Map<K,V>/Option<T>/Result<T,E>` usage
- composable type-safe abstractions

Run:

```bash
arden run examples/single_file/language_core/09_generics/09_generics.arden
```

Useful command variants:

```bash
arden check examples/single_file/language_core/09_generics/09_generics.arden
arden compile examples/single_file/language_core/09_generics/09_generics.arden --emit-llvm
arden run examples/single_file/language_core/09_generics/09_generics.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
