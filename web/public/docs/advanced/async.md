# Async / Await

## Why This Matters

Async lets you represent latency/concurrency work with explicit types.
In Arden, `Task<T>` keeps async state visible in signatures, so callers know what must be awaited.

## Core Model

- `async function` returns `Task<T>`
- `await` converts `Task<T>` into `T`
- `async { ... }` creates inline task expressions

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

`await_timeout` rules enforced by compiler:

- argument must be `Integer`
- negative compile-time constants are rejected

## Runtime Behavior Guidance

- `await` is the normal completion path
- `is_done()` is for polling-style checks
- `await_timeout(...)` is for bounded waiting with fallback logic
- `cancel()` requests task cancellation; design code so cancellation is safe/idempotent where possible

## Borrowing Interaction

Captures inside async blocks participate in borrow-check rules.
Invalid moves/mutations after borrowed capture are rejected at compile time.

See [Ownership and Borrowing](ownership.md).

## Common Mistakes

- forgetting that async APIs return `Task<T>` (not `T`)
- using wrong timeout type for `await_timeout`
- writing long async flows without explicit timeout or cancellation strategy

## Related

- [Effects](effects.md)
- [Error Handling](error_handling.md)
- Examples:
  - [`14_async`](../../examples/single_file/safety_and_async/14_async/14_async.arden)
  - [`28_async_runtime_control`](../../examples/single_file/tooling_and_ffi/28_async_runtime_control/28_async_runtime_control.arden)
