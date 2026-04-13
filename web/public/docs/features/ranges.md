# Range Types

## Why This Matters

Ranges are Arden's core numeric iteration primitive.
You use them in loops, manual iteration, and simple numeric traversal without building lists first.

## Quick Mental Model

- `range(start, end)` -> values from `start` up to `end` (end-exclusive)
- optional third arg is `step`
- works with `Integer` and `Float`
- step must be non-zero

## Constructing Ranges

```arden
import std.io.*;

function main(): None {
    r1: Range<Integer> = range(0, 5);
    r2: Range<Integer> = range(0, 10, 2);
    rf: Range<Float> = range(0.0, 1.0, 0.25);
    println("constructed ranges");
    return None;
}
```

## Manual Iteration (Runnable)

```arden
import std.io.*;

function main(): None {
    r: Range<Integer> = range(0, 5);
    while (r.has_next()) {
        value: Integer = r.next();
        println("value={value}");
    }
    return None;
}
```

Iteration contract:

- `has_next()` tells you whether the current position is within configured bounds
- `next()` advances and returns current value
- `next()` itself does not enforce bounds; calling it after `has_next() == false`
  continues arithmetic progression

Example behavior:

- `range(0, 1)` then repeated `next()` yields `0`, `1`, `2`, ...
- therefore, always gate `next()` with `has_next()` in bounded loops

## In `for` Loops

```arden
import std.io.*;

function main(): None {
    for (i in 5) {
        println("i={i}");
    }
    return None;
}
```

`for (i in 5)` is shorthand numeric iteration and is often the cleanest option.

Descending numeric traversal:

```arden
import std.io.*;

function main(): None {
    r: Range<Integer> = range(10, 0, -2);
    while (r.has_next()) {
        println("v={r.next()}");
    }
    return None;
}
```

## Common Mistakes

- assuming end is included (it is excluded)
- using zero step (`range(0, 10, 0)`) which is invalid
- mismatching numeric kinds across arguments
- using wrong step direction (`range(0, 10, -1)` starts with `has_next=false`)
- calling `next()` after `has_next()` is already `false`

## Related

- [Control Flow](../basics/control_flow.md)
- Example: [`25_range_types`](../../examples/single_file/stdlib_and_system/25_range_types/25_range_types.arden)
