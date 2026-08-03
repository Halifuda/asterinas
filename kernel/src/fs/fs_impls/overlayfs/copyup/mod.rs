// SPDX-License-Identifier: MPL-2.0

//! The module root of the `copyup_authority_file_views` meso (meso-04).
//!
//! This module declares the four `copyup/*` submodules and hosts the thin
//! inode-level delegation entries of the frozen meso-04 spec §4: the
//! `OverlayFs`-extension impl block (the `workdir_temp_serial` unique-naming
//! accessor, P1-34), the `OverlayInode` delegation helpers
//! (`select_real_inode`, `fs_arc`, `record_copyup_transition`), and the VFS
//! helper bodies called by the canonical `FileOps`/`Inode` trait impls in
//! `projection/inode.rs`: `read_at_impl`/`write_at_impl` (P1-10), `open_impl`
//! (P1-08), `seek_end_impl` (P1-11), `resize_impl` (P1-02),
//! `fallocate_impl` (P1-14), `sync_all_impl`/`sync_data_impl` (P1-13), and
//! `read_link_impl`/`page_cache_impl` (P1-32/P1-37). The real control flow
//! lives in the sibling files created in parallel from the same frozen spec:
//! `trigger.rs` (winner/waiter protocol + top-down ancestor walk),
//! `promote.rs` (object-kind promotion body and publication), `workdir.rs`
//! (temp lifecycle).
//!
//! Visibility: `coordination` is declared `pub(super)` — read through the
//! spec's overlayfs-ceiling audit as `pub(in crate::fs::fs_impls::overlayfs)` —
//! because the frozen Wave-3 `OverlayInode::copyup_transition` field (in
//! `projection/inode.rs`) names `copyup::coordination::CopyUpTransition` from
//! a sibling module; the other three submodules stay private to `copyup`
//! (spec §1 "Must Remain Internal"). The delegation helpers are published at
//! the same ceiling because the meso-02 positive-binding hook and the
//! cross-meso consumers (meso-05/06) call them from sibling module trees.
//!
//! Lock contract (spec §3): the delegation entries hold no Overlay lock beyond
//! the brief `INODE` facts snapshot inside `select_real_inode`
//! (snapshot-and-release, never held across an underlying call);
//! `record_copyup_transition` takes a brief non-blocking `CUL` `try_lock`
//! (Hazard 3; invariant I3); the EROFS gate precedes every promotion side
//! effect (I10). No per-open real-inode view carrier exists: every call
//! re-resolves the current authority per operation (invariant I4, Linux
//! `ovl_real_file_path` follow-copy-up, file.c:128-171).

use core::sync::atomic::Ordering;

use self::coordination::{CopyUpTransition, CopyUpPhase};
use crate::{
    fs::{
        file::{AccessMode, PerOpenFileOps, Permission, StatusFlags},
        fs_impls::overlayfs::{
            mount::OverlayFs, projection::OverlayInode, AccessType,
        },
        vfs::inode::{FallocMode, Inode, SymbolicLink},
    },
    prelude::*,
    vm::page_cache::PageCache,
};

pub(super) mod coordination;

mod promote;
mod trigger;
mod workdir;

pub(in crate::fs::fs_impls::overlayfs) use workdir::WorkdirTempRequest;

impl OverlayFs {
    /// Returns the next saturating workdir temp serial (P1-34).
    ///
    /// The per-mount serial is the unique-naming context of the workdir temp
    /// lifecycle; the consuming `generate_workdir_temp_name` (frozen sibling
    /// pass, `copyup/workdir.rs`) composites it as
    /// `#{target_name}#{parent_ino}#{serial}` (spec §4 `workdir.rs`). The
    /// fetch is saturating — `AtomicU64::try_update` commits
    /// `saturating_add(1)` and retries on contention, so the counter converges
    /// to and stays at `u64::MAX` (the same pattern as
    /// `IdentityPolicy::allocate_fallback_ino`) — and never gates I/O.
    /// Uniqueness is by construction (target name + upper-parent real ino +
    /// per-mount serial); no lock is held (spec §3.0: workdir temp naming is
    /// uniqueness-based, not lock-based).
    pub(super) fn workdir_temp_serial(&self) -> u64 {
        match self.workdir_temp_serial.try_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |current| Some(current.saturating_add(1)),
        ) {
            // The closure never returns `None`, so `try_update` always
            // succeeds; this arm is defensive and unreachable.
            Ok(previous) => previous.saturating_add(1),
            Err(_) => u64::MAX,
        }
    }
}

