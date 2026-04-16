fn main() {
    let size = 262_144_usize;
    let passes = 4_usize;

    let mut input = Vec::with_capacity(size);
    let mut output = vec![0_i64; size];
    for i in 0..size {
        input.push((((i as i64) * 48_271 + 13) % 65_521) - 32_760);
    }

    for _ in 0..passes {
        let mut running = 0_i64;
        for i in 0..size {
            running += input[i];
            output[i] = running;
        }
        for i in 0..size {
            input[i] = output[i] % 104_729;
        }
    }

    let mut checksum = 0_i64;
    for (i, value) in output.iter().enumerate() {
        checksum += *value * (((i as i64) % 17) + 1);
    }

    println!("{checksum}");
}
