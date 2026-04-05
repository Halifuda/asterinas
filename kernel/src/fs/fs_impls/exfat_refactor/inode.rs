// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "The metadata shell is staged before the refactor consumes it."
    )
)]

use alloc::sync::Arc;
use core::convert::TryFrom;

use super::{
    fat::{ChainMode, ClusterId, ExfatChain},
    fileset::ExfatDentrySet,
    read::ExfatInodeReadView,
};
use crate::fs::vfs::page_cache::{PageCache, PageCacheBackend};
use crate::prelude::*;

bitflags! {
    /// exFAT file attribute bits preserved from the validated file record.
    pub(super) struct FatAttr: u16 {
        const READONLY = 0x0001;
        const HIDDEN = 0x0002;
        const SYSTEM = 0x0004;
        const VOLUME = 0x0008;
        const DIRECTORY = 0x0010;
        const ARCHIVE = 0x0020;
    }
}

/// Stores a decoded exFAT timestamp in local metadata form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DosTimestamp {
    pub(super) time: u16,
    pub(super) date: u16,
    pub(super) increment_10ms: u8,
    pub(super) utc_offset: u8,
}

/// Represents a stable exFAT inode identity derived from on-disk location data.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ExfatInodeKey(u64);

/// Stores the validated, read-only inode metadata shell.
///
/// Cross-module query helpers are added only when a downstream component proves that a specific
/// fact needs to cross the module boundary.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct ExfatInodeMeta {
    inode_key: ExfatInodeKey,
    file_attributes: FatAttr,
    created_at: DosTimestamp,
    modified_at: DosTimestamp,
    accessed_at: DosTimestamp,
    valid_data_length: usize,
    data_length: usize,
    chain: ExfatChain,
    raw_name_units: Vec<u16>,
}

/// Owns the regular-file page cache runtime state.
pub(super) struct ExfatRegularFileRuntime {
    page_cache: PageCache,
    // Keeps a strong backend reference alive for `PageCache`.
    backend: Arc<dyn PageCacheBackend>,
}

impl ExfatInodeKey {
    /// Creates the reserved root inode key.
    pub(super) fn root() -> Self {
        Self(0)
    }

    /// Creates a key from a validated cluster id and byte offset within that cluster.
    pub(super) fn from_cluster_and_offset(
        cluster: ClusterId,
        byte_offset_in_cluster: usize,
    ) -> Result<Self> {
        // Preserve the legacy packed `(cluster << 32) | offset` layout without truncating the
        // offset field.
        let byte_offset_in_cluster = u32::try_from(byte_offset_in_cluster).map_err(|_| {
            Error::with_message(Errno::EINVAL, "inode key offset does not fit in 32 bits")
        })?;

        Ok(Self(
            (u64::from(cluster) << 32) | u64::from(byte_offset_in_cluster),
        ))
    }
}

impl ExfatInodeMeta {
    /// Creates a metadata shell from validated file-record facts.
    pub(super) fn new(
        inode_key: ExfatInodeKey,
        file_record: &ExfatDentrySet,
        chain: ExfatChain,
    ) -> Result<Self> {
        if inode_key == ExfatInodeKey::root() {
            return Err(Error::with_message(
                Errno::EINVAL,
                "root inode must use the synthetic root constructor",
            ));
        }

        let file_dentry = file_record.file_dentry();
        let stream_dentry = file_record.stream_dentry();
        let file_attributes = FatAttr::from_bits_truncate(file_dentry.attribute);
        let valid_data_length = usize::try_from(stream_dentry.valid_size).map_err(|_| {
            Error::with_message(Errno::EINVAL, "valid data length does not fit in usize")
        })?;
        let data_length = usize::try_from(stream_dentry.size)
            .map_err(|_| Error::with_message(Errno::EINVAL, "data length does not fit in usize"))?;

        if file_attributes.contains(FatAttr::DIRECTORY) && valid_data_length != data_length {
            return Err(Error::with_message(
                Errno::EINVAL,
                "directory metadata must keep valid and allocated lengths equal",
            ));
        }

        Ok(Self {
            inode_key,
            file_attributes,
            created_at: DosTimestamp {
                time: file_dentry.create_time,
                date: file_dentry.create_date,
                increment_10ms: file_dentry.create_time_cs,
                utc_offset: file_dentry.create_utc_offset,
            },
            modified_at: DosTimestamp {
                time: file_dentry.modify_time,
                date: file_dentry.modify_date,
                increment_10ms: file_dentry.modify_time_cs,
                utc_offset: file_dentry.modify_utc_offset,
            },
            accessed_at: DosTimestamp {
                time: file_dentry.access_time,
                date: file_dentry.access_date,
                increment_10ms: 0,
                utc_offset: file_dentry.access_utc_offset,
            },
            valid_data_length,
            data_length,
            chain,
            // Preserve the validated logical UTF-16 name units exactly as exposed by the
            // file-record boundary.
            raw_name_units: file_record.raw_name_units(),
        })
    }

