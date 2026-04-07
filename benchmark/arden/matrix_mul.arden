import std.io.*;

function idx(i: Integer, j: Integer, n: Integer): Integer {
    return i * n + j;
}

function main(): None {
    size: Integer = 100;
    total: Integer = size * size;

    a: List<Integer> = List<Integer>();
    b: List<Integer> = List<Integer>();
    c: List<Integer> = List<Integer>();

    // Fill matrices A and B deterministically.
    for (p in 0..total) {
        a.push(((p * 17 + 13) % 97) - 48);
        b.push(((p * 31 + 7) % 89) - 44);
        c.push(0);
    }

    mut i: Integer = 0;
    while (i < size) {
        mut j: Integer = 0;
        while (j < size) {
            mut sum: Integer = 0;
            mut k: Integer = 0;
            while (k < size) {
                left: Integer = a.get(idx(i, k, size));
                right: Integer = b.get(idx(k, j, size));
                sum = sum + (left * right);
                k = k + 1;
            }
            c.set(idx(i, j, size), sum);
            j = j + 1;
        }
        i = i + 1;
    }

    mut checksum: Integer = 0;
    mut q: Integer = 0;
    while (q < total) {
        checksum = checksum + c.get(q);
        q = q + 1;
    }

    println(to_string(checksum));
    return None;
}
