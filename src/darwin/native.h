#pragma once

#include <stdint.h>

typedef struct {
    uint64_t user;
    uint64_t system;
    uint64_t idle;
    uint64_t nice;
} macvmtop_cpu_ticks_t;

typedef struct {
    uint64_t page_size;
    uint64_t total_bytes;
    uint64_t free_pages;
    uint64_t active_pages;
    uint64_t inactive_pages;
    uint64_t speculative_pages;
    uint64_t wired_pages;
    uint64_t compressed_pages;
    uint64_t compressor_pages;
    uint64_t pageins;
    uint64_t pageouts;
    uint64_t swapins;
    uint64_t swapouts;
    uint64_t compressions;
    uint64_t decompressions;
} macvmtop_vm_stats_t;

typedef struct {
    char name[32];
    uint64_t rx_bytes;
    uint64_t tx_bytes;
    uint64_t rx_packets;
    uint64_t tx_packets;
} macvmtop_net_iface_t;

typedef struct {
    char mount_path[1024];
    char mounted_from[1024];
    char fs_type[32];
    uint64_t block_size;
    uint64_t total_blocks;
    uint64_t free_blocks;
    uint64_t available_blocks;
    uint64_t file_count;
    uint64_t free_file_count;
    uint8_t read_only;
    uint8_t local;
} macvmtop_volume_t;

typedef struct {
    int32_t pid;
    uint32_t uid;
    char command[256];
    uint64_t cpu_time_ns;
    uint64_t resident_bytes;
    uint64_t virtual_bytes;
    uint64_t threads;
} macvmtop_process_t;

int macvmtop_cpu_load(macvmtop_cpu_ticks_t *out, uint32_t max_cpus, uint32_t *count);
int macvmtop_vm_stats(macvmtop_vm_stats_t *out);
int macvmtop_network_interfaces(macvmtop_net_iface_t *out, uint32_t max_ifaces, uint32_t *count);
int macvmtop_storage_volumes(macvmtop_volume_t *out, uint32_t max_volumes, uint32_t *count);
int macvmtop_processes(macvmtop_process_t *out, uint32_t max_processes, uint32_t *count);
