.PHONY: test
test:
	cargo fmt -- --check
	cargo-sort --check --workspace
	cargo test --all-features
	cargo clippy -- -D warnings

.PHONY: format
format:
	cargo fmt
	cargo-sort --workspace

.PHONY: spec
spec:
	cargo run --bin calpol-spec
