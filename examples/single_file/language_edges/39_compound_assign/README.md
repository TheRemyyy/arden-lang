# 39_compound_assign

Focused example: **Compound Assignment**.

What this demonstrates:
- `+=`, `-=`, `*=`, `/=`, `%=` operators
- compound updates on bindings/fields/indexes
- clearer update expressions

Run:

```bash
arden run examples/single_file/language_edges/39_compound_assign/39_compound_assign.arden
```

Useful command variants:

```bash
arden check examples/single_file/language_edges/39_compound_assign/39_compound_assign.arden
arden compile examples/single_file/language_edges/39_compound_assign/39_compound_assign.arden --emit-llvm
arden run examples/single_file/language_edges/39_compound_assign/39_compound_assign.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
