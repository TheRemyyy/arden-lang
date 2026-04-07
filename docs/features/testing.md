# Testing

Arden includes a built-in test framework based on attributes and assertion helpers.

The goal is to keep normal test workflows inside the same compiler CLI instead of forcing a separate external runner.

## Minimal Test

```arden
@Test
function testAddition(): None {
    assert_eq(2 + 2, 4);
    return None;
}
```

Run it with:

```bash
arden test --path test_math.arden
```

## Test Attributes

### `@Test`

Marks a function as a test case.

### `@Ignore`

Skips a test.

```arden
@Test
@Ignore("waiting for feature")
function skipped(): None {
    fail("should not run");
    return None;
}
```

### `@Before` / `@After`

Run before and after each test.

Use these for per-test setup/cleanup.

### `@BeforeAll` / `@AfterAll`

Run once per suite.

Use these when repeated setup would be wasteful.

## Assertion Helpers

Common built-ins:

- `assert(condition)`
- `assert_true(condition)`
- `assert_false(condition)`
- `assert_eq(a, b)`
- `assert_ne(a, b)`
- `fail()`
- `fail("message")`

Assertion helpers can also be stored as typed function values.

## CLI

```bash
arden test
arden test --list
arden test --filter math
arden test --path tests/
arden test --path tests/math_test.arden
```

Options:

- `-p, --path <file-or-dir>`
- `-l, --list`
- `-f, --filter <pattern>`

## Typical Workflows

### Run The Current Project's Tests

```bash
arden test
```

### List Tests Without Executing

```bash
arden test --list
```

### Run A Focused Subset

```bash
arden test --filter math
```

### Point At A Specific Directory Or File

```bash
arden test --path tests/
arden test --path tests/math_test.arden
```

## Behavior Notes

- without `--path`, project mode uses the current project's configured files
- directory discovery walks nested folders
- generated runner files are isolated from the source tree
- ignored tests are reported but not executed
- `--list` shows discovered tests without running them

## Complete Example

```arden
@BeforeAll
function initSuite(): None {
    println("starting tests");
    return None;
}

@Test
function testNumbers(): None {
    assert_eq(3 * 7, 21);
    return None;
}

@Test
@Ignore("example")
function skipped(): None {
    fail("should not run");
    return None;
}
```

## When To Prefer `arden test`

Use the built-in test runner when:

- you want attribute-driven unit-style coverage
- you are already in project mode
- you want test discovery integrated with the compiler

Use the repository example sweep scripts when:

- you changed compiler behavior broadly
- you want to validate the public example corpus
- you need a regression pass beyond one project's tests

Reference example:

- [examples/24_test_attributes.arden](../../examples/24_test_attributes.arden)
- [scripts/README.md](../../scripts/README.md)
