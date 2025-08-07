# Variables
CARGO := cargo
CARGO_WATCH := cargo-watch
CARGO_AUDIT := cargo-audit
CARGO_CLIPPY := $(CARGO) clippy --all-targets --all-features -- -D warnings
CARGO_TEST := $(CARGO) test
CARGO_BUILD := $(CARGO) build
CARGO_FMT := $(CARGO) fmt

GO := go
GO_VERSION := 1.21.5
GO_DEV_DIR := dev

EXCLUDED_DIRS := --exclude target --exclude node_modules --exclude package.json --exclude package-lock.json
TOKEI := tokei

# Default target
all: fmt clippy build
	@echo "Build completed successfully!"

# Help target
help:
	@echo "Available targets:"
	@echo "  all                  - Run fmt, clippy, and build (default)"
	@echo "  help                 - Display this help message"
	@echo "  clippy               - Run clippy with warnings as errors"
	@echo "  fmt                  - Format code and verify formatting"
	@echo "  test                 - Execute all tests"
	@echo "  build                - Build the project"
	@echo "  release              - Build the project in release mode"
	@echo "  run                  - Run the project"
	@echo "  clean                - Remove build artifacts"
	@echo "  watch                - Watch for file changes and rebuild"
	@echo "  install-tools        - Install essential development tools"
	@echo "  loc                  - Count lines of code"
	@echo "  quick                - Run fmt, clippy, and build"
	@echo "  pre-commit           - Run pre-commit checks (fmt, clippy, test)"
	@echo "  fix-all              - Run comprehensive fixes and checks"
	@echo "  check-tools          - Verify all required tools are installed"
	@echo "  go-install           - Install Go programming language"
	@echo "  go-version-analyzer  - Run Go version of version analyzer"
	@echo "  go-version-manager   - Run Go version of version manager"
	@echo "  go-build             - Build all Go projects"
	@echo "  go-clean             - Clean Go build artifacts"

# Check for required tools
check-tools:
	@echo "Checking for required tools..."
	@command -v $(CARGO) >/dev/null 2>&1 || { echo "Error: cargo is not installed"; exit 1; }
	@command -v $(GO) >/dev/null 2>&1 || echo "Warning: go not installed, use 'make go-install' to install"
	@command -v $(TOKEI) >/dev/null 2>&1 || echo "Warning: tokei not installed, will install when needed"
	@command -v $(CARGO_WATCH) >/dev/null 2>&1 || echo "Warning: cargo-watch not installed, will install when needed"
	@command -v $(CARGO_AUDIT) >/dev/null 2>&1 || echo "Warning: cargo-audit not installed, will install when needed"
	@echo "Tool check completed!"

# Install Go programming language
go-install:
	@echo "Checking for Go installation..."
	@if command -v $(GO) >/dev/null 2>&1; then \
		echo "Go is already installed: $$($(GO) version)"; \
	else \
		echo "Installing Go $(GO_VERSION)..."; \
		if [[ "$$OSTYPE" == "darwin"* ]]; then \
			if command -v brew >/dev/null 2>&1; then \
				brew install go; \
			else \
				echo "Please install Homebrew first or download Go from https://golang.org/dl/"; \
				exit 1; \
			fi; \
		elif [[ "$$OSTYPE" == "linux-gnu"* ]]; then \
			if command -v apt >/dev/null 2>&1; then \
				sudo apt update && sudo apt install -y golang-go; \
			elif command -v yum >/dev/null 2>&1; then \
				sudo yum install -y golang; \
			elif command -v pacman >/dev/null 2>&1; then \
				sudo pacman -S go; \
			else \
				echo "Please install Go manually from https://golang.org/dl/"; \
				exit 1; \
			fi; \
		else \
			echo "Unsupported OS. Please install Go manually from https://golang.org/dl/"; \
			exit 1; \
		fi; \
		echo "Go installed successfully!"; \
	fi

# Build all Go projects
go-build: go-install
	@echo "Building Go projects..."
	@if [ -d "$(GO_DEV_DIR)/version-manager" ]; then \
		echo "Building version-manager..."; \
		cd $(GO_DEV_DIR)/version-manager && $(GO) mod tidy && $(GO) build -o version-manager .; \
	fi
	@echo "Go build completed!"

# Clean Go build artifacts
go-clean:
	@echo "Cleaning Go build artifacts..."
	@if [ -d "$(GO_DEV_DIR)/version-manager" ]; then \
		cd $(GO_DEV_DIR)/version-manager && $(GO) clean && rm -f version-manager; \
	fi
	@echo "Go clean completed!"

