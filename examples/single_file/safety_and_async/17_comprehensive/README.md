# 17_comprehensive

Focused example: **Comprehensive Feature Mix**.

What this demonstrates:
- many language features in one flow
- end-to-end sample structure
- reference for larger single-file style

Run:

```bash
arden run examples/single_file/safety_and_async/17_comprehensive/17_comprehensive.arden
```

Useful command variants:

```bash
arden check examples/single_file/safety_and_async/17_comprehensive/17_comprehensive.arden
arden compile examples/single_file/safety_and_async/17_comprehensive/17_comprehensive.arden --emit-llvm
arden run examples/single_file/safety_and_async/17_comprehensive/17_comprehensive.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