impl OverlayInode {
    /// Resolves the current authority's real inode for one delegated call.
    ///
    /// A brief `INODE` facts snapshot selects `facts.upper` when present, else
    /// the topmost lower (`lowers[0]`); the guard is released before any
    /// underlying call and the returned strong pin keeps the resolved real
    /// inode alive for the delegation. Every operation re-resolves this way
    /// (invariant I4), so an fd opened while lower-backed observes the upper
    /// real inode on its next operation after a copy-up (Linux
    /// `ovl_real_file_path`, file.c:128-171). The `lowers[0]` index is safe by
    /// the frozen facts invariant `upper.is_some() || !lowers.is_empty()`
    /// (meso-02 spec §4).
    pub(super) fn select_real_inode(&self) -> Arc<dyn Inode> {
        let facts = self.facts_snapshot();
        match facts.upper() {
            Some(upper) => upper.real_inode().clone(),
            None => facts.lowers()[0].real_inode().clone(),
        }
    }

    /// Upgrades the owning mount's `Weak` reference into an `Arc` (spec §3.4
    /// item 5).
    ///
    /// The upgrade routes through the public `Inode::fs()` surface — the only
    /// mount route a sibling module can name, since the `OverlayInode::fs`
    /// field stays `pub(super)` inside `projection` per the frozen meso-02
    /// carrier — and downcasts the `Arc<dyn FileSystem>` to `Arc<OverlayFs>`.
    /// The downcast cannot fail for an `OverlayInode` (its `fs` field is a
    /// `Weak<OverlayFs>`); the failure arm is defensive. The post-teardown
    /// failure arm is the meso-02 §3.5 item-4 platform-lifetime question
    /// carried verbatim by `Inode::fs()` (`unreachable!`); no `.unwrap()`/
    /// `.expect()` is introduced.
    pub(super) fn fs_arc(&self) -> Result<Arc<OverlayFs>> {
        let fs = self.fs();
        Arc::downcast::<OverlayFs>(fs).map_err(|_| {
            Error::with_message(
                Errno::EIO,
                "the inode does not belong to an overlay filesystem",
            )
        })
    }

    /// Records the copy-up transition coordinate at the first positive
    /// binding publication (meso-02 §3.4 item 2 hook; invoked from
    /// `OverlayFs::lookup_binding` before `publish_binding`).
    ///
    /// Invariant I3: the coordinate (`publication_parent` + `name`) is set
    /// once — the first positive binding wins — and is immutable thereafter.
    /// The guard is a non-blocking `try_lock` that skips when contended:
    /// contention implies a transition is already running, hence the
    /// coordinate is already set (Hazard 3: waiters hold nothing while
    /// blocked). The initial phase is [`CopyUpPhase::Idle`].
    pub(super) fn record_copyup_transition(
        &self,
        publication_parent: Arc<OverlayInode>,
        name: &str,
    ) {
        let Some(mut guard) = self.copyup_transition.try_lock() else {
            return;
        };
        if guard.is_some() {
            return;
        }
        *guard = Some(CopyUpTransition {
            publication_parent,
            name: String::from(name),
            phase: CopyUpPhase::Idle,
        });
    }
}

impl OverlayInode {
    // P1-10 read delegation: per-call authority re-resolution; a lower-backed
    // read passes `O_NOATIME` (P1-08 §19a) so a read never updates the lower
    // atime. The two brief facts snapshots (here and inside
    // `select_real_inode`) may observe an authority advance between them,
    // which is benign (Hazard 7); no Overlay lock is held across any
    // underlying call (spec §4).
    pub(in crate::fs::fs_impls::overlayfs) fn read_at_impl(
        &self,
        offset: usize,
        writer: &mut VmWriter,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        let facts = self.facts_snapshot();
        let is_lower_backed = facts.upper().is_none();
        let real = self.select_real_inode();
        let status_flags = if is_lower_backed {
            status_flags | StatusFlags::O_NOATIME
        } else {
            status_flags
        };
        real.read_at(offset, writer, status_flags)
    }

