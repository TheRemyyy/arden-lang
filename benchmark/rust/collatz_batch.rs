fn collatz_steps(n: i64) -> i64 {
    let mut value = n;
    let mut steps = 0_i64;

    while value != 1 {
        if value % 2 == 0 {
            value /= 2;
        } else {
            value = value * 3 + 1;
        }
        steps += 1;
    }

    steps
}

fn main() {
    let limit = 120_000_i64;
    let mut checksum = 0_i64;
    let mut n = 2_i64;

    while n <= limit {
        checksum += collatz_steps(n) * ((n % 97) + 1);
        n += 1;
    }

    println!("{checksum}");
}
