.PHONY: lint format build-and-run

## lint: Run cargo clippy and report all warnings
lint:
	cargo clippy

## format: Run cargo fmt to enforce consistent formatting
format:
	cargo fmt

## build-and-run: Build the app and launch it
build-and-run:
	cargo run
