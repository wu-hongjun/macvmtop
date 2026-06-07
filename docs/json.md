# JSON Output

Use JSON mode for scripts and integrations. JSON mode reports only data exposed
inside the macOS guest.

## System Information

```sh
macvmtop json
```

This returns:

- `machine`: guest-reported name, model, CPU, kernel, CPU counts, memory, and
  VM detection
- `available`: metrics `macvmtop` can collect and the API source for each

`machine` fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `machine_name` | string | Guest machine name from `kern.hostname` |
| `model` | string | Guest-reported Mac model |
| `cpu_brand` | string | Guest-reported CPU brand |
| `kernel_version` | string | Full Darwin kernel version |
| `os_release` | string | Darwin release |
| `logical_cpus` | number | Guest logical CPU count |
| `physical_cpus` | number | Guest physical CPU count |
| `perf_levels` | number or null | Reported performance levels when present |
| `total_memory_bytes` | number | Guest memory size |
| `vm_guest` | boolean | VM detection based on guest-reported strings |

## Sampled Metrics

```sh
macvmtop json --sample --interval 0.5 --processes 10
macvmtop json --sample --pid 123 --interval 0.5
```

This returns:

- `machine`
- `samples`: sampled frames with timestamp, uptime, load, CPU, memory, network,
  storage, and process list

The `samples` field is always an array in `json --sample` output, including
when only one sample is requested. Use `--count` for repeated headless samples:

```sh
macvmtop json --sample --count 5 --interval 1 --processes 10
```

Each `samples` item has these fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `timestamp_unix_ms` | number | Wall-clock sample time |
| `elapsed_ms` | number | Time between sampler frames |
| `uptime_seconds` | number | Guest uptime |
| `load_average` | number array | 1, 5, and 15 minute load averages |
| `cpu` | object | Aggregate and per-vCPU utilization |
| `memory` | object | Guest memory and VM counters |
| `network` | array | Guest interface counters and rates |
| `storage` | object | Guest mounted volume usage under `volumes` |
| `processes` | array | Top processes after sorting and limit |

Use `--pid <PID>` to restrict sampled process rows to one PID. Repeat the flag
to include multiple PIDs.

Each `storage.volumes` item is derived from the guest mount table and includes
mount path, source, filesystem type, capacity, free and available bytes, block
counts, file counts, and read-only/local flags.

By default, JSON output is pretty-printed. Use `--compact` for single-line JSON
or `--pretty` to request the default explicitly:

```sh
macvmtop json --sample --count 3 --compact
macvmtop json --pretty
```

`--count` requires `--sample`; plain `macvmtop json` returns one system
information object.

Exact values depend on the guest VM and sampling interval.
