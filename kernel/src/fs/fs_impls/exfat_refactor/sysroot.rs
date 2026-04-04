// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "Root-system-entry discovery is staged before the later loaders consume it."
    )
)]

use core::convert::TryFrom;

use aster_block::BlockDevice;

use super::{
    dentry::{
        ExfatDentry, ExfatBitmapDentry, ExfatUpcaseDentry, DENTRY_SIZE, RawExfatDentry,
    },
    fat::ExfatChain,
    io::read_metadata_bytes,
    super_block::ExfatSuperBlock,
};
use crate::prelude::*;

/// Stores the opaque location token for a discovered root-directory system entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ExfatSysRootEntryLocation(u64);

/// Stores the discovered bitmap entry facts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ExfatSysRootBitmapDiscovery {
    pub(super) location: ExfatSysRootEntryLocation,
    pub(super) start_cluster: u32,
    pub(super) byte_size: usize,
}

/// Stores the discovered upcase entry facts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ExfatSysRootUpcaseDiscovery {
    pub(super) location: ExfatSysRootEntryLocation,
    pub(super) start_cluster: u32,
    pub(super) byte_size: usize,
    pub(super) checksum: u32,
}

/// Stores the read-only discovery aggregate for the root system entries.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct ExfatSysRootFacts {
    pub(super) bitmap: Option<ExfatSysRootBitmapDiscovery>,
    pub(super) upcase: Option<ExfatSysRootUpcaseDiscovery>,
}

impl ExfatSysRootEntryLocation {
    fn from_byte_offset(byte_offset: usize) -> Result<Self> {
        let byte_offset = u64::try_from(byte_offset).map_err(|_| {
            Error::with_message(Errno::EINVAL, "root entry location does not fit in 64 bits")
        })?;

        Ok(Self(byte_offset))
    }
}

impl ExfatSysRootBitmapDiscovery {
    fn try_new(
        super_block: &ExfatSuperBlock,
        location: ExfatSysRootEntryLocation,
        bitmap: ExfatBitmapDentry,
    ) -> Result<Self> {
        if !super_block.is_data_cluster_id(bitmap.start_cluster) {
            return Err(Error::with_message(
                Errno::EINVAL,
                "root bitmap entry has an invalid start cluster",
            ));
        }

        let byte_size = usize::try_from(bitmap.size).map_err(|_| {
            Error::with_message(Errno::EINVAL, "root bitmap size does not fit in usize")
        })?;

        Ok(Self {
            location,
            start_cluster: bitmap.start_cluster,
            byte_size,
        })
    }
}

impl ExfatSysRootUpcaseDiscovery {
    fn try_new(
        super_block: &ExfatSuperBlock,
        location: ExfatSysRootEntryLocation,
        upcase: ExfatUpcaseDentry,
    ) -> Result<Self> {
        if !super_block.is_data_cluster_id(upcase.start_cluster) {
            return Err(Error::with_message(
                Errno::EINVAL,
                "root upcase entry has an invalid start cluster",
            ));
        }

        let byte_size = usize::try_from(upcase.size).map_err(|_| {
            Error::with_message(Errno::EINVAL, "root upcase size does not fit in usize")
        })?;

        Ok(Self {
            location,
            start_cluster: upcase.start_cluster,
            byte_size,
            checksum: upcase.checksum,
        })
    }
}

impl ExfatSysRootFacts {
    fn record_bitmap(&mut self, bitmap: ExfatSysRootBitmapDiscovery) -> Result<()> {
        if self.bitmap.is_some() {
            return Err(Error::with_message(
                Errno::EINVAL,
                "duplicate root bitmap entry",
            ));
        }

        self.bitmap = Some(bitmap);
        Ok(())
    }

    fn record_upcase(&mut self, upcase: ExfatSysRootUpcaseDiscovery) -> Result<()> {
        if self.upcase.is_some() {
            return Err(Error::with_message(
                Errno::EINVAL,
                "duplicate root upcase entry",
            ));
        }

        self.upcase = Some(upcase);
        Ok(())
    }

    fn finish(self) -> Result<Self> {
        if self.bitmap.is_none() {
            return Err(Error::with_message(
                Errno::EINVAL,
                "missing root bitmap entry",
            ));
        }
        if self.upcase.is_none() {
            return Err(Error::with_message(
                Errno::EINVAL,
                "missing root upcase entry",
            ));
        }

        Ok(self)
    }
}

