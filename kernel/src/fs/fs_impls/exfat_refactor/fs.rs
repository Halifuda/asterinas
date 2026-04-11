// SPDX-License-Identifier: MPL-2.0
#![expect(
    dead_code,
    reason = "Filesystem owner is staged before mount integration."
)]

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use core::time::Duration;

use aster_block::BlockDevice;

use super::{
    bitmap::AllocationBitmap,
    boot_sector::BOOT_SIGNATURE,
    dentry::{ExfatBitmapDentry, ExfatDentry, ExfatUpcaseDentry},
    directory::{DirectoryEngine, DirectoryRecord},
    fat::{ChainMode, ExfatChain},
    fileset::ExfatDentrySet,
    inode::{ExfatInode, ExfatInodeLocation},
    io::read_metadata_bytes,
    super_block::ExfatSuperBlock,
};
use crate::{
    fs::{
        file::{InodeMode, InodeType},
        vfs::{
            file_system::{FileSystem, FsEventSubscriberStats, SuperBlock},
            inode::{Inode, Metadata},
        },
    },
    prelude::*,
    process::{Gid, Uid},
};

const EXFAT_FS_NAME: &str = "exfat";
const EXFAT_NAME_MAX: usize = 255;
const UPCASE_TABLE_UNIT_COUNT: usize = 0x1_0000;
const UPCASE_TABLE_IDENTITY_RUN_MARKER: u16 = 0xFFFF;

/// Carries the owner-private opened-inode publication boundary for `ExfatFs`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct InodeKey {
    parent_ino: Option<u64>,
    dentry_set_byte_offset: usize,
    dentry_entry_index: u32,
}

impl InodeKey {
    /// Creates an opened-inode identity from trusted directory-location facts.
    pub(super) fn new(
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
}

#[derive(Default)]
struct UpcaseState {
    table: Option<Arc<UpcaseTable>>,
}

impl UpcaseState {
    fn publish_table(&mut self, table: Arc<UpcaseTable>) -> Result<()> {
        if self.table.is_some() {
            return Err(Error::with_message(
                Errno::EINVAL,
                "upcase table is already installed",
            ));
        }

        self.table = Some(table);
        Ok(())
    }

    fn table(&self) -> Result<Arc<UpcaseTable>> {
        self.table.clone().ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "upcase table has not been installed")
        })
    }
}

#[derive(Debug)]
struct UpcaseTable {
    upcase_units: Box<[u16]>,
}

impl UpcaseTable {
    fn validate_and_decode(
        upcase_dentry: ExfatUpcaseDentry,
        raw_table_bytes: &[u8],
    ) -> Result<Self> {
        let expected_size = usize::try_from(upcase_dentry.size).map_err(|_| {
            Error::with_message(Errno::EINVAL, "upcase table size does not fit the host")
        })?;
        if raw_table_bytes.len() != expected_size {
            return Err(Error::with_message(
                Errno::EINVAL,
                "upcase table size mismatched",
            ));
        }

        if table_checksum(raw_table_bytes) != upcase_dentry.checksum {
            return Err(Error::with_message(
                Errno::EINVAL,
                "upcase table checksum mismatched",
            ));
        }

        let decoded_units = decode_upcase_units(raw_table_bytes)?;
        validate_mandatory_prefix(&decoded_units)?;

        Ok(Self {
            upcase_units: decoded_units.into_boxed_slice(),
        })
    }

    fn fold_utf16(&self, utf16_units: &[u16]) -> Vec<u16> {
        utf16_units
            .iter()
            .map(|unit| {
                self.upcase_units
                    .get(usize::from(*unit))
                    .copied()
                    .unwrap_or(*unit)
            })
            .collect()
    }
}

#[derive(Default)]
struct OpenedInodeState {
    opened_inodes: BTreeMap<InodeKey, Arc<ExfatInode>>,
    root_inode: Option<Arc<ExfatInode>>,
}

impl OpenedInodeState {
    fn lookup_opened_inode(&self, key: &InodeKey) -> Option<Arc<ExfatInode>> {
        self.opened_inodes.get(key).cloned()
    }

    fn publish_opened_inode(&mut self, key: InodeKey, inode: Arc<ExfatInode>) -> Arc<ExfatInode> {
        self.opened_inodes.entry(key).or_insert(inode).clone()
    }

    fn remove_opened_inode(&mut self, key: &InodeKey) -> Option<Arc<ExfatInode>> {
        self.opened_inodes.remove(key)
    }

    fn publish_root_inode(&mut self, inode: Arc<ExfatInode>) -> Arc<ExfatInode> {
        if let Some(root_inode) = &self.root_inode {
            return root_inode.clone();
        }

        self.root_inode = Some(inode);
        self.root_inode
            .as_ref()
            .expect("root inode was just published")
            .clone()
    }

    fn root_inode(&self) -> Option<Arc<ExfatInode>> {
        self.root_inode.clone()
    }
}

