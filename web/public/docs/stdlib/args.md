# Args Module

The `Args` module provides access to command-line arguments passed to the program.

## Functions

The `Args` object provides static methods for argument retrieval.

### `Args.count(): Integer`

Returns the total number of command-line arguments, including the program name itself at index 0.

```apex
import std.io.*;

count: Integer = Args.count();
println("Received {count} arguments");
```

You can also import the function directly by symbol:

```apex
import std.args.count as count;

println("argc = {count()}");
```

That direct symbol alias can also be stored in a typed function value:

```apex
import std.args.get as get;

fetch: (Integer) -> String = get;
```

### `Args.get(index: Integer): String`

Returns the argument at the specified index as a `String`. 

- Index `0` is always the path to the executable.
- Indices `1` and above are user-provided arguments.

```apex
import std.io.*;

if (Args.count() > 1) {
    firstParam: String = Args.get(1);
    println("First argument: {firstParam}");
}
```
