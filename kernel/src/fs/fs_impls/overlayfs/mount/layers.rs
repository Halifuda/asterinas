// SPDX-License-Identifier: MPL-2.0

//! Layer stack assembly for the overlay filesystem (`P0-02`/`P0-03` slice).
//!
//! This module resolves the real `upperdir`/`lowerdir` roots into pinned
//! [`OverlayLayer`]s and freezes them into an immutable [`OverlayLayerStack`].
//! It owns layer-root resolution, layer ordering, the per-unique-underlying-
//! superblock `fsid` assignment (`BC-1` §8), and the `P0-02` mount-time layer
//! gates. The stack is constructed once by [`OverlayLayerStack::assemble`]
//! during `OverlayFs::new` and is immutable afterwards for the mount lifetime;
//! sibling Mesos read it only.

use device_id::DeviceId;

use crate::{
    fs::vfs::{
        file_system::{FileSystem, FsFlags},
        inode::Inode,
        path::{AT_FDCWD, EmptyPathStr, FsPath, Path},
        registry::FsCreationCtx,
    },
    prelude::*,
};

/// Resolves `raw_path` through `lookup_no_follow` in the mounting task's
/// filesystem context.
///
/// This is the single shared path-resolution seam of the `mount` meso
/// (wave-1 review `dry` fix, item 8): [`OverlayLayer::resolve`], the sibling
/// `build.rs` upper/workdir resolution, and the `claims.rs`
/// instance-stability probe all go through this helper instead of each
/// re-implementing the `FsPath::from_fd_at(AT_FDCWD, …)` +
/// `resolver().read().lookup_no_follow(…)` sequence (whitelist Rule B: the
/// exact logic is required at three sites within this meso). Intermediate
/// symlink components are followed; the final component is not (mount-time
/// roots are the literal resolved directories).
pub(super) fn resolve_root_path(fs_creation_ctx: &FsCreationCtx, raw_path: &str) -> Result<Path> {
    let fs_path = FsPath::from_fd_at(AT_FDCWD, raw_path, EmptyPathStr::Reject)?;
    // Resolve inside a single statement so the `borrow_fs()` `Ref` and the
    // resolver read guard live exactly as long as the lookup (same shape as
    // `registry.rs::resolve_block_device`); neither escapes this scope.
    fs_creation_ctx
        .task_ctx()
        .thread_local
        .borrow_fs()
        .resolver()
        .read()
        .lookup_no_follow(&fs_path)
}

/// The ordered, immutable layer stack of an overlay mount.
///
/// Lookup searches the upper layer first (when present) and then the lower
/// layers top-to-bottom. The stack is assembled exactly once and is immutable
/// after construction (`P0-02` immutability invariant); sibling Mesos read it
/// only and never re-create, copy ownership of, or mutate it.
#[derive(Debug)]
pub(super) struct OverlayLayerStack {
    /// Pinned upper layer; present iff the overlay is writable.
    pub(super) upper: Option<OverlayLayer>,
    /// Pinned lower layers; non-empty and ordered topmost-first.
    pub(super) lowers: Vec<OverlayLayer>,
}

/// One pinned real layer root of an overlay mount.
///
/// The pins keep the underlying layer roots alive for the mount lifetime
/// (layer-lifetime pins, `P0-02`). `container_dev_id` carries the `st_dev`
/// same-filesystem evidence used by the `P0-03` upper/workdir validation.
#[derive(Debug)]
pub(super) struct OverlayLayer {
    /// Pinned real layer root (lifetime pin).
    pub(super) root_inode: Arc<dyn Inode>,
    /// Underlying filesystem identity of the layer root.
    pub(super) fs: Arc<dyn FileSystem>,
    /// Per-unique-underlying-superblock identifier assigned at assembly (`BC-1` §8).
    pub(super) fsid: u64,
    /// `st_dev` of the layer root, used for same-filesystem comparisons (`P0-03`).
    pub(super) container_dev_id: DeviceId,
}