pub(super) struct ExfatFs {
    block_device: Arc<dyn BlockDevice>,
    super_block: ExfatSuperBlock,
    vfs_super_block: SuperBlock,
    fs_event_subscriber_stats: FsEventSubscriberStats,
    mount_open_state: Mutex<()>,
    upcase_state: Mutex<UpcaseState>,
    allocation_bitmap: Mutex<Option<AllocationBitmap>>,
    opened_inode_state: Mutex<OpenedInodeState>,
}

impl ExfatFs {
    pub(super) fn new(
        block_device: Arc<dyn BlockDevice>,
        super_block: ExfatSuperBlock,
    ) -> Result<Self> {
        let sector_count = usize::try_from(super_block.num_sectors).map_err(|_| {
            Error::with_message(
                Errno::EINVAL,
                "exFAT sector count does not fit VFS super block",
            )
        })?;
        let mut vfs_super_block = SuperBlock::new(
            u64::from(BOOT_SIGNATURE),
            super_block.sector_size(),
            EXFAT_NAME_MAX,
            block_device.id(),
        );
        vfs_super_block.blocks = sector_count;

        Ok(Self {
            block_device,
            super_block,
            vfs_super_block,
            fs_event_subscriber_stats: FsEventSubscriberStats::new(),
            mount_open_state: Mutex::new(()),
            upcase_state: Mutex::new(UpcaseState::default()),
            allocation_bitmap: Mutex::new(None),
            opened_inode_state: Mutex::new(OpenedInodeState::default()),
        })
    }

    /// Opens the mounted root directory, installing mount prerequisites first.
    pub(super) fn open_root_inode(fs: &Arc<Self>) -> Result<Arc<dyn Inode>> {
        if let Some(root_inode) = fs.opened_inode_state.lock().root_inode() {
            let root_inode: Arc<dyn Inode> = root_inode;
            return Ok(root_inode);
        }

        let _mount_open_guard = fs.mount_open_state.lock();
        if let Some(root_inode) = fs.opened_inode_state.lock().root_inode() {
            let root_inode: Arc<dyn Inode> = root_inode;
            return Ok(root_inode);
        }

        let root_chain = root_directory_chain(fs)?;
        let (upcase_dentry, bitmap_dentry) = discover_root_prerequisites(fs, root_chain)?;

        ensure_upcase_table(fs, upcase_dentry)?;
        ensure_allocation_bitmap(fs, bitmap_dentry)?;

        let root_inode = build_root_inode(fs, root_chain)?;
        Ok(fs.publish_root_inode(root_inode))
    }
}

impl FileSystem for ExfatFs {
    fn name(&self) -> &'static str {
        EXFAT_FS_NAME
    }

    fn sync(&self) -> Result<()> {
        // Real flush ordering belongs to EXR-SYNC-31.
        Ok(())
    }

    fn root_inode(&self) -> Arc<dyn Inode> {
        let opened_inode_state = self.opened_inode_state.lock();
        let Some(root_inode) = opened_inode_state.root_inode() else {
            panic!("exFAT root inode has not been published");
        };
        root_inode
    }

    fn sb(&self) -> SuperBlock {
        self.vfs_super_block.clone()
    }

    fn fs_event_subscriber_stats(&self) -> &FsEventSubscriberStats {
        &self.fs_event_subscriber_stats
    }
}

impl ExfatFs {
    /// Validates and publishes the mounted volume's upcase table once.
    pub(super) fn install_upcase_table(
        &self,
        upcase_dentry: ExfatUpcaseDentry,
        raw_table_bytes: &[u8],
    ) -> Result<()> {
        let upcase_table = UpcaseTable::validate_and_decode(upcase_dentry, raw_table_bytes)?;
        self.upcase_state
            .lock()
            .publish_table(Arc::new(upcase_table))
    }

    /// Validates and publishes the mounted volume's allocation bitmap once.
    pub(super) fn load_allocation_bitmap(
        &self,
        bitmap_dentry: ExfatBitmapDentry,
        bitmap_chain: ExfatChain,
    ) -> Result<()> {
        let bitmap = AllocationBitmap::load(
            self.block_device.as_ref(),
            &self.super_block,
            bitmap_dentry,
            bitmap_chain,
        )?;

        let mut allocation_bitmap = self.allocation_bitmap.lock();
        if allocation_bitmap.is_some() {
            return Err(Error::with_message(
                Errno::EINVAL,
                "allocation bitmap is already installed",
            ));
        }

        *allocation_bitmap = Some(bitmap);
        Ok(())
    }

    /// Folds UTF-16 name units through the installed upcase table.
    pub(super) fn fold_utf16(&self, utf16_units: &[u16]) -> Result<Vec<u16>> {
        let upcase_table = self.upcase_state.lock().table()?;
        Ok(upcase_table.fold_utf16(utf16_units))
    }

    /// Computes the exFAT name hash from already folded UTF-16 units.
    pub(super) fn name_hash_from_folded_utf16(&self, folded_utf16_units: &[u16]) -> Result<u16> {
        let _upcase_table = self.upcase_state.lock().table()?;
        Ok(name_hash_from_utf16_units(folded_utf16_units))
    }

