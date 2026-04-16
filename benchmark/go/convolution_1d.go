package main

import "fmt"

func main() {
	size := 131072
	kernelSize := 33
	radius := kernelSize / 2

	input := make([]int64, size)
	output := make([]int64, size)
	for i := 0; i < size; i++ {
		input[i] = ((int64(i)*1103515245 + 12345) % 65521) - 32760
	}

	kernel := make([]int64, kernelSize)
	for k := 0; k < kernelSize; k++ {
		kernel[k] = ((int64(k)*97 + 13) % 29) - 14
	}

	for i := 0; i < size; i++ {
		acc := int64(0)
		for tap := 0; tap < kernelSize; tap++ {
			idx := i + tap - radius
			if idx >= 0 && idx < size {
				acc += input[idx] * kernel[tap]
			}
		}
		output[i] = acc
	}

	checksum := int64(0)
	for i, value := range output {
		checksum += value * ((int64(i) % 23) + 1)
	}

	fmt.Println(checksum)
}
