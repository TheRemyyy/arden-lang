# Testing

## Why This Matters

Tests are a built-in part of Arden workflow, so verification stays close to source and CI.

## Basic Test

```arden
@Test
function sampleTest(): None {
    assert_eq(1, 1);
    return None;
}
```

Run tests:

```bash
arden test
```

Single file:

```bash
arden test --path examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden
```

List tests without running:

```bash
arden test --list --path examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden
```

Filter by substring:

```bash
arden test --filter Addition --path examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden
```

## Useful Features

- test listing
- ignored tests with optional reason
- suite setup/teardown patterns (by example)

## Practical Guidance

- keep tests deterministic
- favor small focused tests for language semantics
- keep integration-style flows in dedicated suites

## Example

- [`24_test_attributes`](../../examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden)
