# Math Module

Mathematical functions and constants. All functions are available as static methods on the `Math` object.

Import requirement:

```apex
import std.math.*;
```

Calls like `Math.abs(...)` are import-checked and require `std.math` to be imported.

## Constants

### `Math.pi(): Float`
Returns the value of PI (~3.14159).

### `Math.e(): Float`
Returns the value of Euler's number (~2.71828).

## Functions

| Function | Type Signature | Description |
| :--- | :--- | :--- |
| `Math.abs` | `(x: Integer) -> Integer` | Absolute value of an integer. |
| `Math.sqrt` | `(x: Float) -> Float` | Square root. |
| `Math.pow` | `(base: Float, exp: Float) -> Float` | Power function. |
| `Math.sin` | `(x: Float) -> Float` | Sine (radians). |
| `Math.cos` | `(x: Float) -> Float` | Cosine (radians). |
| `Math.tan` | `(x: Float) -> Float` | Tangent (radians). |
| `Math.floor` | `(x: Float) -> Float` | Floor rounding. |
| `Math.ceil` | `(x: Float) -> Float` | Ceiling rounding. |
| `Math.round` | `(x: Float) -> Float` | Nearest integer rounding. |
| `Math.log` | `(x: Float) -> Float` | Natural logarithm. |
| `Math.log10` | `(x: Float) -> Float` | Base 10 logarithm. |
| `Math.exp` | `(x: Float) -> Float` | Exponential (e^x). |
| `Math.random` | `() -> Float` | Random number between 0.0 and 1.0. |
| `Math.min` | `<T>(a: T, b: T) -> T` | Smaller of two values. |
| `Math.max` | `<T>(a: T, b: T) -> T` | Larger of two values. |

`Math.abs(Integer)` fails fast on the minimum signed `Integer` value because its positive counterpart cannot be represented in the same type.

## Conversions

| Function | Type Signature | Description |
| :--- | :--- | :--- |
| `to_int` | `(x: Float | Integer | String) -> Integer` | Float to Integer (truncates), Integer identity, or decimal String to Integer. |
| `to_float` | `(x: Integer | Float) -> Float` | Integer to Float, or Float identity. |
