.PHONY: all build clean test setup install migrate run help docker-build docker-up docker-down ci

# Variables
BINARY_NAME=bd-workflow
BINARY_PATH=./bin/$(BINARY_NAME)
MAIN_PATH=./cmd/bd-workflow/main.go
DATA_DIR=./data
BEADS_DIR=./.beads

# Build flags
LDFLAGS=-ldflags "-s -w -X main.version=0.1.0"

# Default target
all: setup build

# Help
help:
	@echo "Beads Workflow System - Available targets:"
	@echo ""
	@echo "  make setup      - Initialize the project (create directories)"
	@echo "  make build      - Build the binary"
	@echo "  make install    - Install the binary to GOPATH/bin"
	@echo "  make migrate    - Run database migrations"
	@echo "  make run        - Build and run the application"
	@echo "  make test       - Run all tests"
	@echo "  make clean      - Clean build artifacts"
	@echo "  make fmt        - Format Go code"
	@echo "  make vet        - Run go vet"
	@echo "  make deps       - Download dependencies"
	@echo "  make dev-setup  - Full development setup"
	@echo ""

# Setup - create necessary directories
setup:
	@echo "Setting up beads-workflow-system..."
	@mkdir -p $(DATA_DIR)
	@mkdir -p $(BEADS_DIR)
	@mkdir -p ./bin
	@echo "✓ Directories created"
	@echo "✓ Run 'make migrate' to initialize databases"

# Development setup - complete setup for development
dev-setup: deps setup migrate
	@echo "✓ Development environment ready"

# Download dependencies
deps:
	@echo "Downloading dependencies..."
	@go mod download
	@go mod tidy
	@echo "✓ Dependencies downloaded"

# Build the binary
build: setup
	@echo "Building $(BINARY_NAME)..."
	@go build $(LDFLAGS) -o $(BINARY_PATH) $(MAIN_PATH)
	@echo "✓ Binary built: $(BINARY_PATH)"

# Install the binary
install: build
	@echo "Installing $(BINARY_NAME)..."
	@go install $(LDFLAGS) $(MAIN_PATH)
	@echo "✓ Binary installed to GOPATH/bin"

# Run database migrations
migrate: setup
	@echo "Running database migrations..."
	@go run $(MAIN_PATH) migrate
	@echo "✓ Migrations completed"

# Build and run
run: build
	@echo "Running $(BINARY_NAME)..."
	@$(BINARY_PATH)

# Run tests (excludes scripts dir which has multiple mains)
test:
	@echo "Running tests..."
	@go test -v ./internal/... ./pkg/... ./cmd/...
	@echo "✓ Tests completed"

# Run tests with coverage
test-cover:
	@echo "Running tests with coverage..."
	@go test -coverprofile=coverage.out ./internal/... ./pkg/...
	@go tool cover -func=coverage.out
	@echo "✓ Coverage report generated"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@rm -rf ./bin
	@rm -rf $(DATA_DIR)
	@go clean
	@echo "✓ Cleaned"

# Format code
fmt:
	@echo "Formatting code..."
	@go fmt ./...
	@echo "✓ Code formatted"

# Run go vet
vet:
	@echo "Running go vet..."
	@go vet ./...
	@echo "✓ Vet completed"

# Lint (requires golangci-lint)
lint:
	@echo "Running linter..."
	@which golangci-lint > /dev/null || (echo "Installing golangci-lint..." && go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest)
	@golangci-lint run ./...
	@echo "✓ Linting completed"

# Quick workflow test
workflow-test: build
	@echo "Testing workflow commands..."
	@$(BINARY_PATH) workflow list
	@echo ""
	@echo "Try running:"
	@echo "  $(BINARY_PATH) workflow start research 'Test workflow' --agent research"

# Create a new migration
migration:
	@read -p "Enter migration name: " name; \
	go run scripts/create_migration.go $$name

# Full CI check
ci: fmt vet test build
	@echo "✓ All CI checks passed"

# Release build (all platforms)
release:
	@echo "Building release binaries..."
	@mkdir -p dist
	@CGO_ENABLED=1 GOOS=darwin GOARCH=arm64 go build $(LDFLAGS) -o dist/$(BINARY_NAME)-darwin-arm64 $(MAIN_PATH)
	@CGO_ENABLED=1 GOOS=darwin GOARCH=amd64 go build $(LDFLAGS) -o dist/$(BINARY_NAME)-darwin-amd64 $(MAIN_PATH)
	@echo "✓ Release binaries built in dist/"

# Docker build
docker-build:
	@echo "Building Docker image..."
	@docker build -t beads-workflow:latest .
	@echo "✓ Docker image built"

# Docker compose up
docker-up:
	@echo "Starting services..."
	@docker compose up -d
	@echo "✓ Services started"

# Docker compose down
docker-down:
	@echo "Stopping services..."
	@docker compose down
	@echo "✓ Services stopped"

# Generate code (if needed)
generate:
	@go generate ./...

# Check for security vulnerabilities (requires govulncheck)
security-check:
	@which govulncheck > /dev/null || (echo "Installing govulncheck..." && go install golang.org/x/vuln/cmd/govulncheck@latest)
	@govulncheck ./...

# Update dependencies
update-deps:
	@go get -u ./...
	@go mod tidy