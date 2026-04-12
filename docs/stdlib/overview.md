# Standard Library

## Why This Matters

This page maps common runtime capabilities to the right module and import.
If a stdlib call unexpectedly fails, this is the first place to check.

## Module Map

- [I/O + File](io.md)
- [Math](math.md)
- [Strings (`Str`)](string.md)
- [Time](time.md)
- [System](system.md)
- [Args](args.md)
- [Collections](collections.md)

## Import Rules (Important)

Arden stdlib is compiler-intrinsic, but module usage still requires explicit imports:

- console I/O: `import std.io.*;`
- file API (`File.*`): `import std.fs.*;`
- math (`Math.*`): `import std.math.*;`
- strings (`Str.*`): `import std.string.*;`
- time (`Time.*`): `import std.time.*;`
- system (`System.*`): `import std.system.*;`
- args (`Args.*`): `import std.args.*;`

## Builtins Available Without Import

- conversions: `to_string`, `to_int`, `to_float`
- ranges: `range`
- process exit: `exit`
- assertions: `assert`, `assert_eq`, `assert_ne`, `assert_true`, `assert_false`
- panic helper: `fail`

## Function Values

You can store builtin and stdlib members as typed function values:

```arden
import std.args.*;
import std.math.*;
import std.system.*;

conv: (Integer) -> Float = to_float;
cwd: () -> String = System.cwd;
argc: () -> Integer = Args.count;
rand: () -> Float = Math.random;
```

## Quick Decision Guide

- logging/user text output -> `std.io`
- file read/write/delete/exists -> `std.fs`
- numeric operations/constants/random -> `std.math`
- string transforms/search/compare -> `std.string`
- OS/env/shell/cwd/exit code behavior -> `std.system`
- command-line arg parsing -> `std.args`
- time formatting/sleep/unix timestamp -> `std.time`

## Common Mistakes

- assuming module symbols are globally available without import
- mixing `System.shell` (exit code) and `System.exec` (stdout text)
- calling `Args.get(i)` without checking `Args.count()` first
- treating `Str.compare(a, b)` as boolean instead of integer relation (`<0`, `0`, `>0`)

## Where To Start

- new user: [I/O](io.md), [Args](args.md), [String](string.md)
- systems tooling: [System](system.md), [Time](time.md)
- numeric work: [Math](math.md), [Collections](collections.md)
