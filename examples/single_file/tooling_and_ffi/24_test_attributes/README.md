# 24_test_attributes

Focused example: **Test Attributes**.

What this demonstrates:
- `@Test/@Ignore/@Before/@After/...`
- running test discovery via `arden test`
- test lifecycle hooks

Run:

```bash
arden run examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden
```

Run tests:

```bash
arden test --path examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden
```

Useful command variants:

```bash
arden check examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden
arden compile examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden --emit-llvm
arden run examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden -- --demo-arg
```

Repository smoke mode (no rebuild):

```bash
CI_SKIP_COMPILER_BUILD=1 ARDEN_COMPILER_PATH=target/release/arden bash scripts/examples_smoke_linux.sh
```
