# Range Types

## Why This Matters

Ranges are Arden's core numeric iteration primitive and power `for` loops and manual iteration patterns.

## Constructing Ranges

```arden
r1: Range<Integer> = range(0, 5);
r2: Range<Integer> = range(0, 10, 2);
rf: Range<Float> = range(0.0, 1.0, 0.25);
```

## Rules

- arguments must be consistently numeric (`Integer` set or `Float` set)
- step must be non-zero

## Iteration API

```arden
while (r1.has_next()) {
    value: Integer = r1.next();
    println(to_string(value));
}
```

## In `for` Loops

```arden
for (i in 5) {
    println("{i}");
}
```

## Related

- [Control Flow](../basics/control_flow.md)
- Example: [`25_range_types`](../../examples/single_file/stdlib_and_system/25_range_types/25_range_types.arden)
