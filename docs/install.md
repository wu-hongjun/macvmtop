# Installation

## Install Script

Install `macvmtop` with:

```sh
curl -fsSL https://macvmtop.hongjunwu.com/install.sh | sh
```

The script supports macOS guests on Apple Silicon and Intel Macs.

Install behavior:

- It first tries to install a prebuilt release archive from GitHub Releases.
- If no release archive exists for the current architecture, it falls back to
  `cargo install --git https://github.com/wu-hongjun/macvmtop.git --locked`.
- The source-install fallback requires Rust and Cargo.

Set `MACVMTOP_INSTALL_DIR` to choose the install directory for prebuilt release
archives:

```sh
curl -fsSL https://macvmtop.hongjunwu.com/install.sh | MACVMTOP_INSTALL_DIR=/usr/local/bin sh
```

Cargo fallback installs to Cargo's configured binary directory, usually:

```text
~/.cargo/bin
```

## Build From Source

```sh
git clone https://github.com/wu-hongjun/macvmtop.git
cd macvmtop
cargo build --release
```

The release binary will be written to:

```text
target/release/macvmtop
```

## Verify

```sh
macvmtop probe
macvmtop once --interval 0.5 --processes 10
```

## Update

Check for a newer release:

```sh
macvmtop check-update
```

Update to the latest release:

```sh
macvmtop update
```

This runs the same hosted installer:

```sh
curl -fsSL https://macvmtop.hongjunwu.com/install.sh | sh
```

If the custom-domain installer is temporarily unavailable, `macvmtop update`
falls back to the install script in the GitHub repository.
