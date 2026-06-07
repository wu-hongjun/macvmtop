#include "native.h"

#include <errno.h>
#include <ifaddrs.h>
#include <libproc.h>
#include <mach/mach_host.h>
#include <mach/mach_init.h>
#include <mach/mach_time.h>
#include <mach/vm_map.h>
#include <net/if.h>
#include <net/if_dl.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/sysctl.h>

static bool should_skip_interface(const char *name) {
    return strncmp(name, "lo", 2) == 0 ||
           strncmp(name, "awdl", 4) == 0 ||
           strncmp(name, "llw", 3) == 0 ||
           strncmp(name, "utun", 4) == 0 ||
           strncmp(name, "bridge", 6) == 0 ||
           strncmp(name, "gif", 3) == 0 ||
           strncmp(name, "stf", 3) == 0;
}

static const char *basename_c(const char *path) {
    const char *last = strrchr(path, '/');
    return last ? last + 1 : path;
}

int macvmtop_cpu_load(macvmtop_cpu_ticks_t *out, uint32_t max_cpus, uint32_t *count) {
    if (!out || !count || max_cpus == 0) {
        return EINVAL;
    }

    processor_cpu_load_info_t cpu_load = NULL;
    mach_msg_type_number_t cpu_msg_count = 0;
    natural_t cpu_count = 0;

    kern_return_t kr = host_processor_info(
        mach_host_self(),
        PROCESSOR_CPU_LOAD_INFO,
        &cpu_count,
        (processor_info_array_t *)&cpu_load,
        &cpu_msg_count);

    if (kr != KERN_SUCCESS || cpu_load == NULL) {
        return kr == KERN_SUCCESS ? EIO : kr;
    }

    uint32_t n = cpu_count < max_cpus ? cpu_count : max_cpus;
    for (uint32_t i = 0; i < n; i++) {
        out[i].user = cpu_load[i].cpu_ticks[CPU_STATE_USER];
        out[i].system = cpu_load[i].cpu_ticks[CPU_STATE_SYSTEM];
        out[i].idle = cpu_load[i].cpu_ticks[CPU_STATE_IDLE];
        out[i].nice = cpu_load[i].cpu_ticks[CPU_STATE_NICE];
    }
    *count = n;

    vm_deallocate(
        mach_task_self(),
        (vm_address_t)cpu_load,
        (vm_size_t)(cpu_msg_count * sizeof(integer_t)));

    return 0;
}

int macvmtop_vm_stats(macvmtop_vm_stats_t *out) {
    if (!out) {
        return EINVAL;
    }

    memset(out, 0, sizeof(*out));

    vm_statistics64_data_t vm;
    mach_msg_type_number_t count = HOST_VM_INFO64_COUNT;
    kern_return_t kr = host_statistics64(
        mach_host_self(),
        HOST_VM_INFO64,
        (host_info64_t)&vm,
        &count);

    if (kr != KERN_SUCCESS) {
        return kr;
    }

    int64_t memsize = 0;
    size_t memsize_len = sizeof(memsize);
    if (sysctlbyname("hw.memsize", &memsize, &memsize_len, NULL, 0) != 0) {
        memsize = 0;
    }

    out->page_size = (uint64_t)vm_kernel_page_size;
    out->total_bytes = (uint64_t)memsize;
    out->free_pages = vm.free_count;
    out->active_pages = vm.active_count;
    out->inactive_pages = vm.inactive_count;
    out->speculative_pages = vm.speculative_count;
    out->wired_pages = vm.wire_count;
    out->compressed_pages = vm.total_uncompressed_pages_in_compressor;
    out->compressor_pages = vm.compressor_page_count;
    out->pageins = vm.pageins;
    out->pageouts = vm.pageouts;
    out->swapins = vm.swapins;
    out->swapouts = vm.swapouts;
    out->compressions = vm.compressions;
    out->decompressions = vm.decompressions;

    return 0;
}

