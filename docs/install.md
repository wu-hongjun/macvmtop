# Installation

## Install Script

Install `macvmtop` with:

```sh
curl -fsSL https://macvmtop.hongjunwu.com/install.sh | sh
```

The script supports macOS guests on Apple Silicon and Intel Macs.

Install behavior:

- It first tries to install a prebuilt release archive from GitHub Releases.
- It verifies the archive against the release `SHA256SUMS` file before
  extracting it.
- If no release archive exists for the current architecture, it falls back to
  `cargo install --git https://github.com/wu-hongjun/macvmtop.git --locked`.
- The source-install fallback requires Rust and Cargo.
- After installing, it checks whether the install directory is on `PATH`. If it
  is not, the installer prints the shell profile line to add and, when running
  interactively, offers to append it for you.

Set `MACVMTOP_INSTALL_DIR` to choose the install directory for prebuilt release
archives:

```sh
curl -fsSL https://macvmtop.hongjunwu.com/install.sh | MACVMTOP_INSTALL_DIR=/usr/local/bin sh
```

Cargo fallback installs to Cargo's configured binary directory, usually:

```text
~/.cargo/bin
```

Set `MACVMTOP_NO_PATH_PROMPT=1` to make the installer print PATH guidance
without prompting.

!!! note "Signing Status"

    Release binaries are currently ad-hoc signed, not Developer ID signed or
    notarized. Command-line installs through the hosted installer work, but
    manual browser downloads may still encounter Gatekeeper friction. Developer
    ID signing and notarization are tracked as future distribution work.

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
