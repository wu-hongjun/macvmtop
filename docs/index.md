# macvmtop

`macvmtop` is a Rust command line monitor for macOS virtual machines. It is
inspired by `mactop`, but it focuses on the telemetry virtualized macOS reports
from inside the guest.

The project has three user-facing modes:

- `tui`: live terminal dashboard
- `once`: one text snapshot
- `json`: machine/system information, optionally with repeated sampled metrics
  frames

## What It Shows

`macvmtop` reports VM-visible data:

- guest identity and kernel details
- per-vCPU utilization
- guest load average
- memory and VM counters
- process CPU, memory, and thread counts
- guest network interface counters
- guest mounted volume usage
- guest uptime

The data comes from Darwin APIs available inside the VM. `macvmtop` does not
synthesize physical host metrics.

## Quick Example

Install:

```sh
curl -fsSL https://macvmtop.hongjunwu.com/install.sh | sh
```

Run one sample:

```sh
macvmtop once --interval 0.5 --processes 10
```

For machine-readable output:

```sh
macvmtop json --sample --interval 0.5 --processes 10
```

For repeated headless samples:

```sh
macvmtop json --sample --count 5 --interval 1 --processes 10
```

## Project Status

The current build is a working early Rust implementation. The TUI, one-shot
text output, and JSON output are implemented. GitHub Release archives are
published for Apple Silicon and Intel macOS guests, and the hosted installer
downloads and verifies those archives.
