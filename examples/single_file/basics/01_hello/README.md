# 01_hello

Focused example: **Hello World**.

What this demonstrates:
- running your first Arden file
- console output with `print/println`
- escaped characters in output strings

Run:

```bash
arden run examples/single_file/basics/01_hello/01_hello.arden
```

Useful command variants:

```bash
arden check examples/single_file/basics/01_hello/01_hello.arden
arden compile examples/single_file/basics/01_hello/01_hello.arden --emit-llvm
arden run examples/single_file/basics/01_hello/01_hello.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
