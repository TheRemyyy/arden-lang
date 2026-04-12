# Async / Await

## Why This Matters

Async lets you model concurrent or latency-heavy work while keeping types explicit and flow readable.

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
task: Task<Integer> = async {
    return 21 * 2;
};

result: Integer = await task;
```

## Task Methods

- `task.is_done(): Boolean`
- `task.cancel(): None`
- `task.await_timeout(ms: Integer): Option<T>`

## Borrowing Interaction

Captures inside async blocks participate in borrow-check rules. Invalid moves/mutations after borrowed capture are rejected.

See [Ownership and Borrowing](ownership.md).

## Examples

- [`14_async`](../../examples/single_file/safety_and_async/14_async/14_async.arden)
- [`28_async_runtime_control`](../../examples/single_file/tooling_and_ffi/28_async_runtime_control/28_async_runtime_control.arden)
