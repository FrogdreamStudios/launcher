# Default target
help:
	@echo "Available commands:"
	@echo "  dev          	- Run the application in development mode"
	@echo "  build        	- Build the application"
	@echo "  build-release 	- Build the application in release mode"
	@echo "  test         	- Run all tests"
	@echo "  lint         	- Run clippy linter"
	@echo "  lint-strict  	- Run clippy with strict settings"
	@echo "  fix          	- Auto-fix clippy issues"
	@echo "  fmt          	- Format code"
	@echo "  fmt-check    	- Check code formatting"
	@echo "  clean        	- Clean build artifacts"
	@echo "  install-deps 	- Install development dependencies"
	@echo "  check-all    	- Run all checks (format, lint, test)"
	@echo "  pre-commit   	- Run pre-commit checks"
	@echo "  watch        	- Watch for changes and run clippy automatically"

# Development
run:
	cargo run

# Build commands
build:
	cargo build

# Release build
release:
	cargo build --release

# Testing
test:
	cargo test --all-features

# Linting and formatting
lint:
	cargo clippy --all-targets --all-features

lint-strict:
	cargo clippy --all-targets --all-features -- -D warnings

fix:
	cargo clippy --all-targets --all-features --fix

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

# Maintenance
clean:
	cargo clean
	rm -rf target/
	rm -rf node_modules/

install-deps:
	rustup component add clippy
	rustup component add rustfmt
	cargo install cargo-watch
	npm install

# Combined checks
check-all: fmt-check lint test
	@echo "All checks passed!"

pre-commit: fmt lint-strict
	@echo "Pre-commit checks completed!"

# Watch for changes and run clippy automatically
watch:
	cargo watch -x "clippy --all-targets --all-features" -x "run"

# Watch only clippy (no run)
watch-lint:
	cargo watch -x "clippy --all-targets --all-features"

# Development with auto-reload
dev-watch:
	cargo watch -x "run"