    /// Creates the explicit synthetic root metadata shell.
    pub(super) fn new_root(
        inode_key: ExfatInodeKey,
        chain: ExfatChain,
        valid_data_length: usize,
        data_length: usize,
        created_at: DosTimestamp,
        modified_at: DosTimestamp,
        accessed_at: DosTimestamp,
    ) -> Result<Self> {
        if inode_key != ExfatInodeKey::root() {
            return Err(Error::with_message(
                Errno::EINVAL,
                "synthetic root metadata must use the reserved root key",
            ));
        }
        if valid_data_length != data_length {
            return Err(Error::with_message(
                Errno::EINVAL,
                "synthetic root metadata must keep valid and allocated lengths equal",
            ));
        }

        Ok(Self {
            inode_key,
            file_attributes: FatAttr::DIRECTORY,
            created_at,
            modified_at,
            accessed_at,
            valid_data_length,
            data_length,
            chain,
            raw_name_units: Vec::new(),
        })
    }

    /// Returns the immutable read-mapping facts for an existing regular file.
    pub(super) fn read_view(&self) -> Result<ExfatInodeReadView<'_>> {
        let valid_data_length = self.regular_file_valid_data_length()?;

        Ok(ExfatInodeReadView::new(&self.chain, valid_data_length))
    }

    /// Returns the regular-file visible length used by read and cache boundaries.
    pub(super) fn regular_file_valid_data_length(&self) -> Result<usize> {
        if self.file_attributes.contains(FatAttr::DIRECTORY) {
            return Err(Error::with_message(
                Errno::EISDIR,
                "directory metadata cannot cross the read-mapping boundary",
            ));
        }

        Ok(self.valid_data_length)
    }

    /// Returns the backend-visible page count derived from `valid_data_length`.
    pub(super) fn regular_file_page_count(&self) -> Result<usize> {
        Ok(self.regular_file_valid_data_length()?.div_ceil(PAGE_SIZE))
    }

    /// Returns the initial page-cache capacity derived from `valid_data_length`.
    pub(super) fn regular_file_cache_capacity(&self) -> Result<usize> {
        self.regular_file_page_count()?
            .checked_mul(PAGE_SIZE)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "page cache capacity overflow"))
    }
}

impl ExfatRegularFileRuntime {
    /// Creates a regular-file runtime that owns `PageCache` and backend lifetime.
    pub(super) fn new(page_cache: PageCache, backend: Arc<dyn PageCacheBackend>) -> Self {
        Self { page_cache, backend }
    }

    /// Returns the owned page cache for this regular-file runtime.
    pub(super) fn page_cache(&self) -> &PageCache {
        &self.page_cache
    }

    /// Returns the backend-visible page count for this runtime.
    pub(super) fn backend_page_count(&self) -> usize {
        self.backend.npages()
    }
}

#[cfg(ktest)]
mod tests {
    use ostd::prelude::ktest;

    use super::*;
    use crate::fs::fs_impls::exfat_refactor::{
        boot_sector::read_primary_super_block,
        dentry::{ExfatFileDentry, ExfatStreamDentry},
        test_support::load_exfat_disk,
    };

