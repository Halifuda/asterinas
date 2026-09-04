// SPDX-License-Identifier: MPL-2.0

#![short_vis_path::add(overlayfs)]
//! The copy-up authority: per-object coordination, winner/waiter trigger,
//! and object-kind promotion.
//!
//! Key concepts:
//! - **copy-up coordinate**: the pending publication name a lower-backed
//!   object carries, with its recorded parent, from projection time until the
//!   copy-up commit retires it.
//! - **winner/waiter**: the tasks racing on one object's `copyup` mutex — the
//!   winner performs the promotion; the waiters re-observe upper authority
//!   and return.
//! - **copy-up frame**: one recursive promotion step for one object, from
//!   coordinate read through commit and retirement.
//!
//! [`OverlayInode::copy_up`] is the single promotion entry: it promotes
//! ancestors before the child and serializes winners through the `copyup`
//! mutex.
//!
//! ## Structure
//!
//! | Submodule | Responsibility |
//! | --- | --- |
//! | `workdir` | private workdir temp create/publish/cleanup lifecycle |
//!
//! ## Locking
//!
//! The `InodeCache` write guard is the innermost leaf: `publish_rekey` runs
//! under it alone, never waits, never acquires another overlay lock, and
//! never calls back into projection or copy-up.
//!
//! Within one copy-up frame the locks nest strictly in this order: the
//! object's `copyup` mutex, then the publication parent's directory
//! transaction lock, then the `InodeCache` write guard.
//!
//! - The coordinate read acquires and releases the `copyup` mutex together
//!   with the `recorded_parent` read guard before the ancestor walk; no
//!   child-to-parent `copyup` mutex edge exists.
//! - The winner then holds the `copyup` mutex continuously from
//!   re-acquisition through the commit tail and the coordinate retirement;
//!   `publish_by_rename` takes the parent directory transaction lock inside
//!   that hold, and the cache write is the innermost leaf.
//! - No lock is released and reacquired inside the cache guard, and no new
//!   lock domain or lock edge is introduced. The pre-existing orders —
//!   `copyup` mutex before directory transaction lock, directory transaction
//!   lock before the `recorded_parent` guard, and `copyup` mutex before the
//!   `recorded_parent` guard — are unchanged.
//!
//! ## References
//!
//! - Linux `ovl_real_file_path` follow-copy-up:
//!   <https://elixir.bootlin.com/linux/latest/source/fs/overlayfs/file.c#L128-L171>
//! - Linux `ovl_set_attr` (symlink mode skip):
//!   <https://elixir.bootlin.com/linux/latest/source/fs/overlayfs/copy_up.c#L392-L416>

use core::cmp::min;

use self::workdir::{WorkdirTemp, WorkdirTempRequest};
use crate::{
    fs::{
        file::{InodeType, StatusFlags},
        fs_impls::overlayfs::{
            fs::OverlayFs,
            inode::{Lookup, OverlayInode, xattr::XattrCopyPolicy},
            real::RealObject,
        },
        vfs::inode::{Inode, MknodType, RenameMode, SymbolicLink},
    },
    prelude::*,
};

pub(super) mod workdir;

/// Each frame holds only two live `Arc`s and no guard, so 1024 frames fit the default kernel
/// task stack and deeper chains fail closed with `ELOOP`.
const MAX_COPYUP_DEPTH: usize = 1024;

const COPY_CHUNK_SIZE: usize = 64 * 1024;

impl OverlayInode {
    pub(super) fn copy_up(&self) -> Result<()> {
        self.copy_up_inner(0)
    }

