use crate::darwin::{self, CpuTicks, NetIface, ProcessCommand, ProcessInfo, VmStats, VolumeInfo};
use crate::model::{
    AvailableMetric, CpuCoreSample, CpuSample, MachineInfo, MemorySample, NetworkSample,
    ProcessSample, StorageSample, SystemInfoReport, SystemSample, VolumeSample,
};
use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Default)]
pub struct Sampler {
    previous_cpu: Option<Vec<CpuTicks>>,
    previous_network: HashMap<String, NetIface>,
    previous_process_cpu: HashMap<i32, u64>,
    username_cache: HashMap<u32, String>,
    previous_instant: Option<Instant>,
}

#[derive(Clone, Debug, Default)]
pub struct ProcessFilter {
    pids: Vec<i32>,
}

impl ProcessFilter {
    pub fn from_pids(mut pids: Vec<i32>) -> Self {
        pids.retain(|pid| *pid > 0);
        pids.sort_unstable();
        pids.dedup();
        Self { pids }
    }

    fn matches_pid(&self, pid: i32) -> bool {
        self.pids.is_empty() || self.pids.binary_search(&pid).is_ok()
    }
}

impl Sampler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sample(
        &mut self,
        process_limit: usize,
        process_filter: &ProcessFilter,
    ) -> Result<SystemSample> {
        let now = Instant::now();
        let elapsed = self
            .previous_instant
            .map(|previous| now.saturating_duration_since(previous))
            .unwrap_or_else(|| Duration::from_millis(0));
        let elapsed_secs = elapsed.as_secs_f64();

        let raw_cpu = darwin::cpu_load()?;
        let vm = darwin::vm_stats()?;
        let raw_network = darwin::network_interfaces()?;
        let raw_volumes = darwin::storage_volumes()?;
        let raw_processes = darwin::processes()?;

        let cpu = self.cpu_sample(&raw_cpu);
        let memory = memory_sample(vm);
        let network = self.network_sample(raw_network, elapsed_secs);
        let storage = storage_sample(raw_volumes);
        let mut processes = self.process_sample(
            raw_processes,
            elapsed_secs,
            memory.total_bytes,
            process_filter,
        );
        select_top_processes(&mut processes, process_limit);
        let processes = processes
            .into_iter()
            .map(|process| {
                let user = self
                    .username_cache
                    .entry(process.uid)
                    .or_insert_with(|| darwin::username(process.uid))
                    .clone();

                ProcessSample {
                    pid: process.pid,
                    user,
                    command: process.command_text(),
                    cpu_percent: process.cpu_percent,
                    memory_percent: process.memory_percent,
                    resident_bytes: process.resident_bytes,
                    virtual_bytes: process.virtual_bytes,
                    threads: process.threads,
                }
            })
            .collect();

        self.previous_cpu = Some(raw_cpu);
        self.previous_instant = Some(now);

        Ok(SystemSample {
            timestamp_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            elapsed_ms: elapsed.as_millis(),
            uptime_seconds: darwin::uptime_seconds().unwrap_or_default(),
            load_average: darwin::load_average(),
            cpu,
            memory,
            network,
            storage,
            processes,
        })
    }

    fn cpu_sample(&self, raw: &[CpuTicks]) -> CpuSample {
        let cores: Vec<_> = raw
            .iter()
            .enumerate()
            .map(|(idx, current)| {
                let percent = self
                    .previous_cpu
                    .as_ref()
                    .and_then(|previous| previous.get(idx))
                    .map(|previous| cpu_percent_delta(*previous, *current))
                    .unwrap_or_else(|| cpu_percent_absolute(*current));

                CpuCoreSample { id: idx, percent }
            })
            .collect();

        let aggregate_percent = if cores.is_empty() {
            0.0
        } else {
            cores.iter().map(|core| core.percent).sum::<f64>() / cores.len() as f64
        };

        CpuSample {
            aggregate_percent,
            cores,
        }
    }

    fn network_sample(&mut self, raw: Vec<NetIface>, elapsed_secs: f64) -> Vec<NetworkSample> {
        let mut next = HashMap::new();
        let mut samples = Vec::with_capacity(raw.len());

        for iface in raw {
            let previous = self.previous_network.get(&iface.name);
            let rx_bytes_per_sec = rate(previous.map(|p| p.rx_bytes), iface.rx_bytes, elapsed_secs);
            let tx_bytes_per_sec = rate(previous.map(|p| p.tx_bytes), iface.tx_bytes, elapsed_secs);

            samples.push(NetworkSample {
                name: iface.name.clone(),
                rx_bytes: iface.rx_bytes,
                tx_bytes: iface.tx_bytes,
                rx_bytes_per_sec,
                tx_bytes_per_sec,
                rx_packets: iface.rx_packets,
                tx_packets: iface.tx_packets,
            });
            next.insert(iface.name.clone(), iface);
        }

        self.previous_network = next;
        samples
    }

    fn process_sample(
        &mut self,
        raw: Vec<ProcessInfo>,
        elapsed_secs: f64,
        total_memory_bytes: u64,
        process_filter: &ProcessFilter,
    ) -> Vec<RawProcessSample> {
        let mut next = HashMap::new();
        let mut samples = Vec::with_capacity(raw.len());

        for process in raw {
            if !process_filter.matches_pid(process.pid) {
                continue;
            }

            let cpu_percent = if elapsed_secs > 0.0 {
                self.previous_process_cpu
                    .get(&process.pid)
                    .and_then(|previous_cpu_ns| {
                        process
                            .cpu_time_ns
                            .checked_sub(*previous_cpu_ns)
                            .map(|delta| (delta as f64 / 1_000_000_000.0) / elapsed_secs * 100.0)
                    })
                    .unwrap_or(0.0)
            } else {
                0.0
            };

            let memory_percent = if total_memory_bytes > 0 {
                process.resident_bytes as f64 / total_memory_bytes as f64 * 100.0
            } else {
                0.0
            };

            next.insert(process.pid, process.cpu_time_ns);
            samples.push(RawProcessSample {
                pid: process.pid,
                uid: process.uid,
                command: process.command,
                cpu_percent,
                memory_percent,
                resident_bytes: process.resident_bytes,
                virtual_bytes: process.virtual_bytes,
                threads: process.threads,
            });
        }

        self.previous_process_cpu = next;
        samples
    }
}

