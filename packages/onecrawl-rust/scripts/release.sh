#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    echo "Usage: ./scripts/release.sh <version>"
    echo "Example: ./scripts/release.sh 0.1.0"
    exit 1
fi

echo "=== OneCrawl Rust Release v${VERSION} ==="

# 1. Check
echo ">> Running checks..."
cargo check --workspace
cargo test --workspace

# 2. Update version in Cargo.toml files
echo ">> Updating versions to ${VERSION}..."
for toml in crates/*/Cargo.toml bindings/*/Cargo.toml; do
    if [ -f "$toml" ]; then
        sed -i.bak "s/^version = \".*\"/version = \"${VERSION}\"/" "$toml"
        rm -f "${toml}.bak"
    fi
done

# 3. Build release artifacts
echo ">> Building CLI..."
cargo build --release --package onecrawl-cli-rs

echo ">> Building NAPI..."
cd bindings/napi && npm run build && cd ../..

echo ">> Building Python wheel..."
cd bindings/python && maturin build --release && cd ../..

# 4. Package
mkdir -p dist
cp target/release/onecrawl-cli-rs dist/onecrawl-cli-rs 2>/dev/null || true
cp bindings/napi/*.node dist/ 2>/dev/null || true
cp bindings/python/target/wheels/*.whl dist/ 2>/dev/null || true

echo "=== Release v${VERSION} artifacts in dist/ ==="
ls -la dist/
