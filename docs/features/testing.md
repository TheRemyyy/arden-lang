# Testing Framework

Apex includes a built-in testing framework with annotations and assertions for writing unit tests.

## Writing Tests

Tests are functions marked with the `@Test` attribute:

```apex
@Test
function testAddition(): None {
    result: Integer = 2 + 2;
    assert_eq(result, 4);
}
```

## Test Attributes

### @Test
Marks a function as a test case:

```apex
@Test
function testSomething(): None {
    // Test code here
}
```

### @Ignore
Skips a test (optionally with a reason):

```apex
@Test
@Ignore("Not implemented yet")
function testFutureFeature(): None {
    // This test will be skipped
}

@Test
@Ignore  // No reason given
function testAnotherSkipped(): None {
    // This test will also be skipped
}
```

### @Before
Runs before each test in the suite:

```apex
@Before
function setup(): None {
    // Setup code runs before each test
}
```

### @After
Runs after each test in the suite:

```apex
@After
function teardown(): None {
    // Cleanup code runs after each test
}
```

### @BeforeAll
Runs once before all tests in the suite:

```apex
@BeforeAll
function initSuite(): None {
    // Suite setup code
}
```

### @AfterAll
Runs once after all tests in the suite:

```apex
@AfterAll
function cleanupSuite(): None {
    // Suite cleanup code
}
```

## Assertion Functions

### assert(condition)
Panics if the condition is false:

```apex
assert(x > 0);
assert(name != "");
```

### assert_eq(a, b)
Panics if `a` is not equal to `b`:

```apex
assert_eq(2 + 2, 4);
assert_eq("hello", greeting);
```

### assert_ne(a, b)
Panics if `a` equals `b`:

```apex
assert_ne(0, divisor);
assert_ne("error", status);
```

### assert_true(condition)
Panics if the condition is false (same as `assert`):

```apex
assert_true(isValid);
```

### assert_false(condition)
Panics if the condition is true:

```apex
assert_false(isEmpty);
```

### fail(message)
Unconditionally fails the test:

```apex
fail("This should not be reached");
```

## Running Tests

Use the `apex test` command to discover and run tests:

```bash
# Run all tests in current project
apex test

# Run tests in a specific file
apex test --path tests/math_test.apex

# List tests without running them
apex test --list

# Filter tests by name pattern
apex test --filter "math"
```

Notes:
- Without `--path`, `apex test` uses the current project's `apex.toml` file list when available, so unrelated `*_test.apex` files elsewhere under the working directory are ignored.
- The test runner auto-injects `import std.io.*;` when needed.
- Existing user `main(...)` entrypoints are removed from runner input before generation (including `public function main(...)`) so the generated test entrypoint remains unique.
- Main stripping is signature-aware and avoids stripping comment text that only mentions `function main(...)`.
- `async main(...)` forms are also stripped from test-runner input before generated entrypoint insertion.
- Directory-based discovery now walks nested folders, so `apex test --path tests/` picks up files like `tests/unit/math_spec.apex`.
- Discovery matches `test/spec` case-insensitively, so names like `MathTest.apex` and `USER_SPEC.apex` are picked up too.
- Missing test directories now fail fast with a CLI error instead of being treated as an empty test set.
- Explicit file paths must target `.apex` files; passing a non-Apex file now fails before lex/parse.
- Bare `@Ignore` and `@Ignore("reason")` are both skipped correctly by `apex test`.
- Ignored tests do not run `@Before` or `@After` hooks.
- Final summary `Total` counts all discovered tests, including ignored ones.
- String escapes inside tests follow normal Apex string semantics, so `\n`, `\t`, `\"`, `\\`, `\{`, and `\}` are decoded before execution.
- Ignore reasons containing backslashes, control characters, or literal braces are rendered safely by the generated runner instead of being reinterpreted as escape sequences.
- `apex test --list` also escapes control characters inside ignore reasons so discovery output stays single-line per test.

## Complete Example

```apex
// test_math.apex

import std.io.*;
import std.string.*;

@BeforeAll
function initTests(): None {
    println("=== Starting Math Tests ===");
}

@Before
function setup(): None {
    // Reset state before each test
}

@Test
function testAddition(): None {
    assert_eq(2 + 2, 4);
    assert_eq(10 + 5, 15);
}

@Test
function testMultiplication(): None {
    result: Integer = 6 * 7;
    assert_eq(result, 42);
}

@Test
function testStringConcat(): None {
    assert_eq("Hello, " + "World!", "Hello, World!");
}

@Test
@Ignore("Division by zero check not ready")
function testDivisionByZero(): None {
    // This test will be skipped
}

@After
function teardown(): None {
    // Cleanup after each test
}

@AfterAll
function cleanup(): None {
    println("=== Math Tests Complete ===");
}
```

## Test Output

```
$ apex test --path test_math.apex

========================================
         Apex Test Runner
========================================

--- Running Tests ---

Running: testAddition... [PASS]
Running: testMultiplication... [PASS]
Running: testStringConcat... [PASS]

[IGNORE] testDivisionByZero
      Reason: Division by zero check not ready

========================================
         Test Summary
========================================
Total:   4
Passed:  3
Failed:  0
Ignored: 1

ALL TESTS PASSED
```

## Best Practices

1. **Test one thing at a time**: Each test should verify a single concept
2. **Use descriptive names**: `testDivisionByZero` is better than `test3`
3. **Group related tests**: Use @Before/@After for common setup
4. **Skip with reason**: Always provide a reason when using @Ignore
5. **Assert with messages**: The assertion functions provide clear error messages

## Error Messages

When an assertion fails, you get a clear error message:

```
Assertion failed: values are not equal!
```

The test runner then reports which test failed and exits with error code 1.