    /// Folds UTF-16 name units and then computes the exFAT name hash.
    pub(super) fn name_hash(&self, utf16_units: &[u16]) -> Result<u16> {
        let folded_utf16_units = self.fold_utf16(utf16_units)?;
        Ok(name_hash_from_utf16_units(&folded_utf16_units))
    }

    /// Returns whether a data-cluster id is allocated or bad in the bitmap snapshot.
    pub(super) fn cluster_is_allocated(&self, cluster: u32) -> Result<bool> {
        let allocation_bitmap = self.allocation_bitmap.lock();
        let Some(bitmap) = allocation_bitmap.as_ref() else {
            return Err(Error::with_message(
                Errno::EINVAL,
                "allocation bitmap has not been installed",
            ));
        };

        bitmap.cluster_is_allocated(cluster)
    }

    /// Returns the number of allocated clusters in the published bitmap snapshot.
    pub(super) fn used_cluster_count(&self) -> Result<u32> {
        let allocation_bitmap = self.allocation_bitmap.lock();
        let Some(bitmap) = allocation_bitmap.as_ref() else {
            return Err(Error::with_message(
                Errno::EINVAL,
                "allocation bitmap has not been installed",
            ));
        };

        Ok(bitmap.used_cluster_count())
    }

    /// Returns the number of free clusters in the published bitmap snapshot.
    pub(super) fn free_cluster_count(&self) -> Result<u32> {
        let allocation_bitmap = self.allocation_bitmap.lock();
        let Some(bitmap) = allocation_bitmap.as_ref() else {
            return Err(Error::with_message(
                Errno::EINVAL,
                "allocation bitmap has not been installed",
            ));
        };

        Ok(bitmap.free_cluster_count())
    }

    fn lookup_opened_inode(&self, key: &InodeKey) -> Option<Arc<ExfatInode>> {
        self.opened_inode_state.lock().lookup_opened_inode(key)
    }

    fn publish_opened_inode(&self, key: InodeKey, inode: Arc<ExfatInode>) -> Arc<ExfatInode> {
        self.opened_inode_state
            .lock()
            .publish_opened_inode(key, inode)
    }

    fn remove_opened_inode(&self, key: &InodeKey) -> Option<Arc<ExfatInode>> {
        self.opened_inode_state.lock().remove_opened_inode(key)
    }

    fn publish_root_inode(&self, inode: Arc<ExfatInode>) -> Arc<ExfatInode> {
        self.opened_inode_state.lock().publish_root_inode(inode)
    }
}

fn decode_upcase_units(raw_table_bytes: &[u8]) -> Result<Vec<u16>> {
    if raw_table_bytes.len() % 2 != 0 {
        return Err(Error::with_message(
            Errno::EINVAL,
            "upcase table bytes must be UTF-16 aligned",
        ));
    }

    let mut decoded_units = Vec::with_capacity(UPCASE_TABLE_UNIT_COUNT);
    let mut raw_units = raw_table_bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]));

    while let Some(unit) = raw_units.next() {
        if unit == UPCASE_TABLE_IDENTITY_RUN_MARKER {
            let Some(identity_count) = raw_units.next() else {
                return Err(Error::with_message(
                    Errno::EINVAL,
                    "compressed upcase table ended early",
                ));
            };
            let identity_count = usize::from(identity_count);
            let new_len = decoded_units
                .len()
                .checked_add(identity_count)
                .ok_or_else(|| {
                    Error::with_message(Errno::EINVAL, "upcase table decode overflow")
                })?;
            if new_len > UPCASE_TABLE_UNIT_COUNT {
                return Err(Error::with_message(
                    Errno::EINVAL,
                    "upcase table decode overflow",
                ));
            }

            while decoded_units.len() < new_len {
                let next_unit = u16::try_from(decoded_units.len()).map_err(|_| {
                    Error::with_message(Errno::EINVAL, "upcase table decode overflow")
                })?;
                decoded_units.push(next_unit);
            }
            continue;
        }

        if decoded_units.len() == UPCASE_TABLE_UNIT_COUNT {
            return Err(Error::with_message(
                Errno::EINVAL,
                "upcase table decode overflow",
            ));
        }

        decoded_units.push(unit);
    }

    if decoded_units.len() != UPCASE_TABLE_UNIT_COUNT {
        return Err(Error::with_message(
            Errno::EINVAL,
            "upcase table did not cover the full Unicode range",
        ));
    }

    Ok(decoded_units)
}

fn validate_mandatory_prefix(decoded_units: &[u16]) -> Result<()> {
    for (unit, mapped_unit) in decoded_units.iter().take(128).enumerate() {
        let expected_unit = mandatory_upcase_unit(unit as u16);
        if *mapped_unit != expected_unit {
            return Err(Error::with_message(
                Errno::EINVAL,
                "upcase table has an invalid mandatory mapping",
            ));
        }
    }

    Ok(())
}

fn mandatory_upcase_unit(unit: u16) -> u16 {
    match unit {
        0x61..=0x7A => unit - 0x20,
        _ => unit,
    }
}

fn table_checksum(raw_table_bytes: &[u8]) -> u32 {
    raw_table_bytes.iter().fold(0u32, |checksum, byte| {
        checksum.rotate_right(1).wrapping_add(u32::from(*byte))
    })
}

