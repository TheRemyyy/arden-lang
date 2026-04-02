package main

import "fmt"

func idx(i, j, n int) int {
	return i*n + j
}

func main() {
	const size = 220
	const total = size * size

	a := make([]int64, total)
	b := make([]int64, total)
	c := make([]int64, total)

	for p := 0; p < total; p++ {
		a[p] = int64((p*17+13)%97 - 48)
		b[p] = int64((p*31+7)%89 - 44)
	}

	for i := 0; i < size; i++ {
		for j := 0; j < size; j++ {
			var sum int64
			for k := 0; k < size; k++ {
				sum += a[idx(i, k, size)] * b[idx(k, j, size)]
			}
			c[idx(i, j, size)] = sum
		}
	}

	var checksum int64
	for q := 0; q < total; q++ {
		checksum += c[q]
	}

	fmt.Println(checksum)
}
