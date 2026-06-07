# macvmtop

`macvmtop` is a Rust CLI monitor for macOS virtual machines. It is inspired by
`mactop`, but focuses on the telemetry that virtualized macOS reports from
inside the guest.

The rule is simple: report values the macOS VM exposes and do not synthesize
physical host metrics.

## Current Status

The current build has three useful modes:

- `tui`: live mactop-style terminal UI implemented in Rust with `ratatui`
- `once`: one sampled text snapshot for quick terminal use
- `json`: machine/system info as JSON, optionally with repeated sampled metrics
  frames

The default command is `tui`.

Public documentation lives in `docs/`, is built with MkDocs, and is served from:

```text
https://macvmtop.hongjunwu.com/
```

Install with:

```sh
curl -fsSL https://macvmtop.hongjunwu.com/install.sh | sh
```

GitHub Release archives are published for Apple Silicon and Intel macOS. The
hosted installer verifies release checksums before extraction. Release binaries
are currently ad-hoc signed, not Developer ID signed or notarized.

## Reported Metrics

| Area | Source | Meaning |
| --- | --- | --- |
| VM identity | `sysctl` | Guest-reported model, kernel, CPU brand, CPU counts, memory size |
| vCPU usage | `host_processor_info(PROCESSOR_CPU_LOAD_INFO)` | Per assigned virtual CPU, sampled by counter deltas |
| Load average | `getloadavg` | Guest scheduler load |
| Memory and VM counters | `host_statistics64(HOST_VM_INFO64)`, `hw.memsize` | Guest memory pressure, pages, compressor, pageins/pageouts, swap counters |
| Processes | `proc_listallpids`, `proc_pidinfo`, `proc_pidpath` | Process CPU time deltas, RSS, virtual size, thread count, user, command |
| Network | `getifaddrs` / `if_data` | Guest interface byte and packet counters |
| Mounted volumes | `getmntinfo(MNT_NOWAIT)` | Guest mount table, filesystem type, capacity, block counters, and mount flags |
| Uptime | `clock_gettime(CLOCK_UPTIME_RAW)` | Guest uptime |

Protected processes may have limited path or task details when macOS denies
access. In that case `macvmtop` reports the fields it can read.

## Build

Install Rust if needed:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --default-toolchain stable --profile minimal
~/.cargo/bin/rustup component add rustfmt
```

Build and test:

```sh
~/.cargo/bin/cargo build
~/.cargo/bin/cargo test
```

Run the full local publish-readiness checks:

```sh
~/.cargo/bin/cargo fmt --check
~/.cargo/bin/cargo check
~/.cargo/bin/cargo test
~/.cargo/bin/cargo clippy --all-targets --all-features -- -D warnings
~/.cargo/bin/cargo package --list
~/.cargo/bin/cargo package
```

Push a version tag to build macOS release archives and create a GitHub Release:

```sh
git tag vX.Y.Z
git push origin vX.Y.Z
```

The project uses the published `foundations = 5.7.2` crate for service metadata
and logging primitives. The local `references/foundations` checkout is only for
research and comparison.

## Documentation

Install documentation dependencies and build the site:

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install -r docs/requirements.txt
mkdocs build --strict
```

Serve locally:

```sh
mkdocs serve
```

## Usage

Live TUI:

```sh
target/debug/macvmtop
target/debug/macvmtop tui
target/debug/macvmtop live
```

In TUI mode, use arrows or `j`/`k` to move through processes, `/` to search,
`Esc` to clear search, `Space` to pause or resume samples, and `q` or `Ctrl-C`
to quit.

One text snapshot:

```sh
target/debug/macvmtop once --interval 0.5 --processes 10
target/debug/macvmtop once --pid 123 --interval 0.5
```

One sampled JSON snapshot:

```sh
target/debug/macvmtop once --json
target/debug/macvmtop json --sample --interval 0.5 --processes 10
target/debug/macvmtop json --sample --pid 123 --interval 0.5
```

Repeated sampled JSON snapshots:

```sh
target/debug/macvmtop json --sample --count 5 --interval 1 --processes 10
target/debug/macvmtop json --sample --count 5 --compact
```

Machine/system info JSON, without waiting for a sample:

```sh
target/debug/macvmtop json
```

Probe VM-visible metrics:

```sh
target/debug/macvmtop probe
```

Check for and install updates:

```sh
target/debug/macvmtop check-update
target/debug/macvmtop update
```

## JSON Output

`macvmtop json` returns:

- `machine`: guest-reported model, CPU, kernel, CPU counts, memory, VM detection
- `available`: metrics `macvmtop` can collect and the API source for each

`macvmtop json --sample` returns:

- `machine`
- `samples`: timestamp, uptime, load, CPU, memory, network, storage, and process
  list for each sampled frame

Process CPU is macOS-style task CPU. A multi-threaded process can exceed `100%`
when it consumes more than one vCPU.

`--pid <PID>` restricts sampled process rows to one PID. Repeat it to watch
multiple PIDs.

## Reference Repositories

Local reference checkouts live under `references/` and are ignored by git:

- `references/mactop`: original project used for TUI and feature comparison
- `references/foundations`: Cloudflare Foundations source

To refresh them manually:

```sh
git clone https://github.com/metaspartan/mactop.git references/mactop
git clone https://github.com/cloudflare/foundations.git references/foundations
```

They are not required to build `macvmtop`.

`references/`, `target/`, `site/`, and `.venv/` are ignored locally. The crate
package also excludes `references/`, `target/`, and `site/`.

## License

MIT