fn name_hash_from_utf16_units(utf16_units: &[u16]) -> u16 {
    utf16_units.iter().fold(0u16, |checksum, unit| {
        let [low, high] = unit.to_le_bytes();
        let checksum = checksum.rotate_right(1).wrapping_add(u16::from(low));
        checksum.rotate_right(1).wrapping_add(u16::from(high))
    })
}

fn root_directory_chain(fs: &Arc<ExfatFs>) -> Result<ExfatChain> {
    ExfatChain::new(
        fs.block_device.as_ref(),
        &fs.super_block,
        fs.super_block.root_dir,
        None,
        ChainMode::FatBacked,
    )
}

fn discover_root_prerequisites(
    fs: &Arc<ExfatFs>,
    root_chain: ExfatChain,
) -> Result<(ExfatUpcaseDentry, ExfatBitmapDentry)> {
    let mut directory_engine =
        DirectoryEngine::new(fs.block_device.as_ref(), &fs.super_block, root_chain)?;
    let mut upcase_dentry = None;
    let mut bitmap_dentry = None;

    while upcase_dentry.is_none() || bitmap_dentry.is_none() {
        let Some(record) = directory_engine.next_record()? else {
            break;
        };

        match record {
            DirectoryRecord::Singleton(ExfatDentry::Upcase(dentry)) => {
                upcase_dentry = Some(dentry);
            }
            DirectoryRecord::Singleton(ExfatDentry::Bitmap(dentry)) => {
                bitmap_dentry = Some(dentry);
            }
            DirectoryRecord::File(_) => {}
            DirectoryRecord::Singleton(_) => {}
        }
    }

    let Some(upcase_dentry) = upcase_dentry else {
        return Err(Error::with_message(
            Errno::EINVAL,
            "root directory is missing the upcase table",
        ));
    };
    let Some(bitmap_dentry) = bitmap_dentry else {
        return Err(Error::with_message(
            Errno::EINVAL,
            "root directory is missing the allocation bitmap",
        ));
    };

    Ok((upcase_dentry, bitmap_dentry))
}

fn ensure_upcase_table(fs: &Arc<ExfatFs>, upcase_dentry: ExfatUpcaseDentry) -> Result<()> {
    if fs.upcase_state.lock().table.is_some() {
        return Ok(());
    }

    let upcase_chain = ExfatChain::new(
        fs.block_device.as_ref(),
        &fs.super_block,
        upcase_dentry.start_cluster,
        None,
        ChainMode::FatBacked,
    )?;
    let raw_table_size = usize::try_from(upcase_dentry.size).map_err(|_| {
        Error::with_message(Errno::EINVAL, "upcase table size does not fit the host")
    })?;
    let raw_table_bytes = read_chain_bytes(
        fs.block_device.as_ref(),
        &fs.super_block,
        upcase_chain,
        raw_table_size,
    )?;

    fs.install_upcase_table(upcase_dentry, &raw_table_bytes)
}

fn ensure_allocation_bitmap(fs: &Arc<ExfatFs>, bitmap_dentry: ExfatBitmapDentry) -> Result<()> {
    if fs.allocation_bitmap.lock().is_some() {
        return Ok(());
    }

    let bitmap_chain = ExfatChain::new(
        fs.block_device.as_ref(),
        &fs.super_block,
        bitmap_dentry.start_cluster,
        None,
        ChainMode::FatBacked,
    )?;

    fs.load_allocation_bitmap(bitmap_dentry, bitmap_chain)
}

fn build_root_inode(fs: &Arc<ExfatFs>, root_chain: ExfatChain) -> Result<Arc<ExfatInode>> {
    let root_dentry_set = root_dentry_set(root_chain.current_cluster())?;
    let cluster_size = fs.super_block.cluster_size();
    let metadata = Metadata {
        ino: 1,
        size: 0,
        optimal_block_size: cluster_size,
        nr_sectors_allocated: 0,
        last_access_at: Duration::ZERO,
        last_modify_at: Duration::ZERO,
        last_meta_change_at: Duration::ZERO,
        type_: InodeType::Dir,
        mode: InodeMode::S_IRUSR,
        nr_hard_links: 1,
        uid: Uid::new(0),
        gid: Gid::new(0),
        container_dev_id: fs.block_device.id(),
        self_dev_id: None,
    };

    ExfatInode::new(
        Arc::downgrade(fs),
        metadata,
        &root_dentry_set,
        &root_chain,
        cluster_size,
        Some(ExfatInodeLocation::new(None, 0, 0)),
    )
}

fn root_dentry_set(start_cluster: u32) -> Result<ExfatDentrySet> {
    let mut file_dentry = super::dentry::ExfatFileDentry::default();
    file_dentry.attribute = 0x10;

    let mut stream_dentry = super::dentry::ExfatStreamDentry::default();
    stream_dentry.valid_size = 0;
    stream_dentry.start_cluster = start_cluster;
    stream_dentry.size = 0;

    ExfatDentrySet::from_trusted_metadata(
        file_dentry,
        stream_dentry,
        &[b'i' as u16, b'n' as u16, b'o' as u16],
        Vec::new(),
    )
}

