fn main() {
    let bucket_count = 4_096_usize;
    let samples = 8_000_000_i64;

    let mut buckets = vec![0_i64; bucket_count];
    let mut x = 1_i64;
    let mut i = 0_i64;
    while i < samples {
        x = (x * 1_664_525 + 1_013_904_223) % 2_147_483_647;
        let bucket_index = (x as usize) % bucket_count;
        buckets[bucket_index] += 1;
        i += 1;
    }

    let mut checksum = 0_i64;
    for (i, value) in buckets.iter().enumerate() {
        checksum += *value * (i as i64 + 1);
    }

    println!("{checksum}");
}
