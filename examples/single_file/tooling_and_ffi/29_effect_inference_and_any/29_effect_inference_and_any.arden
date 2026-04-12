// Test 29: Effect Inference + @Any
// Tests: automatic effect inference and @Any escape hatch

import std.io.*;
import std.time.*;

function ioLeaf(msg: String): None {
    println(msg);
    return None;
}

function ioWrapper(): None {
    ioLeaf("inferred io effect propagated");
    return None;
}

@Any
function permissiveWorker(): None {
    println("any: io allowed");
    Time.sleep(1);
    println("any: thread effect allowed");
    return None;
}

function main(): None {
    println("=== Effect Inference + @Any ===");
    ioWrapper();
    permissiveWorker();
    println("done");
    return None;
}
