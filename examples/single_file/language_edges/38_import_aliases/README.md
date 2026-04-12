# 38_import_aliases

Focused example: **Import Aliases**.

What this demonstrates:
- `import ... as alias` syntax
- shorter namespace usage
- readability in larger files

Run:

```bash
arden run examples/single_file/language_edges/38_import_aliases/38_import_aliases.arden
```

Useful command variants:

```bash
arden check examples/single_file/language_edges/38_import_aliases/38_import_aliases.arden
arden compile examples/single_file/language_edges/38_import_aliases/38_import_aliases.arden --emit-llvm
arden run examples/single_file/language_edges/38_import_aliases/38_import_aliases.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