    fn copy_up_inner(&self, depth: usize) -> Result<()> {
        let fs = self.fs_arc()?;

        if self.upper.get().is_some() {
            return Ok(());
        }

        let (publication_parent, name) = {
            let published = self.lock_copyup();
            let Some(name) = published.as_ref() else {
                // No pending coordinate means the object was already published (the
                // mount root never carries one); every mutating entry is EROFS-gated
                // upstream, so this read-only exit is sound.
                return Ok(());
            };
            let Some(parent) = self.recorded_parent.read().upgrade() else {
                return Err(Error::with_message(
                    Errno::EIO,
                    "the copy-up publication parent no longer exists",
                ));
            };
            (parent, name.clone())
        };

        if depth >= MAX_COPYUP_DEPTH {
            return_errno_with_message!(
                Errno::ELOOP,
                "the copy-up ancestor chain exceeds the depth limit"
            );
        }
        publication_parent.copy_up_inner(depth + 1)?;

        let mut published = self.lock_copyup();

        if self.upper.get().is_some() {
            return Ok(());
        }

        // A retired coordinate with `upper` unset is structurally unreachable — retirement
        // only follows publication, under this same guard — so fail closed defensively.
        if published.is_none() {
            return Err(Error::with_message(
                Errno::EIO,
                "the overlay copy-up coordinate turned inconsistent during arbitration",
            ));
        }

        // Every promoted object makes its publication parent impure: persist the marker
        // strictly before the object-kind dispatch and the physical upper commit.
        OverlayInode::set_impure_marker(
            &publication_parent.select_real_inode(),
            fs.policy().xattr_prefix(),
        )?;

        let staged = self.stage_in_workdir(&name)?;
        self.publish_by_rename(&publication_parent, &name, staged)?;
        *published = None;
        Ok(())
    }

