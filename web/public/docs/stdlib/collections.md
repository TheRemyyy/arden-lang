# Collections

## Why This Matters

Collections are core to everyday Arden code; understanding mutation + access semantics avoids many logic bugs.

## `List<T>`

Create:

```arden
xs: List<Integer> = List<Integer>();
```

Optional capacity:

```arden
xs: List<Integer> = List<Integer>(32);
```

Common methods:

- `push(value: T): None`
- `pop(): T`
- `get(index: Integer): T`
- `set(index: Integer, value: T): None`
- `length(): Integer`

## `Map<K, V>`

Create:

```arden
scores: Map<String, Integer> = Map<String, Integer>();
```

Common methods:

- `insert(key: K, value: V): None`
- `set(key: K, value: V): None`
- `get(key: K): V`
- `contains(key: K): Boolean`
- `length(): Integer`

## `Set<T>`

Create:

```arden
seen: Set<Integer> = Set<Integer>();
```

Common methods include `add`, `contains`, `remove`, `length`.

## `Range<T>`

Produced by `range(...)` and used for numeric iteration.

```arden
r: Range<Integer> = range(0, 5);
while (r.has_next()) {
    println(to_string(r.next()));
}
```

## Borrowing/Mutability Reminder

Mutating methods and index assignment require mutable access paths (`mut` owner or `&mut` reference).

## Related

- [Range Types](../features/ranges.md)
- [Ownership](../advanced/ownership.md)
