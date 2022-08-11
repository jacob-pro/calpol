.PHONY: test
test:
	cargo fmt -- --check
	cargo-sort --check --workspace
	cargo clippy --package calpol
	cargo clippy --package calpol-cli
	cargo test --package calpol
	cargo test --package calpol-cli

.PHONY: format
format:
	cargo fmt
	cargo-sort --workspace
