# Repository Guidelines

## Project Structure & Module Organization

- Workspace: Rust (nightly), members: `simkit-core` and `stitchlands`.
- Source: `simkit-core/src` (core sim + a demo in `src/bin/main.rs`), `stitchlands/src` (app/CLI, plugins, scenarios).
- Tests: crate-level unit tests inline; integration tests in `stitchlands/tests` (e.g., `scenario_invariants.rs`, data in `stitchlands/tests/data`).
- Assets/docs: `assets/` for runtime assets, `stitchlands/docs/` for design/architecture notes.

## Build, Test, and Development Commands

- Format: `cargo fmt --all` — applies repo `rustfmt.toml` rules.
- Lint: `cargo clippy --all-targets --all-features` — CI treats warnings as errors.
- Build: `cargo build --workspace` — builds all crates.
- Run (headless): `cargo run -p stitchlands -- --mode headless --ticks 100 --scenario stitchlands/tests/data/small.toml`.

## Coding Style & Naming Conventions

- Rust edition 2024, nightly toolchain (`rust-toolchain.toml`).
- Formatting enforced by `rustfmt.toml` (80 cols, grouped/reordered imports, wrapped comments).
- Lint with Clippy; fix or `#[allow]` with justification.
- Naming: modules/files `snake_case`; types/traits `UpperCamelCase`; functions/vars `snake_case`; constants `SCREAMING_SNAKE_CASE`.

## Testing Guidelines

- Framework: standard Rust with tracing logging support `#[test_log::test]` and integration tests in `crate/tests`.
- Prefer integration tests for cross-system flows (see `stitchlands/tests/scenario_invariants.rs`).
- Keep test data under `crate/tests/data` and use deterministic RNG seeds.
- Run: `cargo test -p stitchlands -q` or `cargo test -q` if changes made to simkit. No hard coverage threshold; aim for meaningful invariants and state assertions.

## Commit & Pull Request Guidelines

- Use concise, conventional prefixes: `feat:`, `fix:`, `tests:`, `chore:`, `refactor:`, `docs:` (e.g., `tests: add scenario assertions`).
- CI must pass: fmt, clippy, build, and tests (`.github/workflows`).


After each feature, run clippy, fmt and commit 