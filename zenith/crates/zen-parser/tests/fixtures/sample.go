package sample

import "fmt"

// ProcessItems processes a slice of items and returns results.
// It handles errors gracefully.
func ProcessItems(items []string, limit int) ([]string, error) {
	return items[:limit], nil
}

func privateHelper(n int) int {
	return n * 2
}

// Config holds application configuration.
type Config struct {
	Name    string
	Count   int
	Enabled bool
}

// Run executes the config handler.
func (c *Config) Run() error {
	fmt.Println(c.Name)
	return nil
}
