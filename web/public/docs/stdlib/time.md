# Time Module

## Why This Matters

Time functions are used for retries, delays, scheduling checks, timestamps, and logging context.

## Import

```arden
import std.time.*;
```

## Functions

- `Time.now(format: String): String`
- `Time.unix(): Integer`
- `Time.sleep(ms: Integer): None`

## 1. Current Time String

```arden
import std.io.*;
import std.time.*;

function main(): None {
    now_default: String = Time.now("");
    now_iso_like: String = Time.now("%Y-%m-%d %H:%M:%S");
    println("default={now_default}");
    println("custom={now_iso_like}");
    return None;
}
```

`Time.now("")` uses default `%H:%M:%S` format.

## 2. Unix Timestamp

```arden
import std.io.*;
import std.time.*;

function main(): None {
    ts: Integer = Time.unix();
    println("unix={ts}");
    return None;
}
```

## 3. Sleep / Delay

```arden
import std.time.*;
import std.io.*;

function main(): None {
    println("waiting...");
    Time.sleep(250);
    println("done");
    return None;
}
```

## Validation Rule

Negative sleep values are invalid.
If statically known negative, compiler rejects during check; otherwise runtime path errors.

## Effect Note

`Time.*` calls participate in effect checks and require `thread` capability.
See [Effects](../advanced/effects.md).

## Common Mistakes

- using sleep as synchronization substitute for real signaling
- assuming formatted `Time.now` output is stable across all locales/environments
- passing unvalidated timeout values from user input

## Example In Repo

- [`19_time`](../../examples/single_file/stdlib_and_system/19_time/19_time.arden)
