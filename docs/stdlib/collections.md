# Collections

Arden provides built-in collection types for managing groups of data. These are implemented as efficient intrinsic types.

## List`<T>`

`List<T>` is a dynamic array that grows automatically.

`List<T>(capacity)` optionally preallocates backing storage, but the list still starts empty with `length() == 0`.

> **Note**: `Set<T>` and `Map<K, V>` are currently defined in the type system but standard library (methods) support is still in development. The following documentation for Maps refers to the intended API.

### List Methods

#### `push(element: T): None`

Adds an element to the end of the list.

```arden
list: List<Integer> = List<Integer>();
list.push(42);
```

#### `pop(): T`

Removes and returns the last element from the list.

```arden
last: Integer = list.pop();
```

#### `get(index: Integer): T`

Returns the element at the specified index. Panics if index is out of bounds.

```arden
val: Integer = list.get(0);
```

#### `set(index: Integer, value: T): None`

Updates the element at the specified index. Panics if index is out of bounds.

```arden
list.set(0, 100);
```

#### `length(): Integer`

Returns the number of elements in the list.

```arden
size: Integer = list.length();
```

## Map`<K, V>`

`Map<K, V>` is a key-value store. Currently implemented as an association list (O(n) lookup), with hash map optimization planned.

### Map Methods

#### `insert(key: K, value: V): None`

Inserts a key-value pair into the map. If the key already exists, the value is updated.

```arden
scores: Map<String, Integer> = Map<String, Integer>();
scores.insert("Alice", 100);
```

#### `get(key: K): V`

Retrieves the value associated with the key. Panics if the key is not found (use `contains` check first).

```arden
score: Integer = scores.get("Alice");
```

#### `contains(key: K): Boolean`

Returns `true` if the map contains the specified key.

```arden
import std.io.*;

if (scores.contains("Alice")) {
    println("Alice found");
}
```

#### `length(): Integer`

Returns the number of key-value pairs in the map.

```arden
count: Integer = scores.length();
```

## Range`<T>`

`Range<T>` represents a sequence of values from start to end (exclusive) with a specified step. It's an iterator-based type for efficient numeric sequences.

### Creating Ranges

```arden
// Basic range (step defaults to 1)
r: Range<Integer> = range(0, 5);     // 0, 1, 2, 3, 4

// Range with custom step
r = range(0, 10, 2);                  // 0, 2, 4, 6, 8

// Counting down
r = range(10, 0, -1);                 // 10, 9, 8, ..., 1

// Float range
r = range(0.0, 1.0, 0.25);            // 0.0, 0.25, 0.5, 0.75
```

`range()` accepts either all-`Integer` arguments or all-`Float` arguments. The optional `step` must be non-zero.

### Range Methods

#### `has_next(): Boolean`

Returns `true` if there are more elements to iterate over.

```arden
r = range(0, 5);
while (r.has_next()) {
    // Iterates 5 times
}
```

#### `next(): T`

Returns the current value and advances the iterator.

```arden
r = range(0, 5);
val: Integer = r.next();  // Returns 0
val = r.next();           // Returns 1
```

### Example: Sum of 1 to N

```arden
function sum_to_n(n: Integer): Integer {
    mut sum: Integer = 0;
    r: Range<Integer> = range(1, n + 1);
    while (r.has_next()) {
        sum = sum + r.next();
    }
    return sum;
}
```

See [Range Types](../features/ranges.md) for detailed documentation.
