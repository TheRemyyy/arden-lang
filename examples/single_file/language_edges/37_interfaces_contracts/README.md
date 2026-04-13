# 37_interfaces_contracts

Focused example: **Interface Contracts**.

What this demonstrates:
- contract-driven API design
- interface implementation and usage
- simple polymorphic flow

Current behavior note:
- keep interface methods explicitly implemented on classes even if interface method body exists inline

Run:

```bash
arden run examples/single_file/language_edges/37_interfaces_contracts/37_interfaces_contracts.arden
```

Useful command variants:

```bash
arden check examples/single_file/language_edges/37_interfaces_contracts/37_interfaces_contracts.arden
arden compile examples/single_file/language_edges/37_interfaces_contracts/37_interfaces_contracts.arden --emit-llvm
arden run examples/single_file/language_edges/37_interfaces_contracts/37_interfaces_contracts.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
