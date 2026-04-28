// SPDX-License-Identifier: MPL-2.0

fn rotate_checksum(bytes: &[u8], mut skip_offset_fn: impl FnMut(usize) -> bool) -> u32 {
    let mut checksum = 0u32;
    for (offset, byte) in bytes.iter().enumerate() {
        if skip_offset_fn(offset) {
            continue;
        }
        checksum = checksum.rotate_right(1).wrapping_add(u32::from(*byte));
    }
    checksum
}

pub(super) fn boot_region_checksum(bytes: &[u8]) -> u32 {
    rotate_checksum(bytes, |offset| matches!(offset, 106 | 107 | 112))
}

pub(super) fn stream_checksum(bytes: &[u8]) -> u32 {
    rotate_checksum(bytes, |_| false)
}
