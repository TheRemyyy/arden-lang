# Arden Examples

Examples are organized for progressive learning, not as a flat file dump.

## Layout

- `single_file/` - focused language/tooling examples grouped by topic
- `demos/` - larger app-shaped single-file programs
- `starter_project/`, `minimal_project/`, `nested_package_project/`, `showcase_project/` - multi-file project examples

Each single-file example folder contains:

- `<example>.arden`
- `README.md`

## Recommended Path

1. `single_file/basics/01_hello/01_hello.arden`
2. `single_file/language_core/05_classes/05_classes.arden`
3. `single_file/safety_and_async/10_ownership/10_ownership.arden`
4. `single_file/safety_and_async/14_async/14_async.arden`
5. `single_file/safety_and_async/41_async_boundary_rules/41_async_boundary_rules.arden`
6. `single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden`
7. `single_file/language_edges/35_visibility_enforcement/35_visibility_enforcement.arden`
8. `single_file/language_edges/44_exact_import_values/44_exact_import_values.arden`
9. `single_file/language_edges/45_interface_inline_body_rules/45_interface_inline_body_rules.arden`
10. `starter_project/`

## Run

```bash
arden run examples/single_file/basics/01_hello/01_hello.arden
```

Project example:

```bash
cd examples/starter_project
arden check --timings
arden run --timings
```

## Compiler Flags You Will Actually Use

```bash
arden compile examples/single_file/basics/01_hello/01_hello.arden --emit-llvm
arden run examples/single_file/basics/01_hello/01_hello.arden --no-check
```

## Advanced Build Perf Knobs

Large-project profiling / tuning:

```bash
ARDEN_OBJECT_SHARD_THRESHOLD=1 ARDEN_OBJECT_SHARD_SIZE=2 arden build --timings
```

Program args passthrough:

```bash
arden run examples/single_file/stdlib_and_system/22_args/22_args.arden -- --demo-arg
```

## No-build Smoke Loop

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```

## Category Indexes

- [single_file/README.md](single_file/README.md)
- [demos/README.md](demos/README.md)
