fn main() {
    let size = 131_072_usize;
    let rounds = 7_i64;

    let mut values = Vec::with_capacity(size);
    let mut indices = Vec::with_capacity(size);
    let mut scratch = vec![0_i64; size];

    for i in 0..size {
        values.push((((i as i64) * 1_103_515_245 + 12_345) % 2_147_483_647) - 1_073_741_824);
        indices.push(((i as i64 * 8_191 + 127) % size as i64) as usize);
    }

    for round in 0..rounds {
        for i in 0..size {
            let src = indices[i];
            let left = values[src];
            let right = values[(src + 1) % size];
            scratch[i] = left + right + round;
        }

        for i in 0..size {
            let target = indices[((i as i64 * 13 + round) as usize) % size];
            let mixed = scratch[i] * 31 + i as i64 * 17 + round;
            values[target] = mixed % 2_147_483_647;
        }
    }

    let mut checksum = 0_i64;
    for (i, value) in values.iter().enumerate() {
        checksum += *value * (((i as i64) % 29) + 1);
    }

    println!("{checksum}");
}
