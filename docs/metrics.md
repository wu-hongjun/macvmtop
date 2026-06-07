# Metrics

`macvmtop` reports values available from Darwin APIs inside macOS virtual
machines.

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

## CPU

CPU usage is sampled per assigned virtual CPU. The sampler reads cumulative CPU
ticks and computes percentages from deltas between frames.

Process CPU is macOS-style task CPU. A multi-threaded process can exceed `100%`
when it consumes more than one vCPU.

## Memory

Memory is guest memory. The sampler reports:

- total bytes
- used bytes
- available bytes
- free, active, inactive, and wired bytes
- compressed and compressor bytes
- pageins and pageouts
- swapins and swapouts

## Processes

Processes are sorted by CPU usage and then resident memory. Protected processes
may have limited path or task details when macOS denies access. In that case
`macvmtop` reports the fields it can read.

## Network

Network rates are computed from byte counter deltas for guest interfaces. The
sampler filters loopback interfaces.

## Mounted Volumes

Mounted volume usage comes from the guest mount table. Each volume reports mount
path, source, filesystem type, total/free/available bytes, block counters, file
counts, and read-only/local flags.
