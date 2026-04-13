// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "Directory record streaming is staged before later directory consumers land."
    )
)]

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::{
    allocator::AllocationResult,
    dentry::{DENTRY_SIZE, ExfatDeletedDentry, ExfatDentry, ExfatFileDentry, RawExfatDentry},
    fat::{ChainMode, ClusterId, ExfatChain, FatValue, write_next_fat_value},
    fileset::ExfatDentrySet,
    io::read_metadata_bytes,
    super_block::ExfatSuperBlock,
};
use crate::{fs::file::InodeType, prelude::*};

const EXFAT_FILE_ATTRIBUTE_DIRECTORY: u16 = 0x10;
const EXFAT_STREAM_FLAG_CONTIGUOUS: u8 = 0x02;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DirectorySlotSearch {
    Fits {
        start_entry_index: usize,
        consumes_unused_terminator: bool,
    },
    TailTooShort {
        start_entry_index: usize,
        consumes_unused_terminator: bool,
    },
    NoReusableTail,
}

/// Streams directory records in on-disk order.
#[derive(Debug)]
pub(super) struct DirectoryEngine<'a> {
    block_device: &'a dyn BlockDevice,
    super_block: &'a ExfatSuperBlock,
    parent_ino: Option<u64>,
    chain: ExfatChain,
    directory_end_offset: usize,
    cursor: DirectoryCursor,
}

#[derive(Clone, Copy, Debug)]
struct DirectoryCursor {
    byte_offset: usize,
    cluster_offset: usize,
}

/// Carries the trusted location facts for one validated file record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DirectoryRecordLocation {
    parent_ino: Option<u64>,
    dentry_set_byte_offset: usize,
    dentry_entry_index: u32,
}

impl DirectoryRecordLocation {
    fn new(
        parent_ino: Option<u64>,
        dentry_set_byte_offset: usize,
        dentry_entry_index: u32,
    ) -> Self {
        Self {
            parent_ino,
            dentry_set_byte_offset,
            dentry_entry_index,
        }
    }

    pub(super) fn inode_key_parts(self) -> (Option<u64>, usize, u32) {
        (
            self.parent_ino,
            self.dentry_set_byte_offset,
            self.dentry_entry_index,
        )
    }

    fn stable_inode_number(self) -> u64 {
        let parent_ino = self.parent_ino.unwrap_or(0);
        let byte_offset = u64::try_from(self.dentry_set_byte_offset).unwrap_or(u64::MAX);
        // Keep dirent inode numbers stable across rescans by deriving them only
        // from the validated location facts that also feed `InodeKey`.
        let mixed = parent_ino.wrapping_mul(0x9E37_79B1_85EB_CA87)
            ^ byte_offset.rotate_left(17)
            ^ u64::from(self.dentry_entry_index).rotate_left(3)
            ^ 0xA57E_B707_EF32_A31D;

        if mixed <= 1 {
            mixed.wrapping_add(2)
        } else {
            mixed
        }
    }
}

/// Carries one validated file record plus the trusted location facts that identify it.
#[derive(Debug)]
pub(super) struct DirectoryFileRecord {
    dentry_set: ExfatDentrySet,
    location: DirectoryRecordLocation,
}

impl DirectoryFileRecord {
    fn new(dentry_set: ExfatDentrySet, location: DirectoryRecordLocation) -> Self {
        Self {
            dentry_set,
            location,
        }
    }

    pub(super) fn dentry_set(&self) -> &ExfatDentrySet {
        &self.dentry_set
    }

    pub(super) fn location(&self) -> DirectoryRecordLocation {
        self.location
    }

    pub(super) fn raw_name_units(&self) -> Vec<u16> {
        self.dentry_set.raw_name_units()
    }

    pub(super) fn inode_type(&self) -> InodeType {
        if self.file_attribute() & EXFAT_FILE_ATTRIBUTE_DIRECTORY != 0 {
            InodeType::Dir
        } else {
            InodeType::File
        }
    }

    pub(super) fn inode_number(&self) -> u64 {
        self.location.stable_inode_number()
    }

    pub(super) fn file_attribute(&self) -> u16 {
        self.dentry_set.file_dentry().attribute
    }

    pub(super) fn start_cluster(&self) -> ClusterId {
        self.dentry_set.stream_dentry().start_cluster
    }

    pub(super) fn chain_mode(&self) -> ChainMode {
        if self.dentry_set.stream_dentry().flags & EXFAT_STREAM_FLAG_CONTIGUOUS != 0 {
            ChainMode::Contiguous
        } else {
            ChainMode::FatBacked
        }
    }

    pub(super) fn cluster_count(&self, cluster_size: usize) -> Result<u32> {
        if cluster_size == 0 {
            return Err(Error::with_message(
                Errno::EINVAL,
                "directory record cluster size must be non-zero",
            ));
        }

        let allocated_size =
            usize::try_from(self.dentry_set.stream_dentry().size).map_err(|_| {
                Error::with_message(
                    Errno::EOVERFLOW,
                    "directory record allocated size overflowed usize",
                )
            })?;
        let cluster_count = allocated_size.div_ceil(cluster_size);
        u32::try_from(cluster_count).map_err(|_| {
            Error::with_message(
                Errno::EOVERFLOW,
                "directory record cluster count overflowed u32",
            )
        })
    }
}

/// Describes one logical directory record emitted by the scan service.
#[derive(Debug)]
pub(super) enum DirectoryRecord {
    File(DirectoryFileRecord),
    Singleton(ExfatDentry),
}

