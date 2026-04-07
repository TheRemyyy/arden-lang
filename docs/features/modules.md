# Modules

Modules organize code into namespaces.

## Definition

```arden
module Network {
    function connect(): None {
        println("Connecting...");
        return None;
    }
}
```

## Usage

Functions inside a module can be accessed using dot notation.
The compiler lowers `Module.function()` to an internal mangled symbol `Module__function`.

```arden
result: Integer = Math.square(5);
```

You can also alias imports:

```arden
import std.math as math;
import std.io as io;

value: Integer = math.abs(-5);
io.println("{value}");
```

Aliases also work for typed function values:

```arden
import std.math as math;

f: (Integer) -> Integer = math.abs;
```

Builtin free functions with no import requirement can be used the same way:

```arden
make: (Integer, Integer) -> Range<Integer> = range;
check: (Integer, Integer) -> None = assert_eq;
stop: (Integer) -> None = exit;
```

The same also applies to direct stdlib object members:

```arden
cwd: () -> String = System.cwd;
now_unix: () -> Integer = Time.unix;
rand: () -> Float = Math.random;
```

Backward compatibility: direct `Module__function()` calls are still accepted.
