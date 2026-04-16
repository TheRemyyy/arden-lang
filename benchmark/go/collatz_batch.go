package main

import "fmt"

func collatzSteps(n int64) int64 {
	value := n
	steps := int64(0)
	for value != 1 {
		if value%2 == 0 {
			value /= 2
		} else {
			value = value*3 + 1
		}
		steps++
	}
	return steps
}

func main() {
	limit := int64(120000)
	checksum := int64(0)
	for n := int64(2); n <= limit; n++ {
		checksum += collatzSteps(n) * ((n % 97) + 1)
	}
	fmt.Println(checksum)
}