/// Scans the root directory for the validated `BITMAP` and `UPCASE` system entries.
pub(super) fn scan_root_system_entries(
    block_device: &dyn BlockDevice,
    super_block: &ExfatSuperBlock,
    root_chain: ExfatChain,
) -> Result<ExfatSysRootFacts> {
    if root_chain.is_empty() {
        return Err(Error::with_message(
            Errno::EINVAL,
            "root directory chain must not be empty",
        ));
    }

    let entries_per_cluster = super_block.cluster_size() / DENTRY_SIZE;
    if entries_per_cluster == 0 {
        return Err(Error::with_message(
            Errno::EINVAL,
            "root directory cluster is too small for a dentry",
        ));
    }

    let mut facts = ExfatSysRootFacts::default();
    let mut cluster_chain = root_chain;
    let mut cluster_start_offset = cluster_chain.physical_cluster_start_offset(super_block)?;
    let mut slot_index = 0usize;
    let mut pending_skip = 0usize;

    loop {
        let entry_offset = cluster_start_offset
            .checked_add(slot_index.checked_mul(DENTRY_SIZE).ok_or_else(|| {
                Error::with_message(Errno::EINVAL, "root entry offset overflow")
            })?)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "root entry offset overflow"))?;

        let dentry = read_root_dentry(block_device, entry_offset)?;
        if pending_skip > 0 {
            if !is_skip_entry(&dentry) {
                return Err(Error::with_message(
                    Errno::EINVAL,
                    "malformed root directory record",
                ));
            }

            pending_skip -= 1;
        } else {
            match dentry {
                ExfatDentry::Unused => break,
                ExfatDentry::Deleted(_) => {}
                ExfatDentry::Bitmap(bitmap) => {
                    let discovery = ExfatSysRootBitmapDiscovery::try_new(
                        super_block,
                        ExfatSysRootEntryLocation::from_byte_offset(entry_offset)?,
                        bitmap,
                    )?;
                    facts.record_bitmap(discovery)?;
                }
                ExfatDentry::Upcase(upcase) => {
                    let discovery = ExfatSysRootUpcaseDiscovery::try_new(
                        super_block,
                        ExfatSysRootEntryLocation::from_byte_offset(entry_offset)?,
                        upcase,
                    )?;
                    facts.record_upcase(discovery)?;
                }
                ExfatDentry::File(file) => {
                    pending_skip = usize::from(file.num_secondary);
                }
                ExfatDentry::GenericPrimary(primary) => {
                    pending_skip = usize::from(primary.secondary_count);
                }
                ExfatDentry::Stream(_)
                | ExfatDentry::Name(_)
                | ExfatDentry::GenericSecondary(_)
                | ExfatDentry::VendorExt(_)
                | ExfatDentry::VendorAlloc(_) => {
                    return Err(Error::with_message(
                        Errno::EINVAL,
                        "unexpected root directory secondary entry",
                    ));
                }
            }
        }

        if !advance_root_entry_position(
            block_device,
            super_block,
            &mut cluster_chain,
            &mut cluster_start_offset,
            &mut slot_index,
            entries_per_cluster,
        )? {
            break;
        }
    }

    if pending_skip > 0 {
        return Err(Error::with_message(
            Errno::EINVAL,
            "truncated root directory record",
        ));
    }

    facts.finish()
}

fn read_root_dentry(block_device: &dyn BlockDevice, byte_offset: usize) -> Result<ExfatDentry> {
    let mut raw_bytes = [0u8; DENTRY_SIZE];
    read_metadata_bytes(block_device, byte_offset, &mut raw_bytes)?;

    Ok(ExfatDentry::from(RawExfatDentry::from_bytes(&raw_bytes)))
}

fn is_skip_entry(dentry: &ExfatDentry) -> bool {
    matches!(
        dentry,
        ExfatDentry::Stream(_)
            | ExfatDentry::Name(_)
            | ExfatDentry::GenericSecondary(_)
            | ExfatDentry::VendorExt(_)
            | ExfatDentry::VendorAlloc(_)
    )
}

fn advance_root_entry_position(
    block_device: &dyn BlockDevice,
    super_block: &ExfatSuperBlock,
    cluster_chain: &mut ExfatChain,
    cluster_start_offset: &mut usize,
    slot_index: &mut usize,
    entries_per_cluster: usize,
) -> Result<bool> {
    *slot_index += 1;
    if *slot_index < entries_per_cluster {
        return Ok(true);
    }

    let next_cluster_chain = match cluster_chain.walk(block_device, super_block, 1) {
        Ok(chain) => chain,
        Err(error) if error.error() == Errno::EINVAL => return Ok(false),
        Err(error) => return Err(error),
    };

    *cluster_chain = next_cluster_chain;
    *cluster_start_offset = cluster_chain.physical_cluster_start_offset(super_block)?;
    *slot_index = 0;
    Ok(true)
}

