# Arden Examples

Examples are organized for progressive learning, not as a flat file dump.

## Layout

- `single_file/` - focused language/tooling examples grouped by topic
- `demos/` - larger app-shaped single-file programs
- `starter_project/`, `minimal_project/`, `nested_package_project/`, `showcase_project/` - multi-file project examples

Each single-file example has its own folder with:

- `<example>.arden`
- `README.md`

## Recommended Path

1. `single_file/basics/01_hello/01_hello.arden`
2. `single_file/language_core/05_classes/05_classes.arden`
3. `single_file/safety_and_async/10_ownership/10_ownership.arden`
4. `single_file/safety_and_async/14_async/14_async.arden`
5. `single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden`
6. `single_file/language_edges/35_visibility_enforcement/35_visibility_enforcement.arden`
7. `starter_project/`

## Run

```bash
arden run examples/single_file/basics/01_hello/01_hello.arden
```

Project example:

```bash
cd examples/starter_project
arden run
```

## Category Indexes

- [single_file/README.md](single_file/README.md)
- [demos/README.md](demos/README.md)