# Run Go version of version manager
# Run Go version of version analyzer
go-version-analyzer: go-install
	@echo "Running Go version analyzer..."
	@if [ -d "$(GO_DEV_DIR)/version-analyzer" ]; then \
		cd $(GO_DEV_DIR)/version-analyzer && $(GO) mod tidy && $(GO) run .; \
	else \
		echo "Go version-analyzer not found in $(GO_DEV_DIR)/version-analyzer"; \
		exit 1; \
	fi

# Run Go version of version manager
go-version-manager: go-install
	@echo "Running Go version manager..."
	@if [ -d "$(GO_DEV_DIR)/version-manager" ]; then \
		cd $(GO_DEV_DIR)/version-manager && $(GO) mod tidy && $(GO) run .; \
	else \
		echo "Go version-manager not found in $(GO_DEV_DIR)/version-manager"; \
		exit 1; \
	fi

# Run clippy with warnings as errors
clippy:
	@echo "Running Clippy..."
	@$(CARGO_CLIPPY) || { echo "Clippy checks failed"; exit 1; }

# Format code and check formatting
fmt:
	@echo "Formatting code..."
	@$(CARGO_FMT)
	@echo "🔍 Checking code formatting..."
	@$(CARGO_FMT) -- --check || { echo "Formatting check failed"; exit 1; }

# Run tests
test:
	@echo "Running tests..."
	@$(CARGO_TEST) || { echo "Tests failed"; exit 1; }

# Build the project
build:
	@echo "Building project..."
	@$(CARGO_BUILD) || { echo "Build failed"; exit 1; }

# Release build
release:
	@echo "Building release version..."
	@$(CARGO) build --release || { echo "Release build failed"; exit 1; }

# Run the project
run:
	@echo "Running project..."
	@$(CARGO) run || { echo "Run failed"; exit 1; }

# Clean build artifacts
clean: go-clean
	@echo "Cleaning build artifacts..."
	@$(CARGO) clean
	@echo "Clean completed!"

# Watch for file changes and rebuild
watch:
	@echo "Watching for changes..."
	@if command -v $(CARGO_WATCH) >/dev/null 2>&1; then \
		$(CARGO_WATCH) -x "clippy -- -D warnings" -x "test" -x "build"; \
	else \
		$(CARGO) install cargo-watch && $(CARGO_WATCH) -x "clippy -- -D warnings" -x "test" -x "build"; \
	fi

# Install essential development tools
install-tools: go-install
	@echo "Installing development tools..."
	@$(CARGO) install cargo-watch cargo-audit cargo-outdated cargo-bloat cargo-tarpaulin flamegraph cargo-criterion cargo-asm cargo-expand tokei cargo-deps cargo-modules cargo-tree cargo-edit || { echo "Failed to install some tools"; exit 1; }
	@echo "Development tools installed successfully!"

# Count lines of code
loc:
	@echo "Counting lines of code..."
	@if command -v $(TOKEI) >/dev/null 2>&1; then \
		$(TOKEI) . $(EXCLUDED_DIRS); \
	else \
		$(CARGO) install tokei && $(TOKEI) . $(EXCLUDED_DIRS); \
	fi

# Quick development cycle
quick: fmt clippy build
	@echo "Quick development cycle completed!"

# Pre-commit hook simulation
pre-commit: fmt clippy test
	@echo "Pre-commit checks passed!"

# Comprehensive fix and check
fix-all:
	@echo "Running comprehensive fixes..."
	@$(CARGO) clean
	@$(CARGO) update || { echo "Cargo update failed"; exit 1; }
	@$(CARGO_FMT)
	@$(CARGO) clippy --fix --allow-dirty --allow-staged || { echo "Clippy fix failed"; exit 1; }
	@$(CARGO_TEST) || { echo "Tests failed"; exit 1; }
	@if command -v $(CARGO_AUDIT) >/dev/null 2>&1; then \
		$(CARGO_AUDIT) --fix || true; \
	else \
		$(CARGO) install cargo-audit && $(CARGO_AUDIT) --fix || true; \
	fi
	@$(CARGO_BUILD) || { echo "Build failed"; exit 1; }
	@echo "All fixes completed successfully!"

# Install cargo-edit
install-cargo-edit:
	@echo "Checking for cargo-edit..."
	@if ! command -v cargo-set-version >/dev/null 2>&1; then \
		$(CARGO) install cargo-edit; \
	fi
	@echo "cargo-edit is ready!"