fn read_chain_bytes(
    block_device: &dyn BlockDevice,
    super_block: &ExfatSuperBlock,
    chain: ExfatChain,
    byte_len: usize,
) -> Result<Vec<u8>> {
    let mut bytes = vec![0; byte_len];
    let mut loaded_chain = chain;
    let mut copied_bytes = 0usize;
    let cluster_size = super_block.cluster_size();

    for cluster_index in 0..chain.cluster_count() {
        let cluster_offset = loaded_chain.physical_cluster_start_offset(super_block)?;
        let remaining_bytes = byte_len - copied_bytes;
        let copy_len = remaining_bytes.min(cluster_size);
        read_metadata_bytes(
            block_device,
            cluster_offset,
            &mut bytes[copied_bytes..copied_bytes + copy_len],
        )?;
        copied_bytes += copy_len;

        if cluster_index + 1 < chain.cluster_count() {
            loaded_chain = loaded_chain.walk(block_device, super_block, 1)?;
        }
    }

    Ok(bytes)
}

#[cfg(ktest)]
mod tests {
    use alloc::{sync::Arc, vec, vec::Vec};
    use core::time::Duration;

    use aster_block::BlockDevice;
    use ostd::prelude::ktest;
    use zerocopy::IntoBytes;

    use super::{
        super::{
            fat::{ChainMode, ExfatChain},
            inode::{ExfatInode, ExfatInodeLocation},
        },
        EXFAT_FS_NAME, EXFAT_NAME_MAX, ExfatFs, ExfatUpcaseDentry, InodeKey, OpenedInodeState,
        UPCASE_TABLE_IDENTITY_RUN_MARKER, UPCASE_TABLE_UNIT_COUNT, mandatory_upcase_unit,
        table_checksum,
    };
    use crate::{
        fs::{
            file::{InodeMode, InodeType},
            fs_impls::exfat_refactor::{
                boot_sector::{BOOT_SIGNATURE, read_primary_super_block},
                dentry::{
                    DENTRY_SIZE, ExfatDentry, ExfatFileDentry, ExfatStreamDentry, RawExfatDentry,
                },
                fileset::ExfatDentrySet,
                io::read_metadata_bytes,
                test_support::{ExfatMemoryDisk, load_exfat_disk},
            },
            vfs::{
                file_system::{FileSystem, SuperBlock},
                inode::Inode,
            },
        },
        prelude::{Errno, Pod},
        process::{Gid, Uid},
    };

    fn valid_upcase_fixture() -> (ExfatUpcaseDentry, Vec<u8>) {
        let mut raw_table_bytes = Vec::with_capacity(130 * 2);
        let identity_count = u16::try_from(UPCASE_TABLE_UNIT_COUNT - 128).unwrap();
        for unit in 0u16..128 {
            let mapped_unit = mandatory_upcase_unit(unit);
            raw_table_bytes.extend_from_slice(&mapped_unit.to_le_bytes());
        }
        raw_table_bytes.extend_from_slice(&UPCASE_TABLE_IDENTITY_RUN_MARKER.to_le_bytes());
        raw_table_bytes.extend_from_slice(&identity_count.to_le_bytes());

        let upcase_dentry = ExfatUpcaseDentry {
            dentry_type: 0x82,
            reserved1: [0; 3],
            checksum: table_checksum(&raw_table_bytes),
            reserved2: [0; 12],
            start_cluster: 7,
            size: raw_table_bytes.len() as u64,
        };

        (upcase_dentry, raw_table_bytes)
    }

    fn new_exfat_fs() -> ExfatFs {
        let block_device: Arc<dyn BlockDevice> = Arc::new(load_exfat_disk());
        let super_block = read_primary_super_block(block_device.as_ref()).unwrap();

        ExfatFs::new(block_device, super_block).unwrap()
    }

    fn new_mount_ready_exfat_fs() -> ExfatFs {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let (upcase_dentry, raw_table_bytes) = valid_upcase_fixture();
        write_upcase_prerequisite(&disk, &super_block, upcase_dentry, &raw_table_bytes);

        let block_device: Arc<dyn BlockDevice> = Arc::new(disk);
        ExfatFs::new(block_device, super_block).unwrap()
    }

    fn write_upcase_prerequisite(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
        mut upcase_dentry: ExfatUpcaseDentry,
        raw_table_bytes: &[u8],
    ) {
        let root_dir_offset = super_block
            .cluster_to_byte_offset(super_block.root_dir)
            .unwrap();
        let (upcase_entry_index, existing_upcase_dentry) =
            first_existing_upcase_root_entry(disk, super_block);
        let upcase_entry_offset = root_dir_offset + upcase_entry_index * DENTRY_SIZE;
        upcase_dentry.start_cluster = existing_upcase_dentry.start_cluster;

        disk.write_bytes(upcase_entry_offset, upcase_dentry.as_bytes());
        disk.write_bytes(
            super_block
                .cluster_to_byte_offset(upcase_dentry.start_cluster)
                .unwrap(),
            raw_table_bytes,
        );
    }

