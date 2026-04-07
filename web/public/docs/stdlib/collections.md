# Collections

Arden provides built-in collection and iterator types as compiler-supported intrinsics.

## `List<T>`

`List<T>` is a growable sequence.

Create one with:

```arden
xs: List<Integer> = List<Integer>();
```

Optional capacity:

```arden
xs: List<Integer> = List<Integer>(32);
```

That reserves backing storage, but the list still starts empty.

### Common Methods

- `push(value: T): None`
- `pop(): T`
- `get(index: Integer): T`
- `set(index: Integer, value: T): None`
- `length(): Integer`

Example:

```arden
xs: List<Integer> = List<Integer>();
xs.push(10);
xs.push(20);
xs.set(0, 99);
println("{xs.get(0)}");
```

## `Map<K, V>`

`Map<K, V>` is the built-in key/value collection type.

Create one with:

```arden
scores: Map<String, Integer> = Map<String, Integer>();
```

### Common Methods

- `insert(key: K, value: V): None`
- `get(key: K): V`
- `contains(key: K): Boolean`
- `length(): Integer`

Example:

```arden
scores: Map<String, Integer> = Map<String, Integer>();
scores.insert("Alice", 100);

if (scores.contains("Alice")) {
    println("{scores.get("Alice")}");
}
```

## `Set<T>`

`Set<T>` stores unique values.

Create one with:

```arden
seen: Set<Integer> = Set<Integer>();
```

Common runtime surface includes membership and mutation helpers such as:

- `add(value: T): None`
- `contains(value: T): Boolean`
- `remove(value: T): Boolean`

Use the corresponding example files to confirm current behavior when exploring the language.

## `Range<T>`

`Range<T>` is the iterator type produced by `range(...)`.

Examples:

```arden
r: Range<Integer> = range(0, 5);
r2: Range<Integer> = range(0, 10, 2);
rf: Range<Float> = range(0.0, 1.0, 0.25);
```

Rules:

- arguments must be either all `Integer` or all `Float`
- step must be non-zero

### Common Methods

- `has_next(): Boolean`
- `next(): T`

Example:

```arden
mut total: Integer = 0;
r: Range<Integer> = range(1, 6);

while (r.has_next()) {
    total += r.next();
}
```

See also:

- [Range Types](../features/ranges.md)

