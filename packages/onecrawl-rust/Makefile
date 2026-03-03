.PHONY: check test build-cli build-napi build-python build-all clean bench

# Default target
all: check test build-all

# Check & lint
check:
	cargo check --workspace
	cargo clippy --workspace -- -D warnings 2>/dev/null || true
	cargo fmt --all -- --check 2>/dev/null || true

# Run all Rust tests
test:
	cargo test --workspace

# Build CLI release binary
build-cli:
	cargo build --release --package onecrawl-cli-rs
	@echo "CLI binary: target/release/onecrawl-cli-rs"
	@ls -lh target/release/onecrawl-cli-rs 2>/dev/null || true

# Build NAPI Node.js addon
build-napi:
	cd bindings/napi && npm run build
	@echo "NAPI addon built"

# Build Python wheel
build-python:
	cd bindings/python && maturin build --release
	@echo "Python wheel built"

# Build everything
build-all: build-cli build-napi build-python

# Run benchmarks
bench:
	cargo run --release --package onecrawl-cli-rs -- bench run

# Clean
clean:
	cargo clean
	rm -rf bindings/napi/*.node
	rm -rf bindings/python/target

# Install CLI locally
install:
	cargo install --path crates/onecrawl-cli-rs

# Package for distribution
package: build-all
	mkdir -p dist
	cp target/release/onecrawl-cli-rs dist/ 2>/dev/null || true
	cp bindings/napi/*.node dist/ 2>/dev/null || true
	@echo "Artifacts in dist/"