#[derive(Clone, Debug)]
struct RawProcessSample {
    pid: i32,
    uid: u32,
    command: ProcessCommand,
    cpu_percent: f64,
    memory_percent: f64,
    resident_bytes: u64,
    virtual_bytes: u64,
    threads: u64,
}

impl RawProcessSample {
    fn command_text(&self) -> String {
        let command = self.command.text();
        if command.is_empty() {
            format!("[{}]", self.pid)
        } else {
            command
        }
    }
}

fn select_top_processes(processes: &mut Vec<RawProcessSample>, process_limit: usize) {
    if process_limit == 0 {
        processes.clear();
        return;
    }

    if processes.len() > process_limit {
        processes.select_nth_unstable_by(process_limit, compare_processes);
        processes.truncate(process_limit);
    }

    processes.sort_by(compare_processes);
}

fn compare_processes(a: &RawProcessSample, b: &RawProcessSample) -> std::cmp::Ordering {
    b.cpu_percent
        .total_cmp(&a.cpu_percent)
        .then_with(|| b.resident_bytes.cmp(&a.resident_bytes))
}

pub fn machine_info() -> MachineInfo {
    let machine_name = darwin::sysctl_string("kern.hostname")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let model = darwin::sysctl_string("hw.model").unwrap_or_else(|_| "unknown".to_string());
    let cpu_brand =
        darwin::sysctl_string("machdep.cpu.brand_string").unwrap_or_else(|_| "unknown".to_string());
    let kernel_version =
        darwin::sysctl_string("kern.version").unwrap_or_else(|_| "unknown".to_string());
    let os_release =
        darwin::sysctl_string("kern.osrelease").unwrap_or_else(|_| "unknown".to_string());
    let logical_cpus = darwin::sysctl_i32("hw.logicalcpu")
        .unwrap_or_default()
        .max(0) as u32;
    let physical_cpus = darwin::sysctl_i32("hw.physicalcpu")
        .unwrap_or_default()
        .max(0) as u32;
    let perf_levels = darwin::sysctl_i32("hw.nperflevels")
        .ok()
        .map(|value| value.max(0) as u32);
    let total_memory_bytes = darwin::sysctl_u64("hw.memsize").unwrap_or_default();
    let vm_guest = model.starts_with("VirtualMac")
        || cpu_brand.contains("(Virtual)")
        || kernel_version.contains("VMAPPLE");

    MachineInfo {
        machine_name,
        model,
        cpu_brand,
        kernel_version,
        os_release,
        logical_cpus,
        physical_cpus,
        perf_levels,
        total_memory_bytes,
        vm_guest,
    }
}

