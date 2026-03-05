# Standard Library

The Apex Standard Library (`std`) provides core functionality for building applications.

## Modules

- [Math](math.md): Mathematical functions and constants.
- [Str](string.md): String manipulation utilities.
- [Time](time.md): Time retrieval and sleeping.
- [File](io.md): File system operations.
- [System](system.md): System-level interactions (exit, getenv, etc.).
- [Args](args.md): Command-line arguments.
- [Collections](collections.md): Built-in List and Map types.
- [I/O](io.md): Console input and output.

## Import Behavior (Important)

The stdlib is implemented as **compiler intrinsics**, but import behavior is split:

- `print`, `println`, and `read_line` are free functions in `std.io` and should be imported:
  - `import std.io.*;` (or specific function imports).
- Module-style APIs such as `Math.*`, `Str.*`, `Time.*`, `System.*`, `File.*`, and `Args.*` are intrinsic objects and are available directly in the current compiler.
- Builtins like `to_string`, `range`, `exit`, and assertion helpers (`assert*`, `fail`) are available without import.

There are no external `.apex` stdlib source files; calls are lowered directly by the compiler/codegen pipeline.