impl<'a> DirectoryEngine<'a> {
    /// Creates a read-only directory stream over a validated chain.
    pub(super) fn new(
        block_device: &'a dyn BlockDevice,
        super_block: &'a ExfatSuperBlock,
        parent_ino: Option<u64>,
        chain: ExfatChain,
    ) -> Result<Self> {
        if chain.is_empty() {
            return Err(Error::with_message(
                Errno::EINVAL,
                "directory chain must not be empty",
            ));
        }

        let cluster_count = usize::try_from(chain.cluster_count()).map_err(|_| {
            Error::with_message(Errno::EINVAL, "directory chain size overflows usize")
        })?;
        let directory_end_offset = cluster_count
            .checked_mul(super_block.cluster_size())
            .ok_or_else(|| {
                Error::with_message(Errno::EINVAL, "directory byte size overflows usize")
            })?;

        Ok(Self {
            block_device,
            super_block,
            parent_ino,
            chain,
            directory_end_offset,
            cursor: DirectoryCursor {
                byte_offset: 0,
                cluster_offset: 0,
            },
        })
    }

    /// Returns the next record in on-disk order.
    pub(super) fn next_record(&mut self) -> Result<Option<DirectoryRecord>> {
        loop {
            let Some(dentry) = self.read_next_dentry()? else {
                return Ok(None);
            };

            match dentry {
                ExfatDentry::Deleted(_) => continue,
                ExfatDentry::Unused => return Ok(None),
                dentry if dentry.is_volume_label() => continue,
                ExfatDentry::File(file_dentry) => {
                    let record_start_offset = self
                        .cursor
                        .byte_offset
                        .checked_sub(DENTRY_SIZE)
                        .ok_or_else(|| {
                            Error::with_message(
                                Errno::EINVAL,
                                "directory file record offset underflow",
                            )
                        })?;
                    let record = self.read_file_record(file_dentry, record_start_offset)?;
                    return Ok(Some(DirectoryRecord::File(record)));
                }
                ExfatDentry::Bitmap(bitmap_dentry) => {
                    return Ok(Some(DirectoryRecord::Singleton(ExfatDentry::Bitmap(
                        bitmap_dentry,
                    ))));
                }
                ExfatDentry::Upcase(upcase_dentry) => {
                    return Ok(Some(DirectoryRecord::Singleton(ExfatDentry::Upcase(
                        upcase_dentry,
                    ))));
                }
                _ => {
                    return Err(Error::with_message(
                        Errno::EINVAL,
                        "unexpected top-level directory dentry",
                    ));
                }
            }
        }
    }

    fn read_file_record(
        &mut self,
        file_dentry: ExfatFileDentry,
        record_start_offset: usize,
    ) -> Result<DirectoryFileRecord> {
        let secondary_count = usize::from(file_dentry.num_secondary);
        let mut dentries = Vec::with_capacity(secondary_count + 1);
        dentries.push(ExfatDentry::File(file_dentry));

        for _ in 0..secondary_count {
            let Some(dentry) = self.read_next_dentry()? else {
                return Err(Error::with_message(
                    Errno::EINVAL,
                    "truncated directory file record",
                ));
            };

            if matches!(dentry, ExfatDentry::Deleted(_) | ExfatDentry::Unused) {
                return Err(Error::with_message(
                    Errno::EINVAL,
                    "directory file record was truncated or interrupted",
                ));
            }

            dentries.push(dentry);
        }

        let dentry_entry_index =
            u32::try_from(record_start_offset / DENTRY_SIZE).map_err(|_| {
                Error::with_message(
                    Errno::EOVERFLOW,
                    "directory record entry index overflowed u32",
                )
            })?;
        let location =
            DirectoryRecordLocation::new(self.parent_ino, record_start_offset, dentry_entry_index);

        Ok(DirectoryFileRecord::new(
            ExfatDentrySet::new(dentries)?,
            location,
        ))
    }

    fn read_next_dentry(&mut self) -> Result<Option<ExfatDentry>> {
        if self.cursor.byte_offset >= self.directory_end_offset {
            return Ok(None);
        }

        if self.cursor.cluster_offset >= self.super_block.cluster_size() {
            self.advance_cluster()?;
        }

        let cluster_start_offset = self.chain.physical_cluster_start_offset(self.super_block)?;
        let physical_offset = cluster_start_offset
            .checked_add(self.cursor.cluster_offset)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory offset overflow"))?;

        let mut raw_bytes = [0; DENTRY_SIZE];
        read_metadata_bytes(self.block_device, physical_offset, &mut raw_bytes)?;

        self.cursor.byte_offset = self
            .cursor
            .byte_offset
            .checked_add(DENTRY_SIZE)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory cursor overflow"))?;
        self.cursor.cluster_offset += DENTRY_SIZE;

        Ok(Some(ExfatDentry::from(RawExfatDentry::from_bytes(
            &raw_bytes,
        ))))
    }

    fn advance_cluster(&mut self) -> Result<()> {
        self.chain = self.chain.walk(self.block_device, self.super_block, 1)?;
        self.cursor.cluster_offset = 0;
        Ok(())
    }