int macvmtop_network_interfaces(macvmtop_net_iface_t *out, uint32_t max_ifaces, uint32_t *count) {
    if (!out || !count || max_ifaces == 0) {
        return EINVAL;
    }

    struct ifaddrs *ifap = NULL;
    if (getifaddrs(&ifap) != 0) {
        return errno;
    }

    uint32_t n = 0;
    for (struct ifaddrs *ifa = ifap; ifa != NULL && n < max_ifaces; ifa = ifa->ifa_next) {
        if (!ifa->ifa_addr || ifa->ifa_addr->sa_family != AF_LINK || !ifa->ifa_data) {
            continue;
        }
        if (should_skip_interface(ifa->ifa_name)) {
            continue;
        }

        bool seen = false;
        for (uint32_t i = 0; i < n; i++) {
            if (strncmp(out[i].name, ifa->ifa_name, sizeof(out[i].name)) == 0) {
                seen = true;
                break;
            }
        }
        if (seen) {
            continue;
        }

        const struct if_data *data = (const struct if_data *)ifa->ifa_data;
        memset(&out[n], 0, sizeof(out[n]));
        strlcpy(out[n].name, ifa->ifa_name, sizeof(out[n].name));
        out[n].rx_bytes = data->ifi_ibytes;
        out[n].tx_bytes = data->ifi_obytes;
        out[n].rx_packets = data->ifi_ipackets;
        out[n].tx_packets = data->ifi_opackets;
        n++;
    }

    freeifaddrs(ifap);
    *count = n;
    return 0;
}

int macvmtop_storage_volumes(macvmtop_volume_t *out, uint32_t max_volumes, uint32_t *count) {
    if (!out || !count || max_volumes == 0) {
        return EINVAL;
    }

    struct statfs *mounts = NULL;
    int mount_count = getmntinfo(&mounts, MNT_NOWAIT);
    if (mount_count <= 0 || mounts == NULL) {
        return errno ? errno : EIO;
    }

    uint32_t n = 0;
    for (int i = 0; i < mount_count && n < max_volumes; i++) {
        memset(&out[n], 0, sizeof(out[n]));
        strlcpy(out[n].mount_path, mounts[i].f_mntonname, sizeof(out[n].mount_path));
        strlcpy(out[n].mounted_from, mounts[i].f_mntfromname, sizeof(out[n].mounted_from));
        strlcpy(out[n].fs_type, mounts[i].f_fstypename, sizeof(out[n].fs_type));
        out[n].block_size = mounts[i].f_bsize;
        out[n].total_blocks = mounts[i].f_blocks;
        out[n].free_blocks = mounts[i].f_bfree;
        out[n].available_blocks = mounts[i].f_bavail;
        out[n].file_count = mounts[i].f_files;
        out[n].free_file_count = mounts[i].f_ffree;
        out[n].read_only = (mounts[i].f_flags & MNT_RDONLY) != 0;
        out[n].local = (mounts[i].f_flags & MNT_LOCAL) != 0;
        n++;
    }

    *count = n;
    return 0;
}

int macvmtop_processes(macvmtop_process_t *out, uint32_t max_processes, uint32_t *count) {
    if (!out || !count || max_processes == 0) {
        return EINVAL;
    }

    int pid_count = proc_listallpids(NULL, 0);
    if (pid_count <= 0) {
        return errno ? errno : EIO;
    }

    pid_t *pids = calloc((size_t)pid_count, sizeof(pid_t));
    if (!pids) {
        return ENOMEM;
    }

    int bytes = proc_listallpids(pids, pid_count * (int)sizeof(pid_t));
    if (bytes <= 0) {
        int err = errno ? errno : EIO;
        free(pids);
        return err;
    }

    int actual_count = bytes / (int)sizeof(pid_t);
    mach_timebase_info_data_t timebase = {0, 0};
    mach_timebase_info(&timebase);
    if (timebase.denom == 0) {
        timebase.denom = 1;
    }

    uint32_t n = 0;
    for (int i = 0; i < actual_count && n < max_processes; i++) {
        pid_t pid = pids[i];
        if (pid <= 0) {
            continue;
        }

        struct proc_taskallinfo info;
        int ret = proc_pidinfo(pid, PROC_PIDTASKALLINFO, 0, &info, sizeof(info));
        if (ret != sizeof(info)) {
            continue;
        }

        memset(&out[n], 0, sizeof(out[n]));
        out[n].pid = pid;
        out[n].uid = info.pbsd.pbi_uid;
        out[n].resident_bytes = info.ptinfo.pti_resident_size;
        out[n].virtual_bytes = info.ptinfo.pti_virtual_size;
        out[n].threads = info.ptinfo.pti_threadnum;

        uint64_t raw_time = info.ptinfo.pti_total_user + info.ptinfo.pti_total_system;
        out[n].cpu_time_ns = (raw_time * (uint64_t)timebase.numer) / (uint64_t)timebase.denom;

        char path[PROC_PIDPATHINFO_MAXSIZE];
        int path_len = proc_pidpath(pid, path, sizeof(path));
        if (path_len > 0) {
            strlcpy(out[n].command, basename_c(path), sizeof(out[n].command));
        } else {
            strlcpy(out[n].command, info.pbsd.pbi_comm, sizeof(out[n].command));
        }

        n++;
    }

    free(pids);
    *count = n;
    return 0;
}
