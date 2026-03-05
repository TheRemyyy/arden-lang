# Quick Start

This guide will get you up and running with your first Apex program.

## Hello World

Create a file named `hello.apex` with the following content:

```apex
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
apex run hello.apex
```

#### 2. Separate Compilation

To compile to a native executable:

```bash
apex compile hello.apex
```

This produces an executable named `hello` (or `hello.exe` on Windows). Run it as usual:

```bash
./hello       # Linux/macOS
.\hello.exe   # Windows
```

#### 3. Syntax Checking

To check for errors without compiling (faster for development):

```bash
apex check hello.apex
```

## Your First Real Program

Here is a more complex example demonstrating variables, loops, string interpolation, and mutability.

`program.apex`:

```apex
import std.io.*;

function main(): None {
    // Immutable variables
    name: String = "Apex";
    
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
apex run program.apex
```

## Next Steps

- Explore the [Syntax Guide](../basics/syntax.md) to learn about the language structure.
- Check out the [Examples](../../examples/) directory for more complex programs.
