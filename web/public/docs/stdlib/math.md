# Math Module

## Why This Matters

Math functions are core utility tools for deterministic numeric logic.

Import:

```arden
import std.math.*;
```

## Constants

- `Math.pi(): Float`
- `Math.e(): Float`

## Frequently Used Functions

- `Math.abs(x: Integer): Integer`
- `Math.sqrt(x: Float): Float`
- `Math.pow(base: Float, exp: Float): Float`
- `Math.min/max<T>(a: T, b: T): T`
- `Math.floor/ceil/round(x: Float): Float`
- `Math.log/log10/exp(x: Float): Float`
- `Math.random(): Float`

## Example

```arden
import std.io.*;
import std.math.*;

x: Integer = Math.abs(-7);
root: Float = Math.sqrt(9.0);
power: Float = Math.pow(2.0, 8.0);

println("abs={x}, sqrt={root}, pow={power}");
```

## Conversions

- `to_int(x: Float | Integer | String): Integer`
- `to_float(x: Integer | Float): Float`

## Function Value Usage

```arden
import std.math.*;

root_fn: (Integer) -> Float = Math.sqrt;
rand_fn: () -> Float = Math.random;
```

## Edge Behavior

`Math.abs(Integer)` rejects minimum signed overflow edge case explicitly.

## Common Mistakes

- mixing integer and float assumptions in one formula
- using `Math.random()` for security-sensitive randomness
- forgetting conversion when API expects `Float`

## Example In Repo

- [`03_math`](../../examples/single_file/basics/03_math/03_math.arden)