#[cfg(ktest)]
mod tests {
    use ostd::prelude::ktest;

    use super::*;
    use crate::fs::fs_impls::exfat_refactor::{
        boot_sector::read_primary_super_block,
        dentry::{
            ExfatBitmapDentry, ExfatDentry, ExfatFileDentry, ExfatNameDentry, ExfatStreamDentry,
            ExfatUpcaseDentry,
        },
        fat::{ChainMode, ExfatChain},
        test_support::{load_exfat_disk, ExfatMemoryDisk},
    };

    fn root_scan_context() -> (ExfatMemoryDisk, ExfatSuperBlock, ExfatChain, usize) {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let root_offset = super_block
            .cluster_to_byte_offset(super_block.root_dir)
            .unwrap();
        let root_chain = ExfatChain::new(
            &disk,
            &super_block,
            super_block.root_dir,
            Some(1),
            ChainMode::Contiguous,
        )
        .unwrap();

        (disk, super_block, root_chain, root_offset)
    }

    fn entry_offset(root_offset: usize, entry_index: usize) -> usize {
        root_offset
            .checked_add(
                entry_index
                    .checked_mul(DENTRY_SIZE)
                    .expect("entry index should fit in a byte offset"),
            )
            .expect("entry offset should fit in a byte offset")
    }

    fn write_root_entry(
        disk: &ExfatMemoryDisk,
        root_offset: usize,
        entry_index: usize,
        entry: &ExfatDentry,
    ) {
        disk.write_bytes(entry_offset(root_offset, entry_index), entry.as_bytes());
    }

    fn file_primary(num_secondary: u8) -> ExfatDentry {
        ExfatDentry::File(ExfatFileDentry {
            dentry_type: 0x85,
            num_secondary,
            checksum: 0,
            attribute: 0,
            reserved1: 0,
            create_time: 0,
            create_date: 0,
            modify_time: 0,
            modify_date: 0,
            access_time: 0,
            access_date: 0,
            create_time_cs: 0,
            modify_time_cs: 0,
            create_utc_offset: 0,
            modify_utc_offset: 0,
            access_utc_offset: 0,
            reserved2: [0; 7],
        })
    }

    fn stream_secondary() -> ExfatDentry {
        ExfatDentry::Stream(ExfatStreamDentry {
            dentry_type: 0xC0,
            flags: 0,
            reserved1: 0,
            name_len: 0,
            name_hash: 0,
            reserved2: 0,
            valid_size: 0,
            reserved3: 0,
            start_cluster: 0,
            size: 0,
        })
    }

    fn name_secondary() -> ExfatDentry {
        ExfatDentry::Name(ExfatNameDentry {
            dentry_type: 0xC1,
            flags: 0,
            unicode_0_14: [0; 15],
        })
    }

    fn bitmap_entry(start_cluster: u32, size: u64) -> ExfatDentry {
        ExfatDentry::Bitmap(ExfatBitmapDentry {
            dentry_type: 0x81,
            flags: 0,
            reserved: [0; 18],
            start_cluster,
            size,
        })
    }

    fn upcase_entry(start_cluster: u32, size: u64, checksum: u32) -> ExfatDentry {
        ExfatDentry::Upcase(ExfatUpcaseDentry {
            dentry_type: 0x82,
            reserved1: [0; 3],
            checksum,
            reserved2: [0; 12],
            start_cluster,
            size,
        })
    }

    fn unused_entry() -> ExfatDentry {
        ExfatDentry::Unused
    }

    // Confirms the scanner can step over unrelated file records and preserve both root facts.
    #[ktest]
    fn mixed_root_discovery_preserves_bitmap_and_upcase_facts() {
        let (disk, super_block, root_chain, root_offset) = root_scan_context();
        let bitmap_cluster = super_block.root_dir;
        let upcase_cluster = super_block.root_dir;
        let bitmap_size = 0x600;
        let upcase_size = 0x800;
        let upcase_checksum = 0x1357_9BDF;

        write_root_entry(&disk, root_offset, 0, &file_primary(1));
        write_root_entry(&disk, root_offset, 1, &name_secondary());
        write_root_entry(&disk, root_offset, 2, &bitmap_entry(bitmap_cluster, bitmap_size));
        write_root_entry(
            &disk,
            root_offset,
            3,
            &upcase_entry(upcase_cluster, upcase_size, upcase_checksum),
        );
        write_root_entry(&disk, root_offset, 4, &unused_entry());

        let facts = scan_root_system_entries(&disk, &super_block, root_chain).unwrap();
        let bitmap = facts.bitmap.as_ref().unwrap();
        let upcase = facts.upcase.as_ref().unwrap();

        assert_eq!(bitmap.location.0, entry_offset(root_offset, 2) as u64);
        assert_eq!(bitmap.start_cluster, bitmap_cluster);
        assert_eq!(bitmap.byte_size, bitmap_size as usize);
        assert_eq!(upcase.location.0, entry_offset(root_offset, 3) as u64);
        assert_eq!(upcase.start_cluster, upcase_cluster);
        assert_eq!(upcase.byte_size, upcase_size as usize);
        assert_eq!(upcase.checksum, upcase_checksum);
    }

