// SPDX-License-Identifier: MPL-2.0

//! Layer stack assembly for the overlay filesystem.
//!
//! This module resolves the real `upperdir`/`lowerdir` roots into pinned
//! [`OverlayLayer`]s and freezes them into an [`OverlayLayerStack`]. It owns
//! layer-root resolution, layer ordering, the per-unique-underlying-superblock
//! `fsid` assignment, and the layer-root overlap validation. The stack is
//! constructed once by [`OverlayLayerStack::assemble`] during `OverlayFs::new`.
//!
//! Lower layers are read-only: the overlay never writes the lower layers.
//!
//! - Non-`default_permissions` mounts promote mutating paths to the upper
//!   first.
//! - `default_permissions` mounts keep a documented limitation: the persisted
//!   directory-merging staleness marker (the overlay `trusted.overlay.impure`
//!   xattr) is not refreshed after mutations, so the marker can remain stale.
//!   This limitation is scoped to that persisted marker; the other layer-stack
//!   invariants in this module still hold.
//! - External concurrent modification of the lower layers is unsupported:
//!   projection and identity assume a stable layer stack, and an external
//!   lower writer can corrupt the visible merge.
//! - The mount boundary rejects the one mountable corruption form — lower/
//!   upper/workdir/lower-root overlap — while read-write lower backends
//!   remain accepted.
//!
//! References:
//!
//! - <https://elixir.bootlin.com/linux/v7.0/source/Documentation/filesystems/overlayfs.rst#L350-L364>
//!   (Linux overlayfs parity; stacks colon-separated lowerdirs with the first entry topmost)
//! - <https://elixir.bootlin.com/linux/v7.0/source/fs/overlayfs/super.c#L1273>
//!   (Linux `ovl_check_overlapping_layers`)
//! - <https://elixir.bootlin.com/linux/v7.0/source/fs/overlayfs/ovl_entry.h#L33-L42>
//!   (Linux `ovl_layer[].fsid`, upper fsid 0)

use device_id::DeviceId;

use crate::{
    fs::vfs::{
        file_system::{FileSystem, FsFlags},
        inode::Inode,
        path::{AT_FDCWD, Dentry, EmptyPathStr, FsPath, Mount, Path},
    },
    prelude::*,
};

/// Two-phase assembly input: resolve-then-assign.
type LayerParts = (RealPath, Arc<dyn Inode>, Arc<dyn FileSystem>, DeviceId);

/// Resolves `raw_path` through `lookup_no_follow` in the mounting task's
/// filesystem context: intermediate symlink components are followed, the
/// final component is not (mount-time roots are the literal resolved
/// directories). This is the single shared path-resolution helper of the
/// mount module, used for the upper/workdir resolution and the
/// instance-stability probe.
pub(super) fn resolve_root_path(raw_path: &str) -> Result<Path> {
    let fs_path = FsPath::from_fd_at(AT_FDCWD, raw_path, EmptyPathStr::Reject)?;
    super::with_current_posix_thread(|posix_thread| {
        let fs = posix_thread.read_fs();
        fs.resolver().read().lookup_no_follow(&fs_path)
    })
}

/// The ordered, immutable layer stack of an overlay mount.
#[derive(Debug)]
pub(in overlayfs) struct OverlayLayerStack {
    pub(in overlayfs) upper: Option<OverlayLayer>,
    pub(in overlayfs) lowers: Vec<OverlayLayer>,
}

#[derive(Clone, Debug)]
pub(in overlayfs) struct RealPath {
    mount: Weak<Mount>,
    dentry: Arc<Dentry>,
    inode: Arc<dyn Inode>,
}

impl RealPath {
    pub(in overlayfs) fn from_path(path: &Path) -> Self {
        Self {
            mount: Arc::downgrade(path.mount_node()),
            dentry: path.dentry().clone(),
            inode: path.inode().clone(),
        }
    }

    /// Returns `Err(EIO)` when the anchor mount is no longer alive (the
    /// parent overlay was unmounted while a stored path survived).
    pub(in overlayfs) fn upgrade(&self) -> Result<Path> {
        let mount = self.mount.upgrade().ok_or_else(|| {
            Error::with_message(
                Errno::EIO,
                "the anchor mount of the stored real path is no longer alive",
            )
        })?;
        Ok(Path::new(mount, self.dentry.clone()))
    }

    pub(in overlayfs) fn inode(&self) -> &Arc<dyn Inode> {
        &self.inode
    }
}

/// One pinned real layer root of an overlay mount.
#[derive(Debug)]
pub(in overlayfs) struct OverlayLayer {
    pub(in overlayfs) root_path: RealPath,
    pub(in overlayfs) root_inode: Arc<dyn Inode>,
    pub(in overlayfs) fs: Arc<dyn FileSystem>,
    /// Per-unique-underlying-superblock identifier assigned at assembly.
    pub(in overlayfs) fsid: u64,
    /// `st_dev` of the layer root, used for same-filesystem comparisons.
    pub(in overlayfs) container_dev_id: DeviceId,
}