    /// Places a validated file-record set into a reusable slot range.
    pub(super) fn place_dentry_set(
        &mut self,
        dentry_set: &ExfatDentrySet,
        committed_growth: Option<AllocationResult>,
    ) -> Result<DirectoryRecordLocation> {
        let bytes = dentry_set.to_le_bytes();
        let required_slots = bytes.len() / DENTRY_SIZE;
        let search = self.find_reusable_slot_run(required_slots)?;

        match search {
            DirectorySlotSearch::Fits {
                start_entry_index,
                consumes_unused_terminator,
            } => {
                self.write_dentry_bytes_at(start_entry_index, &bytes)?;
                if consumes_unused_terminator {
                    self.publish_unused_terminator(start_entry_index, required_slots)?;
                }
                self.location_for_entry_index(start_entry_index)
            }
            DirectorySlotSearch::TailTooShort {
                start_entry_index,
                consumes_unused_terminator,
            } => {
                let Some(committed_growth) = committed_growth else {
                    return Err(Error::with_message(
                        Errno::ENOSPC,
                        "directory has no reusable slot range for the validated record",
                    ));
                };
                self.extend_directory_chain(committed_growth)?;
                self.write_dentry_bytes_at(start_entry_index, &bytes)?;
                if consumes_unused_terminator
                    || start_entry_index
                        .checked_add(required_slots)
                        .ok_or_else(|| {
                            Error::with_message(Errno::EINVAL, "directory tail overflow")
                        })?
                        < self.directory_entry_count()
                {
                    self.publish_unused_terminator(start_entry_index, required_slots)?;
                }
                self.location_for_entry_index(start_entry_index)
            }
            DirectorySlotSearch::NoReusableTail => {
                let Some(committed_growth) = committed_growth else {
                    return Err(Error::with_message(
                        Errno::ENOSPC,
                        "directory has no reusable slot range for the validated record",
                    ));
                };
                let placement_entry_index = self.directory_entry_count();
                self.extend_directory_chain(committed_growth)?;
                self.write_dentry_bytes_at(placement_entry_index, &bytes)?;
                self.publish_unused_terminator(placement_entry_index, required_slots)?;
                self.location_for_entry_index(placement_entry_index)
            }
        }
    }

    /// Rewrites a validated file-record set at a trusted directory location.
    pub(super) fn rewrite_dentry_set(
        &mut self,
        location: DirectoryRecordLocation,
        dentry_set: &ExfatDentrySet,
        committed_growth: Option<AllocationResult>,
    ) -> Result<DirectoryRecordLocation> {
        let bytes = dentry_set.to_le_bytes();
        let required_slots = bytes.len() / DENTRY_SIZE;
        let location_entry_index = self.entry_index_from_location(location)?;
        let existing_slots = self.record_slot_count(location)?;
        let trailing_start_index = location_entry_index
            .checked_add(existing_slots)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory slot range overflow"))?;
        let (trailing_reusable_slots, trailing_consumes_unused_terminator) =
            self.trailing_reusable_tail_state(trailing_start_index, required_slots)?;
        let in_place_capacity = existing_slots
            .checked_add(trailing_reusable_slots)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory slot range overflow"))?;

        if required_slots <= in_place_capacity {
            self.write_dentry_bytes_at(location_entry_index, &bytes)?;
            if required_slots < existing_slots {
                let tombstone_start_index = location_entry_index
                    .checked_add(required_slots)
                    .ok_or_else(|| {
                        Error::with_message(Errno::EINVAL, "directory slot range overflow")
                    })?;
                self.tombstone_slot_range(tombstone_start_index, existing_slots - required_slots)?;
            } else if trailing_consumes_unused_terminator {
                self.publish_unused_terminator(location_entry_index, required_slots)?;
            }

            return Ok(location);
        }

        let search = self.find_reusable_slot_run(required_slots)?;
        match search {
            DirectorySlotSearch::Fits {
                start_entry_index,
                consumes_unused_terminator,
            } => {
                self.write_dentry_bytes_at(start_entry_index, &bytes)?;
                self.tombstone_slot_range(location_entry_index, existing_slots)?;
                if consumes_unused_terminator {
                    self.publish_unused_terminator(start_entry_index, required_slots)?;
                }
                self.location_for_entry_index(start_entry_index)
            }
            DirectorySlotSearch::TailTooShort {
                start_entry_index,
                consumes_unused_terminator,
            } => {
                let Some(committed_growth) = committed_growth else {
                    return Err(Error::with_message(
                        Errno::ENOSPC,
                        "directory has no reusable slot range for the relocated record",
                    ));
                };
                self.extend_directory_chain(committed_growth)?;
                self.write_dentry_bytes_at(start_entry_index, &bytes)?;
                self.tombstone_slot_range(location_entry_index, existing_slots)?;
                if consumes_unused_terminator
                    || start_entry_index
                        .checked_add(required_slots)
                        .ok_or_else(|| {
                            Error::with_message(Errno::EINVAL, "directory tail overflow")
                        })?
                        < self.directory_entry_count()
                {
                    self.publish_unused_terminator(start_entry_index, required_slots)?;
                }
                self.location_for_entry_index(start_entry_index)
            }
            DirectorySlotSearch::NoReusableTail => {
                let Some(committed_growth) = committed_growth else {
                    return Err(Error::with_message(
                        Errno::ENOSPC,
                        "directory has no reusable slot range for the relocated record",
                    ));
                };
                let placement_entry_index = self.directory_entry_count();
                self.extend_directory_chain(committed_growth)?;
                self.write_dentry_bytes_at(placement_entry_index, &bytes)?;
                self.tombstone_slot_range(location_entry_index, existing_slots)?;
                self.publish_unused_terminator(placement_entry_index, required_slots)?;
                self.location_for_entry_index(placement_entry_index)
            }
        }
    }

    /// Tombstones the live slots for a validated file record.
    pub(super) fn tombstone_dentry_set(&mut self, location: DirectoryRecordLocation) -> Result<()> {
        let location_entry_index = self.entry_index_from_location(location)?;
        let slot_count = self.record_slot_count(location)?;
        self.tombstone_slot_range(location_entry_index, slot_count)
    }

    fn directory_entry_count(&self) -> usize {
        self.directory_end_offset / DENTRY_SIZE
    }

