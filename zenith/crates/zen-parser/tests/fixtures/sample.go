package sample

import (
	"fmt"
	"io"
)

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

// String returns the string representation.
func (c Config) String() string {
	return c.Name
}

// Handler defines a request handler.
type Handler interface {
	Handle(input string) error
	Name() string
}

// Reader wraps io.Reader with extra methods.
type Reader interface {
	io.Reader
	Close() error
}

// MyInt is a type alias for int.
type MyInt = int

// Callback is a function type.
type Callback func(event string, data []byte) error

// MaxRetries is the maximum number of retries.
const MaxRetries = 3

const (
	StatusOK = iota
	StatusError
	StatusPending
)

// DefaultTimeout is the default timeout in seconds.
var DefaultTimeout = 30

var (
	GlobalCount int
	GlobalName  string
)

// AppError represents an application error.
type AppError struct {
	Code    int
	Message string
}

// Error implements the error interface.
func (e *AppError) Error() string {
	return fmt.Sprintf("error %d: %s", e.Code, e.Message)
}

// NewConfig creates a new Config with defaults.
func NewConfig(name string) *Config {
	return &Config{
		Name:    name,
		Count:   0,
		Enabled: true,
	}
}

// Pair is a generic struct.
type Pair[T any, U any] struct {
	First  T
	Second U
}

// Map applies a function to each element.
func Map[T any, U any](items []T, fn func(T) U) []U {
	result := make([]U, len(items))
	for i, item := range items {
		result[i] = fn(item)
	}
	return result
}

// Direction represents a direction enum via const block.
type Direction int

const (
	North Direction = iota
	South
	East
	West
)

// init initializes the package.
func init() {
	fmt.Println("initialized")
}

// Printf formats and prints.
func Printf(format string, args ...interface{}) {
	fmt.Printf(format, args...)
}

// Divide divides two floats with named returns.
func Divide(a, b float64) (result float64, err error) {
	if b == 0 {
		return 0, fmt.Errorf("division by zero")
	}
	return a / b, nil
}

// Logger is a basic logger.
type Logger struct {
	Level int
}

// Server embeds Config and *Logger.
type Server struct {
	Config
	*Logger
	Port int
	Host string `json:"host"`
}

// Number is a type constraint interface.
type Number interface {
	~int | ~float64
}

// Pi is a typed constant.
const Pi float64 = 3.14159

// StringMap is a map type definition.
type StringMap map[string]string

// EventChan is a channel type definition.
type EventChan chan Event

// Event represents a system event.
type Event struct {
	Kind string
}

// Listen starts the server and accepts variadic options.
func (s *Server) Listen(opts ...string) error {
	return nil
}
