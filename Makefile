.PHONY: build test clean docker-build docker-run

# Basic build commands
build:
	cargo build --release

# Run tests
test:
	cargo test

# Clean up build artifacts
clean:
	cargo clean
	rm -rf target/

# Docker related commands
docker-build:
	docker-compose build

docker-run:
	docker-compose up

# Development commands
dev:
	cargo run

# Lint and format code
lint:
	cargo clippy
	cargo fmt -- --check

# Update dependencies
update:
	cargo update

# Generate documentation
doc:
	cargo doc --no-deps --document-private-items

