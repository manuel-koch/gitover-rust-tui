.PHONY: lint format build-and-run test

## lint: Run cargo check/clippy and report all warnings
lint:
	cargo check
	cargo clippy

## format: Run cargo fmt to enforce consistent formatting
format:
	cargo fmt

## build-and-run: Build the app and launch it
build-and-run:
	cargo run

## test: Run all unit and integration tests
test:
	cargo test