    fn sample_chain() -> (ExfatChain, ClusterId) {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let root_cluster = super_block.root_dir;

        (
            ExfatChain::new(
                &disk,
                &super_block,
                root_cluster,
                Some(1),
                ChainMode::Contiguous,
            )
            .unwrap(),
            root_cluster,
        )
    }

    fn sample_file_record(
        attribute: u16,
        valid_data_length: u64,
        data_length: u64,
    ) -> ExfatDentrySet {
        ExfatDentrySet::from_trusted_metadata(
            ExfatFileDentry {
                dentry_type: 0x85,
                num_secondary: 0,
                checksum: 0,
                attribute,
                reserved1: 0,
                create_time: 0x1234,
                create_date: 0x5678,
                modify_time: 0x9abc,
                modify_date: 0xdef0,
                access_time: 0x1357,
                access_date: 0x2468,
                create_time_cs: 0x2a,
                modify_time_cs: 0x33,
                create_utc_offset: 0x44,
                modify_utc_offset: 0x55,
                access_utc_offset: 0x66,
                reserved2: [0; 7],
            },
            ExfatStreamDentry {
                dentry_type: 0xC0,
                flags: 0,
                reserved1: 0,
                name_len: 0,
                name_hash: 0,
                reserved2: 0,
                valid_size: valid_data_length,
                reserved3: 0,
                start_cluster: 2,
                size: data_length,
            },
            &[0x0041, 0x0042, 0x0043, 0x0044],
            vec![],
        )
        .unwrap()
    }

    #[ktest]
    fn inode_key_preserves_packed_location_layout() {
        // Stable packing keeps the legacy cluster-plus-offset layout intact.
        let cluster = 0x1234_5678;
        let byte_offset_in_cluster = 0x9abc_def0usize;

        let key = ExfatInodeKey::from_cluster_and_offset(cluster, byte_offset_in_cluster)
            .expect("valid packed location should produce a key");
        let repeated_key = ExfatInodeKey::from_cluster_and_offset(cluster, byte_offset_in_cluster)
            .expect("repeating the same packed location should stay stable");

        assert_eq!(
            key.0,
            (u64::from(cluster) << 32) | u64::from(byte_offset_in_cluster as u32)
        );
        assert_eq!(key, repeated_key);
    }

    #[ktest]
    fn root_inode_key_is_reserved() {
        // Root stays a dedicated reserved key rather than a packed location.
        let root_key = ExfatInodeKey::root();
        let ordinary_key = ExfatInodeKey::from_cluster_and_offset(2, 0x40)
            .expect("ordinary location should produce a packed key");

        assert_eq!(root_key.0, 0);
        assert_ne!(root_key, ordinary_key);
    }

    #[ktest]
    fn inode_key_rejects_offset_overflow() {
        // Oversized offsets must fail instead of truncating into a different key.
        let overflow_offset = (u32::MAX as usize) + 1;

        assert!(ExfatInodeKey::from_cluster_and_offset(1, overflow_offset).is_err());
    }

    #[ktest]
    fn inode_meta_preserves_validated_file_record_facts() {
        // Ordinary construction preserves the validated file-record and chain facts verbatim.
        let (chain, start_cluster) = sample_chain();
        let inode_key = ExfatInodeKey::from_cluster_and_offset(start_cluster, 0x40).unwrap();
        let file_record = sample_file_record(0x0020, 0x1234, 0x1234);

        let inode_meta = ExfatInodeMeta::new(inode_key, &file_record, chain).unwrap();

        assert_eq!(inode_meta.inode_key, inode_key);
        assert_ne!(inode_meta.inode_key, ExfatInodeKey::root());
        assert!(!inode_meta.file_attributes.contains(FatAttr::DIRECTORY));
        assert_eq!(inode_meta.file_attributes, FatAttr::ARCHIVE);
        assert_eq!(
            inode_meta.created_at,
            DosTimestamp {
                time: 0x1234,
                date: 0x5678,
                increment_10ms: 0x2a,
                utc_offset: 0x44,
            }
        );
        assert_eq!(
            inode_meta.modified_at,
            DosTimestamp {
                time: 0x9abc,
                date: 0xdef0,
                increment_10ms: 0x33,
                utc_offset: 0x55,
            }
        );
        assert_eq!(
            inode_meta.accessed_at,
            DosTimestamp {
                time: 0x1357,
                date: 0x2468,
                increment_10ms: 0,
                utc_offset: 0x66,
            }
        );
        assert_eq!(inode_meta.valid_data_length, 0x1234);
        assert_eq!(inode_meta.data_length, 0x1234);
        assert_eq!(inode_meta.chain, chain);
        assert_eq!(inode_meta.raw_name_units, file_record.raw_name_units());
    }

