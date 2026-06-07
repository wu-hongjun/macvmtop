# Repository Guidelines

## Project Structure & Module Organization

`macvmtop` is a Rust CLI for macOS VM telemetry. The root `Cargo.toml` defines the crate and dependencies. Runtime code lives in `src/`: `main.rs` owns CLI commands, update flow, and inline unit tests; `sampler.rs` collects and converts metrics; `render.rs` handles TUI and text output; `model.rs` defines JSON-serializable data types; `darwin/` contains macOS native C/FFI bindings. Public documentation and the hosted installer live in `docs/`, with MkDocs configuration in `mkdocs.yml`. GitHub Actions workflows are in `.github/workflows/`. Keep generated or local-only paths out of commits: `target/`, `site/`, `.venv/`, and `references/`.

## Build, Test, and Development Commands

- `cargo build`: compile a debug binary at `target/debug/macvmtop`.
- `cargo run -- once --interval 0.2 --processes 5`: run a quick text snapshot.
- `cargo run -- json --sample --count 2 --compact`: smoke-test sampled JSON output.
- `cargo fmt --check`: verify Rust formatting.
- `cargo check`: run a fast compiler check.
- `cargo test`: run unit tests.
- `cargo clippy --all-targets --all-features -- -D warnings`: enforce lint cleanliness.
- `mkdocs build --strict`: validate documentation output after installing `docs/requirements.txt`.

Runtime checks require macOS because the sampler uses Darwin APIs.

## Coding Style & Naming Conventions

Use Rust 2024 idioms and `rustfmt` defaults. Prefer `snake_case` for functions, variables, modules, and test names; `PascalCase` for types; and `SCREAMING_SNAKE_CASE` for constants. Keep modules focused on their existing responsibilities. When adding metrics, report values exposed by the macOS guest only; do not synthesize physical host metrics. Treat JSON field names as part of the public interface and document intentional changes.

## Testing Guidelines

Current tests are inline unit tests under `#[cfg(test)]` in `src/main.rs`. Add focused unit tests near pure helpers, and use behavior names such as `compares_semver_like_versions`. If CLI behavior grows, add integration tests under `tests/`. For sampler, TUI, or JSON changes, run `cargo test` plus at least one macOS VM smoke command from the development docs.

## Commit & Pull Request Guidelines

Recent commits use concise imperative subjects, for example `Improve installer PATH guidance` and `Harden update command`. Follow that style and keep subjects specific. Pull requests should include a short summary, the commands run, any documentation or JSON compatibility impact, and linked issues when applicable. Include screenshots or terminal output for visible TUI, installer, or documentation UX changes. For releases, ensure the pushed tag matches `Cargo.toml` as `vX.Y.Z`.
