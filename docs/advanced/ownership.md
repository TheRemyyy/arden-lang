# Ownership and Borrowing

Arden uses ownership and borrowing to prevent memory and mutation bugs at compile time.

## Why This Exists

Without ownership rules, systems code often fails in three expensive ways:

- reading data after it was already moved/freed
- mutating data through one path while another path still expects a stable view
- hiding aliasing bugs until runtime

Arden makes these states illegal before native code is produced. You pay with a few explicit rules (`mut`, `&`, `&mut`, move semantics), and you get predictable behavior without a tracing GC.

## Quick Mental Model (No Rust Background Required)

Think in terms of **who can change a value right now**:

- plain variable (`x`) means you own the value
- `mut x` means the owner may reassign it
- `&x` gives read-only borrowed access
- `&mut x` gives temporary exclusive write access

The compiler guarantees:

1. one mutable access path **or** many immutable access paths
2. no use-after-move
3. no writes through immutable access paths

Snippet note:

- this page intentionally includes focused fragments that may omit `main()`
- use linked example files for fully runnable end-to-end programs

## Ownership Basics

Each non-trivial value has exactly one owner at a time.

```arden
import std.io.*;

s: String = "hello";
t: String = s; // move ownership from s -> t
// println(s); // Error: s is moved
println(t);
```

For values that need runtime cleanup (`String`, collections, classes, tasks, etc.), assignment/pass-by-owned moves ownership.

## `mut` vs non-`mut`

`mut` is on the **binding**, not on the type.

```arden
x: Integer = 1;
// x = 2; // Error: immutable variable

mut y: Integer = 1;
y = 2; // OK
```

If a variable is not `mut`, Arden blocks reassignment even if you try through nested operations.

## Borrowing With `&` (Immutable Borrow)

Use `&` when you need read access without transferring ownership.

```arden
import std.io.*;

function len(s: &String): Integer {
    return Str.len(*s);
}

import std.string.*;
name: String = "arden";
size: Integer = len(&name);
println(name); // still valid, not moved
```

You can create multiple immutable borrows at the same time:

```arden
import std.io.*;

n: Integer = 42;
a: &Integer = &n;
b: &Integer = &n;
println("{*a} {*b}");
```

### Implicit Read Through References

Arden allows read operations through borrowed receivers without forcing explicit `*` everywhere:

```arden
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
    function get(): Integer { return this.value; }
}

box: Boxed = Boxed(42);
ref: &Boxed = &box;

nums: List<Integer> = List<Integer>();
nums.push(10);
view: &List<Integer> = &nums;

v: Integer = ref.value;
g: Integer = ref.get();
n: Integer = view.get(0);
```

## Mutable Borrowing With `&mut`

Use `&mut` when another function/path should mutate your value.

```arden
import std.io.*;

function write_ref(r: &mut Integer): None {
    *r = 17;
    return None;
}

mut x: Integer = 5;
write_ref(&mut x);
println("{x}"); // 17
```

Rules:

- only one active mutable borrow of the same owner
- mutable borrow of an immutable binding is rejected
- while mutably borrowed, direct reassignment of the owner is rejected

Invalid pattern:

```arden
mut x: Integer = 1;
a: &mut Integer = &mut x;
// b: &mut Integer = &mut x; // Error: already mutably borrowed
```

## Borrow Modes In Function Parameters

Arden function parameters have explicit ownership modes:

- `owned value: T` (default): takes ownership (move)
- `borrow value: T`: immutable borrow
- `borrow mut value: T`: borrow-mut mode (caller-side exclusivity contract)

```arden
import std.io.*;

function consume(owned s: String): None { return None; }
function read(borrow s: String): None { println(s); return None; }
function inspect_mut(borrow mut x: Integer): None { _v: Integer = x; return None; }
```

Practical rule:

- for explicit caller-visible in-place mutation semantics, prefer `&mut T` parameters
- use borrow modes to communicate call-site ownership/borrowing intent explicitly

Current compiler behavior notes:

- `borrow mut` requires mutable caller binding
- inside callee, `borrow mut` parameter supports reads and reassignment
- caller-visible mutation propagation is type-dependent
  (for predictable propagation, prefer explicit `&mut T`)

## Method Receiver Mutability (Important)

Mutating methods require mutable access to the receiver. Arden enforces this for both user classes and builtin containers.

