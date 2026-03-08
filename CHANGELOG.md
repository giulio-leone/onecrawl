# Changelog

All notable changes to OneCrawl will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [4.0.0-alpha.2] - 2025-07-22

### Performance

- **Architectural channel optimization**: Converted PageInner→Target channel from bounded `mpsc::channel(1)` to unbounded, eliminating `sender.clone()` (Arc increment) on every CDP command. CommandFuture and TargetMessageFuture now send eagerly in constructors, removing stored sender/message fields and shrinking future struct sizes.
- **FnvHashMap migration**: Replaced 6 internal HashMap instances with FnvHashMap for faster hashing on small string keys (targets, sessions, frames, context_ids, requests, listeners).
- **Move semantics in CDP event handling**: Eliminated redundant clones in 4 CDP event types by consuming events by value instead of cloning fields.
- **Zero-waste FlushState**: FlushState::Pending no longer stores MethodCall, reducing memory per pending command.
- **CommandFuture optimization**: Uses `std::mem::take` to move method instead of cloning, avoiding String allocation on every command response.
- **Network path optimizations**: Serialize extra_headers by reference (avoiding HashMap clone), reduced triple `request_id.clone()` to single clone per network request.
- **EventListeners polling**: Uses `retain_mut` instead of swap_remove+push loop for cleaner and faster listener eviction.
- **Inline hints on 10 hot-path functions**: Added `#[inline]` to dispatch loop functions called per CDP message (submit_command, on_event, on_response, etc.).

### Fixed

- **E2E navigate timeout**: `goto()` no longer hangs on `about:blank` and `data:` URLs (special URL detection bypasses polling loop).
- **CLI version assertion**: E2E tests use semver regex instead of hardcoded version string.

## [4.0.0-alpha.1] - 2026-03-08

### Breaking Changes

- Internalized chromiumoxide as first-party crates under OneCrawl namespace
- `chromiumoxide::Page` → `onecrawl_browser::Page`
- `chromiumoxide::Browser` → `onecrawl_browser::Browser`
- All CDP types now under `onecrawl_protocol::*`

### Architecture

- **M1 — Internalization**: Vendored chromiumoxide as four first-party crates (`onecrawl-browser`, `onecrawl-protocol`, `onecrawl-protocol-gen`, `onecrawl-browser-types`), eliminating external dependency (coupling 9/10 → 0/10)
- **M2 — Hexagonal port traits**: Introduced 6 async port traits (`BrowserPort`, `PagePort`, `ElementPort`, `NetworkPort`, `EmulationPort`, `InputPort`) with `create_browser()` / `connect_browser()` factory functions for dependency injection
- **M3 — KISS/DRY/SOLID refactor**: Split monolithic `Page` (3,700 LOC) and `Browser` modules into focused sub-modules; extracted shared utilities
- **M4 — Port trait re-exports**: Re-exported all port traits and factory functions from `onecrawl-cdp` public API for ergonomic consumer access
- **M5 — Validation & release prep**: Full test suite validation (364 pass, 1 pre-existing), release build benchmarking (~53s), documentation updates, version bump to 4.0.0-alpha.1

## [3.9.2] - 2025-03-07

### Security

- 28 security hardening fixes including webhook HMAC validation, atomic file writes, ZeroizeOnDrop for sensitive data, input sanitization, and cryptographic best practices.

### Added

- 87 E2E tests covering 9 new features for comprehensive regression coverage.
- NAPI bindings for 8 features (+648 lines) — Durable Sessions, Event Reactor, AI Agent Auto, Multi-Device Orchestration, Encrypted Vault, Webhook & Event Bus, Plugin System, Streaming AI Vision.
- PyO3 bindings for 8 features (+486 lines) — matching NAPI feature parity for Python consumers.
- Real-world testing report covering 2,332 features across all modules.

### Fixed

- `agent_auto` planner `about:blank` default bug — planner no longer falls back to blank page when no URL is provided.

### Changed

- DRY optimization: extracted shared `util.rs` module to eliminate cross-crate duplication.
- KISS optimization: simplified 12 files, removing unnecessary abstractions (-53 lines net).
- SOLID optimization: Single Responsibility Principle refactors in `agent_auto`, `orchestrator`, and `reactor` modules.

## [3.9.0] - 2025-03-06

### Added

- **Durable Sessions** — Auto-checkpoint and crash recovery for long-running crawl sessions. Sessions persist state to disk and resume transparently after unexpected termination.
- **Event Reactor** — Persistent observer pattern for reacting to page events (navigation, DOM mutations, network activity) with user-defined handlers.
- **AI Agent Auto** — Goal-based planning with autonomous multi-step browser automation. Accepts natural language objectives and decomposes them into executable action sequences.
- **Multi-Device Orchestration** — Coordinate crawling across desktop, Android, and iOS devices from a single control plane with synchronized state.
- **Encrypted Vault** — Secure credential storage with PBKDF2 key derivation for managing authentication tokens, cookies, and secrets at rest.
- **Webhook & Event Bus** — HMAC-signed webhook delivery and internal event bus for integrating OneCrawl with external systems and CI/CD pipelines.
- **Plugin System** — Extensible plugin architecture with JSON manifests for loading custom extractors, transformers, and reporters.
- **Streaming AI Vision** — Continuous page understanding via streaming visual analysis, enabling real-time interpretation of page content and layout changes.
- **Visual Workflow Builder (Studio)** — Drag-and-drop UI for composing crawl workflows visually, with live preview and export to CLI-compatible configurations.

[4.0.0-alpha.1]: https://github.com/giulio-leone/onecrawl/compare/v3.9.2...v4.0.0-alpha.1
[3.9.2]: https://github.com/giulio-leone/onecrawl/compare/v3.9.0...v3.9.2
[3.9.0]: https://github.com/giulio-leone/onecrawl/releases/tag/v3.9.0