pub fn system_info_report() -> SystemInfoReport {
    SystemInfoReport {
        machine: machine_info(),
        available: available_metrics(),
    }
}

pub fn available_metrics() -> Vec<AvailableMetric> {
    vec![
        AvailableMetric {
            metric: "VM identity",
            source: "sysctl kern.hostname, kern.version, kern.osrelease, hw.model, machdep.cpu.brand_string",
            note: "guest-visible identity; VM detection is based on reported model/kernel strings",
        },
        AvailableMetric {
            metric: "vCPU utilization",
            source: "host_processor_info(PROCESSOR_CPU_LOAD_INFO)",
            note: "per assigned virtual CPU, sampled by counter deltas",
        },
        AvailableMetric {
            metric: "load average",
            source: "getloadavg",
            note: "guest OS scheduler load average",
        },
        AvailableMetric {
            metric: "memory and VM counters",
            source: "host_statistics64(HOST_VM_INFO64), sysctl hw.memsize",
            note: "guest memory only; page size is read from the kernel",
        },
        AvailableMetric {
            metric: "process CPU and memory",
            source: "proc_listallpids, proc_pidinfo(PROC_PIDTASKALLINFO), proc_pidpath",
            note: "protected processes may omit details if macOS denies access",
        },
        AvailableMetric {
            metric: "network counters",
            source: "getifaddrs AF_LINK if_data",
            note: "guest interface byte and packet counters",
        },
        AvailableMetric {
            metric: "mounted volume usage",
            source: "getmntinfo(MNT_NOWAIT)",
            note: "guest mount table, filesystem type, capacity, block counters, and mount flags",
        },
        AvailableMetric {
            metric: "uptime",
            source: "clock_gettime(CLOCK_UPTIME_RAW)",
            note: "guest uptime",
        },
    ]
}

fn memory_sample(vm: VmStats) -> MemorySample {
    let page_size = vm.page_size;
    let free_bytes = vm.free_pages.saturating_mul(page_size);
    let active_bytes = vm.active_pages.saturating_mul(page_size);
    let inactive_bytes = vm.inactive_pages.saturating_mul(page_size);
    let speculative_bytes = vm.speculative_pages.saturating_mul(page_size);
    let wired_bytes = vm.wired_pages.saturating_mul(page_size);
    let compressed_bytes = vm.compressed_pages.saturating_mul(page_size);
    let compressor_bytes = vm.compressor_pages.saturating_mul(page_size);
    let available_bytes = free_bytes
        .saturating_add(inactive_bytes)
        .saturating_add(speculative_bytes);
    let used_bytes = vm.total_bytes.saturating_sub(available_bytes);
    let pressure_percent = if vm.total_bytes > 0 {
        used_bytes as f64 / vm.total_bytes as f64 * 100.0
    } else {
        0.0
    };

    MemorySample {
        total_bytes: vm.total_bytes,
        used_bytes,
        available_bytes,
        free_bytes,
        active_bytes,
        inactive_bytes,
        wired_bytes,
        compressed_bytes,
        compressor_bytes,
        pressure_percent,
        pageins: vm.pageins,
        pageouts: vm.pageouts,
        swapins: vm.swapins,
        swapouts: vm.swapouts,
    }
}

