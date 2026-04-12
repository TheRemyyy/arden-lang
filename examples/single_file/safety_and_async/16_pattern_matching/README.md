# 16_pattern_matching

Focused example: **Pattern Matching**.

What this demonstrates:
- `match` branching
- pattern bindings in arms
- exhaustive decision flow

Run:

```bash
arden run examples/single_file/safety_and_async/16_pattern_matching/16_pattern_matching.arden
```

Useful command variants:

```bash
arden check examples/single_file/safety_and_async/16_pattern_matching/16_pattern_matching.arden
arden compile examples/single_file/safety_and_async/16_pattern_matching/16_pattern_matching.arden --emit-llvm
arden run examples/single_file/safety_and_async/16_pattern_matching/16_pattern_matching.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
