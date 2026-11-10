// SPDX-License-Identifier: MPL-2.0

//! The VFS trait surface of Squashfs.
//!
//! Each method forwards in one line to the file that owns the answer,
//! so this file lists what the filesystem promises rather than how it delivers it.
//! A question about the mount goes to [`Squashfs`],
//! a question about one inode to [`SquashfsInode`];
//! the mutating methods keep their signatures only to refuse.
//! [`super::inode`] implements the page-cache backend instead,
//! because no caller asks the filesystem about its two methods.

use core::time::Duration;

use super::{fs::Squashfs, inode::SquashfsInode};
use crate::{
    fs::{
        file::{InodeMode, InodeType, StatusFlags},
        utils::DirentVisitor,
        vfs::{
            file_system::{FileSystem, FsEventSubscriberStats, FsFlags, SuperBlock},
            inode::{Extension, FileOps, Inode, Metadata},
            path::Dentry,
        },
    },
    prelude::*,
    process::{Gid, Uid},
    vm::page_cache::Vmo,
};

impl FileSystem for Squashfs {
    fn name(&self) -> &'static str {
        self.name_impl()
    }

    fn sync(&self) -> Result<()> {
        self.sync_impl()
    }

    fn root_inode(&self) -> Arc<dyn Inode> {
        self.root_inode_impl()
    }

    fn sb(&self) -> SuperBlock {
        self.super_block_impl()
    }

    fn flags(&self) -> FsFlags {
        self.flags_impl()
    }

    fn fs_event_subscriber_stats(&self) -> &FsEventSubscriberStats {
        self.fs_event_stats_impl()
    }
}

impl FileOps for SquashfsInode {
    fn read_at(
        &self,
        offset: usize,
        writer: &mut VmWriter,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        self.read_at_impl(offset, writer, status_flags)
    }

    fn write_at(
        &self,
        offset: usize,
        reader: &mut VmReader,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        self.write_at_impl(offset, reader, status_flags)
    }

    fn readdir_at(&self, offset: usize, visitor: &mut dyn DirentVisitor) -> Result<usize> {
        self.readdir_impl(offset, visitor)
    }
}

impl Inode for SquashfsInode {
    fn size(&self) -> usize {
        self.size_impl()
    }

    fn resize(&self, _self_dentry: &Dentry, new_size: usize) -> Result<()> {
        self.resize_impl(new_size)
    }

    fn metadata(&self) -> Result<Metadata> {
        self.metadata_impl()
    }

    fn ino(&self) -> u64 {
        self.ino_impl()
    }

    fn type_(&self) -> InodeType {
        self.type_impl()
    }

    fn mode(&self) -> Result<InodeMode> {
        self.mode_impl()
    }

    fn set_mode(&self, _self_dentry: &Dentry, mode: InodeMode) -> Result<()> {
        self.set_mode_impl(mode)
    }

    fn owner(&self) -> Result<Uid> {
        self.owner_impl()
    }

    fn set_owner(&self, _self_dentry: &Dentry, uid: Uid) -> Result<()> {
        self.set_owner_impl(uid)
    }

    fn group(&self) -> Result<Gid> {
        self.group_impl()
    }

    fn set_group(&self, _self_dentry: &Dentry, gid: Gid) -> Result<()> {
        self.set_group_impl(gid)
    }

    fn atime(&self) -> Duration {
        self.atime_impl()
    }

    fn set_atime(&self, _self_dentry: &Dentry, time: Duration) {
        self.set_atime_impl(time)
    }

    fn mtime(&self) -> Duration {
        self.mtime_impl()
    }

    fn set_mtime(&self, _self_dentry: &Dentry, time: Duration) {
        self.set_mtime_impl(time)
    }

    fn ctime(&self) -> Duration {
        self.ctime_impl()
    }

    fn set_ctime(&self, _self_dentry: &Dentry, time: Duration) {
        self.set_ctime_impl(time)
    }

    fn page_cache(&self) -> Option<Arc<Vmo>> {
        self.page_cache_impl()
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>> {
        self.lookup_impl(name)
    }

    fn extension(&self) -> &Extension {
        self.extension_impl()
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
        self.fs_impl()
    }
}