    fn lock_copyup(&self) -> MutexGuard<'_, Option<String>> {
        self.copyup.lock()
    }

    /// Metadata, eligible xattrs, and the origin record are written on the temp before the
    /// timestamp replay (the origin write may refresh `ctime`), and a regular-file temp is
    /// synced before publication.
    fn stage_in_workdir(&self, name: &str) -> Result<WorkdirTemp> {
        let fs = self.fs_arc()?;
        let prefix = fs.policy().xattr_prefix();
        let lower = self.lower_source()?;
        match lower.real_inode().type_() {
            InodeType::Dir => {
                // Publication uses atomic `RenameMode::Replace` so a stale upper entry is replaced
                // instead of failing `create` with `EEXIST`; only the directory itself
                // is copied — its children remain lower-backed.
                let mode = lower.real_inode().mode()?;
                let temp = fs.create_workdir_temp(
                    name,
                    WorkdirTempRequest::Create {
                        kind: InodeType::Dir,
                        mode,
                    },
                )?;
                if let Err(err) = self
                    .transfer_metadata(lower.real_inode(), temp.inode())
                    .and_then(|_| {
                        OverlayInode::copy_eligible_xattrs(
                            lower.real_inode(),
                            temp.inode(),
                            XattrCopyPolicy::Strict,
                            prefix,
                        )
                    })
                    .and_then(|_| fs.store_lower_id(temp.inode(), &lower))
                    .and_then(|_| self.transfer_timestamps(lower.real_inode(), temp.inode()))
                {
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                    return Err(err);
                }
                Ok(temp)
            }
            InodeType::File => {
                // Data is streamed into the temp and `sync_all`-ed before the atomic rename, so the
                // published object is durable.
                let mode = lower.real_inode().mode()?;
                let temp = fs.create_workdir_temp(
                    name,
                    WorkdirTempRequest::Create {
                        kind: InodeType::File,
                        mode,
                    },
                )?;
                if let Err(err) = self
                    .transfer_metadata(lower.real_inode(), temp.inode())
                    .and_then(|_| {
                        OverlayInode::copy_eligible_xattrs(
                            lower.real_inode(),
                            temp.inode(),
                            XattrCopyPolicy::Strict,
                            prefix,
                        )
                    })
                    .and_then(|_| self.promote_regular_file(temp.inode()))
                    .and_then(|_| fs.store_lower_id(temp.inode(), &lower))
                    .and_then(|_| self.transfer_timestamps(lower.real_inode(), temp.inode()))
                    .and_then(|_| temp.inode().sync_all())
                {
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                    return Err(err);
                }
                Ok(temp)
            }
            InodeType::SymLink => {
                // The symlink is recreated from the lower target string; the target object
                // itself is never promoted.
                let mode = lower.real_inode().mode()?;
                let temp = fs.create_workdir_temp(
                    name,
                    WorkdirTempRequest::Create {
                        kind: InodeType::SymLink,
                        mode,
                    },
                )?;
                if let Err(err) = self
                    .promote_symlink(temp.inode())
                    .and_then(|_| self.transfer_metadata(lower.real_inode(), temp.inode()))
                    .and_then(|_| {
                        OverlayInode::copy_eligible_xattrs(
                            lower.real_inode(),
                            temp.inode(),
                            XattrCopyPolicy::Strict,
                            prefix,
                        )
                    })
                    .and_then(|_| fs.store_lower_id(temp.inode(), &lower))
                    .and_then(|_| self.transfer_timestamps(lower.real_inode(), temp.inode()))
                {
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                    return Err(err);
                }
                Ok(temp)
            }
            InodeType::CharDevice
            | InodeType::BlockDevice
            | InodeType::NamedPipe
            | InodeType::Socket => {
                let mknod_type = match lower.real_inode().type_() {
                    InodeType::NamedPipe => MknodType::NamedPipe,
                    InodeType::CharDevice => {
                        let rdev = lower
                            .real_inode()
                            .metadata()?
                            .self_dev_id
                            .ok_or_else(|| {
                                Error::with_message(
                                    Errno::EINVAL,
                                    "the lower char device has no device id",
                                )
                            })?
                            .as_encoded_u64();
                        MknodType::CharDevice(rdev)
                    }
                    InodeType::BlockDevice => {
                        let rdev = lower
                            .real_inode()
                            .metadata()?
                            .self_dev_id
                            .ok_or_else(|| {
                                Error::with_message(
                                    Errno::EINVAL,
                                    "the lower block device has no device id",
                                )
                            })?
                            .as_encoded_u64();
                        MknodType::BlockDevice(rdev)
                    }
                    _ => {
                        return Err(Error::with_message(
                            Errno::EOPNOTSUPP,
                            "socket nodes cannot be copied up",
                        ));
                    }
                };
                let mode = lower.real_inode().mode()?;
                let temp = fs.create_workdir_temp(
                    name,
                    WorkdirTempRequest::Mknod {
                        mode,
                        node: &mknod_type,
                    },
                )?;
                if let Err(err) = self
                    .transfer_metadata(lower.real_inode(), temp.inode())
                    .and_then(|_| {
                        OverlayInode::copy_eligible_xattrs(
                            lower.real_inode(),
                            temp.inode(),
                            XattrCopyPolicy::Strict,
                            prefix,
                        )
                    })
                    .and_then(|_| fs.store_lower_id(temp.inode(), &lower))
                    .and_then(|_| self.transfer_timestamps(lower.real_inode(), temp.inode()))
                {
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                    return Err(err);
                }
                Ok(temp)
            }
            InodeType::Unknown => Err(Error::with_message(
                Errno::EINVAL,
                "cannot promote an overlay object of unknown type",
            )),
        }
    }

    /// `publication_parent` is passed explicitly rather than re-derived from `recorded_parent`:
    /// a concurrent cross-parent rename may repoint `recorded_parent` before the commit lock, so
    /// the commit must target the parent the ancestor walk promoted (first-bound coordinate rule).
    fn publish_by_rename(
        &self,
        publication_parent: &Arc<OverlayInode>,
        name: &str,
        staged: WorkdirTemp,
    ) -> Result<()> {
        let fs = self.fs_arc()?;
        // Computed once before the commit scope so the post-rename segment
        // contains no fallible step.
        let upper_dir_path = publication_parent.upper_parent_path()?;
        let upper_layer = fs.layer_stack().upper_layer()?;
        let result: Result<()> = (|| {
            let _publication_guard = publication_parent.lock.lock();
            // The freshly looked-up child must still denote the same logical publication
            // target: negative or unrelated hits abort, so the rename can never resurrect an
            // unlinked name.
            let current = match fs.lookup(publication_parent, name)? {
                Lookup::Positive(current) if self.is_same_publication_target(&current, &fs) => {
                    current
                }
                _ => {
                    return Err(Error::with_message(
                        Errno::ENOENT,
                        "the copy-up target name is no longer visible",
                    ));
                }
            };
            let pin = self.commit_pin(&current)?;
            let published_path =
                fs.publish_temp(&staged, &upper_dir_path, name, RenameMode::Replace)?;
            let upper_real = upper_layer.child_real_object(&published_path);
            self.replace_facts(upper_real, &pin);
            Ok(())
        })();
        if let Err(err) = result {
            let _ = fs.cleanup_workdir_temp(staged.name(), staged.kind());
            return Err(err);
        }
        Ok(())
    }

    /// On the sibling arms the pin is resolved through this instance's cache key and verified
    /// to denote `self`, so the double-mint window (a sibling in the cache slot) fails closed
    /// instead of misregistering.
    fn commit_pin(&self, current: &Arc<OverlayInode>) -> Result<Arc<OverlayInode>> {
        if core::ptr::addr_eq(Arc::as_ptr(current), self) {
            return Ok(current.clone());
        }
        let pin = self.cached_self_arc()?;
        if !core::ptr::addr_eq(Arc::as_ptr(&pin), self) {
            return Err(Error::with_message(
                Errno::EIO,
                "this instance is no longer the cache-resident identity for its key",
            ));
        }
        Ok(pin)
    }

    /// An absent origin record and a record read error both fail closed to `false`; mounts
    /// without the private-xattr capability cannot detect sibling copies at all (documented gap).
    fn is_same_publication_target(&self, current: &Arc<OverlayInode>, fs: &Arc<OverlayFs>) -> bool {
        if core::ptr::addr_eq(Arc::as_ptr(current), self) {
            return true;
        }
        if current.upper.get().is_none()
            && self.upper.get().is_none()
            && let (Some(current_lower), Some(self_lower)) =
                (current.lowers.first(), self.lowers.first())
            && Arc::ptr_eq(current_lower.real_inode(), self_lower.real_inode())
        {
            return true;
        }
        if current.upper.get().is_some() {
            let Ok(Some(record)) = fs.read_lower_id(current.visible_source().real_inode()) else {
                return false;
            };
            return fs.origin_real_ino_resolves(&record, &self.real_object_stack());
        }
        false
    }

    /// A zero-length read before the declared size, or a short write, is `EIO` — a partial
    /// transfer is never treated as successful I/O.
    fn promote_regular_file(&self, temp: &Arc<dyn Inode>) -> Result<()> {
        let lower = self.lower_source()?;
        let size = lower.real_inode().size();
        let mut offset = 0usize;
        let mut buffer = vec![0u8; COPY_CHUNK_SIZE];
        while offset < size {
            let chunk = min(COPY_CHUNK_SIZE, size - offset);
            let mut writer = VmWriter::from(&mut buffer[..chunk]).to_fallible();
            let read_len = lower
                .real_inode()
                .read_at(offset, &mut writer, StatusFlags::empty())?;
            if read_len == 0 {
                return_errno_with_message!(
                    Errno::EIO,
                    "the lower source returned a zero-length read before its declared size"
                );
            }
            let mut reader = VmReader::from(&buffer[..read_len]).to_fallible();
            let write_len = temp.write_at(offset, &mut reader, StatusFlags::empty())?;
            if write_len != read_len {
                return_errno_with_message!(
                    Errno::EIO,
                    "the workdir temp accepted a short write during copy-up"
                );
            }
            offset += write_len;
        }
        Ok(())
    }

    fn promote_symlink(&self, temp: &Arc<dyn Inode>) -> Result<()> {
        let lower = self.lower_source()?;
        let target = match lower.real_inode().read_link()? {
            SymbolicLink::Plain(target) => target,
            SymbolicLink::Path(_) => {
                return_errno_with_message!(
                    Errno::EOPNOTSUPP,
                    "a path-style symlink target cannot be copied up"
                );
            }
        };
        temp.write_link(&target)
    }

    /// Mode transfer skips symlinks: backing filesystems treat a symlink `set_mode` as a no-op
    /// or reject it, and copy-up must not depend on that per-fs behavior.
    pub(super) fn transfer_metadata(
        &self,
        source: &Arc<dyn Inode>,
        temp: &Arc<dyn Inode>,
    ) -> Result<()> {
        temp.set_owner(source.owner()?)?;
        temp.set_group(source.group()?)?;
        if !matches!(source.type_(), InodeType::SymLink) {
            temp.set_mode(source.mode()?)?;
        }
        if source.type_().is_regular_file() {
            temp.resize(source.size())?;
        }
        Ok(())
    }

    /// Runs last — after every step that could refresh `mtime`/`ctime` — so the copy-up
    /// preserves the lower timestamps instead of publishing the copy-up instant.
    pub(super) fn transfer_timestamps(
        &self,
        source: &Arc<dyn Inode>,
        temp: &Arc<dyn Inode>,
    ) -> Result<()> {
        temp.set_atime(source.atime());
        temp.set_mtime(source.mtime());
        temp.set_ctime(source.ctime());
        Ok(())
    }

    fn lower_source(&self) -> Result<RealObject> {
        self.lowers.first().cloned().ok_or_else(|| {
            Error::with_message(
                Errno::EIO,
                "a lower-backed overlay object has no lower source",
            )
        })
    }
}
