# Range Types

Range types provide a way to represent and iterate over sequences of values efficiently.

## Overview

The `Range<T>` type represents a half-open interval `[start, end)` that can be iterated over. It's particularly useful for:
- Looping over numeric sequences
- Generating sequences with custom steps
- Iterating backwards (counting down)
- Walking Float sequences with explicit step sizes

## Creating Ranges

Use the `range()` function to create a range:

```apex
// Range from 0 to 5 (exclusive) - step defaults to 1
r = range(0, 5);        // 0, 1, 2, 3, 4

// Range with custom step
r = range(0, 10, 2);    // 0, 2, 4, 6, 8

// Counting down
r = range(10, 0, -1);   // 10, 9, 8, 7, 6, 5, 4, 3, 2, 1

// Float range
r = range(0.5, 2.0, 0.5); // 0.5, 1.0, 1.5
```

`range()` accepts either all-`Integer` arguments or all-`Float` arguments. Mixed numeric types are rejected. The optional `step` must be non-zero.

`range` is also a first-class builtin function value when a typed function is expected:

```apex
build_ints: (Integer, Integer) -> Range<Integer> = range;
build_floats: (Float, Float, Float) -> Range<Float> = range;
```

## Type Annotation

Explicitly type your range variables:

```apex
r: Range<Integer> = range(0, 10);
r2: Range<Float> = range(0.0, 1.0, 0.25);
```

## Iterator Interface

Ranges implement the iterator protocol with two methods:

### `has_next()` -> Boolean

Returns `true` if there are more elements to iterate over:

```apex
r = range(0, 5);
while (r.has_next()) {
    // Will execute 5 times
}
```

### `next()` -> T

Returns the current value and advances the iterator:

```apex
r = range(0, 5);
val: Integer = r.next();  // Returns 0, iterator now at 1
val = r.next();           // Returns 1, iterator now at 2
```

## Examples

### Basic Iteration

```apex
import std.io.*;

function main(): None {
    println("Counting to 5:");
    r: Range<Integer> = range(0, 5);
    while (r.has_next()) {
        val: Integer = r.next();
        println(to_string(val));
    }
    return None;
}
```

Output:
```
Counting to 5:
0
1
2
3
4
```

### Even Numbers

```apex
function print_even_numbers(): None {
    println("Even numbers from 0 to 10:");
    r = range(0, 11, 2);  // Include 10 by going to 11
    while (r.has_next()) {
        println(to_string(r.next()));
    }
}
```

### Countdown

```apex
function countdown(): None {
    println("Launch countdown:");
    r = range(10, 0, -1);
    while (r.has_next()) {
        println(to_string(r.next()));
    }
    println("Liftoff!");
}
```

### Float Steps

```apex
function sample_curve(): None {
    r: Range<Float> = range(0.0, 1.0, 0.25);
    while (r.has_next()) {
        println(to_string(r.next()));
    }
}
```

### Sum Calculation

```apex
function sum_range(start: Integer, end: Integer): Integer {
    mut sum: Integer = 0;
    r: Range<Integer> = range(start, end);
    while (r.has_next()) {
        sum = sum + r.next();
    }
    return sum;
}

// Usage
result: Integer = sum_range(1, 6);  // Returns 15 (1+2+3+4+5)
```

## Range Semantics

- **Half-open interval**: The start is inclusive, the end is exclusive
  - `range(0, 5)` yields: 0, 1, 2, 3, 4
  - `range(5, 5)` yields: (empty range)
  
- **Step direction matters**: 
  - Positive step: iterates while current < end
  - Negative step: iterates while current > end
  - Zero step is invalid and is rejected
  - Integer and Float ranges both follow the same half-open semantics

- **One-time use**: A Range iterator can only be traversed once. To iterate again, create a new range.

## Implementation Details

Internally, `Range<T>` is implemented as a struct with four fields:
- `start`: The starting value
- `end`: The ending value (exclusive)
- `step`: The increment/decrement amount
- `current`: The current position in the iteration

The struct is heap-allocated and accessed via pointer, making range passing efficient.
