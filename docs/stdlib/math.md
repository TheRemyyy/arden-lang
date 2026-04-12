# Math Module

## Why This Matters

Math drives validation rules, scoring, geometry, retries/backoff formulas, and
many "small but critical" decisions in system code. A wrong numeric assumption
usually becomes a production bug.

Import:

```arden
import std.math.*;
```

## Constants

- `Math.pi(): Float`
- `Math.e(): Float`

Use these instead of hardcoded approximations.

## API Reference (Current Compiler)

### Core Numeric

- `Math.abs(x: Integer | Float): Integer | Float`
  returns same numeric kind as input
- `Math.min(a: Integer | Float, b: Integer | Float): Integer | Float`
- `Math.max(a: Integer | Float, b: Integer | Float): Integer | Float`
- `Math.pow(base: Integer | Float, exp: Integer | Float): Float`

### Trig / Log / Rounding

- `Math.sqrt(x: Integer | Float): Float`
- `Math.sin(x: Integer | Float): Float`
- `Math.cos(x: Integer | Float): Float`
- `Math.tan(x: Integer | Float): Float`
- `Math.floor(x: Integer | Float): Float`
- `Math.ceil(x: Integer | Float): Float`
- `Math.round(x: Integer | Float): Float`
- `Math.log(x: Integer | Float): Float`
- `Math.log10(x: Integer | Float): Float`
- `Math.exp(x: Integer | Float): Float`

### Constants and Random

- `Math.pi(): Float`
- `Math.e(): Float`
- `Math.random(): Float`

## Integer vs Float Mental Model

- integer operations are discrete (`10 / 3 = 3`)
- float operations keep fractional precision (`10.0 / 3.0 = 3.333...`)

Pick one numeric model per formula and convert explicitly at the boundary.

`Math.min/max` can accept mixed numeric kinds.
Mixed `Integer` + `Float` resolves to `Float`.

## Example

```arden
import std.io.*;
import std.math.*;

function main(): None {
    x: Integer = Math.abs(-7);
    root: Float = Math.sqrt(9.0);
    power: Float = Math.pow(2.0, 8.0);

    println("abs={x}, sqrt={root}, pow={power}");
    return None;
}
```

## Typical Real Use Cases

### Clamp with `min`/`max`

```arden
import std.math.*;

function clampScore(raw: Integer): Integer {
    return Math.max(0, Math.min(100, raw));
}

function main(): None {
    _v: Integer = clampScore(120);
    return None;
}
```

### Deterministic Rounding

```arden
import std.math.*;
import std.io.*;

function cents(amount: Float): Integer {
    return to_int(Math.round(amount * 100.0));
}

function main(): None {
    c: Integer = cents(12.345);
    println("cents={c}");
    return None;
}
```

## Conversions

- `to_int(x: Float | Integer | String): Integer`
- `to_float(x: Integer | Float): Float`

## Function Value Usage

```arden
import std.math.*;

function main(): None {
    root_fn: (Integer) -> Float = Math.sqrt;
    rand_fn: () -> Float = Math.random;
    _r: Float = root_fn(9);
    _n: Float = rand_fn();
    return None;
}
```

## Edge Behavior

`Math.abs(Integer)` rejects minimum signed overflow edge case explicitly.

## Randomness Note

`Math.random()` is fine for simulation, demos, and non-security sampling.
Do not use it for secrets, tokens, auth, or cryptographic workflows.

## Common Mistakes

- mixing integer and float assumptions in one formula
- using implicit truncation where rounding was intended
- using `Math.random()` for security-sensitive randomness
- forgetting conversion when API expects `Float`

## Example In Repo

- [`03_math`](../../examples/single_file/basics/03_math/03_math.arden)
