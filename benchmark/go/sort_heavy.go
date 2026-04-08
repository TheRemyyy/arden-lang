package main

import "fmt"

func main() {
	const n = 20000
	a := make([]int64, n)
	for p := int64(0); p < n; p++ {
		a[p] = ((p*1_664_525 + 1_013_904_223) % 2_147_483_647) % 100_000
	}

	// Insertion sort — same algorithm as all three language variants
	for i := 1; i < n; i++ {
		key := a[i]
		j := i
		for j > 0 && a[j-1] > key {
			a[j] = a[j-1]
			j--
		}
		a[j] = key
	}

	var checksum int64
	for q := 0; q < n; q++ {
		checksum += a[q] * int64(q+1)
	}
	fmt.Println(checksum)
}
