# Testing

## Why This Matters

Testing is built into Arden CLI, so correctness checks stay in the same workflow as build and run.
For beginners: you mark functions with `@Test` and run `arden test`.

## Minimal Test

```arden
@Test
function sampleTest(): None {
    assert_eq(1, 1);
    return None;
}
```

## Run Tests

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

## Test Attributes

- `@Test`: marks a test
- `@Ignore`: skips a test (optional reason)
- `@Before`: runs before each test
- `@After`: runs after each test
- `@BeforeAll`: runs once before suite
- `@AfterAll`: runs once after suite

## Signature Rules (Compiler-Enforced)

`@Test`, `@Before`, `@After`, `@BeforeAll`, `@AfterAll` functions must be:

- non-`extern`
- non-`async`
- parameterless
- non-generic
- return type `None`

`@Ignore` requires `@Test` on the same function.
`@Test` cannot be combined with lifecycle attributes.
Each suite allows at most one `@BeforeAll` and one `@AfterAll`.

`@Ignore` with reason example:

```arden
@Test
@Ignore("waiting for fixture update")
function notReadyYet(): None {
    return None;
}
```

## Execution Order

1. `@BeforeAll`
2. per non-ignored test: `@Before` -> test -> `@After`
3. `@AfterAll`

## Practical Guidance

- keep tests deterministic and independent
- test one behavior per test where possible
- use `--list` to verify discovery before long runs
- use `--filter` for fast local iteration on one failure
- use `fail("why this is invalid")` when a branch should be unreachable in a test

## Common Mistakes

- hidden shared mutable state between tests
- assertions that depend on timing/random/environment
- relying only on happy-path tests
- tagging async/extern/parameterized function as `@Test`
- using `@Ignore` on non-`@Test` function

## Related

- [Attributes](attributes.md)
- Example: [`24_test_attributes`](../../examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden)
