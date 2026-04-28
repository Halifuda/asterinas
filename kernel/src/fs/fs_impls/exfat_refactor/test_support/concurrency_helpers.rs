// SPDX-License-Identifier: MPL-2.0

use alloc::sync::Arc;
use core::sync::atomic::AtomicBool;

use super::disk::{ExfatLookupFlushControlDisk, ExfatLookupTestDisk};
use crate::thread::Thread;

pub(in super::super) fn install_root_file_with_cluster_contents(
    disk: &Arc<ExfatLookupTestDisk>,
    entry_index: usize,
    name: &str,
    clusters: &[u32],
    data_length: usize,
    valid_data_length: usize,
    no_fat_chain: bool,
    contents: &[u8],
) {
    assert_eq!(contents.len(), data_length);
    disk.install_root_file_with_cluster_chain(
        entry_index,
        name,
        clusters[0],
        data_length,
        valid_data_length,
        no_fat_chain,
        clusters,
    );

    let cluster_size = disk.root_cluster_size();
    for (cluster_index, cluster) in clusters.iter().enumerate() {
        let start = cluster_index * cluster_size;
        if start >= contents.len() {
            break;
        }
        let end = (start + cluster_size).min(contents.len());
        disk.write_cluster_prefix(*cluster, &contents[start..end]);
    }

    if !no_fat_chain {
        for cluster_pair in clusters.windows(2) {
            disk.set_fat_chain_step(cluster_pair[0], cluster_pair[1]);
        }
        disk.terminate_fat_chain(*clusters.last().unwrap());
    }
}

pub(in super::super) fn wait_for_flag(flag: &AtomicBool) {
    while !flag.load(core::sync::atomic::Ordering::Relaxed) {
        Thread::yield_now();
    }
}

pub(in super::super) fn wait_for_blocked_flush(flush_control_disk: &ExfatLookupFlushControlDisk) {
    while !flush_control_disk.flush_started() {
        Thread::yield_now();
    }
}
