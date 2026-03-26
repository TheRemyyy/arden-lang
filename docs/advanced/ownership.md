# Ownership and Borrowing

Apex uses an ownership system inspired by Rust to ensure memory safety without a garbage collector.

## Ownership Rules

1. Each value in Apex has a variable that's called its **owner**.
2. There can only be one owner at a time.
3. When the owner goes out of scope, the value will be dropped.

## Move Semantics

When you assign a value to another variable or pass it to a function, ownership is transferred (moved).

```apex
s1: String = "hello";
s2: String = s1; // s1 is moved to s2
// println("{s1}"); // Error: s1 is invalid
```

## Borrowing

You can allow other code to access data without taking ownership by using **references**.

### Immutable References

Create an immutable reference with `&`. You can have multiple immutable references.

```apex
function len(s: &String): Integer {
    return strlen(*s); // Dereference might be implicit
}

s1: String = "hello";
leng: Integer = len(&s1); // Pass reference
println("{s1}"); // s1 is still valid
```

Immutable references can be used directly for read access on borrowed values without spelling an explicit `*` first. The compiler now accepts field access, read-only method calls, and indexing through borrowed receivers when the underlying type supports them.

```apex
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
    function get(): Integer { return this.value; }
}

box: Boxed = Boxed(42);
ref: &Boxed = &box;
nums: List<Integer> = List<Integer>(1, 2, 3);
nums_ref: &List<Integer> = &nums;

v: Integer = ref.value;
g: Integer = ref.get();
n: Integer = nums_ref.get(0);
```

### Mutable References

Create a mutable reference with `&mut`. You can only have **one** mutable reference at a time.

```apex
function append(s: &mut String): None {
    // ... modify s
    return None;
}

mut s: String = "hello";
append(&mut s);
```

Mutable references also forward receiver mutability for built-in container and iterator methods. That means `&mut List<T>`, `&mut Map<K, V>`, `&mut Set<T>`, and `&mut Range<T>` can call mutating methods directly, while the corresponding immutable references are rejected for those same calls.

```apex
mut xs: List<Integer> = List<Integer>();
items: &mut List<Integer> = &mut xs;
items.push(1);
items.set(0, 2);

view: &List<Integer> = &xs;
// view.push(3); // Error: mutating method through immutable reference
```

The same mutability forwarding now applies to index assignment. Mutable borrowed containers can be updated through `[]`, including nested field chains, while immutable references are rejected explicitly.

```apex
mut xs: List<Integer> = List<Integer>();
xs.push(1);
items: &mut List<Integer> = &mut xs;
items[0] = 2;

mut table: Map<String, Integer> = Map<String, Integer>();
lookup: &mut Map<String, Integer> = &mut table;
lookup["k"] = 7;

view: &List<Integer> = &xs;
// view[0] = 3; // Error: assign through immutable reference
```

Borrowed values also work correctly when the runtime representation itself is pointer-backed. In particular, borrowed `Range<T>` and borrowed `Task<T>` receivers now dispatch against the underlying runtime object rather than an extra reference layer.

```apex
mut r: Range<Integer> = range(0, 3);
rr: &mut Range<Integer> = &mut r;
first: Integer = rr.next();
more: Boolean = rr.has_next();
```

Field borrows through borrowed class receivers are supported as well, including both immutable and mutable field references.

```apex
class Boxed {
    mut value: Integer;
    constructor(value: Integer) { this.value = value; }
}

mut box: Boxed = Boxed(9);
rb: &mut Boxed = &mut box;
slot: &mut Integer = &mut rb.value;
*slot = 11;
```

Assignments through `*ref` follow the same rule: `*slot = ...` is valid only for `&mut T`. Immutable references remain read-only even when explicitly dereferenced.

Mutable references also stay sound across ordinary helper-function calls. Passing a borrowed local into a normal function now preserves mutations correctly at optimized codegen too.

```apex
function write_ref(r: &mut Integer): None {
    *r = 17;
    return None;
}

mut x: Integer = 5;
write_ref(&mut x);
```

## Lifetimes

(Advanced)
Apex tracks lifetimes to ensure references do not outlive the data they refer to. This is currently handled implicitly by the compiler.

## Borrow Checker Edge Behavior

The compiler enforces these edge cases explicitly:

- **use-after-move** is rejected
- **move while borrowed** is rejected
- **double mutable borrow** is rejected
- immutable borrow state is released after scope exit, so the value is movable again
- lambda captures participate in move/borrow analysis for outer variables
- compound assignment on a currently borrowed variable is rejected
- assignments through nested lvalues (`obj.field = ...`, `arr[i] = ...`) are rejected when the owner is currently borrowed
- method calls on `this` use declared parameter borrow modes (no fallback to default owned move behavior)
- built-in receiver methods now use the correct borrow mode too, including nested chains like `ref.items.push(...)` and `ref.range.next()`
- index assignments through borrowed mutable containers now follow the same root borrow mutability, including nested chains like `ref.items[0] = 1` and `ref.map["k"] = 2`
- methods that mutate `this` only via built-in field receivers, such as `this.items.push(1)`, `this.map.set("k", 2)`, or `this.inner.items.push(1)`, are treated as mutating methods for receiver borrow analysis too
- dereference assignments like `*rx = 19` now compile as ordinary mutable lvalues when `rx` is a valid mutable reference
- compound assignments on lvalues with side effects now evaluate the target only once, so patterns like `factory.make()[0] += 2`, `factory.make_box().value += 2`, and `factory.make_map()["k"] += 2` no longer re-run the receiver call path
- ordinary user-function calls are no longer force-marked as LLVM tail calls, preventing optimizer miscompiles for stack-backed borrowed locals
