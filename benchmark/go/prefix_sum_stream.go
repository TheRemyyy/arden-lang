package main

import "fmt"

func main() {
	size := 262144
	passes := 4

	input := make([]int64, size)
	output := make([]int64, size)
	for i := 0; i < size; i++ {
		input[i] = ((int64(i)*48271 + 13) % 65521) - 32760
	}

	for pass := 0; pass < passes; pass++ {
		running := int64(0)
		for i := 0; i < size; i++ {
			running += input[i]
			output[i] = running
		}
		for i := 0; i < size; i++ {
			input[i] = output[i] % 104729
		}
	}

	checksum := int64(0)
	for i, value := range output {
		checksum += value * ((int64(i) % 17) + 1)
	}

	fmt.Println(checksum)
}
