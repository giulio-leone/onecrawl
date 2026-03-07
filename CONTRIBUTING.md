# Contributing to OneCrawl

Thank you for your interest in contributing to OneCrawl! This guide will help you get started.

## Table of Contents

- [Development Environment](#development-environment)
- [Building](#building)
- [Testing](#testing)
- [Code Style](#code-style)
- [Architecture](#architecture)
- [Where to Add New Features](#where-to-add-new-features)
- [Pull Request Process](#pull-request-process)
- [Commit Convention](#commit-convention)

## Development Environment

### Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| **Rust** | stable (latest) | Core engine |
| **Node.js** | 18+ | NAPI bindings |
| **Python** | 3.9+ | PyO3 bindings |
| **pnpm** | 8+ | Workspace management |

### Setup

```bash
# Clone the repository
git clone https://github.com/giulio-leone/onecrawl.git
cd onecrawl

# Install Rust toolchain (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js dependencies
pnpm install

# Verify the setup
cd packages/onecrawl-rust
cargo build --workspace
```

## Building

```bash
cd packages/onecrawl-rust
cargo build --workspace
```

For release builds:

```bash
cargo build --workspace --release
```

## Testing

OneCrawl has two test suites:

### Unit Tests (362 tests)

```bash
cargo test --workspace --exclude onecrawl-e2e
```

### End-to-End Tests (188 tests)

```bash
cargo test -p onecrawl-e2e
```

### Run All Tests

```bash
cargo test --workspace
```

## Code Style

- Follow standard **Rust idioms** and conventions.
- Code must compile with **zero clippy warnings**:
  ```bash
  cargo clippy --workspace -- -D warnings
  ```
- Use `cargo fmt` before committing:
  ```bash
  cargo fmt --all
  ```

## Architecture

For a detailed overview of OneCrawl's architecture, see [`docs/docs/architecture.md`](docs/docs/architecture.md).

OneCrawl is organized as a Cargo workspace under `packages/onecrawl-rust/` with the following key crates:

| Crate | Purpose |
|-------|---------|
| `onecrawl` | CLI entry point and command routing |
| `onecrawl-core` | Core crawling engine |
| `onecrawl-cdp` | Chrome DevTools Protocol integration |
| `onecrawl-mcp` | Model Context Protocol server |
| `onecrawl-napi` | Node.js bindings via NAPI |
| `onecrawl-pyo3` | Python bindings via PyO3 |
| `onecrawl-e2e` | End-to-end test suite |

## Where to Add New Features

| Feature Area | Directory | Notes |
|-------------|-----------|-------|
| CLI commands | `cli/` | Add new subcommands here |
| MCP tools | `handlers/` | New MCP tool handlers |
| CDP actions | `cdp/src/` | Chrome DevTools Protocol actions |
| Core engine | `core/src/` | Crawling and extraction logic |
| NAPI bindings | `napi/` | Node.js API surface |
| PyO3 bindings | `pyo3/` | Python API surface |

## Pull Request Process

1. **Fork** the repository and create your branch from `main`.
2. **Branch naming**: Use descriptive names like `feat/streaming-extraction` or `fix/session-recovery`.
3. **Implement** your changes following the code style guidelines.
4. **Test** your changes — all existing tests must pass, and new features must include tests.
5. **Submit** a pull request with a clear description of your changes.

### PR Requirements

- [ ] All tests pass (`cargo test --workspace`)
- [ ] No clippy warnings (`cargo clippy --workspace -- -D warnings`)
- [ ] Code is formatted (`cargo fmt --all --check`)
- [ ] New features include tests
- [ ] Documentation is updated if applicable

## Commit Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

| Prefix | Purpose |
|--------|---------|
| `feat:` | New feature |
| `fix:` | Bug fix |
| `docs:` | Documentation changes |
| `test:` | Adding or updating tests |
| `refactor:` | Code refactoring (no behavior change) |
| `perf:` | Performance improvement |
| `chore:` | Maintenance tasks |
| `ci:` | CI/CD changes |

**Examples:**

```
feat: add streaming extraction for large pages
fix: resolve session recovery crash on corrupted checkpoints
docs: update architecture diagram with event reactor
test: add E2E tests for webhook validation
```

## Questions?

If you have questions about contributing, please [open an issue](https://github.com/giulio-leone/onecrawl/issues/new) and we'll be happy to help.
