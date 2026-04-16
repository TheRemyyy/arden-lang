package main

import "fmt"

func main() {
	bucketCount := 4096
	samples := int64(8000000)

	buckets := make([]int64, bucketCount)
	x := int64(1)
	for i := int64(0); i < samples; i++ {
		x = (x*1664525 + 1013904223) % 2147483647
		bucketIndex := int(x % int64(bucketCount))
		buckets[bucketIndex]++
	}

	checksum := int64(0)
	for i, value := range buckets {
		checksum += value * (int64(i) + 1)
	}

	fmt.Println(checksum)
}