    fn entry_index_from_location(&self, location: DirectoryRecordLocation) -> Result<usize> {
        let entry_index = usize::try_from(location.dentry_entry_index).map_err(|_| {
            Error::with_message(
                Errno::EOVERFLOW,
                "directory record entry index overflowed usize",
            )
        })?;
        let expected_offset = self.entry_byte_offset(entry_index)?;
        if expected_offset != location.dentry_set_byte_offset {
            return Err(Error::with_message(
                Errno::EINVAL,
                "directory record location is inconsistent",
            ));
        }

        Ok(entry_index)
    }

    fn location_for_entry_index(&self, entry_index: usize) -> Result<DirectoryRecordLocation> {
        let byte_offset = self.entry_byte_offset(entry_index)?;
        let dentry_entry_index = u32::try_from(entry_index).map_err(|_| {
            Error::with_message(
                Errno::EOVERFLOW,
                "directory record entry index overflowed u32",
            )
        })?;
        Ok(DirectoryRecordLocation::new(
            self.parent_ino,
            byte_offset,
            dentry_entry_index,
        ))
    }

    fn entry_byte_offset(&self, entry_index: usize) -> Result<usize> {
        entry_index.checked_mul(DENTRY_SIZE).ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "directory entry byte offset overflow")
        })
    }

    fn read_dentry_at(&self, entry_index: usize) -> Result<ExfatDentry> {
        let mut raw_bytes = [0; DENTRY_SIZE];
        let byte_offset = self.entry_byte_offset(entry_index)?;
        self.read_directory_bytes(byte_offset, &mut raw_bytes)?;

        Ok(ExfatDentry::from(RawExfatDentry::from_bytes(&raw_bytes)))
    }

    fn record_slot_count(&self, location: DirectoryRecordLocation) -> Result<usize> {
        let entry_index = self.entry_index_from_location(location)?;
        match self.read_dentry_at(entry_index)? {
            ExfatDentry::File(file_dentry) => Ok(usize::from(file_dentry.num_secondary) + 1),
            _ => Err(Error::with_message(
                Errno::EINVAL,
                "directory record location does not point at a file primary",
            )),
        }
    }

    fn trailing_reusable_tail_state(
        &self,
        start_entry_index: usize,
        required_slots: usize,
    ) -> Result<(usize, bool)> {
        let total_entries = self.directory_entry_count();
        if start_entry_index >= total_entries {
            return Ok((0, false));
        }

        let mut reusable_slots = 0usize;
        for entry_index in start_entry_index..total_entries {
            match self.read_dentry_at(entry_index)? {
                ExfatDentry::Deleted(_) => {
                    reusable_slots += 1;
                }
                ExfatDentry::Unused => {
                    let consumes_unused_terminator = required_slots > reusable_slots;
                    return Ok((
                        total_entries - start_entry_index,
                        consumes_unused_terminator,
                    ));
                }
                _ => break,
            }
        }

        Ok((reusable_slots, false))
    }

    fn find_reusable_slot_run(&self, required_slots: usize) -> Result<DirectorySlotSearch> {
        if required_slots == 0 {
            return Err(Error::with_message(
                Errno::EINVAL,
                "validated directory records must occupy at least one slot",
            ));
        }

        let total_entries = self.directory_entry_count();
        let mut run_start = None;
        let mut run_length = 0usize;

        for entry_index in 0..total_entries {
            match self.read_dentry_at(entry_index)? {
                ExfatDentry::Deleted(_) => {
                    run_start.get_or_insert(entry_index);
                    run_length += 1;
                    if run_length >= required_slots {
                        return Ok(DirectorySlotSearch::Fits {
                            start_entry_index: run_start.expect("deleted run start must be set"),
                            consumes_unused_terminator: false,
                        });
                    }
                }
                ExfatDentry::Unused => {
                    let start_entry_index = run_start.unwrap_or(entry_index);
                    let consumes_unused_terminator =
                        required_slots > entry_index - start_entry_index;
                    if total_entries - start_entry_index >= required_slots {
                        return Ok(DirectorySlotSearch::Fits {
                            start_entry_index,
                            consumes_unused_terminator,
                        });
                    }
                    return Ok(DirectorySlotSearch::TailTooShort {
                        start_entry_index,
                        consumes_unused_terminator,
                    });
                }
                _ => {
                    run_start = None;
                    run_length = 0;
                }
            }
        }

        if let Some(start_entry_index) = run_start {
            if total_entries - start_entry_index >= required_slots {
                return Ok(DirectorySlotSearch::Fits {
                    start_entry_index,
                    consumes_unused_terminator: false,
                });
            }
            return Ok(DirectorySlotSearch::TailTooShort {
                start_entry_index,
                consumes_unused_terminator: false,
            });
        }

        Ok(DirectorySlotSearch::NoReusableTail)
    }

    fn tombstone_slot_range(&self, start_entry_index: usize, slot_count: usize) -> Result<()> {
        if slot_count == 0 {
            return Ok(());
        }

        let end_entry_index = start_entry_index.checked_add(slot_count).ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "directory tombstone range overflow")
        })?;
        let tombstone = deleted_dentry();
        for entry_index in start_entry_index..end_entry_index {
            self.write_dentry_bytes_at(entry_index, tombstone.as_bytes())?;
        }

        Ok(())
    }

    fn write_dentry_bytes_at(&self, entry_index: usize, bytes: &[u8]) -> Result<()> {
        let byte_offset = self.entry_byte_offset(entry_index)?;
        self.write_directory_bytes(byte_offset, bytes)
    }

    fn publish_unused_terminator(
        &self,
        record_start_entry_index: usize,
        record_slot_count: usize,
    ) -> Result<()> {
        let terminator_entry_index = record_start_entry_index
            .checked_add(record_slot_count)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory tail overflow"))?;

        if terminator_entry_index >= self.directory_entry_count() {
            if terminator_entry_index == self.directory_entry_count() {
                return Ok(());
            }
            return Err(Error::with_message(
                Errno::EINVAL,
                "directory growth did not leave room for an Unused terminator",
            ));
        }

        self.write_dentry_bytes_at(terminator_entry_index, ExfatDentry::Unused.as_bytes())
    }

    fn read_directory_bytes(&self, offset: usize, buf: &mut [u8]) -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }

        let read_end = offset
            .checked_add(buf.len())
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory read overflow"))?;
        if read_end > self.directory_end_offset {
            return Err(Error::with_message(
                Errno::EINVAL,
                "directory read exceeded the logical directory stream",
            ));
        }

        let mut copied_bytes = 0usize;
        while copied_bytes < buf.len() {
            let logical_offset = offset
                .checked_add(copied_bytes)
                .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory read overflow"))?;
            let (physical_offset, chunk_len) =
                self.logical_directory_chunk_at(logical_offset, buf.len() - copied_bytes)?;
            read_metadata_bytes(
                self.block_device,
                physical_offset,
                &mut buf[copied_bytes..copied_bytes + chunk_len],
            )?;
            copied_bytes += chunk_len;
        }

        Ok(())
    }

    fn write_directory_bytes(&self, offset: usize, buf: &[u8]) -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }

        let write_end = offset
            .checked_add(buf.len())
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory write overflow"))?;
        if write_end > self.directory_end_offset {
            return Err(Error::with_message(
                Errno::EINVAL,
                "directory write exceeded the logical directory stream",
            ));
        }

        let mut written_bytes = 0usize;
        while written_bytes < buf.len() {
            let logical_offset = offset
                .checked_add(written_bytes)
                .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory write overflow"))?;
            let (physical_offset, chunk_len) =
                self.logical_directory_chunk_at(logical_offset, buf.len() - written_bytes)?;
            self.write_physical_metadata_bytes(
                physical_offset,
                &buf[written_bytes..written_bytes + chunk_len],
            )?;
            written_bytes += chunk_len;
        }

        Ok(())
    }

    fn write_physical_metadata_bytes(&self, offset: usize, buf: &[u8]) -> Result<()> {
        use aster_block::BLOCK_SIZE;

        if buf.is_empty() {
            return Ok(());
        }

        let write_end = offset
            .checked_add(buf.len())
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "metadata write overflow"))?;
        let aligned_start = offset / BLOCK_SIZE * BLOCK_SIZE;
        let aligned_blocks = write_end
            .div_ceil(BLOCK_SIZE)
            .checked_sub(aligned_start / BLOCK_SIZE)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "metadata write underflow"))?;
        let aligned_len = aligned_blocks
            .checked_mul(BLOCK_SIZE)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "metadata write overflow"))?;

        let mut aligned_buf = Vec::with_capacity(aligned_len);
        aligned_buf.resize(aligned_len, 0);
        read_metadata_bytes(self.block_device, aligned_start, &mut aligned_buf)?;

        let start_offset = offset - aligned_start;
        let end_offset = start_offset
            .checked_add(buf.len())
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "metadata slice overflow"))?;
        aligned_buf[start_offset..end_offset].copy_from_slice(buf);

        self.block_device.write_bytes(aligned_start, &aligned_buf)?;

        Ok(())
    }

    fn logical_directory_chunk_at(&self, offset: usize, max_len: usize) -> Result<(usize, usize)> {
        let (cluster_chain, cluster_intra_offset) =
            self.chain
                .walk_to_cluster_at_offset(self.block_device, self.super_block, offset)?;
        let cluster_start_offset = cluster_chain.physical_cluster_start_offset(self.super_block)?;
        let cluster_remaining = self
            .super_block
            .cluster_size()
            .checked_sub(cluster_intra_offset)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory chunk underflow"))?;
        let chunk_len = max_len.min(cluster_remaining);
        let physical_offset = cluster_start_offset
            .checked_add(cluster_intra_offset)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory chunk overflow"))?;

        Ok((physical_offset, chunk_len))
    }

    fn extend_directory_chain(&mut self, growth: AllocationResult) -> Result<()> {
        if growth.cluster_count == 0 {
            return Err(Error::with_message(
                Errno::EINVAL,
                "directory growth must allocate at least one cluster",
            ));
        }

        let appended_chain = ExfatChain::new(
            self.block_device,
            self.super_block,
            growth.start_cluster,
            Some(growth.cluster_count),
            growth.chain_mode,
        )?;
        let current_clusters = self.collect_chain_clusters(self.chain)?;
        let appended_clusters = self.collect_chain_clusters(appended_chain)?;
        let mut combined_clusters = Vec::with_capacity(
            current_clusters
                .len()
                .checked_add(appended_clusters.len())
                .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory chain overflow"))?,
        );
        combined_clusters.extend_from_slice(&current_clusters);
        combined_clusters.extend_from_slice(&appended_clusters);

        // Materialize the full chain locally so the directory can keep using a
        // single owner-private traversal model after growth.
        self.materialize_directory_chain(&combined_clusters)?;

        let combined_cluster_count = u32::try_from(combined_clusters.len()).map_err(|_| {
            Error::with_message(
                Errno::EOVERFLOW,
                "directory chain cluster count overflowed u32",
            )
        })?;
        let head_cluster = *combined_clusters.first().ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "directory chain must not be empty")
        })?;
        self.chain = ExfatChain::new(
            self.block_device,
            self.super_block,
            head_cluster,
            Some(combined_cluster_count),
            ChainMode::FatBacked,
        )?;

        let growth_clusters = usize::try_from(growth.cluster_count).map_err(|_| {
            Error::with_message(
                Errno::EOVERFLOW,
                "directory growth cluster count overflowed usize",
            )
        })?;
        let growth_bytes = growth_clusters
            .checked_mul(self.super_block.cluster_size())
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory growth overflow"))?;
        self.directory_end_offset = self
            .directory_end_offset
            .checked_add(growth_bytes)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory growth overflow"))?;

        Ok(())
    }

    fn collect_chain_clusters(&self, chain: ExfatChain) -> Result<Vec<ClusterId>> {
        let cluster_count = usize::try_from(chain.cluster_count()).map_err(|_| {
            Error::with_message(Errno::EINVAL, "directory chain size overflows usize")
        })?;
        let mut clusters = Vec::with_capacity(cluster_count);
        let mut loaded_chain = chain;

        for cluster_index in 0..chain.cluster_count() {
            clusters.push(loaded_chain.current_cluster());
            if cluster_index + 1 < chain.cluster_count() {
                loaded_chain = loaded_chain.walk(self.block_device, self.super_block, 1)?;
            }
        }

        Ok(clusters)
    }

    fn materialize_directory_chain(&self, clusters: &[ClusterId]) -> Result<()> {
        let Some(&last_cluster) = clusters.last() else {
            return Err(Error::with_message(
                Errno::EINVAL,
                "directory chain must not be empty",
            ));
        };

        for window in clusters.windows(2) {
            write_next_fat_value(
                self.block_device,
                self.super_block,
                window[0],
                FatValue::Next(window[1]),
            )?;
        }

        write_next_fat_value(
            self.block_device,
            self.super_block,
            last_cluster,
            FatValue::EndOfChain,
        )?;

        Ok(())
    }
}

