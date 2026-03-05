# Async / Await

Apex has first-class support for asynchronous programming.

## Async Functions

Define an async function using `async`. It returns a `Task<T>`.

```apex
async function fetchData(): Task<String> {
    return "Data";
}
```

## Await

Use `await` to resolve a `Task<T>` into `T`.

```apex
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

Current runtime behavior is thread-backed (`pthread` runtime): multiple tasks can run in parallel, and `await` joins the task if it is not finished yet.

## Async Blocks

`async { ... }` creates a `Task<T>` expression.

```apex
task: Task<Integer> = async {
    return 21 * 2;
};

value: Integer = await task; // value == 42
```
