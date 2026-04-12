# Control Flow

Arden provides explicit control-flow constructs with predictable typing and scope behavior.

## Why This Matters

Control flow is where state bugs hide. Arden keeps it explicit: typed conditions, explicit loop variables, exhaustive matching.

## `if` / `else if` / `else`

```arden
import std.io.*;

function main(): None {
    x: Integer = 10;
    if (x > 5) {
        println("Large");
    } else if (x == 5) {
        println("Equal");
    } else {
        println("Small");
    }
    return None;
}
```

## `while`

Use for condition-driven loops.

```arden
import std.io.*;

function main(): None {
    mut i: Integer = 0;
    while (i < 5) {
        println("{i}");
        i += 1;
    }
    return None;
}
```

## `for`

Use for range/iterable-driven loops.

### Range iteration

```arden
import std.io.*;

function main(): None {
    for (i in 5) {
        println("{i}"); // 0..4
    }

    for (i: Float in 5) {
        println("{i}"); // loop binding widened to Float
    }

    r: Range<Integer> = range(1, 10, 2); // 1,3,5,7,9
    while (r.has_next()) {
        value: Integer = r.next();
        println(to_string(value));
    }
    return None;
}
```

`for (i: Float in 5)` widens the loop variable, not iterable type itself.

### Collection iteration

```arden
import std.io.*;

function main(): None {
    numbers: List<Integer> = List<Integer>();
    numbers.push(1);
    numbers.push(2);

    for (n in numbers) {
        println("{n}");
    }

    text: String = "Ahoj";
    for (ch in text) {
        println("{ch}");
    }
    return None;
}
```

Borrowed views are also iterable:

```arden
import std.io.*;

function main(): None {
    numbers: List<Integer> = List<Integer>();
    numbers.push(10);
    numbers.push(20);

    view: &List<Integer> = &numbers;
    for (n in view) {
        println("{n}");
    }
    return None;
}
```

## `match`

`match` is exhaustive and suited for enum/pattern branching.

```arden
import std.io.*;

function main(): None {
    value: Integer = 2;
    match (value) {
        1 => { println("One"); },
        2 => { println("Two"); },
        _ => { println("Other"); }
    }
    return None;
}
```

Use `_` as explicit catch-all when needed.

### Important Rule

`match` must contain at least one arm; empty match statement/expression is rejected.

## Common Mistakes

- using long `if/else` chain where enum+`match` would be clearer
- assuming `for (i in N)` includes `N` (it is end-exclusive)
- forgetting explicit catch-all `_` when not handling all values explicitly

## Related

- [Range Types](../features/ranges.md)
- [Enums](../features/enums.md)
- [Pattern Matching Example](../../examples/single_file/safety_and_async/16_pattern_matching/16_pattern_matching.arden)