    #[ktest]
    fn root_inode_meta_uses_explicit_synthetic_constructor() {
        // Confirms root construction stays on the explicit synthetic path and keeps root reserved.
        let (chain, start_cluster) = sample_chain();
        let root_key = ExfatInodeKey::root();
        let non_root_key = ExfatInodeKey::from_cluster_and_offset(start_cluster, 0x80).unwrap();
        let created_at = DosTimestamp {
            time: 0x0101,
            date: 0x0202,
            increment_10ms: 0x03,
            utc_offset: 0x04,
        };
        let modified_at = DosTimestamp {
            time: 0x0505,
            date: 0x0606,
            increment_10ms: 0x07,
            utc_offset: 0x08,
        };
        let accessed_at = DosTimestamp {
            time: 0x0909,
            date: 0x0a0a,
            increment_10ms: 0x00,
            utc_offset: 0x0b,
        };

        let root_meta = ExfatInodeMeta::new_root(
            root_key,
            chain,
            0x2000,
            0x2000,
            created_at,
            modified_at,
            accessed_at,
        )
        .unwrap();

        assert_eq!(root_meta.inode_key, root_key);
        assert!(root_meta.file_attributes.contains(FatAttr::DIRECTORY));
        assert_eq!(root_meta.file_attributes, FatAttr::DIRECTORY);
        assert_eq!(root_meta.created_at, created_at);
        assert_eq!(root_meta.modified_at, modified_at);
        assert_eq!(root_meta.accessed_at, accessed_at);
        assert_eq!(root_meta.valid_data_length, 0x2000);
        assert_eq!(root_meta.data_length, 0x2000);
        assert_eq!(root_meta.chain, chain);
        assert!(root_meta.raw_name_units.is_empty());
        assert!(
            ExfatInodeMeta::new_root(
                non_root_key,
                chain,
                0x2000,
                0x2000,
                created_at,
                modified_at,
                accessed_at,
            )
            .is_err()
        );
        assert!(
            ExfatInodeMeta::new(root_key, &sample_file_record(0x0020, 0x20, 0x20), chain).is_err()
        );
    }

    #[ktest]
    fn inode_meta_rejects_directory_length_mismatch() {
        // Confirms directory shells reject mismatched valid and allocated lengths instead of fixing them.
        let (chain, start_cluster) = sample_chain();
        let inode_key = ExfatInodeKey::from_cluster_and_offset(start_cluster, 0xC0).unwrap();
        let directory_record = sample_file_record(0x0010, 0x1200, 0x1400);

        assert!(ExfatInodeMeta::new(inode_key, &directory_record, chain).is_err());
    }

    #[ktest]
    fn root_inode_meta_rejects_synthetic_length_mismatch() {
        // Synthetic root metadata must reject mismatched logical and allocated lengths.
        let (chain, _) = sample_chain();
        let root_key = ExfatInodeKey::root();
        let timestamp = DosTimestamp {
            time: 0x1010,
            date: 0x2020,
            increment_10ms: 0x30,
            utc_offset: 0x40,
        };

        assert!(
            ExfatInodeMeta::new_root(
                root_key, chain, 0x2000, 0x2400, timestamp, timestamp, timestamp
            )
            .is_err()
        );
    }
}
