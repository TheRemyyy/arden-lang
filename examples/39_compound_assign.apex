// Test 39: Compound Assignment
// Tests: +=, -=, *=, /= on variables and field targets

import std.io.*;

class Counter {
    mut value: Integer;

    constructor(initial: Integer) {
        this.value = initial;
    }

    function add(delta: Integer): None {
        this.value += delta;
        return None;
    }

    function sub(delta: Integer): None {
        this.value -= delta;
        return None;
    }

    function mul(factor: Integer): None {
        this.value *= factor;
        return None;
    }

    function div(divisor: Integer): None {
        this.value /= divisor;
        return None;
    }
}

function main(): None {
    println("=== Compound Assignment ===");

    mut x: Integer = 10;
    x += 5;
    x -= 3;
    x *= 4;
    x /= 2;
    println("x = {x}");

    mut c: Counter = Counter(20);
    c.value += 4;
    c.value -= 2;
    c.value *= 3;
    c.value /= 2;
    println("counter = {c.value}");

    c.add(5);
    c.sub(1);
    c.mul(2);
    c.div(4);
    println("counter after methods = {c.value}");

    println("=== Compound Assignment OK ===");
    return None;
}