    fn first_existing_upcase_root_entry(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
    ) -> (usize, ExfatUpcaseDentry) {
        let root_dir_offset = super_block
            .cluster_to_byte_offset(super_block.root_dir)
            .unwrap();
        let cluster_size = super_block.cluster_size();
        let entry_count = cluster_size / DENTRY_SIZE;

        for entry_index in 0..entry_count {
            let mut raw_bytes = [0; DENTRY_SIZE];
            read_metadata_bytes(
                disk,
                root_dir_offset + entry_index * DENTRY_SIZE,
                &mut raw_bytes,
            )
            .unwrap();
            match ExfatDentry::from(RawExfatDentry::from_bytes(&raw_bytes)) {
                ExfatDentry::Upcase(upcase_dentry) => return (entry_index, upcase_dentry),
                _ => {}
            }
        }

        panic!("expected an existing upcase slot in the root directory");
    }

    fn trusted_dentry_set(
        file_size: u64,
        valid_size: u64,
        file_attribute: u16,
        start_cluster: u32,
    ) -> ExfatDentrySet {
        let mut file_dentry = ExfatFileDentry::default();
        file_dentry.attribute = file_attribute;

        let mut stream_dentry = ExfatStreamDentry::default();
        stream_dentry.valid_size = valid_size;
        stream_dentry.start_cluster = start_cluster;
        stream_dentry.size = file_size;

        ExfatDentrySet::from_trusted_metadata(
            file_dentry,
            stream_dentry,
            &[b'i' as u16, b'n' as u16, b'o' as u16],
            Vec::new(),
        )
        .unwrap()
    }

    fn test_inode() -> Arc<ExfatInode> {
        let disk = Arc::new(load_exfat_disk());
        let super_block = read_primary_super_block(disk.as_ref()).unwrap();
        let container_dev_id = disk.id();
        let chain = ExfatChain::new(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            Some(1),
            ChainMode::Contiguous,
        )
        .unwrap();
        let fs = Arc::new(ExfatFs::new(disk, super_block).unwrap());
        let cluster_size = fs.super_block.cluster_size();
        let dentry_set = trusted_dentry_set(128, 96, 0x20, chain.current_cluster());
        let metadata = crate::fs::vfs::inode::Metadata {
            ino: 42,
            size: 0,
            optimal_block_size: 512,
            nr_sectors_allocated: 0,
            last_access_at: Duration::from_secs(1),
            last_modify_at: Duration::from_secs(2),
            last_meta_change_at: Duration::from_secs(3),
            type_: InodeType::File,
            mode: InodeMode::S_IRUSR,
            nr_hard_links: 1,
            uid: Uid::new(1000),
            gid: Gid::new(1001),
            container_dev_id,
            self_dev_id: None,
        };

        ExfatInode::new(
            Arc::downgrade(&fs),
            metadata,
            &dentry_set,
            &chain,
            cluster_size,
            Some(ExfatInodeLocation::new(Some(7), 4096, 3)),
        )
        .unwrap()
    }

    fn test_root_inode() -> Arc<ExfatInode> {
        let disk = Arc::new(load_exfat_disk());
        let super_block = read_primary_super_block(disk.as_ref()).unwrap();
        let container_dev_id = disk.id();
        let chain = ExfatChain::new(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            Some(1),
            ChainMode::Contiguous,
        )
        .unwrap();
        let fs = Arc::new(ExfatFs::new(disk, super_block).unwrap());
        let cluster_size = fs.super_block.cluster_size();
        let dentry_set = trusted_dentry_set(0, 0, 0x10, chain.current_cluster());
        let metadata = crate::fs::vfs::inode::Metadata {
            ino: 1,
            size: 0,
            optimal_block_size: 512,
            nr_sectors_allocated: 0,
            last_access_at: Duration::from_secs(1),
            last_modify_at: Duration::from_secs(2),
            last_meta_change_at: Duration::from_secs(3),
            type_: InodeType::Dir,
            mode: InodeMode::S_IRUSR,
            nr_hard_links: 1,
            uid: Uid::new(0),
            gid: Gid::new(0),
            container_dev_id,
            self_dev_id: None,
        };

        ExfatInode::new(
            Arc::downgrade(&fs),
            metadata,
            &dentry_set,
            &chain,
            cluster_size,
            Some(ExfatInodeLocation::new(None, 0, 0)),
        )
        .unwrap()
    }

    fn assert_same_super_block(left: &SuperBlock, right: &SuperBlock) {
        assert_eq!(left.magic, right.magic);
        assert_eq!(left.bsize, right.bsize);
        assert_eq!(left.blocks, right.blocks);
        assert_eq!(left.bfree, right.bfree);
        assert_eq!(left.bavail, right.bavail);
        assert_eq!(left.files, right.files);
        assert_eq!(left.ffree, right.ffree);
        assert_eq!(left.fsid, right.fsid);
        assert_eq!(left.namelen, right.namelen);
        assert_eq!(left.frsize, right.frsize);
        assert_eq!(left.flags, right.flags);
        assert_eq!(left.container_dev_id, right.container_dev_id);
    }

