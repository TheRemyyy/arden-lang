# System Module

## Why This Matters

`System` is the boundary between your Arden program and the host operating system.
If you are not sure when to read env vars, run shell commands, inspect cwd/OS, or return explicit exit codes, start here.

## Quick Mental Model

- `System.getenv(...)` -> read an environment variable
- `System.cwd()` -> get current working directory
- `System.os()` -> detect operating system
- `System.shell(...)` -> run command, get exit code
- `System.exec(...)` -> run command, capture stdout text
- `System.exit(...)` -> terminate program with explicit exit code

Import:

```arden
import std.system.*;
```

For console output, also import I/O:

```arden
import std.io.*;
```

## 1. Environment Variables: `System.getenv(name)`

Use this for runtime configuration (`APP_ENV`, `API_URL`, feature flags).

```arden
import std.io.*;
import std.system.*;

function main(): None {
    env: String = System.getenv("APP_ENV");
    if (env == "") {
        println("APP_ENV not set");
    } else {
        println("APP_ENV={env}");
    }
    return None;
}
```

If the variable is missing, Arden returns an empty string.

## 2. Working Directory: `System.cwd()`

Returns the directory where the process currently runs.

```arden
import std.io.*;
import std.system.*;

function main(): None {
    cwd: String = System.cwd();
    println("cwd={cwd}");
    return None;
}
```

Useful for debugging path issues in CI and scripts.

## 3. OS Detection: `System.os()`

Returns one of: `windows`, `macos`, `linux`, `unknown`.

```arden
import std.io.*;
import std.system.*;

function main(): None {
    os: String = System.os();
    if (os == "windows") {
        println("Using Windows-specific behavior");
    } else {
        println("Using Unix-like behavior");
    }
    return None;
}
```

## 4. Run Command + Exit Code: `System.shell(command)`

Use when you only care whether a command succeeded.

```arden
import std.io.*;
import std.system.*;

function main(): None {
    code: Integer = System.shell("echo hello");
    println("exit code={code}");
    return None;
}
```

## 5. Run Command + Capture Output: `System.exec(command)`

Use when you need stdout text (`whoami`, `git rev-parse`, etc.).

```arden
import std.io.*;
import std.system.*;

function main(): None {
    user: String = System.exec("whoami");
    println("current user={user}");
    return None;
}
```

### `shell` vs `exec`

- need status code only -> `System.shell(...)`
- need command output text -> `System.exec(...)`

## 6. Explicit Exit: `System.exit(code)`

Useful for CLI tools that must return clear OS status.

```arden
import std.system.*;

function main(): None {
    ok: Boolean = true;
    if (!ok) {
        System.exit(1);
    }
    System.exit(0);
}
```

The global builtin `exit` is also available:

```arden
function main(): None {
    stop: (Integer) -> None = exit;
    stop(0);
}
```

## Safety Notes (Important)

- Never pass unvalidated user input directly into `System.shell`/`System.exec`.
  - that is a command-injection risk
- Always check shell exit codes for command-based workflows.
- Do not rely on exact command output formatting across OSes without fallback logic.

## Example In Repo

- [`20_system`](../../examples/single_file/stdlib_and_system/20_system/20_system.arden)
