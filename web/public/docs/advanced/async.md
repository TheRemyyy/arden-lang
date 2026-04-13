# Async / Await

## Why This Matters

Async lets you represent latency/concurrency work with explicit types.
In Arden, `Task<T>` keeps async state visible in signatures, so callers know what must be awaited.

## Core Model

- `async function` returns `Task<T>`
- `await` converts `Task<T>` into `T`
- `async { ... }` creates inline task expressions

If you are new to async:

- `Task<T>` means "result will exist later"
- `await` means "pause here until that result is ready"
- without `await`, you are still holding deferred work, not final data

## Basic Usage

```arden
async function fetchData(): Task<String> {
    return "Data";
}

async function load(): Task<String> {
    value: String = await fetchData();
    return value;
}
```

## Async Blocks

```arden
async function mainAsync(): Task<None> {
    task: Task<Integer> = async {
        return 21 * 2;
    };

    result: Integer = await task;
    return None;
}
```

## Task Methods

- `task.is_done(): Boolean`
- `task.cancel(): None`
- `task.await_timeout(ms: Integer): Option<T>`

### When To Use Which

- normal code path: `await task`
- polling loop / non-blocking checks: `task.is_done()`
- bounded wait with fallback: `task.await_timeout(ms)`
- cooperative stop request: `task.cancel()`

`await_timeout` rules enforced by compiler:

- argument must be `Integer`
- negative compile-time constants are rejected

## Runtime Behavior Guidance

- `await` is the normal completion path
- `is_done()` is for polling-style checks
- `await_timeout(...)` is for bounded waiting with fallback logic
- `cancel()` requests task cancellation; design code so cancellation is safe/idempotent where possible

## Runnable Timeout Pattern

```arden
import std.io.*;
import std.time.*;

function slow(): Task<Integer> {
    return async {
        Time.sleep(200);
        return 7;
    };
}

function main(): None {
    maybe: Option<Integer> = slow().await_timeout(50);
    if (maybe.is_some()) {
        println("completed: {maybe.unwrap()}");
    } else {
        println("timed out");
    }
    return None;
}
```

This pattern is the safest default for latency-sensitive code: always handle both
`Some(value)` and `None` branches explicitly.

## Borrowing Interaction

Captures inside async blocks participate in borrow-check rules.
Invalid moves/mutations after borrowed capture are rejected at compile time.

See [Ownership and Borrowing](ownership.md).

## Async Borrow Boundary Rules (Important)

Current compiler boundary rules:

- async function parameters cannot contain borrowed references (`&T`, `&mut T`, or nested borrowed-reference-bearing types)
- async blocks cannot capture bindings whose types contain borrowed references

Typical invalid patterns:

```arden
// invalid
// async function bad(x: &String): Task<Integer> { return 1; }

// invalid
// s: String = "x";
// r: &String = &s;
// t: Task<Integer> = async { return Str.len(*r); };
```

Safe default: convert to owned async boundaries.
Do borrowed work synchronously, then pass/move owned values into async functions/blocks.

## Common Mistakes

- forgetting that async APIs return `Task<T>` (not `T`)
- using wrong timeout type for `await_timeout`
- writing long async flows without explicit timeout or cancellation strategy
- attempting to pass/capture borrowed references across async boundaries

## Related

- [Effects](effects.md)
- [Error Handling](error_handling.md)
- Examples:
  - [`14_async`](../../examples/single_file/safety_and_async/14_async/14_async.arden)
  - [`41_async_boundary_rules`](../../examples/single_file/safety_and_async/41_async_boundary_rules/41_async_boundary_rules.arden)
  - [`28_async_runtime_control`](../../examples/single_file/tooling_and_ffi/28_async_runtime_control/28_async_runtime_control.arden)
