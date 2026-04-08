fn main() {
    let n: usize = 20000;
    let mut a: Vec<i64> = (0..n as i64)
        .map(|p| ((p * 1_664_525 + 1_013_904_223) % 2_147_483_647) % 100_000)
        .collect();

    // Insertion sort — same algorithm as all three language variants
    for i in 1..n {
        let key = a[i];
        let mut j = i;
        while j > 0 && a[j - 1] > key {
            a[j] = a[j - 1];
            j -= 1;
        }
        a[j] = key;
    }

    let checksum: i64 = a.iter().enumerate().map(|(i, &v)| v * (i as i64 + 1)).sum();
    println!("{checksum}");
}
