package main

import "fmt"

func main() {
	size := 131072
	rounds := int64(7)

	values := make([]int64, size)
	indices := make([]int, size)
	scratch := make([]int64, size)

	for i := 0; i < size; i++ {
		values[i] = ((int64(i)*1103515245 + 12345) % 2147483647) - 1073741824
		indices[i] = int((int64(i)*8191 + 127) % int64(size))
	}

	for round := int64(0); round < rounds; round++ {
		for i := 0; i < size; i++ {
			src := indices[i]
			left := values[src]
			right := values[(src+1)%size]
			scratch[i] = left + right + round
		}

		for i := 0; i < size; i++ {
			target := indices[int((int64(i)*13+round)%int64(size))]
			mixed := scratch[i]*31 + int64(i)*17 + round
			values[target] = mixed % 2147483647
		}
	}

	checksum := int64(0)
	for i, value := range values {
		checksum += value * ((int64(i) % 29) + 1)
	}

	fmt.Println(checksum)
}
