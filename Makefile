.PHONY: test
test:
	cargo fmt -- --check
	cargo-sort --check --workspace
	cargo clippy -- -D warnings
	cargo test --all-features

.PHONY: format
format:
	cargo fmt
	cargo-sort --workspace