    // Confirms valid publication is accepted once and malformed size/checksum inputs are rejected.
    #[ktest]
    fn upcase_table_installation_accepts_valid_and_rejects_invalid_fixtures() {
        let fs = new_exfat_fs();
        let (upcase_dentry, raw_table_bytes) = valid_upcase_fixture();

        let too_short_error = fs
            .install_upcase_table(upcase_dentry, &raw_table_bytes[..raw_table_bytes.len() - 2])
            .unwrap_err();
        assert_eq!(too_short_error.error(), Errno::EINVAL);

        let mut checksum_mismatch_dentry = upcase_dentry;
        checksum_mismatch_dentry.checksum = checksum_mismatch_dentry.checksum.wrapping_add(1);
        let checksum_error = fs
            .install_upcase_table(checksum_mismatch_dentry, &raw_table_bytes)
            .unwrap_err();
        assert_eq!(checksum_error.error(), Errno::EINVAL);

        fs.install_upcase_table(upcase_dentry, &raw_table_bytes)
            .unwrap();

        let folded = fs
            .fold_utf16(&[b'a' as u16, b'B' as u16, b'c' as u16])
            .unwrap();
        assert_eq!(folded, vec![b'A' as u16, b'B' as u16, b'C' as u16]);

        let already_installed_error = fs
            .install_upcase_table(valid_upcase_fixture().0, &raw_table_bytes)
            .unwrap_err();
        assert_eq!(already_installed_error.error(), Errno::EINVAL);
    }

    // Confirms repeated folds use the same installed table and remain deterministic.
    #[ktest]
    fn fold_utf16_uses_installed_table_deterministically() {
        let fs = new_exfat_fs();
        let (upcase_dentry, raw_table_bytes) = valid_upcase_fixture();
        fs.install_upcase_table(upcase_dentry, &raw_table_bytes)
            .unwrap();

        let mixed_case_name = [b'a' as u16, b'Z' as u16, b'0' as u16];
        let canonical_name = [b'A' as u16, b'Z' as u16, b'0' as u16];

        let first_fold = fs.fold_utf16(&mixed_case_name).unwrap();
        let second_fold = fs.fold_utf16(&mixed_case_name).unwrap();
        let uppercase_fold = fs.fold_utf16(&canonical_name).unwrap();

        assert_eq!(first_fold, vec![b'A' as u16, b'Z' as u16, b'0' as u16]);
        assert_eq!(first_fold, second_fold);
        assert_eq!(first_fold, uppercase_fold);
    }

    // Confirms hash computation follows the folded UTF-16 bytes consumed by later lookup code.
    #[ktest]
    fn name_hash_uses_folded_utf16_bytes() {
        let fs = new_exfat_fs();
        let (upcase_dentry, raw_table_bytes) = valid_upcase_fixture();
        fs.install_upcase_table(upcase_dentry, &raw_table_bytes)
            .unwrap();

        let hash_from_lowercase = fs
            .name_hash(&[b'a' as u16, b'b' as u16, b'c' as u16])
            .unwrap();
        let hash_from_uppercase = fs
            .name_hash(&[b'A' as u16, b'B' as u16, b'C' as u16])
            .unwrap();
        let hash_from_other_folded_name = fs
            .name_hash_from_folded_utf16(&[b'A' as u16, b'B' as u16, b'D' as u16])
            .unwrap();

        assert_eq!(hash_from_lowercase, hash_from_uppercase);
        assert_ne!(hash_from_uppercase, hash_from_other_folded_name);
        assert_eq!(
            hash_from_other_folded_name,
            fs.name_hash(&[b'a' as u16, b'b' as u16, b'd' as u16])
                .unwrap()
        );
    }

    #[ktest]
    fn filesystem_identity_and_super_block_snapshot_are_stable() {
        // Confirms the owner exposes one stable VFS identity and reuses the
        // normalized superblock snapshot rather than rebuilding mount state.
        let fs = new_exfat_fs();
        let filesystem: &dyn FileSystem = &fs;
        let first_super_block = filesystem.sb();
        let second_super_block = filesystem.sb();
        let expected_blocks = usize::try_from(fs.super_block.num_sectors).unwrap();

        assert_eq!(filesystem.name(), EXFAT_FS_NAME);
        assert_eq!(first_super_block.magic, u64::from(BOOT_SIGNATURE));
        assert_eq!(first_super_block.bsize, fs.super_block.sector_size());
        assert_eq!(first_super_block.blocks, expected_blocks);
        assert_eq!(first_super_block.namelen, EXFAT_NAME_MAX);
        assert_eq!(first_super_block.frsize, fs.super_block.sector_size());
        assert_eq!(first_super_block.container_dev_id, fs.block_device.id());
        assert_same_super_block(&first_super_block, &second_super_block);
    }

