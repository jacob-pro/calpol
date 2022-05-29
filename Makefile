.PHONY: test
test:
	cargo fmt -- --check
	cargo-sort --check --workspace
	cargo clippy --all-features
	cargo clippy --package calpol-cli --all-features
	cargo test
	cargo test --package calpol-cli

.PHONY: format
format:
	cargo fmt
	cargo-sort --workspace
