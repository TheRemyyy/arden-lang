# 18_file_io

Focused example: **File I/O**.

What this demonstrates:
- `File.exists/read/write/delete` API
- basic text file workflow
- post-write/delete verification

Run:

```bash
arden run examples/single_file/stdlib_and_system/18_file_io/18_file_io.arden
```

Useful command variants:

```bash
arden check examples/single_file/stdlib_and_system/18_file_io/18_file_io.arden
arden compile examples/single_file/stdlib_and_system/18_file_io/18_file_io.arden --emit-llvm
arden run examples/single_file/stdlib_and_system/18_file_io/18_file_io.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
