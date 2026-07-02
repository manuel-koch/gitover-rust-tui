REPO_ROOT := $(realpath $(dir $(abspath $(firstword $(MAKEFILE_LIST)))))

ifeq ($(CARGO_TARGET_DIR),)
export CARGO_TARGET_DIR := $(REPO_ROOT)/target
endif

.PHONY: lint format \
	build build-and-run release \
	test test-coverage test-coverage-missing \
	install tag-version \
	outdated-dependencies upgrade-dependencies

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

build-and-run-with-sandbox-repos: build
	mkdir -p ~/tmp/gitover-sandbox
	./create-sandbox-repos.sh ~/tmp/gitover-sandbox
	cd ~/tmp/gitover-sandbox && $(CARGO_TARGET_DIR)/debug/gitover --state gitover.state.yaml

# Run all unit and integration tests
test:
	cargo test

# Run all tests and print a per-file coverage summary.
# Fails if total line coverage of testable files drops below the threshold.
# ui.rs and main.rs are excluded: they require a live terminal (ratatui/crossterm)
# and cannot be unit-tested without a full terminal emulator harness.
# Requires: cargo install cargo-llvm-cov
#           rustup component add llvm-tools-preview
test-coverage:
	cargo llvm-cov \
		--ignore-filename-regex "(ui|main)\.rs" \
		--fail-under-lines 80

# Same as test-coverage but also prints uncovered line numbers per file.
# Useful for finding exactly which lines to target with new tests.
test-coverage-missing:
	cargo llvm-cov \
		--ignore-filename-regex "(ui|main)\.rs" \
		--show-missing-lines

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
