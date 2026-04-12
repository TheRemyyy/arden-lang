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
import std.time.*;

now_default: String = Time.now("");
now_iso_like: String = Time.now("%Y-%m-%d %H:%M:%S");
```

`Time.now("")` uses default `%H:%M:%S` format.

## 2. Unix Timestamp

```arden
import std.time.*;

ts: Integer = Time.unix();
```

## 3. Sleep / Delay

```arden
import std.time.*;
import std.io.*;

println("waiting...");
Time.sleep(250);
println("done");
```

Negative sleep values are invalid (rejected at compile-time when constant, otherwise runtime error).

## Example In Repo

- [`19_time`](../../examples/single_file/stdlib_and_system/19_time/19_time.arden)
