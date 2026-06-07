use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct MachineInfo {
    pub model: String,
    pub cpu_brand: String,
    pub kernel_version: String,
    pub os_release: String,
    pub logical_cpus: u32,
    pub physical_cpus: u32,
    pub perf_levels: Option<u32>,
    pub total_memory_bytes: u64,
    pub vm_guest: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SystemSample {
    pub timestamp_unix_ms: u128,
    pub elapsed_ms: u128,
    pub uptime_seconds: u64,
    pub load_average: [f64; 3],
    pub cpu: CpuSample,
    pub memory: MemorySample,
    pub network: Vec<NetworkSample>,
    pub storage: StorageSample,
    pub processes: Vec<ProcessSample>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SystemInfoReport {
    pub machine: MachineInfo,
    pub available: Vec<AvailableMetric>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SystemSnapshotReport {
    pub machine: MachineInfo,
    pub sample: SystemSample,
}

#[derive(Clone, Debug, Serialize)]
pub struct SystemSamplesReport {
    pub machine: MachineInfo,
    pub samples: Vec<SystemSample>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CpuSample {
    pub aggregate_percent: f64,
    pub cores: Vec<CpuCoreSample>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CpuCoreSample {
    pub id: usize,
    pub percent: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct MemorySample {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub free_bytes: u64,
    pub active_bytes: u64,
    pub inactive_bytes: u64,
    pub wired_bytes: u64,
    pub compressed_bytes: u64,
    pub compressor_bytes: u64,
    pub pressure_percent: f64,
    pub pageins: u64,
    pub pageouts: u64,
    pub swapins: u64,
    pub swapouts: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct NetworkSample {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_bytes_per_sec: f64,
    pub tx_bytes_per_sec: f64,
    pub rx_packets: u64,
    pub tx_packets: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct StorageSample {
    pub volumes: Vec<VolumeSample>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VolumeSample {
    pub mount_path: String,
    pub mounted_from: String,
    pub fs_type: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub free_bytes: u64,
    pub used_percent: f64,
    pub block_size: u64,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub available_blocks: u64,
    pub file_count: u64,
    pub free_file_count: u64,
    pub read_only: bool,
    pub local: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProcessSample {
    pub pid: i32,
    pub user: String,
    pub command: String,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub resident_bytes: u64,
    pub virtual_bytes: u64,
    pub threads: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct AvailableMetric {
    pub metric: &'static str,
    pub source: &'static str,
    pub note: &'static str,
}
