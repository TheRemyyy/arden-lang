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
import std.io.*;

function main(): None {
    xs: List<Integer> = List<Integer>();
    xs.push(10);
    xs.push(20);
    first: Integer = xs.get(0);
    xs.set(1, 99);
    println("first={first}, len={xs.length()}");
    return None;
}
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
import std.io.*;

function main(): None {
    scores: Map<String, Integer> = Map<String, Integer>();
    scores.insert("alice", 10);
    scores.set("bob", 7);
    exists: Boolean = scores.contains("alice");
    println("has-alice={exists}, len={scores.length()}");
    return None;
}
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
import std.io.*;

function main(): None {
    seen: Set<Integer> = Set<Integer>();
    seen.add(42);
    seen.add(42); // duplicate does not create second entry
    has42: Boolean = seen.contains(42);
    println("has42={has42}, len={seen.length()}");
    return None;
}
```

Common methods include `add`, `contains`, `remove`, `length`.

## `Range<T>`

Produced by `range(...)` and consumed through iteration APIs.

```arden
import std.io.*;

function main(): None {
    r: Range<Integer> = range(0, 5);
    while (r.has_next()) {
        println(to_string(r.next()));
    }
    return None;
}
```

## Mutability Rules (Important)

Mutating operations require mutable access paths:

- mutable owner (`mut xs`)
- mutable borrow (`&mut List<T>`, `&mut Map<K, V>`, ...)

Immutable references can call read methods, but mutating methods and index writes are rejected.

## Decision Guide

- choose `List<T>` when order/index access matters
- choose `Map<K, V>` when lookup by key is primary operation
- choose `Set<T>` when uniqueness/membership is the primary requirement
- choose `Range<T>` for numeric traversal without allocating collection storage first

## Common Mistakes

- mutating through immutable path
- forgetting bounds checks before indexing logic
- using `List` for key lookup-heavy workloads where `Map` is better

## Related

- [Range Types](../features/ranges.md)
- [Ownership](../advanced/ownership.md)
