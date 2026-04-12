# Single-File Examples

Every example lives in its own folder:

- `<example>.arden`
- `README.md`

Categories:

- [basics](basics/README.md)
- [language_core](language_core/README.md)
- [safety_and_async](safety_and_async/README.md)
- [stdlib_and_system](stdlib_and_system/README.md)
- [tooling_and_ffi](tooling_and_ffi/README.md)
- [language_edges](language_edges/README.md)

Useful commands while learning:

```bash
arden check examples/single_file/basics/01_hello/01_hello.arden
arden compile examples/single_file/basics/01_hello/01_hello.arden --emit-llvm
arden run examples/single_file/basics/01_hello/01_hello.arden -- --demo-arg
```

Project-mode perf diagnostics:

```bash
ARDEN_OBJECT_SHARD_THRESHOLD=1 ARDEN_OBJECT_SHARD_SIZE=2 arden build --timings
```

No-build smoke (reuse built compiler):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
