.PHONY: lint format build-and-run test release install

# Run cargo check/clippy and report all warnings
lint:
	cargo check
	cargo clippy

# Run cargo fmt to enforce consistent formatting
format:
	cargo fmt

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
