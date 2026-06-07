use anyhow::{Context, Result, anyhow};
use libc::{c_char, c_int, c_uint, c_ulonglong};
use std::ffi::{CStr, CString};
use std::mem::{self, MaybeUninit};
use std::ptr;

const MAX_CPUS: usize = 256;
const MAX_INTERFACES: usize = 64;
const MAX_PROCESSES: usize = 16_384;
const MAX_VOLUMES: usize = 256;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CpuTicks {
    pub user: u64,
    pub system: u64,
    pub idle: u64,
    pub nice: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VmStats {
    pub page_size: u64,
    pub total_bytes: u64,
    pub free_pages: u64,
    pub active_pages: u64,
    pub inactive_pages: u64,
    pub speculative_pages: u64,
    pub wired_pages: u64,
    pub compressed_pages: u64,
    pub compressor_pages: u64,
    pub pageins: u64,
    pub pageouts: u64,
    pub swapins: u64,
    pub swapouts: u64,
    pub compressions: u64,
    pub decompressions: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeNetIface {
    pub name: [c_char; 32],
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NativeVolume {
    pub mount_path: [c_char; 1024],
    pub mounted_from: [c_char; 1024],
    pub fs_type: [c_char; 32],
    pub block_size: u64,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub available_blocks: u64,
    pub file_count: u64,
    pub free_file_count: u64,
    pub read_only: u8,
    pub local: u8,
}

impl Default for NativeVolume {
    fn default() -> Self {
        Self {
            mount_path: [0; 1024],
            mounted_from: [0; 1024],
            fs_type: [0; 32],
            block_size: 0,
            total_blocks: 0,
            free_blocks: 0,
            available_blocks: 0,
            file_count: 0,
            free_file_count: 0,
            read_only: 0,
            local: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NativeProcess {
    pub pid: i32,
    pub uid: u32,
    pub command: [c_char; 256],
    pub cpu_time_ns: u64,
    pub resident_bytes: u64,
    pub virtual_bytes: u64,
    pub threads: u64,
}

impl Default for NativeProcess {
    fn default() -> Self {
        Self {
            pid: 0,
            uid: 0,
            command: [0; 256],
            cpu_time_ns: 0,
            resident_bytes: 0,
            virtual_bytes: 0,
            threads: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NetIface {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
}

#[derive(Clone, Debug)]
pub struct VolumeInfo {
    pub mount_path: String,
    pub mounted_from: String,
    pub fs_type: String,
    pub block_size: u64,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub available_blocks: u64,
    pub file_count: u64,
    pub free_file_count: u64,
    pub read_only: bool,
    pub local: bool,
}

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: i32,
    pub uid: u32,
    pub command: ProcessCommand,
    pub cpu_time_ns: u64,
    pub resident_bytes: u64,
    pub virtual_bytes: u64,
    pub threads: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct ProcessCommand([c_char; 256]);

impl Default for ProcessCommand {
    fn default() -> Self {
        Self([0; 256])
    }
}

impl ProcessCommand {
    pub fn text(&self) -> String {
        c_char_array_to_string(&self.0)
    }
}

unsafe extern "C" {
    fn macvmtop_cpu_load(out: *mut CpuTicks, max_cpus: c_uint, count: *mut c_uint) -> c_int;
    fn macvmtop_vm_stats(out: *mut VmStats) -> c_int;
    fn macvmtop_network_interfaces(
        out: *mut NativeNetIface,
        max_ifaces: c_uint,
        count: *mut c_uint,
    ) -> c_int;
    fn macvmtop_storage_volumes(
        out: *mut NativeVolume,
        max_volumes: c_uint,
        count: *mut c_uint,
    ) -> c_int;
    fn macvmtop_processes(
        out: *mut NativeProcess,
        max_processes: c_uint,
        count: *mut c_uint,
    ) -> c_int;
}

pub fn cpu_load() -> Result<Vec<CpuTicks>> {
    let mut out: Vec<MaybeUninit<CpuTicks>> = Vec::with_capacity(MAX_CPUS);
    let mut count: c_uint = 0;
    let rc = unsafe { macvmtop_cpu_load(out.as_mut_ptr().cast(), MAX_CPUS as c_uint, &mut count) };
    ensure_native_ok("macvmtop_cpu_load", rc)?;
    unsafe { assume_init_vec(out, count as usize, MAX_CPUS) }
}

pub fn vm_stats() -> Result<VmStats> {
    let mut out = VmStats::default();
    let rc = unsafe { macvmtop_vm_stats(&mut out) };
    ensure_native_ok("macvmtop_vm_stats", rc)?;
    Ok(out)
}

pub fn network_interfaces() -> Result<Vec<NetIface>> {
    let mut out: Vec<MaybeUninit<NativeNetIface>> = Vec::with_capacity(MAX_INTERFACES);
    let mut count: c_uint = 0;
    let rc = unsafe {
        macvmtop_network_interfaces(
            out.as_mut_ptr().cast(),
            MAX_INTERFACES as c_uint,
            &mut count,
        )
    };
    ensure_native_ok("macvmtop_network_interfaces", rc)?;
    let out = unsafe { assume_init_vec(out, count as usize, MAX_INTERFACES) }?;

    Ok(out
        .into_iter()
        .map(|iface| NetIface {
            name: c_char_array_to_string(&iface.name),
            rx_bytes: iface.rx_bytes,
            tx_bytes: iface.tx_bytes,
            rx_packets: iface.rx_packets,
            tx_packets: iface.tx_packets,
        })
        .collect())
}

pub fn storage_volumes() -> Result<Vec<VolumeInfo>> {
    let mut out: Vec<MaybeUninit<NativeVolume>> = Vec::with_capacity(MAX_VOLUMES);
    let mut count: c_uint = 0;
    let rc = unsafe {
        macvmtop_storage_volumes(out.as_mut_ptr().cast(), MAX_VOLUMES as c_uint, &mut count)
    };
    ensure_native_ok("macvmtop_storage_volumes", rc)?;
    let out = unsafe { assume_init_vec(out, count as usize, MAX_VOLUMES) }?;

    Ok(out
        .into_iter()
        .map(|volume| VolumeInfo {
            mount_path: c_char_array_to_string(&volume.mount_path),
            mounted_from: c_char_array_to_string(&volume.mounted_from),
            fs_type: c_char_array_to_string(&volume.fs_type),
            block_size: volume.block_size,
            total_blocks: volume.total_blocks,
            free_blocks: volume.free_blocks,
            available_blocks: volume.available_blocks,
            file_count: volume.file_count,
            free_file_count: volume.free_file_count,
            read_only: volume.read_only != 0,
            local: volume.local != 0,
        })
        .collect())
}

pub fn processes() -> Result<Vec<ProcessInfo>> {
    let mut out: Vec<MaybeUninit<NativeProcess>> = Vec::with_capacity(MAX_PROCESSES);
    let mut count: c_uint = 0;
    let rc =
        unsafe { macvmtop_processes(out.as_mut_ptr().cast(), MAX_PROCESSES as c_uint, &mut count) };
    ensure_native_ok("macvmtop_processes", rc)?;
    let out = unsafe { assume_init_vec(out, count as usize, MAX_PROCESSES) }?;

    Ok(out
        .into_iter()
        .map(|proc| ProcessInfo {
            pid: proc.pid,
            uid: proc.uid,
            command: ProcessCommand(proc.command),
            cpu_time_ns: proc.cpu_time_ns,
            resident_bytes: proc.resident_bytes,
            virtual_bytes: proc.virtual_bytes,
            threads: proc.threads,
        })
        .collect())
}

pub fn sysctl_string(name: &str) -> Result<String> {
    let cname = CString::new(name).context("sysctl name contains NUL")?;
    let mut size = 0usize;

    let rc = unsafe {
        libc::sysctlbyname(
            cname.as_ptr(),
            ptr::null_mut(),
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if rc != 0 {
        return Err(std::io::Error::last_os_error()).with_context(|| format!("sysctl {name}"));
    }

    if size == 0 {
        return Ok(String::new());
    }

    let mut buf = vec![0u8; size];
    let rc = unsafe {
        libc::sysctlbyname(
            cname.as_ptr(),
            buf.as_mut_ptr().cast(),
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if rc != 0 {
        return Err(std::io::Error::last_os_error()).with_context(|| format!("sysctl {name}"));
    }

    if let Some(pos) = buf.iter().position(|b| *b == 0) {
        buf.truncate(pos);
    }

    Ok(String::from_utf8_lossy(&buf).trim().to_string())
}

pub fn sysctl_i32(name: &str) -> Result<i32> {
    let mut value = 0i32;
    sysctl_value(name, &mut value)?;
    Ok(value)
}

pub fn sysctl_u64(name: &str) -> Result<u64> {
    let mut value = 0u64;
    sysctl_value(name, &mut value)?;
    Ok(value)
}

pub fn load_average() -> [f64; 3] {
    let mut loads = [0f64; 3];
    let n = unsafe { libc::getloadavg(loads.as_mut_ptr(), 3) };
    if n < 0 { [0.0, 0.0, 0.0] } else { loads }
}

pub fn uptime_seconds() -> Result<u64> {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_UPTIME_RAW, &mut ts) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error()).context("clock_gettime CLOCK_UPTIME_RAW");
    }
    Ok(ts.tv_sec as u64)
}

pub fn username(uid: u32) -> String {
    let pwd = unsafe { libc::getpwuid(uid) };
    if pwd.is_null() {
        return uid.to_string();
    }

    unsafe { CStr::from_ptr((*pwd).pw_name) }
        .to_string_lossy()
        .to_string()
}

fn sysctl_value<T>(name: &str, out: &mut T) -> Result<()> {
    let cname = CString::new(name).context("sysctl name contains NUL")?;
    let mut size = mem::size_of::<T>();
    let rc = unsafe {
        libc::sysctlbyname(
            cname.as_ptr(),
            (out as *mut T).cast(),
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if rc != 0 {
        return Err(std::io::Error::last_os_error()).with_context(|| format!("sysctl {name}"));
    }

    Ok(())
}

fn ensure_native_ok(function: &str, rc: c_int) -> Result<()> {
    if rc == 0 {
        Ok(())
    } else {
        Err(anyhow!("{function} failed with native error code {rc}"))
    }
}

unsafe fn assume_init_vec<T>(
    mut values: Vec<MaybeUninit<T>>,
    len: usize,
    max_len: usize,
) -> Result<Vec<T>> {
    if len > max_len || len > values.capacity() {
        return Err(anyhow!(
            "native call returned {len} rows for capacity {max_len}"
        ));
    }

    // Native collectors fully initialize the first `len` slots before returning success.
    unsafe {
        values.set_len(len);
        let ptr = values.as_mut_ptr().cast::<T>();
        let capacity = values.capacity();
        mem::forget(values);
        Ok(Vec::from_raw_parts(ptr, len, capacity))
    }
}

fn c_char_array_to_string<const N: usize>(chars: &[c_char; N]) -> String {
    let ptr = chars.as_ptr();
    if ptr.is_null() {
        return String::new();
    }

    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .trim()
        .to_string()
}

#[allow(dead_code)]
fn _assert_c_types() {
    let _: c_ulonglong = 0;
}