fn deleted_dentry() -> ExfatDentry {
    ExfatDentry::Deleted(ExfatDeletedDentry {
        dentry_type: 0x01,
        reserved: [0; DENTRY_SIZE - 1],
    })
}

#[cfg(ktest)]
mod tests {
    use alloc::{vec, vec::Vec};

    use ostd::{mm::VmIo, prelude::ktest};
    use zerocopy::IntoBytes;

    use super::{DirectoryEngine, DirectoryRecord};
    use crate::{
        fs::fs_impls::exfat_refactor::{
            allocator::AllocationResult,
            boot_sector::read_primary_super_block,
            dentry::{
                DENTRY_SIZE, ExfatBitmapDentry, ExfatDentry, ExfatStreamDentry, ExfatUpcaseDentry,
                RawExfatDentry,
            },
            fat::{ChainMode, ExfatChain},
            fileset::ExfatDentrySet,
            super_block::ExfatSuperBlock,
            test_support::{ExfatMemoryDisk, load_exfat_disk},
        },
        prelude::{Errno, Pod},
    };

    fn make_directory_engine<'a>(
        disk: &'a ExfatMemoryDisk,
        super_block: &'a ExfatSuperBlock,
        cluster_count: u32,
    ) -> DirectoryEngine<'a> {
        let chain = ExfatChain::new(
            disk,
            super_block,
            super_block.root_dir,
            Some(cluster_count),
            ChainMode::Contiguous,
        )
        .unwrap();

        DirectoryEngine::new(disk, super_block, Some(1), chain).unwrap()
    }

    fn write_raw_dentry(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
        entry_index: usize,
        dentry: RawExfatDentry,
    ) {
        let cluster_start = super_block
            .cluster_to_byte_offset(super_block.root_dir)
            .unwrap();
        let offset = cluster_start + entry_index * DENTRY_SIZE;
        disk.write_bytes(offset, dentry.as_bytes());
    }

    fn write_entry_sequence(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
        start_index: usize,
        entries: &[ExfatDentry],
    ) {
        for (offset, entry) in entries.iter().enumerate() {
            let cluster_start = super_block
                .cluster_to_byte_offset(super_block.root_dir)
                .unwrap();
            let byte_offset = cluster_start + (start_index + offset) * DENTRY_SIZE;
            disk.write_bytes(byte_offset, entry.as_bytes());
        }
    }

    fn write_serialized_bytes(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
        start_index: usize,
        bytes: &[u8],
    ) {
        for (offset, chunk) in bytes.chunks_exact(DENTRY_SIZE).enumerate() {
            write_raw_dentry(
                disk,
                super_block,
                start_index + offset,
                RawExfatDentry::from_bytes(chunk),
            );
        }
    }

    fn deleted_dentry() -> RawExfatDentry {
        RawExfatDentry {
            dentry_type: 0x01,
            value: [0; 31],
        }
    }

    fn unused_dentry() -> RawExfatDentry {
        RawExfatDentry {
            dentry_type: 0x00,
            value: [0; 31],
        }
    }

    fn read_raw_dentry(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
        entry_index: usize,
    ) -> RawExfatDentry {
        let cluster_start = super_block
            .cluster_to_byte_offset(super_block.root_dir)
            .unwrap();
        let offset = cluster_start + entry_index * DENTRY_SIZE;
        let mut bytes = [0; DENTRY_SIZE];
        disk.read_bytes(offset, &mut bytes);
        RawExfatDentry::from_bytes(&bytes)
    }

    fn committed_growth(
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
    ) -> AllocationResult {
        AllocationResult {
            start_cluster: super_block.data_cluster_end_exclusive() - 1,
            cluster_count: 1,
            chain_mode: ChainMode::FatBacked,
        }
    }

    fn validated_file_set(
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
        raw_name_units: &[u16],
        tail_dentries: Vec<ExfatDentry>,
    ) -> ExfatDentrySet {
        ExfatDentrySet::from_trusted_metadata(
            Default::default(),
            ExfatStreamDentry {
                start_cluster: super_block.root_dir,
                size: 0,
                ..Default::default()
            },
            raw_name_units,
            tail_dentries,
        )
        .unwrap()
    }

    fn bitmap_dentry(start_cluster: u32, size: u64) -> ExfatDentry {
        ExfatDentry::Bitmap(ExfatBitmapDentry {
            dentry_type: 0x81,
            flags: 0,
            reserved: [0; 18],
            start_cluster,
            size,
        })
    }

    fn upcase_dentry(start_cluster: u32, size: u64) -> ExfatDentry {
        ExfatDentry::Upcase(ExfatUpcaseDentry {
            dentry_type: 0x82,
            reserved1: [0; 3],
            checksum: 0,
            reserved2: [0; 12],
            start_cluster,
            size,
        })
    }

    fn volume_label_dentry() -> RawExfatDentry {
        RawExfatDentry {
            dentry_type: 0x83,
            value: [0; 31],
        }
    }

    // Confirms the stream crosses a cluster boundary without reordering or duplicating records.
    #[ktest]
    fn directory_engine_preserves_order_across_cluster_boundary() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 2);
        let cluster_entries = super_block.cluster_size() / DENTRY_SIZE;

        let file_set = ExfatDentrySet::from_trusted_metadata(
            Default::default(),
            ExfatStreamDentry {
                start_cluster: super_block.root_dir,
                size: 0,
                ..Default::default()
            },
            &[b'a' as u16, b'b' as u16, b'c' as u16],
            vec![ExfatDentry::VendorExt(Default::default())],
        )
        .unwrap();
        let file_bytes = file_set.to_le_bytes();
        let file_start = cluster_entries - (file_bytes.len() / DENTRY_SIZE) + 1;

        write_raw_dentry(&disk, &super_block, 0, deleted_dentry());
        write_serialized_bytes(&disk, &super_block, file_start, &file_bytes);
        write_entry_sequence(
            &disk,
            &super_block,
            file_start + file_bytes.len() / DENTRY_SIZE,
            &[upcase_dentry(7, 1234)],
        );
        write_raw_dentry(
            &disk,
            &super_block,
            file_start + file_bytes.len() / DENTRY_SIZE + 1,
            unused_dentry(),
        );

        assert!(matches!(
            engine.next_record().unwrap(),
            Some(DirectoryRecord::File(_))
        ));
        assert!(matches!(
            engine.next_record().unwrap(),
            Some(DirectoryRecord::Singleton(ExfatDentry::Upcase(_)))
        ));
        assert!(engine.next_record().unwrap().is_none());
    }

    // Confirms a valid file record is emitted as a validated `ExfatDentrySet`.
    #[ktest]
    fn directory_engine_emits_validated_file_records() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 1);
        let file_set = ExfatDentrySet::from_trusted_metadata(
            Default::default(),
            ExfatStreamDentry {
                start_cluster: super_block.root_dir,
                size: 4096,
                valid_size: 4096,
                ..Default::default()
            },
            &[b'i' as u16, b'n' as u16, b'o' as u16],
            vec![ExfatDentry::GenericSecondary(Default::default())],
        )
        .unwrap();
        let file_bytes = file_set.to_le_bytes();

        write_serialized_bytes(&disk, &super_block, 0, &file_bytes);
        write_raw_dentry(
            &disk,
            &super_block,
            file_bytes.len() / DENTRY_SIZE,
            unused_dentry(),
        );

        let record = engine.next_record().unwrap().unwrap();
        match record {
            DirectoryRecord::File(actual) => {
                assert!(actual.dentry_set().verify_checksum());
                assert_eq!(
                    actual.raw_name_units(),
                    vec![b'i' as u16, b'n' as u16, b'o' as u16]
                );
            }
            DirectoryRecord::Singleton(_) => panic!("expected validated file record"),
        }
        assert!(engine.next_record().unwrap().is_none());
    }

    // Confirms `Bitmap` and `Upcase` entries are surfaced as raw singleton candidates.
    #[ktest]
    fn directory_engine_surfaces_raw_singletons_without_policy() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 1);

        write_entry_sequence(
            &disk,
            &super_block,
            0,
            &[bitmap_dentry(11, 8192), upcase_dentry(12, 16384)],
        );
        write_raw_dentry(&disk, &super_block, 2, unused_dentry());

        let first = engine.next_record().unwrap().unwrap();
        let second = engine.next_record().unwrap().unwrap();

        match first {
            DirectoryRecord::Singleton(ExfatDentry::Bitmap(bitmap)) => {
                let start_cluster = bitmap.start_cluster;
                let size = bitmap.size;
                assert_eq!(start_cluster, 11);
                assert_eq!(size, 8192);
            }
            _ => panic!("expected bitmap singleton"),
        }

        match second {
            DirectoryRecord::Singleton(ExfatDentry::Upcase(upcase)) => {
                let start_cluster = upcase.start_cluster;
                let size = upcase.size;
                assert_eq!(start_cluster, 12);
                assert_eq!(size, 16384);
            }
            _ => panic!("expected upcase singleton"),
        }
    }

    // Confirms tombstones are skipped and `Unused` terminates the scan.
    #[ktest]
    fn directory_engine_skips_deleted_and_stops_at_unused() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 1);

        write_raw_dentry(&disk, &super_block, 0, deleted_dentry());
        write_entry_sequence(
            &disk,
            &super_block,
            1,
            &[bitmap_dentry(21, 4096), upcase_dentry(22, 4096)],
        );
        write_raw_dentry(&disk, &super_block, 3, unused_dentry());

        let first = engine.next_record().unwrap().unwrap();
        assert!(matches!(
            first,
            DirectoryRecord::Singleton(ExfatDentry::Bitmap(_))
        ));
        assert!(matches!(
            engine.next_record().unwrap(),
            Some(DirectoryRecord::Singleton(ExfatDentry::Upcase(_)))
        ));
        assert!(engine.next_record().unwrap().is_none());
    }

    // Confirms the stream ignores the root volume-label metadata entry while
    // still preserving the explicit singleton candidate boundary.
    #[ktest]
    fn directory_engine_skips_volume_label_and_keeps_system_singletons() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 1);

        write_raw_dentry(&disk, &super_block, 0, volume_label_dentry());
        write_entry_sequence(
            &disk,
            &super_block,
            1,
            &[bitmap_dentry(31, 4096), upcase_dentry(32, 8192)],
        );
        write_raw_dentry(&disk, &super_block, 3, unused_dentry());

        assert!(matches!(
            engine.next_record().unwrap(),
            Some(DirectoryRecord::Singleton(ExfatDentry::Bitmap(_)))
        ));
        assert!(matches!(
            engine.next_record().unwrap(),
            Some(DirectoryRecord::Singleton(ExfatDentry::Upcase(_)))
        ));
        assert!(engine.next_record().unwrap().is_none());
    }

    // Confirms unexpected top-level dentries fail instead of surfacing as generic singletons.
    #[ktest]
    fn directory_engine_rejects_unexpected_top_level_dentry() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 1);

        write_raw_dentry(
            &disk,
            &super_block,
            0,
            RawExfatDentry {
                dentry_type: 0x80,
                value: [0; 31],
            },
        );
        write_raw_dentry(&disk, &super_block, 1, unused_dentry());

        let error = engine.next_record().unwrap_err();
        assert_eq!(error.error(), Errno::EINVAL);
    }

    // Confirms tombstoned slots are reused before the write path asks for growth.
    #[ktest]
    fn directory_engine_reuses_deleted_slots_before_growth() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 1);
        let file_set = validated_file_set(&super_block, &[b'r' as u16], vec![]);
        let slot_count = file_set.to_le_bytes().len() / DENTRY_SIZE;

        for entry_index in 0..slot_count {
            write_raw_dentry(&disk, &super_block, entry_index, deleted_dentry());
        }
        write_raw_dentry(&disk, &super_block, slot_count, unused_dentry());

        let location = engine.place_dentry_set(&file_set, None).unwrap();

        assert_eq!(location.inode_key_parts(), (Some(1), 0, 0));
        assert!(matches!(
            ExfatDentry::from(read_raw_dentry(&disk, &super_block, 0)),
            ExfatDentry::File(_)
        ));
        assert!(matches!(
            ExfatDentry::from(read_raw_dentry(&disk, &super_block, slot_count)),
            ExfatDentry::Unused
        ));
    }

    // Confirms a rewrite stays at the trusted location when the smaller set still fits.
    #[ktest]
    fn directory_engine_preserves_location_when_rewrite_still_fits() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 1);
        let original_set = validated_file_set(
            &super_block,
            &[b'o' as u16, b'r' as u16],
            vec![ExfatDentry::GenericSecondary(Default::default())],
        );
        let smaller_set = validated_file_set(&super_block, &[b'o' as u16], vec![]);
        let slot_count = original_set.to_le_bytes().len() / DENTRY_SIZE;
        let original_slots = slot_count;

        for entry_index in 0..original_slots {
            write_raw_dentry(&disk, &super_block, entry_index, deleted_dentry());
        }
        write_raw_dentry(&disk, &super_block, original_slots, unused_dentry());

        let location = engine
            .place_dentry_set(&original_set, None)
            .expect("initial placement should reuse deleted slots");
        let rewritten_location = engine
            .rewrite_dentry_set(location, &smaller_set, None)
            .unwrap();

        assert_eq!(rewritten_location, location);
        assert!(matches!(
            ExfatDentry::from(read_raw_dentry(&disk, &super_block, original_slots - 1)),
            ExfatDentry::Deleted(_)
        ));
    }

    // Confirms expansion consumes the committed allocation result instead of reopening search.
    #[ktest]
    fn directory_engine_consumes_committed_growth_for_directory_expansion() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 1);
        let file_set = validated_file_set(&super_block, &[b'g' as u16], vec![]);
        let cluster_entries = super_block.cluster_size() / DENTRY_SIZE;
        let growth = committed_growth(&super_block);
        let growth_start_offset = super_block
            .cluster_to_byte_offset(growth.start_cluster)
            .unwrap();

        for entry_index in 0..cluster_entries {
            write_entry_sequence(&disk, &super_block, entry_index, &[bitmap_dentry(11, 8192)]);
        }

        let error = engine.place_dentry_set(&file_set, None).unwrap_err();
        assert_eq!(error.error(), Errno::ENOSPC);

        let location = engine.place_dentry_set(&file_set, Some(growth)).unwrap();

        assert_eq!(
            location.inode_key_parts(),
            (
                Some(1),
                cluster_entries * DENTRY_SIZE,
                cluster_entries as u32
            )
        );
        let mut bytes = [0; DENTRY_SIZE];
        disk.read_bytes(growth_start_offset, &mut bytes);
        assert!(matches!(
            ExfatDentry::from(RawExfatDentry::from_bytes(&bytes)),
            ExfatDentry::File(_)
        ));
        assert!(matches!(
            ExfatDentry::from(read_raw_dentry(&disk, &super_block, 0)),
            ExfatDentry::Bitmap(_)
        ));
    }
}
