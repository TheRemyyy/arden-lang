# 11_lambdas

Focused example: **Lambdas**.

What this demonstrates:
- lambda syntax with typed parameters
- passing functions as values
- callback-style usage

Run:

```bash
arden run examples/single_file/safety_and_async/11_lambdas/11_lambdas.arden
```

Useful command variants:

```bash
arden check examples/single_file/safety_and_async/11_lambdas/11_lambdas.arden
arden compile examples/single_file/safety_and_async/11_lambdas/11_lambdas.arden --emit-llvm
arden run examples/single_file/safety_and_async/11_lambdas/11_lambdas.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
