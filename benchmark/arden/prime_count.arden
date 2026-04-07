import std.io.*;

function is_prime(n: Integer): Boolean {
    if (n < 2) {
        return false;
    }
    if (n == 2) {
        return true;
    }
    if ((n % 2) == 0) {
        return false;
    }

    mut d: Integer = 3;
    while (d * d <= n) {
        if ((n % d) == 0) {
            return false;
        }
        d = d + 2;
    }
    return true;
}

function main(): None {
    limit: Integer = 200000;

    mut count: Integer = 0;
    mut n: Integer = 2;
    while (n <= limit) {
        if (is_prime(n)) {
            count = count + 1;
        }
        n = n + 1;
    }

    println(to_string(count));
    return None;
}
