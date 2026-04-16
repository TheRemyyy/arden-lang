fn main() {
    let size = 131_072_usize;
    let kernel_size = 33_usize;
    let radius = (kernel_size / 2) as isize;

    let mut input = Vec::with_capacity(size);
    let mut output = vec![0_i64; size];
    for i in 0..size {
        input.push((((i as i64) * 1_103_515_245 + 12_345) % 65_521) - 32_760);
    }

    let mut kernel = Vec::with_capacity(kernel_size);
    for k in 0..kernel_size {
        kernel.push((((k as i64) * 97 + 13) % 29) - 14);
    }

    for i in 0..size {
        let mut acc = 0_i64;
        for tap in 0..kernel_size {
            let idx = i as isize + tap as isize - radius;
            if (0..size as isize).contains(&idx) {
                acc += input[idx as usize] * kernel[tap];
            }
        }
        output[i] = acc;
    }

    let mut checksum = 0_i64;
    for (i, value) in output.iter().enumerate() {
        checksum += *value * (((i as i64) % 23) + 1);
    }

    println!("{checksum}");
}
