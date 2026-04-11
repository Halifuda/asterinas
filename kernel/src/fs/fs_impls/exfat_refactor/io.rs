// SPDX-License-Identifier: MPL-2.0

use aster_block::{BLOCK_SIZE, BlockDevice};
use ostd::mm::VmIo;

use crate::prelude::*;

/// Reads metadata bytes through a block-aligned bounce buffer.
pub(super) fn read_metadata_bytes(
    block_device: &dyn BlockDevice,
    offset: usize,
    buf: &mut [u8],
) -> Result<()> {
    if buf.is_empty() {
        return Ok(());
    }

    let read_end = offset
        .checked_add(buf.len())
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "metadata read overflow"))?;
    // Expand the request to whole device blocks so callers can ask for any
    // metadata slice without inheriting the device's alignment rules.
    let aligned_start = offset / BLOCK_SIZE * BLOCK_SIZE;
    let aligned_blocks = read_end
        .div_ceil(BLOCK_SIZE)
        .checked_sub(aligned_start / BLOCK_SIZE)
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "metadata read underflow"))?;
    let aligned_len = aligned_blocks
        .checked_mul(BLOCK_SIZE)
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "metadata read overflow"))?;

    let mut aligned_buf = vec![0; aligned_len];
    block_device.read_bytes(aligned_start, &mut aligned_buf)?;

    // Copy only the caller-visible subrange back out of the aligned buffer.
    let start_offset = offset - aligned_start;
    let end_offset = start_offset
        .checked_add(buf.len())
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "metadata slice overflow"))?;
    buf.copy_from_slice(&aligned_buf[start_offset..end_offset]);

    Ok(())
}

#[cfg(ktest)]
mod tests {
    use alloc::vec;

    use aster_block::BLOCK_SIZE;
    use ostd::prelude::ktest;

    use super::read_metadata_bytes;
    use crate::fs::fs_impls::exfat_refactor::{
        boot_sector::read_primary_boot_sector,
        test_support::{ExfatMemoryDisk, load_exfat_disk},
    };

    fn assert_metadata_read_matches_disk(disk: &ExfatMemoryDisk, offset: usize, len: usize) {
        let mut expected = vec![0; len];
        disk.read_bytes(offset, &mut expected);

        let mut actual = vec![0; len];
        read_metadata_bytes(disk, offset, &mut actual).unwrap();

        assert_eq!(actual, expected);
    }

    #[ktest]
    fn metadata_reads_unaligned_slice_across_block_boundary() {
        // Confirms the helper can span a block boundary without exposing the
        // caller to alignment constraints from the block device.
        let disk = load_exfat_disk();

        assert_metadata_read_matches_disk(&disk, BLOCK_SIZE - 13, 64);
    }

    #[ktest]
    fn metadata_reads_checksum_sector_bytes_exactly() {
        // Confirms the helper returns the exact checksum-sector contents even
        // when the requested range starts at an exFAT metadata sector boundary.
        let disk = load_exfat_disk();
        let boot_sector = read_primary_boot_sector(&disk).unwrap();
        let sector_size = 1usize << boot_sector.sector_size_bits;
        let checksum_sector_offset = 11 * sector_size;

        assert_metadata_read_matches_disk(&disk, checksum_sector_offset, sector_size);
    }
}
