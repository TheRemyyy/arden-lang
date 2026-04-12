# Standard Library

## Why This Matters

This page tells you where each common runtime capability lives and which imports you need.

## Module Map

- [I/O + File](io.md)
- [Math](math.md)
- [Strings (`Str`)](string.md)
- [Time](time.md)
- [System](system.md)
- [Args](args.md)
- [Collections](collections.md)

## Import Rules (Important)

Arden stdlib is compiler-intrinsic, but module usage still follows explicit imports:

- console I/O: `import std.io.*;`
- file API (`File.*`): `import std.fs.*;`
- math (`Math.*`): `import std.math.*;`
- strings (`Str.*`): `import std.string.*;`
- time (`Time.*`): `import std.time.*;`
- system (`System.*`): `import std.system.*;`
- args (`Args.*`): `import std.args.*;`

Global builtins available without import include:

- `to_string`, `to_int`, `to_float`
- `range`
- `exit`
- assertions: `assert`, `assert_eq`, `assert_ne`, `assert_true`, `assert_false`
- `fail`

## Function Values

You can store builtin or stdlib members in typed function values:

```arden
import std.args.*;
import std.math.*;
import std.system.*;

conv: (Integer) -> Float = to_float;
cwd: () -> String = System.cwd;
argc: () -> Integer = Args.count;
rand: () -> Float = Math.random;
```

## Where To Start

- new user: [I/O](io.md), [Args](args.md), [String](string.md)
- systems tooling: [System](system.md), [Time](time.md)
- numeric work: [Math](math.md), [Collections](collections.md)
