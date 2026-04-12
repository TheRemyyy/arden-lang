# Collections

## Why This Matters

Collections are where most non-trivial Arden programs spend their time.
Knowing mutation and access semantics prevents subtle logic bugs.

## Import

Collection types are language-level types and constructors.
For output in examples below:

```arden
import std.io.*;
```

## `List<T>`

Use for ordered dynamic sequences.

```arden
xs: List<Integer> = List<Integer>();
xs.push(10);
xs.push(20);
first: Integer = xs.get(0);
xs.set(1, 99);
```

Common methods:

- `push(value: T): None`
- `pop(): T`
- `get(index: Integer): T`
- `set(index: Integer, value: T): None`
- `length(): Integer`

## `Map<K, V>`

Use for key-value lookups.

```arden
scores: Map<String, Integer> = Map<String, Integer>();
scores.insert("alice", 10);
scores.set("bob", 7);
exists: Boolean = scores.contains("alice");
```

Common methods:

- `insert(key: K, value: V): None`
- `set(key: K, value: V): None`
- `get(key: K): V`
- `contains(key: K): Boolean`
- `length(): Integer`

## `Set<T>`

Use for uniqueness checks and membership.

```arden
seen: Set<Integer> = Set<Integer>();
seen.add(42);
has42: Boolean = seen.contains(42);
```

Common methods include `add`, `contains`, `remove`, `length`.

## `Range<T>`

Produced by `range(...)` and consumed through iteration APIs.

```arden
import std.io.*;

r: Range<Integer> = range(0, 5);
while (r.has_next()) {
    println(to_string(r.next()));
}
```

## Mutability Rules (Important)

Mutating operations require mutable access paths:

- mutable owner (`mut xs`)
- mutable borrow (`&mut List<T>`, `&mut Map<K, V>`, ...)

Immutable references can call read methods, but mutating methods and index writes are rejected.

## Common Mistakes

- mutating through immutable path
- forgetting bounds checks before indexing logic
- using `List` for key lookup-heavy workloads where `Map` is better

## Related

- [Range Types](../features/ranges.md)
- [Ownership](../advanced/ownership.md)
