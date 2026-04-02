#[inline]
fn idx(i: usize, j: usize, n: usize) -> usize {
    i * n + j
}

fn main() {
    let size: usize = 220;
    let total = size * size;

    let mut a = vec![0i64; total];
    let mut b = vec![0i64; total];
    let mut c = vec![0i64; total];

    for p in 0..total {
        a[p] = ((p as i64 * 17 + 13) % 97) - 48;
        b[p] = ((p as i64 * 31 + 7) % 89) - 44;
    }

    for i in 0..size {
        for j in 0..size {
            let mut sum = 0i64;
            for k in 0..size {
                sum += a[idx(i, k, size)] * b[idx(k, j, size)];
            }
            c[idx(i, j, size)] = sum;
        }
    }

    let checksum: i64 = c.iter().copied().sum();
    println!("{checksum}");
}
