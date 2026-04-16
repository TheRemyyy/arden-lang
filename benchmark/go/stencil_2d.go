package main

import "fmt"

func idx(x int, y int, width int) int {
	return y*width + x
}

func main() {
	width := 256
	height := 256
	steps := 5
	total := width * height

	current := make([]int64, total)
	next := make([]int64, total)
	for i := 0; i < total; i++ {
		current[i] = ((int64(i)*48271 + 97) % 131071) - 65535
	}

	for step := 0; step < steps; step++ {
		for y := 1; y < height-1; y++ {
			for x := 1; x < width-1; x++ {
				center := current[idx(x, y, width)]
				north := current[idx(x, y-1, width)]
				south := current[idx(x, y+1, width)]
				west := current[idx(x-1, y, width)]
				east := current[idx(x+1, y, width)]
				next[idx(x, y, width)] = center*4 + north + south + west + east
			}
		}

		for i := 0; i < total; i++ {
			current[i] = next[i] % 104729
		}
	}

	checksum := int64(0)
	for i, value := range current {
		checksum += value * ((int64(i) % 31) + 1)
	}

	fmt.Println(checksum)
}
