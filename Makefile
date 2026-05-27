.PHONY: lint format build build-and-run test release install clean rebuild tag-version outdated-dependencies upgrade-dependencies

# Run cargo check/clippy and report all warnings
lint:
	cargo check
	cargo clippy

# Run cargo fmt to enforce consistent formatting
format:
	cargo fmt

# Remove all build artifacts (forces a full recompilation on next build)
clean:
	cargo clean

# Build debug binary (output: target/debug/gitover)
build:
	cargo build

# Clean all build artifacts, then build the debug binary from scratch
rebuild: clean build

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

# Show available dependency upgrades (within semver bounds)
outdated-dependencies:
	cargo update --dry-run

# Apply dependency upgrades to Cargo.lock (within semver bounds)
upgrade-dependencies:
	cargo update

# Tag HEAD with the version from Cargo.toml (e.g. v0.2.0)
tag-version:
	$(eval VERSION := $(shell grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/'))
	git tag v$(VERSION) HEAD
	@echo "Tagged HEAD as v$(VERSION)"
