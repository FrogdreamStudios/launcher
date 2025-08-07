.PHONY: all help clippy fmt test build release run clean watch install-tools loc quick pre-commit fix-all analyze-versions check-tools install-cargo-edit ver bump-version

# Variables
CARGO := cargo
PYTHON := python3
TOKEI := tokei
CARGO_WATCH := cargo-watch
CARGO_AUDIT := cargo-audit
CARGO_CLIPPY := $(CARGO) clippy --all-targets --all-features -- -D warnings
CARGO_TEST := $(CARGO) test
CARGO_BUILD := $(CARGO) build
CARGO_FMT := $(CARGO) fmt
EXCLUDED_DIRS := --exclude target --exclude node_modules --exclude package.json --exclude package-lock.json

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
	@echo "  analyze-versions     - Analyze Minecraft versions"
	@echo "  check-tools          - Verify all required tools are installed"
	@echo "  ver                  - Run version manager"

# Check for required tools
check-tools:
	@echo "Checking for required tools..."
	@command -v $(CARGO) >/dev/null 2>&1 || { echo "Error: cargo is not installed"; exit 1; }
	@command -v $(PYTHON) >/dev/null 2>&1 || { echo "Error: python3 is not installed"; exit 1; }
	@command -v $(TOKEI) >/dev/null 2>&1 || echo "Warning: tokei not installed, will install when needed"
	@command -v $(CARGO_WATCH) >/dev/null 2>&1 || echo "Warning: cargo-watch not installed, will install when needed"
	@command -v $(CARGO_AUDIT) >/dev/null 2>&1 || echo "Warning: cargo-audit not installed, will install when needed"
	@echo "Tool check completed!"

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
clean:
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
install-tools:
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

# Analyze Minecraft versions
analyze-versions:
	@echo "Analyzing Minecraft versions..."
	@if command -v $(PYTHON) >/dev/null 2>&1; then \
		$(PYTHON) version_analyzer.py; \
	else \
		echo "Python3 required for version analysis"; \
		echo "Install with: brew install python3 (macOS) or apt install python3 (Ubuntu)"; \
		exit 1; \
	fi

# Install cargo-edit
install-cargo-edit:
	@echo "Checking for cargo-edit..."
	@if ! command -v cargo-set-version >/dev/null 2>&1; then \
		$(CARGO) install cargo-edit; \
	fi
	@echo "cargo-edit is ready!"

# Run version manager
ver: install-cargo-edit
	@echo "Running version manager..."
	@if command -v $(PYTHON) >/dev/null 2>&1; then \
		$(PYTHON) version_manager.py; \
	else \
		echo "Installing Python3..."; \
		if command -v brew >/dev/null 2>&1; then \
			brew install python3; \
		elif command -v apt >/dev/null 2>&1; then \
			sudo apt update && sudo apt install -y python3; \
		else \
			echo "Please install Python 3 manually"; \
			exit 1; \
		fi; \
		$(PYTHON) version_manager.py; \
	fi
