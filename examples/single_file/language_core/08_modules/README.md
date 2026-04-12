# 08_modules

Focused example: **Modules**.

What this demonstrates:
- namespacing via `module`
- feature grouping by domain
- symbol access through module prefixes

Run:

```bash
arden run examples/single_file/language_core/08_modules/08_modules.arden
```

Useful command variants:

```bash
arden check examples/single_file/language_core/08_modules/08_modules.arden
arden compile examples/single_file/language_core/08_modules/08_modules.arden --emit-llvm
arden run examples/single_file/language_core/08_modules/08_modules.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
