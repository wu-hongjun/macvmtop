# Getting Started

## Install

```sh
curl -fsSL https://macvmtop.hongjunwu.com/install.sh | sh
```

The install script prefers prebuilt GitHub Release archives. Until release
archives are published, it falls back to `cargo install --git` and requires a
working Rust toolchain.

## Build From Source

Install Rust, then build the binary:

```sh
cargo build --release
```

The release binary will be written to:

```sh
target/release/macvmtop
```

During development, use the debug binary:

```sh
cargo run -- once
```

## Commands

Run the live terminal UI:

```sh
macvmtop
macvmtop tui
macvmtop live
```

Print one text snapshot:

```sh
macvmtop once --interval 0.5 --processes 10
macvmtop once --pid 123 --interval 0.5
```

Print system information as JSON:

```sh
macvmtop json
```

Print a sampled JSON frame:

```sh
macvmtop json --sample --interval 0.5 --processes 10
macvmtop json --sample --pid 123 --interval 0.5
```

Print repeated sampled JSON frames:

```sh
macvmtop json --sample --count 5 --interval 1 --processes 10
```

Print compact JSON:

```sh
macvmtop json --sample --count 5 --compact
```

Probe readable VM metrics:

```sh
macvmtop probe
```

## TUI Controls

In TUI mode:

- `q` exits
- `Ctrl-C` exits
- arrow keys or `j`/`k` move the process selection
- `/` starts process search
- `Enter` or `Esc` exits search entry
- `Esc` clears an inactive search
- `Space` pauses or resumes sampling

## Global Options

```text
--interval <seconds>     Seconds between samples
--processes <count>      Number of processes to display
--pid <PID>              Restrict sampled processes to one PID; repeatable
--json                   Emit JSON for the once command
--verbose                Enable debug logs on stderr
```

## JSON Options

```text
--sample                 Include sampled metrics
--count <count>          Number of samples to collect with --sample
--pretty                 Print human-readable JSON
--compact                Print compact single-line JSON
```