impl OverlayLayer {
    /// Resolves `raw_path` into a pinned overlay layer root.
    ///
    /// The path is resolved with `lookup_no_follow` in the mounting task's
    /// filesystem context (`P0-02`), so a missing path surfaces the resolver's
    /// `ENOENT` and a non-directory root fails with `ENOTDIR`. The resolved
    /// inode and its filesystem are pinned for the mount lifetime. `fsid` is a
    /// placeholder here; [`OverlayLayerStack::assemble`] assigns the real
    /// per-unique-underlying-superblock identifier afterwards.
    pub(super) fn resolve(fs_creation_ctx: &FsCreationCtx, raw_path: &str) -> Result<Self> {
        let path = resolve_root_path(fs_creation_ctx, raw_path)?;
        if !path.type_().is_directory() {
            return_errno_with_message!(Errno::ENOTDIR, "the layer root is not a directory");
        }
        Ok(Self {
            root_inode: path.inode().clone(),
            fs: path.fs(),
            fsid: 0,
            container_dev_id: path.metadata().container_dev_id,
        })
    }
}

impl OverlayLayerStack {
    /// Assembles the resolved upper/lower layer stack of an overlay mount.
    ///
    /// `upper_dir` is present only when a writable overlay was requested;
    /// `lower_dirs` carries the option-order lower paths (topmost lower =
    /// rightmost option). The upper root fails with `EROFS` when its backend
    /// reports `FsFlags::RDONLY` and the overlay itself was not forced
    /// read-only (`P0-02` failure, Case 3); `is_forced_read_only` is the
    /// already-parsed option value fed from `OverlayMountOptions` (wave-1
    /// review `dry` fix, item 10 — the flag is no longer re-derived from
    /// `fs_creation_ctx.flags()` here). Every layer root is resolved through
    /// [`OverlayLayer::resolve`] and one `fsid` is assigned per unique
    /// underlying filesystem instance, deduplicated at assembly time.
    ///
    /// Non-empty lower layers are enforced at this checked constructor
    /// (wave-1 review `rust-type-invariants` fix, item 8): an empty
    /// `lower_dirs` is rejected with `EINVAL` instead of being admitted by a
    /// `Vec` that documents the invariant in a comment only.
    pub(super) fn assemble(
        fs_creation_ctx: &FsCreationCtx,
        upper_dir: Option<String>,
        lower_dirs: Vec<String>,
        is_forced_read_only: bool,
    ) -> Result<Self> {
        let mut upper = match upper_dir {
            Some(raw_path) => {
                let layer = OverlayLayer::resolve(fs_creation_ctx, &raw_path)?;
                // A writable overlay cannot be served by a read-only upper
                // backend unless the overlay itself was forced read-only
                // (`P0-02` pre-condition; Case 3 EROFS).
                if !is_forced_read_only && layer.fs.flags().contains(FsFlags::RDONLY) {
                    return_errno_with_message!(Errno::EROFS, "the upper filesystem is read-only");
                }
                Some(layer)
            }
            None => None,
        };

        // Defensive structural rejection of the illegal empty state (`P0-02`
        // invariant, wave-1 review item 8): `OverlayMountOptions::parse`
        // guarantees a non-empty `lowerdir`, but the checked constructor is
        // the last line of defense so the published stack can never carry an
        // empty `lowers` vector.
        if lower_dirs.is_empty() {
            return_errno_with_message!(
                Errno::EINVAL,
                "at least one lower layer is required to assemble the layer stack"
            );
        }
        let mut lowers = Vec::with_capacity(lower_dirs.len());
        for raw_path in Self::normalize_lower_ordering(lower_dirs) {
            lowers.push(OverlayLayer::resolve(fs_creation_ctx, &raw_path)?);
        }

        // Assign one `fsid` per unique underlying superblock (`BC-1` §8):
        // layers pinned on the same underlying filesystem instance share a
        // single identifier. The identifier is assigned in stack order (upper
        // first when present), so the upper filesystem always owns `fsid` 0 on
        // writable overlays, mirroring the Linux `ovl_layer[].fsid` layout.
        // The assignments complete inside `assemble`, so the published stack
        // is still immutable after construction.
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
        if let Some(upper_layer) = &mut upper {
            upper_layer.fsid = fsid_of_fn(&upper_layer.fs);
        }
        for lower_layer in &mut lowers {
            lower_layer.fsid = fsid_of_fn(&lower_layer.fs);
        }

        Ok(Self { upper, lowers })
    }

    /// Returns `lower_dirs` reordered topmost-first.
    ///
    /// Lower layers are stacked from the topmost (the rightmost option in the
    /// mount option list) down to the bottommost (the leftmost option), so the
    /// first element of the returned vector is the topmost lower layer.
    fn normalize_lower_ordering(lower_dirs: Vec<String>) -> Vec<String> {
        lower_dirs.into_iter().rev().collect()
    }
}