    #[ktest]
    fn subscriber_stats_and_snapshot_survive_placeholder_sync() {
        // Confirms `sync()` is still a no-op placeholder for owner-visible
        // state, and subscriber stats stay attached to this `ExfatFs` instance.
        let fs = new_exfat_fs();
        let filesystem: &dyn FileSystem = &fs;
        let stats_before_sync = filesystem.fs_event_subscriber_stats();
        let super_block_before_sync = filesystem.sb();

        stats_before_sync.add_subscriber();
        filesystem.sync().unwrap();

        let stats_after_sync = filesystem.fs_event_subscriber_stats();
        let super_block_after_sync = filesystem.sb();

        assert!(core::ptr::eq(stats_before_sync, stats_after_sync));
        assert!(stats_after_sync.has_any_subscribers());
        assert_same_super_block(&super_block_before_sync, &super_block_after_sync);

        stats_after_sync.remove_subscriber();
        assert!(!filesystem.fs_event_subscriber_stats().has_any_subscribers());
    }

    #[ktest]
    fn inode_key_tracks_only_trusted_location_facts() {
        let same_location_a = InodeKey::new(Some(7), 4096, 3);
        let same_location_b = InodeKey::new(Some(7), 4096, 3);
        let different_entry = InodeKey::new(Some(7), 4096, 4);
        let different_parent = InodeKey::new(Some(8), 4096, 3);

        assert_eq!(same_location_a, same_location_b);
        assert_ne!(same_location_a, different_entry);
        assert_ne!(same_location_a, different_parent);
    }

    #[ktest]
    fn opened_inode_state_reuses_canonical_handle_and_exact_key_removal() {
        let inode = test_inode();
        let same_key = InodeKey::new(Some(7), 4096, 3);
        let other_key = InodeKey::new(Some(7), 4096, 4);
        let mut state = OpenedInodeState::default();

        let published_first = state.publish_opened_inode(same_key, inode.clone());
        let published_second = state.publish_opened_inode(same_key, test_inode());
        let unrelated = state.publish_opened_inode(other_key, test_inode());

        assert!(Arc::ptr_eq(&published_first, &published_second));
        assert!(!Arc::ptr_eq(&published_first, &unrelated));
        assert!(state.lookup_opened_inode(&same_key).is_some());
        assert!(state.lookup_opened_inode(&other_key).is_some());

        let removed = state.remove_opened_inode(&same_key).unwrap();
        assert!(Arc::ptr_eq(&removed, &published_first));
        assert!(state.lookup_opened_inode(&same_key).is_none());
        assert!(state.lookup_opened_inode(&other_key).is_some());
    }

    #[ktest]
    fn root_special_case_stays_outside_the_ordinary_keyspace() {
        let root_inode = test_inode();
        let ordinary_key = InodeKey::new(Some(7), 4096, 3);
        let mut state = OpenedInodeState::default();

        let published_root = state.publish_root_inode(root_inode.clone());

        assert!(Arc::ptr_eq(&published_root, &root_inode));
        assert!(state.lookup_opened_inode(&ordinary_key).is_none());
        assert!(state.root_inode().is_some());
        assert!(Arc::ptr_eq(&state.root_inode().unwrap(), &published_root));
    }

    #[ktest]
    fn root_inode_publication_returns_the_canonical_root_handle() {
        // Confirms the owner-private root slot is the canonical publication
        // path and repeated access reuses the same handle.
        let fs = new_exfat_fs();
        let filesystem: &dyn FileSystem = &fs;
        let root_inode = test_root_inode();

        let published_root = fs.publish_root_inode(root_inode.clone());
        let first_root = filesystem.root_inode();
        let second_root = filesystem.root_inode();
        let published_root_as_inode: Arc<dyn Inode> = published_root.clone();

        assert!(Arc::ptr_eq(&published_root, &root_inode));
        assert!(Arc::ptr_eq(&first_root, &second_root));
        assert!(Arc::ptr_eq(&published_root_as_inode, &second_root));
    }

    #[ktest]
    fn root_mount_sequence_installs_prerequisites_before_publishing_root() {
        // Confirms the owner-side open sequence discovers the mount-time
        // prerequisites, installs them, and publishes the canonical root
        // handle under the root-special-case slot.
        let fs = Arc::new(new_mount_ready_exfat_fs());
        let filesystem: &dyn FileSystem = fs.as_ref();

        assert!(fs.fold_utf16(&[b'a' as u16]).is_err());
        assert!(fs.used_cluster_count().is_err());

        let first_root = ExfatFs::open_root_inode(&fs).unwrap();
        let second_root = ExfatFs::open_root_inode(&fs).unwrap();
        let first_root_as_inode: Arc<dyn Inode> = first_root.clone();
        let second_root_as_inode: Arc<dyn Inode> = second_root.clone();
        let trait_root = filesystem.root_inode();

        assert!(Arc::ptr_eq(&first_root, &second_root));
        assert!(Arc::ptr_eq(&first_root_as_inode, &second_root_as_inode));
        assert!(Arc::ptr_eq(&first_root_as_inode, &trait_root));
        assert_eq!(
            fs.fold_utf16(&[b'a' as u16, b'z' as u16]).unwrap(),
            vec![b'A' as u16, b'Z' as u16]
        );
        assert!(fs.used_cluster_count().unwrap() > 0);
    }
}
