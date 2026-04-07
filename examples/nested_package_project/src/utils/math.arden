// Math utilities - namespace: utils.math
package utils.math;

// Exported functions
function factorial(n: Integer): Integer {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

function power(base: Float, exp: Integer): Float {
    if (exp == 0) {
        return 1.0;
    }
    if (exp < 0) {
        return 1.0 / power(base, -exp);
    }
    mut result: Float = 1.0;
    mut i: Integer = 0;
    while (i < exp) {
        result = result * base;
        i = i + 1;
    }
    return result;
}

function is_prime(n: Integer): Boolean {
    if (n <= 1) {
        return false;
    }
    if (n <= 3) {
        return true;
    }
    if (n % 2 == 0) {
        return false;
    }
    
    mut i: Integer = 3;
    while (i * i <= n) {
        if (n % i == 0) {
            return false;
        }
        i = i + 2;
    }
    return true;
}
