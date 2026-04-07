# Quick Start

This guide will get you up and running with your first Arden program.

## Hello World

Create a file named `hello.arden` with the following content:

```arden
import std.io.*;

function main(): None {
    println("Hello, World!");
    return None;
}
```

### Compiling and Running

You can compile and run your program in a few different ways:

#### 1. Compile and Run (One Step)

The `run` command handles compilation and execution automatically:

```bash
arden run hello.arden
```

#### 2. Separate Compilation

To compile to a native executable:

```bash
arden compile hello.arden
```

This produces an executable named `hello` (or `hello.exe` on Windows). Run it as usual:

```bash
./hello       # Linux/macOS
.\hello.exe   # Windows
```

#### 3. Syntax Checking

To check for errors without compiling (faster for development):

```bash
arden check hello.arden
```

#### 4. Formatting

To normalize layout before commits or in CI:

```bash
arden fmt hello.arden
arden fmt --check
arden lint hello.arden
arden fix hello.arden
```

For Unix-like scripting, Arden also accepts a shebang:

```arden
#!/usr/bin/env arden
import std.io.*;

function main(): None {
    println("hello");
    return None;
}
```

## Your First Real Program

Here is a more complex example demonstrating variables, loops, string interpolation, and mutability.

`program.arden`:

```arden
import std.io.*;

function main(): None {
    // Immutable variables
    name: String = "Arden";
    
    // Mutable variable
    mut counter: Integer = 0;
    
    println("Starting count for {name}...");
    
    while (counter < 5) {
        counter = counter + 1;
        println("Count: {counter}");
    }
    
    println("Done!");
    return None;
}
```

Run it:

```bash
arden run program.arden
```

## Next Steps

- Explore the [Syntax Guide](../basics/syntax.md) to learn about the language structure.
- Check out the [Examples](../../examples/) directory for more complex programs.
