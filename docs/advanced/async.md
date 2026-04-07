# Async / Await

Arden has first-class support for asynchronous programming.

## Async Functions

Define an async function using `async`. It returns a `Task<T>`.

```arden
async function fetchData(): Task<String> {
    return "Data";
}
```

## Await

Use `await` to resolve a `Task<T>` into `T`.

```arden
async function loadMessage(): Task<String> {
    data: String = await fetchData();
    return data;
}
```

## Tasks

`Task<T>` is runtime-scheduled:

- Creating a task (`async function` call or `async { ... }`) immediately spawns a runtime worker thread.
- The task body runs concurrently in that worker.
- Subsequent `await` calls return the cached result.

Current runtime behavior is thread-backed: Unix-like platforms use the `pthread` runtime, while Windows uses Win32 thread primitives. Multiple tasks can run in parallel, and `await` joins the task if it is not finished yet.

### Task Methods

- `task.is_done(): Boolean`
- `task.cancel(): None`
- `task.await_timeout(ms: Integer): Option<T>`

Reference examples:
- `examples/14_async.arden`
- `examples/28_async_runtime_control.arden`

## Async Blocks

`async { ... }` creates a `Task<T>` expression.

```arden
task: Task<Integer> = async {
    return 21 * 2;
};

value: Integer = await task; // value == 42
```

Expression-bodied async blocks also infer the tail expression type directly, so builtin/static calls, lambdas, ranges, and assertion-style expressions work without an explicit `return`.

```arden
import std.string.*;

task: Task<String> = async { Str.upper("arden") };
f: Task<(Integer) -> Integer> = async { (x: Integer) => x + 1 };
r: Task<Range<Integer>> = async { range(0, 3) };
```