    // Confirms the scanner rejects duplicate bitmap discovery instead of silently picking one.
    #[ktest]
    fn duplicate_root_bitmap_entry_is_rejected() {
        let (disk, super_block, root_chain, root_offset) = root_scan_context();
        let bitmap_cluster = super_block.root_dir;
        let upcase_cluster = super_block.root_dir;

        write_root_entry(&disk, root_offset, 0, &file_primary(1));
        write_root_entry(&disk, root_offset, 1, &name_secondary());
        write_root_entry(&disk, root_offset, 2, &bitmap_entry(bitmap_cluster, 0x600));
        write_root_entry(&disk, root_offset, 3, &bitmap_entry(bitmap_cluster, 0x700));
        write_root_entry(&disk, root_offset, 4, &upcase_entry(upcase_cluster, 0x800, 0x1));

        let error = scan_root_system_entries(&disk, &super_block, root_chain).unwrap_err();

        assert_eq!(error.error(), Errno::EINVAL);
    }

    // Confirms the scanner rejects a fixture that omits the bitmap discovery fact.
    #[ktest]
    fn missing_root_bitmap_entry_is_rejected() {
        let (disk, super_block, root_chain, root_offset) = root_scan_context();
        let upcase_cluster = super_block.root_dir;

        write_root_entry(&disk, root_offset, 0, &file_primary(1));
        write_root_entry(&disk, root_offset, 1, &name_secondary());
        write_root_entry(&disk, root_offset, 2, &upcase_entry(upcase_cluster, 0x800, 0x2));
        write_root_entry(&disk, root_offset, 3, &unused_entry());

        let error = scan_root_system_entries(&disk, &super_block, root_chain).unwrap_err();

        assert_eq!(error.error(), Errno::EINVAL);
    }

    // Confirms the scanner rejects a fixture that omits the upcase discovery fact.
    #[ktest]
    fn missing_root_upcase_entry_is_rejected() {
        let (disk, super_block, root_chain, root_offset) = root_scan_context();
        let bitmap_cluster = super_block.root_dir;

        write_root_entry(&disk, root_offset, 0, &file_primary(1));
        write_root_entry(&disk, root_offset, 1, &name_secondary());
        write_root_entry(&disk, root_offset, 2, &bitmap_entry(bitmap_cluster, 0x600));
        write_root_entry(&disk, root_offset, 3, &unused_entry());

        let error = scan_root_system_entries(&disk, &super_block, root_chain).unwrap_err();

        assert_eq!(error.error(), Errno::EINVAL);
    }

    // Confirms the scanner rejects illegal root metadata before it can become a discovery fact.
    #[ktest]
    fn malformed_root_bitmap_entry_is_rejected() {
        let (disk, super_block, root_chain, root_offset) = root_scan_context();

        write_root_entry(&disk, root_offset, 0, &bitmap_entry(1, 0x600));

        let error = scan_root_system_entries(&disk, &super_block, root_chain).unwrap_err();

        assert_eq!(error.error(), Errno::EINVAL);
    }

    // Confirms the scanner does not reinterpret a stray secondary entry as a valid root record.
    #[ktest]
    fn wrong_kind_root_entry_is_rejected() {
        let (disk, super_block, root_chain, root_offset) = root_scan_context();

        write_root_entry(&disk, root_offset, 0, &stream_secondary());

        let error = scan_root_system_entries(&disk, &super_block, root_chain).unwrap_err();

        assert_eq!(error.error(), Errno::EINVAL);
    }

    // Confirms a primary record that runs off the end of the root cluster is reported as truncated.
    #[ktest]
    fn truncated_root_directory_record_is_rejected() {
        let (disk, super_block, root_chain, root_offset) = root_scan_context();
        let entries_per_cluster = super_block.cluster_size() / DENTRY_SIZE;
        let last_entry_index = entries_per_cluster - 1;

        write_root_entry(&disk, root_offset, last_entry_index, &file_primary(1));

        let error = scan_root_system_entries(&disk, &super_block, root_chain).unwrap_err();

        assert_eq!(error.error(), Errno::EINVAL);
    }
}
