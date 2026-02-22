# Control Flow

## If Expressions

Conditional execution.

```apex
if (condition) {
    // ...
} else if (other_condition) {
    // ...
} else {
    // ...
}
```

Example:

```apex
x: Integer = 10;
if (x > 5) {
    println("Large");
} else {
    println("Small");
}
```

## Loops

### While Loop

Executes as long as the condition is true.

```apex
mut i: Integer = 0;
while (i < 5) {
    println("{i}");
    i = i + 1;
}
```

### For Loop

Iterates over a range or collection.

**Range iteration:**

```apex
// 'in 5' creates a range from 0 to 4
for (i in 5) {
    println("Iteration {i}");
}

// Using Range<T> explicitly
r: Range<Integer> = range(1, 10, 2);  // 1, 3, 5, 7, 9
while (r.has_next()) {
    val: Integer = r.next();
    println(to_string(val));
}
```

See [Range Types](../features/ranges.md) for more details on ranges.

**Collection iteration:**

```apex
numbers: List<Integer> = List<Integer>();
// ... add items ...
for (n in numbers) {
    println("{n}");
}
```

## Pattern Matching

The `match` statement is a powerful control flow operator.

```apex
val: Integer = 2;
match (val) {
    1 => { println("One"); }
    2 => { println("Two"); }
    _ => { println("Other"); }
}
```

It is exhaustive, meaning all cases must be covered (using `_` as a catch-all if needed).
