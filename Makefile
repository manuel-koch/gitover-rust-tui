.PHONY: lint format build-and-run test release install tag-version

# Run cargo check/clippy and report all warnings
lint:
	cargo check
	cargo clippy

# Run cargo fmt to enforce consistent formatting
format:
	cargo fmt

# Build debug binary (output: target/debug/gitover)
build:
	cargo build

# Build debug binary and launch it
build-and-run:
	cargo run

# Run all unit and integration tests
test:
	cargo test

# Build optimized release binary (output: target/release/gitover)
release:
	cargo build --release

# Build optimized release binary and install it `~/.cargo/bin`
install:
	cargo install --path .

# Tag HEAD with the version from Cargo.toml (e.g. v0.2.0)
tag-version:
	$(eval VERSION := $(shell grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/'))
	git tag v$(VERSION) HEAD
	@echo "Tagged HEAD as v$(VERSION)"
