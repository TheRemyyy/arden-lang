# 07_interfaces

Focused example: **Interfaces**.

What this demonstrates:
- interface contracts
- `implements` on classes
- polymorphic calls through interface types

Current behavior note:
- interface method bodies can be declared inline, but classes should still implement required methods explicitly

Run:

```bash
arden run examples/single_file/language_core/07_interfaces/07_interfaces.arden
```

Useful command variants:

```bash
arden check examples/single_file/language_core/07_interfaces/07_interfaces.arden
arden compile examples/single_file/language_core/07_interfaces/07_interfaces.arden --emit-llvm
arden run examples/single_file/language_core/07_interfaces/07_interfaces.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
