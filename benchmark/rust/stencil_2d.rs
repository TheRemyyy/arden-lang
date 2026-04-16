#[inline(always)]
fn idx(x: usize, y: usize, width: usize) -> usize {
    y * width + x
}

fn main() {
    let width = 256_usize;
    let height = 256_usize;
    let steps = 5_usize;
    let total = width * height;

    let mut current = Vec::with_capacity(total);
    let mut next = vec![0_i64; total];
    for i in 0..total {
        current.push((((i as i64) * 48_271 + 97) % 131_071) - 65_535);
    }

    for _ in 0..steps {
        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                let center = current[idx(x, y, width)];
                let north = current[idx(x, y - 1, width)];
                let south = current[idx(x, y + 1, width)];
                let west = current[idx(x - 1, y, width)];
                let east = current[idx(x + 1, y, width)];
                next[idx(x, y, width)] = center * 4 + north + south + west + east;
            }
        }

        for i in 0..total {
            current[i] = next[i] % 104_729;
        }
    }

    let mut checksum = 0_i64;
    for (i, value) in current.iter().enumerate() {
        checksum += *value * (((i as i64) % 31) + 1);
    }

    println!("{checksum}");
}