```arden
class C {
    mut v: Integer;
    constructor(v: Integer) { this.v = v; }
    function touch(): None { this.v += 1; return None; }
    function get(): Integer { return this.v; }
}

mut c: C = C(1);
r: &mut C = &mut c;
r.touch(); // OK
x: Integer = r.get(); // OK
```

Calling a mutating method through `&C` is rejected.

## Mutability Forwarding For Builtins

This is a key Arden feature: mutable references to builtins forward mutability to mutating methods.

Works for `List`, `Map`, `Set`, and `Range`:

```arden
mut xs: List<Integer> = List<Integer>();
mut m: Map<String, Integer> = Map<String, Integer>();
mut s: Set<Integer> = Set<Integer>();
mut r: Range<Integer> = range(0, 3);

rxs: &mut List<Integer> = &mut xs;
rm: &mut Map<String, Integer> = &mut m;
rs: &mut Set<Integer> = &mut s;
rr: &mut Range<Integer> = &mut r;

rxs.push(1);
rm.set("k", 7);
rs.add(7);
first: Integer = rr.next();
```

Immutable references to these same values can call read methods, but mutating methods are rejected.

## Index Assignment Through Borrowed Containers

Index assignment follows the same mutability rules.

```arden
mut xs: List<Integer> = List<Integer>();
xs.push(1);
rxs: &mut List<Integer> = &mut xs;
rxs[0] = 2; // OK

mut table: Map<String, Integer> = Map<String, Integer>();
rm: &mut Map<String, Integer> = &mut table;
rm["k"] = 10; // OK

view: &List<Integer> = &xs;
// view[0] = 3; // Error: immutable reference assignment
```

This also works through nested field chains on borrowed class receivers.

## Lifetimes In Arden

Arden tracks reference lifetimes automatically. You currently do **not** write lifetime annotations in source.

Practical rule:

- a reference cannot outlive the value it points to
- borrow state is released when the relevant scope ends

```arden
import std.io.*;

function consume(owned s: String): None { return None; }

function main(): None {
    s: String = "hello";

    if (true) {
        r: &String = &s;
        println(*r);
    } // borrow ends here

    consume(s); // OK: move allowed after borrow scope ends
    return None;
}
```

So: lifetimes are real, checked, and important, but implicit in current Arden syntax.

## Borrow Checker Behavior (With Examples)

### 1. Use After Move Is Rejected

```arden
import std.io.*;

function consume(owned s: String): None { return None; }

s: String = "x";
consume(s);
// println(s); // Error: Use of moved value
```

### 2. Move While Borrowed Is Rejected

```arden
function consume(owned s: String): None { return None; }

s: String = "x";
r: &String = &s;
// consume(s); // Error: Cannot move while borrowed
```

### 3. Mutable vs Immutable Borrow Conflicts Are Rejected

```arden
mut x: Integer = 1;
read: &Integer = &x;
// write: &mut Integer = &mut x; // Error: immutably borrowed
```

### 4. Assignment While Borrowed Is Rejected

```arden
mut x: Integer = 10;
r: &mut Integer = &mut x;
// x += 1; // Error: owner is mutably borrowed
```

### 5. Lambda/Async Captures Participate In Borrow Analysis

```arden
import std.io.*;

function consume(owned s: String): None { return None; }

s: String = "x";
f: () -> None = () => println(s);
// consume(s); // Error: captured borrow keeps s borrowed
```

`async { ... }` captures follow the same safety model.

### 6. Nested Assignment Checks The Root Owner

```arden
class C {
    mut value: Integer;
    constructor(v: Integer) { this.value = v; }
}

mut c: C = C(1);
r: &C = &c;
// c.value += 1; // Error: cannot assign through borrowed owner
```

## Practical Guidance

- default to `owned` parameters unless the caller must keep using the value
- use `borrow` for read-only helpers
- use `&mut T` parameters for in-place mutation APIs
- keep borrow scopes small (introduce blocks) when you need to move later
- if a borrow error feels confusing, simplify to one owner variable and one borrow at a time, then rebuild

## Related Docs

- [Memory Management](memory_management.md)
- [Types](../basics/types.md)
- [Variables and Mutability](../basics/variables.md)
- borrow-mut behavior example:
  [`43_borrow_mut_semantics`](../../examples/single_file/tooling_and_ffi/43_borrow_mut_semantics/43_borrow_mut_semantics.arden)
- [Examples](../../examples/README.md)
