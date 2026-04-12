# 19_time

Focused example: **Time Module**.

What this demonstrates:
- `Time.now`, `Time.unix`, `Time.sleep`
- time formatting examples
- delay and timing scenarios

Run:

```bash
arden run examples/single_file/stdlib_and_system/19_time/19_time.arden
```

Useful command variants:

```bash
arden check examples/single_file/stdlib_and_system/19_time/19_time.arden
arden compile examples/single_file/stdlib_and_system/19_time/19_time.arden --emit-llvm
arden run examples/single_file/stdlib_and_system/19_time/19_time.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