impl OverlayLayer {
    /// Resolves `raw_path` into pinned layer-root parts, downgrading the
    /// `Path` into the layer-root anchor [`RealPath`].
    fn resolve_parts(raw_path: &str) -> Result<LayerParts> {
        // Missing paths surface the resolver's `ENOENT`; non-directory roots
        // fail with `ENOTDIR`.
        let path = resolve_root_path(raw_path)?;
        if !path.type_().is_directory() {
            return_errno_with_message!(Errno::ENOTDIR, "the layer root is not a directory");
        }
        Ok((
            RealPath::from_path(&path),
            path.inode().clone(),
            path.fs(),
            path.metadata()?.container_dev_id,
        ))
    }

    /// Rejects an overlap between `new` and every already-assembled layer
    /// root in `others`.
    ///
    /// - Same directory: identical dentry or inode objects.
    /// - Ancestor/descendant: one root lies within the other's resolved
    ///   hierarchy ([`Dentry::is_equal_or_descendant_of`]).
    /// - Mount boundary: parent chains never cross a mount root, so another
    ///   mount's root is never misjudged as nested.
    ///
    /// Only the layer roots themselves are compared, so legal nested
    /// subdirectories are never rejected. Violations return `EINVAL`.
    fn validate_layer_overlap(new: &OverlayLayer, others: &[&OverlayLayer]) -> Result<()> {
        let new_path = new.root_path.upgrade()?;
        let new_dentry = new_path.dentry();
        for other in others {
            let other_path = other.root_path.upgrade()?;
            let other_dentry = other_path.dentry();
            if Arc::ptr_eq(new_dentry, other_dentry)
                || Arc::ptr_eq(&new.root_inode, &other.root_inode)
            {
                return_errno_with_message!(
                    Errno::EINVAL,
                    "overlay layer roots must be distinct directories"
                );
            }
            if new_dentry.is_equal_or_descendant_of(other_dentry)
                || other_dentry.is_equal_or_descendant_of(new_dentry)
            {
                return_errno_with_message!(
                    Errno::EINVAL,
                    "overlay layer roots must not be each other's ancestor or descendant"
                );
            }
        }
        Ok(())
    }
}

impl OverlayLayerStack {
    /// Assembles the resolved upper/lower layer stack of an overlay mount.
    ///
    /// The upper root fails with `EROFS` when its backend is read-only and
    /// the overlay itself was not forced read-only. Non-empty `lower_dirs` is
    /// enforced here: an empty lower stack is rejected with `EINVAL`.
    pub(super) fn assemble(
        upper_dir: Option<String>,
        lower_dirs: Vec<String>,
        is_forced_read_only: bool,
    ) -> Result<Self> {
        let mut upper_parts = None;
        if let Some(raw_path) = upper_dir {
            let (root_path, root_inode, fs, container_dev_id) =
                OverlayLayer::resolve_parts(&raw_path)?;
            if !is_forced_read_only && fs.flags().contains(FsFlags::RDONLY) {
                return_errno_with_message!(Errno::EROFS, "the upper filesystem is read-only");
            }
            upper_parts = Some((root_path, root_inode, fs, container_dev_id));
        }

        if lower_dirs.is_empty() {
            return_errno_with_message!(
                Errno::EINVAL,
                "at least one lower layer is required to assemble the layer stack"
            );
        }
        let lower_parts: Vec<LayerParts> = lower_dirs
            .iter()
            .map(|raw_path| OverlayLayer::resolve_parts(raw_path))
            .collect::<Result<_>>()?;

        // The upper filesystem owns `fsid` 0 on writable overlays.
        let mut unique_fses: Vec<Arc<dyn FileSystem>> = Vec::new();
        let mut fsid_of_fn = |fs: &Arc<dyn FileSystem>| -> u64 {
            if let Some(index) = unique_fses
                .iter()
                .position(|seen_fs| Arc::ptr_eq(seen_fs, fs))
            {
                index as u64
            } else {
                unique_fses.push(fs.clone());
                (unique_fses.len() - 1) as u64
            }
        };

        let upper = upper_parts.map(|(root_path, root_inode, fs, container_dev_id)| {
            let fsid = fsid_of_fn(&fs);
            OverlayLayer {
                root_path,
                root_inode,
                fs,
                fsid,
                container_dev_id,
            }
        });
        let lowers = lower_parts
            .into_iter()
            .map(|(root_path, root_inode, fs, container_dev_id)| {
                let fsid = fsid_of_fn(&fs);
                OverlayLayer {
                    root_path,
                    root_inode,
                    fs,
                    fsid,
                    container_dev_id,
                }
            })
            .collect::<Vec<_>>();

        let all_layers: Vec<&OverlayLayer> = upper.iter().chain(lowers.iter()).collect();
        for (index, new_layer) in all_layers.iter().enumerate() {
            OverlayLayer::validate_layer_overlap(new_layer, &all_layers[index + 1..])?;
        }

        Ok(Self { upper, lowers })
    }
}
