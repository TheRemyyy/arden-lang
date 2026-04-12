# 42_build_timings_and_shards

Focused example: **Build Timings, Program Args, and Shard Tuning Knobs**.

What this demonstrates:
- forwarding runtime args via `arden run ... -- ...`
- practical `--timings` usage in project mode
- advanced object-codegen shard tuning env vars for perf diagnostics

Run:

```bash
arden run examples/single_file/tooling_and_ffi/42_build_timings_and_shards/42_build_timings_and_shards.arden -- --demo-arg
```

Useful command variants:

```bash
arden check examples/single_file/tooling_and_ffi/42_build_timings_and_shards/42_build_timings_and_shards.arden
arden compile examples/single_file/tooling_and_ffi/42_build_timings_and_shards/42_build_timings_and_shards.arden --emit-llvm
ARDEN_OBJECT_SHARD_THRESHOLD=1 ARDEN_OBJECT_SHARD_SIZE=2 arden build --timings
```

Notes:
- `ARDEN_OBJECT_SHARD_THRESHOLD` default is `256`
- `ARDEN_OBJECT_SHARD_SIZE` default is `4`
- shard env vars are advanced build tuning knobs (project mode), not language semantics

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
