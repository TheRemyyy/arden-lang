# Attributes

## Why This Matters

Attributes are compiler-checked metadata on declarations (primarily functions/methods).
They control test discovery and effect contracts.

## Where Attributes Are Allowed

Attributes are supported on functions and class methods.
Applying attributes to class/enum/interface/module declarations is rejected.
Unknown attribute names are rejected.

## Attribute Families

Arden currently uses two main groups:

- testing attributes (`@Test`, `@Ignore`, `@Before`, `@After`, `@BeforeAll`, `@AfterAll`)
- effect attributes (`@Pure`, `@Io`, `@Thread`, `@Net`, `@Alloc`, `@Unsafe`, `@Any`)

## Placement Rules (Important)

Supported placement today:

- function declarations
- class methods

Rejected placement:

- classes
- enums
- interfaces
- modules

Unknown attribute names are rejected with a compiler error.

## Testing Attributes

### `@Test`

Marks function as a test discovered by `arden test`.

### `@Ignore`

Skips a test.
Can be used with optional reason:

```arden
@Test
@Ignore("flaky on CI")
function skipped(): None {
    return None;
}
```

### `@Before` and `@After`

Hooks run around each test.

### `@BeforeAll` and `@AfterAll`

Hooks run once before/after full suite.

### Typical Lifecycle

1. `@BeforeAll`
2. for each `@Test` (not ignored): `@Before` -> test -> `@After`
3. `@AfterAll`

Validation rules enforced by compiler/test runner:

- duplicate attributes of same kind are rejected (for example duplicate `@Io`)
- lifecycle attributes are mutually exclusive on one function (`@Before`, `@After`, `@BeforeAll`, `@AfterAll`)
- suite allows at most one `@BeforeAll` and one `@AfterAll`
- `@Ignore` without `@Test` is rejected
- `@Test` cannot be combined with lifecycle attributes
- test/lifecycle functions must be synchronous `function ...(): None` with no params and no generics

## Effect Attributes

### `@Pure`

Function must remain side-effect free.

### `@Io`, `@Thread`, `@Net`, `@Alloc`, `@Unsafe`

Declare specific effect categories function is allowed to use.

### `@Any`

Escape hatch that allows mixed effects in one function.

Important rules:

- `@Pure` cannot be combined with explicit effects
- `@Pure` cannot be combined with `@Any`
- missing required effect on caller is a type-check error
- required effects propagate transitively through intermediate wrappers
- calling an `@Any` function requires `@Any` on caller path (pure/effect-specific callers cannot call `@Any`)

`@Net` is an effect contract category.
It is not the same thing as having runtime `Net.*` stdlib calls.

## Practical Diagnostics You Will See

Typical compile-time failures:

- `@Pure` function calling `println`/`File.*`/`System.*`
- caller missing required propagated effect (`@Io`, `@Thread`, ...)
- transitive wrapper case: `@Io` caller -> unannotated wrapper -> `@Net` callee (fails with missing `net`)
- malformed `@Ignore(...)` usage on non-test declaration
- unknown attribute typo (`@Tset` instead of `@Test`)

When in doubt:

1. annotate boundary/public functions explicitly
2. keep helpers inferred or narrowly annotated
3. avoid `@Any` unless function is intentionally orchestration-heavy

Quick mental check before commit:

1. does any callee in this chain require `@Net`/`@Io`/`@Thread`/...?
2. if yes, are all callers in the path annotated (or intentionally `@Any`)?

## Where To Use Which

- tests and test lifecycle: testing attributes
- side-effect contracts in production logic: effect attributes

## Examples

- test attributes: [`24_test_attributes`](../../examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden)
- effect core: [`26_effect_system`](../../examples/single_file/tooling_and_ffi/26_effect_system/26_effect_system.arden)
- effect inference + `@Any`: [`29_effect_inference_and_any`](../../examples/single_file/tooling_and_ffi/29_effect_inference_and_any/29_effect_inference_and_any.arden)
- effect attribute matrix: [`41_effect_attributes_reference`](../../examples/single_file/tooling_and_ffi/41_effect_attributes_reference/41_effect_attributes_reference.arden)

## Related

- [Testing](testing.md)
- [Effects](../advanced/effects.md)
