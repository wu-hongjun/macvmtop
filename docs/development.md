# Development

## Prerequisites

- macOS VM for runtime testing
- Rust stable toolchain
- Python 3 for documentation builds

Install Rust if needed:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --default-toolchain stable --profile minimal
rustup component add rustfmt clippy
```

## Rust Checks

```sh
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Runtime smoke checks:

```sh
cargo run -- json
cargo run -- json --sample --interval 0.2 --processes 3
cargo run -- json --sample --count 2 --compact --interval 0.2 --processes 3
cargo run -- once --interval 0.2 --processes 5
cargo run -- once --interval 0.2 --pid $$
cargo run -- probe --interval 0.2 --processes 5
```

## Documentation Checks

Install the documentation dependencies:

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install -r docs/requirements.txt
```

Build the site:

```sh
mkdocs build --strict
```

Serve the site locally:

```sh
mkdocs serve
```

## Reference Repositories

Local research checkouts live under `references/`:

- `references/mactop`
- `references/foundations`

They are ignored by git and excluded from crate packaging. Keep them local.

Reference review takeaways to carry forward:

- Keep the startup path seeded before first TUI render so widgets do not flash
  blank values.
- Process-table ergonomics now include PID filtering, search, pause, and
  keyboard navigation. Keep future changes similarly bounded; sorting is the
  next process-table improvement to consider.
- Keep the headless JSON shape stable as more guest-visible metrics are added.
- Keep VM scope strict: add a metric only when the guest exposes a real value.

## Generated Files

These paths are generated locally and should not be committed:

- `target/`
- `site/`
- `.venv/`
- `references/`
