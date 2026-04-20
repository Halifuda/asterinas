// SPDX-License-Identifier: MPL-2.0

use core::time::Duration;

use super::fs::ExfatFs;
use crate::{
    fs::{
        file::{AccessMode, FileIo, InodeMode, InodeType, StatusFlags, mkmod},
        utils::DirentVisitor,
        vfs::{
            file_system::FileSystem,
            inode::{Extension, FallocMode, Inode, Metadata, MknodType, SymbolicLink},
        },
    },
    prelude::*,
    process::{Gid, Uid},
    vm::vmo::Vmo,
};

pub(super) struct ExfatInode {
    extension: Extension,
    fs: Weak<ExfatFs>,
    metadata: RwLock<Metadata>,
    this: Weak<Self>,
}

impl ExfatInode {
    pub(super) fn new_root(
        fs: &Arc<ExfatFs>,
        root_cluster: u32,
        cluster_size: usize,
    ) -> Arc<Self> {
        Arc::new_cyclic(|weak_self| {
            let mut metadata = Metadata::new_dir(
                u64::from(root_cluster),
                mkmod!(u+rwx, g+rx, o+rx),
                cluster_size,
                fs.container_device_id(),
            );
            metadata.size = cluster_size;
            Self {
                extension: Extension::new(),
                fs: Arc::downgrade(fs),
                metadata: RwLock::new(metadata),
                this: weak_self.clone(),
            }
        })
    }
}

impl crate::fs::vfs::inode::InodeIo for ExfatInode {
    fn read_at(
        &self,
        _offset: usize,
        _writer: &mut VmWriter,
        _status_flags: StatusFlags,
    ) -> Result<usize> {
        return_errno!(Errno::EISDIR);
    }

    fn write_at(
        &self,
        _offset: usize,
        _reader: &mut VmReader,
        _status_flags: StatusFlags,
    ) -> Result<usize> {
        return_errno!(Errno::EISDIR);
    }
}

impl Inode for ExfatInode {
    fn size(&self) -> usize {
        self.metadata.read().size
    }

    fn resize(&self, _new_size: usize) -> Result<()> {
        return_errno!(Errno::EISDIR);
    }

    fn metadata(&self) -> Metadata {
        *self.metadata.read()
    }

    fn ino(&self) -> u64 {
        self.metadata.read().ino
    }

    fn type_(&self) -> InodeType {
        InodeType::Dir
    }

    fn mode(&self) -> Result<InodeMode> {
        Ok(self.metadata.read().mode)
    }

    fn set_mode(&self, mode: InodeMode) -> Result<()> {
        self.metadata.write().mode = mode;
        Ok(())
    }

    fn owner(&self) -> Result<Uid> {
        Ok(self.metadata.read().uid)
    }

    fn set_owner(&self, uid: Uid) -> Result<()> {
        self.metadata.write().uid = uid;
        Ok(())
    }

    fn group(&self) -> Result<Gid> {
        Ok(self.metadata.read().gid)
    }

    fn set_group(&self, gid: Gid) -> Result<()> {
        self.metadata.write().gid = gid;
        Ok(())
    }

    fn atime(&self) -> Duration {
        self.metadata.read().last_access_at
    }

    fn set_atime(&self, time: Duration) {
        self.metadata.write().last_access_at = time;
    }

    fn mtime(&self) -> Duration {
        self.metadata.read().last_modify_at
    }

    fn set_mtime(&self, time: Duration) {
        self.metadata.write().last_modify_at = time;
    }

    fn ctime(&self) -> Duration {
        self.metadata.read().last_meta_change_at
    }

    fn set_ctime(&self, time: Duration) {
        self.metadata.write().last_meta_change_at = time;
    }

    fn page_cache(&self) -> Option<Arc<Vmo>> {
        None
    }

    fn create(&self, _name: &str, _type_: InodeType, _mode: InodeMode) -> Result<Arc<dyn Inode>> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn mknod(&self, _name: &str, _mode: InodeMode, _type_: MknodType) -> Result<Arc<dyn Inode>> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn open(
        &self,
        _access_mode: AccessMode,
        _status_flags: StatusFlags,
    ) -> Option<Result<Box<dyn FileIo>>> {
        None
    }

    fn readdir_at(&self, offset: usize, visitor: &mut dyn DirentVisitor) -> Result<usize> {
        // TODO: Absorb this mount-only root seam once later inode passes own
        // durable directory enumeration instead of only `.` and `..` exposure.
        let mut next_offset = offset;
        if next_offset == 0 {
            visitor.visit(".", self.ino(), self.type_(), next_offset)?;
            next_offset += 1;
        }
        if next_offset == 1 {
            visitor.visit("..", self.ino(), self.type_(), next_offset)?;
            next_offset += 1;
        }
        Ok(next_offset.saturating_sub(offset))
    }

    fn link(&self, _old: &Arc<dyn Inode>, _name: &str) -> Result<()> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn unlink(&self, _name: &str) -> Result<()> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn rmdir(&self, _name: &str) -> Result<()> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>> {
        // TODO: Absorb this mount-only root seam once later inode passes own
        // child lookup beyond the eagerly published root carrier.
        if name == "." || name == ".." {
            let inode: Arc<dyn Inode> = self.this.upgrade().unwrap();
            return Ok(inode);
        }
        return_errno!(Errno::ENOENT);
    }

    fn rename(&self, _old_name: &str, _target: &Arc<dyn Inode>, _new_name: &str) -> Result<()> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn read_link(&self) -> Result<SymbolicLink> {
        return_errno!(Errno::EISDIR);
    }

    fn write_link(&self, _target: &str) -> Result<()> {
        return_errno!(Errno::EISDIR);
    }

    fn sync_all(&self) -> Result<()> {
        Ok(())
    }

    fn sync_data(&self) -> Result<()> {
        Ok(())
    }

    fn fallocate(&self, _mode: FallocMode, _offset: usize, _len: usize) -> Result<()> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
        Weak::upgrade(&self.fs).unwrap()
    }

    fn extension(&self) -> &Extension {
        &self.extension
    }
}