    // P1-10 write delegation: per-call authority re-resolution; `O_APPEND` is
    // applied from the passed status flags (offset := real size) as the
    // packet-ruled defense (spec §3.4 item 6). Write-capable fds are upper by
    // construction (I5), so delegation never bypasses the trigger.
    pub(in crate::fs::fs_impls::overlayfs) fn write_at_impl(
        &self,
        offset: usize,
        reader: &mut VmReader,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        let real = self.select_real_inode();
        let offset = if status_flags.contains(StatusFlags::O_APPEND) {
            real.size()
        } else {
            offset
        };
        real.write_at(offset, reader, status_flags)
    }
}

impl OverlayInode {
    // P1-08: directory opens are served by the merged readdir path (meso-03)
    // and read-only opens take no side effect; only writable opens reach the
    // EROFS gate (I10) and the write-intent promotion trigger (P1-12
    // anchoring). The VFS handle uses this inode's own `FileOps`, so the
    // successful path returns `None`; failures surface as `Some(Err)`.
    pub(in crate::fs::fs_impls::overlayfs) fn open_impl(
        &self,
        access_mode: AccessMode,
        _status_flags: StatusFlags,
    ) -> Option<Result<Box<dyn PerOpenFileOps>>> {
        if self.type_().is_directory() {
            return None;
        }
        if !access_mode.is_writable() {
            return None;
        }
        let fs = match self.fs_arc() {
            Ok(fs) => fs,
            Err(err) => return Some(Err(err)),
        };
        if fs.policy().is_effective_read_only() {
            return Some(Err(Error::with_message(
                Errno::EROFS,
                "the overlay mount is read-only",
            )));
        }
        match self.ensure_upper_authority() {
            Ok(()) => None,
            Err(err) => Some(Err(err)),
        }
    }

    // P1-11: the end position of the current authority's real inode.
    pub(in crate::fs::fs_impls::overlayfs) fn seek_end_impl(&self) -> Option<usize> {
        self.select_real_inode().seek_end()
    }

    // P1-02 truncate leg: EROFS, then the uniform mutating admission
    // (wave-4 repair item 3 — the path-based `truncate()` syscall performs no
    // VFS-level `MAY_WRITE` check of its own, so this entry must run the
    // meso-05 two-stage admission BEFORE any side effect, including the
    // copy-up promotion), then the promotion trigger and delegation to the
    // (upper) current authority.
    pub(in crate::fs::fs_impls::overlayfs) fn resize_impl(&self, new_size: usize) -> Result<()> {
        if self.fs_arc()?.policy().is_effective_read_only() {
            return_errno_with_message!(Errno::EROFS, "the overlay mount is read-only");
        }
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.ensure_upper_authority()?;
        self.select_real_inode().resize(new_size)
    }

    // P1-14: EROFS, then the uniform mutating admission (wave-4 repair item
    // 3 — gate independence: `fallocate` shares `resize`'s side-effect class,
    // so the meso-05 admission runs here too rather than relying on the fd
    // path alone), then the promotion trigger and delegation to the (upper)
    // current authority.
    pub(in crate::fs::fs_impls::overlayfs) fn fallocate_impl(
        &self,
        mode: FallocMode,
        offset: usize,
        len: usize,
    ) -> Result<()> {
        if self.fs_arc()?.policy().is_effective_read_only() {
            return_errno_with_message!(Errno::EROFS, "the overlay mount is read-only");
        }
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.ensure_upper_authority()?;
        self.select_real_inode().fallocate(mode, offset, len)
    }

    // P1-13: pure delegation to the current authority; no promotion (durability
    // policy = auto, P2-12 note).
    pub(in crate::fs::fs_impls::overlayfs) fn sync_all_impl(&self) -> Result<()> {
        self.select_real_inode().sync_all()
    }

    // P1-13: same delegation as `sync_all`; durability policy = auto (P2-12
    // note).
    pub(in crate::fs::fs_impls::overlayfs) fn sync_data_impl(&self) -> Result<()> {
        self.select_real_inode().sync_data()
    }

    // P1-32: pure delegation to the current authority; no promotion.
    pub(in crate::fs::fs_impls::overlayfs) fn read_link_impl(&self) -> Result<SymbolicLink> {
        self.select_real_inode().read_link()
    }

    // P1-37: pure forwarder to the current authority's real page cache (upper
    // after promotion; the lower source for lower-backed read views). Never
    // promotes: the parameterless seam carries no write intent (§4.3).
    pub(in crate::fs::fs_impls::overlayfs) fn page_cache_impl(&self) -> Option<PageCache> {
        self.select_real_inode().page_cache()
    }
}