fn storage_sample(raw: Vec<VolumeInfo>) -> StorageSample {
    let mut volumes = raw
        .into_iter()
        .map(|volume| {
            let total_bytes = volume.total_blocks.saturating_mul(volume.block_size);
            let free_bytes = volume.free_blocks.saturating_mul(volume.block_size);
            let available_bytes = volume.available_blocks.saturating_mul(volume.block_size);
            let used_bytes = total_bytes.saturating_sub(free_bytes);
            let used_percent = if total_bytes > 0 {
                used_bytes as f64 / total_bytes as f64 * 100.0
            } else {
                0.0
            };

            VolumeSample {
                mount_path: volume.mount_path,
                mounted_from: volume.mounted_from,
                fs_type: volume.fs_type,
                total_bytes,
                used_bytes,
                available_bytes,
                free_bytes,
                used_percent,
                block_size: volume.block_size,
                total_blocks: volume.total_blocks,
                free_blocks: volume.free_blocks,
                available_blocks: volume.available_blocks,
                file_count: volume.file_count,
                free_file_count: volume.free_file_count,
                read_only: volume.read_only,
                local: volume.local,
            }
        })
        .collect::<Vec<_>>();

    volumes.sort_by(|a, b| {
        mount_sort_key(a)
            .cmp(&mount_sort_key(b))
            .then_with(|| a.mount_path.cmp(&b.mount_path))
    });

    StorageSample { volumes }
}

fn mount_sort_key(volume: &VolumeSample) -> u8 {
    if volume.mount_path == "/" {
        0
    } else if volume.total_bytes > 0 && volume.local {
        1
    } else if volume.total_bytes > 0 {
        2
    } else {
        3
    }
}

fn cpu_percent_delta(previous: CpuTicks, current: CpuTicks) -> f64 {
    let previous_total = cpu_total(previous);
    let current_total = cpu_total(current);
    let total_delta = current_total.saturating_sub(previous_total);
    let idle_delta = current.idle.saturating_sub(previous.idle);

    if total_delta == 0 {
        0.0
    } else {
        (total_delta.saturating_sub(idle_delta)) as f64 / total_delta as f64 * 100.0
    }
}

fn cpu_percent_absolute(ticks: CpuTicks) -> f64 {
    let total = cpu_total(ticks);
    if total == 0 {
        0.0
    } else {
        ticks
            .user
            .saturating_add(ticks.system)
            .saturating_add(ticks.nice) as f64
            / total as f64
            * 100.0
    }
}

fn cpu_total(ticks: CpuTicks) -> u64 {
    ticks
        .user
        .saturating_add(ticks.system)
        .saturating_add(ticks.idle)
        .saturating_add(ticks.nice)
}

fn rate(previous: Option<u64>, current: u64, elapsed_secs: f64) -> f64 {
    if elapsed_secs <= 0.0 {
        return 0.0;
    }

    previous
        .and_then(|previous| current.checked_sub(previous))
        .map(|delta| delta as f64 / elapsed_secs)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(pid: i32, cpu_percent: f64, resident_bytes: u64) -> RawProcessSample {
        RawProcessSample {
            pid,
            uid: pid as u32,
            command: ProcessCommand::default(),
            cpu_percent,
            memory_percent: 0.0,
            resident_bytes,
            virtual_bytes: 0,
            threads: 1,
        }
    }

    #[test]
    fn selects_only_top_processes_before_sorting() {
        let mut processes = vec![
            process(1, 1.0, 100),
            process(2, 40.0, 100),
            process(3, 40.0, 900),
            process(4, 10.0, 100),
        ];

        select_top_processes(&mut processes, 2);

        let pids = processes
            .into_iter()
            .map(|process| process.pid)
            .collect::<Vec<_>>();
        assert_eq!(pids, vec![3, 2]);
    }

    #[test]
    fn zero_process_limit_skips_process_rows() {
        let mut processes = vec![process(1, 1.0, 100)];

        select_top_processes(&mut processes, 0);

        assert!(processes.is_empty());
    }
}
